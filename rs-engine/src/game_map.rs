use crate::active_npc::ActiveNpc;
use num_enum::TryFromPrimitive;
use rs_entity::{EntityLifeTime, Loc, Obj};
use rs_grid::CoordGrid;
use rs_grid::mapsquare_coord::MapsquareCoordGrid;
use rs_io::Packet;
use rs_io::bz2::bz2_decompress;
use rs_pack::cache::CacheStore;
use rs_pack::cache::dbrow::DbRowValue;
use rs_pack::types::{LocAngle, LocLayer, LocShape, ParamValue, ScriptVarType};
use rs_var::VarSet;
use rs_zone::zone_map::ZoneMap;
use rsmod::rsmod::flag::zone_flag::ZoneFlag;
use tracing::info;

/// Mapsquare width in tiles.
const X: usize = 64;
/// Number of vertical levels (floors).
const Y: usize = 4;
/// Mapsquare height in tiles.
const Z: usize = 64;
/// Total number of tiles per mapsquare (X * Y * Z = 1 << 14).
const MAPSQUARE: usize = 1 << 14;

/// Tile flag: no special flags set.
const OPEN: u8 = 0x0;
/// Tile flag: tile is blocked (floor collision).
const BLOCK_MAP_SQUARE: u8 = 0x1;
/// Tile flag: entities on this tile are rendered one level below (bridge).
const LINK_BELOW: u8 = 0x2;
/// Tile flag: roofs above this tile are removed when viewing.
const REMOVE_ROOFS: u8 = 0x4;
/// Tile flag: the tile below is visible through the floor.
#[allow(unused)]
const VISIBLE_BELOW: u8 = 0x8;
/// Tile flag: the tile is not included in low-detail rendering.
#[allow(unused)]
const NOT_LOW_DETAIL: u8 = 0x10;

/// Provides static methods for loading the world map data (ground,
/// locations, NPCs, and objects) from the game cache into the collision
/// system and zone map.
pub struct GameMap;

/// Decompresses a BZip2-compressed map data blob prefixed with a 4-byte
/// big-endian uncompressed size.
///
/// # Arguments
/// * `data` - The raw compressed data, where bytes 0..4 are the decompressed
///   size and bytes 4.. are the BZip2 payload.
///
/// # Returns
/// A [`Packet`] wrapping the decompressed bytes.
fn decompress_map(data: &[u8]) -> Packet {
    let size = u32::from_be_bytes(data[..4].try_into().unwrap()) as usize;
    Packet::from(bz2_decompress(&data[4..], size, true, 0))
}

/// One bit per (zone, level) across the whole world, recording which zones
/// hold real content and therefore need a collision zone allocated.
///
/// Zones left unmarked are never allocated in `rsmod`: their tiles read back
/// `CollisionFlag::Null`, so walking and teleporting into them fails closed,
/// exactly like terrain outside any mapsquare. Line of sight is unaffected
/// because `is_flagged` reports never-written tiles as unflagged.
///
/// A zone gets marked when it contains (at the bridge-shifted level entities
/// actually occupy):
/// * a ground tile with an underlay or overlay;
/// * a loc footprint, expanded by one tile because straight walls also flag
///   the neighboring tile;
/// * an NPC spawn footprint (`size` x `size`) or an obj spawn tile;
/// * a coordinate authored in any cache config (enum keys/values, coord
///   params on npc/loc/obj/struct types, dbrow coord columns) -- scripts
///   teleport entities to those (e.g. fishing spot movement, agility fail
///   coords), and [`rs_entity::pathing::Pathing::teleport`] refuses
///   unallocated zones.
///
/// The whole structure is 2 MiB and only lives for the duration of
/// [`GameMap::load`].
struct ZoneMarks {
    bits: Box<[u64]>,
}

impl ZoneMarks {
    /// 2048 x 2048 zones across 4 levels, one bit each: 1 << 24 bits (11 + 11
    /// + 2 index bits, see [`index`](Self::index)) packed 64 per.
    const MAX: usize = (1 << 24) >> 6;

    fn new() -> ZoneMarks {
        ZoneMarks {
            bits: vec![0; ZoneMarks::MAX].into_boxed_slice(),
        }
    }

    /// Bit index of the zone containing tile (x, z) on level `y`, mirroring
    /// the `rsmod` zone index layout.
    #[inline(always)]
    fn index(x: u16, z: u16, y: u8) -> usize {
        ((x as usize >> 3) & 0x7FF)
            | (((z as usize >> 3) & 0x7FF) << 11)
            | ((y as usize & 0x3) << 22)
    }

    /// Marks the zone containing tile (x, z) on level `y`.
    #[inline(always)]
    fn mark(&mut self, x: u16, z: u16, y: u8) {
        let index = ZoneMarks::index(x, z, y);
        self.bits[index >> 6] |= 1 << (index & 0x3F);
    }

