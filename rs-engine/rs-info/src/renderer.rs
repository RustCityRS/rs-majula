use crate::EntityMasks;
use rs_grid::CoordGrid;
use rs_io::Packet;
#[cfg(since_647)]
use rs_protocol::network::game::info::hit::Hit;
use rs_protocol::network::game::info::npc::{
    anim::NpcInfoAnim, changetype::NpcInfoChangeType, damage::NpcInfoDamage,
    face_coord::NpcInfoFaceCoord, face_entity::NpcInfoFaceEntity, say::NpcInfoSay,
    spotanim::NpcInfoSpotAnim,
};
#[cfg(before_647)]
use rs_protocol::network::game::info::player::chat::PlayerInfoChat;
use rs_protocol::network::game::info::player::{
    anim::PlayerInfoAnim, appearance::PlayerInfoAppearance, damage::PlayerInfoDamage,
    exactmove::PlayerInfoExactMove, face_coord::PlayerInfoFaceCoord,
    face_entity::PlayerInfoFaceEntity, say::PlayerInfoSay, spotanim::PlayerInfoSpotAnim,
};
#[cfg(since_647)]
use rs_protocol::network::game::info::player::{
    move_type::PlayerInfoMoveType, temp_move_type::PlayerInfoTempMoveType,
};
use rs_protocol::network::game::info_prot::{NpcInfoProt, PlayerInfoProt};
use rs_protocol::network::game::info_prot_message::InfoMessage;
#[cfg(since_647)]
use rs_protocol::network::game::server::npc_info::add_direction;
use std::mem;

const MAX_PLAYERS: usize = 2048;
#[cfg(before_254)]
const MAX_NPCS: usize = 8192;
#[cfg(all(since_254, before_647))]
const MAX_NPCS: usize = 16384;
#[cfg(since_647)]
const MAX_NPCS: usize = 32768;

#[cfg(before_647)]
const SLOT_SIZE: usize = 8;
// Two soaked hits are the largest block: a count, then per hit 32767, mark,
// damage, soak mark, soak, delay and health bar (1 + 2 * 10 bytes).
#[cfg(since_647)]
const SLOT_SIZE: usize = 24;
#[cfg(since_647)]
const DEFAULT_MOVE_TYPE: u8 = 1;

/// The size a player faces from, and before 647 an npc.
const ONE_TILE: u8 = 1;

/// The mask header an add's size estimate counts.
const LOW_HEADER: usize = 1;

/// One entity's block, encoded.
#[derive(Clone, Copy)]
#[repr(C)]
struct Slot {
    data: [u8; SLOT_SIZE],
    len: u8,
}

impl Slot {
    /// The encoded block (`data[..len]`), empty when the slot holds none.
    #[inline(always)]
    fn bytes(&self) -> &[u8] {
        unsafe { self.data.get_unchecked(..self.len as usize) }
    }

    /// Encodes `block` in place of the one the slot held.
    #[inline(always)]
    fn set(&mut self, block: &impl InfoMessage) {
        let mut buf = Packet::from(&mut self.data[..]);
        block.encode(&mut buf);
        self.len = buf.pos as u8;
    }
}

/// Encodes a block that can outgrow a [`Slot`] in place of the one `bytes`
/// held.
#[inline(always)]
fn set_bytes(bytes: &mut Option<Vec<u8>>, block: &impl InfoMessage) {
    let bytes = bytes.get_or_insert_with(Vec::new);
    bytes.clear();
    bytes.resize(block.sizeof(), 0);
    block.encode(&mut Packet::from(&mut bytes[..]));
}

/// Blocks as their bits in the mask header (`masks`) and as where they sit
/// in the order the client reads them (`order`, a bit per `to_index`).
#[derive(Clone, Copy, Default)]
struct Blocks {
    masks: u16,
    order: u16,
}

impl Blocks {
    const fn player(prot: PlayerInfoProt) -> Blocks {
        Blocks {
            masks: prot as u16,
            order: 1 << prot.to_index(),
        }
    }

    const fn npc(prot: NpcInfoProt) -> Blocks {
        Blocks {
            masks: prot as u16,
            order: 1 << prot.to_index(),
        }
    }

    const fn with(self, other: Blocks) -> Blocks {
        Blocks {
            masks: self.masks | other.masks,
            order: self.order | other.order,
        }
    }

    const fn without(self, other: Blocks) -> Blocks {
        Blocks {
            masks: self.masks & !other.masks,
            order: self.order & !other.order,
        }
    }
}

/// Where an entity's update block sits in this tick's block buffer.
#[derive(Clone, Copy, Default)]
struct Span {
    start: u32,
    end: u32,
}

impl Span {
    const fn new(start: usize, end: usize) -> Span {
        Span {
            start: start as u32,
            end: end as u32,
        }
    }
}

/// Which way an entity faces for an observer that adds it: the entity it
/// faces, if any, and the tile it faces, noted in the info phase.
#[derive(Clone, Copy, Default)]
struct LowFace<C> {
    entity: Option<u16>,
    coord: C,
}

/// The fine coords an entity of `size` tiles at `coord` faces: this tick's
/// face coord, else its orientation, else its own.
fn facing(info: &EntityMasks, coord: CoordGrid, size: u8) -> (u16, u16) {
    info.face_x
        .zip(info.face_z)
        .or_else(|| info.orientation_x.zip(info.orientation_z))
        .unwrap_or((
            CoordGrid::fine(coord.x(), size),
            CoordGrid::fine(coord.z(), size),
        ))
}

/// A player at `coord` facing fine coords `x`, `z`.
fn face_coord(
    x: u16,
    z: u16,
    #[cfg_attr(before_647, allow(unused_variables))] coord: CoordGrid,
) -> PlayerInfoFaceCoord {
    PlayerInfoFaceCoord {
        x,
        z,
        #[cfg(since_647)]
        from_x: CoordGrid::fine(coord.x(), ONE_TILE),
        #[cfg(since_647)]
        from_z: CoordGrid::fine(coord.z(), ONE_TILE),
    }
}

/// The player a block is written for and, before 647, where its client's
/// build area starts: an exact move is sent relative to it.
#[derive(Clone, Copy, Default)]
pub struct Observer {
    pub pid: u16,
    #[cfg(before_647)]
    pub origin: CoordGrid,
}

