use crate::direction::Direction;
use crate::player::MoveSpeed;
use rs_grid::CoordGrid;
use rs_info::{EntityMasks, FocusKind};
use rs_pack::types::MoveRestrict;
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use rsmod::rsmod::flag::collision_flag::CollisionFlag;
use rsmod::rsmod::flag::zone_flag::ZoneFlag;

/// The pathfinding strategy used by a pathing entity.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveStrategy {
    /// Uses smart pathfinding (A* or similar) to navigate around obstacles. Used by players.
    Smart = 0,
    /// Uses naive step-by-step movement toward the destination without pre-computed routes. Used by NPCs.
    Naive = 1,
}

/// Movement and pathfinding state for an entity (player or NPC).
///
/// Manages the waypoint queue, current coordinate, movement direction outputs,
/// and collision parameters. Each tick, `process_movement` consumes waypoints and
/// produces walk/run direction values that are sent to the client for animation.
pub struct PathingEntity {
    pub last_movement: u32,
    pub waypoints: [CoordGrid; 25],
    pub coord: CoordGrid,
    pub waypoint_index: i32,
    pub walk_step: Option<Direction>,
    pub move_restrict: MoveRestrict,
    pub move_strategy: MoveStrategy,
    pub move_speed: MoveSpeed,
    pub walk_dir: i8,
    pub run_dir: i8,
    pub steps_taken: u8,
    pub size: u8,
    pub tele: bool,
    pub jump: bool,
    pub last_crawl: bool,
    pub last_coord: CoordGrid,
    pub last_step_coord: CoordGrid,
    pub follow_coord: CoordGrid,
}

impl PathingEntity {
    /// Creates a new `PathingEntity` at the given coordinate with the specified parameters.
    ///
    /// # Arguments
    /// * `coord` - Starting grid coordinate.
    /// * `size` - Collision size in tiles.
    /// * `move_restrict` - Movement restriction type for collision checks.
    /// * `move_strategy` - Pathfinding strategy (smart or naive).
    ///
    /// # Returns
    /// A new entity with no waypoints, walk speed, and all direction outputs reset to -1.
    pub fn new(
        coord: CoordGrid,
        size: u8,
        move_restrict: MoveRestrict,
        move_strategy: MoveStrategy,
    ) -> Self {
        Self {
            coord,
            walk_dir: -1,
            run_dir: -1,
            walk_step: None,
            waypoints: [CoordGrid::from(0); 25],
            waypoint_index: -1,
            steps_taken: 0,
            last_movement: 0,
            jump: true,
            tele: true,
            size,
            move_restrict,
            move_strategy,
            move_speed: MoveSpeed::Walk,
            last_crawl: false,
            // these get reapplied during profile load.
            last_coord: coord,
            last_step_coord: coord,
            follow_coord: coord,
        }
    }

    /// Processes one tick of movement, consuming waypoints and producing direction outputs.
    ///
    /// Handles crawl speed (moves every other tick), walk speed (one step), and run speed
    /// (two steps). Updates `walk_dir`, `run_dir`, and `steps_taken` accordingly.
    ///
    /// # Arguments
    /// * `members` - Whether this world loads members-only areas; forwarded to
    ///   [`can_travel`] so members tiles are blocked on a free-to-play world.
    /// * `info` - The entity's info masks, updated with the new facing direction.
    /// * `kind` - Whether this is a player or NPC focus, affecting how orientation is set.
    ///
    /// # Returns
    /// `true` if at least one step was taken this tick, `false` otherwise.
    ///
    /// # Side Effects
    /// * Mutates `self.walk_dir`, `self.run_dir`, `self.steps_taken`, `self.last_crawl`.
    /// * Advances `self.coord` by calling `try_step`.
    /// * Updates entity orientation in `info`.
    ///
    /// # Call Stack
    /// **Calls:** `has_waypoints`, `try_step`
    pub fn process_movement(
        &mut self,
        members: bool,
        info: &mut EntityMasks,
        kind: FocusKind,
    ) -> bool {
        if self.move_restrict == MoveRestrict::NoMove {
            return false;
        }

        if !self.has_waypoints() {
            return false;
        }

        if self.move_speed == MoveSpeed::Crawl {
            self.last_crawl = !self.last_crawl;
            if self.last_crawl && self.walk_dir == -1 {
                self.walk_dir = self.try_step(members, info, kind);
            }
        } else if self.walk_dir == -1 {
            self.walk_dir = self.try_step(members, info, kind);
            if self.move_speed == MoveSpeed::Run && self.walk_dir != -1 && self.run_dir == -1 {
                self.run_dir = self.try_step(members, info, kind);
            }
        }

        self.steps_taken > 0
    }