    /// Returns whether the zone containing tile (x, z) on level `y` is marked.
    #[inline(always)]
    fn is_marked(&self, x: u16, z: u16, y: u8) -> bool {
        let index = ZoneMarks::index(x, z, y);
        self.bits[index >> 6] & (1 << (index & 0x3F)) != 0
    }

    /// Marks the zone containing a script-packed coordinate
    /// (`z | x << 14 | y << 28`), ignoring null and invalid (`<= 0`) values.
    fn mark_packed(&mut self, packed: i32) {
        if packed > 0 {
            let coord = CoordGrid::from(packed as u32);
            self.mark(coord.x(), coord.z(), coord.y());
        }
    }

    /// Marks every zone overlapped by the inclusive tile rectangle
    /// `(x0..=x1, z0..=z1)` on level `y`.
    fn mark_rect(&mut self, x0: u16, z0: u16, x1: u16, z1: u16, y: u8) {
        for zx in (x0 >> 3)..=(x1.min(0x3FFF) >> 3) {
            for zz in (z0 >> 3)..=(z1.min(0x3FFF) >> 3) {
                self.mark(zx << 3, zz << 3, y);
            }
        }
    }

    /// Returns whether every zone overlapped by the inclusive tile rectangle
    /// `(x0..=x1, z0..=z1)` on level `y` is marked.
    fn is_rect_marked(&self, x0: u16, z0: u16, x1: u16, z1: u16, y: u8) -> bool {
        for zx in (x0 >> 3)..=(x1.min(0x3FFF) >> 3) {
            for zz in (z0 >> 3)..=(z1.min(0x3FFF) >> 3) {
                if !self.is_marked(zx << 3, zz << 3, y) {
                    return false;
                }
            }
        }
        true
    }
}

/// The inclusive tile rectangle whose zones a loc placement touches: every
/// collision write target (straight walls also flag the neighboring tile;
/// decor writes nothing) plus the origin tile that a static registration is
/// keyed on. Both the decode-pass marking and the apply-pass guard use this,
/// so a loc is applied exactly when every zone it would write to is marked.
fn loc_zone_rect(shape: LocShape, x: u16, z: u16, extent: u16) -> (u16, u16, u16, u16) {
    match shape.layer() {
        LocLayer::Wall => (
            x.saturating_sub(1),
            z.saturating_sub(1),
            x + extent,
            z + extent,
        ),
        LocLayer::WallDecor | LocLayer::GroundDecor => (x, z, x, z),
        LocLayer::Ground => (x, z, x + extent - 1, z + extent - 1),
    }
}

/// A decoded loc entry from an `l` mapsquare file, buffered between the
/// decode and apply passes.
struct MapLocEntry {
    id: u16,
    coord: u16,
    info: u8,
}

/// A decoded NPC spawn entry from an `n` mapsquare file.
struct MapNpcEntry {
    id: u16,
    coord: u16,
}

/// A decoded obj spawn entry from an `o` mapsquare file.
struct MapObjEntry {
    id: u16,
    coord: u16,
    count: u16,
}

/// One decoded mapsquare, buffered between the decode pass (which computes
/// the [`ZoneMarks`]) and the apply pass (which allocates zones and applies
/// collision and content).
struct MapsquareData {
    originx: u16,
    originz: u16,
    lands: Box<[u8; MAPSQUARE]>,
    locs: Vec<MapLocEntry>,
    npcs: Vec<MapNpcEntry>,
    objs: Vec<MapObjEntry>,
}