/// A player's exact move this tick in world tiles, for each observer to get
/// relative to its build area.
#[cfg(before_647)]
#[derive(Clone, Copy, Default)]
struct ExactMove {
    start_x: u16,
    start_z: u16,
    end_x: u16,
    end_z: u16,
    begin: u16,
    finish: u16,
    dir: u8,
}

#[cfg(before_647)]
impl ExactMove {
    fn of(info: &EntityMasks) -> ExactMove {
        ExactMove {
            start_x: info.exactmove_start_x.unwrap(),
            start_z: info.exactmove_start_z.unwrap(),
            end_x: info.exactmove_end_x.unwrap(),
            end_z: info.exactmove_end_z.unwrap(),
            begin: info.exactmove_begin.unwrap(),
            finish: info.exactmove_finish.unwrap(),
            dir: info.exactmove_dir.unwrap(),
        }
    }

    /// The move as a client whose build area starts at `origin` reads it.
    fn block(&self, origin: CoordGrid) -> PlayerInfoExactMove {
        let (x, z) = (origin.zone_origin_x(), origin.zone_origin_z());
        PlayerInfoExactMove {
            start_x: (self.start_x - x) as u8,
            start_z: (self.start_z - z) as u8,
            end_x: (self.end_x - x) as u8,
            end_z: (self.end_z - z) as u8,
            begin: self.begin,
            finish: self.finish,
            dir: self.dir,
        }
    }
}

/// An entity's hits this tick and who sees each bright (its receiver, if a
/// player, and its dealer, if a player), with where the hits sit in the
/// entity's high block, so a bright view can be written in their place.
#[cfg(since_647)]
#[derive(Clone, Copy, Default)]
struct HitView {
    hits: [Hit; 2],
    count: u8,
    bright: [[Option<u16>; 2]; 2],
    offset: u16,
    len: u16,
}

#[cfg(since_647)]
impl HitView {
    fn hits(&self) -> &[Hit] {
        &self.hits[..self.count as usize]
    }

    /// The hits `observer` sees bright, a bit each.
    fn bright(&self, observer: u16) -> u8 {
        let mut bright = 0;
        for (i, who) in self.bright[..self.count as usize].iter().enumerate() {
            if who.contains(&Some(observer)) {
                bright |= 1 << i;
            }
        }
        bright
    }
}

/// `block` with its `len` bytes at `offset` replaced by what `write` writes.
#[cfg(since_647)]
#[cold]
#[inline(never)]
fn splice(buf: &mut Packet, block: &[u8], offset: u16, len: u16, write: impl FnOnce(&mut Packet)) {
    let (offset, len) = (offset as usize, len as usize);
    buf.pdata(block, 0, offset);
    write(buf);
    buf.pdata(block, offset + len, block.len() - offset - len);
}

/// Each hit's dealer, in [`hits`]' order.
#[cfg(since_647)]
fn hit_sources(info: &EntityMasks, first: bool, second: bool) -> [Option<u16>; 2] {
    match (first, second) {
        (true, true) => [info.damage_source, info.damage2_source],
        (true, false) => [info.damage_source, None],
        (false, true) => [info.damage2_source, None],
        (false, false) => [None, None],
    }
}

#[cfg(since_647)]
fn hits(info: &EntityMasks, first: bool, second: bool) -> ([Hit; 2], usize) {
    let mut out = [Hit::default(); 2];
    let mut n = 0;
    if first {
        out[n] = Hit {
            hitmark: info.damage_type.unwrap_or(0),
            damage: info.damage_taken.unwrap_or(0),
            soak: info.damage_soak.unwrap_or(0),
            health: Hit::health(
                info.damage_current.unwrap_or(0),
                info.damage_base.unwrap_or(0),
            ),
        };
        n += 1;
    }
    if second {
        out[n] = Hit {
            hitmark: info.damage2_type.unwrap_or(0),
            damage: info.damage2_taken.unwrap_or(0),
            soak: info.damage2_soak.unwrap_or(0),
            health: Hit::health(
                info.damage2_current.unwrap_or(0),
                info.damage2_base.unwrap_or(0),
            ),
        };
        n += 1;
    }
    (out, n)
}

/// Every player's info blocks this tick, encoded once in the info phase and
/// copied into each observer's packet.
///
/// Fixed-size blocks live in inline [`Slot`]s, one row per block in the
/// client's read order, while the appearance, say and chat use `Vec<u8>`
/// buffers. Each player's blocks are also kept together under their mask
/// header in one high block, with its length (`highs`) and the most an add's
/// blocks take (`lows`) for the output phase's capacity checks.
pub struct PlayerRenderer {
    fixed: Box<[[Slot; MAX_PLAYERS]; PlayerInfoProt::COUNT]>,
    appearances: Vec<Option<Vec<u8>>>,
    says: Vec<Option<Vec<u8>>>,
    #[cfg(before_647)]
    chats: Vec<Option<Vec<u8>>>,
    #[cfg(before_647)]
    exact_moves: Box<[ExactMove; MAX_PLAYERS]>,
    #[cfg(since_647)]
    move_types: Box<[u8; MAX_PLAYERS]>,
    #[cfg(since_647)]
    hit_views: Vec<HitView>,
    #[cfg(since_647)]
    regions: Box<[u32; MAX_PLAYERS]>,
    #[cfg(since_647)]
    next_regions: Box<[u32; MAX_PLAYERS]>,
    #[cfg(since_647)]
    region_moves: Box<[u64; MAX_PLAYERS >> 6]>,
    blocks: Vec<u8>,
    spans: Box<[Span; MAX_PLAYERS]>,
    highs: Box<[u16; MAX_PLAYERS]>,
    lows: Box<[u16; MAX_PLAYERS]>,
    ticks: Box<[Blocks; MAX_PLAYERS]>,
    low_faces: Box<[LowFace<PlayerInfoFaceCoord>; MAX_PLAYERS]>,
    appearance_clocks: Box<[Option<u32>; MAX_PLAYERS]>,
    written: Vec<u16>,
}

