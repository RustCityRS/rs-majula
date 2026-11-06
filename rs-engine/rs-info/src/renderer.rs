use crate::EntityMasks;
use rs_grid::CoordGrid;
use rs_io::Packet;
use rs_protocol::network::game::info_prot::{NpcInfoProt, PlayerInfoProt};
use std::mem;

const MAX_PLAYERS: usize = 2048;
#[cfg(before_254)]
const MAX_NPCS: usize = 8192;
#[cfg(all(since_254, before_647))]
const MAX_NPCS: usize = 16384;
#[cfg(since_647)]
const MAX_NPCS: usize = 32768;

#[cfg(rev = "225")]
const PLAYER_PROT_COUNT: usize = 8;
#[cfg(all(since_244, before_647))]
const PLAYER_PROT_COUNT: usize = 9;
#[cfg(since_647)]
const PLAYER_PROT_COUNT: usize = 10;

#[cfg(rev = "225")]
const NPC_PROT_COUNT: usize = 7;
#[cfg(all(since_244, before_647))]
const NPC_PROT_COUNT: usize = 8; // +1 for the Damage2 fixed slot
#[cfg(since_647)]
const NPC_PROT_COUNT: usize = 7;

#[cfg(before_647)]
const SLOT_SIZE: usize = 8;
#[cfg(since_647)]
const SLOT_SIZE: usize = 16;
#[cfg(since_647)]
const DEFAULT_MOVE_TYPE: u8 = 1;

#[derive(Clone, Copy)]
#[repr(C)]
struct Slot {
    data: [u8; SLOT_SIZE],
    len: u8,
}

impl Slot {
    const EMPTY: Self = Self {
        data: [0; SLOT_SIZE],
        len: 0,
    };

    /// Checks whether this slot contains serialized data.
    ///
    /// A slot is considered set when its `len` field is greater than zero,
    /// meaning one of the `set_*` methods has been called to store a value.
    ///
    /// # Returns
    ///
    /// `true` if this slot holds data (`len > 0`), `false` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `PlayerRenderer::has`, `NpcRenderer::has` for
    /// fixed-size protocol fields.
    #[inline(always)]
    const fn is_set(&self) -> bool {
        self.len > 0
    }

    /// Returns the serialized bytes this slot holds (`data[0..len]`).
    ///
    /// Used when pre-coalescing an entity's high-definition update block so
    /// the bytes are guaranteed identical to what [`Slot::write_to`] emits.
    #[inline(always)]
    fn bytes(&self) -> &[u8] {
        unsafe { self.data.get_unchecked(0..self.len as usize) }
    }

    /// Writes the serialized slot data into a packet buffer.
    ///
    /// Copies exactly `self.len` bytes from the internal `data` array into
    /// `buf` via `Packet::pdata`. This should only be called on slots where
    /// `is_set()` returns `true`; calling it on an empty slot writes zero
    /// bytes harmlessly.
    ///
    /// # Arguments
    ///
    /// * `buf` - The target packet buffer to append data into.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `PlayerRenderer::write`, `NpcRenderer::write` when
    /// dispatching fixed-size protocol fields.
    ///
    /// **Calls:** `Packet::pdata`.
    #[inline(always)]
    fn write_to(&self, buf: &mut Packet) {
        buf.pdata(&self.data, 0, self.len as usize);
    }

    /// Stores a single 2-byte big-endian value into the slot.
    ///
    /// Sets `len` to 2 and writes the big-endian representation of `a` into
    /// `data[0..2]`.
    ///
    /// # Arguments
    ///
    /// * `a` - The 16-bit unsigned value to serialize.
    ///
    /// # Side Effects
    ///
    /// Overwrites the first 2 bytes of `data` and sets `len` to 2.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `PlayerRenderer::compute_info` (FaceEntity),
    /// `NpcRenderer::compute_info` (FaceEntity, ChangeType),
    /// `PlayerRenderer::cache_face_entity`, `NpcRenderer::cache_face_entity`.
    /// # Safety
    ///
    /// The `data` buffer is 8 bytes and all writes stay within bounds (max 6 bytes).
    /// `#[repr(C)]` guarantees `data` is at offset 0 so pointer arithmetic is valid.
    #[inline(always)]
    fn set_p2(&mut self, a: u16) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p2(a);
        self.len = buf.pos as u8;
    }

    /// Stores a 2-byte big-endian value followed by a 1-byte value into the slot.
    ///
    /// # Arguments
    ///
    /// * `a` - The 16-bit unsigned value to serialize first (e.g. animation id).
    /// * `b` - The 8-bit unsigned value to serialize second (e.g. animation delay).
    ///
    /// # Safety
    ///
    /// All writes are within the 8-byte `data` buffer.
    #[inline(always)]
    #[cfg(before_647)]
    fn set_p2_p1(&mut self, a: u16, b: u8) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p2(a);
        buf.p1(b);
        self.len = buf.pos as u8;
    }

    /// Stores four individual bytes into the slot.
    ///
    /// # Arguments
    ///
    /// * `a` - First byte (e.g. damage taken).
    /// * `b` - Second byte (e.g. damage type).
    /// * `c` - Third byte (e.g. current hitpoints).
    /// * `d` - Fourth byte (e.g. base hitpoints).
    ///
    /// # Safety
    ///
    /// All writes are within the 8-byte `data` buffer.
    #[inline(always)]
    #[cfg(before_647)]
    fn set_p1_p1_p1_p1(&mut self, a: u8, b: u8, c: u8, d: u8) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p1(a);
        buf.p1(b);
        buf.p1(c);
        buf.p1(d);
        self.len = buf.pos as u8;
    }

    /// Stores two 2-byte big-endian values into the slot.
    ///
    /// # Arguments
    ///
    /// * `a` - First 16-bit value (e.g. face x coordinate).
    /// * `b` - Second 16-bit value (e.g. face z coordinate).
    ///
    /// # Safety
    ///
    /// All writes are within the 8-byte `data` buffer.
    #[inline(always)]
    #[cfg(before_647)]
    fn set_p2_p2(&mut self, a: u16, b: u16) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p2(a);
        buf.p2(b);
        self.len = buf.pos as u8;
    }

    /// Stores a 2-byte big-endian value followed by a 4-byte big-endian value.
    ///
    /// # Arguments
    ///
    /// * `a` - The 16-bit unsigned value (e.g. spot-anim id).
    /// * `b` - The 32-bit signed value (e.g. packed spot-anim height and delay).
    ///
    /// # Safety
    ///
    /// All writes are within the 8-byte `data` buffer.
    #[inline(always)]
    #[cfg(before_647)]
    fn set_p2_p4(&mut self, a: u16, b: i32) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p2(a);
        buf.p4(b);
        self.len = buf.pos as u8;
    }

    #[cfg(since_647)]
    #[inline(always)]
    fn set_p2_ip2(&mut self, x: u16, z: u16) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p2(x);
        buf.p2_alt1(z);
        self.len = buf.pos as u8;
    }

    #[cfg(since_647)]
    #[inline(always)]
    fn set_p1(&mut self, a: u8) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p1(a);
        self.len = buf.pos as u8;
    }

    #[cfg(since_647)]
    #[inline(always)]
    fn set_p2_alt2(&mut self, a: u16) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p2_alt2(a);
        self.len = buf.pos as u8;
    }

    #[cfg(since_647)]
    #[inline(always)]
    fn set_p2_alt3(&mut self, a: u16) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p2_alt3(a);
        self.len = buf.pos as u8;
    }

    #[cfg(since_647)]
    #[inline(always)]
    fn set_ip2(&mut self, a: u16) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p2_alt1(a);
        self.len = buf.pos as u8;
    }

    #[cfg(since_647)]
    #[inline(always)]
    fn set_p1_alt2(&mut self, a: u8) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p1_alt2(a);
        self.len = buf.pos as u8;
    }

    #[cfg(since_647)]
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    fn set_exactmove(
        &mut self,
        start_x: i8,
        start_z: i8,
        end_x: i8,
        end_z: i8,
        begin: u16,
        finish: u16,
        dir: u8,
    ) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p1_alt3(start_x as u8);
        buf.p1(start_z as u8);
        buf.p1_alt2(end_x as u8);
        buf.p1_alt3(end_z as u8);
        buf.p2_alt2(begin);
        buf.p2(finish);
        buf.p1_alt2(dir);
        self.len = buf.pos as u8;
    }

    #[cfg(since_647)]
    #[inline(always)]
    fn set_hits(&mut self, count: u8, hits: &[Hit]) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p1(count);
        for hit in hits {
            buf.psmart1or2(hit.hitmark);
            buf.psmart1or2(hit.damage);
            buf.psmart1or2(0);
            buf.p1(hit.health_percent);
        }
        self.len = buf.pos as u8;
    }

    #[cfg(since_647)]
    #[inline(always)]
    fn set_player_anim(&mut self, anim: u16, delay: u8) {
        let mut buf = Packet::from(&mut self.data[..]);
        for _ in 0..4 {
            buf.p2(anim);
        }
        buf.p1_alt1(delay);
        self.len = buf.pos as u8;
    }

    #[cfg(since_647)]
    #[inline(always)]
    fn set_npc_anim(&mut self, anim: u16, delay: u8) {
        let mut buf = Packet::from(&mut self.data[..]);
        for _ in 0..4 {
            buf.p2_alt2(anim);
        }
        buf.p1_alt2(delay);
        self.len = buf.pos as u8;
    }

    #[cfg(since_647)]
    #[inline(always)]
    fn set_player_spotanim(&mut self, id: u16, height: u16, delay: u16) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p2_alt2(id);
        buf.p4_alt3(((height as i32) << 16) | delay as i32);
        buf.p1_alt3(15 << 3);
        self.len = buf.pos as u8;
    }

    #[cfg(since_647)]
    #[inline(always)]
    fn set_npc_spotanim(&mut self, id: u16, height: u16, delay: u16) {
        let mut buf = Packet::from(&mut self.data[..]);
        buf.p2_alt3(id);
        buf.p4_alt3(((height as i32) << 16) | delay as i32);
        buf.p1_alt1(15 << 3);
        self.len = buf.pos as u8;
    }
}

