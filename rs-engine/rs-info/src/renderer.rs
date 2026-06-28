use crate::EntityMasks;
use rs_io::Packet;
use rs_protocol::network::game::info_prot::{NpcInfoProt, PlayerInfoProt};

const MAX_PLAYERS: usize = 2048;
#[cfg(before_254)]
const MAX_NPCS: usize = 8192;
#[cfg(since_254)]
const MAX_NPCS: usize = 16384;

#[cfg(rev = "225")]
const PLAYER_PROT_COUNT: usize = 8;
#[cfg(since_244)]
const PLAYER_PROT_COUNT: usize = 9; // +1 for the Damage2 fixed slot

#[cfg(rev = "225")]
const NPC_PROT_COUNT: usize = 7;
#[cfg(since_244)]
const NPC_PROT_COUNT: usize = 8; // +1 for the Damage2 fixed slot

/// An 8-byte inline buffer that stores a single pre-serialized protocol field
/// in big-endian format.
///
/// `Slot` avoids heap allocation for fixed-size info-update fields (animations,
/// face targets, damage, spot-anims, etc.) by keeping the encoded bytes and
/// their length directly on the stack. Variable-length fields such as
/// appearance, chat, and say data are stored separately in `Vec` buffers.
///
/// # Side Effects
///
/// None -- `Slot` is a pure data container with no external side effects.
#[derive(Clone, Copy)]
#[repr(C)]
struct Slot {
    data: [u8; 8],
    len: u8,
}