impl PlayerRenderer {
    #[inline]
    pub fn new() -> PlayerRenderer {
        PlayerRenderer {
            fixed: unsafe { Box::new_zeroed().assume_init() },
            appearances: vec![None; MAX_PLAYERS],
            says: vec![None; MAX_PLAYERS],
            #[cfg(before_647)]
            chats: vec![None; MAX_PLAYERS],
            #[cfg(before_647)]
            exact_moves: Box::new([ExactMove::default(); MAX_PLAYERS]),
            #[cfg(since_647)]
            move_types: Box::new([DEFAULT_MOVE_TYPE; MAX_PLAYERS]),
            #[cfg(since_647)]
            hit_views: vec![HitView::default(); MAX_PLAYERS],
            #[cfg(since_647)]
            regions: Box::new([0; MAX_PLAYERS]),
            #[cfg(since_647)]
            next_regions: Box::new([0; MAX_PLAYERS]),
            #[cfg(since_647)]
            region_moves: Box::new([0; MAX_PLAYERS >> 6]),
            blocks: Vec::new(),
            spans: Box::new([Span::default(); MAX_PLAYERS]),
            highs: Box::new([0; MAX_PLAYERS]),
            lows: Box::new([0; MAX_PLAYERS]),
            ticks: Box::new([Blocks::default(); MAX_PLAYERS]),
            low_faces: Box::new([LowFace::default(); MAX_PLAYERS]),
            appearance_clocks: Box::new([None; MAX_PLAYERS]),
            written: Vec::new(),
        }
    }

    /// Notes what an observer adding player `pid` this tick gets besides its
    /// blocks: which way it faces, and which appearance it has. Once a tick
    /// in the info phase, so an add doesn't read the player;
    /// [`ready_low`](Self::ready_low) writes the blocks it notes.
    #[inline(always)]
    pub fn cache_low(&mut self, pid: u16, info: &EntityMasks, coord: CoordGrid) {
        self.low_faces[pid as usize] = Self::low_face(info, coord);
        self.appearance_clocks[pid as usize] = info.last_appearance;
    }

    /// Caches the facing blocks (and on 647 the move type) that
    /// [`cache_low`](Self::cache_low) noted for player `pid`, for its adds.
    #[inline(always)]
    pub fn ready_low(&mut self, pid: u16) {
        self.cache_face(pid, self.low_faces[pid as usize]);
        #[cfg(since_647)]
        self.cache_move_type(pid);
    }

    /// Which appearance player `pid` has this tick, if it has one.
    #[inline(always)]
    pub fn appearance_clock(&self, pid: u16) -> Option<u32> {
        self.appearance_clocks[pid as usize]
    }

    /// Records the 18-bit region (`level << 16 | x >> 6 << 8 | z >> 6`)
    /// player `pid` is in this tick. Every client keeps a region for each slot
    /// it doesn't track, so one that moved is sent to all of them this tick.
    #[cfg(since_647)]
    #[inline(always)]
    pub fn track_region(&mut self, pid: u16, region: u32) {
        let idx = pid as usize;
        if region != self.regions[idx] {
            self.next_regions[idx] = region;
            self.region_moves[idx >> 6] |= 1 << (idx & 63);
        }
    }

    /// The region every client has for slot `pid`, as of last tick.
    #[cfg(since_647)]
    #[inline(always)]
    pub fn last_region(&self, pid: u16) -> u32 {
        self.regions[pid as usize]
    }

    /// The region slot `pid` is in this tick.
    #[cfg(since_647)]
    #[inline(always)]
    pub fn region(&self, pid: u16) -> u32 {
        let idx = pid as usize;
        if (self.region_moves[idx >> 6] >> (idx & 63)) & 1 != 0 {
            self.next_regions[idx]
        } else {
            self.regions[idx]
        }
    }

    /// The slots whose region moved this tick, one bit each.
    #[cfg(since_647)]
    #[inline(always)]
    pub fn region_moves(&self) -> &[u64] {
        &self.region_moves[..]
    }

    /// Every slot's region as of last tick, for a new client's first
    /// `PLAYER_INFO` state.
    #[cfg(since_647)]
    #[inline(always)]
    pub fn regions(&self) -> &[u32] {
        &self.regions[..]
    }

    /// Encodes player `pid`'s blocks this tick (`info.masks`) into its slots
    /// and, under their mask header, its high block, and notes their sizes.
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

