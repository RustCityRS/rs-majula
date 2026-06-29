use crate::active_npc::ActiveNpc;
use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::clients::client_db::{DbRequest, DbResponse};
use crate::clients::client_ether::{EtherInbound, EtherOutbound, max_friends_cap};
use crate::clients::client_game::ClientHandle;
use crate::game_map::{GameMap, apply_loc_collision};
use crate::info::{NpcInfo, NpcSnapshot, PlayerInfo, PlayerSnapshot};
use crate::phases::shared::panic_message;
use crate::player_save::*;
use crate::{MAX_NPCS, MAX_PLAYERS};
use mpsc::{UnboundedReceiver, UnboundedSender};
use rs_cam::CamKind;
use rs_datastruct::{HashTable, LinkList};
#[cfg(before_254)]
use rs_entity::StaffModLevel;
use rs_entity::{EntityLifeTime, InteractionTarget, Loc, Obj, REVEAL_TICKS};
use rs_entity::{MODAL_MAIN, MODAL_NONE, NpcUid, PlayerUid};
use rs_grid::{CoordGrid, ZoneCoordGrid};
use rs_info::{NpcRenderer, PlayerRenderer};
use rs_inv::{Inventory, STACK_LIMIT, StackMode};
use rs_pack::cache::script::{Script, ScriptProvider};
use rs_pack::cache::{CacheStore, VarValue};
use rs_pack::types::{BlockWalk, LocAngle, LocLayer, LocShape, NpcMode, NpcStat, PlayerStat};
use rs_protocol::LoginResponse;
use rs_protocol::network::game::info_prot::{NpcInfoProt, PlayerInfoProt};
use rs_protocol::network::game::server::obj_count::ObjCount;
use rs_protocol::network::game::server::update_reboot_timer::UpdateRebootTimer;
use rs_util::random::JavaRandom;
use rs_var::VarSet;
use rs_vm::engine::{ScriptEngine, ScriptNpc, ScriptPlayer, engine_typed, engine_typed_mut};
pub use rs_vm::engine::{cache, with_engine};
use rs_vm::pointer::ScriptPointer;
use rs_vm::register::OpsRegistry;
use rs_vm::state::*;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ScriptError, ops, vm};
use rs_zone::zone_map::ZoneMap;
use rs_zone::{ZoneEventType, ZoneMessage};
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use rsmod::rsmod::flag::collision_flag::CollisionFlag;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use tokio::sync::{mpsc, watch};
use tracing::{error, info};
use watch::{Sender, channel};

/// Returns an immutable reference to the global [`Engine`] singleton.
///
/// This is a convenience wrapper around the generic `engine_typed` accessor,
/// pinned to the concrete `Engine` type used by this server.
///
/// # Safety
///
/// Internally calls `engine_typed::<Engine>()` which dereferences a global
/// raw pointer. The caller must ensure the engine has been installed via
/// `with_engine` before calling this function.
///
/// # Returns
///
/// A `&'static Engine` pointing to the globally-registered engine instance.
///
/// # Panics
///
/// Will dereference a null or dangling pointer (undefined behavior) if
/// called before the engine is initialized.
pub fn engine() -> &'static Engine {
    unsafe { engine_typed::<Engine>() }
}

/// Returns a mutable reference to the global [`Engine`] singleton.
///
/// This is a convenience wrapper around the generic `engine_typed_mut` accessor,
/// pinned to the concrete `Engine` type used by this server.
///
/// # Safety
///
/// Internally calls `engine_typed_mut::<Engine>()` which dereferences a global
/// raw pointer. The caller must ensure the engine has been installed via
/// `with_engine` before calling this function, and that no aliasing mutable
/// references exist.
///
/// # Returns
///
/// A `&'static mut Engine` pointing to the globally-registered engine instance.
///
/// # Panics
///
/// Will dereference a null or dangling pointer (undefined behavior) if
/// called before the engine is initialized.
pub fn engine_mut() -> &'static mut Engine {
    unsafe { engine_typed_mut::<Engine>() }
}

/// Represents an incoming player login attempt received from the network layer.
///
/// Contains all the information needed to authenticate a player and establish
/// their session: the network handle for sending responses, credentials, and
/// client capability flags.
pub struct LoginRequest {
    pub handle: ClientHandle,
    pub username: Box<str>,
    pub password: Box<str>,
    pub low_memory: bool,
    pub remote_addr: SocketAddr,
    pub reconnect: bool,
}

/// Per-tick performance statistics for the game engine cycle.
///
/// Each field (except `clock`, `total_ms`, `player_count`, `npc_count`)
/// records the wall-clock duration in milliseconds of the corresponding
/// engine phase within a single tick. Published to the `tick_stats_tx`
/// watch channel at the end of every [`Engine::cycle`] call so that
/// external monitoring (e.g. a dashboard or admin console) can observe
/// engine performance in real time.
#[derive(Debug, Clone, Default)]
pub struct TickStats {
    pub clock: u32,
    pub total_ms: f64,
    pub player_count: usize,
    pub npc_count: usize,
    pub world: f64,
    pub logins: f64,
    pub logouts: f64,
    pub input: f64,
    pub npcs: f64,
    pub players: f64,
    pub zones: f64,
    pub info: f64,
    pub ether: f64,
    pub saves: f64,
    pub autosave: f64,
    pub out: f64,
    pub cleanup: f64,
}

/// A deferred zone event scheduled for future execution at a specific game tick.
///
/// These events are stored in `Engine::pending_zone_events` (a `BTreeMap` keyed
/// by clock tick) and processed during the world phase of each game cycle.
///
/// Variants:
/// - `ObjReveal` -- makes a previously receiver-only object visible to all players.
/// - `ObjDelete` -- removes a ground object after its lifetime expires.
/// - `ObjAdd` -- respawns a ground object after it was picked up or removed.
/// - `LocDelete` -- reverts or removes a temporary location change after its duration.
#[derive(PartialEq, Eq)]
pub enum PendingZoneEvent {
    ObjReveal {
        coord: CoordGrid,
        id: u16,
        receiver37: u64,
    },
    ObjDelete {
        coord: CoordGrid,
        id: u16,
        clock: u32,
    },
    ObjAdd {
        coord: CoordGrid,
        id: u16,
    },
    LocDelete {
        coord: CoordGrid,
        layer: LocLayer,
        clock: u32,
    },
}

/// A request to spawn a ground object after a delay (measured in game ticks).
///
/// Queued into `Engine::obj_delayed_queue` and processed during the world phase.
/// Once the `delay` expires, the object is created at `coord` with the specified
/// `id`, `count`, `receiver37`, and `duration` (despawn lifetime).
pub struct ObjDelayedRequest {
    pub coord: u32,
    pub id: u16,
    pub count: u32,
    pub receiver37: Option<u64>,
    pub duration: u64,
    pub delay: u64,
}

/// Tracks the state of a login that is waiting for asynchronous validation.
///
/// A login requires both ether (cross-world) authorisation and database
/// profile loading before it can be finalised. Each response arrives
/// independently, so this struct accumulates the results until both are
/// available, at which point [`Engine::try_complete_login`] promotes it
/// into a full player session.
///
/// The `profile` field uses `Option<Option<PlayerProfile>>`:
/// - `None` -- profile has not been fetched yet.
/// - `Some(None)` -- profile was fetched but does not exist (new player).
/// - `Some(Some(..))` -- profile was fetched and contains saved data.
pub struct PendingLogin {
    pub user37: u64,
    pub request: LoginRequest,
    pub clock: u32,
    pub ether_allowed: bool,
    pub auth_ok: bool,
    pub profile: Option<Option<PlayerProfile>>,
}

fn next_free_id(cursor: u16, upper: u16, lower: u16, is_free: impl Fn(u16) -> bool) -> Option<u16> {
    for i in (cursor + 1)..upper {
        if is_free(i) {
            return Some(i);
        }
    }
    (lower..=cursor).find(|&i| is_free(i))
}

/// Whether a privately-dropped obj should ever be revealed to all players.
///
/// Untradeable objs are never revealed (they stay receiver-only until they
/// despawn); members objs are revealed only on a members world.
const fn obj_revealable(tradeable: bool, members: bool, world_members: bool) -> bool {
    tradeable && (!members || world_members)
}

/// Cancels the earliest pending [`PendingZoneEvent::ObjReveal`] scheduled for the
/// given `(coord, id, receiver37)`, if one exists.
///
/// Called when a privately-dropped obj is removed (e.g. picked up) before its
/// reveal fires. Unlike `ObjDelete` -- which `remove_obj_by_clock` clock-guards
/// so a stale event cannot delete a re-dropped obj -- `reveal_obj` matches only
/// coord/id/receiver37, so a leftover reveal would reveal a later same-tile/id
/// re-drop by the same receiver too early. Exactly ONE reveal is removed so a
/// multi-drop (two identical non-stackable objs on one tile, each with its own
/// scheduled reveal) keeps one reveal per remaining obj.
fn cancel_one_pending_obj_reveal(
    pending: &mut BTreeMap<u32, Vec<PendingZoneEvent>>,
    coord: CoordGrid,
    id: u16,
    receiver37: u64,
) {
    let target = PendingZoneEvent::ObjReveal {
        coord,
        id,
        receiver37,
    };
    for events in pending.values_mut() {
        if let Some(pos) = events.iter().position(|e| *e == target) {
            events.remove(pos);
            return;
        }
    }
}

/// Reapplies the active colour tag at the start of each `\n`-delimited line so
/// colours carry across line breaks, inserting `@bla@` after a `@str@` reset.
///
/// The delimiter is the literal two-character sequence `\n` (backslash + `n`), not a newline byte.
fn preserve_settext_colors(text: &str) -> String {
    let mut lines: Vec<String> = text.split("\\n").map(str::to_string).collect();
    let mut saved_col: Option<String> = None;

    for (i, line) in lines.iter_mut().enumerate() {
        // apply the color carried over from the previous line
        if i > 0 {
            if let Some(col) = saved_col.clone() {
                if !line.is_empty() {
                    if let Some(str_index) = line.find("@str@") {
                        let after = str_index + 5;
                        if line.as_bytes().get(after..after + 5) != Some(&b"@bla@"[..]) {
                            line.insert_str(after, "@bla@");
                        }
                        saved_col = None;
                    } else {
                        *line = format!("{col}{line}");
                    }
                }
            }
        }

        // scan this line for the last color tag, tracking @str@/@bla@ resets
        let bytes = line.as_bytes();
        let mut j = 0;
        while j + 4 < bytes.len() {
            if bytes[j] == b'@' && bytes[j + 4] == b'@' {
                if &bytes[j + 1..j + 4] == b"str" {
                    saved_col = None;
                    if bytes.get(j + 5..j + 10) == Some(&b"@bla@"[..]) {
                        j += 10;
                        continue;
                    }
                } else {
                    saved_col = Some(line[j..j + 5].to_string());
                }
                j += 5;
                continue;
            }
            j += 1;
        }
    }

    lines.join("\\n")
}

pub struct PlayerList {
    pub players: Vec<Option<ActivePlayer>>,
    pub processing: HashTable<u16>,
    node_map: Vec<usize>,
    cursor: u16,
    pid_scratch: Vec<u16>,
}

impl PlayerList {
    pub fn new() -> Self {
        let mut players = Vec::with_capacity(MAX_PLAYERS);
        players.resize_with(MAX_PLAYERS, || None);
        Self {
            players,
            processing: HashTable::new(8),
            node_map: vec![0; MAX_PLAYERS],
            cursor: (MAX_PLAYERS - 2) as u16,
            pid_scratch: Vec::with_capacity(MAX_PLAYERS),
        }
    }

    /// Takes the reusable pid buffer, filled with the current
    /// processing-order pids. A stable owned snapshot is required because the
    /// phase loops may remove entries (emergency removal) mid-iteration.
    /// Return it with [`put_pids`](Self::put_pids) to reuse the allocation.
    pub fn take_pids(&mut self) -> Vec<u16> {
        let mut v = std::mem::take(&mut self.pid_scratch);
        v.clear();
        v.extend(self.processing.iter().copied());
        v
    }

    /// Returns the buffer taken by [`take_pids`](Self::take_pids) for reuse.
    pub fn put_pids(&mut self, v: Vec<u16>) {
        self.pid_scratch = v;
    }

    pub fn next_pid(&self) -> Option<u16> {
        next_free_id(self.cursor, (MAX_PLAYERS - 1) as u16, 1, |i| {
            self.players[i as usize].is_none()
        })
    }

    pub fn add(&mut self, pid: u16, active: ActivePlayer, key: i64) {
        self.cursor = pid;
        let node_idx = self.processing.put(key, pid);
        self.node_map[pid as usize] = node_idx;
        self.players[pid as usize] = Some(active);
    }

    pub fn remove(&mut self, pid: u16) -> Option<ActivePlayer> {
        if self.players[pid as usize].is_some() {
            self.processing.unlink(self.node_map[pid as usize]);
        }
        self.players[pid as usize].take()
    }

    pub fn get(&self, pid: u16) -> Option<&ActivePlayer> {
        self.players.get(pid as usize)?.as_ref()
    }

    pub fn get_mut(&mut self, pid: u16) -> Option<&mut ActivePlayer> {
        self.players.get_mut(pid as usize)?.as_mut()
    }

    pub fn pids(&self) -> Vec<u16> {
        self.processing.iter().copied().collect()
    }

    pub fn count(&self) -> usize {
        self.processing.len()
    }
}

pub struct NpcList {
    pub npcs: Vec<Option<ActiveNpc>>,
    pub processing: HashTable<u16>,
    node_map: Vec<usize>,
    cursor: u16,
    nid_scratch: Vec<u16>,
}

impl NpcList {
    pub fn new() -> Self {
        let mut npcs = Vec::with_capacity(MAX_NPCS);
        npcs.resize_with(MAX_NPCS, || None);
        Self {
            npcs,
            processing: HashTable::new(8),
            node_map: vec![0; MAX_NPCS],
            cursor: (MAX_NPCS - 2) as u16,
            nid_scratch: Vec::with_capacity(MAX_NPCS),
        }
    }

    /// Takes the reusable nid buffer, filled with the current processing-order
    /// nids. Return it with [`put_nids`](Self::put_nids) to reuse the
    /// allocation. See [`PlayerList::take_pids`].
    pub fn take_nids(&mut self) -> Vec<u16> {
        let mut v = std::mem::take(&mut self.nid_scratch);
        v.clear();
        v.extend(self.processing.iter().copied());
        v
    }

    /// Returns the buffer taken by [`take_nids`](Self::take_nids) for reuse.
    pub fn put_nids(&mut self, v: Vec<u16>) {
        self.nid_scratch = v;
    }

    pub fn next_nid(&self) -> Option<u16> {
        next_free_id(self.cursor, (MAX_NPCS - 1) as u16, 0, |i| {
            self.npcs[i as usize].is_none()
        })
    }

    pub fn add(&mut self, nid: u16, active: ActiveNpc, key: i64) {
        self.cursor = nid;
        let node_idx = self.processing.put(key, nid);
        self.node_map[nid as usize] = node_idx;
        self.npcs[nid as usize] = Some(active);
    }

    pub fn remove(&mut self, nid: u16) -> Option<ActiveNpc> {
        if self.npcs[nid as usize].is_some() {
            self.processing.unlink(self.node_map[nid as usize]);
        }
        self.npcs[nid as usize].take()
    }

    pub fn get(&self, nid: u16) -> Option<&ActiveNpc> {
        self.npcs.get(nid as usize)?.as_ref()
    }

    pub fn get_mut(&mut self, nid: u16) -> Option<&mut ActiveNpc> {
        self.npcs.get_mut(nid as usize)?.as_mut()
    }

    pub fn nids(&self) -> Vec<u16> {
        self.processing.iter().copied().collect()
    }

    pub fn count(&self) -> usize {
        self.processing.len()
    }
}

/// The central game-state container and tick orchestrator.
///
/// `Engine` owns every piece of mutable world state: all players, NPCs,
/// zones, inventories, scripts, and the collision map. A single instance
/// is created at startup, leaked into a `'static` reference, and driven
/// by the main tokio task which calls [`Engine::cycle`] once per game tick
/// (nominally every 600 ms).
///
/// # Thread Safety
///
/// `Engine` is only ever accessed from the single world-tick task.
/// `unsafe impl Send` is provided so it can be moved into that task;
/// it is *not* `Sync` and must never be shared across threads.
pub struct Engine {
    pub clock: u32,
    pub shutdown_clock: Option<u32>,
    pub members: bool,
    pub multi_xp: u8,
    pub client_pathfinder: bool,
    pub player_list: PlayerList,
    pub npc_list: NpcList,
    pub zones: ZoneMap,
    pub new_player_rx: UnboundedReceiver<LoginRequest>,
    pub scripts: ScriptProvider,
    pub cache: &'static CacheStore,
    cache_ptr: *mut CacheStore,
    tick_stats_tx: Option<Sender<TickStats>>,
    pub reload_tx: UnboundedSender<()>,
    pub player_renderer: PlayerRenderer,
    pub npc_renderer: NpcRenderer,
    pub player_info: PlayerInfo,
    pub npc_info: NpcInfo,
    pub player_snapshots: Box<[PlayerSnapshot; MAX_PLAYERS]>,
    pub npc_snapshots: Box<[NpcSnapshot; MAX_NPCS]>,
    pub invs: FxHashMap<u16, Inventory>,
    pub ops: OpsRegistry,
    pub zones_tracking: FxHashSet<ZoneCoordGrid>,
    pub pending_zone_events: BTreeMap<u32, Vec<PendingZoneEvent>>,
    pub world_queue: LinkList<ScriptState>,
    pub obj_delayed_queue: LinkList<ObjDelayedRequest>,
    pub clock_rate_tx: Sender<u64>,
    pub node_id: u8,
    pub ether_tx: Option<UnboundedSender<EtherOutbound>>,
    pub ether_rx: UnboundedReceiver<EtherInbound>,
    pub db_tx: Option<UnboundedSender<DbRequest>>,
    pub db_rx: UnboundedReceiver<DbResponse>,
    pub db_ready: bool,
    pub ether_ready: bool,
    pub pending_logins: Vec<PendingLogin>,
    pub random: JavaRandom,
    pub vars: VarSet,
    /// A reusable [`ScriptState`] kept alive between script invocations to
    /// avoid repeated heap allocations for the fixed-size stacks (int_stack,
    /// string_stack) and frame stacks. Taken before each `run_script_inner`
    /// call and put back when the script finishes or aborts without being
    /// suspended (suspended states are stored elsewhere and must not be
    /// reused until they complete).
    reusable_script: Option<ScriptState>,
}

// SAFETY: Engine is only accessed from the single world-tick tokio task.
// The *mut CacheStore points to the same Box::leak'd allocation that all
// &'static CacheStore references share; it is only written during reload_assets
// which runs exclusively on that task.
unsafe impl Send for Engine {}

impl Engine {
    /// Constructs a new [`Engine`], loading the game map and spawning all
    /// static NPCs defined in the cache.
    ///
    /// # Arguments
    ///
    /// * `members` -- Whether this world runs in members mode.
    /// * `multi_xp` -- Stat experience multiplier.
    /// * `client_pathfinder` -- Whether to use the client-side pathfinder.
    /// * `new_player_rx` -- Channel receiver for incoming [`LoginRequest`]s.
    /// * `scripts` -- Pre-loaded script provider (RuneScript bytecode).
    /// * `cache` -- Leaked static reference to the game cache store.
    /// * `cache_ptr` -- Raw pointer to the same `CacheStore` allocation, used
    ///   by [`Engine::reload_assets`] to swap cache data in place.
    /// * `stats_tx` -- Watch channel sender for publishing [`TickStats`].
    /// * `reload_tx` -- Channel to signal the asset-reload task.
    /// * `node_id` -- Unique identifier for this world node (multi-world).
    /// * `ether_tx` -- Optional sender for cross-world (ether) messages.
    /// * `ether_rx` -- Receiver for inbound cross-world messages.
    /// * `db_tx` -- Optional sender for database requests.
    /// * `db_rx` -- Receiver for database responses.
    ///
    /// # Returns
    ///
    /// A tuple of `(Engine, watch::Receiver<u64>)` where the receiver streams
    /// the current clock rate (tick interval in milliseconds) to the scheduler.
    ///
    /// # Side Effects
    ///
    /// - Calls [`GameMap::load`] to populate the zone map with static locs, objs, and NPC spawn points.
    /// - Spawns all static NPCs via [`Engine::add_npc`], triggering their `ai_spawn` scripts.
    /// - Registers all VM opcodes via `register_ops`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** server startup (`main` / bootstrap).
    /// **Calls:** `GameMap::load`, `register_ops`, `Engine::add_npc`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        members: bool,
        multi_xp: u8,
        client_pathfinder: bool,
        new_player_rx: UnboundedReceiver<LoginRequest>,
        scripts: ScriptProvider,
        cache: &'static CacheStore,
        cache_ptr: *mut CacheStore,
        stats_tx: Sender<TickStats>,
        reload_tx: UnboundedSender<()>,
        node_id: u8,
        ether_tx: Option<UnboundedSender<EtherOutbound>>,
        ether_rx: UnboundedReceiver<EtherInbound>,
        db_tx: Option<UnboundedSender<DbRequest>>,
        db_rx: UnboundedReceiver<DbResponse>,
    ) -> (Self, watch::Receiver<u64>) {
        let ops = register_ops();

        let mut zones = ZoneMap::new();
        let spawned_npcs = GameMap::load(members, cache, &mut zones);

        let vars = VarSet::new(cache.varss.types.iter().map(|v| v.var_type));

        let (clock_rate_tx, clock_rate_rx) = channel(600);

        let mut engine = Self {
            clock: 0,
            shutdown_clock: None,
            members,
            multi_xp,
            client_pathfinder,
            player_list: PlayerList::new(),
            npc_list: NpcList::new(),
            zones,
            new_player_rx,
            scripts,
            cache,
            cache_ptr,
            tick_stats_tx: Some(stats_tx),
            reload_tx,
            player_renderer: PlayerRenderer::new(),
            npc_renderer: NpcRenderer::new(),
            player_info: PlayerInfo::new(),
            npc_info: NpcInfo::new(),
            player_snapshots: Box::new([PlayerSnapshot::ABSENT; MAX_PLAYERS]),
            npc_snapshots: Box::new([NpcSnapshot::ABSENT; MAX_NPCS]),
            invs: FxHashMap::default(),
            ops,
            zones_tracking: FxHashSet::default(),
            pending_zone_events: BTreeMap::new(),
            world_queue: LinkList::new(),
            obj_delayed_queue: LinkList::new(),
            clock_rate_tx,
            node_id,
            ether_tx,
            ether_rx,
            db_tx,
            db_rx,
            db_ready: false,
            ether_ready: false,
            pending_logins: Vec::new(),
            random: JavaRandom::new(1084838400000),
            vars,
            reusable_script: None,
        };
        for npc in spawned_npcs {
            engine.add_npc(npc);
        }

        (engine, clock_rate_rx)
    }
}

