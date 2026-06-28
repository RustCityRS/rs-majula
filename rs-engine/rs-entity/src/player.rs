use crate::build::BuildArea;
use crate::direction::Direction;
use crate::interaction::InteractionState;
use crate::interaction::InteractionTarget;
use crate::pathing::{MoveStrategy, PathingEntity};
use crate::state::EntityState;
use rs_cam::CamQueue;
use rs_grid::CoordGrid;
use rs_hero::HeroPoints;
use rs_info::{EntityMasks, FocusKind};
use rs_inv::Inventory;
use rs_pack::types::{BlockWalk, MoveRestrict, PlayerStat};
use rs_stat::Stats;
use rs_var::VarSet;
pub use rs_vm::PlayerUid;
use rs_vm::state::ExecutionState;
use rustc_hash::{FxHashMap, FxHashSet};

/// No modal interface is open.
pub const MODAL_NONE: u8 = 0;
/// Bitmask flag: the main (fullscreen) modal interface is open.
pub const MODAL_MAIN: u8 = 1 << 0;
/// Bitmask flag: the chatbox modal interface is open.
pub const MODAL_CHAT: u8 = 1 << 1;
/// Bitmask flag: the sidebar modal interface is open.
pub const MODAL_SIDE: u8 = 1 << 2;
/// Bitmask flag: the tutorial modal interface is open.
pub const MODAL_TUT: u8 = 1 << 3;

/// The staff moderation level of a player, controlling access to privileged commands
/// and visibility of the moderator crown icon.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaffModLevel {
    /// Regular player with no moderation privileges.
    Normal = 0,
    /// Player moderator with mute capabilities.
    PlayerModerator = 1,
    /// Jagex moderator with full moderation capabilities.
    JagexModerator = 2,
    /// Developer with unrestricted access.
    Developer = 3,
}

impl StaffModLevel {
    /// Converts a stored numeric staff level back into the enum, mapping any
    /// unrecognized value to [`StaffModLevel::Normal`].
    ///
    /// This is the inverse of `staff_mod_level as u8` and is used when
    /// restoring a persisted profile.
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::PlayerModerator,
            2 => Self::JagexModerator,
            3 => Self::Developer,
            _ => Self::Normal,
        }
    }
}

/// The movement speed mode of an entity, determining how many tiles it moves per tick.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveSpeed {
    /// Entity does not move (e.g., during a cutscene or when locked in place).
    Stationary = 0,
    /// Entity moves one tile every other tick (half speed).
    Crawl = 1,
    /// Entity moves one tile per tick (normal speed).
    Walk = 2,
    /// Entity moves two tiles per tick (double speed).
    Run = 3,
}

/// Public chat visibility setting.
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum ChatSettingsPublic {
    /// Public chat visible to all.
    On = 0,
    /// Public chat visible to friends only.
    Friends = 1,
    /// Public chat disabled.
    Off = 2,
    /// Public chat hidden from the player's own view.
    Hide = 3,
}

/// Private message visibility setting.
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum ChatSettingsPrivate {
    /// Private messages from anyone.
    On = 0,
    /// Private messages from friends only.
    Friends = 1,
    /// Private messages disabled.
    Off = 2,
}

/// Trade and duel request visibility setting.
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum ChatSettingsTradeDuel {
    /// Trade/duel requests from anyone.
    On = 0,
    /// Trade/duel requests from friends only.
    Friends = 1,
    /// Trade/duel requests disabled.
    Off = 2,
}

impl ChatSettingsPublic {
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Friends,
            2 => Self::Off,
            3 => Self::Hide,
            _ => Self::On,
        }
    }
}

impl ChatSettingsPrivate {
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Friends,
            2 => Self::Off,
            _ => Self::On,
        }
    }
}

impl ChatSettingsTradeDuel {
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Friends,
            2 => Self::Off,
            _ => Self::On,
        }
    }
}

/// A player entity in the game world with full game state.
///
/// Contains all per-player state: identity, movement/pathing, combat stats, skill
/// levels, inventories, interface modals, interaction targets, chat settings,
/// appearance, build area viewport, and script execution state. This is the
/// central struct manipulated by the engine, script VM, and network layer.
pub struct Player {
    pub uid: PlayerUid,
    pub low_memory: bool,
    pub info: EntityMasks,
    pub pathing: PathingEntity,
    pub runenergy: u16,
    pub last_runenergy: Option<u16>,
    pub runweight: i32,
    pub playtime: i32,
    pub last_date: i64,
    pub last_login_date: i64,
    pub stats: Stats<21>,
    pub combat_level: u8,
    pub hero_points: HeroPoints,
    pub vars: VarSet,
    pub state: EntityState,
    pub interaction: InteractionState,
    pub cam_queue: CamQueue,
    pub staff_mod_level: StaffModLevel,
    pub is_member: bool,
    pub allow_design: bool,
    pub run_step: Option<Direction>,
    pub run: bool,
    pub temprun: bool,
    pub move_request: bool,
    pub last_com: Option<u16>,
    pub last_comsubid: Option<u16>,
    pub last_slot: Option<u16>,
    pub last_target_slot: Option<u16>,
    pub last_use_slot: Option<u16>,
    pub last_item: Option<u16>,
    pub last_use_item: Option<u16>,
    pub modal_state: u8,
    pub modal_main: Option<u16>,
    pub last_modal_main: Option<u16>,
    pub modal_chat: Option<u16>,
    pub last_modal_chat: Option<u16>,
    pub modal_side: Option<u16>,
    pub last_modal_side: Option<u16>,
    pub modal_tutorial: Option<u16>,
    pub refresh_modal: bool,
    pub refresh_modal_close: bool,
    pub request_modal_close: bool,
    pub resume_buttons: Option<Vec<i32>>,
    pub public: ChatSettingsPublic,
    pub private: ChatSettingsPrivate,
    pub trade: ChatSettingsTradeDuel,
    pub tabs: [Option<u16>; 14],
    pub logout_requested: bool,
    pub logout_idle_requested: bool,
    pub logout_prevented_until: Option<u32>,
    pub logout_prevented_message: Option<Box<str>>,
    pub logout_sent: bool,
    pub disconnected_at: Option<u32>,
    pub last_response: u32,
    pub active: bool,
    pub path: Option<Vec<CoordGrid>>,
    pub block_walk: BlockWalk,
    pub gender: u8,
    pub headicons: u8,
    pub body: [i32; 7],
    pub colours: [u8; 5],
    pub invs: FxHashMap<u16, Inventory>,
    pub inv_transmits: FxHashMap<u16, Vec<u16>>,
    pub inv_other_transmits: FxHashMap<u16, (i32, u16)>,
    pub inv_first_seen: FxHashSet<u16>,
    pub build_area: BuildArea,
    pub last_zone: CoordGrid,
    pub last_map_zone: CoordGrid,
    pub afk_event_ready: bool,
    pub afk_zones: [u32; 2],
    pub last_afk_zone: u16,
    pub opcalled: bool,
    pub next_target: Option<InteractionTarget>,
    pub walktrigger: Option<i32>,
    pub bot: bool,
}