        let mut order = 0;
        if masks & PlayerInfoProt::Appearance as u16 != 0 {
            let bytes = info.last_appearance_info.as_deref().unwrap();
            set_bytes(&mut self.appearances[pid], &PlayerInfoAppearance { bytes });
            order |= 1 << PlayerInfoProt::Appearance.to_index();
        }
        if masks & PlayerInfoProt::Anim as u16 != 0 {
            let anim = PlayerInfoAnim {
                seq: info.anim_id.unwrap_or(u16::MAX),
                delay: info.anim_delay.unwrap_or(0),
            };
            order |= self.set(PlayerInfoProt::Anim, pid, &anim);
        }
        if masks & PlayerInfoProt::FaceEntity as u16 != 0 {
            let entity = info.face_entity.unwrap_or(u16::MAX);
            order |= self.set(
                PlayerInfoProt::FaceEntity,
                pid,
                &PlayerInfoFaceEntity { entity },
            );
        }
        if masks & PlayerInfoProt::Say as u16 != 0 {
            let say = info.say.as_deref().unwrap();
            set_bytes(&mut self.says[pid], &PlayerInfoSay { say });
            order |= 1 << PlayerInfoProt::Say.to_index();
        }
        #[cfg(before_647)]
        if masks & PlayerInfoProt::Damage as u16 != 0 {
            let damage = PlayerInfoDamage {
                damage: info.damage_taken.unwrap() as u8,
                damage_type: info.damage_type.unwrap(),
                current: info.damage_current.unwrap(),
                base: info.damage_base.unwrap(),
            };
            order |= self.set(PlayerInfoProt::Damage, pid, &damage);
        }
        if masks & PlayerInfoProt::FaceCoord as u16 != 0 {
            let here = (
                CoordGrid::fine(coord.x(), ONE_TILE),
                CoordGrid::fine(coord.z(), ONE_TILE),
            );
            let (x, z) = info.face_x.zip(info.face_z).unwrap_or(here);
            order |= self.set(PlayerInfoProt::FaceCoord, pid, &face_coord(x, z, coord));
        }
        #[cfg(before_647)]
        if masks & PlayerInfoProt::Chat as u16 != 0 {
            let chat = PlayerInfoChat {
                colour: info.chat_colour.unwrap(),
                effects: info.chat_effects.unwrap(),
                ignored: info.chat_ignored.unwrap(),
                bytes: info.chat_bytes.as_deref().unwrap(),
            };
            set_bytes(&mut self.chats[pid], &chat);
            order |= 1 << PlayerInfoProt::Chat.to_index();
        }
        if masks & PlayerInfoProt::SpotAnim as u16 != 0 {
            let spotanim = PlayerInfoSpotAnim {
                spotanim: info.spotanim.unwrap_or(u16::MAX),
                height: info.spotanim_height.unwrap_or(0),
                delay: info.spotanim_delay.unwrap_or(0),
            };
            order |= self.set(PlayerInfoProt::SpotAnim, pid, &spotanim);
        }
        if masks & PlayerInfoProt::ExactMove as u16 != 0 {
            #[cfg(before_647)]
            {
                self.exact_moves[pid] = ExactMove::of(info);
                order |= 1 << PlayerInfoProt::ExactMove.to_index();
            }
            // The client adds these to the player's own tile, where the move
            // has already put him (`Class244`, mask 0x4000): tile offsets
            // from `coord`, not world coordinates.
            #[cfg(since_647)]
            {
                let offset = |tile: Option<u16>, here: u16| {
                    (i32::from(tile.unwrap_or(here)) - i32::from(here)) as u8
                };
                let exactmove = PlayerInfoExactMove {
                    start_x: offset(info.exactmove_start_x, coord.x()),
                    start_z: offset(info.exactmove_start_z, coord.z()),
                    end_x: offset(info.exactmove_end_x, coord.x()),
                    end_z: offset(info.exactmove_end_z, coord.z()),
                    begin: info.exactmove_begin.unwrap_or(0),
                    finish: info.exactmove_finish.unwrap_or(0),
                    dir: info.exactmove_dir.unwrap_or(0),
                };
                order |= self.set(PlayerInfoProt::ExactMove, pid, &exactmove);
            }
        }
        #[cfg(all(since_244, before_647))]
        if masks & PlayerInfoProt::Damage2 as u16 != 0 {
            let damage = PlayerInfoDamage {
                damage: info.damage2_taken.unwrap() as u8,
                damage_type: info.damage2_type.unwrap(),
                current: info.damage2_current.unwrap(),
                base: info.damage2_base.unwrap(),
            };
            order |= self.set(PlayerInfoProt::Damage2, pid, &damage);
        }
        #[cfg(since_647)]
        {
            if masks & PlayerInfoProt::MoveType as u16 != 0 {
                let move_type = info.move_type.unwrap_or(0);
                self.move_types[pid] = move_type;
                order |= self.set(
                    PlayerInfoProt::MoveType,
                    pid,
                    &PlayerInfoMoveType { move_type },
                );
            }
            if masks & PlayerInfoProt::TempMoveType as u16 != 0 {
                let move_type = info.temp_move_type.unwrap_or(0);
                order |= self.set(
                    PlayerInfoProt::TempMoveType,
                    pid,
                    &PlayerInfoTempMoveType { move_type },
                );
            }
            let first = masks & PlayerInfoProt::Damage as u16 != 0;
            let second = masks & PlayerInfoProt::Damage2 as u16 != 0;
            if first || second {
                let (hits, count) = hits(info, first, second);
                let damage = PlayerInfoDamage {
                    hits: &hits[..count],
                    bright: 0,
                };
                order |= self.set(PlayerInfoProt::Damage, pid, &damage);
                // The player takes each hit, so sees it bright, as its
                // dealer does.
                let me = Some(pid as u16);
                self.hit_views[pid] = HitView {
                    hits,
                    count: count as u8,
                    bright: hit_sources(info, first, second).map(|by| [me, by]),
                    offset: 0,
                    len: 0,
                };
            }
        }
        self.ticks[pid] = Blocks { masks, order };

        let mut blocks = mem::take(&mut self.blocks);
        let start = blocks.len();
        let (header, len) = PlayerInfoProt::header(masks);
        blocks.extend_from_slice(&header[..len]);
        let mut bits = order;
        while bits != 0 {
            let index = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            blocks.extend_from_slice(self.bytes(index, pid));
            #[cfg(since_647)]
            if index == PlayerInfoProt::Damage.to_index() {
                let len = self.fixed[index][pid].len as usize;
                let view = &mut self.hit_views[pid];
                view.offset = (blocks.len() - len - start) as u16;
                view.len = len as u16;
            }
        }
        let high = blocks.len() - start;
        self.spans[pid] = Span::new(start, blocks.len());
        self.blocks = blocks;
        #[cfg(before_647)]
        let high = if masks & PlayerInfoProt::ExactMove as u16 != 0 {
            high + PlayerInfoExactMove::default().sizeof()
        } else {
            high
        };
        self.highs[pid] = high as u16;

