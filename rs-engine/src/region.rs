use crate::console::home_for;
use crate::engine::{Engine, PendingZoneEvent};
use crate::game_map::{
    BLOCK_MAP_SQUARE, GameMap, LINK_BELOW, MAPSQUARE, REMOVE_ROOFS, SquareTemplate,
    apply_loc_collision, change_loc_collision,
};
use crate::phases::shared::EntityId;
use crate::player_save::PlayerProfile;
use num_enum::TryFromPrimitive;
use rs_entity::{EntityLifeTime, Loc, LocSerial};
use rs_grid::mapsquare_coord::MapsquareCoordGrid;
use rs_grid::{CoordGrid, ZoneCoordGrid};
use rs_pack::cache::CacheStore;
use rs_pack::types::{BlockWalk, LocAngle, LocShape};
use rs_vm::ScriptError;
use rsmod::rsmod::flag::zone_flag::ZoneFlag;
use rustc_hash::FxHashMap;
use std::mem;

/// Where region space starts: no static map lies at or east of this x.
pub const REGION_X: u16 = 6400;
const PADDING: u16 = 64;
const LARGE_Z: u16 = 5248;
const SMALL_SLOTS: usize = 51 * 27;
const SLOTS: usize = SMALL_SLOTS + 25 * 28;
const QUARANTINE: u32 = 500;
/// How long a region stays with nobody in it, counting from its creation
/// until someone comes, before it is freed: time for a script to build one
/// and bring its players.
pub(crate) const FREE_AFTER: u32 = 50;
const NONE: u32 = u32::MAX;
const SQUARES: usize = 64;

/// How a size of region tiles its part of region space: 128x128 regions
/// south of z 5248, 320x320 north of it, each 64 tiles from the next.
struct Layout {
    tiles: u16,
    columns: u16,
    rows: u16,
    z: u16,
    first: usize,
}

const SMALL: Layout = Layout {
    tiles: 128,
    columns: 51,
    rows: 27,
    z: 0,
    first: 0,
};

const LARGE: Layout = Layout {
    tiles: 320,
    columns: 25,
    rows: 28,
    z: LARGE_Z,
    first: SMALL_SLOTS,
};

impl Layout {
    const fn of(slot: usize) -> &'static Layout {
        if slot < SMALL_SLOTS { &SMALL } else { &LARGE }
    }

    const fn stride(&self) -> u16 {
        self.tiles + PADDING
    }
}

/// The south-west tile of `slot` and the number of zones along its sides.
const fn bounds(slot: usize) -> (u16, u16, u16) {
    let layout = Layout::of(slot);
    let index = (slot - layout.first) as u16;
    let (column, row) = (index % layout.columns, index / layout.columns);
    (
        REGION_X + PADDING + column * layout.stride(),
        layout.z + PADDING + row * layout.stride(),
        layout.tiles >> 3,
    )
}

/// The slot whose region area holds tile (`x`, `z`).
pub const fn slot_at(x: u16, z: u16) -> Option<usize> {
    if x < REGION_X + PADDING {
        return None;
    }
    let layout = if z < LARGE_Z { &SMALL } else { &LARGE };
    if z < layout.z + PADDING {
        return None;
    }
    let (dx, dz) = (x - REGION_X - PADDING, z - layout.z - PADDING);
    let stride = layout.stride();
    let (column, row) = (dx / stride, dz / stride);
    if column >= layout.columns
        || row >= layout.rows
        || dx % stride >= layout.tiles
        || dz % stride >= layout.tiles
    {
        return None;
    }
    Some(layout.first + (row * layout.columns + column) as usize)
}

const fn template(level: u8, zone_x: u16, zone_z: u16, rotation: u8) -> u32 {
    ((level as u32) << 24)
        | ((zone_x as u32) << 14)
        | ((zone_z as u32) << 3)
        | ((rotation as u32) << 1)
}

const fn template_level(template: u32) -> u8 {
    (template >> 24 & 0x3) as u8
}

const fn template_zone_x(template: u32) -> u16 {
    (template >> 14 & 0x3FF) as u16
}

const fn template_zone_z(template: u32) -> u16 {
    (template >> 3 & 0x7FF) as u16
}

const fn template_rotation(template: u32) -> u8 {
    (template >> 1 & 0x3) as u8
}

/// The mapsquare a template's zone lies in.
pub(crate) const fn template_square(template: u32) -> (u16, u16) {
    (
        template_zone_x(template) >> 3,
        template_zone_z(template) >> 3,
    )
}

/// The tile of a zone that turning it `rotation` quarter turns clockwise puts
/// on tile (`x`, `z`), undoing how the client places tiles
/// (`AClass13.method623`, `Class244.method1589`).
const fn unrotate_tile(x: u8, z: u8, rotation: u8) -> (u8, u8) {
    match rotation & 3 {
        0 => (x, z),
        1 => (7 - z, x),
        2 => (7 - x, 7 - z),
        _ => (z, 7 - x),
    }
}

/// The south-west tile of a `width` x `length` loc at `angle` whose zone is
/// turned `rotation` quarter turns clockwise, as the client places it
/// (`Class76.method521`, `Class175.method1184`).
const fn rotate_loc(x: u8, z: u8, width: u8, length: u8, angle: u8, rotation: u8) -> (i32, i32) {
    let (width, length) = if angle & 1 == 1 {
        (length as i32, width as i32)
    } else {
        (width as i32, length as i32)
    };
    let (x, z) = (x as i32, z as i32);
    match rotation & 3 {
        0 => (x, z),
        1 => (z, 8 - x - width),
        2 => (8 - x - width, 8 - z - length),
        _ => (8 - z - length, x),
    }
}

/// A region: the levels and zones it was created with, where its players
/// exit to, the zones copied onto its slot of region space, the active locs
/// each zone column put on the map, how many zones past its own zone the
/// longest loc copied so far can block, and the last tick a player was in it
/// (its creation until one is).
pub struct Region {
    pub uid: u32,
    zones: u16,
    levels: u8,
    east: u16,
    north: u16,
    exit: Option<CoordGrid>,
    templates: Box<[u32]>,
    statics: Box<[Vec<(CoordGrid, LocSerial)>]>,
    spill: u16,
    seen: u32,
}