impl Player {
    /// Creates a new player at the given coordinate with the specified identity and variables.
    ///
    /// The player starts with teleport enabled (so the client receives the initial
    /// position), default walk speed, developer staff level, member status, and all
    /// interfaces closed.
    ///
    /// # Arguments
    /// * `uid` - The player's unique identifier (username + slot index).
    /// * `coord` - The starting coordinate.
    /// * `varps` - The variable set (player variables / varps).
    /// * `bot` - Whether this player is a bot (automated) rather than a real client.
    ///
    /// # Returns
    /// A new `Player` with all state initialized to defaults.
    ///
    /// # Call Stack
    /// **Called by:** `Engine::add_player`
    pub fn new(uid: PlayerUid, coord: CoordGrid, vars: VarSet, bot: bool) -> Self {
        let mut pathing = PathingEntity::new(coord, 1, MoveRestrict::Player, MoveStrategy::Smart);
        pathing.tele = true;
        Self {
            uid,
            low_memory: false,
            pathing,
            runenergy: 10000,
            last_runenergy: None,
            runweight: 0,
            playtime: 0,
            last_date: 0,
            last_login_date: 0,
            stats: Stats::new(1),
            combat_level: 3,
            hero_points: HeroPoints::new(),
            vars,
            state: EntityState::new(),
            info: EntityMasks::new(),
            interaction: InteractionState::new(),
            cam_queue: CamQueue::new(),
            #[cfg(debug_assertions)]
            staff_mod_level: StaffModLevel::Developer,
            #[cfg(not(debug_assertions))]
            staff_mod_level: StaffModLevel::Normal,
            is_member: true,
            allow_design: false,
            run_step: None,
            run: false,
            temprun: false,
            move_request: false,
            last_com: None,
            last_comsubid: None,
            last_slot: None,
            last_target_slot: None,
            last_use_slot: None,
            last_item: None,
            last_use_item: None,
            modal_state: MODAL_NONE,
            modal_main: None,
            last_modal_main: None,
            modal_chat: None,
            last_modal_chat: None,
            modal_side: None,
            last_modal_side: None,
            modal_tutorial: None,
            refresh_modal: false,
            refresh_modal_close: false,
            request_modal_close: false,
            resume_buttons: None,
            public: ChatSettingsPublic::On,
            private: ChatSettingsPrivate::On,
            trade: ChatSettingsTradeDuel::On,
            tabs: [None; 14],
            logout_requested: false,
            logout_idle_requested: false,
            logout_prevented_until: None,
            logout_prevented_message: None,
            logout_sent: false,
            disconnected_at: None,
            last_response: 0,
            active: true,
            path: None,
            block_walk: BlockWalk::Npc,
            gender: 0,
            headicons: 0,
            body: [0, 10, 18, 26, 33, 36, 42],
            colours: [0; 5],
            invs: FxHashMap::default(),
            inv_transmits: FxHashMap::default(),
            inv_other_transmits: FxHashMap::default(),
            inv_first_seen: FxHashSet::default(),
            build_area: BuildArea::new(),
            last_zone: CoordGrid::new(0, 0, 0),
            last_map_zone: CoordGrid::new(0, 0, 0),
            afk_event_ready: false,
            afk_zones: [0; 2],
            last_afk_zone: 0,
            opcalled: false,
            next_target: None,
            walktrigger: None,
            bot,
        }
    }

    /// Returns `true` if a blocking modal interface (main or chat) is currently open.
    ///
    /// Modal main and chat interfaces block player actions. Side and tutorial modals
    /// do not block.
    pub const fn contains_modal_interface(&self) -> bool {
        (self.modal_state & (MODAL_MAIN | MODAL_CHAT)) != MODAL_NONE
    }

    /// Returns `true` if the given interface root is currently visible to the player.
    ///
    /// Checks the main modal, chat modal, side modal, tutorial modal, and all 14
    /// tab interfaces. Returns `false` for negative root values.
    ///
    /// # Arguments
    /// * `root` - The interface root id to check. Negative values always return `false`.
    pub fn is_interface_visible(&self, root: i32) -> bool {
        if root < 0 {
            return false;
        }
        let root_u16 = root as u16;
        self.modal_main == Some(root_u16)
            || self.modal_chat == Some(root_u16)
            || self.modal_side == Some(root_u16)
            || self.modal_tutorial == Some(root_u16)
            || self.tabs.iter().flatten().any(|&x| x as i32 == root)
    }

    /// Collects all inventory transmit component IDs attached to the given interface root.
    ///
    /// Iterates over all `inv_transmits` entries and returns the component IDs whose
    /// root layer matches the specified root. These components need to have their
    /// listeners cleared when the interface is closed.
    ///
    /// # Arguments
    /// * `root` - The interface root being closed. If `None`, returns an empty list.
    /// * `get_root_layer` - A closure that maps a component ID to its root layer ID.
    ///
    /// # Returns
    /// A vector of component IDs that were attached to the closing root.
    pub fn clear_interface_listeners<F>(&mut self, root: Option<u16>, get_root_layer: F) -> Vec<u16>
    where
        F: Fn(u16) -> Option<i32>,
    {
        let Some(root) = root else {
            return Vec::new();
        };
        let root = root as i32;
        let mut coms_to_clear = Vec::new();
        for coms in self.inv_transmits.values() {
            for &com in coms {
                if let Some(layer) = get_root_layer(com)
                    && layer == root
                {
                    coms_to_clear.push(com);
                }
            }
        }
        // Cross-player (`invother_transmit`) listeners are keyed by component and
        // live in a separate map, so they must be collected here too -- otherwise a
        // component that only mirrors another player's inventory would leak on close.
        for &com in self.inv_other_transmits.keys() {
            if let Some(layer) = get_root_layer(com)
                && layer == root
            {
                coms_to_clear.push(com);
            }
        }
        coms_to_clear
    }

    /// Removes the given component ID from all inventory transmit lists.
    ///
    /// Called when a component's interface is closed so that inventory updates are
    /// no longer sent to that component.
    ///
    /// # Arguments
    /// * `com` - The component ID to remove from all transmit lists.
    ///
    /// # Side Effects
    /// * Removes `com` from every entry in `self.inv_transmits`.
    pub fn clear_inv_transmits(&mut self, com: u16) {
        for (_, coms) in self.inv_transmits.iter_mut() {
            coms.retain(|&c| c != com);
        }
        self.inv_other_transmits.remove(&com);
        self.inv_first_seen.remove(&com);
    }

    /// Clears the active script if it is suspended waiting for a count dialog or
    /// pause button input.
    ///
    /// This is called when a modal interface is opened, which invalidates any
    /// suspended dialog script that was waiting for player input.
    ///
    /// # Side Effects
    /// * Sets `self.state.active_script` to `None` if the script's execution state
    ///   is `CountDialog` or `PauseButton`.
    /// * Clears `self.resume_buttons`, since the resume targets belonged to the
    ///   script being discarded.
    pub fn clear_suspended_script(&mut self) {
        if let Some(s) = &self.state.active_script
            && matches!(
                s.execution,
                ExecutionState::CountDialog | ExecutionState::PauseButton
            )
        {
            self.state.active_script = None;
            self.resume_buttons = None;
        }
    }

    /// Returns `true` if the player can process new input actions.
    ///
    /// A player cannot be accessed when they are protected (mid-script execution)
    /// or busy (delayed or has a modal interface open).
    pub const fn can_access(&self) -> bool {
        !self.state.protect && !self.busy()
    }

    /// Returns `true` if the player is busy -- either delayed (waiting for a timer)
    /// or has a blocking modal interface open.
    pub const fn busy(&self) -> bool {
        self.state.delayed || self.contains_modal_interface()
    }

