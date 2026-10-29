use rs_grid::CoordGrid;
use rs_info::{EntityMasks, FocusKind};
use rs_pack::types::{LocAngle, LocLayer, LocShape};
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{LocSerial, ObjSerial};

/// Represents the target of an entity's interaction action.
///
/// An interaction target identifies what an entity (player or NPC) is trying to
/// interact with -- a ground object, a placed location, another NPC, or another
/// player. The variant determines how the interaction engine resolves approach
/// range, pathfinding, and which script triggers fire.
#[derive(Debug, Clone, Copy)]
pub enum InteractionTarget {
    /// A ground object target, identified by its coordinate, type id, and the
    /// serial of the exact obj that was clicked.
    Obj {
        coord: CoordGrid,
        id: u16,
        serial: ObjSerial,
    },
    /// A placed location target, identified by its coordinate, type id, dimensions,
    /// shape, angle, layer, and the serial of the exact loc that was clicked.
    Loc {
        coord: CoordGrid,
        id: u16,
        width: u8,
        length: u8,
        shape: LocShape,
        angle: LocAngle,
        layer: LocLayer,
        serial: LocSerial,
    },
    /// An NPC target, identified by its NPC index.
    Npc { nid: u16 },
    /// A player target, identified by its player index.
    Player { pid: u16 },
}

impl InteractionTarget {
    /// Returns the grid coordinate of this target.
    ///
    /// For `Obj` and `Loc` targets, returns the stored coordinate. For `Npc` and
    /// `Player` targets, returns the origin `(0, 0, 0)` because their position is
    /// resolved dynamically from the entity's pathing state.
    pub const fn coord(&self) -> CoordGrid {
        match self {
            Self::Obj { coord, .. } | Self::Loc { coord, .. } => *coord,
            _ => CoordGrid::new(0, 0, 0),
        }
    }

    /// Returns `true` if this target is a moving entity (NPC or player).
    ///
    /// Pathing entities require the interaction engine to re-check distance each tick
    /// because the target may have moved since the interaction was initiated.
    pub const fn is_pathing_entity(&self) -> bool {
        matches!(self, Self::Npc { .. } | Self::Player { .. })
    }

    /// Returns the fine (sub-tile) center coordinate of this target for face-direction
    /// calculations, or `None` when it must be resolved from a live entity.
    ///
    /// `Obj` targets return the fine center of a 1x1 tile; `Loc` targets return the fine
    /// center accounting for width and length. `Npc` and `Player` targets return `None`:
    /// the variant stores only an index, so the coordinate cannot be derived here -- the
    /// caller must read it from the target's live pathing position (see
    /// `Engine::resolve_pathing_face`).
    pub const fn fine_coord(&self) -> Option<(u16, u16)> {
        match self {
            Self::Obj { coord, .. } => {
                Some((CoordGrid::fine(coord.x(), 1), CoordGrid::fine(coord.z(), 1)))
            }
            Self::Loc {
                coord,
                width,
                length,
                ..
            } => Some((
                CoordGrid::fine(coord.x(), *width),
                CoordGrid::fine(coord.z(), *length),
            )),
            Self::Npc { .. } | Self::Player { .. } => None,
        }
    }
}

/// Tracks the current interaction state of an entity (player or NPC).
///
/// Holds the target being interacted with, the trigger operation code, subject type/com
/// identifiers for script lookups, and approach range configuration. The `ap_range_called`
/// flag indicates whether a script has explicitly set the approach range this tick, which
/// affects whether the interaction is cleared at the end of the tick.
pub struct InteractionState {
    pub target: Option<InteractionTarget>,
    pub target_op: Option<u8>,
    pub target_subject_type: Option<u16>,
    pub target_subject_com: Option<u16>,
    pub ap_range: Option<u16>,
    pub ap_range_called: bool,
    pub target_x: i32,
    pub target_z: i32,
    pub last_path_src: u32,
    pub last_path_dst: u32,
}