        if masks
            & (PlayerInfoProt::Appearance as u16
                | PlayerInfoProt::FaceEntity as u16
                | PlayerInfoProt::FaceCoord as u16)
            != 0
        {
            let appearance = self.appearances[pid].as_ref().map_or(0, Vec::len);
            self.lows[pid] = (appearance + Self::low_facing()) as u16;
        }
    }

    /// Returns the precomputed high-definition block for a player.
    /// Before 647 it leaves out an exact move, which is relative to the
    /// observer.
    #[inline(always)]
    pub fn high_block(&self, id: u16) -> &[u8] {
        unsafe {
            let span = *self.spans.get_unchecked(id as usize);
            self.blocks
                .get_unchecked(span.start as usize..span.end as usize)
        }
    }

    /// Writes player `pid`'s blocks this tick as `observer` sees them: the
    /// high block, but a player doesn't get its own chat back, and an exact
    /// move is relative to the observer's build area.
    #[cfg(before_647)]
    #[inline(always)]
    pub fn write_high_block_for(&self, buf: &mut Packet, pid: u16, observer: Observer) {
        let tick = self.ticks[pid as usize];
        let blocks = if pid == observer.pid {
            tick.without(Blocks::player(PlayerInfoProt::Chat))
        } else {
            tick
        };
        if blocks.masks != tick.masks || tick.masks & PlayerInfoProt::ExactMove as u16 != 0 {
            self.write_blocks(buf, pid, blocks, observer);
        } else {
            let block = self.high_block(pid);
            buf.pdata(block, 0, block.len());
        }
    }

    /// Writes a player's high block as `observer` sees it: the block carries
    /// every hit as a bystander sees it, and the hits `observer` took or
    /// dealt are written bright in their place.
    #[cfg(since_647)]
    #[inline(always)]
    pub fn write_high_block_for(&self, buf: &mut Packet, pid: u16, observer: Observer) {
        let block = self.high_block(pid);
        let view = &self.hit_views[pid as usize];
        let bright = view.bright(observer.pid);
        if bright == 0 {
            buf.pdata(block, 0, block.len());
        } else {
            splice(buf, block, view.offset, view.len, |buf| {
                PlayerInfoDamage {
                    hits: view.hits(),
                    bright,
                }
                .encode(buf)
            });
        }
    }

    /// Writes what `observer` gets of player `pid` as it adds it: this tick's
    /// blocks, which way the player faces (and on 647 its move type), and
    /// its appearance if `appearance`.
    #[inline]
    pub fn write_low_block_for(
        &self,
        buf: &mut Packet,
        pid: u16,
        observer: Observer,
        appearance: bool,
    ) {
        let mut blocks = self.ticks[pid as usize].with(Blocks::player(PlayerInfoProt::FaceCoord));
        if self.low_faces[pid as usize].entity.is_some() {
            blocks = blocks.with(Blocks::player(PlayerInfoProt::FaceEntity));
        }
        #[cfg(since_647)]
        {
            blocks = blocks.with(Blocks::player(PlayerInfoProt::MoveType));
        }
        let appearance_block = Blocks::player(PlayerInfoProt::Appearance);
        blocks = if appearance {
            blocks.with(appearance_block)
        } else {
            blocks.without(appearance_block)
        };
        self.write_blocks(buf, pid, blocks, observer);
    }

    /// Writes what a session replay's keyframe shows of player `pid`: which
    /// way it faces, cached for the tick as for an add, and its appearance
    /// if it has one.
    pub fn write_keyframe_block(
        &mut self,
        buf: &mut Packet,
        pid: u16,
        info: &EntityMasks,
        coord: CoordGrid,
    ) {
        let face = Self::low_face(info, coord);
        self.cache_face(pid, face);
        let mut blocks = Blocks::player(PlayerInfoProt::FaceCoord);
        if face.entity.is_some() {
            blocks = blocks.with(Blocks::player(PlayerInfoProt::FaceEntity));
        }
        if self.has(pid, PlayerInfoProt::Appearance) {
            blocks = blocks.with(Blocks::player(PlayerInfoProt::Appearance));
        }
        self.write_blocks(buf, pid, blocks, Observer::default());
    }

    /// Writes `blocks` of player `pid` under their mask header, in the order
    /// the client reads them, as `observer` sees them.
    fn write_blocks(&self, buf: &mut Packet, pid: u16, blocks: Blocks, observer: Observer) {
        let (header, len) = PlayerInfoProt::header(blocks.masks);
        buf.pdata(&header, 0, len);
        let mut order = blocks.order;
        while order != 0 {
            let index = order.trailing_zeros() as usize;
            order &= order - 1;
            #[cfg(before_647)]
            if index == PlayerInfoProt::ExactMove.to_index() {
                self.exact_moves[pid as usize]
                    .block(observer.origin)
                    .encode(buf);
                continue;
            }
            #[cfg(since_647)]
            if index == PlayerInfoProt::Damage.to_index() {
                let view = &self.hit_views[pid as usize];
                PlayerInfoDamage {
                    hits: view.hits(),
                    bright: view.bright(observer.pid),
                }
                .encode(buf);
                continue;
            }
            let bytes = self.bytes(index, pid as usize);
            buf.pdata(bytes, 0, bytes.len());
        }
    }

    /// Encodes `block` as player `pid`'s `prot` block, and returns its bit in
    /// the read order.
    #[inline(always)]
    fn set(&mut self, prot: PlayerInfoProt, pid: usize, block: &impl InfoMessage) -> u16 {
        let index = prot.to_index();
        self.fixed[index][pid].set(block);
        1 << index
    }

    /// Player `pid`'s encoded block at `index` in the read order, empty when
    /// it has none.
    #[inline(always)]
    fn bytes(&self, index: usize, pid: usize) -> &[u8] {
        unsafe {
            match index {
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
            }
        }
    }

    #[inline(always)]
    fn has(&self, pid: u16, prot: PlayerInfoProt) -> bool {
        !self.bytes(prot.to_index(), pid as usize).is_empty()
    }

    fn low_face(info: &EntityMasks, coord: CoordGrid) -> LowFace<PlayerInfoFaceCoord> {
        let (x, z) = facing(info, coord, ONE_TILE);
        LowFace {
            entity: info.face_entity,
            coord: face_coord(x, z, coord),
        }
    }

    /// The header and facing blocks an add carries besides the appearance
    /// and this tick's blocks.
    fn low_facing() -> usize {
        let facing = LOW_HEADER
            + PlayerInfoFaceEntity { entity: 0 }.sizeof()
            + PlayerInfoFaceCoord::default().sizeof();
        #[cfg(since_647)]
        let facing = facing + PlayerInfoMoveType { move_type: 0 }.sizeof();
        facing
    }

    /// Caches the blocks for which way player `pid` faces, for this tick's
    /// adds, unless it has them this tick.
    #[inline(always)]
    fn cache_face(&mut self, pid: u16, face: LowFace<PlayerInfoFaceCoord>) {
        if let Some(entity) = face.entity
            && !self.has(pid, PlayerInfoProt::FaceEntity)
        {
            self.written.push(pid);
            self.set(
                PlayerInfoProt::FaceEntity,
                pid as usize,
                &PlayerInfoFaceEntity { entity },
            );
        }
        if !self.has(pid, PlayerInfoProt::FaceCoord) {
            self.written.push(pid);
            self.set(PlayerInfoProt::FaceCoord, pid as usize, &face.coord);
        }
    }

    #[cfg(since_647)]
    #[inline(always)]
    fn cache_move_type(&mut self, pid: u16) {
        let move_type = self.move_types[pid as usize];
        let slot = &mut self.fixed[PlayerInfoProt::MoveType.to_index()][pid as usize];
        if slot.bytes().is_empty() {
            self.written.push(pid);
        }
        slot.set(&PlayerInfoMoveType { move_type });
    }

    /// Returns the total pre-computed high-definition byte size for a player.
    #[inline(always)]
    pub const fn highdefinitions(&self, id: u16) -> usize {
        unsafe { *self.highs.as_ptr().add(id as usize) as usize }
    }

    /// The most an add's blocks take besides this tick's: the mask header,
    /// the appearance and which way the player faces.
    #[inline(always)]
    pub const fn lowdefinitions(&self, id: u16) -> usize {
        unsafe { *self.lows.as_ptr().add(id as usize) as usize }
    }

    /// Clears the blocks of every player written this tick. Appearances and
    /// add sizes persist; players never written this tick cost nothing.
    #[inline]
    pub fn remove_temporary(&mut self) {
        let written = mem::take(&mut self.written);
        for &pid in &written {
            let idx = pid as usize;
            self.highs[idx] = 0;
            for slots in self.fixed.iter_mut() {
                slots[idx].len = 0;
            }
            if let Some(say) = &mut self.says[idx] {
                say.clear();
            }
            #[cfg(before_647)]
            if let Some(chat) = &mut self.chats[idx] {
                chat.clear();
            }
            #[cfg(since_647)]
            {
                self.hit_views[idx].count = 0;
            }
            self.spans[idx] = Span::default();
            self.ticks[idx] = Blocks::default();
        }
        self.blocks.clear();
        self.written = written;
        self.written.clear();
        #[cfg(since_647)]
        for (w, bits) in self.region_moves.iter_mut().enumerate() {
            while *bits != 0 {
                let idx = (w << 6) | bits.trailing_zeros() as usize;
                self.regions[idx] = self.next_regions[idx];
                *bits &= *bits - 1;
            }
        }
    }

    /// Drops everything kept for a player that logged out.
    #[inline]
    pub fn remove_permanent(&mut self, id: u16) {
        let idx = id as usize;
        #[cfg(since_647)]
        {
            self.move_types[idx] = DEFAULT_MOVE_TYPE;
        }
        self.highs[idx] = 0;
        self.lows[idx] = 0;
        self.spans[idx] = Span::default();
        self.appearances[idx] = None;
    }
}