    /// Computes the player's combat level from their base skill levels.
    ///
    /// Uses the standard formula: base = 0.25 * (Defence + Hitpoints + floor(Prayer/2)),
    /// then adds the highest of melee (0.325 * (Attack + Strength)),
    /// ranged (0.325 * (floor(Ranged/2) + Ranged)),
    /// or magic (0.325 * (floor(Magic/2) + Magic)).
    ///
    /// # Returns
    /// The computed combat level as a `u8`.
    ///
    /// # Call Stack
    /// **Called by:** `impl ScriptPlayer for ActivePlayer`
    pub fn get_combat_level(&self) -> u8 {
        let levels = &self.stats.base_levels;
        let defence = levels[PlayerStat::Defence as usize] as u32;
        let hitpoints = levels[PlayerStat::Hitpoints as usize] as u32;
        let prayer = levels[PlayerStat::Prayer as usize] as u32;
        let attack = levels[PlayerStat::Attack as usize] as u32;
        let strength = levels[PlayerStat::Strength as usize] as u32;
        let ranged = levels[PlayerStat::Ranged as usize] as u32;
        let magic = levels[PlayerStat::Magic as usize] as u32;

        // floor(x / 2) == x >> 1.
        let base_sum = defence + hitpoints + (prayer >> 1);
        let melee_sum = attack + strength;
        let range_sum = (ranged >> 1) + ranged;
        let magic_sum = (magic >> 1) + magic;
        // max(0.325*a, 0.325*b, 0.325*c) == 0.325 * max(a, b, c).
        let max_sum = melee_sum.max(range_sum).max(magic_sum);

        // The original was floor(0.25*base_sum + 0.325*max_sum). With 0.25 = 10/40
        // and 0.325 = 13/40 that is exactly floor((10*base_sum + 13*max_sum) / 40).
        //   *10 = (x << 3) + (x << 1)      *13 = (x << 3) + (x << 2) + x
        let numerator =
            (base_sum << 3) + (base_sum << 1) + (max_sum << 3) + (max_sum << 2) + max_sum;
        (numerator / 40) as u8
    }

    /// Clears the player's interaction state, removing the active target and resetting
    /// all interaction fields to defaults.
    ///
    /// # Side Effects
    /// * Calls `InteractionState::clear()`.
    pub fn clear_interaction(&mut self) {
        self.interaction.clear();
    }