#[cfg(since_647)]
fn face_angle(src_x: i32, src_z: i32, dst_x: i32, dst_z: i32) -> u16 {
    let dx = src_x - dst_x;
    let dz = src_z - dst_z;
    if dx == 0 && dz == 0 {
        return 0;
    }
    let angle = (f64::atan2(dx as f64, dz as f64) * 2607.5945876176133) as i32;
    (angle & 0x3FFF) as u16
}

#[cfg(since_647)]
#[derive(Clone, Copy)]
struct Hit {
    hitmark: i32,
    damage: i32,
    health_percent: u8,
}

#[cfg(since_647)]
fn health_percent(current: Option<u8>, base: Option<u8>) -> u8 {
    match (current, base) {
        (Some(current), Some(base)) if base > 0 => {
            ((current as u32 * 255) / (base as u32)).min(255) as u8
        }
        _ => 0,
    }
}

#[cfg(since_647)]
fn hits(info: &EntityMasks, first: bool, second: bool) -> ([Hit; 2], usize) {
    let mut out = [Hit {
        hitmark: 0,
        damage: 0,
        health_percent: 0,
    }; 2];
    let mut n = 0;
    if first {
        out[n] = Hit {
            hitmark: info.damage_type.unwrap_or(0) as i32,
            damage: info.damage_taken.unwrap_or(0) as i32,
            health_percent: health_percent(info.damage_current, info.damage_base),
        };
        n += 1;
    }
    if second {
        out[n] = Hit {
            hitmark: info.damage2_type.unwrap_or(0) as i32,
            damage: info.damage2_taken.unwrap_or(0) as i32,
            health_percent: health_percent(info.damage2_current, info.damage2_base),
        };
        n += 1;
    }
    (out, n)
}

/// Pre-computed player info update storage for efficient network serialization.
///
/// `PlayerRenderer` maintains a set of 2048 player slots, one per possible
/// player index, where each slot holds pre-serialized protocol field data.
/// Fixed-size fields (Anim, FaceEntity, Damage, FaceCoord, SpotAnim) are
/// stored in inline [`Slot`] buffers, while variable-length fields
/// (Appearance, Say, Chat) use heap-allocated `Vec<u8>` buffers.
///
/// Two accumulated byte-size counters (`highs` and `lows`) track the total
/// serialized size of high-definition and low-definition updates per player,
/// enabling the info output phase to perform capacity checks before writing.
///
/// # Call Stack
///
/// **Called by:** Owned by `PlayerInfo` in `rs-engine/src/info.rs`. Methods
/// are invoked during the info phase (`rs-engine/src/phases/info.rs`),
/// player info output (`rs-engine/src/info.rs`), cleanup phase
/// (`rs-engine/src/phases/cleanup.rs`), and player logout
/// (`rs-engine/src/engine.rs`).
pub struct PlayerRenderer {
    fixed: Box<[[Slot; MAX_PLAYERS]; PLAYER_PROT_COUNT]>,
    appearances: Vec<Option<Vec<u8>>>,
    says: Vec<Option<Vec<u8>>>,
    #[cfg(before_647)]
    chats: Vec<Option<Vec<u8>>>,
    #[cfg(since_647)]
    move_types: Box<[u8; MAX_PLAYERS]>,
    high_blocks: Vec<Vec<u8>>,
    highs: Box<[u16; MAX_PLAYERS]>,
    lows: Box<[u16; MAX_PLAYERS]>,
    written: Vec<u16>,
}

impl PlayerRenderer {
    /// Creates a new `PlayerRenderer` with all slots initialized to empty.
    ///
    /// Allocates the fixed-size slot arrays on the heap via `Box`, and
    /// initializes the variable-length appearance, say, and chat vectors
    /// with `None` for all 2048 player indices. All high-def and low-def
    /// byte-size counters start at zero.
    ///
    /// # Returns
    ///
    /// A fully initialized `PlayerRenderer` with no pre-computed data.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine` constructor in `rs-engine/src/engine.rs`.
    #[inline]
    pub fn new() -> PlayerRenderer {
        PlayerRenderer {
            fixed: unsafe { Box::new_zeroed().assume_init() },
            appearances: vec![None; MAX_PLAYERS],
            says: vec![None; MAX_PLAYERS],
            #[cfg(before_647)]
            chats: vec![None; MAX_PLAYERS],
            #[cfg(since_647)]
            move_types: Box::new([DEFAULT_MOVE_TYPE; MAX_PLAYERS]),
            high_blocks: vec![Vec::new(); MAX_PLAYERS],
            highs: Box::new([0; MAX_PLAYERS]),
            lows: Box::new([0; MAX_PLAYERS]),
            written: Vec::new(),
        }
    }

    /// Pre-computes and serializes all active info masks for a player.
    ///
    /// Iterates over every flag set in `info.masks` and serializes the
    /// corresponding field data into the appropriate slot or variable-length
    /// buffer for the given player index. Also accumulates the total
    /// high-definition and low-definition byte sizes (including the mask
    /// header) into `self.highs[pid]` and `self.lows[pid]`.
    ///
    /// Returns immediately if `info.masks` is zero (no updates pending).
    ///
    /// # Arguments
    ///
    /// * `pid` - The player index (0..2048) identifying which slot to populate.
    /// * `info` - The entity masks containing the raw field values to serialize.
    ///
    /// # Side Effects
    ///
    /// Writes serialized data into the fixed slots and/or variable-length
    /// buffers for `pid`. Updates `self.highs[pid]` and `self.lows[pid]`
    /// with the total byte sizes. Variable-length buffers (appearance, say,
    /// chat) are reused if already allocated, avoiding repeated heap
    /// allocation.
    ///
    /// # Panics
    ///
    /// Panics if required `Option` fields in `info` are `None` when their
    /// corresponding mask bit is set (e.g. `info.anim_delay` when Anim mask
    /// is active).
    ///
    /// # Call Stack
    ///
    /// **Called by:** Info phase in `rs-engine/src/phases/info.rs`.
    ///
    /// **Calls:** `Slot::set_p2`, `Slot::set_p2_p1`, `Slot::set_p1_p1_p1_p1`,
    /// `Slot::set_p2_p2`, `Slot::set_p2_p4`, `Self::header`.
    /// # Safety
    ///
    /// `pid` must be < `MAX_PLAYERS` (2048). This is guaranteed by the engine
    /// which only passes valid player indices from `active_players`.
    #[inline]
    pub fn compute_info(
        &mut self,
        pid: usize,
        info: &EntityMasks,
        #[cfg_attr(before_647, allow(unused_variables))] coord: CoordGrid,
    ) {
        let masks = info.masks;

        if masks == 0 {
            return;
        }

        self.written.push(pid as u16);
        unsafe { self.compute_info_inner(pid, masks, info, coord) }
    }

