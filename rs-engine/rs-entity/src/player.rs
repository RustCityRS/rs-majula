use crate::build::{BuildArea, STAT_COUNT};
use crate::direction::Direction;
use crate::interaction::InteractionState;
use crate::interaction::InteractionTarget;
use crate::pathing::{MoveStrategy, PathingEntity};
use crate::state::EntityState;
use rs_cam::CamQueue;
use rs_grid::CoordGrid;
use rs_hero::HeroPoints;
#[cfg(since_647)]
use rs_info::PlayerInfoProt;
use rs_info::{EntityMasks, FocusKind};
use rs_inv::Inventory;
#[cfg(since_647)]
use rs_pack::cache::area::{NO_AREAS, TileAreas};
use rs_pack::types::{BlockWalk, MoveRestrict, PlayerStat};
use rs_stat::Stats;
use rs_var::VarSet;
pub use rs_vm::PlayerUid;
use rs_vm::state::ExecutionState;
use rustc_hash::{FxHashMap, FxHashSet};
use std::ops::{Deref, DerefMut};

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
    /// Maintainer with unrestricted access.
    Maintainer = 3,
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
            3 => Self::Maintainer,
            _ => Self::Normal,
        }
    }

    /// Whether this level is kept out of the economy and PvP where the world
    /// isolates staff (see `Engine::staff_isolation`): jagex moderators and up
    /// don't trade, duel, fight or use items or spells on other players, nor
    /// they on them, and nothing they drop is ever shown to anyone else.
    pub const fn is_isolated(self) -> bool {
        self as u8 >= Self::JagexModerator as u8
    }
}

/// Rev-647 chat modes, as CHAT_SETMODE carries them (CS2 op 5006): typed chat
/// goes to public chat, or to the clan chat the player is in.
#[cfg(since_647)]
pub const CHAT_MODE_PUBLIC: u8 = 0;
#[cfg(since_647)]
pub const CHAT_MODE_CLAN: u8 = 1;