impl GameMap {
    /// Loads the entire world map from the cache, populating collision data,
    /// static locations, NPC spawns, and ground objects.
    ///
    /// Runs in two passes. The decode pass parses every mapsquare once and
    /// records which zones hold real content in a [`ZoneMarks`] bitset. The
    /// apply pass then allocates collision zones and applies ground and loc
    /// collision, skipping zones that are entirely empty ("black" void, most
    /// of levels 1-3): unallocated zones already read back as blocked, so
    /// only real content needs backing memory.
    ///
    /// # Arguments
    /// * `members` - Whether members-only content should be loaded.
    /// * `cache` - The game cache containing mapsquare data.
    /// * `zones` - The zone map to populate with static locs and objs.
    ///
    /// # Returns
    /// A `Vec<ActiveNpc>` of all NPC spawns loaded from the map data.
    ///
    /// # Side Effects
    /// * Allocates zones and sets collision flags via `rsmod`.
    /// * Populates static locs and objs in the zone map.
    ///
    /// # Call Stack
    /// **Calls:** [`decode_ground`](Self::decode_ground),
    /// [`decode_locations`](Self::decode_locations),
    /// [`decode_npcs`](Self::decode_npcs), [`decode_objs`](Self::decode_objs),
    /// [`mark_config_coords`](Self::mark_config_coords),
    /// [`apply_ground`](Self::apply_ground),
    /// [`apply_locations`](Self::apply_locations),
    /// [`spawn_npcs`](Self::spawn_npcs), [`spawn_objs`](Self::spawn_objs)
    pub fn load(members: bool, cache: &CacheStore, zones: &mut ZoneMap) -> Vec<ActiveNpc> {
        let mapsquare_keys: Vec<(u8, u8)> = cache
            .mapsquares
            .keys()
            .filter(|(c, _, _)| *c == 'm')
            .map(|(_, x, z)| (*x, *z))
            .collect();

        let mut marks = ZoneMarks::new();
        let mut squares = Vec::with_capacity(mapsquare_keys.len());

        for (mx, mz) in mapsquare_keys {
            let originx = (mx as u16) << 6;
            let originz = (mz as u16) << 6;

            let mut square = MapsquareData {
                originx,
                originz,
                lands: Box::new([0; MAPSQUARE]),
                locs: Vec::new(),
                npcs: Vec::new(),
                objs: Vec::new(),
            };

            if let Some(data) = cache.mapsquares.get(&('m', mx, mz)) {
                Self::decode_ground(
                    &mut square.lands,
                    &mut decompress_map(data),
                    &mut marks,
                    originx,
                    originz,
                );
            }

            if let Some(data) = cache.mapsquares.get(&('l', mx, mz)) {
                Self::decode_locations(
                    &square.lands,
                    &mut decompress_map(data),
                    cache,
                    &mut marks,
                    originx,
                    originz,
                    &mut square.locs,
                );
            }

            if let Some(data) = cache.mapsquares.get(&('n', mx, mz)) {
                Self::decode_npcs(
                    &mut decompress_map(data),
                    cache,
                    &mut marks,
                    originx,
                    originz,
                    &mut square.npcs,
                );
            }

            if let Some(data) = cache.mapsquares.get(&('o', mx, mz)) {
                Self::decode_objs(
                    &mut decompress_map(data),
                    &mut marks,
                    originx,
                    originz,
                    &mut square.objs,
                );
            }

            squares.push(square);
        }

        Self::mark_config_coords(cache, &mut marks);

        let mut spawned_npcs = Vec::new();
        for square in &squares {
            Self::apply_ground(
                members,
                &square.lands,
                &marks,
                square.originx,
                square.originz,
            );
            Self::apply_locations(
                members,
                &square.lands,
                &square.locs,
                cache,
                zones,
                &marks,
                square.originx,
                square.originz,
            );
            Self::spawn_npcs(
                members,
                &square.npcs,
                cache,
                &mut spawned_npcs,
                square.originx,
                square.originz,
            );
            Self::spawn_objs(
                members,
                &square.objs,
                cache,
                zones,
                square.originx,
                square.originz,
            );
        }

        let mut allocated = [0; Y];
        for square in &squares {
            for y in 0..Y as u8 {
                for zx in 0..(X as u16 >> 3) {
                    for zz in 0..(Z as u16 >> 3) {
                        if rsmod::is_zone_allocated(
                            square.originx + (zx << 3),
                            square.originz + (zz << 3),
                            y,
                        ) {
                            allocated[y as usize] += 1;
                        }
                    }
                }
            }
        }
        // 8 x 8 zones per mapsquare (<< 6), across Y levels (<< 2).
        let per_level = squares.len() << 6;
        info!(
            "Loaded {} mapsquares: {}/{} zones allocated (level 0: {}, 1: {}, 2: {}, 3: {} of {} each)",
            squares.len(),
            allocated.iter().sum::<usize>(),
            per_level << 2,
            allocated[0],
            allocated[1],
            allocated[2],
            allocated[3],
            per_level
        );

        spawned_npcs
    }