impl InteractionState {
    /// Creates a new `InteractionState` with no active target and default approach range.
    ///
    /// # Returns
    /// An idle interaction state with `ap_range` set to 10 and all targets/ops set to `None`.
    pub fn new() -> Self {
        Self {
            target: None,
            target_op: None,
            target_subject_type: None,
            target_subject_com: None,
            ap_range: Some(10),
            ap_range_called: false,
            target_x: -1,
            target_z: -1,
            last_path_src: 0,
            last_path_dst: 0,
        }
    }

    /// Sets a new interaction target, resetting approach range and path tracking.
    ///
    /// Returns the target's fine coordinate for an immediate focus call, or `None` for
    /// pathing-entity targets whose coordinate is resolved from their live position each
    /// tick. The stationary `target_x`/`target_z` are recorded only when a coordinate is
    /// available (i.e. for non-pathing targets).
    pub fn set(&mut self, target: InteractionTarget, op: u8) -> Option<(u16, u16)> {
        self.target = Some(target);
        self.target_op = Some(op);
        self.reset_path_and_range();
        self.target_subject_com = None;

        match &target {
            InteractionTarget::Obj { id, .. } | InteractionTarget::Loc { id, .. } => {
                self.target_subject_type = Some(*id);
            }
            InteractionTarget::Npc { .. } | InteractionTarget::Player { .. } => {
                self.target_subject_type = None;
            }
        }

        let coord = target.fine_coord();
        if let Some((fine_x, fine_z)) = coord {
            self.target_x = fine_x as i32;
            self.target_z = fine_z as i32;
        }

        coord
    }

    /// Resets all interaction fields to their default idle state.
    ///
    /// # Side Effects
    /// * Clears `target`, `target_op`, `target_subject_type`, `target_subject_com`.
    /// * Resets `ap_range` to `Some(10)` and `ap_range_called` to `false`.
    /// * Resets `target_x` and `target_z` to `-1`.
    pub fn clear(&mut self) {
        self.target = None;
        self.target_op = None;
        self.target_subject_type = None;
        self.target_subject_com = None;
        self.reset_path_and_range();
        // self.target_x = -1;
        // self.target_z = -1;
    }

    /// Resets the approach-range and path-tracking fields shared by `set` and `clear`.
    ///
    /// # Side Effects
    /// * Resets `ap_range` to `Some(10)` and `ap_range_called` to `false`.
    /// * Resets `last_path_src` and `last_path_dst` to `0`.
    fn reset_path_and_range(&mut self) {
        self.ap_range = Some(10);
        self.ap_range_called = false;
        self.last_path_src = 0;
        self.last_path_dst = 0;
    }

    /// Returns `true` if there is an active interaction target.
    pub fn has_interaction(&self) -> bool {
        // The follow interaction doesn't do anything
        if self.target_op == Some(ServerTriggerType::ApPlayer3 as u8)
            || self.target_op == Some(ServerTriggerType::OpPlayer3 as u8)
        {
            return false;
        }
        self.target.is_some()
    }

    /// Updates the face-entity info mask to reflect the current interaction
    /// target. Players are encoded as `pid + 32768`, NPCs as raw `nid`.
    /// Marks the mask if the value changed.
    pub fn set_face_entity(&self, info: &mut EntityMasks, kind: FocusKind) {
        let temp = info.face_entity;
        if let Some(target) = self.target {
            match &target {
                InteractionTarget::Player { pid } => {
                    info.set_face_entity_check(kind, *pid + 32768);
                }
                InteractionTarget::Npc { nid } => {
                    info.set_face_entity_check(kind, *nid);
                }
                _ => {
                    if info.face_entity.is_some() {
                        info.face_entity = None;
                    }
                }
            }
        } else if info.face_entity.is_some() {
            info.face_entity = None;
        }
        if temp != info.face_entity {
            info.mark_face_entity(kind);
        }
    }

    /// Sets the entity's orientation to face south (default idle direction).
    pub fn unfocus(&self, info: &mut EntityMasks, coord: CoordGrid, size: u8) {
        info.orientation_x = Some(CoordGrid::fine(coord.x(), size));
        info.orientation_z = Some(CoordGrid::fine(coord.z().wrapping_sub(1), size));
    }