// -----------------------------------------------------------------------
// Cycle orchestration
// -----------------------------------------------------------------------

/// Outcome of a single [`Engine::cycle`], telling the driver loop how to proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleResult {
    /// Cycle completed normally; keep ticking.
    Continue,
    /// A phase panicked fatally. Players were emergency-saved and removed; the
    /// driver should stop the engine.
    Fatal,
    /// A scheduled reboot has fully drained the world (no players remain). The
    /// driver should flush pending saves and exit the process cleanly.
    Shutdown,
}

impl Engine {
    /// Executes one full game tick, running every engine phase in sequence.
    ///
    /// The phases, executed in order, are:
    /// 1. **world** -- process pending zone events, delayed objs, world scripts.
    /// 2. **input** -- read and dispatch client packets.
    /// 3. **npcs** -- NPC AI timers, movement, and scripts.
    /// 4. **players** -- player timers, queued actions, and movement.
    /// 5. **logouts** -- finalize any pending player disconnections.
    /// 6. **autosave** -- periodic persistence of online player data.
    /// 7. **logins** -- accept and initialize new player sessions.
    /// 8. **ether** -- process cross-world (ether) messages.
    /// 9. **saves** -- process database save responses.
    /// 10. **zones** -- build and buffer zone update messages.
    /// 11. **info** -- compute player and NPC info (appearance) blocks.
    /// 12. **out** -- flush buffered packets to all connected clients.
    /// 13. **cleanup** -- reset per-tick flags and advance entity state.
    ///
    /// Each phase is wrapped in `catch_unwind` so that a panic in one phase
    /// does not crash the entire server. After all phases complete, the
    /// engine clock is incremented and [`TickStats`] are published.
    ///
    /// # Side Effects
    ///
    /// - Increments `self.clock` by 1.
    /// - Publishes a [`TickStats`] snapshot to `tick_stats_tx`.
    /// - Emits a `tick_stats` tracing log line with timing details.
    ///
    /// # Call Stack
    ///
    /// **Called by:** the main tick loop (tokio task scheduler).
    /// **Calls:** `world`, `inputs`, `npcs`, `players`, `logouts`, `autosave`,
    /// `logins`, `ether`, `saves`, `zones`, `infos`, `outputs`, `cleanups`.
    #[rustfmt::skip]
    pub fn cycle(&mut self) -> CycleResult {
        let engine = self as *mut Engine;
        with_engine(self, || {
            let engine = unsafe { &mut *engine };

            let start = Instant::now();
            let mut fatal = false;

            macro_rules! phase {
                ($name:expr, $call:expr) => {{
                    let t = Instant::now();
                    if let Err(panic) = catch_unwind(AssertUnwindSafe(|| { $call; })) {
                        error!("FATAL panic during {} phase: {}", $name, panic_message(&panic));
                        fatal = true;
                    }
                    t.elapsed()
                }};
            }

            let world = phase!("world", engine.world());
            let input = phase!("in", engine.inputs());
            let npcs = phase!("npcs", engine.npcs());
            let players = phase!("players", engine.players());
            let logouts = phase!("logouts", engine.logouts());
            let autosave = phase!("autosave", engine.autosave());
            let logins = phase!("logins", engine.logins());
            let ether = phase!("ether", engine.ether());
            let saves = phase!("saves", engine.saves());
            let zones = phase!("zones", engine.zones());
            let info = phase!("info", engine.infos());
            let out = phase!("out", engine.outputs());
            let cleanup = phase!("cleanup", engine.cleanups());
            engine.clock += 1;

            if fatal {
                error!("Fatal phase panic detected -- emergency saving and removing all players");
                let pids = engine.player_list.pids();
                for pid in pids {
                    error!("emergency removing player {pid} due to fatal phase panic");
                    engine.emergency_remove_player(pid);
                }
                return CycleResult::Fatal;
            }

            if engine.is_shutdown() && engine.process_shutdown() {
                return CycleResult::Shutdown;
            }

            let cycle = start.elapsed();

            let player_count = engine.player_list.count();
            let npc_count = engine.npc_list.count();

            if let Some(tx) = &engine.tick_stats_tx {
                let _ = tx.send(TickStats {
                    clock: engine.clock - 1,
                    total_ms: cycle.as_secs_f64() * 1000.0,
                    player_count,
                    npc_count,
                    world: world.as_secs_f64() * 1000.0,
                    logins: logins.as_secs_f64() * 1000.0,
                    logouts: logouts.as_secs_f64() * 1000.0,
                    input: input.as_secs_f64() * 1000.0,
                    npcs: npcs.as_secs_f64() * 1000.0,
                    players: players.as_secs_f64() * 1000.0,
                    zones: zones.as_secs_f64() * 1000.0,
                    ether: ether.as_secs_f64() * 1000.0,
                    saves: saves.as_secs_f64() * 1000.0,
                    autosave: autosave.as_secs_f64() * 1000.0,
                    info: info.as_secs_f64() * 1000.0,
                    out: out.as_secs_f64() * 1000.0,
                    cleanup: cleanup.as_secs_f64() * 1000.0,
                });
            }

            info!(
                target: "tick_stats",
                "Tick {} | {:.2}ms/600ms ({:.1}%) | players={} npcs={} | \
                 world={:.2} logins={:.2} logouts={:.2} autosave={:.2} input={:.2} npcs={:.2} \
                 players={:.2} zones={:.2} ether={:.2} saves={:.2} info={:.2} \
                 clients_out={:.2} cleanup={:.2}ms",
                engine.clock - 1,
                cycle.as_secs_f64() * 1000.0,
                (cycle.as_secs_f64() / 0.6) * 100.0,
                player_count,
                npc_count,
                world.as_secs_f64() * 1000.0,
                logins.as_secs_f64() * 1000.0,
                logouts.as_secs_f64() * 1000.0,
                autosave.as_secs_f64() * 1000.0,
                input.as_secs_f64() * 1000.0,
                npcs.as_secs_f64() * 1000.0,
                players.as_secs_f64() * 1000.0,
                zones.as_secs_f64() * 1000.0,
                ether.as_secs_f64() * 1000.0,
                saves.as_secs_f64() * 1000.0,
                info.as_secs_f64() * 1000.0,
                out.as_secs_f64() * 1000.0,
                cleanup.as_secs_f64() * 1000.0,
            );

            CycleResult::Continue
        })
    }

    /// Updates the tick interval (clock rate) used by the external scheduler.
    ///
    /// # Arguments
    ///
    /// * `ms` -- The desired tick interval in milliseconds (e.g. 600 for the
    ///   default game speed).
    ///
    /// # Side Effects
    ///
    /// Sends the new rate through `clock_rate_tx`. The scheduler task watches
    /// this channel and adjusts its sleep duration accordingly.
    pub fn set_clock_rate(&self, ms: u64) {
        let _ = self.clock_rate_tx.send(ms);
    }
}

// -----------------------------------------------------------------------
// Reboot / maintenance shutdown
// -----------------------------------------------------------------------

impl Engine {
    /// Whether the scheduled shutdown clock has arrived -- the world is now
    /// being drained towards an orderly stop.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown_clock.is_some_and(|tick| self.clock >= tick)
    }

    /// Whether the server is shutting down within the next 30 seconds
    /// (50 ticks). New logins are refused from this point onward.
    pub fn is_shutdown_soon(&self) -> bool {
        // currentTick >= shutdownTick - 50, rearranged to avoid underflow.
        self.shutdown_clock
            .is_some_and(|tick| self.clock + 50 >= tick)
    }

    /// Whether a reboot is scheduled and still in the future.
    pub fn is_pending_shutdown(&self) -> bool {
        self.shutdown_clock.is_some_and(|tick| tick >= self.clock)
    }

    /// Clocks remaining until the scheduled reboot, or `-1` when none is
    /// scheduled; this is the value shown by [`UpdateRebootTimer`].
    pub fn shutdown_ticks_remaining(&self) -> i64 {
        match self.shutdown_clock {
            Some(clock) => clock as i64 - self.clock as i64,
            None => -1,
        }
    }

    /// The remaining-ticks countdown clamped into the [`UpdateRebootTimer`]
    /// packet's `u16` field.
    fn reboot_timer_clocks(&self) -> u16 {
        self.shutdown_ticks_remaining().clamp(0, u16::MAX as i64) as u16
    }

    /// Schedules a reboot/maintenance shutdown `duration` clocks from now and
    /// shows every online player the resulting countdown. A `duration` of 0
    /// reboots as soon as the world can be drained.
    pub fn reboot_timer(&mut self, duration: u64) {
        self.shutdown_clock = Some(self.clock + duration as u32);
        let clocks = self.reboot_timer_clocks();
        for active in self.player_list.players.iter_mut().flatten() {
            active.write(UpdateRebootTimer { clocks });
        }
        info!(
            "Reboot scheduled in {duration} tick(s) (fires at tick {})",
            self.clock + duration as u32
        );
    }

    /// Drives a triggered shutdown towards process exit. Run once per cycle from
    /// [`Engine::cycle`] (after the clock increment) while [`Engine::is_shutdown`]
    /// holds.
    ///
    /// Each cycle it asks every still-connected player to log out; the logout
    /// phase then removes and saves them cleanly. Players still present 1024
    /// ticks after the shutdown began are force-removed -- they had their
    /// chance to finish processing. Once a brief grace period passes the clock
    /// rate drops to 0 ms so the remaining logout/save flushes drain at full
    /// speed.
    ///
    /// # Returns
    ///
    /// `true` once the world is empty (no players, no pending logins),
    /// signaling [`Engine::cycle`] to return [`CycleResult::Shutdown`] so the
    /// driver can flush database saves and exit the process.
    fn process_shutdown(&mut self) -> bool {
        // Ask every connected (non-bot) player to log out. Idempotent: once
        // `logout_sent` is set the logout phase takes over the removal.
        for active in self.player_list.players.iter_mut().flatten() {
            if !active.player.bot && !active.player.logout_sent {
                active.logout();
            }
        }

        let duration = self.clock - self.shutdown_clock.unwrap_or(self.clock);

        if duration >= 1024 {
            // Force out anyone still here -- they had 1024 ticks to finish.
            for pid in self.player_list.pids() {
                error!("Player (pid={pid}) force removed during shutdown");
                self.emergency_remove_player(pid);
            }
        }

        if self.player_list.count() == 0 && self.pending_logins.is_empty() {
            info!("Server shutdown complete");
            return true;
        }

        if duration > 2 {
            // After ~1 second (time to flush logout packets first) run flat out
            // so the remaining players drain quickly.
            self.set_clock_rate(0);
        }

        false
    }
}

// -----------------------------------------------------------------------
// Script execution
// -----------------------------------------------------------------------

impl Engine {
    /// Computes the script lookup key for a given server trigger.
    ///
    /// Trigger lookup keys encode the trigger type, an optional type-specific
    /// id (`t`), and an optional category (`c`) into a single `i32`. The
    /// method tries the most specific key first (type id), then category,
    /// and finally falls back to the bare trigger ordinal.
    ///
    /// # Arguments
    ///
    /// * `trigger` -- The server trigger type (e.g. `AiSpawn`, `OpObj`).
    /// * `t` -- Optional entity type id to specialise the lookup.
    /// * `c` -- Optional category id for category-level script matching.
    ///
    /// # Returns
    ///
    /// The bound [`Script`] for the most specific matching key (type id, then
    /// category, then the bare trigger ordinal), or `None` if no script is
    /// bound. Resolving the script here lets callers skip a redundant
    /// `get_by_lookup` probe -- the helper would otherwise probe the map up to
    /// twice just to choose a key the caller then re-probed a third time.
    pub fn script_by_key(
        &self,
        trigger: ServerTriggerType,
        t: Option<u16>,
        c: Option<i32>,
    ) -> Option<Arc<Script>> {
        let base = trigger as i32;

        if let Some(t) = t {
            let key = base | (0x2 << 8) | ((t as i32) << 10);
            if let Some(script) = self.scripts.get_by_lookup(key) {
                return Some(Arc::clone(script));
            }
        }

        if let Some(c) = c
            && c != -1
        {
            let key = base | (0x1 << 8) | (c << 10);
            if let Some(script) = self.scripts.get_by_lookup(key) {
                return Some(Arc::clone(script));
            }
        }

        self.scripts.get_by_lookup(base).cloned()
    }

    /// Hot-reloads the game cache and script provider in place.
    ///
    /// Drops the old `CacheStore` data and writes `new_store` into the same
    /// allocation that every `&'static CacheStore` reference points to, then
    /// replaces the script provider. This allows live-reloading of game data
    /// and scripts without restarting the server.
    ///
    /// # Arguments
    ///
    /// * `new_store` -- The replacement cache store (owned box, consumed).
    /// * `new_scripts` -- The replacement script provider with freshly compiled scripts.
    ///
    /// # Safety
    ///
    /// Uses raw pointer operations (`drop_in_place` + `write`) on `self.cache_ptr`.
    /// This is safe only because the engine is accessed exclusively from a single
    /// task and `cache_ptr` points to a `Box::leak`'d allocation.
    ///
    /// # Side Effects
    ///
    /// - Replaces the global cache store in place (all `&'static CacheStore` refs
    ///   now point to the new data).
    /// - Replaces `self.scripts` with `new_scripts`.
    /// - In debug builds, broadcasts a reload notification to all online players.
    /// - Logs the number of loaded scripts.
    ///
    /// # Call Stack
    ///
    /// **Called by:** the asset-reload handler (triggered via `reload_tx`).
    pub fn reload_assets(&mut self, new_store: Box<CacheStore>, new_scripts: ScriptProvider) {
        unsafe {
            std::ptr::drop_in_place(self.cache_ptr);
            std::ptr::write(self.cache_ptr, *new_store);
        }
        self.scripts = new_scripts;
        let count = self.scripts.count();

        #[cfg(debug_assertions)]
        self.broadcast(&format!("Hot-reload applied - {count} scripts loaded"));
        info!("Hot-reload applied - {} scripts loaded", count);
    }

    /// Executes a single RuneScript VM invocation against the given script state.
    ///
    /// Temporarily installs `self` as the global engine (via `with_engine`) so
    /// that VM opcodes can access world state through the `ScriptEngine` trait,
    /// then runs the VM until the script finishes, suspends, or aborts.
    ///
    /// # Arguments
    ///
    /// * `state` -- The mutable script state (program counter, stack, locals, etc.).
    ///
    /// # Returns
    ///
    /// The [`ExecutionState`] indicating how the script exited: `Finished`,
    /// `Aborted`, `Suspended` (player), `WorldSuspended`, or `NpcSuspended`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `runescript_execute_script_player`, `runescript_execute_script_npc`.
    /// **Calls:** `vm::execute`.
    pub fn runescript_vm_execute(&mut self, state: &mut ScriptState) -> ExecutionState {
        let ops = &self.ops as *const OpsRegistry;
        with_engine(self, move || vm::execute::<Engine>(state, unsafe { &*ops }))
    }

    /// Executes a pre-built [`ScriptState`] against the appropriate subject entity.
    ///
    /// Routes execution to `runescript_execute_script_player` or
    /// `runescript_execute_script_npc` depending on the subject variant.
    /// `Loc` and `Obj` subjects are currently no-ops.
    ///
    /// # Arguments
    ///
    /// * `state` -- The fully initialized script state to execute.
    /// * `subject` -- The entity that owns this script execution. `None`
    ///   returns `ScriptError::NoSubject`.
    /// * `protect` -- If `Some(true)`, marks the player as protected for the
    ///   duration of the script (prevents interruption by other scripts).
    /// * `force` -- If `Some(true)`, runs even if the player is already
    ///   protected or delayed.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or `Err(ScriptError::NoSubject)` if no subject
    /// was provided.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ai_spawn`, `ai_despawn`, and various engine phases.
    /// **Calls:** `runescript_execute_script_player`, `runescript_execute_script_npc`.
    pub fn run_script_by_state(
        &mut self,
        state: ScriptState,
        subject: Option<ScriptSubject>,
        protect: Option<bool>,
        force: Option<bool>,
    ) -> Result<(), ScriptError> {
        let Some(subject) = subject else {
            return Err(ScriptError::NoSubject);
        };
        let returned = match subject {
            ScriptSubject::Player(uid) => {
                let protect = protect.unwrap_or(false);
                let force = force.unwrap_or(false);
                self.runescript_execute_script_player(uid, state, protect, force)
            }
            ScriptSubject::Npc(uid) => self.runescript_execute_script_npc(uid, state),
            ScriptSubject::Loc(_) | ScriptSubject::Obj(_) => Some(state),
        };
        if let Some(returned_state) = returned {
            self.reusable_script = Some(returned_state);
        }
        Ok(())
    }

    /// Builds a [`ScriptState`], reusing a pooled state's heap buffers when one
    /// is available (falling back to [`ScriptState::init`]). Mirrors the pool
    /// logic in [`run_script_inner`](Self::run_script_inner); pair it with the
    /// reclaim in [`run_script_by_state`](Self::run_script_by_state) so per-tick
    /// timer/queue scripts cycle a single state instead of allocating ~4 KB
    /// each. `last_int` (if needed) must be set by the caller after building, as
    /// `reset` nulls it.
    pub fn build_state(
        &mut self,
        script: Arc<Script>,
        subject: Option<ScriptSubject>,
        target: Option<ScriptSubject>,
        args: Option<Vec<ScriptArgument>>,
    ) -> ScriptState {
        if let Some(mut reusable) = self.reusable_script.take() {
            reusable.reset(script, subject, target, args);
            reusable
        } else {
            ScriptState::init(script, subject, target, args)
        }
    }

    /// Looks up a script by server trigger and executes it against a subject entity.
    ///
    /// Resolves the trigger triple `(type, entity_id, category)` into a lookup
    /// key via [`Engine::script_by_key`], fetches the corresponding script
    /// from the provider, builds a [`ScriptState`], and delegates to
    /// [`Engine::run_script_inner`].
    ///
    /// # Arguments
    ///
    /// * `trigger` -- Tuple of `(ServerTriggerType, Option<type_id>, Option<category>)`.
    /// * `subject` -- The entity that owns the script execution.
    /// * `target` -- Optional secondary entity (e.g. interaction target).
    /// * `protect` -- Whether the owning player should be marked as protected.
    /// * `force` -- Whether to bypass protection/delay checks.
    /// * `args` -- Optional script arguments pushed onto the stack.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a `ScriptError` if the trigger has no bound
    /// script or no subject was provided.
    ///
    /// # Call Stack
    ///
    /// **Called by:** login phase, input handlers, AI timers, interaction handlers.
    /// **Calls:** `trigger_lookup_key`, `run_script_inner`.
    pub fn run_script_by_trigger(
        &mut self,
        trigger: (ServerTriggerType, Option<u16>, Option<i32>),
        subject: Option<ScriptSubject>,
        target: Option<ScriptSubject>,
        protect: Option<bool>,
        force: Option<bool>,
        args: Option<Vec<ScriptArgument>>,
    ) -> Result<(), ScriptError> {
        let Some(script) = self.script_by_key(trigger.0, trigger.1, trigger.2) else {
            return Err(ScriptError::TriggerNotFound(trigger.0));
        };
        self.run_script_inner(subject, target, protect, force, args, script)
            .unwrap_or_else(|value| value)
    }

    /// Looks up a script by its string name and executes it against a subject entity.
    ///
    /// This is the name-based counterpart to [`Engine::run_script_by_trigger`].
    /// Useful for explicitly invoking scripts by their defined name rather than
    /// by trigger binding (e.g. command scripts, quest scripts).
    ///
    /// # Arguments
    ///
    /// * `name` -- The script name as defined in the script source (e.g. `"[proc,heal]"`).
    /// * `subject` -- The entity that owns the script execution.
    /// * `target` -- Optional secondary entity (e.g. interaction target).
    /// * `protect` -- Whether the owning player should be marked as protected.
    /// * `force` -- Whether to bypass protection/delay checks.
    /// * `args` -- Optional script arguments pushed onto the stack.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a `ScriptError` if the script name is not found
    /// or no subject was provided.
    ///
    /// # Call Stack
    ///
    /// **Called by:** command handlers, quest triggers, misc game logic.
    /// **Calls:** `run_script_inner`.
    pub fn run_script_by_name(
        &mut self,
        name: &str,
        subject: Option<ScriptSubject>,
        target: Option<ScriptSubject>,
        protect: Option<bool>,
        force: Option<bool>,
        args: Option<Vec<ScriptArgument>>,
    ) -> Result<(), ScriptError> {
        let Some(script) = self.scripts.get_by_name(name).cloned() else {
            return Err(ScriptError::ScriptNotFoundName(name.into()));
        };
        self.run_script_inner(subject, target, protect, force, args, script)
            .unwrap_or_else(|value| value)
    }