/// Every NPC's info blocks this tick, encoded once in the info phase and
/// copied into each observer's packet: [`PlayerRenderer`]'s layout over the
/// NPC blocks and index space.
pub struct NpcRenderer {
    fixed: Box<[[Slot; MAX_NPCS]; NpcInfoProt::COUNT]>,
    says: Vec<Option<Vec<u8>>>,
    #[cfg(since_647)]
    hit_views: Vec<HitView>,
    blocks: Vec<u8>,
    spans: Box<[Span; MAX_NPCS]>,
    highs: Box<[u16; MAX_NPCS]>,
    ticks: Box<[Blocks; MAX_NPCS]>,
    low_faces: Box<[LowFace<NpcInfoFaceCoord>; MAX_NPCS]>,
    #[cfg(since_647)]
    add_directions: Box<[u8; MAX_NPCS]>,
    written: Vec<u16>,
}

impl NpcRenderer {
    #[inline]
    pub fn new() -> NpcRenderer {
        NpcRenderer {
            fixed: unsafe { Box::new_zeroed().assume_init() },
            says: vec![None; MAX_NPCS],
            #[cfg(since_647)]
            hit_views: vec![HitView::default(); MAX_NPCS],
            blocks: Vec::new(),
            spans: Box::new([Span::default(); MAX_NPCS]),
            highs: Box::new([0; MAX_NPCS]),
            ticks: Box::new([Blocks::default(); MAX_NPCS]),
            low_faces: Box::new([LowFace::default(); MAX_NPCS]),
            #[cfg(since_647)]
            add_directions: Box::new([0; MAX_NPCS]),
            written: Vec::new(),
        }
    }

    /// Notes what an observer adding NPC `nid` this tick gets besides its
    /// blocks: which way it faces, and on 647 the facing its add entry
    /// carries. Once a tick in the info phase, so an add doesn't read the
    /// NPC; [`ready_low`](Self::ready_low) writes the blocks it notes.
    #[inline(always)]
    pub fn cache_low(
        &mut self,
        nid: u16,
        info: &EntityMasks,
        coord: CoordGrid,
        #[cfg_attr(before_647, allow(unused_variables))] size: u8,
    ) {
        #[cfg(before_647)]
        let size = ONE_TILE;
        self.low_faces[nid as usize] = Self::low_face(info, coord, size);
        #[cfg(since_647)]
        {
            let facing = info
                .face_x
                .zip(info.face_z)
                .or_else(|| info.orientation_x.zip(info.orientation_z))
                .map(|(fx, fz)| {
                    (
                        CoordGrid::fine(coord.x(), size) as i32 - fx as i32,
                        CoordGrid::fine(coord.z(), size) as i32 - fz as i32,
                    )
                });
            self.add_directions[nid as usize] = add_direction(facing);
        }
    }

    /// Caches the facing blocks [`cache_low`](Self::cache_low) noted for NPC
    /// `nid`, for its adds.
    #[inline(always)]
    pub fn ready_low(&mut self, nid: u16) {
        self.cache_face(nid, self.low_faces[nid as usize]);
    }

    /// Which way NPC `nid` faces in its add entry this tick.
    #[cfg(since_647)]
    #[inline(always)]
    pub fn add_direction(&self, nid: u16) -> u8 {
        self.add_directions[nid as usize]
    }

