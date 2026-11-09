use crate::console::home_for;
use crate::engine::{Engine, PendingZoneEvent};
use crate::game_map::{
    BLOCK_MAP_SQUARE, GameMap, LINK_BELOW, REMOVE_ROOFS, SquareTemplate, apply_loc_collision,
    change_loc_collision,
};
use crate::phases::shared::EntityId;
use num_enum::TryFromPrimitive;
use rs_entity::{EntityLifeTime, Loc, LocSerial};
use rs_grid::mapsquare_coord::MapsquareCoordGrid;
use rs_grid::{CoordGrid, ZoneCoordGrid};
use rs_pack::cache::ScriptVarType;
use rs_pack::cache::controller::ControllerType;
use rs_pack::cache::varcon::VarConTypeProvider;
use rs_pack::types::{BlockWalk, LocAngle, LocShape, ParamValue, VarPlayerScope};
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

/// Tile (`x`, `z`) of a zone after turning the zone `rotation` quarter turns
/// clockwise, as the client places it (`AClass13.method623`,
/// `Class244.method1589`).
const fn rotate_tile(x: u8, z: u8, rotation: u8) -> (u8, u8) {
    match rotation & 3 {
        0 => (x, z),
        1 => (z, 7 - x),
        2 => (7 - x, 7 - z),
        _ => (7 - z, x),
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

/// A region: the zones copied onto its slot of region space and the active
/// locs each zone column put on the map.
pub struct Region {
    pub uid: u32,
    zones: u16,
    templates: Box<[u32]>,
    statics: Box<[Vec<(CoordGrid, LocSerial)>]>,
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
    spill: u16,
    squares: FxHashMap<(u16, u16), SquareTemplate>,
}

impl Regions {
    /// `largest_loc` is the longest loc side, so a rebuild reaches every
    /// zone a copied loc can put collision on.
    pub fn new(largest_loc: u8) -> Regions {
        Regions {
            slots: (0..SLOTS).map(|_| Slot::default()).collect(),
            cursors: [0, 0],
            spill: ((largest_loc.max(1) as u16 + 7) >> 3).max(1),
            squares: FxHashMap::default(),
        }
    }

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
        let (x, z) = ((lx & 7) as u8, (lz & 7) as u8);
        let (x, z) = match template_rotation(template) {
            0 => (x, z),
            1 => (7 - z, x),
            2 => (7 - x, 7 - z),
            _ => (z, 7 - x),
        };
        Some(CoordGrid::new(
            (template_zone_x(template) << 3) + x as u16,
            template_level(template),
            (template_zone_z(template) << 3) + z as u16,
        ))
    }

    fn allocate(&mut self, large: bool, clock: u32) -> Option<u32> {
        let (layout, cursor) = if large {
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
            templates: vec![NONE; (zones as usize * zones as usize) << 2].into_boxed_slice(),
            statics: (0..zones as usize * zones as usize)
                .map(|_| Vec::new())
                .collect(),
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
    /// What a player at `coord` saves: a tile of region space never, as no
    /// region outlives the world, but the type of the controller managing
    /// its region, at x [`REGION_X`] where no region lies, for
    /// [`login_coord`](Self::login_coord) to place it by the type's config
    /// on whichever day it comes back.
    pub(crate) fn saved_coord(&self, coord: CoordGrid) -> CoordGrid {
        if coord.x() < REGION_X {
            return coord;
        }
        slot_at(coord.x(), coord.z())
            .filter(|&slot| self.regions.slots[slot].region.is_some())
            .and_then(|slot| self.region_type(slot))
            .map_or(coord, |id| CoordGrid::new(REGION_X, 0, id))
    }

    /// Where a player saved at `coord` logs in: a region's player at the
    /// exit its controller type's config gives today, anything else in
    /// region space home.
    pub(crate) fn login_coord(&self, coord: CoordGrid) -> CoordGrid {
        if coord.x() < REGION_X {
            return coord;
        }
        (coord.x() == REGION_X)
            .then(|| self.controller_exit(coord.z()))
            .flatten()
            .unwrap_or_else(|| home(coord))
    }

    /// The type of the controller managing the region in `slot`: the one on
    /// its south-west tile, where content keeps it.
    fn region_type(&self, slot: usize) -> Option<u16> {
        let (x, z, _) = bounds(slot);
        self.controllers
            .first_at(CoordGrid::new(x, 0, z))
            .map(|controller| controller.id)
    }

    /// Where the players of a region managed by controller type `id` leave
    /// to: the coord its config gives its perm coord varcon, if a player can
    /// stand there.
    pub(crate) fn controller_exit(&self, id: u16) -> Option<CoordGrid> {
        perm_coord(self.cache.controllers.get_by_id(id)?, &self.cache.varcons).filter(|coord| {
            coord.x() < REGION_X && rsmod::is_zone_allocated(coord.x(), coord.z(), coord.y())
        })
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
    /// that selected it: a coord kept from a deleted region never reaches the
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

    pub(crate) fn region_create(&mut self, size: i32) -> Result<Option<u32>, ScriptError> {
        let large = match size {
            0 => false,
            1 => true,
            _ => {
                return Err(ScriptError::Runtime(format!(
                    "region_create: no region size {size}"
                )));
            }
        };
        Ok(self.regions.allocate(large, self.clock))
    }

    pub(crate) fn region_coord(
        &self,
        uid: u32,
        x: i32,
        y: i32,
        z: i32,
    ) -> Result<CoordGrid, ScriptError> {
        let (ox, oz, zones) = bounds(self.regions.live(uid)?);
        let tiles = (zones << 3) as i32;
        if !(0..tiles).contains(&x) || !(0..4).contains(&y) || !(0..tiles).contains(&z) {
            return Err(ScriptError::Runtime(format!(
                "region_getcoord: {x}, {y}, {z} is outside the region"
            )));
        }
        Ok(CoordGrid::new(ox + x as u16, y as u8, oz + z as u16))
    }

    /// Copies the zones from `from_sw` to `from_ne` onto region `uid` from
    /// `to_sw`, the area turned `rotation` quarter turns clockwise, each with
    /// its source zone's free-to-play mark, and rebuilds what the copy
    /// reaches.
    pub(crate) fn region_copy(
        &mut self,
        uid: u32,
        from_sw: CoordGrid,
        from_ne: CoordGrid,
        to_sw: CoordGrid,
        rotation: i32,
    ) -> Result<(), ScriptError> {
        let slot = self.regions.live(uid)?;
        let (ox, oz, zones) = bounds(slot);
        let rotation = u8::try_from(rotation)
            .ok()
            .filter(|&rotation| rotation < 4)
            .ok_or_else(|| ScriptError::Runtime(format!("region_copy: no rotation {rotation}")))?;
        if from_ne.x() < from_sw.x()
            || from_ne.z() < from_sw.z()
            || from_ne.y() < from_sw.y()
            || from_ne.x() >= REGION_X
        {
            return Err(ScriptError::Runtime(format!(
                "region_copy: {from_sw:?} to {from_ne:?} is not an area of the map"
            )));
        }
        let (sx, sz) = (from_sw.x() >> 3, from_sw.z() >> 3);
        let width = (from_ne.x() >> 3) - sx + 1;
        let length = (from_ne.z() >> 3) - sz + 1;
        let levels = from_ne.y() - from_sw.y() + 1;
        let (turned_width, turned_length) = if rotation & 1 == 1 {
            (length, width)
        } else {
            (width, length)
        };
        if to_sw.x() < ox
            || to_sw.z() < oz
            || ((to_sw.x() - ox) >> 3) + turned_width > zones
            || ((to_sw.z() - oz) >> 3) + turned_length > zones
            || to_sw.y() + levels > 4
        {
            return Err(ScriptError::Runtime(format!(
                "region_copy: the area has no room in the region from {to_sw:?}"
            )));
        }
        let (dx, dz) = ((to_sw.x() - ox) >> 3, (to_sw.z() - oz) >> 3);

        let region = self.regions.slots[slot].region.as_mut().unwrap();
        let mut changed = Vec::with_capacity((width * length) as usize);
        for a in 0..width {
            for b in 0..length {
                let (i, j) = match rotation {
                    0 => (a, b),
                    1 => (b, width - 1 - a),
                    2 => (width - 1 - a, length - 1 - b),
                    _ => (length - 1 - b, a),
                };
                for level in 0..levels {
                    *region.template_mut(to_sw.y() + level, dx + i, dz + j) =
                        template(from_sw.y() + level, sx + a, sz + b, rotation);
                    let free = rsmod::is_zone_flagged(
                        (sx + a) << 3,
                        (sz + b) << 3,
                        from_sw.y() + level,
                        ZoneFlag::Free as u8,
                    );
                    rsmod::change_zone(
                        ox + ((dx + i) << 3),
                        oz + ((dz + j) << 3),
                        to_sw.y() + level,
                        ZoneFlag::Free as u8,
                        free,
                    );
                }
                changed.push((dx + i, dz + j));
            }
        }
        self.rebuild_region(slot, &changed);
        Ok(())
    }

    /// Builds again, as the client builds them from the same templates, the
    /// zones of `slot` that the `changed` columns reach: their floor and roof
    /// flags and the collision of every loc and entity on them, with the
    /// active locs of the changed columns put back into their zones. Locs and
    /// entities keep their state everywhere.
    fn rebuild_region(&mut self, slot: usize, changed: &[(u16, u16)]) {
        let (ox, oz, zones) = bounds(slot);
        let spill = self.regions.spill;
        let span = zones + (spill << 1);
        let at = |x: u16, z: u16| x as usize * span as usize + z as usize;
        let mut reach = vec![0u8; span as usize * span as usize];
        for &(x, z) in changed {
            reach[at(x + spill, z + spill)] = 1;
        }
        for (from, to) in [(1, 2), (2, 4)] {
            for x in 0..span {
                for z in 0..span {
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

        let mut region = self.regions.slots[slot].region.take().unwrap();
        let mut squares = mem::take(&mut self.regions.squares);
        if squares.len() > SQUARES {
            squares.clear();
        }
        for &template in region.templates.iter() {
            if template != NONE {
                let (mx, mz) = template_square(template);
                squares
                    .entry((mx, mz))
                    .or_insert_with(|| GameMap::decode_template(self.cache, mx as u8, mz as u8));
            }
        }

        let tiles = (zones << 3) as usize;
        let land = |level: usize, x: usize, z: usize| (level * tiles + x) * tiles + z;
        let mut lands = vec![0; (tiles * tiles) << 2];
        for level in 0..4 {
            for zone_x in 0..zones {
                for zone_z in 0..zones {
                    let template = region.template(level, zone_x, zone_z);
                    if template == NONE {
                        continue;
                    }
                    let square = &squares[&template_square(template)];
                    let base_x = ((template_zone_x(template) & 7) << 3) as u8;
                    let base_z = ((template_zone_z(template) & 7) << 3) as u8;
                    for x in 0..8 {
                        for z in 0..8 {
                            let source = MapsquareCoordGrid::new(
                                base_x + x,
                                template_level(template),
                                base_z + z,
                            );
                            let (dx, dz) = rotate_tile(x, z, template_rotation(template));
                            lands[land(
                                level as usize,
                                ((zone_x << 3) + dx as u16) as usize,
                                ((zone_z << 3) + dz as u16) as usize,
                            )] = square.lands[source.packed() as usize];
                        }
                    }
                }
            }
        }

        for &(zone_x, zone_z) in changed {
            let column = region.column(zone_x, zone_z);
            for (coord, serial) in region.statics[column].drain(..) {
                let (x, y, z) = (coord.x(), coord.y(), coord.z());
                if let Some(zone) = self.zones.zones.get_mut(&ZoneCoordGrid::new(x, y, z))
                    && let Some(index) = zone.locs.iter().position(|loc| loc.serial() == serial)
                {
                    zone.locs.remove(index);
                }
            }
        }

        let has = |level: i8, x: u16, z: u16| {
            level >= 0
                && x >= spill
                && z >= spill
                && x - spill < zones
                && z - spill < zones
                && has_ground(region.template(level as u8, x - spill, z - spill))
        };

        for x in 0..span {
            for z in 0..span {
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

        for x in 0..span {
            for z in 0..span {
                if reach[at(x, z)] & 2 == 0 {
                    continue;
                }
                let (tx, tz) = tile(x, z);
                for level in 0..4 {
                    if !has(level as i8, x, z) {
                        continue;
                    }
                    for dx in 0..8 {
                        for dz in 0..8 {
                            let lx = (((x - spill) << 3) + dx) as usize;
                            let lz = (((z - spill) << 3) + dz) as usize;
                            let flags = lands[land(level as usize, lx, lz)];
                            if flags & REMOVE_ROOFS != 0 {
                                rsmod::change_roof(tx + dx, tz + dz, level, true);
                            }
                            if flags & BLOCK_MAP_SQUARE == 0 {
                                continue;
                            }
                            let bridge = if lands[land(1, lx, lz)] & LINK_BELOW != 0 {
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

        let cache = self.cache;
        let mut registered = Vec::new();
        for x in spill..zones + spill {
            for z in spill..zones + spill {
                let reached = reach[at(x, z)];
                if reached & 4 == 0 {
                    continue;
                }
                let (zone_x, zone_z) = (x - spill, z - spill);
                let column = region.column(zone_x, zone_z);
                for level in 0..4 {
                    let template = region.template(level, zone_x, zone_z);
                    if template == NONE {
                        continue;
                    }
                    let square = &squares[&template_square(template)];
                    let base_x = ((template_zone_x(template) & 7) << 3) as u8;
                    let base_z = ((template_zone_z(template) & 7) << 3) as u8;
                    let rotation = template_rotation(template);
                    for entry in &square.locs {
                        let source = MapsquareCoordGrid::from(entry.coord);
                        if source.y() != template_level(template)
                            || source.x() < base_x
                            || source.x() >= base_x + 8
                            || source.z() < base_z
                            || source.z() >= base_z + 8
                        {
                            continue;
                        }
                        let Some(loc_type) = cache.locs.get_by_id(entry.id) else {
                            continue;
                        };
                        let active = loc_type.active == Some(true);
                        if active && reached & 1 == 0 {
                            continue;
                        }
                        let angle = entry.info & 0x3;
                        let (dx, dz) = rotate_loc(
                            source.x() - base_x,
                            source.z() - base_z,
                            loc_type.width,
                            loc_type.length,
                            angle,
                            rotation,
                        );
                        let lx = ((zone_x as i32) << 3) + dx;
                        let lz = ((zone_z as i32) << 3) + dz;
                        if lx < 0 || lz < 0 || lx >= tiles as i32 || lz >= tiles as i32 {
                            continue;
                        }
                        let (lx, lz) = (lx as u16, lz as u16);
                        let bridge = if lands[land(1, lx as usize, lz as usize)] & LINK_BELOW != 0 {
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
                        registered.push((column, coord, serial));
                    }
                }
            }
        }

        for x in 0..span {
            for z in 0..span {
                if reach[at(x, z)] & 4 == 0 {
                    continue;
                }
                let (tx, tz) = tile(x, z);
                for level in 0..4 {
                    let Some(zone) = self.zones.zones.get(&ZoneCoordGrid::new(tx, level, tz))
                    else {
                        continue;
                    };
                    for loc in zone.locs.iter().filter(|loc| loc.visible()) {
                        apply_loc_collision(loc, loc.world_coord(zone.coord), true);
                    }
                    for &nid in &zone.npcs {
                        if let Some(active) = self.npc_list.get(nid) {
                            let pathing = &active.npc.pathing;
                            occupy(pathing.presence, pathing.size, active.block_walk());
                        }
                    }
                    for &pid in &zone.players {
                        if let Some(active) = self.player_list.get(pid) {
                            let pathing = &active.player.pathing;
                            occupy(pathing.presence, pathing.size, active.player.block_walk);
                        }
                    }
                }
            }
        }

        for x in 0..span {
            for z in 0..span {
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
        let (ox, oz, zones) = bounds(slot);
        for level in 0..4 {
            for zone_x in 0..zones {
                for zone_z in 0..zones {
                    let coord = ZoneCoordGrid::new(ox + (zone_x << 3), level, oz + (zone_z << 3));
                    let Some(zone) = self.zones.zones.get(&coord) else {
                        continue;
                    };
                    for &pid in &zone.players {
                        if let Some(active) = self.player_list.get_mut(pid) {
                            active.player.build_area.rebuild = true;
                        }
                    }
                }
            }
        }
    }

    /// Deletes region `uid`: its players go to its controller type's exit,
    /// or home, its npcs and controllers are deleted, its zones lose their
    /// objs, locs, pending events and free-to-play marks, and its collision
    /// goes now, so nothing later this tick can add, place or teleport
    /// anything into it. Its zones go at cleanup. Its slot waits out
    /// [`QUARANTINE`] before the next region.
    ///
    /// Zone presence moves with every step and teleport, so the zones' lists
    /// hold everything standing in the region.
    pub(crate) fn region_del(&mut self, uid: u32) {
        let Ok(slot) = self.regions.live(uid) else {
            return;
        };
        let exit = self
            .region_type(slot)
            .and_then(|id| self.controller_exit(id));
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
            self.leave_region(pid, exit);
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

    /// Moves player `pid` out of region space to `exit`, or home where there
    /// is none or it can't be stood on, its zone and collision moving with it
    /// now rather than on its turn.
    fn leave_region(&mut self, pid: u16, exit: Option<CoordGrid>) {
        let Some(active) = self.player_list.get_mut(pid) else {
            return;
        };
        let from = active.player.pathing.coord;
        active.player.pathing.clear_waypoints();
        if exit
            .and_then(|exit| active.player.pathing.teleport(exit))
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

    /// Sends a player standing in region space on no region's floor out: to
    /// the exit of the region it stands in, or home. Nothing should leave
    /// one there, but nothing may keep one there.
    pub(crate) fn fall_forward(&mut self, pid: u16) {
        let Some(active) = self.player_list.get(pid) else {
            return;
        };
        let coord = active.player.pathing.coord;
        if coord.x() < REGION_X || rsmod::is_zone_allocated(coord.x(), coord.z(), coord.y()) {
            return;
        }
        let exit = slot_at(coord.x(), coord.z())
            .filter(|&slot| self.regions.slots[slot].region.is_some())
            .and_then(|slot| self.region_type(slot))
            .and_then(|id| self.controller_exit(id));
        self.leave_region(pid, exit);
    }

    /// Drops the zones of the regions deleted this tick, after the npcs
    /// despawned from them, freeing their collision again should anything
    /// have stood on it since.
    pub(crate) fn drop_region_zones(&mut self) {
        for coord in self.dropped_zones.drain(..) {
            rsmod::deallocate_if_present(coord.x(), coord.z(), coord.y());
            self.zones.zones.remove(&coord);
        }
    }
}

/// The coord `controller`'s config gives its perm coord varcon.
fn perm_coord(controller: &ControllerType, varcons: &VarConTypeProvider) -> Option<CoordGrid> {
    controller.vars.iter().find_map(|(varcon, value)| {
        let varcon = varcons.get_by_id(*varcon)?;
        match value {
            ParamValue::Int(packed)
                if *packed >= 0
                    && varcon.var_type == ScriptVarType::Coord
                    && varcon.scope == VarPlayerScope::Perm =>
            {
                Some(CoordGrid::from(*packed as u32))
            }
            _ => None,
        }
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
    use rs_pack::cache::provider::{CacheType, TypeProvider};
    use rs_pack::cache::varcon::VarConType;

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
        let mut regions = Regions::new(1);
        let uid = regions.allocate(false, 0).unwrap();
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
    fn exits_come_from_a_perm_coord_varcon() {
        let varcon = |id: u16, var_type, scope| {
            let mut varcon = VarConType::new(id);
            varcon.var_type = var_type;
            varcon.scope = scope;
            varcon
        };
        let varcons = TypeProvider::<VarConType>::from_types::<VarConType>(
            vec![
                varcon(0, ScriptVarType::Coord, VarPlayerScope::Temp),
                varcon(1, ScriptVarType::Int, VarPlayerScope::Perm),
                varcon(2, ScriptVarType::Coord, VarPlayerScope::Perm),
            ],
            (),
        );
        let exit = CoordGrid::new(3222, 0, 3218);
        let mut controller = ControllerType::new(0);
        controller.vars = Box::new([
            (0, ParamValue::Int(CoordGrid::new(1, 0, 1).packed() as i32)),
            (1, ParamValue::Int(5)),
            (2, ParamValue::Int(exit.packed() as i32)),
        ]);
        assert_eq!(perm_coord(&controller, &varcons), Some(exit));
        controller.vars = Box::new([(0, ParamValue::Int(exit.packed() as i32))]);
        assert_eq!(perm_coord(&controller, &varcons), None);
    }

    #[test]
    fn freed_slots_wait_their_turn() {
        let mut regions = Regions::new(1);
        let first = regions.allocate(false, 0).unwrap();
        let slot = (first & 0xFFF) as usize;
        regions.slots[slot].region = None;
        regions.slots[slot].freed = Some(10);
        regions.cursors[0] = slot;
        let second = regions.allocate(false, 20).unwrap();
        assert_ne!(second & 0xFFF, first & 0xFFF);
        regions.cursors[0] = slot;
        let third = regions.allocate(false, 10 + QUARANTINE).unwrap();
        assert_eq!(third & 0xFFF, first & 0xFFF);
        assert_ne!(third, first);
        assert!(regions.get(first).is_none());
        assert!(regions.get(third).is_some());
    }
}