    /// Internal helper that builds a [`ScriptState`] and dispatches execution
    /// to the appropriate player or NPC handler.
    ///
    /// Used by both [`Engine::run_script_by_trigger`] and
    /// [`Engine::run_script_by_name`] to avoid code duplication.
    ///
    /// When a [`reusable_state`](Self::reusable_state) is available, it is
    /// taken and [`reset`](ScriptState::reset) is called to reinitialize it
    /// for the new script invocation, avoiding fresh heap allocations for the
    /// fixed-size stacks. After execution, if the script finished or aborted
    /// (i.e., was not suspended), the state is recaptured for future reuse.
    /// Suspended states are stored elsewhere and must not be reused until
    /// they complete.
    ///
    /// # Arguments
    ///
    /// * `subject` -- The entity that owns the script. `None` returns an error.
    /// * `target` -- Optional interaction target entity.
    /// * `protect` -- Whether to mark the player as protected.
    /// * `force` -- Whether to bypass protection/delay checks.
    /// * `args` -- Optional script arguments.
    /// * `script` -- The resolved script to execute (already cloned from the provider).
    ///
    /// # Returns
    ///
    /// `Ok(Ok(()))` on success. `Err(Err(ScriptError::NoSubject))` when no
    /// subject is provided. The nested `Result` allows the caller to use
    /// `unwrap_or_else` to flatten the error path.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `run_script_by_trigger`, `run_script_by_name`.
    /// **Calls:** `ScriptState::reset` or `ScriptState::init`,
    /// `runescript_execute_script_player`, `runescript_execute_script_npc`.
    fn run_script_inner(
        &mut self,
        subject: Option<ScriptSubject>,
        target: Option<ScriptSubject>,
        protect: Option<bool>,
        force: Option<bool>,
        args: Option<Vec<ScriptArgument>>,
        script: Arc<Script>,
    ) -> Result<Result<(), ScriptError>, Result<(), ScriptError>> {
        let Some(subject) = subject else {
            return Err(Err(ScriptError::NoSubject));
        };

        // Determine which entity executor to use before moving subject
        // into the state. PlayerUid/NpcUid are Copy.
        enum SubjectKind {
            Player(PlayerUid),
            Npc(NpcUid),
            Other,
        }

        let kind = match &subject {
            ScriptSubject::Player(uid) => SubjectKind::Player(*uid),
            ScriptSubject::Npc(uid) => SubjectKind::Npc(*uid),
            ScriptSubject::Loc(_) | ScriptSubject::Obj(_) => SubjectKind::Other,
        };

        // Take the reusable state if available, otherwise allocate fresh.
        let state = if let Some(mut reusable) = self.reusable_script.take() {
            reusable.reset(script, Some(subject), target, args);
            reusable
        } else {
            ScriptState::init(script, Some(subject), target, args)
        };

        // Execute and recapture the state if it was not suspended.
        let returned = match kind {
            SubjectKind::Player(uid) => {
                let protect = protect.unwrap_or(false);
                let force = force.unwrap_or(false);
                self.runescript_execute_script_player(uid, state, protect, force)
            }
            SubjectKind::Npc(uid) => self.runescript_execute_script_npc(uid, state),
            SubjectKind::Other => Some(state),
        };

        // Reclaim the state for reuse on the next invocation.
        if let Some(returned_state) = returned {
            self.reusable_script = Some(returned_state);
        }

        Ok(Ok(()))
    }

    /// Executes a RuneScript with a player as the subject, handling protection
    /// flags, suspension, and post-execution cleanup.
    ///
    /// If `protect` is true the player's `protect` flag is set before execution
    /// and cleared after. If `force` is false and the player is already
    /// protected or delayed, execution is skipped entirely.
    ///
    /// After VM execution, the method inspects the returned [`ExecutionState`]:
    /// - `Finished` / `Aborted` -- clears the player's active script and
    ///   closes any open modal if appropriate.
    /// - `WorldSuspended` -- enqueues the script into the world queue with
    ///   the delay popped from the stack.
    /// - `NpcSuspended` -- parks the script on the active NPC's state.
    /// - Other suspension -- parks the script on the player's state.
    ///
    /// Also cleans up `ProtectedActivePlayer` / `ProtectedActivePlayer2`
    /// pointer flags to ensure no stale protection lingers.
    ///
    /// # Returns
    ///
    /// `Some(state)` when the script finished or aborted (the caller may
    /// reclaim the state for reuse). `None` when the script was suspended
    /// and its state has been moved into storage for later resumption, or
    /// when execution was skipped due to protection/delay guards.
    ///
    /// # Arguments
    ///
    /// * `uid` -- The player's unique identifier (encodes pid).
    /// * `state` -- The script state to execute (consumed).
    /// * `protect` -- Whether to set the protection flag during execution.
    /// * `force` -- Whether to skip the protection/delay guard.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `run_script_by_state`, `run_script_inner`.
    /// **Calls:** `runescript_vm_execute`, `enqueue_world_script`.
    #[allow(clippy::collapsible_if)]
    fn runescript_execute_script_player(
        &mut self,
        uid: PlayerUid,
        mut state: ScriptState,
        protect: bool,
        force: bool,
    ) -> Option<ScriptState> {
        let pid = uid.pid() as usize;
        if !force && protect {
            if let Some(active) = &mut self.player_list.players[pid] {
                if active.player.state.protect || active.player.state.delayed {
                    return Some(state);
                }
            }
        }

        if protect {
            if let Some(active) = &mut self.player_list.players[pid] {
                state.pointers.add(ScriptPointer::ProtectedActivePlayer);
                active.player.state.protect = true;
            }
        }

        let result = self.runescript_vm_execute(&mut state);

        if protect {
            if let Some(active) = &mut self.player_list.players[pid] {
                active.player.state.protect = false;
            }
        }

        if state.pointers.has(ScriptPointer::ProtectedActivePlayer) && state.active_player.is_some()
        {
            if let Some(uid) = state.active_player {
                if let Some(active) = &mut self.player_list.players[uid.pid() as usize] {
                    active.player.state.protect = false;
                }
            }
            state.pointers.remove(ScriptPointer::ProtectedActivePlayer);
        }

        if state.pointers.has(ScriptPointer::ProtectedActivePlayer2)
            && state.active_player2.is_some()
        {
            if let Some(uid) = state.active_player2 {
                if let Some(active) = &mut self.player_list.players[uid.pid() as usize] {
                    active.player.state.protect = false;
                }
            }
            state.pointers.remove(ScriptPointer::ProtectedActivePlayer2);
        }

        if result != ExecutionState::Finished && result != ExecutionState::Aborted {
            if result == ExecutionState::WorldSuspended {
                let delay = state.pop_int() as u16;
                self.enqueue_world_script(state, delay);
            } else if result == ExecutionState::NpcSuspended {
                let npc_uid = if state.int_operand() == 0 {
                    state.active_npc
                } else {
                    state.active_npc2
                };
                if let Some(uid) = npc_uid {
                    if let Some(active) = self.npc_list.get_mut(uid.nid()) {
                        active.npc.state.active_script = Some(Box::new(state));
                    }
                }
            } else if let Some(active) = &mut self.player_list.players[pid] {
                active.player.state.active_script = Some(Box::new(state));
                active.player.state.protect = protect;
            }
            None
        } else {
            if let Some(active) = &mut self.player_list.players[pid] {
                if let Some(active_script) = &active.player.state.active_script {
                    if active_script.root_script_id == state.root_script_id {
                        active.player.state.active_script = None;
                        if active.player.modal_state & MODAL_MAIN == MODAL_NONE {
                            if let Err(e) = active.close_modal(false) {
                                error!(
                                    "error closing modal after script finish for player {pid}: {e}"
                                );
                            }
                        }
                    }
                }
            }
            Some(state)
        }
    }

    /// Executes a RuneScript with an NPC as the subject, handling suspension
    /// and post-execution cleanup.
    ///
    /// Similar to [`Engine::runescript_execute_script_player`] but without
    /// protection/force semantics (NPCs are never "protected"). After
    /// execution the method routes suspended scripts to the appropriate
    /// entity (world queue, NPC state, or a referenced player).
    ///
    /// Also cleans up `ProtectedActivePlayer` / `ProtectedActivePlayer2`
    /// pointer flags on any players that were marked during script execution.
    ///
    /// # Returns
    ///
    /// `Some(state)` when the script finished or aborted (the caller may
    /// reclaim the state for reuse). `None` when the script was suspended
    /// and its state has been moved into storage for later resumption.
    ///
    /// # Arguments
    ///
    /// * `uid` -- The NPC's unique identifier (encodes nid).
    /// * `state` -- The script state to execute (consumed).
    ///
    /// # Call Stack
    ///
    /// **Called by:** `run_script_by_state`, `run_script_inner`.
    /// **Calls:** `runescript_vm_execute`, `enqueue_world_script`.
    #[allow(clippy::collapsible_if)]
    fn runescript_execute_script_npc(
        &mut self,
        uid: NpcUid,
        mut state: ScriptState,
    ) -> Option<ScriptState> {
        let result = self.runescript_vm_execute(&mut state);

        if state.pointers.has(ScriptPointer::ProtectedActivePlayer) && state.active_player.is_some()
        {
            if let Some(uid) = state.active_player {
                if let Some(active) = &mut self.player_list.players[uid.pid() as usize] {
                    active.player.state.protect = false;
                }
            }
            state.pointers.remove(ScriptPointer::ProtectedActivePlayer);
        }

        if state.pointers.has(ScriptPointer::ProtectedActivePlayer2)
            && state.active_player2.is_some()
        {
            if let Some(uid) = state.active_player2 {
                if let Some(active) = &mut self.player_list.players[uid.pid() as usize] {
                    active.player.state.protect = false;
                }
            }
            state.pointers.remove(ScriptPointer::ProtectedActivePlayer2);
        }

        if result != ExecutionState::Finished && result != ExecutionState::Aborted {
            if result == ExecutionState::WorldSuspended {
                let delay = state.pop_int().max(0) as u16;
                self.enqueue_world_script(state, delay);
            } else if result == ExecutionState::NpcSuspended {
                if let Some(active) = &mut self.npc_list.npcs[uid.nid() as usize] {
                    active.npc.state.active_script = Some(Box::new(state));
                }
            } else if let Some(uid) = state.active_player {
                if let Some(active) = &mut self.player_list.players[uid.pid() as usize] {
                    active.player.state.active_script = Some(Box::new(state));
                }
            } else if let Some(uid) = state.active_player2 {
                if let Some(active) = &mut self.player_list.players[uid.pid() as usize] {
                    active.player.state.active_script = Some(Box::new(state));
                }
            }
            None
        } else {
            if let Some(active) = &mut self.npc_list.npcs[uid.nid() as usize] {
                if let Some(active_script) = &active.npc.state.active_script {
                    if active_script.root_script_id == state.root_script_id {
                        active.npc.state.active_script = None;
                    }
                }
            }
            Some(state)
        }
    }
}

// -----------------------------------------------------------------------
// Zone tracking
// -----------------------------------------------------------------------

impl Engine {
    /// Marks a zone as dirty so that its pending events are flushed to clients
    /// during the zones phase of the current tick.
    ///
    /// # Arguments
    ///
    /// * `x` -- Zone X coordinate.
    /// * `y` -- Zone level (height plane).
    /// * `z` -- Zone Z coordinate.
    ///
    /// # Side Effects
    ///
    /// Inserts the zone coordinate into `self.zones_tracking`.
    pub fn track_zone(&mut self, x: u16, y: u8, z: u16) {
        let coord = ZoneCoordGrid::new(x, y, z);
        self.zones_tracking.insert(coord);
    }

    /// Schedules a [`PendingZoneEvent`] to fire at the given game tick.
    ///
    /// Events are stored in a `BTreeMap<u64, Vec<PendingZoneEvent>>` keyed by
    /// clock tick, ensuring they are processed in chronological order during
    /// the world phase.
    ///
    /// # Arguments
    ///
    /// * `clock` -- The game tick at which the event should be processed.
    /// * `event` -- The zone event to enqueue.
    pub fn schedule_zone_event(&mut self, clock: u32, event: PendingZoneEvent) {
        self.pending_zone_events
            .entry(clock)
            .or_default()
            .push(event);
    }

    /// Enqueues a suspended script into the world script queue with a tick delay.
    ///
    /// World scripts are not bound to a specific player or NPC -- they execute
    /// in the world phase and can interact with any entity. The delay is
    /// incremented by 1 internally (so `delay=0` fires on the next tick).
    ///
    /// # Arguments
    ///
    /// * `script` -- The suspended script state to enqueue.
    /// * `delay` -- Number of additional ticks before the script resumes.
    ///
    /// # Side Effects
    ///
    /// Appends the script to `self.world_queue`.
    pub fn enqueue_world_script(&mut self, mut script: ScriptState, delay: u16) {
        script.delay = (delay + 1) as i32;
        self.world_queue.add_tail(script);
    }
}

// -----------------------------------------------------------------------
// World obj management
// -----------------------------------------------------------------------

impl Engine {
    /// Adds a ground object to the world at its embedded coordinate.
    ///
    /// For stackable items owned by a specific receiver, attempts to merge
    /// the count into an existing stack via [`Engine::merge_obj`] before
    /// creating a new entity. If a receiver is specified, the object starts
    /// as receiver-only and a `ObjReveal` event is scheduled to make it
    /// globally visible after [`REVEAL_TICKS`]. An `ObjDelete` event is
    /// always scheduled to despawn the object after `duration` ticks.
    ///
    /// # Arguments
    ///
    /// * `obj` -- The ground object to place (coordinate is embedded).
    /// * `receiver37` -- Optional Base37-encoded username of the player who
    ///   can see the object before it is revealed globally.
    /// * `duration` -- Lifetime in game ticks before automatic removal.
    ///
    /// # Side Effects
    ///
    /// - Inserts the object into the zone's obj list.
    /// - Schedules `ObjReveal` and/or `ObjDelete` zone events.
    /// - Marks the zone as dirty via [`Engine::track_zone`].
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptEngine::add_obj`, world phase obj-delayed processing.
    /// **Calls:** `merge_obj`, `schedule_zone_event`, `track_zone`.
    pub fn add_obj(
        &mut self,
        coord: CoordGrid,
        mut obj: Obj,
        receiver37: Option<u64>,
        duration: u64,
    ) {
        let obj_type = self.cache.objs.get_by_id(obj.id());

        let stackable = obj_type.is_some_and(|t| t.stackable);

        if stackable
            && obj.lifetime() == EntityLifeTime::Despawn
            && let Some(r) = receiver37
            && self.merge_obj(coord, &obj, r, duration)
        {
            return;
        }

        let clock = self.clock + duration as u32;
        obj.set_last_clock(clock);
        if let Some(r) = receiver37 {
            let reveal_clock = self.clock + REVEAL_TICKS;

            let revealable =
                obj_type.is_some_and(|t| obj_revealable(t.tradeable, t.members, self.members));

            if revealable && reveal_clock < clock {
                self.schedule_zone_event(
                    reveal_clock,
                    PendingZoneEvent::ObjReveal {
                        coord,
                        id: obj.id(),
                        receiver37: r,
                    },
                );
            }
        }

        self.schedule_zone_event(
            clock,
            PendingZoneEvent::ObjDelete {
                coord,
                id: obj.id(),
                clock,
            },
        );

        let (ox, oy, oz) = (coord.x(), coord.y(), coord.z());
        let zone = self.zones.zone_mut(ox, oy, oz);
        zone.add_obj(obj, receiver37);
        self.track_zone(ox, oy, oz);
    }

    /// Attempts to merge a stackable object's count into an existing stack
    /// at the same coordinate, owned by the same receiver.
    ///
    /// Only merges if the existing object is also a `Despawn` entity and the
    /// combined count does not exceed [`STACK_LIMIT`]. On success, sends an
    /// `ObjCount` zone message to update clients and reschedules the deletion
    /// event.
    ///
    /// # Arguments
    ///
    /// * `obj` -- The new object whose count should be merged.
    /// * `receiver` -- Base37 username of the receiver to match.
    /// * `duration` -- New despawn lifetime (resets the deletion timer).
    ///
    /// # Returns
    ///
    /// `true` if the merge succeeded (caller should *not* create a new obj),
    /// `false` if no compatible stack was found or the merge would exceed the
    /// stack limit.
    ///
    /// # Side Effects
    ///
    /// - Mutates the existing obj's `count` and `last_clock` in the zone.
    /// - Queues an `ObjCount` zone event for client updates.
    /// - Schedules a new `ObjDelete` event for the merged stack.
    /// - Marks the zone as dirty.
    fn merge_obj(&mut self, coord: CoordGrid, obj: &Obj, receiver: u64, duration: u64) -> bool {
        let zone = self.zones.zone_mut(coord.x(), coord.y(), coord.z());
        let Some(idx) = zone.get_obj_of_receiver(coord.x(), coord.z(), obj.id(), receiver) else {
            return false;
        };
        if zone.objs[idx].lifetime() != EntityLifeTime::Despawn {
            return false;
        }
        let next_count = obj.count() + zone.objs[idx].count();
        if next_count > STACK_LIMIT {
            return false;
        }

        let old_count = zone.objs[idx].count();
        zone.objs[idx].set_count(next_count);
        let clock = self.clock + duration as u32;
        zone.objs[idx].set_last_clock(clock);

        let oid = zone.objs[idx].oid();
        let message = ZoneMessage::ObjCount(ObjCount {
            coord: obj.packed_zone_coord(),
            id: obj.id(),
            old_count: old_count.clamp(0, 65535) as u16,
            new_count: next_count.clamp(0, 65535) as u16,
        });
        zone.queue_event(Some(oid), ZoneEventType::Follows, Some(receiver), message);
        self.track_zone(coord.x(), coord.y(), coord.z());

        self.schedule_zone_event(
            clock,
            PendingZoneEvent::ObjDelete {
                coord,
                id: obj.id(),
                clock,
            },
        );
        true
    }

    /// Removes a ground object from the world, optionally scheduling a respawn.
    ///
    /// If `duration` is non-zero the respawn delay is scaled by the current
    /// player count (via [`Engine::scale_by_player_count`]) and an `ObjAdd`
    /// event is scheduled to re-create the object after that period.
    ///
    /// # Arguments
    ///
    /// * `coord` -- World coordinate of the object.
    /// * `id` -- Object type id.
    /// * `receiver37` -- Optional Base37 username to identify which instance
    ///   to remove when multiple stacks exist at the same tile.
    /// * `duration` -- Respawn delay in game ticks (0 = no respawn).
    ///
    /// # Side Effects
    ///
    /// - Removes the object from the zone's obj list.
    /// - Schedules an `ObjAdd` respawn event if `duration > 0`.
    /// - Marks the zone as dirty.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptEngine::remove_obj`, obj interaction scripts.
    /// **Calls:** `scale_by_player_count`, `schedule_zone_event`, `track_zone`.
    pub fn remove_obj(
        &mut self,
        coord: CoordGrid,
        id: u16,
        receiver37: Option<u64>,
        duration: u64,
    ) {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        let respawn_at = if duration > 0 {
            let scaled = self.scale_by_player_count(duration);
            Some(self.clock + scaled as u32)
        } else {
            None
        };
        self.zones
            .zone_mut(x, y, z)
            .remove_obj(x, z, id, receiver37, respawn_at);
        if let Some(receiver) = receiver37 {
            cancel_one_pending_obj_reveal(&mut self.pending_zone_events, coord, id, receiver);
        }
        if let Some(clock) = respawn_at {
            self.schedule_zone_event(clock, PendingZoneEvent::ObjAdd { coord, id });
        }
        self.track_zone(x, y, z);
    }
}

// -----------------------------------------------------------------------
// World loc management
// -----------------------------------------------------------------------

impl Engine {
    /// Adds a new loc or changes an existing loc at the given coordinate and layer.
    ///
    /// If a loc already exists on the same layer at the target fine coordinate,
    /// it is changed in place: the old collision is removed, the loc's type/shape/angle
    /// are updated, and new collision is applied. Otherwise a brand-new `Despawn`
    /// loc is created.
    ///
    /// When `duration > 0`, a `LocDelete` event is scheduled to revert or remove
    /// the loc after the specified number of ticks.
    ///
    /// # Arguments
    ///
    /// * `coord` -- World coordinate for the loc.
    /// * `id` -- Loc type id (indexes into `cache.locs`).
    /// * `shape` -- The visual/collision shape of the loc.
    /// * `angle` -- Rotation angle of the loc (0-3).
    /// * `duration` -- Lifetime in game ticks before automatic revert (0 = permanent).
    ///
    /// # Side Effects
    ///
    /// - Updates the collision map (removes old collision, applies new collision).
    /// - Inserts or updates the loc in the zone's loc list.
    /// - Queues a zone change event for client updates.
    /// - Schedules a `LocDelete` event if the loc is temporary.
    /// - Marks the zone as dirty.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptEngine::add_or_change_loc`, loc interaction scripts.
    /// **Calls:** `apply_loc_collision`, `schedule_zone_event`,
    /// `track_zone`.
    pub fn add_or_change_loc(
        &mut self,
        coord: CoordGrid,
        id: u16,
        shape: LocShape,
        angle: LocAngle,
        duration: u64,
        create_if_missing: bool,
    ) {
        let layer = shape.layer();
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        let zone = self.zones.zone_mut(x, y, z);

        let existing = zone
            .locs
            .iter()
            .position(|loc| loc.is_at(x, z) && loc.layer() == layer);

        let cache = cache();

        if let Some(idx) = existing {
            let old_loc = zone.locs[idx];
            if old_loc.visible() {
                apply_loc_collision(&old_loc, coord, false);
            }

            let (blockwalk, blockrange, width, length) = cache
                .locs
                .get_by_id(id)
                .map(|lt| (lt.blockwalk, lt.blockrange, lt.width, lt.length))
                .unwrap_or((false, false, 1, 1));

            zone.locs[idx].change(id, shape, angle, blockwalk, blockrange, width, length);

            // Apply collision from the *stored* loc. The change above stored the new
            // id/angle/flags; only shape is shared (a change can't alter it). Reading
            // the loc back keeps the footprint added here identical to the one a later
            // revert/remove will remove from the same loc.
            let updated = zone.locs[idx];
            apply_loc_collision(&updated, coord, true);

            zone.change_loc(idx);

            let is_changed = zone.locs[idx].is_changed();
            let is_despawn = zone.locs[idx].lifetime() == EntityLifeTime::Despawn;
            if is_changed || is_despawn {
                let clock = self.clock + duration as u32;
                zone.locs[idx].set_last_clock(clock);
                self.schedule_zone_event(
                    clock,
                    PendingZoneEvent::LocDelete {
                        coord,
                        layer,
                        clock,
                    },
                );
            } else {
                zone.locs[idx].set_last_clock(u32::MAX);
            }
            self.track_zone(x, y, z);
        } else {
            if !create_if_missing {
                return;
            }

            let (blockwalk, blockrange, width, length) = self
                .cache
                .locs
                .get_by_id(id)
                .map(|lt| (lt.blockwalk, lt.blockrange, lt.width, lt.length))
                .unwrap_or((false, false, 1, 1));

            let loc = Loc::new(
                coord,
                EntityLifeTime::Despawn,
                id,
                shape,
                angle,
                blockwalk,
                blockrange,
                width,
                length,
            );

            apply_loc_collision(&loc, coord, true);

            // `add_loc` evicts the oldest despawn loc when the zone is at the loc
            // cap; clear that evicted loc's collision (owned by the engine, not
            // the zone) at its own tile.
            if let Some(evicted) = zone.add_loc(loc) {
                apply_loc_collision(&evicted, evicted.world_coord(zone.coord), false);
            }
            self.track_zone(x, y, z);

            if duration > 0 {
                let clock = self.clock + duration as u32;
                if let Some(l) = self.zones.zone_mut(x, y, z).locs.last_mut() {
                    l.set_last_clock(clock);
                }
                self.schedule_zone_event(
                    clock,
                    PendingZoneEvent::LocDelete {
                        coord,
                        layer,
                        clock,
                    },
                );
            }
        }
    }