    /// Decodes ground tile data from the mapsquare buffer into the per-tile
    /// `lands` flag array and marks zones that contain ground.
    ///
    /// A tile counts as ground when it has an underlay or overlay: fully empty
    /// zones are left unmarked so the apply pass never allocates them. Marks
    /// are recorded at the bridge-shifted level (the level entities actually
    /// occupy); bridged tiles additionally mark their decode level so roof
    /// flags there still apply.
    ///
    /// # Arguments
    /// * `lands` - Output array to receive per-tile flags for the mapsquare.
    /// * `buf` - The decompressed mapsquare ground data.
    /// * `marks` - The world zone marks to record ground presence into.
    /// * `originx` - The mapsquare origin X in absolute tile coordinates.
    /// * `originz` - The mapsquare origin Z in absolute tile coordinates.
    ///
    /// # Call Stack
    /// **Called by:** [`load`](Self::load)
    fn decode_ground(
        lands: &mut [u8; MAPSQUARE],
        buf: &mut Packet,
        marks: &mut ZoneMarks,
        originx: u16,
        originz: u16,
    ) {
        let mut overlays = [0; MAPSQUARE];
        let mut underlays = [0; MAPSQUARE];

        for y in 0..Y {
            for x in 0..X {
                for z in 0..Z {
                    let coord = MapsquareCoordGrid::new(x as u8, y as u8, z as u8);
                    loop {
                        let opcode = buf.g1();
                        if opcode == 0 {
                            break;
                        } else if opcode == 1 {
                            buf.pos += 1;
                            break;
                        }
                        if opcode <= 49 {
                            overlays[coord.packed() as usize] = buf.g1();
                        } else if opcode <= 81 {
                            lands[coord.packed() as usize] = opcode - 49;
                        } else {
                            underlays[coord.packed() as usize] = opcode - 81;
                        }
                    }
                }
            }
        }

        for y in 0..Y {
            for x in 0..X {
                for z in 0..Z {
                    let coord = MapsquareCoordGrid::new(x as u8, y as u8, z as u8);
                    if overlays[coord.packed() as usize] == 0
                        && underlays[coord.packed() as usize] == 0
                    {
                        continue;
                    }

                    let bridge = Self::bridge_level(lands, x as u8, y as u8, z as u8);
                    if bridge < 0 {
                        continue;
                    }

                    let coordx = originx + x as u16;
                    let coordz = originz + z as u16;
                    marks.mark(coordx, coordz, bridge as u8);
                    if bridge as usize != y {
                        marks.mark(coordx, coordz, y as u8);
                    }
                }
            }
        }
    }

    /// Decodes location (loc) entries from the mapsquare buffer into a
    /// buffered list and marks the zones their collision touches.
    ///
    /// Each loc marks the zones of its write rectangle (see [`loc_zone_rect`];
    /// the orientation of non-square locs depends on the angle, so the larger
    /// dimension is used for both axes). Roof pieces are exempt (see
    /// [`LocShape::is_roof`]) so building roofs alone do not keep level 1+
    /// zones allocated.
    ///
    /// # Arguments
    /// * `lands` - The tile flags decoded by [`decode_ground`](Self::decode_ground).
    /// * `buf` - The decompressed mapsquare location data.
    /// * `cache` - The game cache for loc type lookups.
    /// * `marks` - The world zone marks to record loc footprints into.
    /// * `originx` - The mapsquare origin X in absolute tile coordinates.
    /// * `originz` - The mapsquare origin Z in absolute tile coordinates.
    /// * `out` - The output vector receiving the decoded entries in file order.
    ///
    /// # Call Stack
    /// **Called by:** [`load`](Self::load)
    fn decode_locations(
        lands: &[u8; MAPSQUARE],
        buf: &mut Packet,
        cache: &CacheStore,
        marks: &mut ZoneMarks,
        originx: u16,
        originz: u16,
        out: &mut Vec<MapLocEntry>,
    ) {
        let mut id: i32 = -1;
        let mut id_offset = buf.gsmart1or2();
        while id_offset != 0 {
            id += id_offset;

            let mut coord: i32 = 0;
            let mut coord_offset = buf.gsmart1or2();

            while coord_offset != 0 {
                coord += coord_offset - 1;
                let packed = coord as u16;

                let info = buf.g1();
                coord_offset = buf.gsmart1or2();

                out.push(MapLocEntry {
                    id: id as u16,
                    coord: packed,
                    info,
                });

                let Some(loc_type) = cache.locs.get_by_id(id as u16) else {
                    continue;
                };

                let shape = LocShape::try_from_primitive(info >> 2).unwrap();
                if shape.is_roof() {
                    continue;
                }

                let coord = MapsquareCoordGrid::from(packed);
                let bridge = Self::bridge_level(lands, coord.x(), coord.y(), coord.z());
                if bridge < 0 {
                    continue;
                }

                let extent = loc_type.width.max(loc_type.length).max(1) as u16;
                let absolute_x = originx + coord.x() as u16;
                let absolute_z = originz + coord.z() as u16;
                let (x0, z0, x1, z1) = loc_zone_rect(shape, absolute_x, absolute_z, extent);
                marks.mark_rect(x0, z0, x1, z1, bridge as u8);
            }
            id_offset = buf.gsmart1or2();
        }
    }