    #[inline(always)]
    unsafe fn compute_info_inner(
        &mut self,
        pid: usize,
        masks: u16,
        info: &EntityMasks,
        #[cfg_attr(before_647, allow(unused_variables))] coord: CoordGrid,
    ) {
        unsafe {
            let mut active: u16 = 0;

            if masks & PlayerInfoProt::Appearance as u16 != 0 {
                let bytes = info.last_appearance_info.as_ref().unwrap();
                let slot = self
                    .appearances
                    .get_unchecked_mut(pid)
                    .get_or_insert_with(|| Vec::with_capacity(bytes.len() + 1));
                slot.clear();
                slot.push(if cfg!(since_647) {
                    128u8.wrapping_sub(bytes.len() as u8)
                } else {
                    bytes.len() as u8
                });
                slot.extend_from_slice(bytes);
                active |= 1 << PlayerInfoProt::Appearance.to_index();
            }
            if masks & PlayerInfoProt::Anim as u16 != 0 {
                let slot = self
                    .fixed
                    .get_unchecked_mut(PlayerInfoProt::Anim.to_index())
                    .get_unchecked_mut(pid);
                #[cfg(before_647)]
                slot.set_p2_p1(info.anim_id.unwrap_or(u16::MAX), info.anim_delay.unwrap());
                #[cfg(since_647)]
                slot.set_player_anim(
                    info.anim_id.unwrap_or(u16::MAX),
                    info.anim_delay.unwrap_or(0),
                );
                active |= 1 << PlayerInfoProt::Anim.to_index();
            }
            if masks & PlayerInfoProt::FaceEntity as u16 != 0 {
                let slot = self
                    .fixed
                    .get_unchecked_mut(PlayerInfoProt::FaceEntity.to_index())
                    .get_unchecked_mut(pid);
                #[cfg(before_647)]
                slot.set_p2(info.face_entity.unwrap_or(u16::MAX));
                #[cfg(since_647)]
                slot.set_p2_alt3(info.face_entity.unwrap_or(u16::MAX));
                active |= 1 << PlayerInfoProt::FaceEntity.to_index();
            }
            if masks & PlayerInfoProt::Say as u16 != 0 {
                let bytes = info.say.as_ref().unwrap().as_bytes();
                let slot = self
                    .says
                    .get_unchecked_mut(pid)
                    .get_or_insert_with(|| Vec::with_capacity(bytes.len() + 1));
                slot.clear();
                slot.extend_from_slice(bytes);
                slot.push(if cfg!(since_647) { 0 } else { 10 });
                active |= 1 << PlayerInfoProt::Say.to_index();
            }
            #[cfg(before_647)]
            if masks & PlayerInfoProt::Damage as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(PlayerInfoProt::Damage.to_index())
                    .get_unchecked_mut(pid)
                    .set_p1_p1_p1_p1(
                        info.damage_taken.unwrap() as u8,
                        info.damage_type.unwrap(),
                        info.damage_current.unwrap(),
                        info.damage_base.unwrap(),
                    );
                active |= 1 << PlayerInfoProt::Damage.to_index();
            }
            if masks & PlayerInfoProt::FaceCoord as u16 != 0 {
                let slot = self
                    .fixed
                    .get_unchecked_mut(PlayerInfoProt::FaceCoord.to_index())
                    .get_unchecked_mut(pid);
                #[cfg(before_647)]
                slot.set_p2_p2(info.face_x.unwrap(), info.face_z.unwrap());
                #[cfg(since_647)]
                {
                    let src_x = CoordGrid::fine(coord.x(), 1) as i32;
                    let src_z = CoordGrid::fine(coord.z(), 1) as i32;
                    let angle = match (info.face_x, info.face_z) {
                        (Some(x), Some(z)) => face_angle(src_x, src_z, x as i32, z as i32),
                        _ => 0,
                    };
                    slot.set_p2_alt2(angle);
                }
                active |= 1 << PlayerInfoProt::FaceCoord.to_index();
            }
            #[cfg(before_647)]
            if masks & PlayerInfoProt::Chat as u16 != 0 {
                let chat = info.chat_bytes.as_ref().unwrap();
                let slot = self
                    .chats
                    .get_unchecked_mut(pid)
                    .get_or_insert_with(|| Vec::with_capacity(chat.len() + 4));
                slot.clear();
                slot.extend_from_slice(&[
                    info.chat_colour.unwrap(),
                    info.chat_effects.unwrap(),
                    info.chat_ignored.unwrap(),
                    chat.len() as u8,
                ]);
                slot.extend_from_slice(chat);
                active |= 1 << PlayerInfoProt::Chat.to_index();
            }
            if masks & PlayerInfoProt::SpotAnim as u16 != 0 {
                let slot = self
                    .fixed
                    .get_unchecked_mut(PlayerInfoProt::SpotAnim.to_index())
                    .get_unchecked_mut(pid);
                #[cfg(before_647)]
                slot.set_p2_p4(
                    info.spotanim.unwrap(),
                    ((info.spotanim_height.unwrap() as i32) << 16)
                        | info.spotanim_delay.unwrap() as i32,
                );
                #[cfg(since_647)]
                slot.set_player_spotanim(
                    info.spotanim.unwrap_or(u16::MAX),
                    info.spotanim_height.unwrap_or(0),
                    info.spotanim_delay.unwrap_or(0),
                );
                active |= 1 << PlayerInfoProt::SpotAnim.to_index();
            }
            #[cfg(all(since_244, before_647))]
            if masks & PlayerInfoProt::Damage2 as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(PlayerInfoProt::Damage2.to_index())
                    .get_unchecked_mut(pid)
                    .set_p1_p1_p1_p1(
                        info.damage2_taken.unwrap() as u8,
                        info.damage2_type.unwrap(),
                        info.damage2_current.unwrap(),
                        info.damage2_base.unwrap(),
                    );
                active |= 1 << PlayerInfoProt::Damage2.to_index();
            }
            #[cfg(since_647)]
            {
                if masks & PlayerInfoProt::MoveType as u16 != 0 {
                    let move_type = info.move_type.unwrap_or(0);
                    self.move_types[pid] = move_type;
                    self.fixed[PlayerInfoProt::MoveType.to_index()][pid].set_p1(move_type);
                    active |= 1 << PlayerInfoProt::MoveType.to_index();
                }
                if masks & PlayerInfoProt::TempMoveType as u16 != 0 {
                    self.fixed[PlayerInfoProt::TempMoveType.to_index()][pid]
                        .set_p1_alt2(info.temp_move_type.unwrap_or(0));
                    active |= 1 << PlayerInfoProt::TempMoveType.to_index();
                }
                if masks & PlayerInfoProt::ExactMove as u16 != 0 {
                    // The client adds these to the player's own tile, where
                    // the move has already put him (`Class244`, mask 0x4000):
                    // tile offsets from `coord`, not world coordinates.
                    let offset = |tile: Option<u16>, here: u16| {
                        (i32::from(tile.unwrap_or(here)) - i32::from(here)) as i8
                    };
                    self.fixed[PlayerInfoProt::ExactMove.to_index()][pid].set_exactmove(
                        offset(info.exactmove_start_x, coord.x()),
                        offset(info.exactmove_start_z, coord.z()),
                        offset(info.exactmove_end_x, coord.x()),
                        offset(info.exactmove_end_z, coord.z()),
                        info.exactmove_begin.unwrap_or(0),
                        info.exactmove_finish.unwrap_or(0),
                        info.exactmove_dir.unwrap_or(0),
                    );
                    active |= 1 << PlayerInfoProt::ExactMove.to_index();
                }
                let first = masks & PlayerInfoProt::Damage as u16 != 0;
                let second = masks & PlayerInfoProt::Damage2 as u16 != 0;
                if first || second {
                    let (hits, count) = hits(info, first, second);
                    self.fixed[PlayerInfoProt::Damage.to_index()][pid]
                        .set_hits(count as u8, &hits[..count]);
                    active |= 1 << PlayerInfoProt::Damage.to_index();
                }
            }