    /// Removes a visible loc at the given coordinate and layer.
    ///
    /// Clears the loc's collision from the pathfinding map and marks it as
    /// removed in the zone. If the loc has a `Respawn` lifetime and `duration`
    /// is non-zero, a `LocDelete` event is scheduled to restore it after the
    /// specified number of ticks.
    ///
    /// # Arguments
    ///
    /// * `coord` -- World coordinate of the loc.
    /// * `layer` -- The loc layer to target (e.g. wall, ground decoration).
    /// * `duration` -- Respawn delay in game ticks (0 = permanent removal).
    ///
    /// # Side Effects
    ///
    /// - Removes collision for the loc from the pathfinding map.
    /// - Marks the loc as removed in the zone.
    /// - Schedules a `LocDelete` respawn event if applicable.
    /// - Marks the zone as dirty.
    ///
    /// # Call Stack
    ///
    /// **Called by:** loc interaction scripts, `ScriptEngine::remove_loc` (if present).
    /// **Calls:** `apply_loc_collision`, `schedule_zone_event`, `track_zone`.
    pub fn remove_loc(&mut self, coord: CoordGrid, layer: LocLayer, duration: u64) {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        let zone = self.zones.zone_mut(x, y, z);

        let Some(idx) = zone.get_loc_by_layer(x, z, layer) else {
            return;
        };

        // Copy the loc out before removal: `remove_loc` swap-removes despawn locs
        // (shrinking `locs`), so `zone.locs[idx]` is invalid afterward. Respawn
        // locs are reverted in place, so indexing `locs[idx]` stays valid in the
        // respawn branch below.
        let loc = zone.locs[idx];

        apply_loc_collision(&loc, coord, false);

        zone.remove_loc(idx);

        if loc.lifetime() == EntityLifeTime::Respawn && duration > 0 {
            let clock = self.clock + duration as u32;
            zone.locs[idx].set_last_clock(clock);
            self.schedule_zone_event(
                clock,
                PendingZoneEvent::LocDelete {
                    coord,
                    layer,
                    clock,
                },
            );
        }
        self.track_zone(x, y, z);
    }

    /// Reverts a loc at the given coordinate and layer to its original state.
    ///
    /// Undoes any `change` that was applied to the loc, restoring its original
    /// type, shape, and angle. Updates collision accordingly: removes the
    /// current collision, calls `revert()` on the loc, then applies the
    /// reverted loc's collision. Clears the loc's `last_clock` so no further
    /// timed events will fire for it.
    ///
    /// # Arguments
    ///
    /// * `coord` -- World coordinate of the loc.
    /// * `layer` -- The loc layer to target.
    ///
    /// # Side Effects
    ///
    /// - Updates the collision map (removes current, applies reverted).
    /// - Reverts the loc's properties in the zone.
    /// - Queues a zone change event for client updates.
    /// - Marks the zone as dirty.
    pub fn revert_loc(&mut self, coord: CoordGrid, layer: LocLayer) {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        let zone = self.zones.zone_mut(x, y, z);

        let Some(idx) = zone.get_loc_by_layer(x, z, layer) else {
            return;
        };

        apply_loc_collision(&zone.locs[idx], coord, false);

        zone.locs[idx].revert();

        let reverted = zone.locs[idx];
        apply_loc_collision(&reverted, coord, true);

        zone.change_loc(idx);
        zone.locs[idx].set_last_clock(u32::MAX);
        self.track_zone(x, y, z);
    }
}

// -----------------------------------------------------------------------
// Entity management
// -----------------------------------------------------------------------

impl Engine {
    pub fn add_player(&mut self, pid: u16, active: ActivePlayer, key: i64) {
        self.player_list.add(pid, active, key);
        if let Some(active) = self.get_player(pid) {
            self.zones
                .zone_mut(
                    active.player.pathing.coord.x(),
                    active.player.pathing.coord.y(),
                    active.player.pathing.coord.z(),
                )
                .add_player(pid);
        }
    }

    pub fn remove_player(&mut self, pid: u16) -> Option<ActivePlayer> {
        self.player_renderer.remove_permanent(pid);
        // Mark absent in the hot-field snapshot so any observer still processed
        // later this tick encodes a remove for this pid (mirrors the slot going
        // `None`). Harmless outside the info/output phases; refilled next tick.
        self.player_snapshots[pid as usize].clear();
        if let Some(active) = self.player_list.get(pid) {
            let coord = active.player.pathing.coord;
            let nids: Vec<u16> = active.player.build_area.npcs.iter().to_vec();
            self.zones
                .zone_mut(coord.x(), coord.y(), coord.z())
                .remove_player(pid);

            for nid in nids {
                if let Some(npc) = self
                    .npc_list
                    .npcs
                    .get_mut(nid as usize)
                    .and_then(|n| n.as_mut())
                {
                    npc.npc.observers = npc.npc.observers.saturating_sub(1);
                }
            }

            let block_walk = active.player.block_walk;
            let size = active.player.pathing.size;
            match block_walk {
                BlockWalk::Npc => {
                    rsmod::change_npc(coord.x(), coord.z(), coord.y(), size, false);
                }
                BlockWalk::All => {
                    rsmod::change_npc(coord.x(), coord.z(), coord.y(), size, false);
                    rsmod::change_player(coord.x(), coord.z(), coord.y(), size, false);
                }
                _ => {}
            }
        }
        self.player_list.remove(pid)
    }

    pub fn get_player(&self, pid: u16) -> Option<&ActivePlayer> {
        self.player_list.get(pid)
    }

    pub fn get_player_mut(&mut self, pid: u16) -> Option<&mut ActivePlayer> {
        self.player_list.get_mut(pid)
    }

    pub fn add_npc(&mut self, mut active: ActiveNpc) -> Option<NpcUid> {
        let nid = self.npc_list.next_nid()?;
        let id = active.npc.uid.id();
        active.npc.uid = NpcUid::new(id, nid);
        let uid = active.npc.uid;
        let type_id = active.npc.base_type;
        let key = active.npc.pathing.coord.packed() as i64;
        self.npc_list.add(nid, active, key);
        if let Some(active) = self.get_npc(nid) {
            self.zones
                .zone_mut(
                    active.npc.pathing.coord.x(),
                    active.npc.pathing.coord.y(),
                    active.npc.pathing.coord.z(),
                )
                .add_npc(nid);
        }

        self.ai_spawn(uid, type_id);

        Some(uid)
    }

    /// Fires the `AiSpawn` trigger script for an NPC, if one is bound.
    ///
    /// Looks up the script by trigger key (type id, category) and executes
    /// it with the NPC as the subject.
    ///
    /// # Arguments
    ///
    /// * `uid` -- The NPC's unique identifier.
    /// * `type_id` -- The NPC's base type id (used for trigger lookup).
    ///
    /// # Side Effects
    ///
    /// Executes the `AiSpawn` script, which may set NPC variables, timers,
    /// or initial behavior modes.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `add_npc`.
    /// **Calls:** `trigger_lookup_key`, `run_script_by_state`.
    pub fn ai_spawn(&mut self, uid: NpcUid, type_id: u16) {
        let category = self
            .cache
            .npcs
            .get_by_id(type_id)
            .and_then(|t| t.category)
            .map(|c| c as i32);
        if let Some(script) =
            self.script_by_key(ServerTriggerType::AiSpawn, Some(type_id), category)
        {
            let state = ScriptState::init(script, Some(ScriptSubject::Npc(uid)), None, None);
            if let Err(e) =
                self.run_script_by_state(state, Some(ScriptSubject::Npc(uid)), None, None)
            {
                error!("error running ai_spawn script for npc {}: {e}", uid.nid());
            }
        }
    }

    /// Fires the `AiDespawn` trigger script for an NPC, if one is bound.
    ///
    /// Called just before an NPC is deactivated, allowing scripts to perform
    /// cleanup such as dropping loot or resetting state.
    ///
    /// # Arguments
    ///
    /// * `uid` -- The NPC's unique identifier.
    /// * `type_id` -- The NPC's base type id (used for trigger lookup).
    ///
    /// # Side Effects
    ///
    /// Executes the `AiDespawn` script if one exists for this NPC type/category.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `deactivate_npc`.
    /// **Calls:** `trigger_lookup_key`, `run_script_by_state`.
    pub fn ai_despawn(&mut self, uid: NpcUid, type_id: u16) {
        let category = self
            .cache
            .npcs
            .get_by_id(type_id)
            .and_then(|t| t.category)
            .map(|c| c as i32);
        if let Some(script) =
            self.script_by_key(ServerTriggerType::AiDespawn, Some(type_id), category)
        {
            let state = ScriptState::init(script, Some(ScriptSubject::Npc(uid)), None, None);
            if let Err(e) =
                self.run_script_by_state(state, Some(ScriptSubject::Npc(uid)), None, None)
            {
                error!("error running ai_despawn script for npc {}: {e}", uid.nid());
            }
        }
    }

    /// Deactivates an NPC, removing it from the world without freeing its slot.
    ///
    /// Fires the `AiDespawn` script, marks the NPC as inactive, removes it
    /// from its zone, clears its collision flags, and removes it from the
    /// renderer. If the NPC has a `Respawn` lifecycle, sets `respawn_at` so
    /// the cleanup phase can re-activate it later.
    ///
    /// # Arguments
    ///
    /// * `nid` -- The NPC id to deactivate.
    ///
    /// # Side Effects
    ///
    /// - Executes the `AiDespawn` script.
    /// - Sets `active.npc.active = false`.
    /// - Removes the NPC from its zone and from the collision map.
    /// - Removes the nid from the NPC renderer.
    /// - Sets `respawn_at` for `Respawn` lifecycle NPCs.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptEngine::remove_npc`, NPC death logic.
    /// **Calls:** `ai_despawn`, `rsmod::change_npc`, `rsmod::change_player`.
    pub fn deactivate_npc(&mut self, nid: u16) {
        let (uid, type_id) = {
            let Some(active) = self.npc_list.npcs[nid as usize].as_ref() else {
                return;
            };
            if !active.npc.active {
                return;
            }
            (active.npc.uid, active.npc.base_type)
        };
        self.ai_despawn(uid, type_id);

        let Some(active) = self.npc_list.npcs[nid as usize].as_mut() else {
            return;
        };
        active.npc.active = false;

        self.zones
            .zone_mut(
                active.npc.pathing.coord.x(),
                active.npc.pathing.coord.y(),
                active.npc.pathing.coord.z(),
            )
            .remove_npc(nid);

        let block_walk = active.block_walk();
        let coord = active.npc.pathing.coord;
        let size = active.npc.pathing.size;
        match block_walk {
            BlockWalk::Npc => {
                rsmod::change_npc(coord.x(), coord.z(), coord.y(), size, false);
            }
            BlockWalk::All => {
                rsmod::change_npc(coord.x(), coord.z(), coord.y(), size, false);
                rsmod::change_player(coord.x(), coord.z(), coord.y(), size, false);
            }
            _ => {}
        }

        self.npc_renderer.remove_permanent(nid);
        self.npc_snapshots[nid as usize].clear();

        if active.npc.lifecycle == EntityLifeTime::Respawn {
            let respawnrate = self
                .cache
                .npcs
                .get_by_id(active.npc.uid.id())
                .map(|t| t.respawnrate as u64)
                .unwrap_or(100);
            active.npc.respawn_at = Some(respawnrate as u32);
        }
    }

    pub fn remove_npc(&mut self, nid: u16) -> Option<ActiveNpc> {
        self.npc_renderer.remove_permanent(nid);
        self.npc_snapshots[nid as usize].clear();
        if let Some(active) = self.get_npc(nid) {
            self.zones
                .zone_mut(
                    active.npc.pathing.coord.x(),
                    active.npc.pathing.coord.y(),
                    active.npc.pathing.coord.z(),
                )
                .remove_npc(nid);
        }
        self.npc_list.remove(nid)
    }

    pub fn get_npc(&self, nid: u16) -> Option<&ActiveNpc> {
        self.npc_list.get(nid)
    }

    pub fn get_npc_mut(&mut self, nid: u16) -> Option<&mut ActiveNpc> {
        self.npc_list.get_mut(nid)
    }

    /// Forcibly removes a player from the engine, saving their profile and
    /// notifying the ether service.
    ///
    /// Used as a last resort when a player's session is in an unrecoverable
    /// state (e.g. a panic during their tick processing). Extracts and saves
    /// the player profile to the database, sends a logout notification to
    /// ether, and then delegates to [`Engine::remove_player`].
    ///
    /// # Arguments
    ///
    /// * `pid` -- The player id to force-remove.
    ///
    /// # Side Effects
    ///
    /// - Saves the player profile to the database (if `db_tx` is available).
    /// - Sends `EtherOutbound::PlayerLogout` (if `ether_tx` is available).
    /// - Calls `remove_player` for full cleanup.
    ///
    /// # Call Stack
    ///
    /// **Called by:** panic recovery in the players/input phases.
    /// **Calls:** `extract_profile`, `save_binary`, `remove_player`.
    pub fn emergency_remove_player(&mut self, pid: u16) {
        if let Some(active) = self.player_list.players[pid as usize].as_ref() {
            let user37 = active.uid().username37();
            let username = active.uid().username();

            if let Some(tx) = &self.db_tx {
                let profile = extract_profile(&active.player, self.cache);
                let binary = save_binary(&profile, self.cache);
                let _ = tx.send(DbRequest::Save {
                    user37,
                    username,
                    profile: Box::new(profile),
                    binary,
                });
            }

            if let Some(tx) = &self.ether_tx {
                let _ = tx.send(EtherOutbound::PlayerLogout { user37 });
            }
        }

        self.remove_player(pid);
    }

    /// Forcibly deactivates an NPC without running its despawn script.
    ///
    /// Used as a last resort when an NPC's tick processing panics. Performs
    /// the same cleanup as [`Engine::deactivate_npc`] (zone removal, collision
    /// cleanup, renderer removal, respawn scheduling) but skips the `AiDespawn`
    /// script to avoid triggering further panics. For `Despawn` lifecycle NPCs,
    /// the slot is fully freed.
    ///
    /// # Arguments
    ///
    /// * `nid` -- The NPC id to force-deactivate.
    ///
    /// # Side Effects
    ///
    /// - Sets `active.npc.active = false`.
    /// - Removes the NPC from its zone and collision map.
    /// - Removes the nid from the NPC renderer.
    /// - For `Respawn` NPCs, sets `respawn_at`.
    /// - For `Despawn` NPCs, removes the nid from `active_npcs` and frees the slot.
    ///
    /// # Call Stack
    ///
    /// **Called by:** panic recovery in the NPC phase.
    pub fn emergency_deactivate_npc(&mut self, nid: u16) {
        let Some(active) = self.npc_list.npcs[nid as usize].as_mut() else {
            return;
        };
        if !active.npc.active {
            return;
        }
        active.npc.active = false;

        self.zones
            .zone_mut(
                active.npc.pathing.coord.x(),
                active.npc.pathing.coord.y(),
                active.npc.pathing.coord.z(),
            )
            .remove_npc(nid);

        let block_walk = active.block_walk();
        let coord = active.npc.pathing.coord;
        let size = active.npc.pathing.size;
        match block_walk {
            BlockWalk::Npc => {
                rsmod::change_npc(coord.x(), coord.z(), coord.y(), size, false);
            }
            BlockWalk::All => {
                rsmod::change_npc(coord.x(), coord.z(), coord.y(), size, false);
                rsmod::change_player(coord.x(), coord.z(), coord.y(), size, false);
            }
            _ => {}
        }

        self.npc_renderer.remove_permanent(nid);
        self.npc_snapshots[nid as usize].clear();

        if active.npc.lifecycle == EntityLifeTime::Respawn {
            let respawnrate = self
                .cache
                .npcs
                .get_by_id(active.npc.uid.id())
                .map(|t| t.respawnrate as u64)
                .unwrap_or(100);
            active.npc.respawn_at = Some(respawnrate as u32);
        } else {
            self.npc_list.remove(nid);
        }
    }

    /// Sends a game message to every currently online player.
    ///
    /// If the text contains newlines, each line is sent as a separate message.
    ///
    /// # Arguments
    ///
    /// * `text` -- The message text to broadcast.
    ///
    /// # Side Effects
    ///
    /// Calls `message_game_wrapped` on every active player.
    fn broadcast(&mut self, text: &str) {
        for active in self.player_list.players.iter_mut().flatten() {
            if text.contains('\n') {
                for line in text.split('\n') {
                    active.message_game_wrapped(line);
                }
            } else {
                active.message_game_wrapped(text);
            }
        }
    }

    /// Finalises a validated login by creating the player session and entering
    /// the world.
    ///
    /// Allocates a pid, sends the `Success` response to the client, constructs
    /// an [`ActivePlayer`], applies the saved profile (or new-player defaults),
    /// adds the player to the engine, calls `on_login`, notifies ether, and
    /// executes the `Login` trigger script.
    ///
    /// If the world is full (no free pid), sends `WorldFull` and returns early.
    ///
    /// # Arguments
    ///
    /// * `request` -- The original [`LoginRequest`] with the client handle.
    /// * `profile` -- The player's saved profile, or `None` for a new player.
    /// * `pid`     -- The accepted pid to use for the active player.
    ///
    /// # Side Effects
    ///
    /// - Sends `LoginResponse::Success` (or `WorldFull`) to the client.
    /// - Calls `add_player` and `on_login`.
    /// - Sends `PlayerLogin` and `RequestLists` to ether.
    /// - Executes the `Login` trigger script.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `try_complete_login`.
    /// **Calls:** `next_free_pid`, `add_player`, `run_script_by_trigger`.
    pub(crate) fn accept_login(
        &mut self,
        request: LoginRequest,
        profile: Option<PlayerProfile>,
        pid: u16,
    ) {
        let mut active = ActivePlayer::new(
            request.handle,
            pid,
            request.username,
            request.low_memory,
            false,
        );

        if let Some(profile) = &profile {
            apply_profile(profile, &mut active.player, cache());
        }
        if active.player.stats.xp.iter().all(|&s| s == 0) {
            apply_new_player_defaults(&mut active.player);
        }

        active.player.last_date = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        active.player.last_response = self.clock;

        let remote_ip = request.remote_addr.ip();
        active.remote_ip = Some(remote_ip);

        let uid = active.player.uid;

        #[cfg(since_254)]
        {
            let level = (active.player.staff_mod_level as u8).min(2);
            let _ = active
                .handle
                .outbox
                .send(vec![LoginResponse::SuccessNormal as u8, level, 1]);
        }
        #[cfg(before_254)]
        {
            let response = match active.player.staff_mod_level {
                StaffModLevel::Normal => LoginResponse::SuccessNormal,
                StaffModLevel::PlayerModerator => LoginResponse::SuccessModerator,
                #[cfg(rev = "225")]
                StaffModLevel::JagexModerator | StaffModLevel::Developer => {
                    LoginResponse::SuccessModerator
                }
                #[cfg(since_244)]
                StaffModLevel::JagexModerator | StaffModLevel::Developer => {
                    LoginResponse::SuccessJagexModerator
                }
            };
            let _ = active.handle.outbox.send(vec![response as u8]);
        }

        info!(
            "Player '{}' joined at ({},{},{}) (uid={:?}, pid={})",
            active.player.uid.username(),
            active.player.pathing.coord.x(),
            active.player.pathing.coord.z(),
            active.player.pathing.coord.y(),
            uid,
            pid,
        );

        let key = match remote_ip {
            IpAddr::V4(ip) => u32::from(ip) as i64,
            IpAddr::V6(ip) => {
                let octets = ip.octets();
                i32::from_be_bytes([octets[12], octets[13], octets[14], octets[15]]) as i64
            }
        };
        self.add_player(pid, active, key);

        let login_info = if let Some(active) = self.get_player_mut(pid) {
            active.on_login();
            Some((active.uid().username37(), active.player.is_member))
        } else {
            None
        };

        if self.is_pending_shutdown() {
            let clocks = self.reboot_timer_clocks();
            if let Some(active) = self.get_player_mut(pid) {
                active.write(UpdateRebootTimer { clocks });
            }
        }

        if let (Some((user37, is_member)), Some(tx)) = (login_info, &self.ether_tx) {
            let _ = tx.send(EtherOutbound::PlayerLogin {
                user37,
                pid,
                max_friends: max_friends_cap(is_member),
                ip: remote_ip.to_string(),
            });
            let _ = tx.send(EtherOutbound::RequestLists { user37 });
        }

        let result = self.run_script_by_trigger(
            (ServerTriggerType::Login, None, None),
            Some(ScriptSubject::Player(uid)),
            None,
            Some(true),
            None,
            None,
        );
        if result.is_err() {
            error!("{:?}", result);
        }
    }