    /// Decodes NPC spawn entries from the mapsquare buffer into a buffered
    /// list and marks the zones covered by each spawn's occupancy footprint.
    ///
    /// # Arguments
    /// * `buf` - The decompressed mapsquare NPC data.
    /// * `cache` - The game cache for NPC type lookups.
    /// * `marks` - The world zone marks to record spawn footprints into.
    /// * `originx` - The mapsquare origin X in absolute tile coordinates.
    /// * `originz` - The mapsquare origin Z in absolute tile coordinates.
    /// * `out` - The output vector receiving the decoded entries in file order.
    ///
    /// # Call Stack
    /// **Called by:** [`load`](Self::load)
    fn decode_npcs(
        buf: &mut Packet,
        cache: &CacheStore,
        marks: &mut ZoneMarks,
        originx: u16,
        originz: u16,
        out: &mut Vec<MapNpcEntry>,
    ) {
        let mut id: i32 = -1;
        let mut id_offset = buf.gsmart1or2();
        while id_offset != 0 {
            id += id_offset;

            let mut coord: i32 = 0;
            let mut coord_offset = buf.gsmart1or2();

            while coord_offset != 0 {
                coord += coord_offset - 1;
                let packed = coord as u16;

                coord_offset = buf.gsmart1or2();

                out.push(MapNpcEntry {
                    id: id as u16,
                    coord: packed,
                });

                let Some(npc) = cache.npcs.get_by_id(id as u16) else {
                    continue;
                };

                let coord = MapsquareCoordGrid::from(packed);
                let size = npc.size.max(1) as u16;
                let absolute_x = originx + coord.x() as u16;
                let absolute_z = originz + coord.z() as u16;
                marks.mark_rect(
                    absolute_x,
                    absolute_z,
                    absolute_x + size - 1,
                    absolute_z + size - 1,
                    coord.y(),
                );
            }
            id_offset = buf.gsmart1or2();
        }
    }

    /// Decodes ground object spawn entries from the mapsquare buffer into a
    /// buffered list and marks each spawn's zone.
    ///
    /// # Arguments
    /// * `buf` - The decompressed mapsquare object data.
    /// * `marks` - The world zone marks to record spawn tiles into.
    /// * `originx` - The mapsquare origin X in absolute tile coordinates.
    /// * `originz` - The mapsquare origin Z in absolute tile coordinates.
    /// * `out` - The output vector receiving the decoded entries in file order.
    ///
    /// # Call Stack
    /// **Called by:** [`load`](Self::load)
    fn decode_objs(
        buf: &mut Packet,
        marks: &mut ZoneMarks,
        originx: u16,
        originz: u16,
        out: &mut Vec<MapObjEntry>,
    ) {
        let mut id: i32 = -1;
        let mut id_offset = buf.gsmart1or2();
        while id_offset != 0 {
            id += id_offset;

            let mut coord: i32 = 0;
            let mut coord_offset = buf.gsmart1or2();

            while coord_offset != 0 {
                coord += coord_offset - 1;
                let packed = coord as u16;

                let count = buf.g2();
                coord_offset = buf.gsmart1or2();

                out.push(MapObjEntry {
                    id: id as u16,
                    coord: packed,
                    count,
                });

                let coord = MapsquareCoordGrid::from(packed);
                marks.mark(
                    originx + coord.x() as u16,
                    originz + coord.z() as u16,
                    coord.y(),
                );
            }
            id_offset = buf.gsmart1or2();
        }
    }

    /// Marks the zone of every coordinate authored in a cache config: enum
    /// keys, values, and defaults of `coord` type; `coord`-typed param
    /// defaults and the param values attached to npc/loc/obj/struct types;
    /// and dbrow columns of `coord` type.
    ///
    /// These are the teleport and forced-movement destinations scripts can
    /// reach (fishing spot movement enums, agility `fail_coord`/`end_coord`
    /// loc params, macro event teleports, ...), and they can sit on otherwise
    /// skippable zones. Teleports silently refuse unallocated zones, so
    /// every authored coordinate keeps its zone allocated.
    ///
    /// # Arguments
    /// * `cache` - The game cache providing the config types.
    /// * `marks` - The world zone marks to record destinations into.
    ///
    /// # Call Stack
    /// **Called by:** [`load`](Self::load)
    fn mark_config_coords(cache: &CacheStore, marks: &mut ZoneMarks) {
        for enum_type in cache.enums.types.iter() {
            let coord_keys = enum_type.inputtype == ScriptVarType::Coord;
            let coord_values = enum_type.outputtype == ScriptVarType::Coord;
            if !coord_keys && !coord_values {
                continue;
            }
            if coord_values {
                marks.mark_packed(enum_type.default_int);
            }
            for (key, value) in enum_type.values.iter() {
                if coord_keys {
                    marks.mark_packed(*key);
                }
                if coord_values && let ParamValue::Int(packed) = value {
                    marks.mark_packed(*packed);
                }
            }
        }

        let coord_params: Vec<i32> = cache
            .params
            .types
            .iter()
            .filter(|param| param.var_type == ScriptVarType::Coord)
            .map(|param| {
                marks.mark_packed(param.default_int);
                param.id as i32
            })
            .collect();

        if !coord_params.is_empty() {
            let param_maps = (cache.npcs.types.iter().filter_map(|t| t.params.as_deref()))
                .chain(cache.locs.types.iter().filter_map(|t| t.params.as_deref()))
                .chain(cache.objs.types.iter().filter_map(|t| t.params.as_deref()))
                .chain(
                    cache
                        .structs
                        .types
                        .iter()
                        .filter_map(|t| t.params.as_deref()),
                );
            for params in param_maps {
                for id in &coord_params {
                    if let Some(ParamValue::Int(packed)) = params.get(id) {
                        marks.mark_packed(*packed);
                    }
                }
            }
        }

        for row in cache.dbrows.types.iter() {
            let (Some(types), Some(columns)) = (&row.types, &row.columns) else {
                continue;
            };
            for (column_types, column) in types.iter().zip(columns.iter()) {
                let Some(values) = column else {
                    continue;
                };
                if !column_types.contains(&(ScriptVarType::Coord as u8)) {
                    continue;
                }
                for (i, value) in values.iter().enumerate() {
                    if column_types[i % column_types.len()] == ScriptVarType::Coord as u8
                        && let DbRowValue::Int(packed) = value
                    {
                        marks.mark_packed(*packed);
                    }
                }
            }
        }
    }