    /// Returns `true` if there are any waypoints queued for movement.
    pub const fn has_waypoints(&self) -> bool {
        self.waypoint_index != -1
    }

    /// Returns `true` if the entity is on its last waypoint or has no waypoints at all.
    ///
    /// Used by the interaction engine to determine whether the entity has reached (or
    /// nearly reached) its destination and should attempt to interact with its target.
    pub const fn is_last_or_no_waypoint(&self) -> bool {
        self.waypoint_index <= 0
    }

    /// Queues a single waypoint at the given coordinates, replacing any existing waypoints.
    ///
    /// # Arguments
    /// * `x` - The target x-coordinate.
    /// * `z` - The target z-coordinate.
    ///
    /// # Side Effects
    /// * Sets `waypoints[0]` to the packed coordinate and `waypoint_index` to 0.
    pub const fn queue_waypoint(&mut self, coord: CoordGrid) {
        self.waypoints[0] = coord;
        self.waypoint_index = 0;
    }

    /// Queues multiple waypoints from the given slice, reversing their order.
    ///
    /// The input slice is expected to be in path order (first to last), but the internal
    /// waypoint array stores them in reverse so that the last waypoint is consumed first
    /// (LIFO). At most 25 waypoints are stored.
    ///
    /// # Arguments
    /// * `waypoints` - Packed waypoint coordinates `(x << 14) | z` in path order.
    ///
    /// # Side Effects
    /// * Fills `self.waypoints` in reverse and sets `self.waypoint_index` accordingly.
    pub const fn queue_waypoints(&mut self, waypoints: &[u32]) {
        let len = if waypoints.len() < self.waypoints.len() {
            waypoints.len()
        } else {
            self.waypoints.len()
        };
        let mut index = 0;
        while index < len {
            self.waypoints[index] = CoordGrid::from(waypoints[waypoints.len() - 1 - index]);
            index += 1;
        }
        self.waypoint_index = if len > 0 { len as i32 - 1 } else { -1 };
    }

    /// Clears all queued waypoints, stopping movement.
    ///
    /// # Side Effects
    /// * Sets `waypoint_index` to -1.
    pub fn clear_waypoints(&mut self) {
        self.waypoint_index = -1;
    }

    /// Teleports this PathingEntity to the given coordinate.
    ///
    /// The teleport is silently ignored if the target zone is not allocated
    /// in the collision map. If the destination is on a
    /// different level, a jump flag is set.
    ///
    /// # Arguments
    /// * `coord` - The destination coordinate.
    ///
    /// # Side Effects
    /// * Sets `coord` and marks `tele = true`.
    /// * Sets `jump = true` when changing levels.
    pub fn teleport(&mut self, coord: CoordGrid) -> Option<(u16, u16)> {
        if !rsmod::is_zone_allocated(coord.x(), coord.z(), coord.y()) {
            return None;
        }

        // Focus entity direction in the correct way when teleporting.
        let current = self.coord;
        let dir = face_dir(
            current.x() as i32,
            current.z() as i32,
            coord.x() as i32,
            coord.z() as i32,
        );

        let (dx, dz) = dir_delta(dir);
        let look_x = (coord.x() as i32 + dx as i32) as u16;
        let look_z = (coord.z() as i32 + dz as i32) as u16;

        self.tele = true;
        // If the entity changes on Y then we have to jump.
        if coord.y() != self.coord.y() {
            self.jump = true;
        }

        // Actually change the coord of the entity and the last stepped coord when teleporting.
        self.coord = coord;
        self.last_step_coord = CoordGrid::new(coord.x().saturating_sub(1), coord.y(), coord.z());

        Some((look_x, look_z))
    }