impl Region {
    fn template(&self, level: u8, zone_x: u16, zone_z: u16) -> u32 {
        let zones = self.zones as usize;
        self.templates[(level as usize * zones + zone_x as usize) * zones + zone_z as usize]
    }

    fn template_mut(&mut self, level: u8, zone_x: u16, zone_z: u16) -> &mut u32 {
        let zones = self.zones as usize;
        &mut self.templates[(level as usize * zones + zone_x as usize) * zones + zone_z as usize]
    }

    fn column(&self, zone_x: u16, zone_z: u16) -> usize {
        zone_x as usize * self.zones as usize + zone_z as usize
    }

    /// The templates of column (`zone_x`, `zone_z`), by level.
    fn templates_at(&self, zone_x: u16, zone_z: u16) -> [u32; 4] {
        [0, 1, 2, 3].map(|level| self.template(level, zone_x, zone_z))
    }
}

#[derive(Default)]
struct Slot {
    region: Option<Region>,
    generation: u32,
    freed: Option<u32>,
}

/// Every slot of region space, handed out round robin so a freed slot is
/// the last one reused, and never within [`QUARANTINE`] ticks of its freeing.
pub struct Regions {
    slots: Box<[Slot]>,
    cursors: [usize; 2],
    squares: FxHashMap<(u16, u16), SquareTemplate>,
}

impl Default for Regions {
    fn default() -> Regions {
        Regions {
            slots: (0..SLOTS).map(|_| Slot::default()).collect(),
            cursors: [0, 0],
            squares: FxHashMap::default(),
        }
    }
}

impl Regions {
    /// Forgets the mapsquares decoded for copying, after the cache changed.
    pub fn forget_squares(&mut self) {
        self.squares.clear();
    }

    /// Every live region's uid.
    pub fn uids(&self) -> Vec<u32> {
        self.slots
            .iter()
            .filter_map(|slot| slot.region.as_ref().map(|region| region.uid))
            .collect()
    }

    /// The region `uid` names, if it still exists.
    pub fn get(&self, uid: u32) -> Option<&Region> {
        self.slots
            .get((uid & 0xFFF) as usize)
            .and_then(|slot| slot.region.as_ref())
            .filter(|region| region.uid == uid)
    }

    /// The region covering tile (`x`, `z`).
    pub fn find(&self, x: u16, z: u16) -> Option<&Region> {
        self.slots[slot_at(x, z)?].region.as_ref()
    }

    /// The copied zone's template at absolute zone (`zone_x`, `zone_z`) on
    /// `level`, as REBUILD_REGION sends it.
    pub fn template_at(&self, zone_x: u16, level: u8, zone_z: u16) -> Option<u32> {
        let (x, z) = (zone_x << 3, zone_z << 3);
        let slot = slot_at(x, z)?;
        let region = self.slots[slot].region.as_ref()?;
        let (ox, oz, _) = bounds(slot);
        let template = region.template(level, (x - ox) >> 3, (z - oz) >> 3);
        (template != NONE).then_some(template)
    }

    /// The static tile a region tile was copied from.
    pub fn source(&self, coord: CoordGrid) -> Option<CoordGrid> {
        let slot = slot_at(coord.x(), coord.z())?;
        let region = self.slots[slot].region.as_ref()?;
        let (ox, oz, _) = bounds(slot);
        let (lx, lz) = (coord.x() - ox, coord.z() - oz);
        let template = region.template(coord.y(), lx >> 3, lz >> 3);
        if template == NONE {
            return None;
        }
        let (x, z) = unrotate_tile((lx & 7) as u8, (lz & 7) as u8, template_rotation(template));
        Some(CoordGrid::new(
            (template_zone_x(template) << 3) + x as u16,
            template_level(template),
            (template_zone_z(template) << 3) + z as u16,
        ))
    }

    /// Takes a slot for a region of `levels` levels and `east` x `north`
    /// zones: a small one if it fits, else a large one.
    fn allocate(&mut self, levels: u8, east: u16, north: u16, clock: u32) -> Option<u32> {
        let small = SMALL.tiles >> 3;
        let (layout, cursor) = if east > small || north > small {
            (&LARGE, &mut self.cursors[1])
        } else {
            (&SMALL, &mut self.cursors[0])
        };
        let count = (layout.columns * layout.rows) as usize;
        let index = (0..count).map(|i| (*cursor + i) % count).find(|&index| {
            let slot = &self.slots[layout.first + index];
            slot.region.is_none()
                && slot
                    .freed
                    .is_none_or(|freed| clock.wrapping_sub(freed) >= QUARANTINE)
        })?;
        *cursor = (index + 1) % count;
        let id = layout.first + index;
        let slot = &mut self.slots[id];
        slot.generation = (slot.generation + 1) & 0xFFFFF;
        let zones = layout.tiles >> 3;
        let uid = (slot.generation << 12) | id as u32;
        slot.region = Some(Region {
            uid,
            zones,
            levels,
            east,
            north,
            exit: None,
            templates: vec![NONE; (zones as usize * zones as usize) << 2].into_boxed_slice(),
            statics: (0..zones as usize * zones as usize)
                .map(|_| Vec::new())
                .collect(),
            spill: 1,
            seen: clock,
        });
        Some(uid)
    }

    fn live(&self, uid: u32) -> Result<usize, ScriptError> {
        self.get(uid)
            .map(|_| (uid & 0xFFF) as usize)
            .ok_or_else(|| ScriptError::Runtime("region no longer exists".into()))
    }
}