    /// Applies floor and roof collision for a mapsquare to the `rsmod`
    /// collision system, allocating only zones marked as holding content.
    ///
    /// Blocked tiles get floor collision, and tiles with the `LINK_BELOW`
    /// flag shift their collision down one level (bridge logic). Writes into
    /// unmarked zones are suppressed: an unallocated zone already reads back
    /// blocked, and letting a write through would allocate the zone filled
    /// with `Open`, turning the rest of it walkable.
    ///
    /// On a free-to-play world, collision and zone allocation are skipped for
    /// tiles that are neither free-to-play nor bordering free-to-play land.
    ///
    /// # Arguments
    /// * `members` - Whether this world loads members-only areas.
    /// * `lands` - The tile flags decoded by [`decode_ground`](Self::decode_ground).
    /// * `marks` - The world zone marks computed by the decode pass.
    /// * `originx` - The mapsquare origin X in absolute tile coordinates.
    /// * `originz` - The mapsquare origin Z in absolute tile coordinates.
    ///
    /// # Side Effects
    /// * Allocates zones and sets floor/roof collision via `rsmod`.
    ///
    /// # Call Stack
    /// **Called by:** [`load`](Self::load)
    fn apply_ground(
        members: bool,
        lands: &[u8; MAPSQUARE],
        marks: &ZoneMarks,
        originx: u16,
        originz: u16,
    ) {
        for y in 0..Y {
            for x in 0..X {
                let coordx = originx + x as u16;
                for z in 0..Z {
                    let coord = MapsquareCoordGrid::new(x as u8, y as u8, z as u8);
                    let coordz = originz + z as u16;

                    if !members
                        && !rsmod::is_zone_flagged(coordx, coordz, y as u8, ZoneFlag::Free as u8)
                        && !rsmod::borders_zone_flag(coordx, coordz, y as u8, ZoneFlag::Free as u8)
                    {
                        continue;
                    }

                    if x % 7 == 0 && z % 7 == 0 && marks.is_marked(coordx, coordz, y as u8) {
                        rsmod::allocate_if_absent(coordx, coordz, y as u8);
                    }

                    let land = lands[coord.packed() as usize];

                    if (land & REMOVE_ROOFS) != OPEN && marks.is_marked(coordx, coordz, y as u8) {
                        rsmod::change_roof(coordx, coordz, y as u8, true);
                    }

                    if (land & BLOCK_MAP_SQUARE) != BLOCK_MAP_SQUARE {
                        continue;
                    }

                    let bridge = Self::bridge_level(lands, x as u8, y as u8, z as u8);
                    if bridge < 0 || !marks.is_marked(coordx, coordz, bridge as u8) {
                        continue;
                    }

                    rsmod::change_floor(coordx, coordz, bridge as u8, true);
                }
            }
        }
    }