    /// Encodes NPC `nid`'s blocks this tick (`info.masks`) into its slots
    /// and, under their mask header, its high block, and notes its size.
    #[inline]
    pub fn compute_info(&mut self, nid: usize, info: &EntityMasks) {
        let masks = info.masks;
        if masks == 0 {
            return;
        }
        self.written.push(nid as u16);

        let mut order = 0;
        if masks & NpcInfoProt::Anim as u16 != 0 {
            let anim = NpcInfoAnim {
                seq: info.anim_id.unwrap_or(u16::MAX),
                delay: info.anim_delay.unwrap_or(0),
            };
            order |= self.set(NpcInfoProt::Anim, nid, &anim);
        }
        if masks & NpcInfoProt::FaceEntity as u16 != 0 {
            let entity = info.face_entity.unwrap_or(u16::MAX);
            order |= self.set(NpcInfoProt::FaceEntity, nid, &NpcInfoFaceEntity { entity });
        }
        if masks & NpcInfoProt::Say as u16 != 0 {
            let say = info.say.as_deref().unwrap();
            set_bytes(&mut self.says[nid], &NpcInfoSay { say });
            order |= 1 << NpcInfoProt::Say.to_index();
        }
        #[cfg(before_647)]
        if masks & NpcInfoProt::Damage as u16 != 0 {
            let damage = NpcInfoDamage {
                damage: info.damage_taken.unwrap() as u8,
                damage_type: info.damage_type.unwrap(),
                current: info.damage_current.unwrap(),
                base: info.damage_base.unwrap(),
            };
            order |= self.set(NpcInfoProt::Damage, nid, &damage);
        }
        if masks & NpcInfoProt::ChangeType as u16 != 0 {
            let ntype = info.changetype.unwrap();
            order |= self.set(NpcInfoProt::ChangeType, nid, &NpcInfoChangeType { ntype });
        }
        if masks & NpcInfoProt::SpotAnim as u16 != 0 {
            let spotanim = NpcInfoSpotAnim {
                spotanim: info.spotanim.unwrap_or(u16::MAX),
                height: info.spotanim_height.unwrap_or(0),
                delay: info.spotanim_delay.unwrap_or(0),
            };
            order |= self.set(NpcInfoProt::SpotAnim, nid, &spotanim);
        }
        if masks & NpcInfoProt::FaceCoord as u16 != 0 {
            let coord = NpcInfoFaceCoord {
                x: info.face_x.unwrap(),
                z: info.face_z.unwrap(),
            };
            order |= self.set(NpcInfoProt::FaceCoord, nid, &coord);
        }
        #[cfg(all(since_244, before_647))]
        if masks & NpcInfoProt::Damage2 as u16 != 0 {
            let damage = NpcInfoDamage {
                damage: info.damage2_taken.unwrap() as u8,
                damage_type: info.damage2_type.unwrap(),
                current: info.damage2_current.unwrap(),
                base: info.damage2_base.unwrap(),
            };
            order |= self.set(NpcInfoProt::Damage2, nid, &damage);
        }
        #[cfg(since_647)]
        {
            let first = masks & NpcInfoProt::Damage as u16 != 0;
            let second = masks & NpcInfoProt::Damage2 as u16 != 0;
            if first || second {
                let (hits, count) = hits(info, first, second);
                let damage = NpcInfoDamage {
                    hits: &hits[..count],
                    bright: 0,
                };
                order |= self.set(NpcInfoProt::Damage, nid, &damage);
                // Only a hit's dealer sees an npc's hit bright.
                self.hit_views[nid] = HitView {
                    hits,
                    count: count as u8,
                    bright: hit_sources(info, first, second).map(|by| [by, None]),
                    offset: 0,
                    len: 0,
                };
            }
        }
        self.ticks[nid] = Blocks { masks, order };

        let mut blocks = mem::take(&mut self.blocks);
        let start = blocks.len();
        let (header, len) = NpcInfoProt::header(masks);
        blocks.extend_from_slice(&header[..len]);
        let mut bits = order;
        while bits != 0 {
            let index = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            blocks.extend_from_slice(self.bytes(index, nid));
            #[cfg(since_647)]
            if index == NpcInfoProt::Damage.to_index() {
                let len = self.fixed[index][nid].len as usize;
                let view = &mut self.hit_views[nid];
                view.offset = (blocks.len() - len - start) as u16;
                view.len = len as u16;
            }
        }
        self.highs[nid] = (blocks.len() - start) as u16;
        self.spans[nid] = Span::new(start, blocks.len());
        self.blocks = blocks;
    }

    /// Returns the pre-coalesced full high-definition block for an NPC (mask
    /// header + every block), written by the encoder with a single `pdata`.
    #[inline(always)]
    pub fn high_block(&self, id: u16) -> &[u8] {
        unsafe {
            let span = *self.spans.get_unchecked(id as usize);
            self.blocks
                .get_unchecked(span.start as usize..span.end as usize)
        }
    }

    /// Writes an NPC's high block as `observer` (a pid) sees it: on 647 the
    /// hits they dealt bright, the rest as a bystander sees them.
    #[inline(always)]
    pub fn write_high_block_for(
        &self,
        buf: &mut Packet,
        nid: u16,
        #[cfg_attr(before_647, allow(unused_variables))] observer: u16,
    ) {
        let block = self.high_block(nid);
        #[cfg(since_647)]
        {
            let view = &self.hit_views[nid as usize];
            let bright = view.bright(observer);
            if bright != 0 {
                splice(buf, block, view.offset, view.len, |buf| {
                    NpcInfoDamage {
                        hits: view.hits(),
                        bright,
                    }
                    .encode(buf)
                });
                return;
            }
        }
        buf.pdata(block, 0, block.len());
    }

    /// Writes what `observer` gets of NPC `nid` as it adds it: this tick's
    /// blocks and which way the NPC faces.
    #[inline]
    pub fn write_low_block_for(&self, buf: &mut Packet, nid: u16, observer: u16) {
        let mut blocks = self.ticks[nid as usize].with(Blocks::npc(NpcInfoProt::FaceCoord));
        if self.low_faces[nid as usize].entity.is_some() {
            blocks = blocks.with(Blocks::npc(NpcInfoProt::FaceEntity));
        }
        self.write_blocks(buf, nid, blocks, observer);
    }