impl Slot {
    const EMPTY: Self = Self {
        data: [0; 8],
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
    const fn set_p2(&mut self, a: u16) {
        unsafe {
            let ptr = self.data.as_mut_ptr();
            core::ptr::write_unaligned(ptr as *mut u16, a.to_be());
        }
        self.len = 2;
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
    const fn set_p2_p1(&mut self, a: u16, b: u8) {
        unsafe {
            let ptr = self.data.as_mut_ptr();
            core::ptr::write_unaligned(ptr as *mut u16, a.to_be());
            core::ptr::write(ptr.add(2), b);
        }
        self.len = 3;
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
    const fn set_p1_p1_p1_p1(&mut self, a: u8, b: u8, c: u8, d: u8) {
        unsafe {
            let ptr = self.data.as_mut_ptr();
            core::ptr::write_unaligned(ptr as *mut u32, u32::from_ne_bytes([a, b, c, d]));
        }
        self.len = 4;
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
    const fn set_p2_p2(&mut self, a: u16, b: u16) {
        unsafe {
            let ptr = self.data.as_mut_ptr();
            core::ptr::write_unaligned(
                ptr as *mut u32,
                u32::from_ne_bytes([(a >> 8) as u8, a as u8, (b >> 8) as u8, b as u8]),
            );
        }
        self.len = 4;
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
    const fn set_p2_p4(&mut self, a: u16, b: i32) {
        unsafe {
            let ptr = self.data.as_mut_ptr();
            core::ptr::write_unaligned(ptr as *mut u16, a.to_be());
            core::ptr::write_unaligned(ptr.add(2) as *mut i32, b.to_be());
        }
        self.len = 6;
    }
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
    chats: Vec<Option<Vec<u8>>>,
    high_blocks: Vec<Vec<u8>>,
    highs: Box<[u16; MAX_PLAYERS]>,
    lows: Box<[u16; MAX_PLAYERS]>,
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
            fixed: Box::new([[Slot::EMPTY; MAX_PLAYERS]; PLAYER_PROT_COUNT]),
            appearances: vec![None; MAX_PLAYERS],
            says: vec![None; MAX_PLAYERS],
            chats: vec![None; MAX_PLAYERS],
            high_blocks: vec![Vec::new(); MAX_PLAYERS],
            highs: Box::new([0; MAX_PLAYERS]),
            lows: Box::new([0; MAX_PLAYERS]),
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
    pub fn compute_info(&mut self, pid: usize, info: &EntityMasks) {
        let masks = info.masks;

        if masks == 0 {
            return;
        }

        unsafe { self.compute_info_inner(pid, masks, info) }
    }

    #[inline(always)]
    unsafe fn compute_info_inner(&mut self, pid: usize, masks: u16, info: &EntityMasks) {
        unsafe {
            let mut highs: u16 = 0;
            let mut lows: u16 = 0;

            if masks & PlayerInfoProt::Appearance as u16 != 0 {
                let bytes = info.last_appearance_info.as_ref().unwrap();
                let len = 1 + bytes.len();
                let slot = self.appearances.get_unchecked_mut(pid);
                match slot {
                    Some(v) => {
                        v.clear();
                        v.push(bytes.len() as u8);
                        v.extend_from_slice(bytes);
                    }
                    None => {
                        let mut buf = Vec::with_capacity(len);
                        buf.push(bytes.len() as u8);
                        buf.extend_from_slice(bytes);
                        *slot = Some(buf);
                    }
                }
                highs += len as u16;
                lows += len as u16;
            }
            if masks & PlayerInfoProt::Anim as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(PlayerInfoProt::Anim.to_index())
                    .get_unchecked_mut(pid)
                    .set_p2_p1(info.anim_id.unwrap_or(u16::MAX), info.anim_delay.unwrap());
                highs += 3;
            }
            if masks & PlayerInfoProt::FaceEntity as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(PlayerInfoProt::FaceEntity.to_index())
                    .get_unchecked_mut(pid)
                    .set_p2(info.face_entity.unwrap_or(u16::MAX));
                highs += 2;
                lows += 2;
            }
            if masks & PlayerInfoProt::Say as u16 != 0 {
                let say = info.say.as_ref().unwrap();
                let bytes = say.as_bytes();
                let len = bytes.len() + 1;
                let slot = self.says.get_unchecked_mut(pid);
                match slot {
                    Some(v) => {
                        v.clear();
                        v.extend_from_slice(bytes);
                        v.push(10);
                    }
                    None => {
                        let mut buf = Vec::with_capacity(len);
                        buf.extend_from_slice(bytes);
                        buf.push(10);
                        *slot = Some(buf);
                    }
                }
                highs += len as u16;
            }
            if masks & PlayerInfoProt::Damage as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(PlayerInfoProt::Damage.to_index())
                    .get_unchecked_mut(pid)
                    .set_p1_p1_p1_p1(
                        info.damage_taken.unwrap(),
                        info.damage_type.unwrap(),
                        info.damage_current.unwrap(),
                        info.damage_base.unwrap(),
                    );
                highs += 4;
            }
            if masks & PlayerInfoProt::FaceCoord as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(PlayerInfoProt::FaceCoord.to_index())
                    .get_unchecked_mut(pid)
                    .set_p2_p2(info.face_x.unwrap(), info.face_z.unwrap());
                highs += 4;
                lows += 4;
            }
            if masks & PlayerInfoProt::Chat as u16 != 0 {
                let chat = info.chat_bytes.as_ref().unwrap();
                let len = 4 + chat.len();
                let slot = self.chats.get_unchecked_mut(pid);
                match slot {
                    Some(v) => {
                        v.clear();
                        v.push(info.chat_colour.unwrap());
                        v.push(info.chat_effects.unwrap());
                        v.push(info.chat_ignored.unwrap());
                        v.push(chat.len() as u8);
                        v.extend_from_slice(chat);
                    }
                    None => {
                        let mut buf = Vec::with_capacity(len);
                        buf.push(info.chat_colour.unwrap());
                        buf.push(info.chat_effects.unwrap());
                        buf.push(info.chat_ignored.unwrap());
                        buf.push(chat.len() as u8);
                        buf.extend_from_slice(chat);
                        *slot = Some(buf);
                    }
                }
                highs += len as u16;
            }
            if masks & PlayerInfoProt::SpotAnim as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(PlayerInfoProt::SpotAnim.to_index())
                    .get_unchecked_mut(pid)
                    .set_p2_p4(
                        info.spotanim.unwrap(),
                        ((info.spotanim_height.unwrap() as i32) << 16)
                            | info.spotanim_delay.unwrap() as i32,
                    );
                highs += 6;
            }
            if masks & PlayerInfoProt::ExactMove as u16 != 0 {
                highs += 9;
            }
            #[cfg(since_244)]
            if masks & PlayerInfoProt::Damage2 as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(PlayerInfoProt::Damage2.to_index())
                    .get_unchecked_mut(pid)
                    .set_p1_p1_p1_p1(
                        info.damage2_taken.unwrap(),
                        info.damage2_type.unwrap(),
                        info.damage2_current.unwrap(),
                        info.damage2_base.unwrap(),
                    );
                highs += 4;
            }

            if highs > 0 {
                *self.highs.get_unchecked_mut(pid) = highs + Self::header(masks);
            }

            if lows > 0 {
                let header = Self::header(
                    PlayerInfoProt::Appearance as u16
                        | PlayerInfoProt::FaceEntity as u16
                        | PlayerInfoProt::FaceCoord as u16,
                );
                let appearance_len = self
                    .appearances
                    .get_unchecked(pid)
                    .as_ref()
                    .map_or(0, |v| v.len()) as u16;
                *self.lows.get_unchecked_mut(pid) = header + appearance_len + 2 + 4;
            }

            // Pre-coalesce the high-definition update block once per tick. This
            // is the exact byte sequence `PlayerInfo::write_blocks` would emit
            // for `masks`, EXCEPT the trailing `ExactMove` field, which depends
            // on the observing player's build-area origin and is appended by the
            // encoder per observer. The mask header is computed from the FULL
            // `masks` (including the ExactMove bit and the BigInfo rule), so it
            // is byte-identical whether or not ExactMove is appended.
            let blk = self.high_blocks.get_unchecked_mut(pid);
            blk.clear();
            if masks > 0xff {
                let v = masks | PlayerInfoProt::BigInfo as u16;
                blk.push(v as u8);
                blk.push((v >> 8) as u8);
            } else {
                blk.push(masks as u8);
            }
            if masks & PlayerInfoProt::Appearance as u16 != 0 {
                if let Some(bytes) = self.appearances.get_unchecked(pid) {
                    blk.extend_from_slice(bytes);
                }
            }
            if masks & PlayerInfoProt::Anim as u16 != 0 {
                blk.extend_from_slice(
                    self.fixed
                        .get_unchecked(PlayerInfoProt::Anim.to_index())
                        .get_unchecked(pid)
                        .bytes(),
                );
            }
            if masks & PlayerInfoProt::FaceEntity as u16 != 0 {
                blk.extend_from_slice(
                    self.fixed
                        .get_unchecked(PlayerInfoProt::FaceEntity.to_index())
                        .get_unchecked(pid)
                        .bytes(),
                );
            }
            if masks & PlayerInfoProt::Say as u16 != 0 {
                if let Some(bytes) = self.says.get_unchecked(pid) {
                    blk.extend_from_slice(bytes);
                }
            }
            if masks & PlayerInfoProt::Damage as u16 != 0 {
                blk.extend_from_slice(
                    self.fixed
                        .get_unchecked(PlayerInfoProt::Damage.to_index())
                        .get_unchecked(pid)
                        .bytes(),
                );
            }
            if masks & PlayerInfoProt::FaceCoord as u16 != 0 {
                blk.extend_from_slice(
                    self.fixed
                        .get_unchecked(PlayerInfoProt::FaceCoord.to_index())
                        .get_unchecked(pid)
                        .bytes(),
                );
            }
            if masks & PlayerInfoProt::Chat as u16 != 0 {
                if let Some(bytes) = self.chats.get_unchecked(pid) {
                    blk.extend_from_slice(bytes);
                }
            }
            if masks & PlayerInfoProt::SpotAnim as u16 != 0 {
                blk.extend_from_slice(
                    self.fixed
                        .get_unchecked(PlayerInfoProt::SpotAnim.to_index())
                        .get_unchecked(pid)
                        .bytes(),
                );
            }
            #[cfg(since_244)]
            if masks & PlayerInfoProt::Damage2 as u16 != 0 {
                blk.extend_from_slice(
                    self.fixed
                        .get_unchecked(PlayerInfoProt::Damage2.to_index())
                        .get_unchecked(pid)
                        .bytes(),
                );
            }
        }
    }

    /// Returns the pre-coalesced high-definition block prefix for a player:
    /// the mask header plus all fields except observer-relative `ExactMove`.
    ///
    /// The encoder writes this with a single `pdata`, then appends `ExactMove`
    /// itself if its mask bit is set. See [`compute_info`](Self::compute_info).
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
                PlayerInfoProt::Chat => {
                    if let Some(bytes) = self.chats.get_unchecked(idx) {
                        buf.pdata(bytes, 0, bytes.len());
                    }
                }
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
                PlayerInfoProt::Chat => self
                    .chats
                    .get_unchecked(id as usize)
                    .as_ref()
                    .is_some_and(|v| !v.is_empty()),
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
    pub const fn cache_face_entity(&mut self, id: u16, entity: u16) {
        unsafe {
            let slot = self
                .fixed
                .as_mut_ptr()
                .add(PlayerInfoProt::FaceEntity.to_index());
            (*slot)
                .as_mut_ptr()
                .add(id as usize)
                .as_mut()
                .unwrap_unchecked()
                .set_p2(entity);
        }
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
    pub const fn cache_face_coord(&mut self, id: u16, x: u16, z: u16) {
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

    /// Clears per-tick temporary data for all active players.
    ///
    /// Resets the high-definition byte-size counter and clears every
    /// temporary protocol slot (Anim, FaceEntity, Damage, FaceCoord,
    /// SpotAnim) back to `Slot::EMPTY`. Also clears the variable-length
    /// Say and Chat buffers. Appearance data and low-definition counters
    /// are preserved because they persist across ticks.
    ///
    /// # Arguments
    ///
    /// * `active` - Slice of player indices that were active this tick and
    ///   need their temporary data cleared.
    ///
    /// # Side Effects
    ///
    /// Zeroes `self.highs` and resets fixed slots and variable-length
    /// buffers for every index in `active`. Does not deallocate
    /// variable-length buffers; they are cleared in place for reuse.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Cleanup phase in `rs-engine/src/phases/cleanup.rs`.
    #[inline]
    pub fn remove_temporary(&mut self, active: &[u16]) {
        for &pid in active {
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
                if let Some(v) = self.says.get_unchecked_mut(idx) {
                    v.clear();
                }
                if let Some(v) = self.chats.get_unchecked_mut(idx) {
                    v.clear();
                }
                self.high_blocks.get_unchecked_mut(idx).clear();
            }
        }
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
    const fn header(masks: u16) -> u16 {
        if masks > 0xff { 2 } else { 1 }
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
/// Two accumulated byte-size counters (`highs` and `lows`) track the total
/// serialized size of high-definition and low-definition updates per NPC,
/// enabling the info output phase to perform capacity checks before writing.
///
/// This mirrors [`PlayerRenderer`] but with NPC-specific protocol types
/// (7 protocols instead of 8) and a larger index space.
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
    lows: Box<[u16; MAX_NPCS]>,
}

impl NpcRenderer {
    /// Creates a new `NpcRenderer` with all slots initialized to empty.
    ///
    /// Allocates the fixed-size slot arrays on the heap via `Box`, and
    /// initializes the variable-length say vector with `None` for all
    /// NPC indices. All high-def and low-def byte-size counters
    /// start at zero.
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
            fixed: Box::new([[Slot::EMPTY; MAX_NPCS]; NPC_PROT_COUNT]),
            says: vec![None; MAX_NPCS],
            high_blocks: vec![Vec::new(); MAX_NPCS],
            highs: Box::new([0; MAX_NPCS]),
            lows: Box::new([0; MAX_NPCS]),
        }
    }

    /// Pre-computes and serializes all active info masks for an NPC.
    ///
    /// Iterates over every flag set in `info.masks` and serializes the
    /// corresponding field data into the appropriate slot or variable-length
    /// buffer for the given NPC index. Also accumulates the total
    /// high-definition and low-definition byte sizes (including the mask
    /// header) into `self.highs[nid]` and `self.lows[nid]`.
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
    /// buffers for `nid`. Updates `self.highs[nid]` and `self.lows[nid]`
    /// with the total byte sizes. The say buffer is reused if already
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

        unsafe { self.compute_info_inner(nid, masks, info) }
    }

    #[inline(always)]
    unsafe fn compute_info_inner(&mut self, nid: usize, masks: u16, info: &EntityMasks) {
        unsafe {
            let mut highs: u16 = 0;
            let mut lows: u16 = 0;

            if masks & NpcInfoProt::Anim as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(NpcInfoProt::Anim.to_index())
                    .get_unchecked_mut(nid)
                    .set_p2_p1(info.anim_id.unwrap_or(u16::MAX), info.anim_delay.unwrap());
                highs += 3;
            }
            if masks & NpcInfoProt::FaceEntity as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(NpcInfoProt::FaceEntity.to_index())
                    .get_unchecked_mut(nid)
                    .set_p2(info.face_entity.unwrap_or(u16::MAX));
                highs += 2;
                lows += 2;
            }
            if masks & NpcInfoProt::Say as u16 != 0 {
                let say = info.say.as_ref().unwrap();
                let bytes = say.as_bytes();
                let len = bytes.len() + 1;
                let slot = self.says.get_unchecked_mut(nid);
                match slot {
                    Some(v) => {
                        v.clear();
                        v.extend_from_slice(bytes);
                        v.push(10);
                    }
                    None => {
                        let mut buf = Vec::with_capacity(len);
                        buf.extend_from_slice(bytes);
                        buf.push(10);
                        *slot = Some(buf);
                    }
                }
                highs += len as u16;
            }
            if masks & NpcInfoProt::Damage as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(NpcInfoProt::Damage.to_index())
                    .get_unchecked_mut(nid)
                    .set_p1_p1_p1_p1(
                        info.damage_taken.unwrap(),
                        info.damage_type.unwrap(),
                        info.damage_current.unwrap(),
                        info.damage_base.unwrap(),
                    );
                highs += 4;
            }
            if masks & NpcInfoProt::ChangeType as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(NpcInfoProt::ChangeType.to_index())
                    .get_unchecked_mut(nid)
                    .set_p2(info.changetype.unwrap());
                highs += 2;
            }
            if masks & NpcInfoProt::SpotAnim as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(NpcInfoProt::SpotAnim.to_index())
                    .get_unchecked_mut(nid)
                    .set_p2_p4(
                        info.spotanim.unwrap(),
                        ((info.spotanim_height.unwrap() as i32) << 16)
                            | info.spotanim_delay.unwrap() as i32,
                    );
                highs += 6;
            }
            if masks & NpcInfoProt::FaceCoord as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(NpcInfoProt::FaceCoord.to_index())
                    .get_unchecked_mut(nid)
                    .set_p2_p2(info.face_x.unwrap(), info.face_z.unwrap());
                highs += 4;
                lows += 4;
            }
            #[cfg(since_244)]
            if masks & NpcInfoProt::Damage2 as u16 != 0 {
                self.fixed
                    .get_unchecked_mut(NpcInfoProt::Damage2.to_index())
                    .get_unchecked_mut(nid)
                    .set_p1_p1_p1_p1(
                        info.damage2_taken.unwrap(),
                        info.damage2_type.unwrap(),
                        info.damage2_current.unwrap(),
                        info.damage2_base.unwrap(),
                    );
                highs += 4;
            }

            if highs > 0 {
                *self.highs.get_unchecked_mut(nid) = highs + Self::header(masks);
            }

            if lows > 0 {
                let header =
                    Self::header(NpcInfoProt::FaceEntity as u16 | NpcInfoProt::FaceCoord as u16);
                *self.lows.get_unchecked_mut(nid) = header + 2 + 4;
            }

            // Pre-coalesce the full high-definition update block once per tick.
            // NPC high-def blocks have no observer-relative fields, so the byte
            // sequence is identical for every observer and matches exactly what
            // `NpcInfo::write_blocks` would emit for `masks`. The mask header is
            // always a single byte (the max NPC mask 0xFE fits in 8 bits).
            let blk = self.high_blocks.get_unchecked_mut(nid);
            blk.clear();
            blk.push(masks as u8);
            #[cfg(since_244)]
            if masks & NpcInfoProt::Damage2 as u16 != 0 {
                blk.extend_from_slice(
                    self.fixed
                        .get_unchecked(NpcInfoProt::Damage2.to_index())
                        .get_unchecked(nid)
                        .bytes(),
                );
            }
            if masks & NpcInfoProt::Anim as u16 != 0 {
                blk.extend_from_slice(
                    self.fixed
                        .get_unchecked(NpcInfoProt::Anim.to_index())
                        .get_unchecked(nid)
                        .bytes(),
                );
            }
            if masks & NpcInfoProt::FaceEntity as u16 != 0 {
                blk.extend_from_slice(
                    self.fixed
                        .get_unchecked(NpcInfoProt::FaceEntity.to_index())
                        .get_unchecked(nid)
                        .bytes(),
                );
            }
            if masks & NpcInfoProt::Say as u16 != 0 {
                if let Some(bytes) = self.says.get_unchecked(nid) {
                    blk.extend_from_slice(bytes);
                }
            }
            if masks & NpcInfoProt::Damage as u16 != 0 {
                blk.extend_from_slice(
                    self.fixed
                        .get_unchecked(NpcInfoProt::Damage.to_index())
                        .get_unchecked(nid)
                        .bytes(),
                );
            }
            if masks & NpcInfoProt::ChangeType as u16 != 0 {
                blk.extend_from_slice(
                    self.fixed
                        .get_unchecked(NpcInfoProt::ChangeType.to_index())
                        .get_unchecked(nid)
                        .bytes(),
                );
            }
            if masks & NpcInfoProt::SpotAnim as u16 != 0 {
                blk.extend_from_slice(
                    self.fixed
                        .get_unchecked(NpcInfoProt::SpotAnim.to_index())
                        .get_unchecked(nid)
                        .bytes(),
                );
            }
            if masks & NpcInfoProt::FaceCoord as u16 != 0 {
                blk.extend_from_slice(
                    self.fixed
                        .get_unchecked(NpcInfoProt::FaceCoord.to_index())
                        .get_unchecked(nid)
                        .bytes(),
                );
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
    pub const fn cache_face_entity(&mut self, id: u16, entity: u16) {
        unsafe {
            let slot = self
                .fixed
                .as_mut_ptr()
                .add(NpcInfoProt::FaceEntity.to_index());
            (*slot)
                .as_mut_ptr()
                .add(id as usize)
                .as_mut()
                .unwrap_unchecked()
                .set_p2(entity);
        }
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
    pub const fn cache_face_coord(&mut self, id: u16, x: u16, z: u16) {
        unsafe {
            let slot = self
                .fixed
                .as_mut_ptr()
                .add(NpcInfoProt::FaceCoord.to_index());
            (*slot)
                .as_mut_ptr()
                .add(id as usize)
                .as_mut()
                .unwrap_unchecked()
                .set_p2_p2(x, z);
        }
    }

    #[inline(always)]
    pub const fn highdefinitions(&self, id: u16) -> usize {
        unsafe { *self.highs.as_ptr().add(id as usize) as usize }
    }

    /// Returns the total pre-computed low-definition byte size for an NPC.
    ///
    /// # Arguments
    ///
    /// * `id` - The NPC index to query.
    ///
    /// # Returns
    ///
    /// The total byte count for a low-definition update, or zero if none pending.
    #[inline(always)]
    pub const fn lowdefinitions(&self, id: u16) -> usize {
        unsafe { *self.lows.as_ptr().add(id as usize) as usize }
    }

    /// Clears per-tick temporary data for all active NPCs.
    ///
    /// Resets the high-definition byte-size counter and clears every
    /// temporary protocol slot (Anim, FaceEntity, Damage, ChangeType,
    /// SpotAnim, FaceCoord) back to `Slot::EMPTY`. Also clears the
    /// variable-length Say buffer. Low-definition counters are preserved
    /// because they persist across ticks.
    ///
    /// # Arguments
    ///
    /// * `active` - Slice of NPC indices that were active this tick and
    ///   need their temporary data cleared.
    ///
    /// # Side Effects
    ///
    /// Zeroes `self.highs` and resets fixed slots and variable-length
    /// buffers for every index in `active`. Does not deallocate
    /// variable-length buffers; they are cleared in place for reuse.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Cleanup phase in `rs-engine/src/phases/cleanup.rs`.
    #[inline]
    pub fn remove_temporary(&mut self, active: &[u16]) {
        for &nid in active {
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
    }

    /// Performs a full cleanup of all renderer data for an NPC.
    ///
    /// Called when an NPC despawns or is otherwise permanently removed
    /// from the game. Resets both byte-size counters to zero. Unlike
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
    /// Zeroes `self.highs[id]` and `self.lows[id]`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** NPC despawn/removal handling in
    /// `rs-engine/src/engine.rs`.
    #[inline]
    pub fn remove_permanent(&mut self, id: u16) {
        let idx = id as usize;
        unsafe {
            *self.highs.get_unchecked_mut(idx) = 0;
            *self.lows.get_unchecked_mut(idx) = 0;
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
    const fn header(masks: u16) -> u16 {
        if masks > 0xff { 2 } else { 1 }
    }
}