/// Rev-647 move types, as the MoveType and TempMoveType blocks carry them
/// (`Class244.method1590`).
#[cfg(since_647)]
pub const MOVE_TYPE_WALK: u8 = 1;
#[cfg(since_647)]
pub const MOVE_TYPE_RUN: u8 = 2;
/// Place the player on the new tile instead of moving it there
/// (`PlayerEntity.method4080`).
#[cfg(since_647)]
pub const MOVE_TYPE_TELEPORT: u8 = 127;

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
    pub stats: Stats<STAT_COUNT>,
    pub combat_level: u8,
    #[cfg(since_274)]
    pub skill_level: u16,
    pub vars: VarSet,
    pub state: EntityState,
    pub interaction: InteractionState,
    pub cam_queue: CamQueue,
    pub staff_mod_level: StaffModLevel,
    pub is_member: bool,
    pub run_step: Option<Direction>,
    pub run: bool,
    pub temprun: bool,
    /// Rev-647: the move type the client has stored for this player, as last
    /// sent in a MoveType block; `None` until one has gone out this session.
    #[cfg(since_647)]
    pub client_move_type: Option<u8>,
    /// Rev-647: the tile of the map the player last left for region space,
    /// where it goes back to from a region with no exit coord.
    #[cfg(since_647)]
    pub region_entry: Option<CoordGrid>,
    pub move_request: bool,
    pub modal_state: u8,
    pub modal_main: Option<u16>,
    pub last_modal_main: Option<u16>,
    pub modal_chat: Option<u16>,
    pub last_modal_chat: Option<u16>,
    /// Rev-647: the parent component hash the chat modal was actually mounted
    /// on, so `if_close` can close it in the slot it opened in.
    ///
    /// Dialogues are sub-interfaces at 647, not a screen-wide chat modal, and
    /// `IF_CLOSESUB` closes by parent hash. Most mount on the default
    /// `dialogue_box` child; objbox and doubleobjbox use the small slot. Closing
    /// the default when the small one is open leaves the dialogue on screen.
    #[cfg(since_647)]
    pub modal_chat_parent: i32,
    /// Rev-647: the component hash the main-screen modal is mounted on, 0
    /// when none is.
    #[cfg(since_647)]
    pub modal_main_parent: i32,
    /// Rev-647: the component hash the side modal is mounted on, 0 when none
    /// is.
    #[cfg(since_647)]
    pub modal_side_parent: i32,
    /// Rev-647: every sub-interface the client holds, by the component hash
    /// it is mounted on, with IF_OPENSUB's type byte. Kept by the same rules
    /// as the client's own table, so the server can tell what is on screen.
    #[cfg(since_647)]
    pub open_subs: FxHashMap<i32, (u16, u8)>,
    pub modal_side: Option<u16>,
    pub last_modal_side: Option<u16>,
    pub modal_tutorial: Option<u16>,
    pub refresh_modal: bool,
    pub refresh_modal_close: bool,
    pub request_modal_close: bool,
    pub public: ChatSettingsPublic,
    pub private: ChatSettingsPrivate,
    pub trade: ChatSettingsTradeDuel,
    pub logout_requested: bool,
    pub logout_idle_requested: bool,
    pub logout_prevented_until: Option<u32>,
    pub logout_prevented_message: Option<Box<str>>,
    pub logout_sent: bool,
    pub disconnected_at: Option<u32>,
    pub last_response: u32,
    pub last_active: u32,
    pub active: bool,
    pub move_clicked: bool,
    pub block_walk: BlockWalk,
    pub invs: FxHashMap<u16, Inventory>,
    /// Rev-647: true until the first REBUILD_NORMAL is sent — the login
    /// rebuild carries the GPI init bit-stream.
    #[cfg(since_647)]
    pub pending_login: bool,
    /// Rev-647 public chat staged by the MESSAGE_PUBLIC handler for the
    /// broadcast phase: `(effects, rights, char_count, huffman_data)`.
    #[cfg(since_647)]
    pub pending_public_chat: Option<(u16, u8, usize, Vec<u8>)>,
    /// Rev-647: where the player's typed chat goes, as the client last set it
    /// with CHAT_SETMODE -- [`CHAT_MODE_PUBLIC`] or [`CHAT_MODE_CLAN`].
    #[cfg(since_647)]
    pub chat_mode: u8,
    /// Rev-647 root interface currently open (set by IF_OPENTOP).
    #[cfg(since_647)]
    pub toplevel: u16,
    /// Rev-647 client window mode from WINDOW_STATUS: 1 = fixed, 2 = resizable,
    /// 3 = fullscreen. Content only tests `= 1`, which is what decides whether
    /// the gameframe mounts under `toplevel` or `toplevel_fullscreen`.
    #[cfg(since_647)]
    pub window_mode: u8,
    /// Rev-647 client viewport width in pixels, from WINDOW_STATUS.
    #[cfg(since_647)]
    pub window_width: u16,
    /// Rev-647 client viewport height in pixels, from WINDOW_STATUS.
    #[cfg(since_647)]
    pub window_height: u16,
    /// Rev-647: server-side mirror of transmitted client variables (varcs);
    /// `applydesign` reads the character-design preview varcs back from it.
    #[cfg(since_647)]
    pub varcs: FxHashMap<u16, i32>,
    #[cfg(since_647)]
    pub varc_strings: FxHashMap<u16, Box<str>>,
    /// Rev-647: display name claimed via `p_setdisplayname`; falls back to the
    /// login username when unset.
    #[cfg(since_647)]
    pub display_name: Option<String>,
    /// Rev-647: available display-name suggestions from the DB (name taken).
    #[cfg(since_647)]
    pub name_suggestions: Vec<String>,
    /// Rev-647: current page into `name_suggestions` (3 names per page).
    #[cfg(since_647)]
    pub name_suggestion_page: usize,
    pub inv_transmits: FxHashMap<u16, Vec<rs_vm::ComId>>,
    pub inv_other_transmits: FxHashMap<rs_vm::ComId, (i32, u16)>,
    pub inv_first_seen: FxHashSet<u16>,
    /// From 647: the other players' invs (`invother_transmit`) the client
    /// has whole, by inv id. The client keeps its copy of another player's
    /// inv apart from the player's own (trade's other offer is inv 90 too).
    #[cfg(since_647)]
    pub inv_other_first_seen: FxHashSet<u16>,
    pub build_area: BuildArea,
    pub last_zone: CoordGrid,
    pub last_map_zone: CoordGrid,
    #[cfg(since_647)]
    pub areas: TileAreas,
    pub afk_event_ready: bool,
    pub afk_zones: [u32; 2],
    pub last_afk_zone: u16,
    pub opcalled: bool,
    pub next_target: Option<InteractionTarget>,
    pub walktrigger: Option<i32>,
    pub bot: bool,
    cold: Box<PlayerCold>,
}

pub struct PlayerCold {
    pub last_date: i64,
    pub last_login_date: i64,
    pub hero_points: HeroPoints,
    pub allow_design: bool,
    #[cfg(before_647)]
    pub last_com: Option<u16>,
    /// Rev-647: interface components are addressed by the full 32-bit
    /// component hash (`iface << 16 | com`); -1 = none.
    #[cfg(since_647)]
    pub last_com: i32,
    pub last_comsubid: Option<u16>,
    #[cfg(before_647)]
    pub last_slot: Option<u16>,
    #[cfg(since_647)]
    pub last_slot: i32,
    pub last_target_slot: Option<u16>,
    /// The component a 647 drag ended on (`iface << 16 | com`): the
    /// drag packet carries both ends, and a drop onto another component
    /// (a bank tab's button) is told apart by it.
    #[cfg(since_647)]
    pub last_target_com: Option<i32>,
    pub last_use_slot: Option<u16>,
    #[cfg(before_647)]
    pub last_item: Option<u16>,
    #[cfg(since_647)]
    pub last_item: i32,
    pub last_use_item: Option<u16>,
    /// The used item's component: its id before 647, its interface hash
    /// from 647 (`rs_vm::ComId`).
    pub last_use_com: Option<rs_vm::ComId>,
    pub resume_buttons: Option<Vec<i32>>,
    pub tabs: [Option<u16>; 14],
    pub gender: u8,
    pub headicons: u8,
    #[cfg(since_254)]
    pub transmog: Option<u16>,
    pub body: [i32; 7],
    pub colours: [u8; 5],
    pub bot_home: CoordGrid,
}