            let block = self.high_blocks.get_unchecked_mut(pid);
            block.clear();
            #[cfg(before_647)]
            {
                let wire = if masks > 0xff {
                    masks | PlayerInfoProt::BigInfo as u16
                } else {
                    masks
                };
                block.extend_from_slice(&wire.to_le_bytes()[..Self::header(masks) as usize]);
            }
            #[cfg(since_647)]
            if active != 0 {
                let (header, len) = PlayerInfoProt::header(PlayerInfoProt::mask(masks));
                block.extend_from_slice(&header[..len]);
            }
            for index in 0..PLAYER_PROT_COUNT {
                if active & (1 << index) == 0 {
                    continue;
                }
                let bytes = match index {
                    i if i == PlayerInfoProt::Appearance.to_index() => self
                        .appearances
                        .get_unchecked(pid)
                        .as_deref()
                        .unwrap_or_default(),
                    i if i == PlayerInfoProt::Say.to_index() => {
                        self.says.get_unchecked(pid).as_deref().unwrap_or_default()
                    }
                    #[cfg(before_647)]
                    i if i == PlayerInfoProt::Chat.to_index() => {
                        self.chats.get_unchecked(pid).as_deref().unwrap_or_default()
                    }
                    _ => self.fixed.get_unchecked(index).get_unchecked(pid).bytes(),
                };
                block.extend_from_slice(bytes);
            }
            #[cfg(before_647)]
            if active != 0 || masks & PlayerInfoProt::ExactMove as u16 != 0 {
                self.highs[pid] = block.len() as u16
                    + if masks & PlayerInfoProt::ExactMove as u16 != 0 {
                        9
                    } else {
                        0
                    };
            }
            #[cfg(since_647)]
            {
                self.highs[pid] = block.len() as u16;
            }