    /// Maps a `MoveRestrict` to the corresponding `CollisionType` for pathfinding.
    ///
    /// # Arguments
    /// * `move_restrict` - The movement restriction type.
    ///
    /// # Returns
    /// The collision type to use for `can_travel` checks, or `None` for `NoMove`
    /// entities that cannot move at all.
    pub fn collision_type(move_restrict: MoveRestrict) -> Option<CollisionType> {
        match move_restrict {
            MoveRestrict::Normal | MoveRestrict::Player => Some(CollisionType::Normal),
            MoveRestrict::Blocked => Some(CollisionType::Blocked),
            MoveRestrict::BlockedNormal => Some(CollisionType::LineOfSight),
            MoveRestrict::Indoors => Some(CollisionType::Indoors),
            MoveRestrict::Outdoors => Some(CollisionType::Outdoors),
            MoveRestrict::NoMove => None,
            MoveRestrict::Passthru => Some(CollisionType::Normal),
        }
    }

    /// Returns the extra collision flag to apply when checking walkability for this
    /// movement restriction type.
    ///
    /// For example, NPC-restricted entities add `CollisionFlag::Npc` to prevent
    /// walking through other NPCs, while player-restricted entities add
    /// `CollisionFlag::Player`.
    ///
    /// # Arguments
    /// * `move_restrict` - The movement restriction type.
    ///
    /// # Returns
    /// The extra collision flag bits to OR into `can_travel` checks.
    pub fn block_walk_extra_flag(move_restrict: MoveRestrict) -> u32 {
        match move_restrict {
            MoveRestrict::Normal
            | MoveRestrict::BlockedNormal
            | MoveRestrict::Indoors
            | MoveRestrict::Outdoors => CollisionFlag::Npc as u32,
            MoveRestrict::Player => CollisionFlag::Player as u32,
            MoveRestrict::Blocked | MoveRestrict::Passthru => CollisionFlag::Open as u32,
            MoveRestrict::NoMove => CollisionFlag::Null as u32,
        }
    }

    /// Attempts to take one step toward the current waypoint using naive pathfinding.
    ///
    /// Calls `take_step` to determine the direction, then updates `self.coord` and the
    /// entity's facing orientation. If the step reaches the current waypoint destination,
    /// advances to the next waypoint. Recurses if the current waypoint is already reached
    /// and there are more waypoints remaining.
    ///
    /// # Arguments
    /// * `members` - Whether this world loads members-only areas; forwarded to
    ///   [`take_step`] / [`can_travel`].
    /// * `info` - The entity's info masks, updated with the new facing direction.
    /// * `kind` - Whether this is a player or NPC focus.
    ///
    /// # Returns
    /// The direction taken (0-7), or -1 if no step could be taken.
    ///
    /// # Side Effects
    /// * Advances `self.coord` by one tile in the step direction.
    /// * Increments `self.steps_taken`.
    /// * Decrements `self.waypoint_index` when a waypoint is reached.
    /// * Updates entity focus/orientation in `info`.
    ///
    /// # Call Stack
    /// **Called by:** `process_movement`
    /// **Calls:** `take_step`, `dir_delta`
    pub fn try_step(&mut self, members: bool, info: &mut EntityMasks, kind: FocusKind) -> i8 {
        let step = self.take_step(members);
        match step {
            None => -1,
            Some(-1) => {
                self.waypoint_index -= 1;
                if self.waypoint_index != -1 {
                    self.try_step(members, info, kind)
                } else {
                    -1
                }
            }
            Some(dir) => {
                let (dx, dz) = dir_delta(dir);
                self.last_step_coord = self.coord;
                self.coord = CoordGrid::new(
                    (self.coord.x() as i32 + dx as i32) as u16,
                    self.coord.y(),
                    (self.coord.z() as i32 + dz as i32) as u16,
                );

                let look_x = (self.coord.x() as i32 + dx as i32) as u16;
                let look_z = (self.coord.z() as i32 + dz as i32) as u16;
                info.focus(
                    kind,
                    CoordGrid::fine(look_x, self.size),
                    CoordGrid::fine(look_z, self.size),
                    false,
                );

                self.steps_taken += 1;

                if self.waypoint_index >= 0 {
                    let coord = self.waypoints[self.waypoint_index as usize];
                    if self.coord.x() == coord.x() && self.coord.z() == coord.z() {
                        self.waypoint_index -= 1;
                    }
                }

                dir
            }
        }
    }