impl Deref for Player {
    type Target = PlayerCold;
    fn deref(&self) -> &PlayerCold {
        &self.cold
    }
}
impl DerefMut for Player {
    fn deref_mut(&mut self) -> &mut PlayerCold {
        &mut self.cold
    }
}

impl Player {
    /// Creates a new player at the given coordinate with the specified identity and variables.
    ///
    /// The player starts with teleport enabled (so the client receives the initial
    /// position), default walk speed, maintainer staff level, member status, and all
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
            stats: Stats::new(1),
            combat_level: 3,
            #[cfg(since_274)]
            skill_level: 0,
            vars,
            state: EntityState::new(),
            info: EntityMasks::new(),
            interaction: InteractionState::new(),
            cam_queue: CamQueue::new(),
            #[cfg(debug_assertions)]
            staff_mod_level: StaffModLevel::Maintainer,
            #[cfg(not(debug_assertions))]
            staff_mod_level: StaffModLevel::Normal,
            is_member: true,
            run_step: None,
            run: false,
            temprun: false,
            #[cfg(since_647)]
            client_move_type: None,
            #[cfg(since_647)]
            region_entry: None,
            move_request: false,
            modal_state: MODAL_NONE,
            modal_main: None,
            last_modal_main: None,
            modal_chat: None,
            last_modal_chat: None,
            #[cfg(since_647)]
            modal_chat_parent: 0,
            #[cfg(since_647)]
            modal_main_parent: 0,
            #[cfg(since_647)]
            modal_side_parent: 0,
            #[cfg(since_647)]
            open_subs: FxHashMap::default(),
            modal_side: None,
            last_modal_side: None,
            modal_tutorial: None,
            refresh_modal: false,
            refresh_modal_close: false,
            request_modal_close: false,
            public: ChatSettingsPublic::On,
            private: ChatSettingsPrivate::On,
            trade: ChatSettingsTradeDuel::On,
            logout_requested: false,
            logout_idle_requested: false,
            #[cfg(since_647)]
            pending_login: true,
            #[cfg(since_647)]
            pending_public_chat: None,
            #[cfg(since_647)]
            chat_mode: CHAT_MODE_PUBLIC,
            #[cfg(since_647)]
            toplevel: 0,
            #[cfg(since_647)]
            window_mode: 1,
            #[cfg(since_647)]
            window_width: 765,
            #[cfg(since_647)]
            window_height: 503,
            #[cfg(since_647)]
            varcs: FxHashMap::default(),
            #[cfg(since_647)]
            varc_strings: FxHashMap::default(),
            #[cfg(since_647)]
            display_name: None,
            #[cfg(since_647)]
            name_suggestions: Vec::new(),
            #[cfg(since_647)]
            name_suggestion_page: 0,
            logout_prevented_until: None,
            logout_prevented_message: None,
            logout_sent: false,
            disconnected_at: None,
            last_response: 0,
            last_active: 0,
            active: true,
            move_clicked: false,
            block_walk: BlockWalk::Npc,
            invs: FxHashMap::default(),
            inv_transmits: FxHashMap::default(),
            inv_other_transmits: FxHashMap::default(),
            inv_first_seen: FxHashSet::default(),
            #[cfg(since_647)]
            inv_other_first_seen: FxHashSet::default(),
            build_area: BuildArea::new(),
            last_zone: CoordGrid::new(0, 0, 0),
            last_map_zone: CoordGrid::new(0, 0, 0),
            #[cfg(since_647)]
            areas: NO_AREAS,
            afk_event_ready: false,
            afk_zones: [0; 2],
            last_afk_zone: 0,
            opcalled: false,
            next_target: None,
            walktrigger: None,
            bot,
            cold: Box::new(PlayerCold {
                last_date: 0,
                last_login_date: 0,
                hero_points: HeroPoints::new(),
                allow_design: false,
                #[cfg(before_647)]
                last_com: None,
                #[cfg(since_647)]
                last_com: -1,
                last_comsubid: None,
                #[cfg(before_647)]
                last_slot: None,
                #[cfg(since_647)]
                last_slot: -1,
                last_target_slot: None,
                #[cfg(since_647)]
                last_target_com: None,
                last_use_slot: None,
                #[cfg(before_647)]
                last_item: None,
                #[cfg(since_647)]
                last_item: -1,
                last_use_item: None,
                last_use_com: None,
                resume_buttons: None,
                tabs: [None; 14],
                gender: 0,
                headicons: 0,
                #[cfg(since_254)]
                transmog: None,
                body: [0, 10, 18, 26, 33, 36, 42],
                colours: [0; 5],
                bot_home: coord,
            }),
        }
    }

    /// Returns `true` if a blocking modal interface (main or chat) is currently open.
    ///
    /// Modal main and chat interfaces block player actions. Side and tutorial modals
    /// do not block.
    pub const fn contains_modal_interface(&self) -> bool {
        (self.modal_state & (MODAL_MAIN | MODAL_CHAT)) != MODAL_NONE
    }

    /// Rev-647: records IF_OPENSUB. As the client does, whatever was mounted
    /// on `parent` closes first.
    #[cfg(since_647)]
    pub fn sub_opened(&mut self, parent: i32, sub: u16, kind: u8) {
        self.sub_closed(parent);
        self.open_subs.insert(parent, (sub, kind));
    }

    /// Rev-647: records IF_CLOSESUB. The client closes the sub mounted on
    /// `parent`, then, recursively, every type-3 sub mounted inside it
    /// (`LightDetailClientOption.method2499`); others stay in its table,
    /// unreachable until something re-parents them.
    #[cfg(since_647)]
    pub fn sub_closed(&mut self, parent: i32) {
        let Some((closed, _)) = self.open_subs.remove(&parent) else {
            return;
        };
        let children: Vec<i32> = self
            .open_subs
            .iter()
            .filter(|(p, (_, kind))| *kind == 3 && (**p as u32 >> 16) as u16 == closed)
            .map(|(&p, _)| p)
            .collect();
        for child in children {
            self.sub_closed(child);
        }
    }

    /// Rev-647: the IF_OPENSUB type for a sub mounted on `parent`: that of
    /// the sub holding `parent`'s interface, or 1 on the toplevel. A main or
    /// chat modal mounts as type 0 at the end of the tick, after the script.
    #[cfg(since_647)]
    pub fn sub_kind(&self, parent: i32) -> u8 {
        let iface = (parent as u32 >> 16) as u16;
        if self.modal_main == Some(iface) || self.modal_chat == Some(iface) {
            return 0;
        }
        self.open_subs
            .values()
            .find(|&&(sub, _)| sub == iface)
            .map_or(1, |&(_, kind)| kind)
    }

    /// Rev-647: records IF_MOVESUB -- the sub on `from` moves to `to`,
    /// closing whatever `to` held, as the client's branch for it does.
    #[cfg(since_647)]
    pub fn sub_moved(&mut self, from: i32, to: i32) {
        let moving = self.open_subs.remove(&from);
        self.sub_closed(to);
        if let Some(sub) = moving {
            self.open_subs.insert(to, sub);
        }
    }

    /// Rev-647: whether interface `iface` is on screen -- the toplevel, or a
    /// sub mounted on a component of something that is. A sub left on a
    /// parent that is no longer reachable (after IF_OPENTOP swaps the root)
    /// is in the client's table but drawn nowhere.
    #[cfg(since_647)]
    pub fn is_interface_open(&self, iface: u16) -> bool {
        let mut current = iface;
        // Each step climbs one parent; a mount chain is a handful deep, and
        // the bound stops a cycle a bad move could make.
        for _ in 0..16 {
            if current == self.toplevel {
                return true;
            }
            let Some(parent) = self
                .open_subs
                .iter()
                .find(|(_, (sub, _))| *sub == current)
                .map(|(&p, _)| (p as u32 >> 16) as u16)
            else {
                return false;
            };
            current = parent;
        }
        false
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

    /// The name the player is drawn with (the appearance block) and named by
    /// (`displayname`): the display name they claimed, else their login name
    /// as 289's `displayname` gives it ("Test Player"). 2011's client finds a
    /// player by it (a trade request's "Accept trade").
    #[cfg(since_647)]
    pub fn shown_name(&self) -> String {
        self.display_name
            .clone()
            .unwrap_or_else(|| self.uid.screen_name())
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
    pub fn clear_interface_listeners<F>(
        &mut self,
        root: Option<u16>,
        get_root_layer: F,
    ) -> Vec<rs_vm::ComId>
    where
        F: Fn(rs_vm::ComId) -> Option<i32>,
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
    pub fn clear_inv_transmits(&mut self, com: rs_vm::ComId) {
        for coms in self.inv_transmits.values_mut() {
            coms.retain(|&c| c != com);
        }
        self.inv_other_transmits.remove(&com);
        // Before 647 a component is seen afresh once it stops; from 647 the
        // client keys inventories by inv id, and so does first-sight.
        #[cfg(before_647)]
        self.inv_first_seen.remove(&com);
        // From 647 an inventory no component transmits any more is dropped:
        // it stops going to the client (a closed shop's restocks), and the
        // next transmit sends it whole again.
        #[cfg(since_647)]
        {
            let first_seen = &mut self.inv_first_seen;
            self.inv_transmits.retain(|inv, coms| {
                if coms.is_empty() {
                    first_seen.remove(inv);
                }
                !coms.is_empty()
            });
            // Another player's inv no component shows any more goes whole
            // next time, so a new trade starts from its partner's offer.
            let others = &self.inv_other_transmits;
            self.inv_other_first_seen
                .retain(|inv| others.values().any(|&(_, other)| other == *inv));
        }
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

    /// Clears all queued waypoints, stopping movement.
    ///
    /// # Side Effects
    /// * Calls `self.pathing.clear_waypoints()`.
    pub fn clear_waypoints(&mut self) {
        self.pathing.clear_waypoints();
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

    /// Returns `true` once the player has stayed inside their AFK zones for 1000
    /// ticks. Aggressive NPCs with `check_afk` stop hunting such players.
    ///
    /// # Call Stack
    /// **Called by:** `Engine::check_afk`, `Engine::npc_hunt_players`
    #[inline(always)]
    pub fn zones_afk(&self) -> bool {
        self.last_afk_zone == 1000
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
    /// Always resets: info masks, walk step, jump flag, teleport flag, move_clicked, walk/run
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
        if self.move_clicked {
            self.move_clicked = false;
        }
        if self.state.protect {
            self.state.protect = false;
        }
        if self.opcalled {
            self.opcalled = false;
        }
        if self.interaction.ap_range_called {
            self.interaction.ap_range_called = false;
        }
        let move_speed = if self.run {
            MoveSpeed::Run
        } else {
            MoveSpeed::Walk
        };
        if self.pathing.move_speed != move_speed {
            self.pathing.move_speed = move_speed;
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

    /// Rev-647: sets `temprun` for a walk the player just clicked.
    ///
    /// The effective speed is `run XOR temprun` (`Engine::process_movement`),
    /// so ctrl flips the run orb for that one walk. The client sets the flag
    /// only while ctrl is held (`ProgressBarElement.method1322`, key 82). A
    /// plain click follows the orb. Running needs energy; walking does not.
    ///
    /// # Call Stack
    /// **Called by:** `move_click::handle`
    #[cfg(since_647)]
    pub fn click_temprun(&mut self, ctrl: bool) {
        self.temprun = ctrl && (self.run || self.runenergy >= 100);
    }

    /// Rev-647: settles this tick's MoveType and TempMoveType blocks.
    ///
    /// The client stores one move type per player (MoveType, 0x8) and plays
    /// each step at it, unless the same update carries a one-tick override
    /// (TempMoveType, 0x800). `Class244.method1590` reads both masks before it
    /// applies the step, so the override wins and a MoveType sent this tick
    /// already counts.
    ///
    /// So the stored type has to follow `run` however it changes -- the run
    /// orb, `p_run`, or energy running out mid-run -- and any step at another
    /// speed needs the override: the odd last tile of a run is a walk, a
    /// ctrl-click run with run off is a run, and a teleport is 127. Without it
    /// the client walks a teleported player to the new tile, and plays a run
    /// as a walk to a tile two away.
    ///
    /// # Side Effects
    /// * May set `info.move_type`/`info.temp_move_type` and their mask bits.
    /// * Updates `client_move_type`.
    ///
    /// # Call Stack
    /// **Called by:** `Engine::compute_player_info`, once every source of
    /// movement and run changes has had its turn for the tick.
    #[cfg(since_647)]
    pub fn sync_move_types(&mut self) {
        let stored = if self.run {
            MOVE_TYPE_RUN
        } else {
            MOVE_TYPE_WALK
        };
        if self.client_move_type != Some(stored) {
            self.client_move_type = Some(stored);
            self.info.move_type = Some(stored);
            self.info.masks |= PlayerInfoProt::MoveType as u16;
        }
        let step = if self.pathing.tele {
            // 289's `p_teleport` is not a jump: its client walks a move of up
            // to two tiles (`move(x, z, jump)` queues it as a step), which is
            // how a door's walk-through looks. 647's client plays any move not
            // marked 127 as a step (`Class244.method1590`), so only a jump or
            // a longer move is placed.
            let (dx, dz) = (
                self.pathing.coord.x() as i32 - self.pathing.last_coord.x() as i32,
                self.pathing.coord.z() as i32 - self.pathing.last_coord.z() as i32,
            );
            if !self.pathing.jump && dx.abs() <= 2 && dz.abs() <= 2 {
                MOVE_TYPE_WALK
            } else {
                MOVE_TYPE_TELEPORT
            }
        } else if self.pathing.run_dir != -1 {
            MOVE_TYPE_RUN
        } else if self.pathing.walk_dir != -1 {
            MOVE_TYPE_WALK
        } else {
            return;
        };
        if step != stored {
            self.info.temp_move_type = Some(step);
            self.info.masks |= PlayerInfoProt::TempMoveType as u16;
        }
    }
}

#[cfg(all(test, since_647))]
mod move_type_tests {
    use super::*;
    use rs_info::PlayerInfoProt;
    use std::iter;

    fn make_player() -> Player {
        let uid = PlayerUid::new("test".into(), 1);
        let vars = VarSet::new(iter::empty());
        Player::new(uid, CoordGrid::new(3222, 0, 3222), vars, false)
    }

    /// One tick: clear last tick's blocks, take the given step, settle.
    fn tick(p: &mut Player, walk_dir: i8, run_dir: i8, tele: bool) {
        p.info.masks = 0;
        p.info.move_type = None;
        p.info.temp_move_type = None;
        p.pathing.walk_dir = walk_dir;
        p.pathing.run_dir = run_dir;
        p.pathing.tele = tele;
        p.sync_move_types();
    }

    fn stored(p: &Player) -> Option<u8> {
        (p.info.masks & (PlayerInfoProt::MoveType as u16) != 0).then(|| p.info.move_type.unwrap())
    }

    fn temp(p: &Player) -> Option<u8> {
        (p.info.masks & (PlayerInfoProt::TempMoveType as u16) != 0)
            .then(|| p.info.temp_move_type.unwrap())
    }

    #[test]
    fn the_first_update_of_a_session_sends_the_stored_type() {
        let mut p = make_player();
        tick(&mut p, -1, -1, false);
        assert_eq!(stored(&p), Some(MOVE_TYPE_WALK));
        tick(&mut p, -1, -1, false);
        assert_eq!(stored(&p), None, "unchanged, so not resent");
    }

    #[test]
    fn toggling_run_resends_the_stored_type() {
        let mut p = make_player();
        tick(&mut p, -1, -1, false);
        p.run = true;
        tick(&mut p, 1, 1, false);
        assert_eq!(stored(&p), Some(MOVE_TYPE_RUN));
        assert_eq!(temp(&p), None, "a run step at run mode needs no override");
        p.run = false;
        tick(&mut p, 1, -1, false);
        assert_eq!(stored(&p), Some(MOVE_TYPE_WALK));
        assert_eq!(temp(&p), None);
    }

    /// Energy runs out during a run step: `update_energy` turns run off in
    /// the same tick, so the client is told "walk" and has to be told this
    /// one step was still a run.
    #[test]
    fn a_run_step_on_the_tick_energy_runs_out_is_still_a_run() {
        let mut p = make_player();
        p.run = true;
        tick(&mut p, -1, -1, false);
        p.runenergy = 50;
        p.pathing.steps_taken = 2;
        assert!(p.update_energy());
        tick(&mut p, 1, 1, false);
        assert_eq!(stored(&p), Some(MOVE_TYPE_WALK));
        assert_eq!(temp(&p), Some(MOVE_TYPE_RUN));
    }

    #[test]
    fn the_odd_last_tile_of_a_run_is_a_walk() {
        let mut p = make_player();
        p.run = true;
        tick(&mut p, 1, -1, false);
        assert_eq!(temp(&p), Some(MOVE_TYPE_WALK));
    }

    #[test]
    fn a_ctrl_click_run_with_run_off_is_a_run() {
        let mut p = make_player();
        tick(&mut p, 1, 1, false);
        assert_eq!(temp(&p), Some(MOVE_TYPE_RUN));
    }

    #[test]
    fn a_teleport_is_placed_not_walked() {
        for run in [false, true] {
            let mut p = make_player();
            p.run = run;
            p.pathing.coord = CoordGrid::new(3232, 0, 3222);
            tick(&mut p, -1, -1, true);
            assert_eq!(temp(&p), Some(MOVE_TYPE_TELEPORT), "run={run}");
        }
    }

    /// 289's door walk-through: `p_teleport` one tile, not a jump. 289's
    /// client walks it; 647's walks it unless told 127.
    #[test]
    fn a_short_teleport_that_is_not_a_jump_is_walked() {
        for run in [false, true] {
            let mut p = make_player();
            p.run = run;
            // A new player's first placement is a jump; a door's step is not.
            p.pathing.jump = false;
            p.pathing.coord = CoordGrid::new(3223, 0, 3222);
            tick(&mut p, -1, -1, true);
            let want = if run { Some(MOVE_TYPE_WALK) } else { None };
            assert_eq!(temp(&p), want, "run={run}: a walk step");
        }
        let mut p = make_player();
        p.pathing.coord = CoordGrid::new(3223, 0, 3222);
        p.pathing.jump = true;
        tick(&mut p, -1, -1, true);
        assert_eq!(temp(&p), Some(MOVE_TYPE_TELEPORT), "a jump is placed");
    }

    /// The bug the T3 RunTest caught: every plain click ran with run off,
    /// and walked with run on, because a plain click set temprun.
    #[test]
    fn a_plain_click_follows_the_run_orb() {
        let mut p = make_player();
        for run in [false, true] {
            p.run = run;
            p.click_temprun(false);
            assert!(!p.temprun, "run={run}");
        }
    }

    #[test]
    fn ctrl_flips_the_orb_for_one_walk() {
        let mut p = make_player();
        p.click_temprun(true);
        assert!(p.temprun, "run off: ctrl runs");
        p.run = true;
        p.click_temprun(true);
        assert!(p.temprun, "run on: ctrl walks");
    }

    #[test]
    fn ctrl_cannot_run_without_energy_but_can_walk() {
        let mut p = make_player();
        p.runenergy = 99;
        p.click_temprun(true);
        assert!(!p.temprun, "no energy: ctrl does not run");
        p.run = true;
        p.click_temprun(true);
        assert!(p.temprun, "ctrl-walking needs no energy");
    }

    #[test]
    fn standing_still_sends_no_override() {
        let mut p = make_player();
        p.run = true;
        tick(&mut p, -1, -1, false);
        assert_eq!(temp(&p), None);
    }
}

#[cfg(test)]
mod combat_level_tests {
    use super::*;
    use std::iter;

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
        let varps = VarSet::new(iter::empty());
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
mod afk_zone_tests {
    use super::*;
    use std::iter;

    /// `zones_afk` turns on after 1000 ticks inside the AFK zones and resets
    /// once the player leaves them, like 2004scape's `zonesAfk()`.
    #[test]
    fn zones_afk_after_1000_ticks_in_place() {
        let uid = PlayerUid::new("test".into(), 1);
        let varps = VarSet::new(iter::empty());
        let mut player = Player::new(uid, CoordGrid::new(3222, 0, 3222), varps, false);

        // First tick records the zone and resets the counter.
        player.update_afk_zones();
        assert_eq!(player.last_afk_zone, 0);

        for _ in 0..999 {
            player.update_afk_zones();
        }
        assert!(!player.zones_afk());
        player.update_afk_zones();
        assert!(player.zones_afk());

        // Walking out of both zones resets it.
        player.pathing.coord = CoordGrid::new(3300, 0, 3300);
        player.update_afk_zones();
        assert!(!player.zones_afk());
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use crate::interaction::InteractionTarget;
    use crate::{LocSerial, ObjSerial};
    use rs_pack::types::{LocAngle, LocLayer, LocShape};
    use rs_vm::trigger::ServerTriggerType;
    use std::iter;

    fn make_player() -> Player {
        let uid = PlayerUid::new("test".into(), 1);
        let vars = VarSet::new(iter::empty());
        Player::new(uid, CoordGrid::new(3222, 0, 3222), vars, false)
    }

    fn obj_target() -> InteractionTarget {
        InteractionTarget::Obj {
            coord: CoordGrid::new(3225, 0, 3222),
            id: 100,
            serial: ObjSerial::FIRST,
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
            serial: LocSerial::FIRST,
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

    // ---- rev-647 open interfaces ----

    #[cfg(since_647)]
    fn hash(iface: u16, com: u16) -> i32 {
        ((iface as i32) << 16) | com as i32
    }

    /// A sub is open when it hangs, mount by mount, off the toplevel.
    #[cfg(since_647)]
    #[test]
    fn an_interface_is_open_when_it_hangs_off_the_toplevel() {
        let mut p = make_player();
        p.toplevel = 548;
        p.sub_opened(hash(548, 206), 149, 1); // backpack in its tab
        p.sub_opened(hash(548, 18), 762, 0); // bank on the main screen
        p.sub_opened(hash(762, 3), 9999, 3); // something inside the bank
        assert!(p.is_interface_open(548), "the toplevel");
        assert!(p.is_interface_open(149) && p.is_interface_open(762));
        assert!(p.is_interface_open(9999), "two mounts down");
        assert!(!p.is_interface_open(763), "never opened");
    }

    /// IF_OPENTOP leaves the client's sub table alone; what hung off the old
    /// root is no longer drawn.
    #[cfg(since_647)]
    #[test]
    fn a_new_toplevel_strands_what_hung_off_the_old_one() {
        let mut p = make_player();
        p.toplevel = 548;
        p.sub_opened(hash(548, 206), 149, 1);
        p.toplevel = 746;
        assert!(!p.is_interface_open(149));
        p.sub_moved(hash(548, 206), hash(746, 90));
        assert!(p.is_interface_open(149), "moved across, it is drawn again");
    }

    /// Mounting on an occupied component closes the occupant, and closing a
    /// sub takes its type-3 children with it.
    #[cfg(since_647)]
    #[test]
    fn opening_over_a_sub_closes_it_and_its_type_3_children() {
        let mut p = make_player();
        p.toplevel = 548;
        p.sub_opened(hash(548, 18), 762, 0);
        p.sub_opened(hash(762, 3), 9999, 3);
        p.sub_opened(hash(762, 4), 8888, 1);
        p.sub_opened(hash(548, 18), 620, 0); // a shop replaces the bank
        assert!(!p.is_interface_open(762) && !p.is_interface_open(9999));
        assert!(
            !p.open_subs.contains_key(&hash(762, 3)),
            "type 3 closed with it"
        );
        assert!(
            p.open_subs.contains_key(&hash(762, 4)),
            "type 1 stays in the table"
        );
        assert!(!p.is_interface_open(8888), "but is drawn nowhere");
        assert!(p.is_interface_open(620));
    }

    // ---- inventory transmit bindings (rev 647: whole component hashes) ----

    /// `inventory:inv` (149:0) and the bank's component 0 (762:0) share a
    /// component index. Cut to 16 bits they were the same key, so stopping one
    /// stopped the other.
    #[cfg(since_647)]
    #[test]
    fn stopping_one_component_leaves_another_with_the_same_index() {
        let mut p = make_player();
        let backpack = 149 << 16;
        let bank = 762 << 16;
        p.inv_transmits.entry(93).or_default().push(backpack);
        p.inv_transmits.entry(95).or_default().push(bank);
        p.clear_inv_transmits(bank);
        assert_eq!(p.inv_transmits[&93], vec![backpack]);
        assert!(!p.inv_transmits.contains_key(&95));
    }

    /// An inventory no component transmits any more is dropped and forgotten,
    /// so it stops going out and the next transmit sends it whole; one still
    /// transmitted elsewhere is kept.
    #[cfg(since_647)]
    #[test]
    fn an_inventory_nothing_transmits_is_forgotten() {
        let mut p = make_player();
        let backpack = 149 << 16;
        let shop_side = 621 << 16;
        let shop = 620 << 16 | 25;
        p.inv_transmits
            .entry(93)
            .or_default()
            .extend([backpack, shop_side]);
        p.inv_transmits.entry(1).or_default().push(shop);
        p.inv_first_seen.extend([93, 1]);
        p.clear_inv_transmits(shop_side);
        p.clear_inv_transmits(shop);
        assert_eq!(p.inv_transmits[&93], vec![backpack]);
        assert!(p.inv_first_seen.contains(&93), "the backpack is still out");
        assert!(!p.inv_transmits.contains_key(&1), "the shop is dropped");
        assert!(!p.inv_first_seen.contains(&1), "and goes whole next time");
    }

    /// Another player's inv is forgotten once no component shows it (a
    /// trade's other offer, when its screen closes), and kept while one does
    /// (the trade's confirm screen, after the offer screen stops).
    #[cfg(since_647)]
    #[test]
    fn another_players_inventory_nothing_shows_is_forgotten() {
        let mut p = make_player();
        let offer_screen = 335 << 16 | 34;
        let confirm_screen = 334 << 16 | 34;
        p.inv_other_transmits.insert(offer_screen, (7, 90));
        p.inv_other_transmits.insert(confirm_screen, (7, 90));
        p.inv_other_first_seen.insert(90);
        p.clear_inv_transmits(offer_screen);
        assert!(
            p.inv_other_first_seen.contains(&90),
            "the confirm screen shows it"
        );
        p.clear_inv_transmits(confirm_screen);
        assert!(p.inv_other_transmits.is_empty());
        assert!(
            !p.inv_other_first_seen.contains(&90),
            "and goes whole next time"
        );
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
        p.pathing.queue_waypoint(CoordGrid::new(3225, 0, 3222));
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
        p.pathing.queue_waypoint(CoordGrid::new(3225, 0, 3222));
        assert!(p.pathing.has_waypoints());
        assert_eq!(p.pathing.waypoints.index, 0);
    }

    #[test]
    fn clear_waypoints_removes_them() {
        let mut p = make_player();
        p.pathing.queue_waypoint(CoordGrid::new(3225, 0, 3222));
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
        assert_eq!(p.pathing.waypoints.index, 2);
        // first walked = waypoints[2] = wp[0] (reversed)
        let first = p.pathing.waypoints[2];
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
        p.pathing.queue_waypoint(CoordGrid::new(3225, 0, 3222));

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
        p.pathing.queue_waypoint(CoordGrid::new(3225, 0, 3222));

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
    use std::iter;

    fn make_player() -> Player {
        let uid = PlayerUid::new("test".into(), 1);
        let vars = VarSet::new(iter::empty());
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
            StaffModLevel::Maintainer,
        ] {
            assert_eq!(StaffModLevel::from_u8(level as u8), level);
        }
    }

    #[test]
    fn from_u8_unknown_falls_back_to_normal() {
        assert_eq!(StaffModLevel::from_u8(4), StaffModLevel::Normal);
        assert_eq!(StaffModLevel::from_u8(255), StaffModLevel::Normal);
    }

    #[test]
    fn jagex_moderators_and_up_are_isolated() {
        assert!(!StaffModLevel::Normal.is_isolated());
        assert!(!StaffModLevel::PlayerModerator.is_isolated());
        assert!(StaffModLevel::JagexModerator.is_isolated());
        assert!(StaffModLevel::Maintainer.is_isolated());
    }
}