    /// Attaches a reconnecting client to its still-in-world player session.
    ///
    /// Called when a validated reconnect login (correct password, ether
    /// approved) finds its player still present on this node -- either
    /// lingering after a disconnect or still nominally connected (in which
    /// case the old socket is kicked and the new one takes over). The
    /// existing player keeps its pid, uid, position, inventories, and all
    /// in-flight gameplay state; nothing is saved or reloaded, so there is
    /// no rollback and no save/load race.
    ///
    /// 1. Sends [`LoginResponse::Reconnect`] (15) on the new connection --
    ///    the client resumes its session instead of resetting.
    /// 2. Swaps the new [`ClientHandle`] onto the player. Dropping the old
    ///    handle closes the old socket task's channels, which terminates the
    ///    old connection (and frees its per-IP permit).
    /// 3. Revives the session: clears the logout/disconnect flags so the
    ///    logout phase stops counting the player down.
    /// 4. Forgets the per-observer view state (tracked players/NPCs and
    ///    appearance hashes), decrementing NPC observer counts exactly like
    ///    [`remove_player`](Engine::remove_player) so the re-adds during the
    ///    next info phase keep the counts balanced. The client rebuilds its
    ///    own entity lists from each info packet, so a clean server-side
    ///    slate is self-healing -- stale entities vanish, missed ones appear.
    /// 5. Calls [`EnginePlayer::on_reconnect`] to re-send all volatile client
    ///    state (varps, map rebuild, modal close, tabs, invs, stats, energy,
    ///    anims) and flag an absolute local-player re-place.
    /// 6. Re-announces the session to ether (`PlayerResync` is idempotent and
    ///    re-sends friend/ignore lists), covering an ether restart during the
    ///    disconnect gap.
    ///
    /// The `Login` script trigger is intentionally not re-run -- the session
    /// never ended.
    ///
    /// # Arguments
    ///
    /// * `pid` -- The live player slot being reclaimed.
    /// * `request` -- The validated reconnect login carrying the new handle.
    ///
    /// # Side Effects
    ///
    /// * Sends the reconnect response and full state refresh to the client.
    /// * Drops the previous client connection.
    /// * Decrements NPC observer counts for the player's old view set.
    /// * Sends [`EtherOutbound::PlayerResync`] to the ether sidecar.
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`try_complete_login`](Engine::try_complete_login)
    /// **Calls:** `EnginePlayer::on_reconnect`
    pub(crate) fn adopt_session(&mut self, pid: u16, request: LoginRequest) {
        let clock = self.clock;

        let Some(active) = self
            .player_list
            .players
            .get_mut(pid as usize)
            .and_then(|slot| slot.as_mut())
        else {
            // find_pid_by_user37 returned this pid in the same phase, so the
            // slot cannot be empty; guard anyway rather than panic.
            error!("adopt_session: pid {pid} vanished before adoption");
            let _ = request
                .handle
                .outbox
                .send(vec![LoginResponse::CouldNotComplete as u8]);
            return;
        };

        let _ = request
            .handle
            .outbox
            .send(vec![LoginResponse::Reconnect as u8]);

        // Swapping the handle drops the old one: its channels close and the
        // old socket task (dead or alive) shuts down. Buffered packets queued
        // earlier this tick survive on `active.buffered` and flush through
        // the new handle's ISAAC cipher at the output phase.
        *active.handle = request.handle;
        active.player.low_memory = request.low_memory;
        active.remote_ip = Some(request.remote_addr.ip());

        active.player.logout_requested = false;
        active.player.logout_idle_requested = false;
        active.player.logout_sent = false;
        active.player.disconnected_at = None;
        active.player.last_response = clock;

        // Forget the per-observer view state (reference: cleanupPlayerBuildArea).
        // NPC observer counts are decremented further down, outside this
        // borrow; the info phase re-increments them as NPCs are re-tracked.
        let nids: Vec<u16> = active.player.build_area.npcs.iter().to_vec();
        active.player.build_area.players.clear();
        active.player.build_area.npcs.clear();
        active.player.build_area.appearances.fill(0);

        if let Err(e) = active.on_reconnect() {
            error!("error during reconnect for player {pid}: {e}");
        }

        let user37 = active.uid().username37();
        let private_mode = active.player.private as u8;
        let is_member = active.player.is_member;
        let username = active.player.uid.username();

        for nid in nids {
            if let Some(npc) = self
                .npc_list
                .npcs
                .get_mut(nid as usize)
                .and_then(|n| n.as_mut())
            {
                npc.npc.observers = npc.npc.observers.saturating_sub(1);
            }
        }

        if let Some(tx) = &self.ether_tx {
            let _ = tx.send(EtherOutbound::PlayerResync {
                user37,
                pid,
                private_mode,
                max_friends: max_friends_cap(is_member),
                ip: request.remote_addr.ip().to_string(),
            });
        }

        info!("Player '{}' reconnected (pid={})", username, pid);
    }

    /// Attempts to complete a pending login if all async prerequisites are met.
    ///
    /// A login requires both `ether_allowed` and `auth_ok` to be true. If
    /// the profile has not yet been fetched from the database, a `DbRequest::Load`
    /// is sent. Once all three conditions are satisfied, the pending login is
    /// removed from the list and forwarded to [`Engine::accept_login`].
    ///
    /// # Arguments
    ///
    /// * `idx` -- Index into `self.pending_logins`.
    ///
    /// # Side Effects
    ///
    /// - May send a `DbRequest::Load` to fetch the player profile.
    /// - On completion, calls `accept_login` and removes the entry from
    ///   `pending_logins` via `swap_remove`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** logins phase, ether inbound handler, db response handler.
    /// **Calls:** `accept_login`.
    /// Removes a pending login, sends the given response to its client, and
    /// releases any cluster-wide login reservations the attempt holds.
    ///
    /// When a login check is granted, the ether sidecar registers account and
    /// IP reservations (10-second TTL) that are normally consumed when the
    /// session starts. A pending that dies after being granted -- failed
    /// password, timeout, world full, database loss -- would otherwise leave
    /// those reservations behind, making an immediate retry fail with
    /// "already logged in" until the TTL expires. The abort is only sent when
    /// `ether_allowed` is set: before that, this attempt owns no reservations
    /// (and an unconditional abort could release a *concurrent* attempt's).
    ///
    /// # Arguments
    ///
    /// * `idx` -- Index into `self.pending_logins` (removed via `swap_remove`).
    /// * `response` -- The login response byte to send to the client.
    ///
    /// # Call Stack
    ///
    /// **Called by:** login timeout sweep, auth-failure handler,
    /// db-disconnect drain, ether-reconnect sweep.
    pub(crate) fn reject_pending_login(&mut self, idx: usize, response: LoginResponse) {
        let pending = self.pending_logins.swap_remove(idx);
        let _ = pending.request.handle.outbox.send(vec![response as u8]);
        if pending.ether_allowed
            && let Some(tx) = &self.ether_tx
        {
            let _ = tx.send(EtherOutbound::LoginAbort {
                user37: pending.user37,
                ip: pending.request.remote_addr.ip().to_string(),
            });
        }
    }

    pub(crate) fn try_complete_login(&mut self, idx: usize) {
        let pending = &self.pending_logins[idx];
        if !pending.ether_allowed || !pending.auth_ok {
            return;
        }

        if pending.request.reconnect
            && let Some(pid) = self.find_pid_by_user37(pending.user37)
        {
            let pending = self.pending_logins.swap_remove(idx);
            self.adopt_session(pid, pending.request);
            return;
        }

        if self.is_shutdown_soon() {
            self.reject_pending_login(idx, LoginResponse::ServerUpdating);
            return;
        }

        if self.player_list.count() >= 2000 {
            self.reject_pending_login(idx, LoginResponse::WorldFull);
            return;
        }

        let Some(pid) = self.player_list.next_pid() else {
            self.reject_pending_login(idx, LoginResponse::WorldFull);
            return;
        };

        if pending.profile.is_none() {
            if let Some(tx) = &self.db_tx {
                let _ = tx.send(DbRequest::Load {
                    user37: pending.user37,
                });
            }
            return;
        }
        let pending = self.pending_logins.swap_remove(idx);
        let profile = pending.profile.unwrap();
        self.accept_login(pending.request, profile, pid);
    }

    /// Finds the pid of an online player by their Base37-encoded username.
    ///
    /// Scans the active player list and returns the pid of the first player
    /// whose `username37` matches the given value.
    ///
    /// # Arguments
    ///
    /// * `user37` -- Base37-encoded username to search for.
    ///
    /// # Returns
    ///
    /// `Some(pid)` if a matching player is online, `None` otherwise.
    pub fn find_pid_by_user37(&self, user37: u64) -> Option<u16> {
        self.player_list.processing.iter().copied().find(|&pid| {
            self.player_list.players[pid as usize]
                .as_ref()
                .is_some_and(|p| p.uid().username37() == user37)
        })
    }

    /// Scales a duration inversely by the current online player count.
    ///
    /// Used to make respawn timers shorter when more players are online,
    /// preventing resource starvation on busy worlds. The formula is
    /// `(4000 - min(player_count, 2000)) * rate / 4000`, so at 0 players the
    /// full rate is returned, and at 2000 players the rate is halved.
    ///
    /// # Arguments
    ///
    /// * `rate` -- The base duration in game ticks.
    ///
    /// # Returns
    ///
    /// The scaled duration, always less than or equal to `rate`.
    pub fn scale_by_player_count(&self, rate: u64) -> u64 {
        let player_count = self.player_list.count().min(2000) as u64;
        (4000 - player_count) * rate / 4000
    }
}

// -----------------------------------------------------------------------
// Loc occupancy
// -----------------------------------------------------------------------