    /// Sets a new interaction target for this player.
    ///
    /// Configures the interaction state with the target, operation code, and approach
    /// range. For `Obj` and `Loc` targets, stores the type id as the subject type and
    /// records the fine coordinate for orientation. For `Npc` and `Player` targets,
    /// the subject type is set to `None`.
    ///
    /// # Arguments
    /// * `target` - The interaction target (obj, loc, npc, or player).
    /// * `op` - The trigger operation code (e.g., `ApObj1`, `OpLoc3`).
    /// * `is_engine` - Whether this interaction was initiated by the engine (affects
    ///   whether orientation is locked for non-pathing targets).
    ///
    /// # Side Effects
    /// * Mutates `self.interaction` fields.
    /// * Updates player orientation via `self.info.focus_player`.
    ///
    /// # Call Stack
    /// **Called by:** `impl ScriptPlayer for ActivePlayer`, input handlers
    pub fn set_interaction(&mut self, target: InteractionTarget, op: u8, is_engine: bool) {
        // Only non-pathing targets yield a coordinate to face here; pathing-entity targets
        // are tracked via the FaceEntity mask and re-faced each tick in `reorient`.
        if let Some((fine_x, fine_z)) = self.interaction.set(target, op) {
            self.info.focus_player(fine_x, fine_z, is_engine);
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
    /// * Calls `self.info.face_entity_player()` if the value changed.
    pub fn set_face_entity(&mut self) {
        self.interaction
            .set_face_entity(&mut self.info, FocusKind::Player);
    }

    /// Returns `true` if the player has an active interaction target.
    pub fn has_interaction(&self) -> bool {
        self.interaction.has_interaction()
    }

    /// Sets the player's orientation to face south (its default idle direction).
    pub fn unfocus(&mut self) {
        self.interaction
            .unfocus(&mut self.info, self.pathing.coord, self.pathing.size);
    }

    /// Updates the player's facing direction toward its current interaction target.
    /// `pathing_face` is the live fine coordinate of a player/NPC target (resolved by the
    /// caller), or `None`; a non-pathing target falls back to its stored stationary
    /// coordinate on arrival.
    pub fn reorient(&mut self, pathing_face: Option<(u16, u16)>) {
        self.interaction.reorient(
            &mut self.info,
            FocusKind::Player,
            self.pathing.steps_taken,
            pathing_face,
        );
    }

    /// Clears all queued waypoints and the pre-computed path, stopping movement.
    ///
    /// # Side Effects
    /// * Calls `self.pathing.clear_waypoints()`.
    /// * Sets `self.path` to `None`.
    pub fn clear_waypoints(&mut self) {
        self.pathing.clear_waypoints();
        self.path = None;
    }

    /// Opens a chatbox modal interface, closing any conflicting main or side modals.
    ///
    /// # Arguments
    /// * `com` - The interface component ID to open in the chatbox.
    ///
    /// # Returns
    /// `true` if a main or side modal was closed to make room for the chat modal.
    ///
    /// # Side Effects
    /// * Clears `MODAL_MAIN` and `MODAL_SIDE` if they were set.
    /// * Sets `MODAL_CHAT` and `self.modal_chat`.
    /// * Sets `self.refresh_modal` to `true`.
    /// * Calls `clear_suspended_script` to cancel any pending dialog input.
    pub fn open_chat_modal(&mut self, com: u16) -> bool {
        let mut needs_close = false;
        if self.modal_state & MODAL_MAIN != MODAL_NONE {
            needs_close = true;
            self.modal_state &= !MODAL_MAIN;
            self.modal_main = None;
        }
        if self.modal_state & MODAL_SIDE != MODAL_NONE {
            needs_close = true;
            self.modal_state &= !MODAL_SIDE;
            self.modal_side = None;
        }
        self.modal_state |= MODAL_CHAT;
        self.modal_chat = Some(com);
        self.refresh_modal = true;
        self.clear_suspended_script();
        needs_close
    }

    /// Opens both a main and side modal interface, closing any conflicting chat modal.
    ///
    /// # Arguments
    /// * `com` - The interface component ID to open as the main modal.
    /// * `side` - The interface component ID to open as the side modal.
    ///
    /// # Returns
    /// `true` if a chat modal was closed to make room.
    ///
    /// # Side Effects
    /// * Clears `MODAL_CHAT` if it was set.
    /// * Sets `MODAL_MAIN`, `MODAL_SIDE`, `self.modal_main`, and `self.modal_side`.
    /// * Sets `self.refresh_modal` to `true`.
    /// * Calls `clear_suspended_script`.
    pub fn open_main_side_modal(&mut self, com: u16, side: u16) -> bool {
        let mut needs_close = false;
        if self.modal_state & MODAL_CHAT != MODAL_NONE {
            needs_close = true;
            self.modal_state &= !MODAL_CHAT;
            self.modal_chat = None;
        }
        self.modal_state |= MODAL_MAIN;
        self.modal_main = Some(com);
        self.modal_state |= MODAL_SIDE;
        self.modal_side = Some(side);
        self.refresh_modal = true;
        self.clear_suspended_script();
        needs_close
    }

    /// Opens a main (fullscreen) modal interface, closing any conflicting chat or side modals.
    ///
    /// # Arguments
    /// * `com` - The interface component ID to open as the main modal.
    ///
    /// # Returns
    /// `true` if a chat or side modal was closed to make room.
    ///
    /// # Side Effects
    /// * Clears `MODAL_CHAT` and `MODAL_SIDE` if they were set.
    /// * Sets `MODAL_MAIN` and `self.modal_main`.
    /// * Sets `self.refresh_modal` to `true`.
    /// * Calls `clear_suspended_script`.
    pub fn open_main_modal(&mut self, com: u16) -> bool {
        let mut needs_close = false;
        if self.modal_state & MODAL_CHAT != MODAL_NONE {
            needs_close = true;
            self.modal_state &= !MODAL_CHAT;
            self.modal_chat = None;
        }
        if self.modal_state & MODAL_SIDE != MODAL_NONE {
            needs_close = true;
            self.modal_state &= !MODAL_SIDE;
            self.modal_side = None;
        }
        self.modal_state |= MODAL_MAIN;
        self.modal_main = Some(com);
        self.refresh_modal = true;
        self.clear_suspended_script();
        needs_close
    }

    /// Opens a side-panel modal interface.
    ///
    /// Unlike the main and chat openers, this does not close any conflicting
    /// modal.
    ///
    /// # Arguments
    /// * `com` - The interface component ID to open as the side modal.
    ///
    /// # Side Effects
    /// * Sets `MODAL_SIDE` and `self.modal_side`.
    /// * Sets `self.refresh_modal` to `true`.
    /// * Calls `clear_suspended_script` to cancel any pending dialog input.
    pub fn open_side_modal(&mut self, com: u16) {
        self.modal_state |= MODAL_SIDE;
        self.modal_side = Some(com);
        self.refresh_modal = true;
        self.clear_suspended_script();
    }

    /// Records a tutorial interface as the active tutorial modal.
    ///
    /// The `TutOpen` packet itself is sent by the caller; this only updates the
    /// modal bookkeeping so the tutorial is tracked alongside any other open
    /// modals.
    ///
    /// # Arguments
    /// * `com` - The interface component ID opened as the tutorial.
    ///
    /// # Side Effects
    /// * Sets `MODAL_TUT` and `self.modal_tutorial`.
    pub fn open_tutorial(&mut self, com: u16) {
        self.modal_state |= MODAL_TUT;
        self.modal_tutorial = Some(com);
    }

    /// Updates the player's active zone list if they have crossed a zone boundary.
    ///
    /// Compares the player's current zone coordinate with `last_zone` and calls
    /// `rebuild_zones` if they differ, then updates `last_zone`.
    ///
    /// # Side Effects
    /// * May call `self.build_area.rebuild_zones`.
    /// * Updates `self.last_zone`.
    pub fn update_map(&mut self) {
        let zone = CoordGrid::new(
            self.pathing.coord.zone_x() << 3,
            self.pathing.coord.y(),
            self.pathing.coord.zone_z() << 3,
        );
        if self.last_zone != zone {
            self.build_area.rebuild_zones(self.pathing.coord);
            self.last_zone = zone;
        }
    }

    /// Updates AFK zone tracking for the anti-macro/idle detection system.
    ///
    /// Increments the `last_afk_zone` counter (capped at 1000). If the player is still
    /// within one of the two tracked AFK zones, does nothing. Otherwise, records the
    /// current position as a new AFK zone. On teleport (`jump == true`), the second zone
    /// slot is set directly; on walk, the second zone inherits the first and the first
    /// is updated.
    ///
    /// # Side Effects
    /// * Updates `self.last_afk_zone`, `self.afk_zones`, and `self.afk_event_ready`.
    pub fn update_afk_zones(&mut self) {
        self.last_afk_zone = self.last_afk_zone.saturating_add(1).min(1000);
        if self.within_afk_zone() {
            return;
        }
        let coord = CoordGrid::new(
            self.pathing.coord.x().wrapping_sub(10),
            0,
            self.pathing.coord.z().wrapping_sub(10),
        )
        .packed();
        if self.pathing.jump {
            self.afk_zones[1] = coord;
        } else {
            self.afk_zones[1] = self.afk_zones[0];
        }
        self.afk_zones[0] = coord;
        self.last_afk_zone = 0;
    }

    /// Returns `true` if the player's current position is within either of the two
    /// tracked 21x21 AFK zones.
    ///
    /// # Call Stack
    /// **Called by:** `update_afk_zones`
    fn within_afk_zone(&self) -> bool {
        for &zone in &self.afk_zones {
            let coord = CoordGrid::from(zone);
            if CoordGrid::intersects(
                self.pathing.coord.x(),
                self.pathing.coord.z(),
                1,
                1,
                coord.x(),
                coord.z(),
                21,
                21,
            ) {
                return true;
            }
        }
        false
    }

    /// Resets the player's pathing and per-tick state for the start of a new tick.
    ///
    /// When `respawn` is `true`, also calls `unfocus` to reset orientation (used when
    /// the player respawns after death).
    ///
    /// Always resets: info masks, walk step, jump flag, teleport flag, path, walk/run
    /// directions, last coordinate, steps taken, protect flag, opcalled flag,
    /// ap_range_called flag, and movement speed (based on the `run` flag).
    /// Also calls `set_face_entity` to update the face-entity mask.
    ///
    /// # Side Effects
    /// * Mutates pathing, info, state, and interaction fields.
    /// * Calls `unfocus` (if respawning) and `set_face_entity`.
    pub fn reset_pathing_entity(&mut self) {
        self.info.reset();
        self.pathing.reset();
        self.path = None;
        self.state.protect = false;
        self.opcalled = false;
        self.interaction.ap_range_called = false;
        if self.run {
            self.pathing.move_speed = MoveSpeed::Run;
        } else {
            self.pathing.move_speed = MoveSpeed::Walk;
        }
        self.set_face_entity();
    }

    /// Recovers or depletes the player's run energy for the current tick.
    ///
    /// Run energy is held in hundredths of a percent, so the `0..=10000`
    /// range maps to the client's `0..=100` orb. The model mirrors the
    /// reference engine:
    ///
    /// * **Delayed** players are skipped -- energy is frozen for the duration
    ///   of a delay.
    /// * **Standing or taking a single step** (`steps_taken < 2`) recovers
    ///   `floor(agility / 9) + 8`, capped at the 10000 maximum.
    /// * **Running** (`steps_taken >= 2`) drains energy as a function of the
    ///   carried run weight in kilograms (`runweight / 1000`), clamped to
    ///   `0..=64`: `floor(67 + 67 * weight / 64)`, floored at 0.
    ///
    /// When energy is exhausted, run mode is forced off; when it drops below
    /// 1% (100 units), the temporary ctrl-run flag is cleared so the player
    /// reverts to walking. The depleted value itself is transmitted to the
    /// client later in the tick by the engine's run-energy stat sync.
    ///
    /// This type lives in `rs-entity` and has no access to the config cache,
    /// so it cannot write the client's `run` varp directly. Instead, a `true`
    /// return tells the engine-layer caller to push the new run state to the
    /// client (via `ActivePlayer::sync_run`).
    ///
    /// # Returns
    /// `true` if run mode was switched off this tick because energy hit zero --
    /// signaling the caller to sync the `run` varp. `false` otherwise.
    ///
    /// # Side Effects
    /// * Mutates `self.runenergy`, and may clear `self.run` / `self.temprun`.
    ///
    /// # Call Stack
    /// **Called by:** `Engine::process_player`
    pub fn update_energy(&mut self) -> bool {
        if self.state.delayed {
            return false;
        }

        if self.pathing.steps_taken < 2 {
            #[cfg(rev = "225")]
            let recovered = self.stats.base_levels[PlayerStat::Agility as usize] / 9 + 8;
            #[cfg(since_244)]
            // https://runescape.wiki/w/Update:Agility_improved_and_bug_fixes
            let recovered = self.stats.base_levels[PlayerStat::Agility as usize] / 6 + 8;
            self.runenergy = (self.runenergy + recovered).min(10000);
        } else {
            let weight_kg = self.runweight as f64 / 1000.0;
            let clamp_weight = weight_kg.clamp(0.0, 64.0);
            let loss = (67.0 + (67.0 * clamp_weight) / 64.0) as u16;
            self.runenergy = self.runenergy.saturating_sub(loss);
        }

        // Only signal a varp sync on the actual on->off transition, so a player
        // running at empty energy does not re-transmit the run varp every tick.
        let mut run_disabled = false;
        if self.runenergy == 0 && self.run {
            self.run = false;
            run_disabled = true;
        }
        if self.runenergy < 100 {
            self.temprun = false;
        }
        run_disabled
    }
}

#[cfg(test)]
mod combat_level_tests {
    use super::*;

    /// Faithful copy of the original f64 combat-level formula, parameterized by
    /// the already-reduced sums. This is the reference the integer rewrite of
    /// `get_combat_level` must reproduce exactly.
    fn float_ref(base_sum: u32, melee_sum: u32, range_sum: u32, magic_sum: u32) -> u8 {
        let base = 0.25 * base_sum as f64;
        let melee = 0.325 * melee_sum as f64;
        let range = 0.325 * range_sum as f64;
        let magic = 0.325 * magic_sum as f64;
        (base + melee.max(range).max(magic)).floor() as u8
    }

    /// The integer core of `Player::get_combat_level` (kept in sync with it).
    fn int_impl(base_sum: u32, max_sum: u32) -> u8 {
        let numerator =
            (base_sum << 3) + (base_sum << 1) + (max_sum << 3) + (max_sum << 2) + max_sum;
        (numerator / 40) as u8
    }

    /// Exhaustively proves the integer formula is bit-identical to the f64 one
    /// over the entire reachable domain. The result depends only on:
    ///   base_sum = Defence + Hitpoints + floor(Prayer/2)  <= 99+99+49 = 247
    ///   max_sum  = max(Atk+Str, floor(Rng/2)+Rng, ...)    <= 99+99    = 198
    /// 0.325 is not exactly representable in f64, so this is the test that
    /// guarantees no boundary case rounds differently.
    #[test]
    fn integer_combat_level_matches_float() {
        for base_sum in 0..=247u32 {
            for max_sum in 0..=198u32 {
                assert_eq!(
                    int_impl(base_sum, max_sum),
                    float_ref(base_sum, max_sum, 0, 0),
                    "mismatch at base_sum={base_sum} max_sum={max_sum}",
                );
            }
        }
    }

    fn player_with(levels: [u8; 7]) -> Player {
        let uid = PlayerUid::new("test".into(), 1);
        let varps = VarSet::new(std::iter::empty());
        let mut player = Player::new(uid, CoordGrid::new(3222, 0, 3222), varps, false);
        // [Attack, Defence, Strength, Hitpoints, Ranged, Prayer, Magic]
        for (i, &lvl) in levels.iter().enumerate() {
            player.stats.base_levels[i] = lvl as u16;
        }
        player
    }

    /// Binds the real `get_combat_level` to the float reference for a spread of
    /// melee / ranged / magic / mixed profiles (so a future edit to the function
    /// that drifts from the verified math is caught here, not on the wire).
    #[test]
    fn real_function_matches_float_ref() {
        let cases = [
            [1, 1, 1, 1, 1, 1, 1],
            [99, 99, 99, 99, 99, 99, 99],
            [40, 1, 50, 40, 1, 43, 1],
            [1, 70, 1, 80, 99, 52, 1],
            [1, 70, 1, 80, 1, 52, 94],
            [70, 70, 70, 70, 70, 70, 70],
            [60, 60, 75, 73, 1, 44, 1],
        ];
        for c in cases {
            let p = player_with(c);
            let base_sum = c[1] as u32 + c[3] as u32 + (c[5] as u32 >> 1);
            let melee = c[0] as u32 + c[2] as u32;
            let range = (c[4] as u32 >> 1) + c[4] as u32;
            let magic = (c[6] as u32 >> 1) + c[6] as u32;
            assert_eq!(
                p.get_combat_level(),
                float_ref(base_sum, melee, range, magic),
                "real get_combat_level disagrees with float ref for {c:?}",
            );
        }
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use crate::interaction::InteractionTarget;
    use rs_pack::types::{LocAngle, LocLayer, LocShape};
    use rs_vm::trigger::ServerTriggerType;

    fn make_player() -> Player {
        let uid = PlayerUid::new("test".into(), 1);
        let vars = VarSet::new(std::iter::empty());
        Player::new(uid, CoordGrid::new(3222, 0, 3222), vars, false)
    }

    fn obj_target() -> InteractionTarget {
        InteractionTarget::Obj {
            coord: CoordGrid::new(3225, 0, 3222),
            id: 100,
            count: 1,
        }
    }

    fn loc_target() -> InteractionTarget {
        InteractionTarget::Loc {
            coord: CoordGrid::new(3225, 0, 3222),
            id: 200,
            width: 1,
            length: 1,
            shape: LocShape::CentrepieceStraight,
            angle: LocAngle::North,
            layer: LocLayer::Ground,
        }
    }

    // ---- set_interaction ----

    #[test]
    fn set_interaction_stores_target() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);
        assert!(p.interaction.target.is_some());
        assert_eq!(
            p.interaction.target_op,
            Some(ServerTriggerType::ApObj1 as u8)
        );
    }

    #[test]
    fn set_interaction_resets_ap_range() {
        let mut p = make_player();
        p.interaction.ap_range = Some(5);
        p.interaction.ap_range_called = true;
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);
        assert_eq!(p.interaction.ap_range, Some(10));
        assert!(!p.interaction.ap_range_called);
    }