    /// Determines the direction to step toward the current waypoint, with collision checks.
    ///
    /// For entities larger than 1 tile, attempts x-axis and z-axis movement separately.
    /// For 1-tile entities, tries the diagonal direction first, then falls back to
    /// x-only or z-only movement if the diagonal is blocked.
    ///
    /// # Arguments
    /// * `members` - Whether this world loads members-only areas; forwarded to
    ///   [`can_travel`] so members tiles are blocked on a free-to-play world.
    ///
    /// # Returns
    /// * `None` - No waypoints are queued.
    /// * `Some(-1)` - Entity is already at the waypoint destination, or cannot move
    ///   due to `NoMove` restriction, or all directions are blocked.
    /// * `Some(0..=7)` - The direction to move (see `dir_delta` for mapping).
    ///
    /// # Call Stack
    /// **Called by:** `try_step`
    /// **Calls:** `collision_type`, `block_walk_extra_flag`, `face_dir`, `dir_delta`, `can_travel`
    pub fn take_step(&self, members: bool) -> Option<i8> {
        if self.waypoint_index == -1 {
            return None;
        }

        let mr = self.move_restrict;
        let Some(collision) = Self::collision_type(mr) else {
            return Some(-1);
        };

        let extra_flag = Self::block_walk_extra_flag(mr);

        let src_x = self.coord.x();
        let src_z = self.coord.z();
        let coord = self.waypoints[self.waypoint_index as usize];
        let dest_x = coord.x();
        let dest_z = coord.z();

        let walk = |dx: i8, dz: i8| {
            can_travel(
                members,
                self.coord.y(),
                src_x,
                src_z,
                dx,
                dz,
                self.size,
                extra_flag,
                Self::collision_type(mr).unwrap(),
            )
        };

        if self.size > 1 {
            let try_dir_x = face_dir(src_x as i32, 0, dest_x as i32, 0);
            let (dx, _) = dir_delta(try_dir_x);
            if walk(dx, 0) {
                return Some(try_dir_x);
            }
            let try_dir_z = face_dir(0, src_z as i32, 0, dest_z as i32);
            let (_, dz) = dir_delta(try_dir_z);
            if walk(0, dz) {
                return Some(try_dir_z);
            }
            return Some(-1);
        }

        let dir = face_dir(src_x as i32, src_z as i32, dest_x as i32, dest_z as i32);
        let (dx, dz) = dir_delta(dir);

        if dx == 0 && dz == 0 {
            return Some(-1);
        }

        if can_travel(
            members,
            self.coord.y(),
            src_x,
            src_z,
            dx,
            dz,
            self.size,
            extra_flag,
            collision,
        ) {
            return Some(dir);
        }

        if dx != 0 && walk(dx, 0) {
            return Some(face_dir(
                src_x as i32,
                src_z as i32,
                dest_x as i32,
                src_z as i32,
            ));
        }

        if dz != 0 && walk(0, dz) {
            return Some(face_dir(
                src_x as i32,
                src_z as i32,
                src_x as i32,
                dest_z as i32,
            ));
        }

        None
    }

    /// Resets the pathing state for this entity.
    pub fn reset(&mut self) {
        if self.walk_step.is_none()
            && !self.jump
            && !self.tele
            && self.walk_dir == -1
            && self.run_dir == -1
            && self.last_coord == self.coord
            && self.steps_taken == 0
        {
            return;
        }
        self.walk_step = None;
        self.jump = false;
        self.tele = false;
        self.walk_dir = -1;
        self.run_dir = -1;
        self.last_coord = self.coord;
        self.steps_taken = 0;
    }
}

