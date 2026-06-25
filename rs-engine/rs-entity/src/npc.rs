use crate::InteractionTarget;
use crate::interaction::InteractionState;
use crate::lifetime::EntityLifeTime;
use crate::pathing::{MoveStrategy, PathingEntity};
use crate::state::EntityState;
use rs_grid::CoordGrid;
use rs_hero::HeroPoints;
use rs_info::{EntityMasks, FocusKind};
use rs_pack::types::{MoveRestrict, NpcMode};
use rs_stat::Stats;
use rs_var::VarSet;
pub use rs_vm::NpcUid;

/// A non-player character in the game world.
///
/// NPCs are spawned from the map or at runtime and have AI behavior driven by
/// modes (wander, patrol, hunt), interaction state machines, combat levels, and
/// script triggers. Each NPC has its own pathing entity for movement, entity state
/// for script execution, and info masks for client updates.
pub struct Npc {
    pub uid: NpcUid,
    pub vars: VarSet,
    pub pathing: PathingEntity,
    pub state: EntityState,
    pub spawn_coord: CoordGrid,
    pub default_mode: NpcMode,
    pub wander_range: u8,
    pub max_range: u16,
    pub stuck_counter: u16,
    pub info: EntityMasks,
    pub interaction: InteractionState,
    pub target_player: Option<u16>,
    pub stats: Stats<6>,
    pub hero_points: HeroPoints,
    pub active: bool,
    pub lifecycle: EntityLifeTime,
    pub respawn_at: Option<u32>,
    pub base_type: u16,
    pub hunt_mode: Option<u16>,
    pub hunt_range: u8,
    pub hunt_clock: u16,
    pub hunt_target: Option<InteractionTarget>,
    pub timer_interval: Option<u16>,
    pub timer_clock: u16,
    pub regen_clock: i16,
    pub next_patrol_point: u8,
    pub patrol_delay_ticks_remaining: i64,
    pub observers: u16,
    pub revert_at: Option<u32>,
    pub revert_reset: bool,
    pub walktrigger: Option<i32>,
    pub walktrigger_arg: i32,
}

impl Npc {
    /// Creates a new NPC with the given type, index, position, size, and variables.
    ///
    /// The NPC starts with default idle state: no interaction target, no hunt, no timer,
    /// all combat levels set to 1, and active visibility.
    ///
    /// # Arguments
    /// * `id` - The NPC type identifier from the config.
    /// * `nid` - The NPC slot index in the engine's NPC array.
    /// * `coord` - The spawn coordinate.
    /// * `size` - The collision size in tiles.
    /// * `vars` - The variable set for this NPC type.
    ///
    /// # Returns
    /// A new `Npc` ready to be added to the engine.
    ///
    /// # Call Stack
    /// **Called by:** `Engine::add_npc_spawned`, map loading
    pub fn new(id: u16, nid: u16, coord: CoordGrid, size: u8, vars: VarSet) -> Self {
        Self {
            uid: NpcUid::new(id, nid),
            vars,
            pathing: PathingEntity::new(coord, size, MoveRestrict::Normal, MoveStrategy::Naive),
            state: EntityState::new(),
            spawn_coord: coord,
            default_mode: NpcMode::None,
            wander_range: 0,
            max_range: 0,
            stuck_counter: 0,
            info: EntityMasks::new(),
            interaction: InteractionState::new(),
            target_player: None,
            stats: Stats::new(1),
            hero_points: HeroPoints::new(),
            active: true,
            lifecycle: EntityLifeTime::Respawn,
            respawn_at: None,
            base_type: id,
            hunt_mode: None,
            hunt_range: 0,
            hunt_clock: 0,
            hunt_target: None,
            timer_interval: None,
            timer_clock: 0,
            regen_clock: 0,
            next_patrol_point: 0,
            patrol_delay_ticks_remaining: -1,
            observers: 0,
            revert_at: None,
            revert_reset: false,
            walktrigger: None,
            walktrigger_arg: 0,
        }
    }

    /// Clears the NPC's interaction state and resets it to the `NpcMode::None` operation.
    ///
    /// Also clears the face entity mask and marks it as changed for client updates.
    ///
    /// # Side Effects
    /// * Calls `InteractionState::clear()`.
    /// * Sets `target_op` to `NpcMode::None`.
    /// * Clears `info.face_entity` and flags the NPC face entity mask.
    pub fn clear_interaction(&mut self) {
        self.interaction.clear();
        self.interaction.target_op = Some(NpcMode::None as u8);
        self.info.face_entity = None;
        self.info.face_entity_npc();
    }

    /// Resets patrol progress so the NPC restarts its route from the first
    /// point with a fresh stuck timer and an uninitialized delay countdown.
    ///
    /// # Side Effects
    /// * Resets `next_patrol_point`, `stuck_counter`, and `patrol_delay_ticks_remaining`.
    pub fn clear_patrol(&mut self) {
        self.next_patrol_point = 0;
        self.stuck_counter = 0;
        self.patrol_delay_ticks_remaining = -1;
    }