    #[test]
    fn set_interaction_stores_obj_type_as_subject() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);
        assert_eq!(p.interaction.target_subject_type, Some(100)); // obj id
    }

    #[test]
    fn set_interaction_stores_loc_type_as_subject() {
        let mut p = make_player();
        p.set_interaction(loc_target(), ServerTriggerType::ApLoc1 as u8, true);
        assert_eq!(p.interaction.target_subject_type, Some(200)); // loc id
    }

    #[test]
    fn set_interaction_npc_has_no_subject_type() {
        let mut p = make_player();
        p.set_interaction(
            InteractionTarget::Npc { nid: 5 },
            ServerTriggerType::ApNpc1 as u8,
            true,
        );
        assert_eq!(p.interaction.target_subject_type, None);
    }

    #[test]
    fn set_interaction_player_has_no_subject_type() {
        let mut p = make_player();
        p.set_interaction(
            InteractionTarget::Player { pid: 2 },
            ServerTriggerType::ApPlayer1 as u8,
            true,
        );
        assert_eq!(p.interaction.target_subject_type, None);
    }

    #[test]
    fn set_interaction_clears_subject_com() {
        let mut p = make_player();
        p.interaction.target_subject_com = Some(42);
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);
        assert_eq!(p.interaction.target_subject_com, None);
    }

    // ---- clear_interaction ----

    #[test]
    fn clear_interaction_removes_target() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);
        p.clear_interaction();
        assert!(p.interaction.target.is_none());
    }

    #[test]
    fn clear_interaction_resets_all_fields() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj3 as u8, true);
        p.interaction.ap_range = Some(5);
        p.interaction.ap_range_called = true;
        p.clear_interaction();
        assert_eq!(p.interaction.target_op, None);
        assert_eq!(p.interaction.target_subject_type, None);
        assert_eq!(p.interaction.target_subject_com, None);
        assert_eq!(p.interaction.ap_range, Some(10));
        assert!(!p.interaction.ap_range_called);
    }

    // ---- has_interaction ----

    #[test]
    fn has_interaction_false_when_no_target() {
        let p = make_player();
        assert!(!p.has_interaction());
    }

    #[test]
    fn has_interaction_true_when_target_set() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);
        assert!(p.has_interaction());
    }

    #[test]
    fn has_interaction_false_after_clear() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);
        p.clear_interaction();
        assert!(!p.has_interaction());
    }

    // ---- can_access ----

    #[test]
    fn can_access_true_by_default() {
        let p = make_player();
        assert!(p.can_access());
    }

    #[test]
    fn can_access_false_when_protected() {
        let mut p = make_player();
        p.state.protect = true;
        assert!(!p.can_access());
    }

    #[test]
    fn can_access_false_when_delayed() {
        let mut p = make_player();
        p.state.delayed = true;
        assert!(!p.can_access());
    }

    #[test]
    fn can_access_false_when_modal_open() {
        let mut p = make_player();
        p.modal_state = MODAL_MAIN;
        assert!(!p.can_access());
    }

    // ---- next_target ----

    #[test]
    fn next_target_none_by_default() {
        let p = make_player();
        assert!(p.next_target.is_none());
    }

    #[test]
    fn next_target_take_clears_it() {
        let mut p = make_player();
        p.next_target = Some(obj_target());
        let taken = p.next_target.take();
        assert!(taken.is_some());
        assert!(p.next_target.is_none());
    }

    // ---- reset_pathing_entity ----

    #[test]
    fn reset_clears_ap_range_called() {
        let mut p = make_player();
        p.interaction.ap_range_called = true;
        p.reset_pathing_entity();
        assert!(!p.interaction.ap_range_called);
    }

    #[test]
    fn reset_clears_opcalled() {
        let mut p = make_player();
        p.opcalled = true;
        p.reset_pathing_entity();
        assert!(!p.opcalled);
    }

    #[test]
    fn reset_clears_steps_taken() {
        let mut p = make_player();
        p.pathing.steps_taken = 5;
        p.reset_pathing_entity();
        assert_eq!(p.pathing.steps_taken, 0);
    }

    #[test]
    fn reset_clears_walk_dir() {
        let mut p = make_player();
        p.pathing.walk_dir = 4;
        p.reset_pathing_entity();
        assert_eq!(p.pathing.walk_dir, -1);
    }

    #[test]
    fn reset_does_not_clear_interaction() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);
        p.reset_pathing_entity();
        assert!(p.interaction.target.is_some());
    }

    #[test]
    fn reset_does_not_clear_waypoints() {
        let mut p = make_player();
        p.pathing.queue_waypoint(3225, 3222);
        assert!(p.pathing.has_waypoints());
        p.reset_pathing_entity();
        assert!(p.pathing.has_waypoints());
    }

    // ---- waypoints ----

    #[test]
    fn no_waypoints_by_default() {
        let p = make_player();
        assert!(!p.pathing.has_waypoints());
    }

    #[test]
    fn queue_waypoint_sets_waypoints() {
        let mut p = make_player();
        p.pathing.queue_waypoint(3225, 3222);
        assert!(p.pathing.has_waypoints());
        assert_eq!(p.pathing.waypoint_index, 0);
    }

    #[test]
    fn clear_waypoints_removes_them() {
        let mut p = make_player();
        p.pathing.queue_waypoint(3225, 3222);
        p.clear_waypoints();
        assert!(!p.pathing.has_waypoints());
    }

    #[test]
    fn queue_waypoints_reverses_input() {
        let mut p = make_player();
        let wp = [
            CoordGrid::new(3223, 0, 3222).packed(),
            CoordGrid::new(3224, 0, 3222).packed(),
            CoordGrid::new(3225, 0, 3222).packed(),
        ];
        p.pathing.queue_waypoints(&wp);
        assert_eq!(p.pathing.waypoint_index, 2);
        // first walked = waypoints[2] = wp[0] (reversed)
        let first = CoordGrid::from(p.pathing.waypoints[2]);
        assert_eq!(first.x(), 3223);
    }

    // ---- trigger arithmetic ----

    #[test]
    fn ap_to_op_offset_is_7() {
        assert_eq!(
            ServerTriggerType::OpObj1 as u8 - ServerTriggerType::ApObj1 as u8,
            7
        );
        assert_eq!(
            ServerTriggerType::OpLoc1 as u8 - ServerTriggerType::ApLoc1 as u8,
            7
        );
        assert_eq!(
            ServerTriggerType::OpNpc1 as u8 - ServerTriggerType::ApNpc1 as u8,
            7
        );
        assert_eq!(
            ServerTriggerType::OpPlayer1 as u8 - ServerTriggerType::ApPlayer1 as u8,
            7
        );
    }

    #[test]
    fn ap_triggers_are_sequential() {
        assert_eq!(
            ServerTriggerType::ApObj2 as u8,
            ServerTriggerType::ApObj1 as u8 + 1
        );
        assert_eq!(
            ServerTriggerType::ApObj3 as u8,
            ServerTriggerType::ApObj1 as u8 + 2
        );
        assert_eq!(
            ServerTriggerType::ApObj4 as u8,
            ServerTriggerType::ApObj1 as u8 + 3
        );
        assert_eq!(
            ServerTriggerType::ApObj5 as u8,
            ServerTriggerType::ApObj1 as u8 + 4
        );
    }

    #[test]
    fn op_triggers_are_sequential() {
        assert_eq!(
            ServerTriggerType::OpLoc2 as u8,
            ServerTriggerType::OpLoc1 as u8 + 1
        );
        assert_eq!(
            ServerTriggerType::OpLoc3 as u8,
            ServerTriggerType::OpLoc1 as u8 + 2
        );
        assert_eq!(
            ServerTriggerType::OpLoc4 as u8,
            ServerTriggerType::OpLoc1 as u8 + 3
        );
        assert_eq!(
            ServerTriggerType::OpLoc5 as u8,
            ServerTriggerType::OpLoc1 as u8 + 4
        );
    }

    // ============================================================
    // Integration-style state machine tests
    // These simulate what process_interaction does tick-by-tick
    // by manipulating Player state directly.
    // ============================================================

    /// Simulates the cleanup phase at the end of process_interaction:
    /// if next_target is set, swap it in; else if interacted && !ap_range_called, clear.
    fn simulate_interaction_cleanup(p: &mut Player, interacted: bool) {
        if p.next_target.is_some() {
            p.interaction.target = p.next_target.take();
        } else if interacted && !p.interaction.ap_range_called {
            p.clear_interaction();
        }
    }

    // ---- Woodcutting pattern: p_oploc keeps interaction alive ----

    #[test]
    fn woodcutting_p_oploc_persists_interaction() {
        let mut p = make_player();

        // OpLoc handler sets interaction
        p.set_interaction(loc_target(), ServerTriggerType::ApLoc1 as u8, true);

        // OP fires, script calls p_oploc → sets next_target
        p.next_target = Some(loc_target());
        simulate_interaction_cleanup(&mut p, true);

        // Interaction persists via next_target
        assert!(p.interaction.target.is_some());
    }

    #[test]
    fn woodcutting_no_p_oploc_clears_interaction() {
        let mut p = make_player();
        p.set_interaction(loc_target(), ServerTriggerType::ApLoc1 as u8, true);

        // OP fires, script does NOT call p_oploc
        // next_target stays None
        simulate_interaction_cleanup(&mut p, true);

        // Interaction cleared
        assert!(p.interaction.target.is_none());
    }

    // ---- Firemaking pattern: world_delay after OP ----

    #[test]
    fn world_delay_no_p_opobj_clears_interaction() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj4 as u8, true);

        // OP fires, script calls world_delay (no p_opobj)
        // Script suspended to world queue, next_target = None
        simulate_interaction_cleanup(&mut p, true);

        // Interaction cleared - won't re-trigger OP next tick
        assert!(p.interaction.target.is_none());
    }

    #[test]
    fn firemaking_cycle_p_opobj_then_world_delay() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj4 as u8, true);

        // Tick 1-3: script calls p_opobj(4) to retry
        for _ in 0..3 {
            p.next_target = Some(obj_target()); // p_opobj sets this
            simulate_interaction_cleanup(&mut p, true);
            assert!(
                p.interaction.target.is_some(),
                "interaction should persist via p_opobj"
            );
            p.reset_pathing_entity(); // between ticks
        }

        // Tick 4: fire succeeds, script calls world_delay (no p_opobj)
        p.next_target = None;
        simulate_interaction_cleanup(&mut p, true);
        assert!(
            p.interaction.target.is_none(),
            "interaction should clear after world_delay"
        );
    }

    // ---- ap_range_called lifecycle ----

    #[test]
    fn ap_range_called_survives_within_tick() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);

        // AP trigger calls p_aprange
        p.interaction.ap_range = Some(10);
        p.interaction.ap_range_called = true;

        // OP fires, interacted = true, but ap_range_called prevents clear
        simulate_interaction_cleanup(&mut p, true);
        assert!(
            p.interaction.target.is_some(),
            "ap_range_called should prevent clear within tick"
        );
    }

    #[test]
    fn ap_range_called_reset_between_ticks() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);

        // Tick 1: AP sets ap_range_called, OP fires
        p.interaction.ap_range_called = true;
        simulate_interaction_cleanup(&mut p, true);
        assert!(p.interaction.target.is_some());

        // Between ticks: reset
        p.reset_pathing_entity();
        assert!(
            !p.interaction.ap_range_called,
            "reset should clear ap_range_called"
        );

        // Tick 2: no AP trigger (no ap_range_called set), OP fires again
        // If next_target is None, interaction should clear
        simulate_interaction_cleanup(&mut p, true);
        assert!(
            p.interaction.target.is_none(),
            "should clear after reset + no ap_range_called"
        );
    }

    // ---- active_script take vs clone (world suspend fix) ----
    // The fix: use .take() instead of .clone() when resuming active_script.
    // .take() moves ownership so there's only one copy.
    // .clone() leaves the original, causing repeated execution.
    // (Cannot unit test without Script construction - tested implicitly by engine.)

    // ---- Multi-tick interaction lifecycle ----

    #[test]
    fn full_obj_interaction_lifecycle() {
        let mut p = make_player();

        // Tick 0: Client clicks obj, handler sets interaction
        p.set_interaction(obj_target(), ServerTriggerType::ApObj3 as u8, true);
        p.opcalled = true;
        assert!(p.has_interaction());

        // Simulate process_in: opcalled reset at start of decode
        p.opcalled = false;

        // Tick 1: process_interaction - player walks, OP not yet in range
        p.pathing.steps_taken = 1; // walked
        simulate_interaction_cleanup(&mut p, false); // didn't interact
        assert!(
            p.interaction.target.is_some(),
            "target persists while walking"
        );

        // Between ticks
        p.reset_pathing_entity();

        // Tick 2: arrive, OP fires, script runs (pickup)
        p.pathing.steps_taken = 0;
        simulate_interaction_cleanup(&mut p, true); // interacted, no next_target
        assert!(
            p.interaction.target.is_none(),
            "interaction clears after successful OP"
        );
    }

    #[test]
    fn loc_interaction_with_p_oploc_chain() {
        let mut p = make_player();

        // Click door
        p.set_interaction(loc_target(), ServerTriggerType::ApLoc1 as u8, true);

        // Tick 1: OP fires, script calls p_oploc(1) to chain
        p.next_target = Some(loc_target());
        simulate_interaction_cleanup(&mut p, true);
        assert!(p.interaction.target.is_some(), "p_oploc chains interaction");

        p.reset_pathing_entity();

        // Tick 2: OP fires again, script finishes without p_oploc
        p.next_target = None;
        simulate_interaction_cleanup(&mut p, true);
        assert!(
            p.interaction.target.is_none(),
            "interaction clears when chain ends"
        );
    }

    // ---- "I can't reach that!" conditions ----

    #[test]
    fn cant_reach_requires_all_three_conditions() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);

        // All three must be true: !interacted && !has_waypoints && steps_taken == 0
        let interacted = false;
        let cant_reach = !interacted && !p.pathing.has_waypoints() && p.pathing.steps_taken == 0;
        assert!(cant_reach);
    }

    #[test]
    fn steps_taken_prevents_cant_reach() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);
        p.pathing.steps_taken = 1;

        let interacted = false;
        let cant_reach = !interacted && !p.pathing.has_waypoints() && p.pathing.steps_taken == 0;
        assert!(!cant_reach, "steps_taken > 0 prevents the message");
    }

    #[test]
    fn waypoints_prevent_cant_reach() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);
        p.pathing.queue_waypoint(3225, 3222);

        let interacted = false;
        let cant_reach = !interacted && !p.pathing.has_waypoints() && p.pathing.steps_taken == 0;
        assert!(!cant_reach, "having waypoints prevents the message");
    }

    #[test]
    fn interacted_prevents_cant_reach() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);

        let interacted = true;
        let cant_reach = !interacted && !p.pathing.has_waypoints() && p.pathing.steps_taken == 0;
        assert!(!cant_reach, "interaction success prevents the message");
    }

    // ---- p_stopaction during interaction ----

    #[test]
    fn p_stopaction_clears_interaction_and_waypoints() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj4 as u8, true);
        p.pathing.queue_waypoint(3225, 3222);

        // p_stopaction = clear_waypoints + clear_interaction
        p.clear_waypoints();
        p.clear_interaction();

        assert!(!p.pathing.has_waypoints());
        assert!(!p.has_interaction());
    }

    #[test]
    fn p_stopaction_then_p_opobj_replaces_interaction() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj4 as u8, true);

        // Script calls p_stopaction
        p.clear_waypoints();
        p.clear_interaction();

        // Then p_opobj(4) sets new interaction
        p.set_interaction(obj_target(), ServerTriggerType::ApObj4 as u8, true);

        assert!(p.has_interaction());
        assert_eq!(
            p.interaction.target_op,
            Some(ServerTriggerType::ApObj4 as u8)
        );
    }

    // ---- Interaction overwrite on new click ----

    #[test]
    fn new_click_replaces_existing_interaction() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);
        assert_eq!(p.interaction.target_subject_type, Some(100)); // obj id

        // Player clicks a different loc
        p.clear_interaction();
        p.set_interaction(loc_target(), ServerTriggerType::ApLoc2 as u8, true);
        assert_eq!(
            p.interaction.target_op,
            Some(ServerTriggerType::ApLoc2 as u8)
        );
        assert_eq!(p.interaction.target_subject_type, Some(200)); // loc id
    }

    // ---- Delayed player cannot interact ----

    #[test]
    fn delayed_player_skips_interaction() {
        let mut p = make_player();
        p.set_interaction(obj_target(), ServerTriggerType::ApObj1 as u8, true);
        p.state.delayed = true;

        // process_interaction checks can_access() which returns false
        assert!(!p.can_access());
        // Both try_interact calls are skipped
    }

    #[test]
    fn delay_expires_allows_interaction() {
        let mut p = make_player();
        p.state.delayed = true;
        assert!(!p.can_access());

        p.state.delayed = false;
        assert!(p.can_access());
    }
}