    /// Applies the buffered location entries: adds their collision to the
    /// `rsmod` system and registers them as static locs in the zone map.
    ///
    /// Handles bridge-level adjustments using the `lands` tile flags. On a
    /// free-to-play world, locs that are neither free-to-play nor bordering
    /// free-to-play land are skipped. Every loc is applied only if all zones
    /// in its write rectangle are marked: locs that marked in the decode pass
    /// always pass, while roof pieces are skipped wherever no flooring or
    /// other content justified the zone.
    ///
    /// # Arguments
    /// * `members` - Whether this world loads members-only areas.
    /// * `lands` - The tile flags decoded by [`decode_ground`](Self::decode_ground).
    /// * `locs` - The loc entries decoded by [`decode_locations`](Self::decode_locations).
    /// * `cache` - The game cache for loc type lookups.
    /// * `zones` - The zone map to add static locs to.
    /// * `marks` - The world zone marks computed by the decode pass.
    /// * `originx` - The mapsquare origin X in absolute tile coordinates.
    /// * `originz` - The mapsquare origin Z in absolute tile coordinates.
    ///
    /// # Side Effects
    /// * Applies loc collision via [`change_loc_collision`].
    /// * Adds static locs to the zone map.
    ///
    /// # Call Stack
    /// **Called by:** [`load`](Self::load)
    /// **Calls:** [`change_loc_collision`]
    #[allow(clippy::too_many_arguments)]
    fn apply_locations(
        members: bool,
        lands: &[u8; MAPSQUARE],
        locs: &[MapLocEntry],
        cache: &CacheStore,
        zones: &mut ZoneMap,
        marks: &ZoneMarks,
        originx: u16,
        originz: u16,
    ) {
        for entry in locs {
            let coord = MapsquareCoordGrid::from(entry.coord);

            let absolute_x = originx + coord.x() as u16;
            let absolute_z = originz + coord.z() as u16;
            if !members
                && !rsmod::is_zone_flagged(absolute_x, absolute_z, coord.y(), ZoneFlag::Free as u8)
                && !rsmod::borders_zone_flag(
                    absolute_x,
                    absolute_z,
                    coord.y(),
                    ZoneFlag::Free as u8,
                )
            {
                continue;
            }

            let bridge = Self::bridge_level(lands, coord.x(), coord.y(), coord.z());
            if bridge < 0 {
                continue;
            }

            let Some(loc_type) = cache.locs.get_by_id(entry.id) else {
                continue;
            };

            let width = loc_type.width;
            let length = loc_type.length;

            let shape = LocShape::try_from_primitive(entry.info >> 2).unwrap();
            let layer = shape.layer();
            let angle = LocAngle::try_from_primitive(entry.info & 0x3).unwrap();

            let extent = width.max(length).max(1) as u16;
            let (x0, z0, x1, z1) = loc_zone_rect(shape, absolute_x, absolute_z, extent);
            if !marks.is_rect_marked(x0, z0, x1, z1, bridge as u8) {
                continue;
            }

            let coord = CoordGrid::new(absolute_x, bridge as u8, absolute_z);

            if loc_type.blockwalk {
                change_loc_collision(
                    shape,
                    layer,
                    angle,
                    loc_type.blockrange,
                    length,
                    width,
                    loc_type.active,
                    coord,
                    true,
                );
            }

            if let Some(active) = loc_type.active
                && active
            {
                let loc = Loc::new(
                    coord,
                    EntityLifeTime::Respawn,
                    entry.id,
                    shape,
                    angle,
                    loc_type.blockwalk,
                    loc_type.blockrange,
                    width,
                    length,
                );

                zones
                    .zone_mut(coord.x(), coord.y(), coord.z())
                    .add_static_loc(loc);
            }
        }
    }

    /// Computes the bridge-adjusted floor level for a tile.
    ///
    /// A tile is "bridged" when the tile directly above the ground floor
    /// (y = 1) has the `LINK_BELOW` flag set, which shifts content down one
    /// level. Returns the level as an `i8` so callers can detect the
    /// underflow case (a negative result) and skip it.
    fn bridge_level(lands: &[u8; MAPSQUARE], x: u8, y: u8, z: u8) -> i8 {
        let coord = MapsquareCoordGrid::new(x, 1, z);
        let bridged = (lands[coord.packed() as usize] & LINK_BELOW) == LINK_BELOW;
        if bridged { y as i8 - 1 } else { y as i8 }
    }

    /// Creates `ActiveNpc` instances for the buffered NPC spawn entries.
    ///
    /// Members-only NPCs are skipped when `members` is false, as are all NPCs
    /// outside free-to-play areas on a free-to-play world.
    ///
    /// # Arguments
    /// * `members` - Whether members-only NPCs should be spawned.
    /// * `npcs` - The spawn entries decoded by [`decode_npcs`](Self::decode_npcs).
    /// * `cache` - The game cache for NPC type lookups.
    /// * `out` - The output vector to receive spawned NPCs.
    /// * `originx` - The mapsquare origin X in absolute tile coordinates.
    /// * `originz` - The mapsquare origin Z in absolute tile coordinates.
    ///
    /// # Call Stack
    /// **Called by:** [`load`](Self::load)
    /// **Calls:** [`ActiveNpc::new`]
    fn spawn_npcs(
        members: bool,
        npcs: &[MapNpcEntry],
        cache: &CacheStore,
        out: &mut Vec<ActiveNpc>,
        originx: u16,
        originz: u16,
    ) {
        for entry in npcs {
            let coord = MapsquareCoordGrid::from(entry.coord);

            let Some(npc) = cache.npcs.get_by_id(entry.id) else {
                continue;
            };

            if npc.members && !members {
                continue;
            }

            let coord = CoordGrid::new(
                originx + coord.x() as u16,
                coord.y(),
                originz + coord.z() as u16,
            );

            if !members
                && !rsmod::is_zone_flagged(coord.x(), coord.z(), coord.y(), ZoneFlag::Free as u8)
            {
                continue;
            }

            let vars = VarSet::new(cache.varns.types.iter().map(|v| v.var_type));
            out.push(ActiveNpc::new(entry.id, 0, coord, npc.size, vars, cache));
        }
    }

