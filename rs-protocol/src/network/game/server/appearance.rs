//! PLAYER_INFO appearance sub-block encoder.
//!
//! The appearance block is a sub-payload of the PLAYER_INFO appearance
//! update mask. The game engine rebuilds it on each appearance change
//! (equipment swap, re-colour, etc.) and caches the resulting bytes on
//! the `Player` so the PLAYER_INFO builder can splice them in verbatim
//! without re-encoding per-tick.
//!
//! Lives in rs-protocol because it is entirely wire-format work: byte
//! layout, alt-byte transforms, length prefix. The engine passes in a
//! plain `AppearanceInput` view of the relevant state.

/// Plain-data view of everything the appearance block needs.
///
/// Callers in rs-engine snapshot the current `Player` state into an
/// `AppearanceInput` and pass it to [`build_appearance_block`].
pub struct AppearanceInput<'a> {
    pub gender: u8,
    /// Body-kit ids: `[hair, beard, torso, arms, hands, legs, feet]`.
    pub body: [i16; 7],
    /// Equipment ids indexed by EquipSlot (0 = head, 1 = cape, …).
    /// Length must be at least 12 — indices 0..=11 are read.
    pub equipment: &'a [i16],
    /// Five body colours: `[hair, torso, legs, feet, skin]`.
    pub colors: [u8; 5],
    /// Slots hidden by equipped items' wearpos2/wearpos3 (matches 2004scape skippedSlots).
    pub skipped_slots: [bool; 12],
    pub render_anim: u16,
    pub display_name: &'a str,
    pub combat_level: u8,
    /// 289's head icons, a bit each (`headicons_set`); the protect prayers
    /// are bits 3 (melee), 4 (missiles) and 5 (magic).
    pub headicons: u8,
}

/// 2011's prayer head icon for 289's head icons. The client draws
/// `headicons_prayer[n]` over the player, none at -1 (`PlayerEntity`,
/// `AClass54_Sub2`): protect from melee 0, missiles 1, magic 2 (void-634
/// `changedPrayerIcon`). 289 drew a sprite per bit.
fn prayer_icon(headicons: u8) -> i8 {
    if headicons & 1 << 5 != 0 {
        2
    } else if headicons & 1 << 4 != 0 {
        1
    } else if headicons & 1 << 3 != 0 {
        0
    } else {
        -1
    }
}

/// Build the appearance sub-block bytes for PLAYER_INFO.
///
/// Layout:
///   - 1 byte: packed flags (bit 0 = gender)
///   - 3 bytes: title, skull and prayer icons (signed, -1 none); the skull
///     (289's head icon bit 0) waits for the wilderness port
///   - 1 byte: hidden flag (0 = visible)
///   - 12 slots: each either 0 (empty), `256 + kit_id` (BE u16), or
///     `16384 + equip_id` (BE u16)
///   - 2 bytes: extra/aura slot (`0x0000`)
///   - 5 bytes: body colours
///   - 2 bytes: render anim (BE u16)
///   - display_name bytes + `0x00` terminator
///   - 1 byte: combat level
///   - 1 byte: summoning combat level (0)
///   - 1 byte: unknown (0xFF)
///   - 1 byte: NPC transmog (0)
///
/// The caller prepends `(128 - len) & 0xFF` as the SUBTRACT length
/// prefix on the full block. We do that here so the returned `Vec<u8>`
/// is ready to slice into the cache on `Player`.
pub fn build_appearance_block(input: &AppearanceInput) -> Vec<u8> {
    let mut data = rs_io::Packet::new(64);

    let flags: u8 = input.gender & 0x1;
    data.p1(flags);
    data.p1(0xFF); // title
    data.p1(0xFF); // pk icon
    data.p1(prayer_icon(input.headicons) as u8); // prayer icon
    data.p1(0); // hidden

    fn put_kit(d: &mut rs_io::Packet, kit_id: i16) {
        if kit_id < 0 {
            d.p1(0);
        } else {
            d.p2(256u16 + kit_id as u16);
        }
    }
    fn put_equip(d: &mut rs_io::Packet, equip_id: i16) {
        if equip_id <= 0 {
            d.p1(0);
        } else {
            d.p2(16384u16 + equip_id as u16);
        }
    }

    let equip = input.equipment;

    // Slot → body-kit index (matching 2004scape getAppearanceInSlot).
    // Slots without a body-kit map to -1.
    const SLOT_TO_BODY: [i8; 12] = [
        -1, // 0  hat
        -1, // 1  cape
        -1, // 2  amulet
        -1, // 3  weapon
        2,  // 4  torso   → body[2]
        -1, // 5  shield
        3,  // 6  arms    → body[3]
        5,  // 7  legs    → body[5]
        0,  // 8  head    → body[0] (hair)
        4,  // 9  hands   → body[4]
        6,  // 10 feet    → body[6]
        1,  // 11 jaw     → body[1] (beard)
    ];

    for slot in 0..12usize {
        if input.skipped_slots[slot] {
            data.p1(0);
        } else if equip[slot] > 0 {
            put_equip(&mut data, equip[slot]);
        } else {
            let body_idx = SLOT_TO_BODY[slot];
            if body_idx >= 0 {
                put_kit(&mut data, input.body[body_idx as usize]);
            } else {
                data.p1(0);
            }
        }
    }

    data.p2(0); // extra / aura slot

    for &c in &input.colors {
        data.p1(c);
    }

    data.p2(input.render_anim);

    data.pjstr(input.display_name, 0);

    data.p1(input.combat_level);
    data.p1(0); // summoning combat level
    data.p1(0xFF); // unknown
    data.p1(0); // npc transmog

    // SUBTRACT length prefix on the front of the cached block.
    let bytes = &data.data[..data.pos];
    let bytes_len = bytes.len();
    let mut out = rs_io::Packet::new(bytes_len + 1);
    out.p1_alt2(bytes_len as u8);
    out.pdata(bytes, 0, bytes_len);
    out.data[..out.pos].to_vec()
}