            if masks
                & (PlayerInfoProt::Appearance as u16
                    | PlayerInfoProt::FaceEntity as u16
                    | PlayerInfoProt::FaceCoord as u16)
                != 0
            {
                let appearance_len = self
                    .appearances
                    .get_unchecked(pid)
                    .as_ref()
                    .map_or(0, |v| v.len()) as u16;
                self.lows[pid] = 1 + appearance_len + 2 + if cfg!(since_647) { 2 + 1 } else { 4 };
            }
        }
    }

    /// Returns the precomputed high-definition block for a player.
    /// Before 647, the encoder appends observer-relative `ExactMove` data.
    #[inline(always)]
    pub fn high_block(&self, id: u16) -> &[u8] {
        unsafe { self.high_blocks.get_unchecked(id as usize) }
    }

    /// Writes the pre-computed data for a specific protocol field to a packet buffer.
    ///
    /// Dispatches on `prot` to select the correct storage: variable-length
    /// buffers for Appearance, Say, and Chat, or the inline fixed slot for
    /// all other protocol types. The raw bytes are copied directly into
    /// `buf` without further encoding.
    ///
    /// # Arguments
    ///
    /// * `buf` - The target packet buffer to write into.
    /// * `id` - The player index identifying which slot to read from.
    /// * `prot` - The protocol field type to write.
    ///
    /// # Side Effects
    ///
    /// Appends bytes to `buf`. Does not modify any renderer state.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Player info output in `rs-engine/src/info.rs`
    /// (`write_blocks`).
    ///
    /// **Calls:** `Packet::pdata`, `Slot::write_to`.
    /// # Safety
    ///
    /// `id` must be < `MAX_PLAYERS` and `prot.to_index()` < `PLAYER_PROT_COUNT`.
    #[inline(always)]
    pub fn write(&self, buf: &mut Packet, id: u16, prot: PlayerInfoProt) {
        let idx = id as usize;
        unsafe {
            match prot {
                PlayerInfoProt::Appearance => {
                    if let Some(bytes) = self.appearances.get_unchecked(idx) {
                        buf.pdata(bytes, 0, bytes.len());
                    }
                }
                PlayerInfoProt::Say => {
                    if let Some(bytes) = self.says.get_unchecked(idx) {
                        buf.pdata(bytes, 0, bytes.len());
                    }
                }
                #[cfg(before_647)]
                PlayerInfoProt::Chat => {
                    if let Some(bytes) = self.chats.get_unchecked(idx) {
                        buf.pdata(bytes, 0, bytes.len());
                    }
                }
                #[cfg(since_647)]
                PlayerInfoProt::BigInfo | PlayerInfoProt::BigInfo2 => {}
                _ => {
                    self.fixed
                        .get_unchecked(prot.to_index())
                        .get_unchecked(idx)
                        .write_to(buf);
                }
            }
        }
    }

    /// Writes exact-move data inline to a packet buffer.
    ///
    /// Unlike other protocol fields, exact-move is not pre-computed into a
    /// slot because its 9-byte payload exceeds the 8-byte `Slot` capacity.
    /// Instead, the caller provides all values and they are written directly
    /// into the packet using individual `p1` and `p2` calls.
    ///
    /// # Arguments
    ///
    /// * `buf` - The target packet buffer to write into.
    /// * `start_x` - Starting x coordinate delta (1 byte).
    /// * `start_z` - Starting z coordinate delta (1 byte).
    /// * `end_x` - Ending x coordinate delta (1 byte).
    /// * `end_z` - Ending z coordinate delta (1 byte).
    /// * `begin` - Start tick of the movement (2 bytes).
    /// * `finish` - End tick of the movement (2 bytes).
    /// * `dir` - Direction/angle of movement (1 byte).
    ///
    /// # Side Effects
    ///
    /// Appends 9 bytes to `buf`. Does not modify any renderer state.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Player info output in `rs-engine/src/info.rs`
    /// (`write_blocks`).
    ///
    /// **Calls:** `Packet::p1`, `Packet::p2`.
    #[allow(clippy::too_many_arguments)]
    #[inline]
    #[cfg(before_647)]
    pub fn write_exactmove(
        &self,
        buf: &mut Packet,
        start_x: u8,
        start_z: u8,
        end_x: u8,
        end_z: u8,
        begin: u16,
        finish: u16,
        dir: u8,
    ) {
        buf.p1(start_x);
        buf.p1(start_z);
        buf.p1(end_x);
        buf.p1(end_z);
        buf.p2(begin);
        buf.p2(finish);
        buf.p1(dir);
    }

    /// Checks whether pre-computed data exists for a specific protocol field.
    ///
    /// For variable-length fields (Appearance, Say, Chat), checks that the
    /// `Option<Vec<u8>>` is `Some` and non-empty. For fixed-size fields,
    /// delegates to `Slot::is_set`.
    ///
    /// # Arguments
    ///
    /// * `id` - The player index to check.
    /// * `prot` - The protocol field type to check.
    ///
    /// # Returns
    ///
    /// `true` if serialized data is available for the given player and
    /// protocol, `false` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Player info output in `rs-engine/src/info.rs`
    /// (low-definition update checks).
    ///
    /// **Calls:** `Slot::is_set`.
    #[inline(always)]
    pub fn has(&self, id: u16, prot: PlayerInfoProt) -> bool {
        unsafe {
            match prot {
                PlayerInfoProt::Appearance => self
                    .appearances
                    .get_unchecked(id as usize)
                    .as_ref()
                    .is_some_and(|v| !v.is_empty()),
                PlayerInfoProt::Say => self
                    .says
                    .get_unchecked(id as usize)
                    .as_ref()
                    .is_some_and(|v| !v.is_empty()),
                #[cfg(before_647)]
                PlayerInfoProt::Chat => self
                    .chats
                    .get_unchecked(id as usize)
                    .as_ref()
                    .is_some_and(|v| !v.is_empty()),
                #[cfg(since_647)]
                PlayerInfoProt::BigInfo | PlayerInfoProt::BigInfo2 => false,
                _ => self
                    .fixed
                    .get_unchecked(prot.to_index())
                    .get_unchecked(id as usize)
                    .is_set(),
            }
        }
    }

    /// Caches a face-entity value for low-definition updates.
    ///
    /// Writes the entity target into the FaceEntity slot so it persists
    /// across ticks for players entering the viewport who need the cached
    /// low-definition state. This is separate from `compute_info`, which
    /// handles the per-tick high-definition update.
    ///
    /// # Arguments
    ///
    /// * `id` - The player index to update.
    /// * `entity` - The entity id the player is facing, or `u16::MAX` for none.
    ///
    /// # Side Effects
    ///
    /// Overwrites the FaceEntity slot for the given player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Player info low-definition output in
    /// `rs-engine/src/info.rs`.
    ///
    /// **Calls:** `Slot::set_p2`.
    #[inline(always)]
    pub fn cache_face_entity(&mut self, id: u16, entity: u16) {
        self.written.push(id);
        let slot = &mut self.fixed[PlayerInfoProt::FaceEntity.to_index()][id as usize];
        #[cfg(before_647)]
        slot.set_p2(entity);
        #[cfg(since_647)]
        slot.set_p2_alt3(entity);
    }

    /// Caches face-coordinate values for low-definition updates.
    ///
    /// # Arguments
    ///
    /// * `id` - The player index to update.
    /// * `x` - The x coordinate the player is facing toward.
    /// * `z` - The z coordinate the player is facing toward.
    ///
    /// # Safety
    ///
    /// `id` must be < `MAX_PLAYERS`.
    #[inline(always)]
    #[cfg(before_647)]
    pub fn cache_face_coord(&mut self, id: u16, x: u16, z: u16) {
        self.written.push(id);
        unsafe {
            let slot = self
                .fixed
                .as_mut_ptr()
                .add(PlayerInfoProt::FaceCoord.to_index());
            (*slot)
                .as_mut_ptr()
                .add(id as usize)
                .as_mut()
                .unwrap_unchecked()
                .set_p2_p2(x, z);
        }
    }

    #[cfg(since_647)]
    #[inline(always)]
    pub fn cache_face_coord(&mut self, id: u16, x: u16, z: u16, coord: CoordGrid) {
        self.written.push(id);
        let angle = face_angle(
            CoordGrid::fine(coord.x(), 1) as i32,
            CoordGrid::fine(coord.z(), 1) as i32,
            x as i32,
            z as i32,
        );
        self.fixed[PlayerInfoProt::FaceCoord.to_index()][id as usize].set_p2_alt2(angle);
    }

    /// Returns the total pre-computed high-definition byte size for a player.
    ///
    /// # Arguments
    ///
    /// * `id` - The player index to query.
    ///
    /// # Returns
    ///
    /// The total byte count for a high-definition update, or zero if none pending.
    #[inline(always)]
    pub const fn highdefinitions(&self, id: u16) -> usize {
        unsafe { *self.highs.as_ptr().add(id as usize) as usize }
    }

    /// Returns the total pre-computed low-definition byte size for a player.
    ///
    /// This includes the mask header plus Appearance, FaceEntity, and
    /// FaceCoord field sizes. A return value of zero means no low-definition
    /// update is pending for this player.
    ///
    /// # Arguments
    ///
    /// * `id` - The player index to query.
    ///
    /// # Returns
    ///
    /// The total byte count that would be written for a low-definition
    /// update, or zero if none is pending.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Player info output in `rs-engine/src/info.rs`
    /// (capacity checks before writing low-def blocks).
    #[inline(always)]
    pub const fn lowdefinitions(&self, id: u16) -> usize {
        unsafe { *self.lows.as_ptr().add(id as usize) as usize }
    }

    /// Clears per-tick temporary data for every player written this tick.
    ///
    /// Drains the internal `written` list (player indices recorded by
    /// `compute_info` and `cache_face_*`) and, for each entry, resets the
    /// high-definition byte-size counter and clears every temporary
    /// protocol slot (Anim, FaceEntity, Damage, FaceCoord, SpotAnim) back
    /// to `Slot::EMPTY`. Also clears the variable-length Say and Chat
    /// buffers. Appearance data and low-definition counters are preserved
    /// because they persist across ticks. Players that were never written
    /// this tick cost nothing.
    ///
    /// # Side Effects
    ///
    /// Zeroes `self.highs` and resets fixed slots and variable-length
    /// buffers for every written index; empties `self.written`. Does not
    /// deallocate variable-length buffers; they are cleared in place for
    /// reuse.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Cleanup phase in `rs-engine/src/phases/cleanup.rs`.
    #[inline]
    pub fn remove_temporary(&mut self) {
        let written = mem::take(&mut self.written);
        for &pid in &written {
            let idx = pid as usize;
            unsafe {
                *self.highs.get_unchecked_mut(idx) = 0;
                *self
                    .fixed
                    .get_unchecked_mut(PlayerInfoProt::Anim.to_index())
                    .get_unchecked_mut(idx) = Slot::EMPTY;
                *self
                    .fixed
                    .get_unchecked_mut(PlayerInfoProt::FaceEntity.to_index())
                    .get_unchecked_mut(idx) = Slot::EMPTY;
                *self
                    .fixed
                    .get_unchecked_mut(PlayerInfoProt::Damage.to_index())
                    .get_unchecked_mut(idx) = Slot::EMPTY;
                *self
                    .fixed
                    .get_unchecked_mut(PlayerInfoProt::FaceCoord.to_index())
                    .get_unchecked_mut(idx) = Slot::EMPTY;
                *self
                    .fixed
                    .get_unchecked_mut(PlayerInfoProt::SpotAnim.to_index())
                    .get_unchecked_mut(idx) = Slot::EMPTY;
                #[cfg(since_647)]
                for prot in [
                    PlayerInfoProt::MoveType,
                    PlayerInfoProt::TempMoveType,
                    PlayerInfoProt::ExactMove,
                ] {
                    self.fixed[prot.to_index()][idx] = Slot::EMPTY;
                }
                if let Some(v) = self.says.get_unchecked_mut(idx) {
                    v.clear();
                }
                #[cfg(before_647)]
                if let Some(v) = self.chats.get_unchecked_mut(idx) {
                    v.clear();
                }
                self.high_blocks.get_unchecked_mut(idx).clear();
            }
        }
        self.written = written;
        self.written.clear();
    }

    /// Performs a full cleanup of all renderer data for a player.
    ///
    /// Called when a player logs out or is otherwise permanently removed
    /// from the game. Resets both byte-size counters to zero and drops
    /// the appearance buffer by setting it to `None`, freeing its heap
    /// allocation.
    ///
    /// # Arguments
    ///
    /// * `id` - The player index to fully clean up.
    ///
    /// # Side Effects
    ///
    /// Zeroes `self.highs[id]` and `self.lows[id]`. Drops the appearance
    /// `Vec` for the given index.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Player logout handling in `rs-engine/src/engine.rs`.
    #[inline]
    pub fn remove_permanent(&mut self, id: u16) {
        let idx = id as usize;
        #[cfg(since_647)]
        {
            self.move_types[idx] = DEFAULT_MOVE_TYPE;
        }
        unsafe {
            *self.highs.get_unchecked_mut(idx) = 0;
            *self.lows.get_unchecked_mut(idx) = 0;
            *self.appearances.get_unchecked_mut(idx) = None;
        }
    }

    /// Computes the mask header size based on the combined mask value.
    ///
    /// If the mask value fits in a single byte (0x00..0xFF), the header is
    /// 1 byte. Otherwise, 2 bytes are needed to encode the full mask.
    ///
    /// # Arguments
    ///
    /// * `masks` - The combined bitmask of all active protocol flags.
    ///
    /// # Returns
    ///
    /// `1` if `masks` fits in a single byte, `2` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Self::compute_info` when computing total byte sizes.
    #[inline(always)]
    #[cfg(before_647)]
    const fn header(masks: u16) -> u16 {
        if masks > 0xff { 2 } else { 1 }
    }

    #[cfg(since_647)]
    #[inline(always)]
    pub fn cache_move_type(&mut self, id: u16) {
        let move_type = self.move_types[id as usize];
        let slot = &mut self.fixed[PlayerInfoProt::MoveType.to_index()][id as usize];
        if !slot.is_set() {
            self.written.push(id);
        }
        slot.set_p1(move_type);
    }
}