    /// Writes what a session replay's keyframe shows of NPC `nid`: which way
    /// it faces, cached for the tick as for an add.
    pub fn write_keyframe_block(
        &mut self,
        buf: &mut Packet,
        nid: u16,
        info: &EntityMasks,
        coord: CoordGrid,
    ) {
        let face = Self::low_face(info, coord, ONE_TILE);
        self.cache_face(nid, face);
        let mut blocks = Blocks::npc(NpcInfoProt::FaceCoord);
        if face.entity.is_some() {
            blocks = blocks.with(Blocks::npc(NpcInfoProt::FaceEntity));
        }
        self.write_blocks(buf, nid, blocks, 0);
    }

    /// Writes `blocks` of NPC `nid` under their mask header, in the order the
    /// client reads them, as `observer` (a pid) sees them.
    fn write_blocks(
        &self,
        buf: &mut Packet,
        nid: u16,
        blocks: Blocks,
        #[cfg_attr(before_647, allow(unused_variables))] observer: u16,
    ) {
        let (header, len) = NpcInfoProt::header(blocks.masks);
        buf.pdata(&header, 0, len);
        let mut order = blocks.order;
        while order != 0 {
            let index = order.trailing_zeros() as usize;
            order &= order - 1;
            #[cfg(since_647)]
            if index == NpcInfoProt::Damage.to_index() {
                let view = &self.hit_views[nid as usize];
                NpcInfoDamage {
                    hits: view.hits(),
                    bright: view.bright(observer),
                }
                .encode(buf);
                continue;
            }
            let bytes = self.bytes(index, nid as usize);
            buf.pdata(bytes, 0, bytes.len());
        }
    }

    /// Encodes `block` as NPC `nid`'s `prot` block, and returns its bit in
    /// the read order.
    #[inline(always)]
    fn set(&mut self, prot: NpcInfoProt, nid: usize, block: &impl InfoMessage) -> u16 {
        let index = prot.to_index();
        self.fixed[index][nid].set(block);
        1 << index
    }

    /// NPC `nid`'s encoded block at `index` in the read order, empty when it
    /// has none.
    #[inline(always)]
    fn bytes(&self, index: usize, nid: usize) -> &[u8] {
        unsafe {
            if index == NpcInfoProt::Say.to_index() {
                self.says.get_unchecked(nid).as_deref().unwrap_or_default()
            } else {
                self.fixed.get_unchecked(index).get_unchecked(nid).bytes()
            }
        }
    }

    #[inline(always)]
    fn has(&self, nid: u16, prot: NpcInfoProt) -> bool {
        !self.bytes(prot.to_index(), nid as usize).is_empty()
    }

    fn low_face(info: &EntityMasks, coord: CoordGrid, size: u8) -> LowFace<NpcInfoFaceCoord> {
        let (x, z) = facing(info, coord, size);
        LowFace {
            entity: info.face_entity,
            coord: NpcInfoFaceCoord { x, z },
        }
    }

    /// Caches the blocks for which way NPC `nid` faces, for this tick's adds,
    /// unless it has them this tick.
    #[inline(always)]
    fn cache_face(&mut self, nid: u16, face: LowFace<NpcInfoFaceCoord>) {
        if let Some(entity) = face.entity
            && !self.has(nid, NpcInfoProt::FaceEntity)
        {
            self.written.push(nid);
            self.set(
                NpcInfoProt::FaceEntity,
                nid as usize,
                &NpcInfoFaceEntity { entity },
            );
        }
        if !self.has(nid, NpcInfoProt::FaceCoord) {
            self.written.push(nid);
            self.set(NpcInfoProt::FaceCoord, nid as usize, &face.coord);
        }
    }

    #[inline(always)]
    pub const fn highdefinitions(&self, id: u16) -> usize {
        unsafe { *self.highs.as_ptr().add(id as usize) as usize }
    }

    /// The most a low-definition update can be: the face coord is always
    /// sent, and the face entity with it when the NPC has one, under one
    /// mask header.
    pub const LOW_LEN: usize = LOW_HEADER + size_of::<u16>() + 2 * size_of::<u16>();

    /// Clears the blocks of every NPC written this tick. NPCs never written
    /// this tick cost nothing.
    #[inline]
    pub fn remove_temporary(&mut self) {
        let written = mem::take(&mut self.written);
        for &nid in &written {
            let idx = nid as usize;
            self.highs[idx] = 0;
            for slots in self.fixed.iter_mut() {
                slots[idx].len = 0;
            }
            if let Some(say) = &mut self.says[idx] {
                say.clear();
            }
            #[cfg(since_647)]
            {
                self.hit_views[idx].count = 0;
            }
            self.spans[idx] = Span::default();
            self.ticks[idx] = Blocks::default();
        }
        self.blocks.clear();
        self.written = written;
        self.written.clear();
    }

    /// Drops everything kept for an NPC that despawned.
    #[inline]
    pub fn remove_permanent(&mut self, id: u16) {
        let idx = id as usize;
        #[cfg(since_647)]
        {
            self.says[idx] = None;
        }
        self.highs[idx] = 0;
        self.spans[idx] = Span::default();
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
        player.cache_face(
            7,
            LowFace {
                entity: Some(42),
                coord: face_coord(6445, 6437, coord),
            },
        );
        player.cache_move_type(7);
        assert_eq!(player.high_block(7), &[0x8, 2]);
        assert_eq!(player.highdefinitions(7), 2);
        assert!(player.has(7, PlayerInfoProt::Appearance));

        info.masks = NpcInfoProt::ChangeType as u16;
        info.changetype = Some(4231);
        npc.compute_info(9, &info);
        assert_eq!(npc.high_block(9), &[0x2, 0x10, 0x87]);
        npc.cache_face(
            9,
            LowFace {
                entity: Some(42),
                coord: NpcInfoFaceCoord { x: 6445, z: 6437 },
            },
        );
        assert_eq!(npc.high_block(9), &[0x2, 0x10, 0x87]);
        assert_eq!(npc.highdefinitions(9), 3);

        player.remove_temporary();
        npc.remove_temporary();
        assert!(player.high_block(7).is_empty());
        assert!(npc.high_block(9).is_empty());
        assert!(player.has(7, PlayerInfoProt::Appearance));
    }
}