    /// Updates the entity's facing direction toward the current interaction target.
    ///
    /// `pathing_face` is the live fine coordinate of a player/NPC target, resolved by the
    /// caller (`None` for non-pathing or despawned targets). A live pathing target takes
    /// priority and is re-faced every tick. Otherwise, if a stationary target coordinate
    /// is stored and the entity has stopped moving this tick, it is faced once and cleared.
    ///
    /// The live pathing target uses `client = false` -- the client already tracks it via
    /// the `FaceEntity` mask. The stationary refocus uses `client = true`, re-broadcasting
    /// the `FaceCoord` to observers on arrival.
    pub fn reorient(
        &mut self,
        info: &mut EntityMasks,
        kind: FocusKind,
        steps_taken: u8,
        pathing_face: Option<(u16, u16)>,
    ) {
        if let Some((fine_x, fine_z)) = pathing_face {
            info.focus(kind, fine_x, fine_z, false);
        } else if self.target_x != -1 && steps_taken == 0 {
            info.focus(kind, self.target_x as u16, self.target_z as u16, true);
            self.target_x = -1;
            self.target_z = -1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj_target(x: u16, z: u16) -> InteractionTarget {
        InteractionTarget::Obj {
            coord: CoordGrid::new(x, 0, z),
            id: 100,
            serial: ObjSerial::FIRST,
        }
    }

    fn loc_target(x: u16, z: u16) -> InteractionTarget {
        InteractionTarget::Loc {
            coord: CoordGrid::new(x, 0, z),
            id: 200,
            width: 1,
            length: 1,
            shape: LocShape::CentrepieceStraight,
            angle: LocAngle::North,
            layer: LocLayer::Ground,
            serial: LocSerial::FIRST,
        }
    }

    #[test]
    fn obj_is_not_pathing_entity() {
        assert!(!obj_target(3222, 3222).is_pathing_entity());
    }

    #[test]
    fn loc_is_not_pathing_entity() {
        assert!(!loc_target(3222, 3222).is_pathing_entity());
    }

    #[test]
    fn npc_is_pathing_entity() {
        assert!(InteractionTarget::Npc { nid: 1 }.is_pathing_entity());
    }

    #[test]
    fn player_is_pathing_entity() {
        assert!(InteractionTarget::Player { pid: 1 }.is_pathing_entity());
    }

    #[test]
    fn obj_coord() {
        let target = obj_target(3225, 3230);
        assert_eq!(target.coord().x(), 3225);
        assert_eq!(target.coord().z(), 3230);
    }

    #[test]
    fn loc_coord() {
        let target = loc_target(3100, 3200);
        assert_eq!(target.coord().x(), 3100);
        assert_eq!(target.coord().z(), 3200);
    }

    #[test]
    fn npc_coord_returns_origin() {
        let target = InteractionTarget::Npc { nid: 5 };
        assert_eq!(target.coord().x(), 0);
        assert_eq!(target.coord().z(), 0);
    }

    #[test]
    fn player_coord_returns_origin() {
        let target = InteractionTarget::Player { pid: 5 };
        assert_eq!(target.coord().x(), 0);
        assert_eq!(target.coord().z(), 0);
    }

    #[test]
    fn copy_semantics() {
        let a = obj_target(10, 20);
        let b = a;
        assert_eq!(a.coord().x(), b.coord().x());
    }

    #[test]
    fn obj_fine_coord_is_some() {
        // fine = pos * 2 + size, with size 1 for a 1x1 obj tile.
        assert_eq!(obj_target(3225, 3230).fine_coord(), Some((6451, 6461)));
    }

    #[test]
    fn loc_fine_coord_is_some() {
        assert_eq!(loc_target(3100, 3200).fine_coord(), Some((6201, 6401)));
    }

    #[test]
    fn pathing_targets_have_no_fine_coord() {
        // Index-only targets can't produce a coordinate; the caller resolves it live.
        assert_eq!(InteractionTarget::Npc { nid: 1 }.fine_coord(), None);
        assert_eq!(InteractionTarget::Player { pid: 1 }.fine_coord(), None);
    }
}