/// Pre-computed NPC info update storage for efficient network serialization.
///
/// `NpcRenderer` maintains a set of NPC slots, one per possible NPC
/// index, where each slot holds pre-serialized protocol field data. Fixed-size
/// fields (Anim, FaceEntity, Damage, ChangeType, SpotAnim, FaceCoord) are
/// stored in inline [`Slot`] buffers, while the variable-length Say field
/// uses a heap-allocated `Vec<u8>` buffer.
///
/// A byte-size counter (`highs`) tracks the serialized size of each NPC's
/// high-definition update so the info output phase can check capacity
/// before adding it to the observer's packet.
///
/// This mirrors [`PlayerRenderer`] but with NPC-specific protocol types
/// and a larger index space.
///
/// # Call Stack
///
/// **Called by:** Owned by `NpcInfo` in `rs-engine/src/info.rs`. Methods
/// are invoked during the info phase (`rs-engine/src/phases/info.rs`),
/// NPC info output (`rs-engine/src/info.rs`), cleanup phase
/// (`rs-engine/src/phases/cleanup.rs`), and NPC despawn/removal
/// (`rs-engine/src/engine.rs`).
pub struct NpcRenderer {
    fixed: Box<[[Slot; MAX_NPCS]; NPC_PROT_COUNT]>,
    says: Vec<Option<Vec<u8>>>,
    high_blocks: Vec<Vec<u8>>,
    highs: Box<[u16; MAX_NPCS]>,
    written: Vec<u16>,
}

impl NpcRenderer {
    /// Creates a new `NpcRenderer` with all slots initialized to empty.
    ///
    /// Allocates the fixed-size slot arrays on the heap via `Box`, and
    /// initializes the variable-length say vector with `None` for all
    /// NPC indices. All high-def byte-size counters start at zero.
    ///
    /// # Returns
    ///
    /// A fully initialized `NpcRenderer` with no pre-computed data.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine` constructor in `rs-engine/src/engine.rs`.
    #[inline]
    pub fn new() -> NpcRenderer {
        NpcRenderer {
            fixed: unsafe { Box::new_zeroed().assume_init() },
            says: vec![None; MAX_NPCS],
            high_blocks: vec![Vec::new(); MAX_NPCS],
            highs: Box::new([0; MAX_NPCS]),
            written: Vec::new(),
        }
    }

    /// Pre-computes and serializes all active info masks for an NPC.
    ///
    /// Iterates over every flag set in `info.masks` and serializes the
    /// corresponding field data into the appropriate slot or variable-length
    /// buffer for the given NPC index. Also accumulates the total
    /// high-definition byte size (including the mask header) into
    /// `self.highs[nid]`.
    ///
    /// Returns immediately if `info.masks` is zero (no updates pending).
    ///
    /// # Arguments
    ///
    /// * `nid` - The NPC index identifying which slot to populate.
    /// * `info` - The entity masks containing the raw field values to serialize.
    ///
    /// # Side Effects
    ///
    /// Writes serialized data into the fixed slots and/or variable-length
    /// buffers for `nid`. Updates `self.highs[nid]` with the total
    /// byte size. The say buffer is reused if already
    /// allocated, avoiding repeated heap allocation.
    ///
    /// # Panics
    ///
    /// Panics if required `Option` fields in `info` are `None` when their
    /// corresponding mask bit is set (e.g. `info.anim_delay` when Anim mask
    /// is active).
    ///
    /// # Call Stack
    ///
    /// **Called by:** Info phase in `rs-engine/src/phases/info.rs`.
    ///
    /// **Calls:** `Slot::set_p2`, `Slot::set_p2_p1`, `Slot::set_p1_p1_p1_p1`,
    /// `Slot::set_p2_p2`, `Slot::set_p2_p4`, `Self::header`.
    /// # Safety
    ///
    /// `nid` must be < `MAX_NPCS`. This is guaranteed by the engine
    /// which only passes valid NPC indices from `active_npcs`.
    #[inline]
    pub fn compute_info(&mut self, nid: usize, info: &EntityMasks) {
        let masks = info.masks;

        if masks == 0 {
            return;
        }

        self.written.push(nid as u16);
        unsafe { self.compute_info_inner(nid, masks, info) }
    }