impl Engine {
    /// The exit a player at `coord` saves with its tile: in region space,
    /// its way out (see [`way_out`](Self::way_out)), which
    /// [`place_returning`] puts it on when it comes back.
    pub(crate) fn saved_exit(
        &self,
        coord: CoordGrid,
        entry: Option<CoordGrid>,
    ) -> Option<CoordGrid> {
        if coord.x() < REGION_X {
            return None;
        }
        self.way_out(coord, entry)
    }

    /// Where a player in region space at `coord` leaves to: the exit coord
    /// of the region it stands in, else `entry`, the map tile it came from,
    /// if a player can stand there.
    fn way_out(&self, coord: CoordGrid, entry: Option<CoordGrid>) -> Option<CoordGrid> {
        slot_at(coord.x(), coord.z())
            .and_then(|slot| self.region_exit(slot))
            .or_else(|| {
                entry.filter(|entry| {
                    entry.x() < REGION_X
                        && rsmod::is_zone_allocated(entry.x(), entry.z(), entry.y())
                })
            })
    }

    /// Where the players of the region in `slot` exit to, if it has an exit
    /// a player can stand on.
    fn region_exit(&self, slot: usize) -> Option<CoordGrid> {
        self.regions.slots[slot]
            .region
            .as_ref()?
            .exit
            .filter(|exit| rsmod::is_zone_allocated(exit.x(), exit.z(), exit.y()))
    }

    /// The tile painted areas are read from for `coord`: a region tile's
    /// source tile.
    pub(crate) fn area_tile(&self, coord: CoordGrid) -> CoordGrid {
        if coord.x() < REGION_X {
            return coord;
        }
        self.regions.source(coord).unwrap_or(coord)
    }

    /// Whether something at `from` may be put at `to`. A tile in a region
    /// slot needs a live region there, entered from inside it or by a script
    /// that selected it: a coord kept from a freed region never reaches the
    /// region that took its slot, nor does an offset from another region.
    pub(crate) fn region_allows(
        &self,
        from: CoordGrid,
        to: CoordGrid,
        selected: Option<u32>,
    ) -> bool {
        let Some(slot) = slot_at(to.x(), to.z()) else {
            return true;
        };
        let Some(region) = &self.regions.slots[slot].region else {
            return false;
        };
        selected == Some(region.uid) || slot_at(from.x(), from.z()) == Some(slot)
    }

    /// Takes a region of `levels` levels and `east` x `north` zones, built
    /// from the south-west of a small slot if it fits one, else a large one.
    pub(crate) fn region_create(
        &mut self,
        levels: i32,
        east: i32,
        north: i32,
    ) -> Result<u32, ScriptError> {
        let zones = (LARGE.tiles >> 3) as i32;
        if !(1..=4).contains(&levels)
            || !(1..=zones).contains(&east)
            || !(1..=zones).contains(&north)
        {
            return Err(ScriptError::Runtime(format!(
                "region_create: no region of {levels} levels and {east}x{north} zones"
            )));
        }
        self.regions
            .allocate(levels as u8, east as u16, north as u16, self.clock)
            .ok_or_else(|| ScriptError::Runtime("region_create: no region is free".into()))
    }

    /// The coord `x`, `z` tiles from region `uid`'s south-west corner on
    /// `level`.
    pub(crate) fn region_coord(
        &self,
        uid: u32,
        level: i32,
        x: i32,
        z: i32,
    ) -> Result<CoordGrid, ScriptError> {
        let (ox, oz, zones) = bounds(self.regions.live(uid)?);
        let tiles = (zones << 3) as i32;
        if !(0..4).contains(&level) || !(0..tiles).contains(&x) || !(0..tiles).contains(&z) {
            return Err(ScriptError::Runtime(format!(
                "region_getcoord: {level}, {x}, {z} is outside the region"
            )));
        }
        Ok(CoordGrid::new(ox + x as u16, level as u8, oz + z as u16))
    }

    /// Copies the zone holding `source` onto region `uid`'s zone `east`,
    /// `north` on `level`, turned `rotation` quarter turns clockwise, with
    /// the source zone's free-to-play mark, and rebuilds what the copy
    /// reaches.
    pub(crate) fn region_set(
        &mut self,
        uid: u32,
        level: i32,
        east: i32,
        north: i32,
        source: CoordGrid,
        rotation: i32,
    ) -> Result<(), ScriptError> {
        let slot = self.regions.live(uid)?;
        let region = self.regions.slots[slot].region.as_ref().unwrap();
        if !(0..region.levels as i32).contains(&level)
            || !(0..region.east as i32).contains(&east)
            || !(0..region.north as i32).contains(&north)
        {
            return Err(ScriptError::Runtime(format!(
                "region_set: zone {east}, {north} on level {level} is outside the region"
            )));
        }
        if source.x() >= REGION_X {
            return Err(ScriptError::Runtime(format!(
                "region_set: {source:?} is not on the map"
            )));
        }
        let rotation = turns("region_set", rotation)?;
        let (east, north) = (east as u16, north as u16);
        let before = region.templates_at(east, north);
        self.put_zone(
            slot,
            (level as u8, east, north),
            (source.y(), source.x() >> 3, source.z() >> 3),
            rotation,
        );
        self.rebuild_region(slot, &[(east, north, before)]);
        Ok(())
    }

    /// Sets where a player who leaves the game inside region `uid` comes
    /// back, and where one left on no floor goes, rather than the tile each
    /// came from.
    pub(crate) fn region_set_exit(&mut self, uid: u32, exit: CoordGrid) -> Result<(), ScriptError> {
        let slot = self.regions.live(uid)?;
        if exit.x() >= REGION_X {
            return Err(ScriptError::Runtime(format!(
                "region_setexitcoord: {exit:?} is in region space"
            )));
        }
        self.regions.slots[slot].region.as_mut().unwrap().exit = Some(exit);
        Ok(())
    }