    /// Adds the buffered ground object spawn entries as static objects in the
    /// zone map.
    ///
    /// Members-only objects are skipped when `members` is false, as are all
    /// objects outside free-to-play areas on a free-to-play world.
    ///
    /// # Arguments
    /// * `members` - Whether members-only objects should be spawned.
    /// * `objs` - The spawn entries decoded by [`decode_objs`](Self::decode_objs).
    /// * `cache` - The game cache for object type lookups.
    /// * `zones` - The zone map to add static objs to.
    /// * `originx` - The mapsquare origin X in absolute tile coordinates.
    /// * `originz` - The mapsquare origin Z in absolute tile coordinates.
    ///
    /// # Side Effects
    /// * Adds static objs to the zone map.
    ///
    /// # Call Stack
    /// **Called by:** [`load`](Self::load)
    fn spawn_objs(
        members: bool,
        objs: &[MapObjEntry],
        cache: &CacheStore,
        zones: &mut ZoneMap,
        originx: u16,
        originz: u16,
    ) {
        for entry in objs {
            let coord = MapsquareCoordGrid::from(entry.coord);

            let Some(obj) = cache.objs.get_by_id(entry.id) else {
                continue;
            };

            if obj.members && !members {
                continue;
            }

            let coord = CoordGrid::new(
                originx + coord.x() as u16,
                coord.y(),
                originz + coord.z() as u16,
            );

            if !members
                && !rsmod::is_zone_flagged(coord.x(), coord.z(), coord.y(), ZoneFlag::Free as u8)
            {
                continue;
            }

            let obj = Obj::new(coord, EntityLifeTime::Respawn, obj.id, entry.count as u32);

            zones
                .zone_mut(coord.x(), coord.y(), coord.z())
                .add_static_obj(obj);
        }
    }
}

/// Applies or removes collision for a location entity based on the collision
/// flags stored on the loc itself.
///
/// Only modifies collision if the loc has `blockwalk` set. The `blockwalk`,
/// `blockrange`, `width` and `length` fields are all bit-packed into the loc (see
/// [`Loc`]), so no cache lookup is needed -- a [`revert`](Loc::revert) restores
/// them along with the rest of the base state.
///
/// # Arguments
/// * `loc` - The location entity whose collision is being changed.
/// * `coord` - The tile coordinate of the location.
/// * `add` - `true` to add collision, `false` to remove it.
///
/// # Call Stack
/// **Calls:** [`change_loc_collision`]
pub fn apply_loc_collision(loc: &Loc, coord: CoordGrid, add: bool) {
    if !loc.blockwalk() {
        return;
    }
    change_loc_collision(
        loc.shape(),
        loc.layer(),
        loc.angle(),
        loc.blockrange(),
        loc.length(),
        loc.width(),
        Some(true),
        coord,
        add,
    );
}

/// Adds or removes collision flags for a single location on the collision
/// map, dispatching to the appropriate `rsmod` function based on the
/// location's layer.
///
/// # Arguments
/// * `shape` - The loc shape.
/// * `layer` - The loc layer (Wall, WallDecor, Ground, GroundDecor).
/// * `angle` - The rotation angle of the loc.
/// * `blockrange` - Whether the loc blocks ranged projectiles.
/// * `length` - The loc's Z-axis tile size.
/// * `width` - The loc's X-axis tile size.
/// * `active` - If `Some(true)` for GroundDecor, a floor collision flag is
///   set.
/// * `coord` - The tile coordinate.
/// * `add` - `true` to add collision, `false` to remove it.
///
/// # Side Effects
/// * Calls `rsmod::change_wall`, `rsmod::change_loc`, or
///   `rsmod::change_floor` depending on the layer.
///
/// # Call Stack
/// **Called by:** [`GameMap::apply_locations`], [`apply_loc_collision`]
#[allow(clippy::too_many_arguments)]
pub fn change_loc_collision(
    shape: LocShape,
    layer: LocLayer,
    angle: LocAngle,
    blockrange: bool,
    length: u8,
    width: u8,
    active: Option<bool>,
    coord: CoordGrid,
    add: bool,
) {
    let x = coord.x();
    let y = coord.y();
    let z = coord.z();
    match layer {
        LocLayer::Wall => {
            rsmod::change_wall(x, z, y, angle as u8, shape as i8, blockrange, false, add)
        }
        LocLayer::WallDecor => {}
        LocLayer::Ground => match angle {
            LocAngle::North | LocAngle::South => {
                rsmod::change_loc(x, z, y, length, width, blockrange, false, add)
            }
            LocAngle::West | LocAngle::East => {
                rsmod::change_loc(x, z, y, width, length, blockrange, false, add)
            }
        },
        LocLayer::GroundDecor => {
            if active == Some(true) {
                rsmod::change_floor(x, z, y, add);
            }
        }
    }
}