    #[inline(always)]
    unsafe fn compute_info_inner(&mut self, nid: usize, masks: u16, info: &EntityMasks) {
        unsafe {
            let mut active: u16 = 0;

            if masks & NpcInfoProt::Anim as u16 != 0 {
                let slot = self
                    .fixed
                    .get_unchecked_mut(NpcInfoProt::Anim.to_index())
                    .get_unchecked_mut(nid);
                #[cfg(before_647)]
                slot.set_p2_p1(info.anim_id.unwrap_or(u16::MAX), info.anim_delay.unwrap());
                #[cfg(since_647)]
                slot.set_npc_anim(
                    info.anim_id.unwrap_or(u16::MAX),
                    info.anim_delay.unwrap_or(0),
                );
                active |= 1 << NpcInfoProt::Anim.to_index();
            }
            if masks & NpcInfoProt::FaceEntity as u16 != 0 {
                let slot = self
                    .fixed
                    .get_unchecked_mut(NpcInfoProt::FaceEntity.to_index())
                    .get_unchecked_mut(nid);
                #[cfg(before_647)]
                slot.set_p2(info.face_entity.unwrap_or(u16::MAX));
                #[cfg(since_647)]
                slot.set_ip2(info.face_entity.unwrap_or(u16::MAX));
                active |= 1 << NpcInfoProt::FaceEntity.to_index();
            }
            if masks & NpcInfoProt::Say as u16 != 0 {
                let bytes = info.say.as_ref().unwrap().as_bytes();
                let slot = self
                    .says
                    .get_unchecked_mut(nid)
                    .get_or_insert_with(|| Vec::with_capacity(bytes.len() + 1));
                slot.clear();
                slot.extend_from_slice(bytes);
                slot.push(if cfg!(since_647) { 0 } else { 10 });
                active |= 1 << NpcInfoProt::Say.to_index();
            }
            #[cfg(before_647)]
            if masks & NpcInfoProt::Damage as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(NpcInfoProt::Damage.to_index())
                    .get_unchecked_mut(nid)
                    .set_p1_p1_p1_p1(
                        info.damage_taken.unwrap() as u8,
                        info.damage_type.unwrap(),
                        info.damage_current.unwrap(),
                        info.damage_base.unwrap(),
                    );
                active |= 1 << NpcInfoProt::Damage.to_index();
            }
            if masks & NpcInfoProt::ChangeType as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(NpcInfoProt::ChangeType.to_index())
                    .get_unchecked_mut(nid)
                    .set_p2(info.changetype.unwrap());
                active |= 1 << NpcInfoProt::ChangeType.to_index();
            }
            if masks & NpcInfoProt::SpotAnim as u16 != 0 {
                let slot = self
                    .fixed
                    .get_unchecked_mut(NpcInfoProt::SpotAnim.to_index())
                    .get_unchecked_mut(nid);
                #[cfg(before_647)]
                slot.set_p2_p4(
                    info.spotanim.unwrap(),
                    ((info.spotanim_height.unwrap() as i32) << 16)
                        | info.spotanim_delay.unwrap() as i32,
                );
                #[cfg(since_647)]
                slot.set_npc_spotanim(
                    info.spotanim.unwrap_or(u16::MAX),
                    info.spotanim_height.unwrap_or(0),
                    info.spotanim_delay.unwrap_or(0),
                );
                active |= 1 << NpcInfoProt::SpotAnim.to_index();
            }
            if masks & NpcInfoProt::FaceCoord as u16 != 0 {
                let slot = self
                    .fixed
                    .get_unchecked_mut(NpcInfoProt::FaceCoord.to_index())
                    .get_unchecked_mut(nid);
                #[cfg(before_647)]
                slot.set_p2_p2(info.face_x.unwrap(), info.face_z.unwrap());
                #[cfg(since_647)]
                slot.set_p2_ip2(info.face_x.unwrap(), info.face_z.unwrap());
                active |= 1 << NpcInfoProt::FaceCoord.to_index();
            }
            #[cfg(all(since_244, before_647))]
            if masks & NpcInfoProt::Damage2 as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(NpcInfoProt::Damage2.to_index())
                    .get_unchecked_mut(nid)
                    .set_p1_p1_p1_p1(
                        info.damage2_taken.unwrap() as u8,
                        info.damage2_type.unwrap(),
                        info.damage2_current.unwrap(),
                        info.damage2_base.unwrap(),
                    );
                active |= 1 << NpcInfoProt::Damage2.to_index();
            }
            #[cfg(since_647)]
            {
                let first = masks & NpcInfoProt::Damage as u16 != 0;
                let second = masks & NpcInfoProt::Damage2 as u16 != 0;
                if first || second {
                    let (hits, count) = hits(info, first, second);
                    self.fixed[NpcInfoProt::Damage.to_index()][nid]
                        .set_hits((count as u8).wrapping_add(128), &hits[..count]);
                    active |= 1 << NpcInfoProt::Damage.to_index();
                }
            }

            let block = self.high_blocks.get_unchecked_mut(nid);
            block.clear();
            #[cfg(before_647)]
            block.push(masks as u8);
            #[cfg(since_647)]
            if active != 0 {
                let (header, len) = NpcInfoProt::header(NpcInfoProt::mask(masks));
                block.extend_from_slice(&header[..len]);
            }
            for index in 0..NPC_PROT_COUNT {
                if active & (1 << index) == 0 {
                    continue;
                }
                let bytes = if index == NpcInfoProt::Say.to_index() {
                    self.says.get_unchecked(nid).as_deref().unwrap_or_default()
                } else {
                    self.fixed.get_unchecked(index).get_unchecked(nid).bytes()
                };
                block.extend_from_slice(bytes);
            }
            if active != 0 {
                *self.highs.get_unchecked_mut(nid) = block.len() as u16;
            }
        }
    }

    /// Returns the pre-coalesced full high-definition block for an NPC (mask
    /// header + every field), written by the encoder with a single `pdata`.
    /// See [`compute_info`](Self::compute_info).
    #[inline(always)]
    pub fn high_block(&self, id: u16) -> &[u8] {
        unsafe { self.high_blocks.get_unchecked(id as usize) }
    }

    /// Writes the pre-computed data for a specific protocol field to a packet buffer.
    ///
    /// Dispatches on `prot` to select the correct storage: the variable-length
    /// buffer for Say, or the inline fixed slot for all other protocol types.
    /// The raw bytes are copied directly into `buf` without further encoding.
    ///
    /// # Arguments
    ///
    /// * `buf` - The target packet buffer to write into.
    /// * `id` - The NPC index identifying which slot to read from.
    /// * `prot` - The protocol field type to write.
    ///
    /// # Side Effects
    ///
    /// Appends bytes to `buf`. Does not modify any renderer state.
    ///
    /// # Call Stack
    ///
    /// **Called by:** NPC info output in `rs-engine/src/info.rs`
    /// (`write_blocks`).
    ///
    /// **Calls:** `Packet::pdata`, `Slot::write_to`.
    #[inline(always)]
    pub fn write(&self, buf: &mut Packet, id: u16, prot: NpcInfoProt) {
        unsafe {
            match prot {
                NpcInfoProt::Say => {
                    if let Some(bytes) = self.says.get_unchecked(id as usize) {
                        buf.pdata(bytes, 0, bytes.len());
                    }
                }
                #[cfg(since_647)]
                NpcInfoProt::BigInfo | NpcInfoProt::BigInfo2 => {}
                _ => {
                    self.fixed
                        .get_unchecked(prot.to_index())
                        .get_unchecked(id as usize)
                        .write_to(buf);
                }
            }
        }
    }

    /// Checks whether pre-computed data exists for a specific protocol field.
    ///
    /// For the variable-length Say field, checks that the `Option<Vec<u8>>`
    /// is `Some` and non-empty. For all fixed-size fields, delegates to
    /// `Slot::is_set`.
    ///
    /// # Arguments
    ///
    /// * `id` - The NPC index to check.
    /// * `prot` - The protocol field type to check.
    ///
    /// # Returns
    ///
    /// `true` if serialized data is available for the given NPC and
    /// protocol, `false` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** NPC info output in `rs-engine/src/info.rs`
    /// (low-definition update checks).
    ///
    /// **Calls:** `Slot::is_set`.
    #[inline(always)]
    pub fn has(&self, id: u16, prot: NpcInfoProt) -> bool {
        unsafe {
            match prot {
                NpcInfoProt::Say => self
                    .says
                    .get_unchecked(id as usize)
                    .as_ref()
                    .is_some_and(|v| !v.is_empty()),
                #[cfg(since_647)]
                NpcInfoProt::BigInfo | NpcInfoProt::BigInfo2 => false,
                _ => self
                    .fixed
                    .get_unchecked(prot.to_index())
                    .get_unchecked(id as usize)
                    .is_set(),
            }
        }
    }

    /// Caches a face-entity value for low-definition updates.
    ///
    /// Writes the entity target into the FaceEntity slot so it persists
    /// across ticks for NPCs entering the viewport who need the cached
    /// low-definition state. This is separate from `compute_info`, which
    /// handles the per-tick high-definition update.
    ///
    /// # Arguments
    ///
    /// * `id` - The NPC index to update.
    /// * `entity` - The entity id the NPC is facing, or `u16::MAX` for none.
    ///
    /// # Side Effects
    ///
    /// Overwrites the FaceEntity slot for the given NPC.
    ///
    /// # Call Stack
    ///
    /// **Called by:** NPC info low-definition output in
    /// `rs-engine/src/info.rs`.
    ///
    /// **Calls:** `Slot::set_p2`.
    #[inline(always)]
    pub fn cache_face_entity(&mut self, id: u16, entity: u16) {
        self.written.push(id);
        let slot = &mut self.fixed[NpcInfoProt::FaceEntity.to_index()][id as usize];
        #[cfg(before_647)]
        slot.set_p2(entity);
        #[cfg(since_647)]
        slot.set_ip2(entity);
    }

    /// Caches face-coordinate values for low-definition NPC updates.
    ///
    /// # Arguments
    ///
    /// * `id` - The NPC index to update.
    /// * `x` - The x coordinate the NPC is facing toward.
    /// * `z` - The z coordinate the NPC is facing toward.
    ///
    /// # Side Effects
    ///
    /// Overwrites the FaceCoord slot for the given NPC.
    ///
    /// # Call Stack
    ///
    /// **Called by:** NPC info low-definition output in
    /// `rs-engine/src/info.rs`.
    ///
    #[inline(always)]
    pub fn cache_face_coord(&mut self, id: u16, x: u16, z: u16) {
        self.written.push(id);
        let slot = &mut self.fixed[NpcInfoProt::FaceCoord.to_index()][id as usize];
        #[cfg(before_647)]
        slot.set_p2_p2(x, z);
        #[cfg(since_647)]
        slot.set_p2_ip2(x, z);
    }

    #[inline(always)]
    pub const fn highdefinitions(&self, id: u16) -> usize {
        unsafe { *self.highs.as_ptr().add(id as usize) as usize }
    }

    /// The most a low-definition update can be: the face coord is always
    /// sent, and the face entity with it when the NPC has one, under one
    /// mask header.
    pub const LOW_LEN: usize = 1 + 2 + 4;

    /// Clears per-tick temporary data for every NPC written this tick.
    ///
    /// Drains the internal `written` list (NPC indices recorded by
    /// `compute_info` and `cache_face_*`) and, for each entry, resets the
    /// high-definition byte-size counter and clears every temporary
    /// protocol slot (Anim, FaceEntity, Damage, ChangeType, SpotAnim,
    /// FaceCoord) back to `Slot::EMPTY`. Also clears the variable-length
    /// Say buffer. NPCs that were never written this tick cost nothing.
    ///
    /// # Side Effects
    ///
    /// Zeroes `self.highs` and resets fixed slots and variable-length
    /// buffers for every written index; empties `self.written`. Does not
    /// deallocate variable-length buffers; they are cleared in place for
    /// reuse.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Cleanup phase in `rs-engine/src/phases/cleanup.rs`.
    #[inline]
    pub fn remove_temporary(&mut self) {
        let written = mem::take(&mut self.written);
        for &nid in &written {
            let idx = nid as usize;
            unsafe {
                *self.highs.get_unchecked_mut(idx) = 0;
                *self
                    .fixed
                    .get_unchecked_mut(NpcInfoProt::Anim.to_index())
                    .get_unchecked_mut(idx) = Slot::EMPTY;
                *self
                    .fixed
                    .get_unchecked_mut(NpcInfoProt::FaceEntity.to_index())
                    .get_unchecked_mut(idx) = Slot::EMPTY;
                if let Some(v) = self.says.get_unchecked_mut(idx) {
                    v.clear();
                }
                *self
                    .fixed
                    .get_unchecked_mut(NpcInfoProt::Damage.to_index())
                    .get_unchecked_mut(idx) = Slot::EMPTY;
                *self
                    .fixed
                    .get_unchecked_mut(NpcInfoProt::ChangeType.to_index())
                    .get_unchecked_mut(idx) = Slot::EMPTY;
                *self
                    .fixed
                    .get_unchecked_mut(NpcInfoProt::SpotAnim.to_index())
                    .get_unchecked_mut(idx) = Slot::EMPTY;
                *self
                    .fixed
                    .get_unchecked_mut(NpcInfoProt::FaceCoord.to_index())
                    .get_unchecked_mut(idx) = Slot::EMPTY;
                self.high_blocks.get_unchecked_mut(idx).clear();
            }
        }
        self.written = written;
        self.written.clear();
    }

    /// Performs a full cleanup of all renderer data for an NPC.
    ///
    /// Called when an NPC despawns or is otherwise permanently removed
    /// from the game. Resets the byte-size counter to zero. Unlike
    /// `PlayerRenderer::remove_permanent`, no variable-length buffer
    /// is dropped here because NPCs do not have persistent appearance
    /// data.
    ///
    /// # Arguments
    ///
    /// * `id` - The NPC index to fully clean up.
    ///
    /// # Side Effects
    ///
    /// Zeroes `self.highs[id]`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** NPC despawn/removal handling in
    /// `rs-engine/src/engine.rs`.
    #[inline]
    pub fn remove_permanent(&mut self, id: u16) {
        let idx = id as usize;
        #[cfg(since_647)]
        {
            self.says[idx] = None;
        }
        unsafe {
            *self.highs.get_unchecked_mut(idx) = 0;
        }
    }
}