    /// Puts static zone `from` (level, zone x, zone z) turned `rotation` on
    /// zone `to` of the region in `slot`, with the zone's free-to-play mark.
    fn put_zone(&mut self, slot: usize, to: (u8, u16, u16), from: (u8, u16, u16), rotation: u8) {
        let (ox, oz, _) = bounds(slot);
        let ((level, zone_x, zone_z), (from_level, from_x, from_z)) = (to, from);
        let region = self.regions.slots[slot].region.as_mut().unwrap();
        *region.template_mut(level, zone_x, zone_z) =
            template(from_level, from_x, from_z, rotation);
        let free =
            rsmod::is_zone_flagged(from_x << 3, from_z << 3, from_level, ZoneFlag::Free as u8);
        rsmod::change_zone(
            ox + (zone_x << 3),
            oz + (zone_z << 3),
            level,
            ZoneFlag::Free as u8,
            free,
        );
    }

    /// Builds again, as the client builds them from the same templates, the
    /// zones of `slot` that the `changed` columns reach: their floor and roof
    /// flags and the collision of every loc and entity on them. The active
    /// locs standing on the zones cleared, and those of the zones the changed
    /// columns gained wherever they stand, are put back as their templates
    /// have them; a loc's south-west tile can lie in the zone beside the one
    /// it came from. Other locs and entities keep their state. Each changed
    /// column comes with its templates from before: a column that only
    /// gained zones, none of them moving its other zones' locs a level down,
    /// takes nothing away from the zones around it. Only zones as near the
    /// changed columns as the region's locs reach are visited, so a region
    /// set a zone at a time stays cheap.
    fn rebuild_region(&mut self, slot: usize, changed: &[(u16, u16, [u32; 4])]) {
        let (ox, oz, zones) = bounds(slot);
        let cache = self.cache;
        let mut region = self.regions.slots[slot].region.take().unwrap();
        let mut squares = mem::take(&mut self.regions.squares);
        if squares.len() > SQUARES {
            squares.clear();
        }
        for &(zone_x, zone_z, _) in changed {
            for level in 0..4 {
                let template = region.template(level, zone_x, zone_z);
                if template == NONE {
                    continue;
                }
                let largest = square(cache, &mut squares, template).largest(
                    template_level(template),
                    (template_zone_x(template) & 7) as u8,
                    (template_zone_z(template) & 7) as u8,
                );
                region.spill = region.spill.max((largest as u16 + 7) >> 3);
            }
        }

        // A column only gained zones when every zone it had stays, and a
        // gained level 1 bridges none of the column's older zones' tiles.
        let gained = |zone_x: u16, zone_z: u16, before: &[u32; 4]| {
            let now = region.templates_at(zone_x, zone_z);
            let kept = (0..4).all(|level| before[level] == NONE || before[level] == now[level]);
            let bridges = before[1] == NONE
                && (0..4).any(|level| level != 1 && before[level] != NONE)
                && zone_lands(&squares, now[1]).is_some_and(|lands| {
                    (0..64).any(|tile| lands.at(tile & 7, tile >> 3) & LINK_BELOW != 0)
                });
            kept && !bridges
        };
        let spill = region.spill;
        let span = zones + (spill << 1);
        let at = |x: u16, z: u16| x as usize * span as usize + z as usize;
        // Bit 1 marks the changed columns and bit 2 the zones cleared and
        // built again: the changed columns and the zones their old locs
        // reached. Each later bit reaches `spill` zones past the one before:
        // the zones whose locs can block a cleared zone, then the zones those
        // locs can block.
        let mut reach = vec![0u8; span as usize * span as usize];
        let (mut x0, mut z0, mut x1, mut z1) = (span, span, 0, 0);
        for (zone_x, zone_z, before) in changed {
            let (x, z) = (zone_x + spill, zone_z + spill);
            reach[at(x, z)] |= 1;
            let around = if gained(*zone_x, *zone_z, before) {
                0
            } else {
                spill
            };
            for nx in x - around..=x + around {
                for nz in z - around..=z + around {
                    reach[at(nx, nz)] |= 2;
                }
            }
            (x0, z0, x1, z1) = (x0.min(x), z0.min(z), x1.max(x), z1.max(z));
        }
        let far = spill * 3;
        let (x0, z0) = (x0.saturating_sub(far), z0.saturating_sub(far));
        let (x1, z1) = ((x1 + far).min(span - 1), (z1 + far).min(span - 1));
        for (from, to) in [(2, 4), (4, 8)] {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    if reach[at(x, z)] & from == 0 {
                        continue;
                    }
                    for nx in x.saturating_sub(spill)..(x + spill + 1).min(span) {
                        for nz in z.saturating_sub(spill)..(z + spill + 1).min(span) {
                            reach[at(nx, nz)] |= to;
                        }
                    }
                }
            }
        }
        let tile = |x: u16, z: u16| (ox + (x << 3) - (spill << 3), oz + (z << 3) - (spill << 3));
        let inside =
            |x: u16, z: u16| x >= spill && z >= spill && x - spill < zones && z - spill < zones;

        for x in x0..=x1 {
            for z in z0..=z1 {
                if reach[at(x, z)] & 8 == 0 || !inside(x, z) {
                    continue;
                }
                for level in 0..4 {
                    let template = region.template(level, x - spill, z - spill);
                    if template != NONE {
                        square(cache, &mut squares, template);
                    }
                }
            }
        }

        // The active locs standing on the cleared zones go, to be put back.
        for x in x0..=x1 {
            for z in z0..=z1 {
                if reach[at(x, z)] & 2 == 0 || !inside(x, z) {
                    continue;
                }
                let column = region.column(x - spill, z - spill);
                for (coord, serial) in region.statics[column].drain(..) {
                    let (x, y, z) = (coord.x(), coord.y(), coord.z());
                    if let Some(zone) = self.zones.zones.get_mut(&ZoneCoordGrid::new(x, y, z))
                        && let Some(index) = zone.locs.iter().position(|loc| loc.serial() == serial)
                    {
                        zone.locs.remove(index);
                    }
                }
            }
        }

        let has = |level: i8, x: u16, z: u16| {
            level >= 0
                && inside(x, z)
                && has_ground(region.template(level as u8, x - spill, z - spill))
        };
        // Whether region tiles `x0`, `z0` to `x1`, `z1` touch a zone whose
        // collision was cleared, which only then needs what stands there.
        let cleared = |x0: i32, z0: i32, x1: i32, z1: i32| {
            let edge = (spill as i32) << 3;
            let zone = |tile: i32| ((tile + edge).max(0) >> 3).min(span as i32 - 1) as u16;
            (zone(x0)..=zone(x1)).any(|x| (zone(z0)..=zone(z1)).any(|z| reach[at(x, z)] & 2 != 0))
        };

        for x in x0..=x1 {
            for z in z0..=z1 {
                if reach[at(x, z)] & 2 == 0 {
                    continue;
                }
                let (tx, tz) = tile(x, z);
                for level in 0..4 {
                    rsmod::deallocate_if_present(tx, tz, level);
                    if has(level as i8, x, z) {
                        rsmod::allocate_if_absent(tx, tz, level);
                    }
                }
            }
        }

        for x in x0..=x1 {
            for z in z0..=z1 {
                if reach[at(x, z)] & 2 == 0 || !inside(x, z) {
                    continue;
                }
                let (tx, tz) = tile(x, z);
                let (zone_x, zone_z) = (x - spill, z - spill);
                let above = zone_lands(&squares, region.template(1, zone_x, zone_z));
                for level in 0..4 {
                    if !has(level as i8, x, z) {
                        continue;
                    }
                    let Some(here) = zone_lands(&squares, region.template(level, zone_x, zone_z))
                    else {
                        continue;
                    };
                    for dx in 0..8 {
                        for dz in 0..8 {
                            let flags = here.at(dx as u8, dz as u8);
                            if flags & REMOVE_ROOFS != 0 {
                                rsmod::change_roof(tx + dx, tz + dz, level, true);
                            }
                            if flags & BLOCK_MAP_SQUARE == 0 {
                                continue;
                            }
                            let link = above
                                .as_ref()
                                .map_or(0, |above| above.at(dx as u8, dz as u8));
                            let bridge = if link & LINK_BELOW != 0 {
                                level as i8 - 1
                            } else {
                                level as i8
                            };
                            if has(bridge, x, z) {
                                rsmod::change_floor(tx + dx, tz + dz, bridge as u8, true);
                            }
                        }
                    }
                }
            }
        }

        let tiles = (zones << 3) as i32;
        let mut registered = Vec::new();
        for x in x0.max(spill)..=x1.min(zones + spill - 1) {
            for z in z0.max(spill)..=z1.min(zones + spill - 1) {
                let reached = reach[at(x, z)];
                if reached & 4 == 0 {
                    continue;
                }
                let (zone_x, zone_z) = (x - spill, z - spill);
                for level in 0..4 {
                    let template = region.template(level, zone_x, zone_z);
                    if template == NONE {
                        continue;
                    }
                    let Some(square) = squares.get(&template_square(template)) else {
                        continue;
                    };
                    let added = changed.iter().any(|(x, z, before)| {
                        (*x, *z) == (zone_x, zone_z) && before[level as usize] != template
                    });
                    let from_x = (template_zone_x(template) & 7) as u8;
                    let from_z = (template_zone_z(template) & 7) as u8;
                    let rotation = template_rotation(template);
                    for entry in square.zone_locs(template_level(template), from_x, from_z) {
                        let source = MapsquareCoordGrid::from(entry.coord);
                        let Some(loc_type) = cache.locs.get_by_id(entry.id) else {
                            continue;
                        };
                        let active = loc_type.active == Some(true);
                        let angle = entry.info & 0x3;
                        let (dx, dz) = rotate_loc(
                            source.x() - (from_x << 3),
                            source.z() - (from_z << 3),
                            loc_type.width,
                            loc_type.length,
                            angle,
                            rotation,
                        );
                        let lx = ((zone_x as i32) << 3) + dx;
                        let lz = ((zone_z as i32) << 3) + dz;
                        if lx < 0 || lz < 0 || lx >= tiles || lz >= tiles {
                            continue;
                        }
                        let (width, length) = if (angle + rotation) & 1 == 1 {
                            (loc_type.length as i32, loc_type.width as i32)
                        } else {
                            (loc_type.width as i32, loc_type.length as i32)
                        };
                        let anchor = ((lx >> 3) as u16, (lz >> 3) as u16);
                        let wanted = if active {
                            added || reach[at(anchor.0 + spill, anchor.1 + spill)] & 2 != 0
                        } else {
                            // A wall blocks the tile beyond it too.
                            reached & 2 != 0 || cleared(lx - 1, lz - 1, lx + width, lz + length)
                        };
                        if !wanted {
                            continue;
                        }
                        let (lx, lz) = (lx as u16, lz as u16);
                        let link = zone_lands(&squares, region.template(1, lx >> 3, lz >> 3))
                            .map_or(0, |lands| lands.at((lx & 7) as u8, (lz & 7) as u8));
                        let bridge = if link & LINK_BELOW != 0 {
                            level as i8 - 1
                        } else {
                            level as i8
                        };
                        if !has(bridge, (lx >> 3) + spill, (lz >> 3) + spill) {
                            continue;
                        }
                        let coord = CoordGrid::new(ox + lx, bridge as u8, oz + lz);
                        let shape = LocShape::try_from_primitive(entry.info >> 2).unwrap();
                        let angle = LocAngle::try_from_primitive((angle + rotation) & 0x3).unwrap();
                        if !active {
                            if loc_type.blockwalk {
                                change_loc_collision(
                                    shape,
                                    shape.layer(),
                                    angle,
                                    loc_type.blockrange,
                                    loc_type.length,
                                    loc_type.width,
                                    loc_type.active,
                                    coord,
                                    true,
                                );
                            }
                            continue;
                        }
                        let zone = self.zones.zone_mut(coord.x(), coord.y(), coord.z());
                        zone.add_static_loc(Loc::new(
                            coord,
                            EntityLifeTime::Respawn,
                            entry.id,
                            shape,
                            angle,
                            loc_type.blockwalk,
                            loc_type.blockrange,
                            loc_type.width,
                            loc_type.length,
                        ));
                        let serial = zone.locs.last().unwrap().serial();
                        registered.push((region.column(anchor.0, anchor.1), coord, serial));
                    }
                }
            }
        }

        let local = |coord: CoordGrid| (coord.x() as i32 - ox as i32, coord.z() as i32 - oz as i32);
        let stands_cleared = |coord: CoordGrid, size: u8| {
            let (x, z) = local(coord);
            cleared(x, z, x + size as i32 - 1, z + size as i32 - 1)
        };
        for x in x0..=x1 {
            for z in z0..=z1 {
                let reached = reach[at(x, z)];
                if reached & 4 == 0 {
                    continue;
                }
                let (tx, tz) = tile(x, z);
                for level in 0..4 {
                    let Some(zone) = self.zones.zones.get(&ZoneCoordGrid::new(tx, level, tz))
                    else {
                        continue;
                    };
                    for loc in zone.locs.iter().filter(|loc| loc.visible()) {
                        let coord = loc.world_coord(zone.coord);
                        if reached & 2 == 0 {
                            let (width, length) = if loc.angle() as u8 & 1 == 1 {
                                (loc.length() as i32, loc.width() as i32)
                            } else {
                                (loc.width() as i32, loc.length() as i32)
                            };
                            let (x, z) = local(coord);
                            if !cleared(x - 1, z - 1, x + width, z + length) {
                                continue;
                            }
                        }
                        apply_loc_collision(loc, coord, true);
                    }
                    for &nid in &zone.npcs {
                        if let Some(active) = self.npc_list.get(nid) {
                            let pathing = &active.npc.pathing;
                            if reached & 2 != 0 || stands_cleared(pathing.presence, pathing.size) {
                                occupy(pathing.presence, pathing.size, active.block_walk());
                            }
                        }
                    }
                    for &pid in &zone.players {
                        if let Some(active) = self.player_list.get(pid) {
                            let pathing = &active.player.pathing;
                            if reached & 2 != 0 || stands_cleared(pathing.presence, pathing.size) {
                                occupy(pathing.presence, pathing.size, active.player.block_walk);
                            }
                        }
                    }
                }
            }
        }

        for x in x0..=x1 {
            for z in z0..=z1 {
                if reach[at(x, z)] & 8 == 0 {
                    continue;
                }
                let (tx, tz) = tile(x, z);
                for level in 0..4 {
                    if !has(level as i8, x, z) {
                        rsmod::deallocate_if_present(tx, tz, level);
                    }
                }
            }
        }

        for (column, coord, serial) in registered {
            region.statics[column].push((coord, serial));
        }
        self.regions.squares = squares;
        self.regions.slots[slot].region = Some(region);
        self.rebuild_viewers(slot);
    }

    /// Sends every player in `slot`'s region its map again.
    fn rebuild_viewers(&mut self, slot: usize) {
        for active in self.player_list.players.iter_mut().flatten() {
            let coord = active.player.pathing.coord;
            if slot_at(coord.x(), coord.z()) == Some(slot) {
                active.player.build_area.rebuild = true;
            }
        }
    }

    /// Frees each region nobody has been in for [`FREE_AFTER`] ticks, a new
    /// one counting from its creation.
    pub(crate) fn free_empty_regions(&mut self) {
        let clock = self.clock;
        for active in self.player_list.players.iter().flatten() {
            let coord = active.player.pathing.coord;
            if let Some(region) = slot_at(coord.x(), coord.z())
                .and_then(|slot| self.regions.slots[slot].region.as_mut())
            {
                region.seen = clock;
            }
        }
        let empty: Vec<u32> = self
            .regions
            .slots
            .iter()
            .filter_map(|slot| slot.region.as_ref())
            .filter(|region| clock.wrapping_sub(region.seen) >= FREE_AFTER)
            .map(|region| region.uid)
            .collect();
        for uid in empty {
            self.free_region(uid);
        }
    }

    /// Frees region `uid`: any players in it leave by its way out, or home,
    /// its npcs and controllers are deleted, its zones lose their objs, locs,
    /// pending events and free-to-play marks, and its collision goes now, so
    /// nothing later this tick can add, place or teleport anything into it.
    /// Its zones go at cleanup. Its slot waits out [`QUARANTINE`] before the
    /// next region.
    ///
    /// Zone presence moves with every step and teleport, so the zones' lists
    /// hold everything standing in the region.
    pub(crate) fn free_region(&mut self, uid: u32) {
        let Ok(slot) = self.regions.live(uid) else {
            return;
        };
        let (ox, oz, zones) = bounds(slot);
        let tiles = zones << 3;
        let inside = move |coord: CoordGrid| {
            coord.x() >= ox && coord.z() >= oz && coord.x() - ox < tiles && coord.z() - oz < tiles
        };

        let mut pids = Vec::new();
        let mut nids = Vec::new();
        for level in 0..4 {
            for zone_x in 0..zones {
                for zone_z in 0..zones {
                    let coord = ZoneCoordGrid::new(ox + (zone_x << 3), level, oz + (zone_z << 3));
                    if let Some(zone) = self.zones.zones.get(&coord) {
                        pids.extend_from_slice(&zone.players);
                        nids.extend_from_slice(&zone.npcs);
                    }
                }
            }
        }
        for pid in pids {
            self.leave_region(pid);
        }
        for nid in nids {
            self.deactivate_npc(nid);
        }
        self.controllers.remove_where(inside);
        for events in self.pending_zone_events.values_mut() {
            events.retain(|event| {
                !inside(match event {
                    PendingZoneEvent::ObjReveal { coord, .. }
                    | PendingZoneEvent::ObjDelete { coord, .. }
                    | PendingZoneEvent::ObjAdd { coord, .. }
                    | PendingZoneEvent::LocDelete { coord, .. } => *coord,
                })
            });
        }

        for level in 0..4 {
            for zone_x in 0..zones {
                for zone_z in 0..zones {
                    let coord = ZoneCoordGrid::new(ox + (zone_x << 3), level, oz + (zone_z << 3));
                    if let Some(zone) = self.zones.zones.get_mut(&coord) {
                        zone.locs.clear();
                        zone.objs.clear();
                        zone.reset();
                    }
                    rsmod::change_zone(coord.x(), coord.z(), level, ZoneFlag::Free as u8, false);
                    rsmod::deallocate_if_present(coord.x(), coord.z(), level);
                    self.dropped_zones.push(coord);
                }
            }
        }

        let slot = &mut self.regions.slots[slot];
        slot.region = None;
        slot.freed = Some(self.clock);
    }

    /// Moves player `pid` out of region space by its way out, or home where
    /// there is none or it can't be stood on, its zone and collision moving
    /// with it now rather than on its turn.
    fn leave_region(&mut self, pid: u16) {
        let Some(active) = self.player_list.get(pid) else {
            return;
        };
        let from = active.player.pathing.coord;
        let out = self.way_out(from, active.player.region_entry);
        let Some(active) = self.player_list.get_mut(pid) else {
            return;
        };
        active.player.pathing.clear_waypoints();
        if out
            .and_then(|out| active.player.pathing.teleport(out))
            .or_else(|| active.player.pathing.teleport(home(from)))
            .is_none()
        {
            return;
        }
        active.player.pathing.jump = true;
        let block_walk = active.player.block_walk;
        Engine::refresh_presence(
            &mut self.zones,
            &mut active.player.pathing,
            EntityId::Player(pid),
            block_walk,
        );
    }

    /// Sends a player standing in region space on no region's floor out by
    /// its way out, or home. Nothing should leave one there, but nothing may
    /// keep one there.
    pub(crate) fn fall_forward(&mut self, pid: u16) {
        let Some(active) = self.player_list.get(pid) else {
            return;
        };
        let coord = active.player.pathing.coord;
        if coord.x() < REGION_X || rsmod::is_zone_allocated(coord.x(), coord.z(), coord.y()) {
            return;
        }
        self.leave_region(pid);
    }

    /// Drops the zones of the regions freed this tick, after the npcs
    /// despawned from them, freeing their collision again should anything
    /// have stood on it since.
    pub(crate) fn drop_region_zones(&mut self) {
        for coord in self.dropped_zones.drain(..) {
            rsmod::deallocate_if_present(coord.x(), coord.z(), coord.y());
            self.zones.zones.remove(&coord);
        }
    }
}