#[cfg(test)]
mod energy_tests {
    use super::*;

    fn make_player() -> Player {
        let uid = PlayerUid::new("test".into(), 1);
        let vars = VarSet::new(std::iter::empty());
        Player::new(uid, CoordGrid::new(3222, 0, 3222), vars, false)
    }

    // ---- delayed players are frozen ----

    #[test]
    fn delayed_player_energy_unchanged() {
        let mut p = make_player();
        p.runenergy = 5000;
        p.pathing.steps_taken = 5; // would deplete if not delayed
        p.state.delayed = true;
        assert!(!p.update_energy(), "delayed returns no sync signal");
        assert_eq!(p.runenergy, 5000);
    }

    // ---- recovery branch (steps_taken < 2) ----

    #[test]
    fn standing_recovers_from_agility() {
        let mut p = make_player();
        p.runenergy = 5000;
        p.pathing.steps_taken = 0;
        p.stats.base_levels[PlayerStat::Agility as usize] = 99;
        // recovered = 99/9 + 8 = 19 (rev 225); 99/6 + 8 = 24 (since 244, June rate)
        #[cfg(rev = "225")]
        let recovered = 99 / 9 + 8;
        #[cfg(since_244)]
        let recovered = 99 / 6 + 8;
        p.update_energy();
        assert_eq!(p.runenergy, 5000 + recovered);
    }