/// Free-to-play–aware wrapper over [`rsmod::can_travel`].
///
/// On a free-to-play world, movement onto a members-only tile is rejected
/// before the collision map is consulted: if the destination tile
/// `(x + offset_x, z + offset_z)` is not flagged free-to-play, travel is
/// blocked. Members worlds skip this check and behave exactly like
/// [`rsmod::can_travel`].
///
/// `members` is the world's members flag (read from the engine by the caller);
/// all remaining arguments are forwarded unchanged to [`rsmod::can_travel`].
///
/// # Call Stack
/// **Called by:** `PathingEntity::take_step`, NPC interaction movement.
/// **Calls:** `rsmod::is_zone_flagged`, `rsmod::can_travel`.
#[allow(clippy::too_many_arguments)]
pub fn can_travel(
    members: bool,
    level: u8,
    x: u16,
    z: u16,
    offset_x: i8,
    offset_z: i8,
    size: u8,
    extra_flag: u32,
    collision: CollisionType,
) -> bool {
    if !members
        && !rsmod::is_zone_flagged(
            (x as i32 + offset_x as i32) as u16,
            (z as i32 + offset_z as i32) as u16,
            level,
            ZoneFlag::Free as u8,
        )
    {
        return false;
    }
    rsmod::can_travel(level, x, z, offset_x, offset_z, size, extra_flag, collision)
}

/// Computes the direction index (0-7) from a source coordinate to a destination coordinate.
///
/// Uses the sign of the delta in each axis to determine the octant. Returns -1 if
/// the source and destination are the same.
///
/// # Arguments
/// * `src_x` - Source x-coordinate.
/// * `src_z` - Source z-coordinate.
/// * `dest_x` - Destination x-coordinate.
/// * `dest_z` - Destination z-coordinate.
///
/// # Returns
/// A direction index: 0=NW, 1=N, 2=NE, 3=W, 4=E, 5=SW, 6=S, 7=SE, or -1 if no movement.
pub const fn face_dir(src_x: i32, src_z: i32, dest_x: i32, dest_z: i32) -> i8 {
    let dx = (dest_x - src_x).signum();
    let dz = (dest_z - src_z).signum();
    match (dx, dz) {
        (-1, 1) => 0,
        (0, 1) => 1,
        (1, 1) => 2,
        (-1, 0) => 3,
        (1, 0) => 4,
        (-1, -1) => 5,
        (0, -1) => 6,
        (1, -1) => 7,
        _ => -1,
    }
}

/// The (dx, dz) tile delta for each direction index 0-7.
///
/// Indexed by the direction values produced by [`face_dir`]:
/// 0=NW, 1=N, 2=NE, 3=W, 4=E, 5=SW, 6=S, 7=SE.
const DIR_DELTAS: [(i8, i8); 8] = [
    (-1, 1),
    (0, 1),
    (1, 1),
    (-1, 0),
    (1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

/// Returns the (dx, dz) tile delta for the given direction index.
///
/// # Arguments
/// * `dir` - Direction index (0-7). Any other value returns `(0, 0)`.
///
/// # Returns
/// A tuple `(dx, dz)` where each component is -1, 0, or 1.
pub const fn dir_delta(dir: i8) -> (i8, i8) {
    if dir >= 0 && (dir as usize) < DIR_DELTAS.len() {
        DIR_DELTAS[dir as usize]
    } else {
        (0, 0)
    }
}

/// Converts a numeric direction index (0-7) to the corresponding `Direction` enum variant.
///
/// Any unrecognized value (including -1) maps to `Direction::South` as a default.
///
/// # Arguments
/// * `dir` - Numeric direction index.
///
/// # Returns
/// The corresponding `Direction` enum variant.
pub fn dir_to_direction(dir: i8) -> Direction {
    match dir {
        0 => Direction::NorthWest,
        1 => Direction::North,
        2 => Direction::NorthEast,
        3 => Direction::West,
        4 => Direction::East,
        5 => Direction::SouthWest,
        6 => Direction::South,
        7 => Direction::SouthEast,
        _ => Direction::South,
    }
}