/// `rotation` as quarter turns, which `command` takes 0 to 3 of.
fn turns(command: &str, rotation: i32) -> Result<u8, ScriptError> {
    u8::try_from(rotation)
        .ok()
        .filter(|&rotation| rotation < 4)
        .ok_or_else(|| ScriptError::Runtime(format!("{command}: no rotation {rotation}")))
}

/// The tile flags of the static zone a template copies, as the client lays
/// them on the copy.
struct ZoneLands<'a> {
    lands: &'a [u8; MAPSQUARE],
    x: u8,
    z: u8,
    level: u8,
    rotation: u8,
}

impl ZoneLands<'_> {
    /// The flags of tile (`x`, `z`) of the copy.
    fn at(&self, x: u8, z: u8) -> u8 {
        let (x, z) = unrotate_tile(x, z, self.rotation);
        self.lands[MapsquareCoordGrid::new(self.x + x, self.level, self.z + z).packed() as usize]
    }
}

/// The mapsquare `template` copies from, decoded into `squares` the first
/// time.
fn square<'a>(
    cache: &CacheStore,
    squares: &'a mut FxHashMap<(u16, u16), SquareTemplate>,
    template: u32,
) -> &'a SquareTemplate {
    let (mx, mz) = template_square(template);
    squares
        .entry((mx, mz))
        .or_insert_with(|| GameMap::decode_template(cache, mx as u8, mz as u8))
}