    #[test]
    fn single_step_still_recovers() {
        let mut p = make_player();
        p.runenergy = 5000;
        p.pathing.steps_taken = 1; // < 2, boundary still recovers
        p.stats.base_levels[PlayerStat::Agility as usize] = 9;
        // recovered = 9/9 + 8 = 1 + 8 = 9
        p.update_energy();
        assert_eq!(p.runenergy, 5009);
    }

    #[test]
    fn recovery_caps_at_max() {
        let mut p = make_player();
        p.runenergy = 9995;
        p.pathing.steps_taken = 0;
        p.stats.base_levels[PlayerStat::Agility as usize] = 99; // +19
        p.update_energy();
        assert_eq!(p.runenergy, 10000);
    }

    // ---- depletion branch (steps_taken >= 2) ----

    #[test]
    fn running_depletes_by_weight() {
        let mut p = make_player();
        p.runenergy = 5000;
        p.pathing.steps_taken = 2; // running boundary
        p.runweight = 32000; // 32 kg
        // loss = floor(67 + 67*32/64) = floor(100.5) = 100
        p.update_energy();
        assert_eq!(p.runenergy, 4900);
    }

    #[test]
    fn running_zero_weight_min_loss() {
        let mut p = make_player();
        p.runenergy = 5000;
        p.pathing.steps_taken = 2;
        p.runweight = 0;
        // loss = floor(67 + 0) = 67
        p.update_energy();
        assert_eq!(p.runenergy, 4933);
    }