impl Engine {
    /// Scans the target zone and its three south-west neighbors for an active
    /// loc whose footprint covers `coord`, returning `true` on the first hit.
    ///
    /// When `skip_invisible_walls` is set, invisible wall locs are ignored
    /// (what [`locaddunsafe`](ScriptEngine::locaddunsafe) wants); `map_loc`
    /// passes `false` so they still count.
    fn loc_occupies(&self, coord: CoordGrid, skip_invisible_walls: bool) -> bool {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        for dx in (-8..=0).step_by(8) {
            for dz in (-8..=0).step_by(8) {
                let zx = x as i32 + dx;
                let zz = z as i32 + dz;
                if zx < 0 || zz < 0 {
                    continue;
                }
                let Some(zone) = self.zones.zone(zx as u16, y, zz as u16) else {
                    continue;
                };
                for loc in &zone.locs {
                    let Some(loc_type) = self.cache.locs.get_by_id(loc.id()) else {
                        continue;
                    };

                    if loc_type.active != Some(true) {
                        continue;
                    }

                    if skip_invisible_walls && !loc.visible() && loc.layer() == LocLayer::Wall {
                        continue;
                    }

                    let loc_coord = loc.world_coord(zone.coord);
                    let (width, length) = match loc.angle() {
                        LocAngle::North | LocAngle::South => (loc.length(), loc.width()),
                        _ => (loc.width(), loc.length()),
                    };

                    for index in 0..(width as u16 * length as u16) {
                        let lx = loc_coord.x() + (index % width as u16);
                        let lz = loc_coord.z() + (index / width as u16);
                        if lx == x && lz == z {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

// -----------------------------------------------------------------------
// ScriptEngine implementation
// -----------------------------------------------------------------------

impl ScriptEngine for Engine {
    /// Returns the current game clock tick.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** reads `self.clock`
    fn clock(&self) -> u32 {
        self.clock
    }

    /// Returns the experience multiplier of the engine.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** reads `self.multi_xp`
    fn multi_experience(&self) -> u8 {
        self.multi_xp
    }

    /// Returns a reference to the global cache store.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** reads `self.cache`
    fn cache(&self) -> &CacheStore {
        self.cache
    }

    /// Looks up a compiled script by its numeric identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - The script identifier to look up.
    ///
    /// # Returns
    ///
    /// `Some(&Arc<Script>)` if the script exists, `None` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ScriptProvider::get_by_id`
    fn get_script(&self, id: i32) -> Option<&Arc<Script>> {
        self.scripts.get_by_id(id)
    }

    /// Returns the number of players currently online.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    fn playercount(&self) -> usize {
        self.player_list.count()
    }

    /// Retrieves a shared (global) inventory, creating it with stock defaults if it does not exist.
    ///
    /// If the inventory type defines `stockobj` and `stockcount` in the cache, the newly
    /// created inventory is pre-populated with those entries.
    ///
    /// # Arguments
    ///
    /// * `id` - The shared inventory identifier.
    /// * `size` - The number of slots to allocate if the inventory is created.
    /// * `stack_mode` - The stacking behavior to use if the inventory is created.
    ///
    /// # Returns
    ///
    /// A mutable reference to the shared inventory.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `FxHashMap::entry`, `Inventory::with_stack_mode`, `Inventory::set`
    fn get_shared_inv(&mut self, id: u16, size: usize, stack_mode: StackMode) -> &mut Inventory {
        self.invs.entry(id).or_insert_with(|| {
            let mut inv = Inventory::with_stack_mode(size, stack_mode);
            if let Some(inv_type) = self.cache.invs.get_by_id(id) {
                if let Some(stockobj) = &inv_type.stockobj {
                    inv.stockobj = stockobj.clone();
                }
                if let (Some(stockobj), Some(stockcount)) =
                    (&inv_type.stockobj, &inv_type.stockcount)
                {
                    for i in 0..stockobj.len() {
                        inv.set(i as u16, stockobj[i], stockcount[i] as u32);
                    }
                }
            }
            inv
        })
    }

    /// Retrieves an existing shared (global) inventory without creating one.
    ///
    /// # Arguments
    ///
    /// * `id` - The shared inventory identifier.
    ///
    /// # Returns
    ///
    /// `Some(&mut Inventory)` if the inventory exists, `None` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `FxHashMap::get_mut`
    fn get_shared_inv_mut(&mut self, id: u16) -> Option<&mut Inventory> {
        self.invs.get_mut(&id)
    }

    /// Looks up a player by their slot index (immutable).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    fn get_player(&self, pid: u16) -> Option<&impl ScriptPlayer> {
        self.player_list.get(pid)
    }

    fn get_player_mut(&mut self, pid: u16) -> Option<&mut impl ScriptPlayer> {
        self.player_list.get_mut(pid)
    }

    fn get_npc(&self, nid: u16) -> Option<&impl ScriptNpc> {
        self.npc_list.get(nid)
    }

    fn get_npc_mut(&mut self, nid: u16) -> Option<&mut impl ScriptNpc> {
        self.npc_list.get_mut(nid)
    }

    /// Finds a player by their Base37-encoded username.
    ///
    /// # Arguments
    ///
    /// * `user37` - The Base37-encoded username to search for.
    ///
    /// # Returns
    ///
    /// `Some(PlayerUid)` if a matching player is online, `None` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::find_pid_by_user37`
    fn find_player_by_user37(&self, user37: u64) -> Option<PlayerUid> {
        self.find_pid_by_user37(user37)
            .and_then(|pid| self.player_list.get(pid).map(|p| p.player.uid))
    }

    /// Returns all NPCs present in the specified zone.
    ///
    /// Iterates the zone's NPC slot list and resolves each to an `NpcRef`
    /// containing the slot index, current type ID, and packed coordinate.
    ///
    /// # Arguments
    ///
    /// * `x` - The zone X coordinate.
    /// * `y` - The zone level (height plane).
    /// * `z` - The zone Z coordinate.
    ///
    /// # Returns
    ///
    /// A `Vec<NpcRef>` of NPC references in the zone.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ZoneMap::zone`, `ActiveNpc::current_type`, `CoordGrid::packed`
    fn get_zone_npcs(&self, x: u16, y: u8, z: u16) -> Vec<NpcRef> {
        let Some(zone) = self.zones.zone(x, y, z) else {
            return Vec::new();
        };
        zone.npcs
            .iter()
            .filter_map(|&nid| {
                let active = self.npc_list.get(nid)?;
                Some(NpcRef {
                    nid,
                    id: active.npc.uid.id(),
                    coord: active.npc.pathing.coord.packed(),
                })
            })
            .collect()
    }

    /// Returns the packed coordinates of all players in the specified zone.
    ///
    /// # Arguments
    ///
    /// * `x` - The zone X coordinate.
    /// * `y` - The zone level (height plane).
    /// * `z` - The zone Z coordinate.
    ///
    /// # Returns
    ///
    /// A `Vec<u32>` of packed `CoordGrid` values for each player in the zone.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ZoneMap::zone`, `CoordGrid::packed`
    fn get_zone_player_coords(&self, x: u16, y: u8, z: u16) -> Vec<u32> {
        let Some(zone) = self.zones.zone(x, y, z) else {
            return Vec::new();
        };
        zone.players
            .iter()
            .filter_map(|&pid| {
                let active = self.player_list.get(pid)?;
                Some(active.player.pathing.coord.packed())
            })
            .collect()
    }

    /// Returns the player slot indices for all players in the specified zone.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ZoneMap::zone`
    fn get_zone_player_pids(&self, x: u16, y: u8, z: u16) -> &[u16] {
        self.zones
            .zone(x, y, z)
            .map(|z| z.players.as_slice())
            .unwrap_or(&[])
    }

    /// Spawns a new NPC at the given coordinate with a despawn lifecycle.
    ///
    /// Creates an `ActiveNpc` with `EntityLifeTime::Despawn`, allocates a free NPC slot,
    /// and returns the unique identifier.
    ///
    /// # Arguments
    ///
    /// * `coord` - The packed coordinate where the NPC should appear.
    /// * `id` - The NPC type identifier.
    /// * `_duration` - Unused (reserved for future lifetime control).
    ///
    /// # Returns
    ///
    /// `Some(NpcUid)` with the unique identifier, or `None` if no free slot is available.
    ///
    /// # Side Effects
    ///
    /// Adds the NPC to the world, collision map, and zone tracking.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ActiveNpc::new`, `Engine::add_npc`
    fn add_npc_spawned(&mut self, coord: u32, id: u16, _duration: u64) -> Option<NpcUid> {
        let npc_type = self.cache.npcs.get_by_id(id)?;
        let coord = CoordGrid::from(coord);
        let vars = VarSet::new(self.cache.varns.types.iter().map(|v| v.var_type));
        let mut active = ActiveNpc::new(id, 0, coord, npc_type.size, vars, self.cache);
        active.npc.lifecycle = EntityLifeTime::Despawn;
        self.add_npc(active)
    }

    /// Removes an NPC from the world by its slot index.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::deactivate_npc`
    fn remove_npc(&mut self, nid: u16) {
        self.deactivate_npc(nid);
    }

    /// Adds a ground object at the given coordinate with a despawn lifecycle.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Obj::new`, `Engine::add_obj` (inherent)
    fn add_obj(&mut self, coord: u32, id: u16, count: u32, receiver37: Option<u64>, duration: u64) {
        let coord = CoordGrid::from(coord);
        self.add_obj(
            coord,
            Obj::new(coord, EntityLifeTime::Despawn, id, count),
            receiver37,
            duration,
        );
    }

    /// Enqueues a ground object to be spawned after a delay.
    ///
    /// The request is placed on the `obj_delayed_queue` and processed during the world phase.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `LinkList::add_tail`
    fn add_obj_delayed(
        &mut self,
        coord: u32,
        id: u16,
        count: u32,
        receiver37: Option<u64>,
        duration: u64,
        delay: u64,
    ) {
        self.obj_delayed_queue.add_tail(ObjDelayedRequest {
            coord,
            id,
            count,
            receiver37,
            duration,
            delay,
        });
    }

    /// Removes a ground object from the given coordinate.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::remove_obj` (inherent)
    fn remove_obj(&mut self, coord: u32, id: u16, receiver37: Option<u64>, duration: u64) {
        self.remove_obj(CoordGrid::from(coord), id, receiver37, duration);
    }

    /// Finds a ground object at the given coordinate by type ID and optional
    /// receiver ownership.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ZoneMap::zone`, `Zone::get_obj`
    fn find_obj(&self, coord: u32, id: u16, receiver37: Option<u64>) -> Option<ObjRef> {
        let coord = CoordGrid::from(coord);
        let zone = self.zones.zone(coord.x(), coord.y(), coord.z())?;
        let idx = zone.get_obj(coord.x(), coord.z(), id, receiver37)?;
        let obj = &zone.objs[idx];
        Some(ObjRef {
            coord: obj.world_coord(zone.coord).packed(),
            id: obj.id(),
            count: obj.count(),
        })
    }

    /// Returns all ground objects present in the specified zone.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ZoneMap::zone`
    fn get_zone_objs(&self, x: u16, y: u8, z: u16) -> Vec<ObjRef> {
        let Some(zone) = self.zones.zone(x, y, z) else {
            return Vec::new();
        };
        zone.objs
            .iter()
            .map(|obj| ObjRef {
                coord: obj.world_coord(zone.coord).packed(),
                id: obj.id(),
                count: obj.count(),
            })
            .collect()
    }

    /// Returns all visible locations in the specified zone.
    ///
    /// Filters out hidden (removed/reverted) locations and maps each to a `LocRef`.
    ///
    /// # Arguments
    ///
    /// * `x` - The zone X coordinate.
    /// * `y` - The zone level (height plane).
    /// * `z` - The zone Z coordinate.
    ///
    /// # Returns
    ///
    /// A `Vec<LocRef>` of location references in the zone.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ZoneMap::zone`, `Loc::visible`, `CoordGrid::packed`
    fn get_zone_locs(&self, x: u16, y: u8, z: u16) -> Vec<LocRef> {
        let Some(zone) = self.zones.zone(x, y, z) else {
            debug_assert!(false, "Zone not found at coord: x={}, y={}, z={}", x, y, z);
            return Vec::new();
        };
        zone.locs
            .iter()
            .filter(|loc| loc.visible())
            .map(|loc| LocRef {
                coord: loc.world_coord(zone.coord).packed(),
                id: loc.id(),
                shape: loc.shape() as u8,
                angle: loc.angle() as u8,
                layer: loc.layer() as u8,
            })
            .collect()
    }

    /// Finds a specific location at the given tile by type ID.
    ///
    /// # Arguments
    ///
    /// * `x` - The tile X coordinate.
    /// * `z` - The tile Z coordinate.
    /// * `y` - The level (height plane).
    /// * `id` - The location type identifier.
    ///
    /// # Returns
    ///
    /// `Some(LocRef)` if a matching location exists, `None` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ZoneMap::zone`, `Zone::get_loc`
    fn find_loc(&self, x: u16, z: u16, y: u8, id: u16) -> Option<LocRef> {
        let zone = self.zones.zone(x, y, z)?;
        let idx = zone.get_loc(x, z, id)?;
        let loc = &zone.locs[idx];
        Some(LocRef {
            coord: loc.world_coord(zone.coord).packed(),
            id: loc.id(),
            shape: loc.shape() as u8,
            angle: loc.angle() as u8,
            layer: loc.layer() as u8,
        })
    }

    /// Adds a new location or changes an existing one at the given coordinate.
    ///
    /// Transmutes raw `u8` shape and angle values to their enum representations
    /// and delegates to the inherent `Engine::add_or_change_loc`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::add_or_change_loc` (inherent)
    fn add_or_change_loc(
        &mut self,
        coord: u32,
        id: u16,
        shape: u8,
        angle: u8,
        duration: u64,
        create_if_missing: bool,
    ) {
        let shape = unsafe { std::mem::transmute::<u8, LocShape>(shape) };
        let angle = unsafe { std::mem::transmute::<u8, LocAngle>(angle) };
        self.add_or_change_loc(
            CoordGrid::from(coord),
            id,
            shape,
            angle,
            duration,
            create_if_missing,
        );
    }

    /// Merges a location so that it is only visible to one player within a bounded area.
    ///
    /// Finds the existing location at the given coordinate and layer, then queues
    /// a `LocMerge` zone message with relative boundary offsets.
    ///
    /// # Side Effects
    ///
    /// Queues a zone message and marks the zone as dirty.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::merge_loc`, `Engine::track_zone`
    fn merge_loc(
        &mut self,
        coord: u32,
        shape: u8,
        angle: u8,
        id: u16,
        start: u16,
        end: u16,
        pid: u16,
        south: u16,
        east: u16,
        north: u16,
        west: u16,
    ) {
        let shape = unsafe { std::mem::transmute::<u8, LocShape>(shape) };
        let angle = unsafe { std::mem::transmute::<u8, LocAngle>(angle) };
        let layer = shape.layer();

        let coord = CoordGrid::from(coord);
        let (x, y, z) = (coord.x(), coord.y(), coord.z());

        let existing = self
            .zones
            .zone_mut(x, y, z)
            .locs
            .iter()
            .position(|l| l.is_at(x, z) && l.layer() == layer);

        if let Some(idx) = existing {
            let coord = CoordGrid::packed_zone_coord(x, z);
            let shape_angle = ((shape as u8) << 2) | (angle as u8 & 3);
            let message =
                ZoneMessage::LocMerge(rs_protocol::network::game::server::loc_merge::LocMerge {
                    coord,
                    shape_angle,
                    id,
                    start,
                    end,
                    pid,
                    east: east.wrapping_sub(x) as i8,
                    south: south.wrapping_sub(z) as i8,
                    west: west.wrapping_sub(x) as i8,
                    north: north.wrapping_sub(z) as i8,
                });
            self.zones.zone_mut(x, y, z).merge_loc(idx, message);
            self.track_zone(x, y, z);
        }
    }

    /// Removes a location from the given coordinate and collision layer.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::remove_loc` (inherent)
    fn remove_loc(&mut self, coord: u32, layer: u8, duration: u64) {
        let layer = unsafe { std::mem::transmute::<u8, LocLayer>(layer) };
        self.remove_loc(CoordGrid::from(coord), layer, duration);
    }

    /// Plays a sequence animation on a location.
    ///
    /// Looks up the location in the zone by type ID and queues the animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::get_loc`, `Zone::anim_loc`, `Engine::track_zone`
    fn anim_loc(&mut self, coord: u32, id: u16, seq: u16) {
        let coord = CoordGrid::from(coord);
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        let zone = self.zones.zone_mut(x, y, z);
        if let Some(idx) = zone.get_loc(x, z, id) {
            zone.anim_loc(idx, seq);
            self.track_zone(x, y, z);
        }
    }

    /// Creates a projectile animation between two map positions.
    ///
    /// Computes relative deltas from source to destination and queues a
    /// `MapProjAnim` zone message.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::map_proj_anim`, `Engine::track_zone`
    fn map_proj_anim(
        &mut self,
        y: u8,
        x: u16,
        z: u16,
        dst_x: u16,
        dst_z: u16,
        target: i16,
        id: u16,
        src_height: u8,
        dst_height: u8,
        start: u16,
        end: u16,
        peak: u8,
        arc: u8,
    ) {
        let coord = CoordGrid::packed_zone_coord(x, z);
        let dx = (dst_x as i32 - x as i32) as i8;
        let dz = (dst_z as i32 - z as i32) as i8;
        let message = ZoneMessage::MapProjAnim(
            rs_protocol::network::game::server::map_projanim::MapProjAnim {
                coord,
                dx,
                dz,
                target,
                spotanim: id,
                src_height,
                dst_height,
                start_delay: start,
                end_delay: end,
                peak,
                arc,
            },
        );
        self.zones.zone_mut(x, y, z).map_proj_anim(message);
        self.track_zone(x, y, z);
    }

    /// Plays a spot animation (graphic) on the map at the given tile.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::anim_map`, `Engine::track_zone`
    fn anim_map(&mut self, y: u8, x: u16, z: u16, spotanim: u16, height: u8, delay: u16) {
        let coord = CoordGrid::packed_zone_coord(x, z);
        let message = ZoneMessage::MapAnim(rs_protocol::network::game::server::map_anim::MapAnim {
            coord,
            spotanim,
            height,
            delay,
        });
        self.zones.zone_mut(x, y, z).anim_map(message);
        self.track_zone(x, y, z);
    }

    /// Checks whether adding a location at the given coordinate is unsafe, i.e.
    /// an active loc already occupies the tile. Invisible wall locs are skipped.
    ///
    /// # Arguments
    ///
    /// * `coord` - The coordinate to test.
    ///
    /// # Returns
    ///
    /// `true` if the coordinate is occupied by an active location.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::loc_occupies`
    fn locaddunsafe(&self, coord: CoordGrid) -> bool {
        self.loc_occupies(coord, true)
    }

    /// Checks whether an active loc occupies the given coordinate. Unlike
    /// [`locaddunsafe`](Self::locaddunsafe), invisible wall locs still count.
    ///
    /// # Arguments
    ///
    /// * `coord` - The coordinate to test.
    ///
    /// # Returns
    ///
    /// `true` if an active loc covers the coordinate.
    #[cfg(since_274)]
    fn map_loc(&self, coord: CoordGrid) -> bool {
        self.loc_occupies(coord, false)
    }

    /// Returns a mutable reference to the engine's random number generator.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** reads `self.random`
    fn random(&mut self) -> &mut JavaRandom {
        &mut self.random
    }

    /// Indicates whether the server is running in members mode.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** reads `self.members`
    fn members(&self) -> bool {
        self.members
    }

    /// Indicates if there is "line of sight" between these coords.
    ///
    /// # Call Stack
    ///
    /// **Calls:** reads `rsmod::has_line_of_sight()`
    fn lineofsight(&self, src: CoordGrid, dst: CoordGrid) -> bool {
        rsmod::has_line_of_sight(src.y(), src.x(), src.z(), dst.x(), dst.z(), 1, 1, 1, 1, 0)
    }

    /// Indicates if there is "line of walk" between these coords.
    ///
    /// # Call Stack
    ///
    /// **Calls:** reads `rsmod::has_line_of_walk()`
    fn lineofwalk(&self, src: CoordGrid, dst: CoordGrid) -> bool {
        rsmod::has_line_of_walk(src.y(), src.x(), src.z(), dst.x(), dst.z(), 1, 1, 1, 1, 0)
    }

    /// Indicates if this coord has a `CollisionFlag::WalkBlocked` on it.
    ///
    /// # Call Stack
    ///
    /// **Calls:** reads `rsmod::is_flagged()`
    fn map_blocked(&self, coord: CoordGrid) -> bool {
        rsmod::is_flagged(
            coord.x(),
            coord.z(),
            coord.y(),
            CollisionFlag::WalkBlocked as u32,
        )
    }

    /// Indicates if this coord has a `CollisionFlag::Roof` collision flag on it.
    ///
    /// # Call Stack
    ///
    /// **Calls:** reads `rsmod::is_flagged()`
    fn map_indoors(&self, coord: CoordGrid) -> bool {
        rsmod::is_flagged(coord.x(), coord.z(), coord.y(), CollisionFlag::Roof as u32)
    }

    /// Reads a shared variable (vars) by its definition ID.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `VarSet::get`
    fn get_var(&self, id: u16) -> VarValue {
        self.vars.get(id).clone()
    }

    /// Writes a shared variable (vars).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `VarSet::set`
    fn set_var(&mut self, id: u16, value: VarValue) {
        self.vars.set(id, value.clone());
    }
}

// -----------------------------------------------------------------------
// ScriptPlayer / ScriptNpc implementations
// -----------------------------------------------------------------------

impl ScriptPlayer for ActivePlayer {
    /// Returns this player's unique identifier.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.uid`
    fn uid(&self) -> PlayerUid {
        self.player.uid
    }

    /// Returns the player's current coordinate as a packed `u32`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `CoordGrid::packed`
    fn coord(&self) -> u32 {
        self.player.pathing.coord.packed()
    }

    /// Returns the component ID of the last interface button clicked, or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::map`, `Option::unwrap_or`
    fn last_com(&self) -> i32 {
        self.player.last_com.map(|v| v as i32).unwrap_or(-1)
    }

    /// Returns the inventory slot index from the last interaction, or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::map`, `Option::unwrap_or`
    fn last_slot(&self) -> i32 {
        self.player.last_slot.map(|v| v as i32).unwrap_or(-1)
    }

    /// Returns the use-slot index from an item-on-X interaction, or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::map`, `Option::unwrap_or`
    fn last_useslot(&self) -> i32 {
        self.player.last_use_slot.map(|v| v as i32).unwrap_or(-1)
    }

    /// Returns the target slot index from an item-on-item interaction, or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::map`, `Option::unwrap_or`
    fn last_targetslot(&self) -> i32 {
        self.player.last_target_slot.map(|v| v as i32).unwrap_or(-1)
    }

    /// Returns the item type ID from the last interaction, or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::map`, `Option::unwrap_or`
    fn last_item(&self) -> i32 {
        self.player.last_item.map(|v| v as i32).unwrap_or(-1)
    }

    /// Returns the use-item type ID from an item-on-X interaction, or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::map`, `Option::unwrap_or`
    fn last_useitem(&self) -> i32 {
        self.player.last_use_item.map(|v| v as i32).unwrap_or(-1)
    }

    /// Indicates whether the player's client is running in low-memory mode.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.low_memory`
    fn lowmem(&self) -> bool {
        self.player.low_memory
    }

    /// Indicates whether the player has members status.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.is_member`
    fn member(&self) -> bool {
        self.player.is_member
    }

    /// Returns the player's staff moderator level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.staff_mod_level`
    fn staffmodlevel(&self) -> u8 {
        self.player.staff_mod_level as u8
    }

    /// Checks whether the player has access rights for privileged operations.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::can_access`
    fn can_access(&self) -> bool {
        self.player.can_access()
    }

    /// Indicates whether the player is currently busy.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::busy`
    fn busy(&self) -> bool {
        self.player.busy()
    }

    /// Indicates whether the player has initiated a logout.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.logout_sent`
    fn logging_out(&self) -> bool {
        self.player.logout_sent
    }

    /// Checks whether the player currently has an active interaction target.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::has_interaction`
    fn has_interaction(&self) -> bool {
        self.player.has_interaction()
    }

    /// Checks whether the player has pending movement waypoints.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Pathing::has_waypoints`
    fn has_waypoints(&self) -> bool {
        self.player.pathing.has_waypoints()
    }

    /// Reads a player variable (varp) by its definition ID.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `VarSet::get`
    fn get_var(&self, id: u16) -> VarValue {
        self.player.vars.get(id).clone()
    }

    /// Writes a player variable (varp) and optionally transmits the change to the client.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::set_varp`
    fn set_var(&mut self, id: u16, value: VarValue, transmit: bool) {
        self.set_varp(id, value, transmit);
    }

    /// Consumes and returns the pending AFK event flag.
    ///
    /// Returns `true` if the player triggered an AFK event since the last check;
    /// the flag is cleared after this call.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads and resets `self.player.afk_event_ready`
    fn afk_event(&mut self) -> bool {
        let ready = self.player.afk_event_ready;
        self.player.afk_event_ready = false;
        ready
    }

    /// Sets whether the player may open the character-design (appearance) screen.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** writes `self.player.allow_design`
    fn set_allow_design(&mut self, allow: bool) {
        self.player.allow_design = allow;
    }

    /// Returns the player's current run energy.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.runenergy`
    fn runenergy(&self) -> u16 {
        self.player.runenergy
    }

    /// Adds run energy to the player, clamped to `0..=10000`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** writes `self.player.runenergy`
    fn healenergy(&mut self, amount: i32) {
        self.player.runenergy = (self.player.runenergy as i32 + amount).clamp(0, 10000) as u16;
    }

    /// Returns the player's current carried weight.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.runweight`
    fn weight(&self) -> i32 {
        self.player.runweight
    }

    /// Reports whether the buffered-packet queue has reached the 5000-byte soft
    /// bandwidth limit.
    ///
    /// Each queued [`Packet`]'s backing buffer is exactly its on-wire size
    /// (opcode byte + frame header + payload), so summing their lengths yields
    /// the total bytes pending for this player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sums `Packet::len` over `self.buffered`
    fn buffer_full(&self) -> bool {
        self.buffered.iter().map(|p| p.len()).sum::<usize>() >= 5000
    }

    /// Makes the player say a message as overhead forced chat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.say` and the `PlayerInfoProt::Say` mask
    fn say(&mut self, msg: &str) {
        self.player.info.say = Some(msg.into());
        self.player.info.masks |= PlayerInfoProt::Say as u16;
    }

    /// Sends the last-login info packet (welcome screen).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::last_login_info`
    fn last_login_info(&mut self) {
        self.last_login_info();
    }

    /// Returns the player's current (boosted/drained) level in the given stat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.stat_block.levels[stat]`
    fn stat(&self, stat: usize) -> u16 {
        self.player.stats.level(stat)
    }

    /// Returns the player's base (unboosted) level in the given stat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.stat_block.base_levels[stat]`
    fn stat_base(&self, stat: usize) -> u16 {
        self.player.stats.base_level(stat)
    }

    /// Returns the sum of all base stat levels.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    fn stat_total(&self) -> i32 {
        self.player.stats.total()
    }

    /// Awards experience in a stat, recalculating levels and combat if needed.
    ///
    /// Delegates XP math to [`StatBlock::add_xp`]. If the base level increases
    /// (a level-up), triggers `change_stat`, enqueues the `AdvanceStat` script
    /// for the stat, and rebuilds the player's appearance if their combat level
    /// changes.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `StatBlock::add_xp`, `change_stat`, `buildappearance`, `get_combat_level`
    fn add_xp(&mut self, stat: usize, xp: i32) {
        if !self.player.stats.add_xp(stat, xp) {
            return;
        }
        self.change_stat(stat);
        if let Some(script) =
            engine().script_by_key(ServerTriggerType::AdvanceStat, Some(stat as u16), None)
        {
            let _ = self
                .player
                .state
                .queues
                .add(QueuePriority::Engine, script.id, 0, None);
        }
        self.recalc_combat_and_appearance();
    }

    /// Raises a stat's current level via [`StatBlock::add`].
    ///
    /// Clears hero points if Hitpoints reaches or exceeds the base level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `StatBlock::add`, `change_stat`
    fn stat_add(&mut self, stat: usize, constant: i32, percent: i32) {
        let prev = self.player.stats.level(stat);
        self.player.stats.add(stat, constant, percent);
        if stat == PlayerStat::Hitpoints as usize
            && self.player.stats.level(PlayerStat::Hitpoints as usize)
                >= self.player.stats.base_level(PlayerStat::Hitpoints as usize)
        {
            self.player.hero_points.clear();
        }
        if self.player.stats.level(stat) != prev {
            self.change_stat(stat);
        }
    }

    /// Boosts a stat's current level via [`StatBlock::boost`].
    ///
    /// Clears hero points if Hitpoints reaches or exceeds the base level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `StatBlock::boost`, `change_stat`
    fn stat_boost(&mut self, stat: usize, constant: i32, percent: i32) {
        let prev = self.player.stats.level(stat);
        self.player.stats.boost(stat, constant, percent);
        if stat == PlayerStat::Hitpoints as usize
            && self.player.stats.level(PlayerStat::Hitpoints as usize)
                >= self.player.stats.base_level(PlayerStat::Hitpoints as usize)
        {
            self.player.hero_points.clear();
        }
        if self.player.stats.level(stat) != prev {
            self.change_stat(stat);
        }
    }

    /// Heals a stat's current level via [`StatBlock::heal`].
    ///
    /// Clears hero points if Hitpoints reaches or exceeds the base level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `StatBlock::heal`, `change_stat`
    fn stat_heal(&mut self, stat: usize, constant: i32, percent: i32) {
        let prev = self.player.stats.level(stat);
        self.player.stats.heal(stat, constant, percent);
        if stat == PlayerStat::Hitpoints as usize
            && self.player.stats.level(PlayerStat::Hitpoints as usize)
                >= self.player.stats.base_level(PlayerStat::Hitpoints as usize)
        {
            self.player.hero_points.clear();
        }
        if self.player.stats.level(stat) != prev {
            self.change_stat(stat);
        }
    }

    /// Lowers a stat's current level via [`StatBlock::sub`].
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `StatBlock::sub`, `change_stat`
    fn stat_sub(&mut self, stat: usize, constant: i32, percent: i32) {
        let prev = self.player.stats.level(stat);
        self.player.stats.sub(stat, constant, percent);
        if self.player.stats.level(stat) != prev {
            self.change_stat(stat);
        }
    }

    /// Drains a stat's current level via [`StatBlock::drain`].
    ///
    /// Unlike `stat_sub`, the percentage is based on the current level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `StatBlock::drain`, `change_stat`
    fn stat_drain(&mut self, stat: usize, constant: i32, percent: i32) {
        let prev = self.player.stats.level(stat);
        self.player.stats.drain(stat, constant, percent);
        if self.player.stats.level(stat) != prev {
            self.change_stat(stat);
        }
    }

    /// Enqueues the `ChangeStat` trigger script for the given stat.
    ///
    /// Looks up the trigger script by stat index and adds it to the player's
    /// engine-priority queue for deferred execution.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `add_xp`, `stat_add`, `stat_boost`, `stat_heal`, `stat_sub`, `stat_drain`
    /// **Calls:** `engine().trigger_lookup_key`, `QueueSet::add`
    fn change_stat(&mut self, stat: usize) {
        if let Some(script) =
            engine().script_by_key(ServerTriggerType::ChangeStat, Some(stat as u16), None)
        {
            let _ = self
                .player
                .state
                .queues
                .add(QueuePriority::Engine, script.id, 0, None);
        }
    }

    /// Awards hero points to a player for contributing damage.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `HeroPoints::add_hero`
    fn heropoints(&mut self, user37: u64, points: i32) {
        self.player.hero_points.add_hero(user37, points);
    }

    /// Applies damage to the player and displays a hitsplat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::damage`
    fn damage(&mut self, amount: u8, damage_type: u8) {
        self.damage(amount, damage_type);
    }

    /// Finds the player with the most hero points on this entity.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `HeroPoints::find_hero`
    fn findhero(&self) -> Option<u64> {
        self.player.hero_points.find_hero()
    }

    /// Sets whether the player's animation is protected from being overridden.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.anim_protect`
    fn animprotect(&mut self, protect: bool) {
        self.player.info.anim_protect = protect;
    }

    /// Sets the player's ready (idle/stand) animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.readyanim`
    fn readyanim(&mut self, id: u16) {
        self.player.info.readyanim = Some(id);
    }

    /// Sets the player's turn-on-spot animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.turnanim`
    fn turnanim(&mut self, id: u16) {
        self.player.info.turnanim = Some(id);
    }

    /// Opens a tutorial interface and tracks it as the active tutorial modal.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::open_tutorial`
    fn tut_open(&mut self, com: u16) {
        self.open_tutorial(com);
    }

    /// Flashes a tutorial tab to draw the player's attention.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::tut_flash`
    fn tut_flash(&mut self, tab: u8) {
        self.tut_flash(tab);
    }

    /// Closes the active tutorial interface, firing its `IfClose` trigger.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::close_tutorial`
    fn tut_close(&mut self) -> rs_vm::Result<()> {
        self.close_tutorial()
    }

    /// Sets the player's forward walk animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.walkanim`
    fn walkanim(&mut self, id: u16) {
        self.player.info.walkanim = Some(id);
    }

    /// Sets the player's backward walk animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.walkanim_b`
    fn walkanim_b(&mut self, id: u16) {
        self.player.info.walkanim_b = Some(id);
    }
    /// Sets the player's left strafe walk animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.walkanim_l`
    fn walkanim_l(&mut self, id: u16) {
        self.player.info.walkanim_l = Some(id);
    }

    /// Sets the player's right strafe walk animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.walkanim_r`
    fn walkanim_r(&mut self, id: u16) {
        self.player.info.walkanim_r = Some(id);
    }

    /// Sets the player's run animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.runanim`
    fn runanim(&mut self, id: u16) {
        self.player.info.runanim = Some(id);
    }

    /// Configures which interface buttons are valid resume targets for a paused script dialog.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.resume_buttons`
    #[cfg(before_254)]
    fn if_setresumebuttons(&mut self, buttons: Option<Vec<i32>>) {
        self.player.resume_buttons = buttons;
    }

    /// Appends a single interface button to the set of valid resume targets for
    /// a paused script dialog, lazily allocating the set on first use.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** pushes onto `self.player.resume_buttons`
    #[cfg(since_254)]
    fn if_addresumebutton(&mut self, button: i32) {
        self.player
            .resume_buttons
            .get_or_insert_with(Vec::new)
            .push(button);
    }

    /// Initiates a player logout.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.logout_requested`
    fn logout(&mut self, requested: bool) {
        self.player.logout_requested = requested;
    }

    /// Prevents the player from logging out until the specified tick.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.logout_prevented_message` and `logout_prevented_until`
    fn prevent_logout(&mut self, message: &str, until: u32) {
        self.player.logout_prevented_message = Some(message.into());
        self.player.logout_prevented_until = Some(until);
    }

    /// Sends a game message to the player's chatbox.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::message_game`
    fn mes(&mut self, msg: &str) {
        self.message_game(msg);
    }

    /// Sends a game message to the player's chatbox with automatic line wrapping.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::message_game_wrapped`
    fn message_game_wrapped(&mut self, msg: &str) {
        self.message_game_wrapped(msg);
    }

    /// Plays an animation on the player's character model.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::anim`
    fn anim(&mut self, id: Option<u16>, delay: u8) {
        self.anim(id, delay);
    }

    /// Rebuilds the player's appearance from their worn equipment inventory.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::buildappearance`
    fn buildappearance(&mut self, inv: u16) {
        self.buildappearance(inv);
    }

    /// Sets the player's skin color (appearance color slot 4).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** writes `self.player.colours[4]`
    fn setskincolour(&mut self, colour: u8) {
        self.player.colours[4] = colour;
    }

    /// Sets the player's color for a specified idk slot.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** writes `self.player.colours[slot] = colour;`
    fn setidkcolour(&mut self, slot: u8, colour: u8) -> rs_vm::Result<()> {
        if slot as usize >= self.player.colours.len() {
            return Err(ScriptError::Runtime(format!("Invalid idk slot: {}", slot)));
        }
        self.player.colours[slot as usize] = colour;
        Ok(())
    }

    /// Sets an identity-kit body part and its color from the character design.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** writes `self.player.body` and `self.player.colours`
    fn setidkit(&mut self, idk_type: u8, idk_id: u16, colour: u8) {
        // Female idks occupy body types 7-13; map them onto the 7 body slots.
        let slot = if self.player.gender == 1 {
            idk_type as i32 - 7
        } else {
            idk_type as i32
        };
        if !(0..7).contains(&slot) {
            return;
        }
        let slot = slot as usize;
        self.player.body[slot] = idk_id as i32;
        // hair/jaw -> 0, torso/arms -> 1, legs -> 2, feet -> 3; hands (4) keep skin.
        let colour_slot = match slot {
            0 | 1 => Some(0),
            2 | 3 => Some(1),
            5 => Some(2),
            6 => Some(3),
            _ => None,
        };
        if let Some(cs) = colour_slot {
            self.player.colours[cs] = colour;
        }
    }

    /// Sets the player's gender, remapping every appearance body part between
    /// the male and female identity-kit sets.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** writes `self.player.body` and `self.player.gender`
    fn setgender(&mut self, gender: u8) {
        fn male_to_female(id: i32) -> i32 {
            match id {
                0 => 45,
                1 => 47,
                2 => 48,
                3 => 49,
                4 => 50,
                5 => 51,
                6 => 52,
                7 => 53,
                8 => 54,
                9 => 55,
                18..=25 => 56,
                26 => 61,
                27 | 31 => 63,
                28 => 62,
                29 => 65,
                30 => 64,
                32 => 66,
                33 => 67,
                34 => 68,
                35 => 69,
                36 => 70,
                37 => 71,
                38 => 72,
                39 => 76,
                40 => 75,
                41 => 78,
                42 => 79,
                43 => 80,
                44 => 81,
                _ => -1,
            }
        }
        fn female_to_male(id: i32) -> i32 {
            match id {
                45 | 46 => 0,
                47 => 1,
                48 => 2,
                49 => 3,
                50 => 4,
                51 => 5,
                52 => 6,
                53 => 7,
                54 => 8,
                55 => 9,
                56..=60 => 18,
                61 => 26,
                62 => 27,
                63 => 28,
                64 | 65 => 29,
                66 => 32,
                67 => 33,
                68 => 34,
                69 => 35,
                70 | 73 | 74 | 77 => 36,
                71 => 37,
                72 => 38,
                75 => 40,
                76 => 39,
                78 => 41,
                79 => 42,
                80 => 43,
                81 => 44,
                _ => -1,
            }
        }
        for i in 0..7 {
            if gender == 1 {
                self.player.body[i] = male_to_female(self.player.body[i]);
            } else if i == 1 {
                // Switching to male resets the jaw/beard slot to the default.
                self.player.body[i] = 14;
            } else {
                self.player.body[i] = female_to_male(self.player.body[i]);
            }
        }
        self.player.gender = gender;
    }

    /// Resets the player's camera to its default position and orientation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::cam_reset`
    fn cam_reset(&mut self) {
        self.cam_reset();
    }

    /// Sends a hint arrow pointing at the NPC with the given world index.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::hint_npc`
    fn hint_npc(&mut self, nid: u16) {
        self.hint_npc(nid);
    }

    /// Sends a hint arrow hovering over the given tile.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::hint_tile`
    fn hint_tile(&mut self, offset: u8, x: u16, z: u16, height: u8) {
        self.hint_tile(offset, x, z, height);
    }

    /// Sends a hint arrow pointing at the player with the given index.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::hint_player`
    fn hint_player(&mut self, slot: u16) {
        self.hint_player(slot);
    }

    /// Clears any active hint arrow on the player's client.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::stop_hint`
    fn stop_hint(&mut self) {
        self.stop_hint();
    }

    /// Returns the player's gender (`0` = male, `1` = female).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.gender`
    fn gender(&self) -> u8 {
        self.player.gender
    }

    /// Returns the currently set walk trigger script ID, or `-1` if unset.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::unwrap_or`
    fn getwalktrigger(&self) -> i32 {
        self.player.walktrigger.unwrap_or(-1)
    }

    /// Sets the walk trigger script ID that fires when the player moves.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.walktrigger`
    fn walktrigger(&mut self, trigger: i32) {
        self.player.walktrigger = if trigger == -1 { None } else { Some(trigger) };
    }

    /// Returns the player's current overhead head icon bitfield.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.headicons`
    fn headicons_get(&self) -> u8 {
        self.player.headicons
    }

    /// Sets the player's overhead head icon bitfield.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.headicons`
    fn headicons_set(&mut self, headicons: u8) {
        self.player.headicons = headicons;
    }

    /// Closes any open modal interface.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::close_modal`
    fn if_close(&mut self, clear: bool) -> rs_vm::Result<()> {
        self.close_modal(clear)
    }

    /// Opens a chatbox interface (e.g. a dialog).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::open_chat_modal`
    fn if_openchat(&mut self, id: u16) {
        self.open_chat_modal(id);
    }

    /// Opens a main-area interface alongside a side-panel interface.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::open_main_side_modal`
    fn if_openmain_side(&mut self, com: u16, side: u16) {
        self.open_main_side_modal(com, side);
    }

    /// Opens a full-screen main-area interface.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::open_main_modal`
    fn if_openmain(&mut self, com: u16) {
        self.open_main_modal(com);
    }

    /// Opens a side-panel interface.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::open_side_modal`
    fn if_openside(&mut self, com: u16) {
        self.open_side_modal(com);
    }

    /// Sets the animation displayed on an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setanim`
    fn if_setanim(&mut self, com: u16, seq: u16) {
        self.if_setanim(com, seq);
    }

    /// Sets the color of an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setcolour`
    fn if_setcolour(&mut self, com: u16, colour: u16) {
        self.if_setcolour(com, colour);
    }

    /// Shows or hides an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_sethide`
    fn if_sethide(&mut self, com: u16, hide: bool) {
        self.if_sethide(com, hide);
    }

    /// Sets the model displayed on an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setmodel`
    fn if_setmodel(&mut self, com: u16, model: u16) {
        self.if_setmodel(com, model);
    }

    /// Sets an NPC chathead model on an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setnpchead`
    fn if_setnpchead(&mut self, com: u16, npc: u16) {
        self.if_setnpchead(com, npc);
    }

    /// Displays an object model on an interface component at the given zoom level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setobject`
    fn if_setobject(&mut self, com: u16, obj: u16, zoom: u16) {
        self.if_setobject(com, obj, zoom);
    }

    /// Sets the player's own chathead model on an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setplayerhead`
    fn if_setplayerhead(&mut self, com: u16) {
        self.if_setplayerhead(com);
    }

    /// Sets the position of an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setposition`
    fn if_setposition(&mut self, com: u16, x: u16, y: u16) {
        self.if_setposition(com, x, y);
    }

    /// Sets the scrollbar position on an interface component.
    #[cfg(since_245_2)]
    fn if_setscrollpos(&mut self, com: u16, y: u16) {
        self.if_setscrollpos(com, y);
    }

    /// Opens a modal overlay interface component.
    #[cfg(since_244)]
    fn if_openoverlay(&mut self, com: u16) {
        self.if_openoverlay(com);
    }

    /// Sets (or clears) a right-click option on this player's menu.
    #[cfg(since_254)]
    fn set_player_op(&mut self, op: u8, value: &str, primary: u8) {
        self.set_player_op(op, value, primary);
    }

    /// Sets the client minimap state.
    #[cfg(since_274)]
    fn minimap_toggle(&mut self, minimap_type: u8) {
        self.minimap_toggle(minimap_type);
    }

    /// Sets the player's appearance skill level.
    #[cfg(since_274)]
    fn set_skill_level(&mut self, level: u16) {
        self.set_skill_level(level);
    }

    /// Recolors an interface component model.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setrecol`
    #[cfg(before_245_2)]
    fn if_setrecol(&mut self, com: u16, src: u16, dst: u16) {
        self.if_setrecol(com, src, dst);
    }

    /// Assigns an interface component to a tab slot.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::set_tab`
    fn if_settab(&mut self, tab: u16, com: u8) {
        self.set_tab(tab, com);
    }

    /// Switches the client's currently selected (active) tab.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_settabactive`
    fn if_settabactive(&mut self, tab: u8) {
        self.if_settabactive(tab);
    }

    /// Sets the text content of an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_settext`
    fn if_settext(&mut self, com: u16, text: &str) {
        if text.contains("\\n") && text.contains('@') {
            self.if_settext(com, &preserve_settext_colors(text));
        } else {
            self.if_settext(com, text);
        }
    }

    /// Plays a MIDI jingle (short musical effect) for the player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::midi_jingle`
    #[cfg(rev = "225")]
    fn midi_jingle(&mut self, length: u16, data: &[u8]) {
        self.midi_jingle(length, data);
    }

    /// Plays a MIDI jingle (short musical effect) for the player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::midi_jingle`
    #[cfg(since_244)]
    fn midi_jingle(&mut self, id: u16, delay: u16) {
        self.midi_jingle(id, delay);
    }

    /// Starts playing a MIDI song (background music) for the player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::midi_song`
    #[cfg(rev = "225")]
    fn midi_song(&mut self, name: &str, crc: i32, len: i32) {
        self.midi_song(name, crc, len);
    }

    /// Starts playing a MIDI song (background music) for the player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::midi_song`
    #[cfg(since_244)]
    fn midi_song(&mut self, id: u16) {
        self.midi_song(id);
    }

    /// Makes the player face a specific tile.
    ///
    /// Converts tile coordinates to fine coordinates and sets the face direction
    /// info mask for the next player info update.
    ///
    /// # Side Effects
    ///
    /// Sets `face_x`, `face_z`, and the `FaceCoord` info mask.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `CoordGrid::fine`
    fn facesquare(&mut self, x: u16, z: u16) {
        let fine_x = CoordGrid::fine(x, 1);
        let fine_z = CoordGrid::fine(z, 1);
        self.player.info.face_x = Some(fine_x);
        self.player.info.face_z = Some(fine_z);
        self.player.info.masks |= PlayerInfoProt::FaceCoord as u16;
    }

    /// Plays a synthesized sound effect for the player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::synth_sound`
    fn sound_synth(&mut self, synth: u16, loops: u8, delay: u16) {
        self.synth_sound(synth, loops, delay);
    }

    /// Clears the player's pending action, if any.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::clear_pending_action`
    fn clearpendingaction(&mut self) -> rs_vm::Result<()> {
        self.clear_pending_action()
    }

    /// Suspends the player's currently running script for a number of ticks.
    ///
    /// # Side Effects
    ///
    /// Sets the player's `delayed` flag and `delayed_until` tick.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.state.delayed` and `delayed_until`
    fn delay(&mut self, delay: u32) {
        self.player.state.delayed = true;
        self.player.state.delayed_until = delay;
    }

    /// Records an arrive-delay so the script resumes after movement completes.
    ///
    /// If the player's last movement happened before the given clock tick, the
    /// delay is skipped (the player has already arrived).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Self::delay`
    fn arrivedelay(&mut self, clock: u32) -> bool {
        if self.player.pathing.last_movement < clock {
            return false;
        }
        self.delay(clock + 1);
        true
    }

    /// Opens a count-dialog input prompt on the client.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::p_countdialog`
    fn countdialog(&mut self) {
        self.p_countdialog();
    }

    /// Sets the player's run mode (`0` = walk, `1` = run) and syncs the change
    /// to the client's run varp so the run orb reflects the new state.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.run`, `ActivePlayer::sync_run`
    fn run(&mut self, run: u8) {
        self.player.run = run == 1;
        self.sync_run();
    }

    /// Stops the player's current action and clears interaction state.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::stop_action`
    fn stopaction(&mut self) -> rs_vm::Result<()> {
        self.stop_action()
    }

    /// Instantly moves the player to a coordinate without walking.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::tele_jump`
    fn telejump(&mut self, coord: u32) {
        self.tele_jump(CoordGrid::from(coord));
    }

    /// Teleports the player to a coordinate, processing zone transitions.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::tele`
    fn teleport(&mut self, coord: u32) {
        self.tele(CoordGrid::from(coord));
    }

    /// Plays an exact-move animation that linearly interpolates between two positions.
    ///
    /// Sets all exact-move info fields, enables the `ExactMove` info mask, and
    /// updates the player's coordinate to the end position with the tele flag set.
    ///
    /// # Side Effects
    ///
    /// Updates the player's coordinate, sets `tele = true`, and sets the `ExactMove` info mask.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.exactmove_*` fields
    fn exactmove(
        &mut self,
        start_x: u16,
        start_z: u16,
        end_x: u16,
        end_z: u16,
        begin: u16,
        finish: u16,
        direction: u8,
    ) {
        self.teleport(CoordGrid::new(end_x, self.player.pathing.coord.y(), end_z).packed());
        self.player.info.exactmove_start_x = Some(start_x);
        self.player.info.exactmove_start_z = Some(start_z);
        self.player.info.exactmove_end_x = Some(end_x);
        self.player.info.exactmove_end_z = Some(end_z);
        self.player.info.exactmove_begin = Some(begin);
        self.player.info.exactmove_finish = Some(finish);
        self.player.info.exactmove_dir = Some(direction);
        self.player.info.masks |= PlayerInfoProt::ExactMove as u16;
    }

    /// Displays a spot animation (graphic) attached to the player.
    ///
    /// # Side Effects
    ///
    /// Sets the `SpotAnim` info mask for the next player info update.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.spotanim*` fields
    fn spotanim(&mut self, spotanim: u16, height: u16, delay: u16) {
        self.player.info.spotanim = Some(spotanim);
        self.player.info.spotanim_height = Some(height);
        self.player.info.spotanim_delay = Some(delay);
        self.player.info.masks |= PlayerInfoProt::SpotAnim as u16;
    }

    /// Enqueues a script for deferred execution on this player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `QueueSet::add`
    fn queue(
        &mut self,
        script_id: i32,
        priority: QueuePriority,
        delay: u16,
        args: Option<Vec<ScriptArgument>>,
    ) -> rs_vm::Result<()> {
        self.player
            .state
            .queues
            .add(priority, script_id, delay, args)
    }

    /// Sets a recurring timer that fires a script at a fixed interval.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `TimerSet::add`
    fn settimer(
        &mut self,
        script_id: i32,
        priority: TimerPriority,
        interval: u16,
        clock: u32,
        args: Option<Vec<ScriptArgument>>,
    ) {
        self.player
            .state
            .timers
            .add(priority, script_id, interval, clock, args)
    }

    /// Clears the player's timer for the given script, regardless of priority
    /// (both the normal and soft lanes are cleared).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ScriptTimer::remove_any`
    fn cleartimer(&mut self, script_id: i32) {
        self.player.state.timers.remove_any(script_id);
    }

    /// Removes all queued (normal and weak) scripts matching the given script.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ScriptQueue::remove_by_script`
    fn clearqueue(&mut self, script_id: i32) {
        self.player.state.queues.remove_any(script_id);
    }

    /// Counts the player's queued (normal and weak) scripts matching the script.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ScriptQueue::count_by_script`
    fn getqueue(&self, script_id: i32) -> i32 {
        self.player.state.queues.count_by_script(script_id)
    }

    /// Returns the clock of the player's timer for the given script.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ScriptTimer::get`
    fn gettimer(&self, script_id: i32) -> i32 {
        match self.player.state.timers.get(script_id) {
            Some(timer) => timer.clock as i32,
            None => -1,
        }
    }

    fn cam_lookat(
        &mut self,
        x: u16,
        z: u16,
        height: u16,
        rate: u8,
        rate2: u8,
    ) -> rs_vm::Result<()> {
        self.player
            .cam_queue
            .add(CamKind::LookAt, x, z, height, rate, rate2)
    }

    fn cam_moveto(
        &mut self,
        x: u16,
        z: u16,
        height: u16,
        rate: u8,
        rate2: u8,
    ) -> rs_vm::Result<()> {
        self.player
            .cam_queue
            .add(CamKind::MoveTo, x, z, height, rate, rate2)
    }

    fn cam_shake(&mut self, direction: u8, jitter: u8, amplitude: u8, frequency: u8) {
        self.cam_shake(direction, jitter, amplitude, frequency)
    }

    /// Returns an immutable reference to a player inventory.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `FxHashMap::get`
    fn get_inv(&mut self, id: u16) -> Option<&Inventory> {
        self.player.invs.get(&id)
    }
    /// Returns a mutable reference to a player inventory.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `FxHashMap::get_mut`
    fn get_inv_mut(&mut self, id: u16) -> Option<&mut Inventory> {
        self.player.invs.get_mut(&id)
    }

    /// Returns mutable references to two distinct player inventories simultaneously.
    ///
    /// # Panics
    ///
    /// Panics if `a == b` (the two keys must differ).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `FxHashMap::get_mut` (via raw pointers for split borrows)
    fn get_inv_pair_mut(&mut self, a: u16, b: u16) -> Option<(&mut Inventory, &mut Inventory)> {
        assert_ne!(a, b, "get_inv_pair_mut: keys must differ");
        let pa = self.player.invs.get_mut(&a)? as *mut Inventory;
        let pb = self.player.invs.get_mut(&b)? as *mut Inventory;
        Some(unsafe { (&mut *pa, &mut *pb) })
    }

    /// Retrieves a player inventory, creating it if it does not yet exist.
    ///
    /// If the inventory type defines `stockobj` and `stockcount` in the cache, the newly
    /// created inventory is pre-populated with those entries. Stock defaults are not
    /// exclusive to shared inventories, so player-scoped invs honor them too.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `FxHashMap::entry`, `Inventory::with_stack_mode`, `Inventory::set`
    fn get_or_create_inv(&mut self, id: u16, size: usize, stack_mode: StackMode) -> &mut Inventory {
        self.player.invs.entry(id).or_insert_with(|| {
            let mut inv = Inventory::with_stack_mode(size, stack_mode);
            if let Some(inv_type) = cache().invs.get_by_id(id) {
                if let Some(stockobj) = &inv_type.stockobj {
                    inv.stockobj = stockobj.clone();
                }
                if let (Some(stockobj), Some(stockcount)) =
                    (&inv_type.stockobj, &inv_type.stockcount)
                {
                    for i in 0..stockobj.len() {
                        inv.set(i as u16, stockobj[i], stockcount[i] as u32);
                    }
                }
            }
            inv
        })
    }

    /// Registers an inventory for automatic transmission to the client via an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `FxHashMap::entry`
    fn add_inv_transmit(&mut self, inv_id: u16, com: u16) {
        self.player
            .inv_transmits
            .entry(inv_id)
            .or_default()
            .push(com);
    }

    /// Checks whether a given interface component has an inventory transmit binding.
    ///
    /// # Returns
    ///
    /// `Some(inv_id)` if the component is bound, `None` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** iterates `self.player.inv_transmits`
    fn has_inv_transmit(&self, com: u16) -> Option<u16> {
        self.player
            .inv_transmits
            .iter()
            .find(|(_, coms)| coms.contains(&com))
            .map(|(id, _)| *id)
    }

    /// Removes all transmit bindings for the given interface component.
    ///
    /// # Side Effects
    ///
    /// Clears the bindings and sends an `UpdateInvStopTransmit` packet.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::clear_inv_transmits`, `ActivePlayer::update_inv_stop_transmit`
    fn clear_inv_transmits(&mut self, com: u16) {
        self.player.clear_inv_transmits(com);
        self.update_inv_stop_transmit(com);
    }

    /// Registers a cross-player inventory listener on a component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait (`INVOTHER_TRANSMIT`)
    /// **Calls:** `HashMap::insert`
    fn add_inv_other_transmit(&mut self, com: u16, inv_id: u16, uid: i32) {
        self.player.inv_other_transmits.insert(com, (uid, inv_id));
    }

    /// Returns the cross-player inventory listener bound to a component, if any.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait (`INVOTHER_TRANSMIT`)
    /// **Calls:** `HashMap::get`
    fn has_inv_other_transmit(&self, com: u16) -> Option<(i32, u16)> {
        self.player.inv_other_transmits.get(&com).copied()
    }

    /// Appends a waypoint to the player's movement queue.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Pathing::queue_waypoint`
    fn queue_waypoint(&mut self, x: u16, z: u16) {
        self.player.pathing.queue_waypoint(x, z);
    }

    /// Starts the player walking toward the given tile using pathfinding.
    ///
    /// Computes a path from the player's current position to the destination
    /// using `rsmod::find_path` with normal collision and queues the waypoints.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `rsmod::find_path`, `Pathing::queue_waypoints`
    fn walk(&mut self, dest_x: u16, dest_z: u16) {
        self.player.pathing.queue_waypoints(rsmod::find_path(
            self.player.pathing.coord.y(),
            self.player.pathing.coord.x(),
            self.player.pathing.coord.z(),
            dest_x,
            dest_z,
            1,
            1,
            1,
            0,
            -1,
            true,
            0,
            25,
            CollisionType::Normal,
        ));
    }

    /// Clears all pending movement waypoints.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::clear_waypoints`
    fn clear_waypoints(&mut self) {
        self.clear_waypoints();
    }

    /// Sets the player's interaction target to a location.
    ///
    /// Transmutes raw `u8` shape, angle, and layer values to their enum types
    /// and builds an `InteractionTarget::Loc`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::set_interaction`
    fn set_interaction_loc(
        &mut self,
        coord: u32,
        id: u16,
        width: u8,
        length: u8,
        shape: u8,
        angle: u8,
        layer: u8,
        op: u8,
    ) {
        let target = InteractionTarget::Loc {
            coord: CoordGrid::from(coord),
            id,
            width,
            length,
            shape: unsafe { std::mem::transmute::<u8, LocShape>(shape) },
            angle: unsafe { std::mem::transmute::<u8, LocAngle>(angle) },
            layer: unsafe { std::mem::transmute::<u8, LocLayer>(layer) },
        };
        self.player.set_interaction(target, op, false);
    }

    /// Sets the player's interaction target to an NPC.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::set_interaction`
    fn set_interaction_npc(&mut self, nid: u16, op: u8) {
        let target = InteractionTarget::Npc { nid };
        self.player.set_interaction(target, op, false);
    }

    /// Sets the player's interaction target to a ground object.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::set_interaction`
    fn set_interaction_obj(&mut self, coord: u32, id: u16, count: u32, op: u8) {
        let target = InteractionTarget::Obj {
            coord: CoordGrid::from(coord),
            id,
            count,
        };
        self.player.set_interaction(target, op, false);
    }

    /// Sets the player's interaction target to another player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::set_interaction`
    fn set_interaction_player(&mut self, pid: u16, op: u8) {
        let target = InteractionTarget::Player { pid };
        self.player.set_interaction(target, op, false);
    }

    /// Records a spell/use component as the subject of the player's current interaction.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.interaction.target_subject_com`
    fn set_interaction_spell(&mut self, com: u16) {
        self.player.interaction.target_subject_com = Some(com);
    }

    /// Checks whether the player is within operable distance of a location.
    ///
    /// Returns `false` immediately if the player is on a different level.
    /// Otherwise delegates to `rsmod::reached` for collision-aware reachability.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `rsmod::reached`
    fn in_operable_distance_loc(
        &self,
        coord: u32,
        width: u8,
        length: u8,
        shape: u8,
        angle: u8,
        forceapproach: u8,
    ) -> bool {
        let c = CoordGrid::from(coord);
        if c.y() != self.player.pathing.coord.y() {
            return false;
        }
        rsmod::reached(
            self.player.pathing.coord.y(),
            self.player.pathing.coord.x(),
            self.player.pathing.coord.z(),
            c.x(),
            c.z(),
            width,
            length,
            1,
            angle,
            shape as i8,
            forceapproach,
        )
    }

    /// Sets the approach range for the player's current interaction.
    ///
    /// # Side Effects
    ///
    /// Sets `ap_range` and marks `ap_range_called` for the interaction processor.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.interaction.ap_range` and `ap_range_called`
    fn aprange(&mut self, range: i32) {
        self.player.interaction.ap_range = Some(range as u16);
        self.player.interaction.ap_range_called = true;
    }
}

impl ScriptNpc for ActiveNpc {
    /// Returns this NPC's unique identifier.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** reads `self.npc.uid`
    fn uid(&self) -> NpcUid {
        self.npc.uid
    }

    /// Returns the NPC's current coordinate as a packed `u32`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `CoordGrid::packed`
    fn coord(&self) -> u32 {
        self.npc.pathing.coord.packed()
    }

    /// Returns the NPC's collision size in tiles.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** reads `self.npc.pathing.size`
    fn size(&self) -> u8 {
        self.npc.pathing.size
    }

    /// Reads an NPC variable (varn) by its definition ID.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `VarSet::get`
    fn get_var(&self, id: u16) -> VarValue {
        self.npc.vars.get(id).clone()
    }

    /// Writes an NPC variable (varn).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `VarSet::set`
    fn set_var(&mut self, id: u16, value: VarValue) {
        self.npc.vars.set(id, value);
    }

    /// Returns the opcode of the NPC's current interaction target, if any.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** reads `self.npc.interaction.target_op`
    fn target_op(&self) -> Option<u8> {
        self.npc.interaction.target_op
    }

    /// Sets the NPC's AI mode.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** sets `self.npc.interaction.target_op`
    fn set_mode(&mut self, mode: Option<u8>) {
        self.npc.interaction.target_op = mode;
    }

    /// Clears the NPC's current interaction target.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Npc::clear_interaction`
    fn clear_interaction(&mut self) {
        self.npc.clear_interaction();
    }

    /// Resets the NPC's patrol progress.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait (`NPC_SETMODE`)
    /// **Calls:** `Npc::clear_patrol`
    fn clear_patrol(&mut self) {
        self.npc.clear_patrol();
    }

    /// Resets the NPC to its default spawn state.
    ///
    /// Reads default mode, hunt mode, hunt range, and timer interval from the NPC type
    /// definition in the cache and applies them. Also clears the face entity.
    ///
    /// # Side Effects
    ///
    /// Resets interaction state, mode, hunt parameters, and timer.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `cache().npcs.get_by_id`, `NpcInfo::clear_face_entity_npc`, `Npc::reset_defaults`
    fn reset_defaults(&mut self) {
        let npc_type = cache().npcs.get_by_id(self.npc.uid.id());
        let default_mode = npc_type.map(|t| t.defaultmode).unwrap_or(NpcMode::None);
        let hunt_mode = npc_type.and_then(|t| t.huntmode);
        let hunt_range = npc_type.map(|t| t.huntrange).unwrap_or(0);
        let timer_interval = npc_type.and_then(|t| t.timer);
        self.npc.info.clear_face_entity_npc();
        self.npc
            .reset_defaults(default_mode, hunt_mode, hunt_range, timer_interval);
    }

    /// Sets the NPC's interaction target to another NPC.
    ///
    /// Also records the target NPC's current type for trigger resolution.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `engine().get_npc`, `Npc::set_interaction`
    fn set_interaction_npc(&mut self, nid: u16, op: u8) {
        if let Some(target_npc) = engine().get_npc(nid) {
            self.npc.interaction.target_subject_type = Some(target_npc.npc.uid.id());
        }
        self.npc
            .set_interaction(InteractionTarget::Npc { nid }, op, false);
    }

    /// Sets the NPC's interaction target to a player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Npc::set_interaction`
    fn set_interaction_player(&mut self, pid: u16, op: u8) {
        self.npc
            .set_interaction(InteractionTarget::Player { pid }, op, false);
    }

    /// Sets the NPC's interaction target to a location.
    ///
    /// Transmutes raw `u8` shape, angle, and layer values to their enum types
    /// and builds an `InteractionTarget::Loc`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Npc::set_interaction`
    fn set_interaction_loc(
        &mut self,
        coord: u32,
        id: u16,
        width: u8,
        length: u8,
        shape: u8,
        angle: u8,
        layer: u8,
        op: u8,
    ) {
        let target = InteractionTarget::Loc {
            coord: CoordGrid::from(coord),
            id,
            width,
            length,
            shape: unsafe { std::mem::transmute::<u8, LocShape>(shape) },
            angle: unsafe { std::mem::transmute::<u8, LocAngle>(angle) },
            layer: unsafe { std::mem::transmute::<u8, LocLayer>(layer) },
        };
        self.npc.set_interaction(target, op, false);
    }

    /// Sets the NPC's interaction target to a ground object.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Npc::set_interaction`
    fn set_interaction_obj(&mut self, coord: u32, id: u16, count: u32, op: u8) {
        let target = InteractionTarget::Obj {
            coord: CoordGrid::from(coord),
            id,
            count,
        };
        self.npc.set_interaction(target, op, false);
    }

    /// Plays an animation on the NPC's model.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `ActiveNpc::anim`
    fn anim(&mut self, id: Option<u16>, delay: u8) {
        self.anim(id, delay);
    }

    /// Displays overhead chat text above the NPC.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `ActiveNpc::say`
    fn say(&mut self, msg: &str) {
        self.say(msg);
    }

    /// Returns the NPC's current level in the given stat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `StatBlock::level`
    fn stat(&self, stat: usize) -> u16 {
        self.npc.stats.level(stat)
    }

    /// Returns the NPC's base (unmodified) level in the given stat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `StatBlock::base_level`
    fn basestat(&self, stat: usize) -> u16 {
        self.npc.stats.base_level(stat)
    }

    /// Applies damage to the NPC and displays a hitsplat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `ActiveNpc::damage`
    fn damage(&mut self, amount: u8, damage_type: u8) {
        self.damage(amount, damage_type);
    }

    /// Awards hero points to a player for contributing damage to this NPC.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `HeroPoints::add_hero`
    fn heropoints(&mut self, user37: u64, points: i32) {
        self.npc.hero_points.add_hero(user37, points);
    }

    /// Finds the player with the most hero points on this NPC.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `HeroPoints::find_hero`
    fn findhero(&self) -> Option<u64> {
        self.npc.hero_points.find_hero()
    }

    /// Enqueues a script for deferred execution on this NPC.
    ///
    /// Uses `QueuePriority::Normal` for all NPC queues.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `QueueSet::add`
    fn queue(
        &mut self,
        script_id: i32,
        delay: u16,
        args: Option<Vec<ScriptArgument>>,
    ) -> rs_vm::Result<()> {
        self.npc
            .state
            .queues
            .add(QueuePriority::Normal, script_id, delay, args)
    }

    /// Sets or clears the NPC's recurring timer interval.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `ActiveNpc::set_timer`
    fn settimer(&mut self, interval: Option<u16>) {
        self.set_timer(interval);
    }

    /// Returns the game tick of the NPC's last movement.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** reads `self.npc.pathing.last_movement`
    fn last_movement(&self) -> u32 {
        self.npc.pathing.last_movement
    }

    /// Suspends the NPC's currently running script for a number of ticks.
    ///
    /// # Side Effects
    ///
    /// Sets the NPC's `delayed` flag and `delayed_until` tick.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** sets `self.npc.state.delayed` and `delayed_until`
    fn delay(&mut self, delay: u32) {
        self.npc.state.delayed = true;
        self.npc.state.delayed_until = delay;
    }

    /// Teleports the NPC to a new coordinate.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `ActiveNpc::tele`
    fn tele(&mut self, coord: u32) {
        self.tele(CoordGrid::from(coord));
    }

    /// Makes the NPC face a specific tile.
    ///
    /// Converts tile coordinates to fine coordinates for smooth facing.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `NpcInfo::focus_npc`, `CoordGrid::fine`
    fn facesquare(&mut self, x: u16, z: u16) {
        self.npc
            .info
            .focus_npc(CoordGrid::fine(x, 1), CoordGrid::fine(z, 1), true);
    }

    /// Queues a waypoint for the NPC to walk toward.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Pathing::queue_waypoint`
    fn walk(&mut self, x: u16, z: u16) {
        self.npc.pathing.queue_waypoint(x, z);
    }

    /// Sets the maximum hunt range for this NPC's AI.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** sets `self.npc.hunt_range`
    fn set_hunt_range(&mut self, range: u8) {
        self.npc.hunt_range = range;
    }

    /// Sets or clears the NPC's hunt mode.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** sets `self.npc.hunt_mode`
    fn set_hunt_mode(&mut self, mode: Option<u16>) {
        self.npc.hunt_mode = mode;
    }

    /// Temporarily transforms the NPC to a different type.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `ActiveNpc::change_type`
    fn change_type(&mut self, new_type: u16, duration: u64, reset: bool) {
        self.change_type(new_type, duration, reset);
    }

    /// Returns whether the NPC's current target is within its max range
    /// from its spawn point.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Engine::npc_target_within_max_range`
    fn inrange(&self) -> bool {
        Engine::npc_target_within_max_range(self)
    }

    /// Adds to the NPC's current stat level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `StatBlock::add`
    fn statadd(&mut self, stat: usize, constant: i32, percent: i32) {
        self.npc.stats.add(stat, constant, percent);
    }

    /// Subtracts from the NPC's current stat level, floored at 0.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `StatBlock::sub`
    fn statsub(&mut self, stat: usize, constant: i32, percent: i32) {
        self.npc.stats.sub(stat, constant, percent);
    }

    /// Heals the NPC's current stat level, capped at the base level.
    ///
    /// Clears hero points if Hitpoints is restored to or above the base level,
    /// so an NPC healed back to full HP loses its stale damage attribution.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `StatBlock::heal`
    fn statheal(&mut self, stat: usize, constant: i32, percent: i32) {
        self.npc.stats.heal(stat, constant, percent);
        if stat == NpcStat::Hitpoints as usize
            && self.npc.stats.level(NpcStat::Hitpoints as usize)
                >= self.npc.stats.base_level(NpcStat::Hitpoints as usize)
        {
            self.npc.hero_points.clear();
        }
    }

    /// Sets the NPC's walk trigger script and argument.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    fn walktrigger(&mut self, trigger: i32, arg: i32) {
        self.npc.walktrigger = Some(trigger);
        self.npc.walktrigger_arg = arg;
    }

    /// Plays a spot animation (graphic) on the NPC.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    fn spotanim(&mut self, id: u16, height: u16, delay: u16) {
        self.npc.info.spotanim = Some(id);
        self.npc.info.spotanim_height = Some(height);
        self.npc.info.spotanim_delay = Some(delay);
        self.npc.info.masks |= NpcInfoProt::SpotAnim as u16;
    }

    /// Returns the NPC's current coord destination.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    fn destination(&self) -> u32 {
        if !self.npc.pathing.has_waypoints() {
            return self.coord();
        }
        self.npc.pathing.waypoints[0]
    }
}

/// Builds and returns the complete [`OpsRegistry`] for the script VM.
///
/// Collects opcode handler functions from all 15 operation modules (core, db, debug,
/// enum, inv, loc, nc, npc, number, obj, oc, player, server, string, struct) and
/// registers them in a single registry that the VM uses at runtime to dispatch
/// script opcodes.
///
/// # Returns
///
/// A fully populated [`OpsRegistry`] ready to be installed in the engine.
///
/// # Call Stack
///
/// **Called by:** `Engine::new`
/// **Calls:** `ops::core::build`, `ops::db::build`, `ops::debug::build`, etc.
pub fn register_ops() -> OpsRegistry {
    let mut ops = OpsRegistry::new();
    ops.extend(ops::core::build::<Engine>());
    ops.extend(ops::db::build());
    ops.extend(ops::debug::build());
    ops.extend(ops::r#enum::build());
    ops.extend(ops::inv::build::<Engine>());
    ops.extend(ops::lc::build());
    ops.extend(ops::loc::build::<Engine>());
    ops.extend(ops::nc::build());
    ops.extend(ops::npc::build::<Engine>());
    ops.extend(ops::number::build::<Engine>());
    ops.extend(ops::obj::build::<Engine>());
    ops.extend(ops::oc::build());
    ops.extend(ops::player::build::<Engine>());
    ops.extend(ops::server::build::<Engine>());
    ops.extend(ops::string::build::<Engine>());
    ops.extend(ops::r#struct::build());
    ops
}

#[cfg(test)]
mod tests {
    use super::next_free_id;

    #[test]
    fn round_robin_wraps_around() {
        let capacity = 10;
        let mut occupied = vec![false; capacity];
        let mut cursor = (capacity - 2) as u16;

        let alloc = |cursor: &mut u16, occupied: &mut Vec<bool>| -> u16 {
            let id =
                next_free_id(*cursor, (capacity - 1) as u16, 1, |i| !occupied[i as usize]).unwrap();
            occupied[id as usize] = true;
            *cursor = id;
            id
        };

        assert_eq!(alloc(&mut cursor, &mut occupied), 1);
        assert_eq!(alloc(&mut cursor, &mut occupied), 2);
        assert_eq!(alloc(&mut cursor, &mut occupied), 3);
        assert_eq!(alloc(&mut cursor, &mut occupied), 4);
        assert_eq!(alloc(&mut cursor, &mut occupied), 5);
        assert_eq!(alloc(&mut cursor, &mut occupied), 6);
        assert_eq!(alloc(&mut cursor, &mut occupied), 7);
        assert_eq!(alloc(&mut cursor, &mut occupied), 8);

        // All slots 1..=8 occupied, should be full
        let full = next_free_id(cursor, (capacity - 1) as u16, 1, |i| !occupied[i as usize]);
        assert_eq!(full, None);
    }

    #[test]
    fn round_robin_reuses_freed_slots() {
        let capacity = 6;
        let mut occupied = vec![false; capacity];
        let next = |cursor: u16, occupied: &[bool]| {
            next_free_id(cursor, (capacity - 1) as u16, 1, |i| !occupied[i as usize])
        };

        // Fill 1..=4
        for id in 1..=4u16 {
            occupied[id as usize] = true;
        }
        let cursor = 4u16;

        // Full
        assert_eq!(next(cursor, &occupied), None);

        // Free slot 2
        occupied[2] = false;

        // Should wrap and find 2
        assert_eq!(next(cursor, &occupied), Some(2));
    }

    #[test]
    fn round_robin_skips_occupied() {
        let capacity = 10;
        let mut occupied = vec![false; capacity];
        occupied[3] = true;
        occupied[4] = true;

        let id = next_free_id(2, (capacity - 1) as u16, 1, |i| !occupied[i as usize]);
        assert_eq!(id, Some(5));
    }

    #[test]
    fn round_robin_continues_from_cursor() {
        let capacity = 10;
        let occupied = vec![false; capacity];

        // cursor=5 → first free after 5 is 6
        assert_eq!(
            next_free_id(5, (capacity - 1) as u16, 1, |i| !occupied[i as usize]),
            Some(6)
        );

        // cursor=7 → 8
        assert_eq!(
            next_free_id(7, (capacity - 1) as u16, 1, |i| !occupied[i as usize]),
            Some(8)
        );

        // cursor=8 → wraps to 1
        assert_eq!(
            next_free_id(8, (capacity - 1) as u16, 1, |i| !occupied[i as usize]),
            Some(1)
        );
    }

    #[test]
    fn npc_round_robin_includes_zero() {
        let capacity = 10;
        let occupied = vec![false; capacity];

        // NPC lower bound is 0, cursor near end → wraps to 0
        assert_eq!(
            next_free_id(8, (capacity - 1) as u16, 0, |i| !occupied[i as usize]),
            Some(0)
        );
    }

    #[test]
    fn player_round_robin_skips_zero() {
        let capacity = 10;
        let mut occupied = vec![false; capacity];
        // Occupy all slots 1..=8
        for i in 1..=8u16 {
            occupied[i as usize] = true;
        }

        // Player lower bound is 1 → slot 0 is never returned
        assert_eq!(
            next_free_id(8, (capacity - 1) as u16, 1, |i| !occupied[i as usize]),
            None
        );

        // But NPC lower bound 0 → finds slot 0
        assert_eq!(
            next_free_id(8, (capacity - 1) as u16, 0, |i| !occupied[i as usize]),
            Some(0)
        );
    }

    #[test]
    fn round_robin_full_cycle() {
        let capacity = 6; // slots 0..5, valid player range 1..=4
        let mut occupied = vec![false; capacity];
        let mut cursor = 4u16;
        let mut allocated = Vec::new();

        // Allocate all 4 player slots
        for _ in 0..4 {
            let id = next_free_id(cursor, 5, 1, |i| !occupied[i as usize]).unwrap();
            occupied[id as usize] = true;
            cursor = id;
            allocated.push(id);
        }
        assert_eq!(allocated, vec![1, 2, 3, 4]);

        // Free 2 and 4
        occupied[2] = false;
        occupied[4] = false;

        // Next from cursor=4 → wraps, finds 2 first
        let id = next_free_id(cursor, 5, 1, |i| !occupied[i as usize]).unwrap();
        assert_eq!(id, 2);
        occupied[id as usize] = true;
        cursor = id;

        // Next from cursor=2 → finds 4
        let id = next_free_id(cursor, 5, 1, |i| !occupied[i as usize]).unwrap();
        assert_eq!(id, 4);
    }
}