/// The tile flags `template` copies, from its mapsquare in `squares`.
fn zone_lands(
    squares: &FxHashMap<(u16, u16), SquareTemplate>,
    template: u32,
) -> Option<ZoneLands<'_>> {
    if template == NONE {
        return None;
    }
    Some(ZoneLands {
        lands: &squares.get(&template_square(template))?.lands,
        x: ((template_zone_x(template) & 7) << 3) as u8,
        z: ((template_zone_z(template) & 7) << 3) as u8,
        level: template_level(template),
        rotation: template_rotation(template),
    })
}

/// Whether the zone `template` copies is ground: the map loader leaves a zone
/// of only void unallocated, so it blocks, and so does its copy.
fn has_ground(template: u32) -> bool {
    template != NONE
        && rsmod::is_zone_allocated(
            template_zone_x(template) << 3,
            template_zone_z(template) << 3,
            template_level(template),
        )
}

/// Where a player at `coord` goes when nowhere else will do.
fn home(coord: CoordGrid) -> CoordGrid {
    let (x, z, y) = home_for(coord.x(), coord.z());
    CoordGrid::new(x, y, z)
}

/// Where a player saved at `coord` with `exit` logs in: a tile of region
/// space never, as no region outlives the world or keeps a player it lost,
/// but the saved exit if a player can stand there, else home.
pub(crate) fn login_coord(coord: CoordGrid, exit: Option<CoordGrid>) -> CoordGrid {
    if coord.x() < REGION_X {
        return coord;
    }
    exit.filter(|exit| {
        exit.x() < REGION_X && rsmod::is_zone_allocated(exit.x(), exit.z(), exit.y())
    })
    .unwrap_or_else(|| home(coord))
}