    #[test]
    fn negative_weight_clamps_to_min_loss() {
        let mut p = make_player();
        p.runenergy = 5000;
        p.pathing.steps_taken = 2;
        p.runweight = -5000; // weightKg clamps up to 0
        p.update_energy();
        assert_eq!(p.runenergy, 4933); // loss = 67
    }

    #[test]
    fn weight_clamps_at_64kg() {
        let mut p = make_player();
        p.runenergy = 5000;
        p.pathing.steps_taken = 2;
        p.runweight = 100000; // 100 kg, clamps down to 64
        // loss = floor(67 + 67*64/64) = 134
        p.update_energy();
        assert_eq!(p.runenergy, 4866);
    }

    #[test]
    fn depletion_floors_at_zero() {
        let mut p = make_player();
        p.runenergy = 50;
        p.pathing.steps_taken = 2;
        p.runweight = 0; // loss 67 > 50
        p.update_energy();
        assert_eq!(p.runenergy, 0);
    }

    // ---- run / temprun side effects ----

    #[test]
    fn zero_energy_disables_run_and_temprun() {
        let mut p = make_player();
        p.runenergy = 50;
        p.run = true;
        p.temprun = true;
        p.pathing.steps_taken = 2;
        p.runweight = 0; // depletes 50 -> 0
        assert!(p.update_energy(), "on->off transition signals a varp sync");
        assert_eq!(p.runenergy, 0);
        assert!(!p.run, "run forced off at 0 energy");
        assert!(!p.temprun, "temprun cleared below 100");
    }

    #[test]
    fn zero_energy_with_run_already_off_does_not_signal() {
        let mut p = make_player();
        p.runenergy = 50;
        p.run = false; // already walking
        p.pathing.steps_taken = 2;
        p.runweight = 0; // depletes 50 -> 0
        assert!(
            !p.update_energy(),
            "no transition means no redundant varp sync"
        );
        assert_eq!(p.runenergy, 0);
        assert!(!p.run);
    }

    #[test]
    fn low_energy_clears_temprun_but_keeps_run() {
        let mut p = make_player();
        p.runenergy = 150;
        p.run = true;
        p.temprun = true;
        p.pathing.steps_taken = 2;
        p.runweight = 0; // 150 - 67 = 83, below 100 but above 0
        assert!(!p.update_energy(), "run still on -> no sync signal");
        assert_eq!(p.runenergy, 83);
        assert!(p.run, "run stays enabled while energy > 0");
        assert!(!p.temprun, "temprun cleared below 100");
    }

    #[test]
    fn healthy_energy_preserves_flags() {
        let mut p = make_player();
        p.runenergy = 5000;
        p.run = true;
        p.temprun = true;
        p.pathing.steps_taken = 2;
        p.runweight = 0; // 5000 - 67 = 4933, well above 100
        assert!(!p.update_energy());
        assert!(p.run);
        assert!(p.temprun);
    }
}

#[cfg(test)]
mod staff_mod_level_tests {
    use super::*;

    #[test]
    fn from_u8_round_trips_every_level() {
        // Persistence stores `level as u8` and restores it via from_u8; that
        // round-trip must be exact for all defined levels.
        for level in [
            StaffModLevel::Normal,
            StaffModLevel::PlayerModerator,
            StaffModLevel::JagexModerator,
            StaffModLevel::Developer,
        ] {
            assert_eq!(StaffModLevel::from_u8(level as u8), level);
        }
    }

    #[test]
    fn from_u8_unknown_falls_back_to_normal() {
        assert_eq!(StaffModLevel::from_u8(4), StaffModLevel::Normal);
        assert_eq!(StaffModLevel::from_u8(255), StaffModLevel::Normal);
    }
}