#[cfg(all(test, since_647))]
mod move_type_tests {
    use super::*;

    fn move_type_block(r: &PlayerRenderer, pid: u16) -> Option<u8> {
        r.has(pid, PlayerInfoProt::MoveType)
            .then(|| r.fixed[PlayerInfoProt::MoveType.to_index()][pid as usize].data[0])
    }

    #[test]
    fn an_added_player_carries_the_move_type_sent_ticks_earlier() {
        let mut r = PlayerRenderer::new();
        let mut info = EntityMasks::new();
        info.masks = PlayerInfoProt::MoveType as u16;
        info.move_type = Some(2);
        r.compute_info(7, &info, CoordGrid::new(3222, 0, 3218));
        assert_eq!(move_type_block(&r, 7), Some(2));

        r.remove_temporary();
        assert_eq!(move_type_block(&r, 7), None, "the tick's block is gone");

        r.cache_move_type(7);
        assert_eq!(
            move_type_block(&r, 7),
            Some(2),
            "and the add still says run"
        );
    }

    #[test]
    fn a_slot_freed_by_logout_goes_back_to_walk() {
        let mut r = PlayerRenderer::new();
        let mut info = EntityMasks::new();
        info.masks = PlayerInfoProt::MoveType as u16;
        info.move_type = Some(2);
        r.compute_info(7, &info, CoordGrid::new(3222, 0, 3218));
        r.remove_temporary();
        r.remove_permanent(7);
        r.cache_move_type(7);
        assert_eq!(move_type_block(&r, 7), Some(DEFAULT_MOVE_TYPE));
    }

    #[test]
    fn observer_caches_do_not_change_the_ticks_update_block() {
        let mut player = PlayerRenderer::new();
        let mut npc = NpcRenderer::new();
        let coord = CoordGrid::new(3222, 0, 3218);
        let mut info = EntityMasks::new();
        info.last_appearance_info = Some(vec![1, 2, 3].into_boxed_slice());
        info.masks = PlayerInfoProt::Appearance as u16;
        player.compute_info(7, &info, coord);
        player.remove_temporary();

        info.masks = PlayerInfoProt::MoveType as u16;
        info.move_type = Some(2);
        player.compute_info(7, &info, coord);
        assert_eq!(player.high_block(7), &[0x8, 2]);
        player.cache_face_coord(7, 6445, 6437, coord);
        player.cache_face_entity(7, 42);
        player.cache_move_type(7);
        assert_eq!(player.high_block(7), &[0x8, 2]);
        assert_eq!(player.highdefinitions(7), 2);
        assert!(player.has(7, PlayerInfoProt::Appearance));

        info.masks = NpcInfoProt::ChangeType as u16;
        info.changetype = Some(4231);
        npc.compute_info(9, &info);
        assert_eq!(npc.high_block(9), &[0x2, 0x10, 0x87]);
        npc.cache_face_coord(9, 6445, 6437);
        npc.cache_face_entity(9, 42);
        assert_eq!(npc.high_block(9), &[0x2, 0x10, 0x87]);
        assert_eq!(npc.highdefinitions(9), 3);

        player.remove_temporary();
        npc.remove_temporary();
        assert!(player.high_block(7).is_empty());
        assert!(npc.high_block(9).is_empty());
        assert!(player.has(7, PlayerInfoProt::Appearance));
    }
}

#[cfg(all(test, since_647))]
mod npc_spotanim_tests {
    use super::*;

    #[test]
    fn npc_spotanim_roundtrips_through_client_decode() {
        let mut slot = Slot::EMPTY;
        slot.set_npc_spotanim(74, 92, 0);
        assert_eq!(slot.len, 7, "spotanim block is 7 bytes (2+4+1)");

        let b = &slot.data[..7];
        assert_eq!(b, &[202, 0, 92, 0, 0, 0, 248], "exact spotanim wire bytes");

        let lo = b[0].wrapping_sub(128) as u16;
        let hi = b[1] as u16;
        assert_eq!((hi << 8) | lo, 74, "id round-trips");

        let g4 =
            ((b[2] as u32) << 16) | ((b[3] as u32) << 24) | (b[4] as u32) | ((b[5] as u32) << 8);
        assert_eq!(g4 >> 16, 92, "height = g4 >> 16");
        assert_eq!(g4 & 0xFFFF, 0, "delay = g4 & 0xFFFF");

        let g1 = b[6].wrapping_sub(128);
        assert_eq!(g1 & 0x7, 0, "type = 0");
        assert_eq!((g1 >> 3) & 0xF, 15, "label = 15 (no attachment)");
    }
}