/// Puts a returning player saved in region space at its saved exit, or home,
/// before anything is sent to it.
pub(crate) fn place_returning(profile: &mut PlayerProfile) {
    let coord = login_coord(
        CoordGrid::new(profile.x, profile.y, profile.z),
        profile.exit,
    );
    (profile.x, profile.y, profile.z) = (coord.x(), coord.y(), coord.z());
    profile.exit = None;
}

/// Blocks the tiles an entity of `size` is present on at `coord`, as moving
/// there does.
fn occupy(coord: CoordGrid, size: u8, block_walk: BlockWalk) {
    match block_walk {
        BlockWalk::Npc => rsmod::change_npc(coord.x(), coord.z(), coord.y(), size, true),
        BlockWalk::All => {
            rsmod::change_npc(coord.x(), coord.z(), coord.y(), size, true);
            rsmod::change_player(coord.x(), coord.z(), coord.y(), size, true);
        }
        BlockWalk::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tile (`x`, `z`) of a zone after turning the zone `rotation` quarter
    /// turns clockwise, as the client places it.
    const fn rotate_tile(x: u8, z: u8, rotation: u8) -> (u8, u8) {
        match rotation & 3 {
            0 => (x, z),
            1 => (z, 7 - x),
            2 => (7 - x, 7 - z),
            _ => (7 - z, x),
        }
    }

    #[test]
    fn slots_tile_region_space() {
        assert_eq!(bounds(0), (6464, 64, 16));
        assert_eq!(bounds(1), (6656, 64, 16));
        assert_eq!(bounds(51), (6464, 256, 16));
        assert_eq!(
            bounds(SMALL_SLOTS - 1),
            (6464 + 50 * 192, 64 + 26 * 192, 16)
        );
        assert_eq!(bounds(SMALL_SLOTS), (6464, 5312, 40));
        assert_eq!(bounds(SLOTS - 1), (6464 + 24 * 384, 5312 + 27 * 384, 40));
        for slot in 0..SLOTS {
            let (x, z, zones) = bounds(slot);
            let tiles = zones << 3;
            assert!(x + tiles + PADDING <= 16384 && z + tiles + PADDING <= 16384);
            assert_eq!(slot_at(x, z), Some(slot));
            assert_eq!(slot_at(x + tiles - 1, z + tiles - 1), Some(slot));
            assert_eq!(slot_at(x + tiles, z), None);
            assert_eq!(slot_at(x, z - 1), None);
        }
        assert_eq!(slot_at(REGION_X - 1, 64), None);
        assert_eq!(slot_at(6464, 5250), None);
    }

    #[test]
    fn rotations_match_the_client() {
        for rotation in 0..4 {
            for x in 0..8 {
                for z in 0..8 {
                    let client_x = match rotation {
                        0 => x,
                        1 => z,
                        2 => 7 - x,
                        _ => 7 - z,
                    };
                    let client_z = match rotation {
                        0 => z,
                        1 => 7 - x,
                        2 => 7 - z,
                        _ => x,
                    };
                    assert_eq!(rotate_tile(x, z, rotation), (client_x, client_z));
                    assert_eq!(unrotate_tile(client_x, client_z, rotation), (x, z));
                    assert_eq!(
                        rotate_loc(x, z, 1, 1, 0, rotation),
                        (client_x as i32, client_z as i32)
                    );
                }
            }
        }
        assert_eq!(rotate_loc(0, 0, 2, 3, 0, 1), (0, 6));
        assert_eq!(rotate_loc(0, 0, 2, 3, 1, 1), (0, 5));
        assert_eq!(rotate_loc(6, 5, 2, 3, 0, 2), (0, 0));
        assert_eq!(rotate_loc(0, 0, 2, 3, 0, 3), (5, 0));
    }

    #[test]
    fn source_inverts_the_rotation() {
        let mut regions = Regions::default();
        let uid = regions.allocate(4, 16, 16, 0).unwrap();
        let slot = (uid & 0xFFF) as usize;
        let (ox, oz, _) = bounds(slot);
        for rotation in 0..4 {
            let region = regions.slots[slot].region.as_mut().unwrap();
            *region.template_mut(0, 3, 4) = template(1, 400, 401, rotation);
            for x in 0..8 {
                for z in 0..8 {
                    let (dx, dz) = rotate_tile(x, z, rotation);
                    let coord = CoordGrid::new(ox + 24 + dx as u16, 0, oz + 32 + dz as u16);
                    assert_eq!(
                        regions.source(coord),
                        Some(CoordGrid::new(3200 + x as u16, 1, 3208 + z as u16))
                    );
                }
            }
        }
    }

    #[test]
    fn a_region_too_wide_for_a_small_slot_takes_a_large_one() {
        let mut regions = Regions::default();
        let small = regions.allocate(1, 16, 16, 0).unwrap();
        assert!(((small & 0xFFF) as usize) < SMALL_SLOTS);
        let wide = regions.allocate(1, 17, 1, 0).unwrap();
        assert!(((wide & 0xFFF) as usize) >= SMALL_SLOTS);
        let long = regions.allocate(4, 1, 40, 0).unwrap();
        assert!(((long & 0xFFF) as usize) >= SMALL_SLOTS);
        let region = regions.get(long).unwrap();
        assert_eq!(
            (region.levels, region.east, region.north, region.zones),
            (4, 1, 40, 40)
        );
    }

    #[test]
    fn freed_slots_wait_their_turn() {
        let mut regions = Regions::default();
        let first = regions.allocate(4, 16, 16, 0).unwrap();
        let slot = (first & 0xFFF) as usize;
        regions.slots[slot].region = None;
        regions.slots[slot].freed = Some(10);
        regions.cursors[0] = slot;
        let second = regions.allocate(4, 16, 16, 20).unwrap();
        assert_ne!(second & 0xFFF, first & 0xFFF);
        regions.cursors[0] = slot;
        let third = regions.allocate(4, 16, 16, 10 + QUARANTINE).unwrap();
        assert_eq!(third & 0xFFF, first & 0xFFF);
        assert_ne!(third, first);
        assert!(regions.get(first).is_none());
        assert!(regions.get(third).is_some());
    }
}