    /// Sets a new interaction target for this NPC.
    ///
    /// Configures the interaction state with the target, operation code, and approach
    /// range. For `Obj` and `Loc` targets, stores the type id as the subject type and
    /// records the fine coordinate for orientation. For `Npc` and `Player` targets,
    /// the subject type is set to `None`.
    ///
    /// # Arguments
    /// * `target` - The interaction target (obj, loc, npc, or player).
    /// * `op` - The trigger operation code (NPC mode value).
    /// * `is_engine` - Whether this interaction was initiated by the engine (affects
    ///   whether orientation is locked for non-pathing targets).
    ///
    /// # Side Effects
    /// * Mutates `self.interaction` fields.
    /// * Updates NPC orientation via `self.info.focus_npc`.
    pub fn set_interaction(&mut self, target: InteractionTarget, op: u8, is_engine: bool) {
        // Only non-pathing targets yield a coordinate to face here; pathing-entity targets
        // are tracked via the FaceEntity mask and re-faced each tick in `reorient`.
        if let Some((fine_x, fine_z)) = self.interaction.set(target, op) {
            self.info.focus_npc(fine_x, fine_z, is_engine);
        }
    }

    /// Updates the face-entity info mask to reflect the current interaction target.
    ///
    /// If the target is a player, the face entity id is the player index offset by 32768.
    /// If the target is an NPC, the face entity id is the NPC index. For all other target
    /// types (or no target), the face entity is cleared. Only flags the mask as changed
    /// if the value actually changed.
    ///
    /// # Side Effects
    /// * Mutates `self.info.face_entity`.
    /// * Calls `self.info.face_entity_npc()` if the value changed.
    pub fn set_face_entity(&mut self) {
        self.interaction
            .set_face_entity(&mut self.info, FocusKind::Npc);
    }

    /// Resets the NPC's AI-related fields to their configured defaults.
    ///
    /// Clears the current interaction and reconfigures the hunt and timer settings
    /// from the NPC type config values. This is called when an NPC finishes its
    /// current behavior and returns to its default mode.
    ///
    /// # Arguments
    /// * `default_mode` - The NPC's default behavior mode (e.g., wander, patrol).
    /// * `hunt_mode` - The hunt mode trigger to use, or `None` for no hunting.
    /// * `hunt_range` - The range in tiles at which hunting begins.
    /// * `timer_interval` - The timer interval in ticks, or `None` for no timer.
    ///
    /// # Side Effects
    /// * Calls `clear_interaction`.
    /// * Resets `hunt_mode`, `hunt_range`, `hunt_clock`, `hunt_target`, and `timer_interval`.
    /// * Sets `target_op` to `default_mode`.
    /// * Clears `face_entity` and flags the NPC face entity mask.
    pub fn reset_defaults(
        &mut self,
        default_mode: NpcMode,
        hunt_mode: Option<u16>,
        hunt_range: u8,
        timer_interval: Option<u16>,
    ) {
        self.clear_interaction();
        self.interaction.target_op = Some(default_mode as u8);
        self.info.face_entity = None;
        self.info.face_entity_npc();
        self.hunt_mode = hunt_mode;
        self.hunt_range = hunt_range;
        self.hunt_clock = 0;
        self.hunt_target = None;
        self.timer_interval = timer_interval;
        self.stuck_counter = 0;
    }

    /// Sets the NPC's orientation to face south (its default idle direction).
    pub fn unfocus(&mut self) {
        self.interaction
            .unfocus(&mut self.info, self.pathing.coord, self.pathing.size);
    }

    /// Updates the NPC's facing direction toward its current interaction target.
    /// `pathing_face` is the live fine coordinate of a player/NPC target (resolved by the
    /// caller), or `None`; a non-pathing target falls back to its stored stationary
    /// coordinate on arrival.
    pub fn reorient(&mut self, pathing_face: Option<(u16, u16)>) {
        self.interaction.reorient(
            &mut self.info,
            FocusKind::Npc,
            self.pathing.steps_taken,
            pathing_face,
        );
    }

    /// Resets the NPC's pathing and state for a new tick or after respawning.
    ///
    /// When `respawn` is `true`, performs a full reset: restores the NPC type to its
    /// base type, regenerates the UID, resets all combat levels to base values, clears
    /// hero points and script queues, sets the teleport flag, and clears revert state.
    ///
    /// When `respawn` is `false`, performs a per-tick reset: clears info masks, walk
    /// step, directions, steps taken, protect flag, and ap_range_called. Also updates
    /// the face entity.
    ///
    /// # Arguments
    /// * `respawn` - `true` for a full respawn reset, `false` for a per-tick reset.
    ///
    /// # Side Effects
    /// * Mutates pathing, info, state, and interaction fields.
    /// * Calls `unfocus` and `set_face_entity`.
    pub fn reset_pathing_entity(&mut self, respawn: bool) {
        if respawn {
            self.uid = NpcUid::new(self.base_type, self.uid.nid());
            self.unfocus();
            self.stats.reset();
            self.hero_points.clear();
            self.state.queues.queue.clear();
            self.pathing.clear_waypoints();
            self.pathing.tele = true;
            self.revert_at = None;
            self.revert_reset = false;
        } else {
            self.info.reset();
            self.pathing.walk_step = None;
            self.pathing.tele = false;
            self.pathing.walk_dir = -1;
            self.pathing.run_dir = -1;
            self.pathing.steps_taken = 0;
            self.state.protect = false;
            self.interaction.ap_range_called = false;
            self.set_face_entity();
        }
    }
}
