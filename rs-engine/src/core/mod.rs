mod cycle;
mod locs;
mod npcs;
mod objs;
mod players;
mod sessions;
mod shutdown;
mod zones;

pub use cycle::{CycleResult, TickStats};
pub use npcs::{NpcEvent, NpcEventKind, NpcList};
pub use objs::ObjDelayedRequest;
pub use players::{OtherPlayers, PlayerList};
pub use sessions::{LoginFailures, LoginRequest, LogoutSave, PendingLogin};
pub use zones::PendingZoneEvent;

use crate::MAX_NPCS;
use crate::clients::client_db::{DbRequest, DbResponse};
use crate::clients::client_ether::{EtherInbound, EtherOutbound};
use crate::controls::Controls;
use crate::info::{NpcInfo, NpcSnapshot, PlayerInfo, PlayerSnapshots};
use crate::map::GameMap;
use crate::pathfinding::InputPaths;
#[cfg(since_647)]
use crate::phases::controller::Controllers;
#[cfg(since_647)]
use crate::region::Regions;
use crate::script::errors::ScriptErrors;
use crate::script::register_ops;
use crate::watch::ReplayRequest;
use mpsc::{UnboundedReceiver, UnboundedSender};
use rs_datastruct::LinkList;
use rs_db::LogBatch;
use rs_grid::{CoordGrid, ZoneCoordGrid};
use rs_info::{NpcRenderer, PlayerRenderer};
use rs_inv::Inventory;
use rs_pack::cache::CacheStore;
use rs_pack::cache::script::ScriptProvider;
#[cfg(since_647)]
use rs_pack::startup::InterfaceLayout;
use rs_util::random::JavaRandom;
use rs_var::VarSet;
pub use rs_vm::engine::with_engine;
use rs_vm::engine::{engine_typed, engine_typed_mut};
use rs_vm::register::OpsRegistry;
use rs_vm::state::*;
use rs_zone::zone_map::ZoneMap;
use rsmod::rsmod::flag::zone_flag::ZoneFlag;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, watch};
use tracing::info;
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

/// The central game-state container and tick orchestrator.
///
/// `Engine` owns the mutable world state: all players, NPCs, zones,
/// inventories and scripts. Collision and zone flags live in rs-pathfinder's
/// global tables (see [`init_collision`]). A single instance is created at
/// startup and moved onto the `engine` thread, whose tick loop calls
/// [`Engine::cycle`] once per game tick (nominally every 600 ms).
///
/// # Thread Safety
///
/// `Engine` is only ever accessed from the single world-tick task.
/// `unsafe impl Send` is provided so it can be moved into that task;
/// it is *not* `Sync` and must never be shared across threads.
pub struct Engine {
    pub clock: u32,
    pub shutdown_clock: Option<u32>,
    pub maintenance_mode: bool,
    pub controls: Controls,
    date_origin_ms: i64,
    date_origin: Instant,
    pub(crate) cycle_date_ms: i64,
    pub members: bool,
    pub multi_xp: u8,
    pub client_pathfinder: bool,
    pub(crate) input_paths: InputPaths,
    pub staff_isolation: bool,
    pub player_list: PlayerList,
    pub npc_list: NpcList,
    pub zones: ZoneMap,
    pub new_player_rx: UnboundedReceiver<LoginRequest>,
    pub scripts: ScriptProvider,
    pub cache: &'static CacheStore,
    #[cfg(since_647)]
    pub layout: InterfaceLayout,
    cache_ptr: *mut CacheStore,
    tick_stats_tx: Option<Sender<TickStats>>,
    pub(crate) gather_output: bool,
    pub reload_tx: UnboundedSender<()>,
    pub player_renderer: PlayerRenderer,
    pub npc_renderer: NpcRenderer,
    pub player_info: PlayerInfo,
    pub npc_info: NpcInfo,
    pub player_snapshots: PlayerSnapshots,
    pub npc_snapshots: Box<[NpcSnapshot; MAX_NPCS]>,
    pub invs: FxHashMap<u16, Inventory>,
    pub ops: OpsRegistry,
    pub zones_tracking: FxHashSet<ZoneCoordGrid>,
    pub pending_zone_events: BTreeMap<u32, Vec<PendingZoneEvent>>,
    #[cfg(since_647)]
    pub regions: Regions,
    #[cfg(since_647)]
    pub controllers: Controllers,
    #[cfg(since_647)]
    pub(crate) dropped_zones: Vec<ZoneCoordGrid>,
    pub obj_appeared: FxHashMap<(CoordGrid, u16), Instant>,
    pub world_queue: LinkList<ScriptState>,
    pub npc_events: LinkList<NpcEvent>,
    pub obj_delayed_queue: LinkList<ObjDelayedRequest>,
    pub clock_rate_tx: Sender<u64>,
    pub node_id: u8,
    pub ether_tx: Option<UnboundedSender<EtherOutbound>>,
    pub ether_rx: UnboundedReceiver<EtherInbound>,
    pub db_tx: Option<UnboundedSender<DbRequest>>,
    pub db_rx: UnboundedReceiver<DbResponse>,
    pub replay_tx: Option<tokio::sync::mpsc::Sender<ReplayRequest>>,
    pub db_ready: bool,
    pub ether_ready: bool,
    pub pending_logins: Vec<PendingLogin>,
    pub next_login_id: u32,
    pub login_failures: FxHashMap<IpAddr, LoginFailures>,
    pub logout_saves: FxHashMap<u64, LogoutSave>,
    pub offline_edits: FxHashSet<u64>,
    pub db_lease: Option<Instant>,
    pub logs: LogBatch,
    pub script_errors: ScriptErrors,
    pub random: JavaRandom,
    pub vars: VarSet,
    pub(crate) reusable_script: Option<ScriptState>,
}

// SAFETY: Engine is only accessed from the single world-tick tokio task.
// The *mut CacheStore points to the same Box::leak'd allocation that all
// &'static CacheStore references share; it is only written during reload_assets
// which runs exclusively on that task.
unsafe impl Send for Engine {}

/// Builds rs-pathfinder's collision and zone flag tables (144 MiB, nearly all
/// of it never written) while the heap is still fresh. Built after the cache
/// pack, on memory the pack already used, mimalloc has to zero them by hand,
/// and every page of them stays resident.
pub fn init_collision() {
    rsmod::is_zone_allocated(0, 0, 0);
    rsmod::is_zone_flagged(0, 0, 0, ZoneFlag::Free as u8);
}

impl Engine {
    /// A receiver for the [`TickStats`] each cycle publishes, for the rs-test harness.
    #[cfg(all(feature = "harness", since_647))]
    pub fn harness_tick_stats(&self) -> watch::Receiver<TickStats> {
        self.tick_stats_tx.as_ref().unwrap().subscribe()
    }

    /// Constructs a new [`Engine`], loading the game map and spawning all
    /// static NPCs defined in the cache.
    ///
    /// # Arguments
    ///
    /// * `members` -- Whether this world runs in members mode.
    /// * `multi_xp` -- Stat experience multiplier.
    /// * `client_pathfinder` -- Whether to use the client-side pathfinder.
    ///   Ignored on rev 647, where the server always pathfinds.
    /// * `new_player_rx` -- Channel receiver for incoming [`LoginRequest`]s.
    /// * `scripts` -- Pre-loaded script provider (RuneScript bytecode).
    /// * `cache` -- Leaked static reference to the game cache store.
    /// * `cache_ptr` -- Raw pointer to the same `CacheStore` allocation, used
    ///   by [`Engine::reload_assets`] to swap cache data in place.
    /// * `layout` -- Rev-647 component lookups (toplevel mounts, dialogue
    ///   slots) derived from interface.pack at pack time.
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
    /// - Flags the cache's freemap zones `Free` on every level and, before
    ///   rev 647, its multimap zones `Multi`, in rs-pathfinder's zone flags.
    /// - Calls [`GameMap::load`] to populate the zone map with static locs, objs, and NPC spawn points.
    /// - Spawns all static NPCs via [`Engine::add_npc`], queueing their `ai_spawn` scripts.
    /// - Registers all VM opcodes via `register_ops`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `bootstrap` (rs-server), `load_engine` (rs-test).
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
        #[cfg(since_647)] layout: InterfaceLayout,
        stats_tx: Sender<TickStats>,
        reload_tx: UnboundedSender<()>,
        node_id: u8,
        ether_tx: Option<UnboundedSender<EtherOutbound>>,
        ether_rx: UnboundedReceiver<EtherInbound>,
        db_tx: Option<UnboundedSender<DbRequest>>,
        db_rx: UnboundedReceiver<DbResponse>,
    ) -> (Self, watch::Receiver<u64>) {
        // Rev-647 move clicks carry ONLY the destination tile -- the client has
        // no waypoint list to hand us (see `handlers/move_click.rs`). Trusting
        // it would queue that single tile as the whole route and walk the
        // player straight into the first wall, so the server always pathfinds.
        // Pre-647 clients do send a full client-side path, which stays trusted.
        #[cfg(since_647)]
        let client_pathfinder = {
            let _ = client_pathfinder;
            false
        };

        let ops = register_ops();

        for &key in &cache.freemap {
            let zone = ZoneCoordGrid::from(key);
            for y in 0..4 {
                rsmod::change_zone(zone.x(), zone.z(), y, ZoneFlag::Free as u8, true);
            }
        }
        #[cfg(before_647)]
        for &key in &cache.multimap {
            let zone = ZoneCoordGrid::from(key);
            rsmod::change_zone(zone.x(), zone.z(), zone.y(), ZoneFlag::Multi as u8, true);
        }

        let mut zones = ZoneMap::new();
        let spawned_npcs = GameMap::load(members, cache, &mut zones);

        let vars = VarSet::new(cache.varss.types.iter().map(|v| v.var_type));

        let (clock_rate_tx, clock_rate_rx) = channel(600);

        let date_origin_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let mut engine = Self {
            clock: 0,
            shutdown_clock: None,
            maintenance_mode: false,
            controls: Controls::default(),
            date_origin_ms,
            date_origin: Instant::now(),
            cycle_date_ms: date_origin_ms,
            members,
            multi_xp,
            client_pathfinder,
            input_paths: InputPaths::default(),
            staff_isolation: !cfg!(debug_assertions),
            player_list: PlayerList::new(),
            npc_list: NpcList::new(),
            zones,
            new_player_rx,
            scripts,
            cache,
            cache_ptr,
            #[cfg(since_647)]
            layout,
            tick_stats_tx: Some(stats_tx),
            gather_output: false,
            reload_tx,
            player_renderer: PlayerRenderer::new(),
            npc_renderer: NpcRenderer::new(),
            player_info: PlayerInfo::new(),
            npc_info: NpcInfo::new(),
            player_snapshots: PlayerSnapshots::new(),
            npc_snapshots: Box::new([NpcSnapshot::ABSENT; MAX_NPCS]),
            invs: FxHashMap::default(),
            ops,
            zones_tracking: FxHashSet::default(),
            pending_zone_events: BTreeMap::new(),
            #[cfg(since_647)]
            regions: Regions::default(),
            #[cfg(since_647)]
            controllers: Controllers::default(),
            #[cfg(since_647)]
            dropped_zones: Vec::new(),
            obj_appeared: FxHashMap::default(),
            world_queue: LinkList::new(),
            npc_events: LinkList::new(),
            obj_delayed_queue: LinkList::new(),
            clock_rate_tx,
            node_id,
            ether_tx,
            ether_rx,
            db_tx,
            db_rx,
            replay_tx: None,
            db_ready: false,
            ether_ready: false,
            pending_logins: Vec::new(),
            next_login_id: 0,
            login_failures: FxHashMap::default(),
            logout_saves: FxHashMap::default(),
            offline_edits: FxHashSet::default(),
            db_lease: None,
            logs: LogBatch::default(),
            script_errors: ScriptErrors::default(),
            random: JavaRandom::new(1084838400000),
            vars,
            reusable_script: None,
        };
        for npc in spawned_npcs {
            engine.add_npc(npc);
        }

        (engine, clock_rate_rx)
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
    /// * `layout` -- The rev-647 component lookups derived from the repacked
    ///   interface.pack.
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
    /// - On rev 647, replaces `self.layout` with `layout` and makes `regions`
    ///   forget the mapsquares it decoded for copying.
    /// - Re-matches the kill switches in `controls` against the new scripts.
    /// - In debug builds, broadcasts a reload notification to all online players.
    /// - Logs the number of loaded scripts.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `apply_reload` (rs-server), on the engine thread between ticks.
    pub fn reload_assets(
        &mut self,
        new_store: Box<CacheStore>,
        new_scripts: ScriptProvider,
        #[cfg(since_647)] layout: InterfaceLayout,
    ) {
        #[cfg(since_647)]
        for active in self.player_list.players.iter_mut().flatten() {
            let mut varcs = VarSet::new(new_store.varcs.types().map(|v| v.var_type));
            for varc in new_store.varcs.types() {
                let string = varc.var_type.is_string();
                if let Some(old) = self.cache.varcs.get_by_id(varc.id, string)
                    && old.var_type == varc.var_type
                {
                    let from = self.cache.varcs.index(varc.id, string);
                    let to = new_store.varcs.index(varc.id, string);
                    varcs[to] = active.player.varcs[from].clone();
                }
            }
            active.player.varcs = varcs;
        }
        unsafe {
            std::ptr::drop_in_place(self.cache_ptr);
            std::ptr::write(self.cache_ptr, *new_store);
        }
        self.scripts = new_scripts;
        #[cfg(since_647)]
        {
            self.layout = layout;
            self.regions.forget_squares();
        }
        self.controls.refresh(&self.scripts);
        let count = self.scripts.count();

        #[cfg(debug_assertions)]
        self.broadcast(&format!("Hot-reload applied - {count} scripts loaded"));
        info!("Hot-reload applied - {} scripts loaded", count);
    }

    /// Swaps in recompiled scripts, leaving the cache as it is: an emergency
    /// fix to a running world. Scripts already running finish on the code
    /// they started with. Returns how many scripts are loaded, and the old
    /// scripts, to free off the engine thread.
    pub fn reload_scripts(&mut self, new_scripts: ScriptProvider) -> (usize, ScriptProvider) {
        let old = std::mem::replace(&mut self.scripts, new_scripts);
        self.controls.refresh(&self.scripts);
        let count = self.scripts.count();
        info!("Script reload applied - {} scripts loaded", count);
        (count, old)
    }
}

/// Finds the next free id in round-robin order, starting after `cursor`.
///
/// Scans `cursor + 1 .. upper`, then wraps around to `lower ..= cursor`, and
/// returns the first id `is_free` accepts, or `None` when none is free.
///
/// # Arguments
///
/// * `cursor` -- The id the list added last; the scan starts just after it.
/// * `upper` -- Exclusive upper bound of the scan before it wraps.
/// * `lower` -- First id checked after wrapping: 1 for pids, so pid 0 is
///   never handed out, and 0 for nids and region slots.
/// * `is_free` -- Whether an id's slot is empty.
///
/// # Call Stack
///
/// **Called by:** `PlayerList::next_pid`, `NpcList::next_nid`, `Regions::allocate`.
pub(crate) fn next_free_id(
    cursor: u16,
    upper: u16,
    lower: u16,
    is_free: impl Fn(u16) -> bool,
) -> Option<u16> {
    for i in (cursor + 1)..upper {
        if is_free(i) {
            return Some(i);
        }
    }
    (lower..=cursor).find(|&i| is_free(i))
}

/// The clock a loc or obj timer of `duration` ticks fires at, set at `clock`.
/// The reference engine counts these down in `processZones` starting on the
/// tick they were set, so a timer fires `duration - 1` ticks later (a 500-tick
/// loc shows for 499 ticks), and 1 means later the same tick.
const fn lifecycle_clock(clock: u32, duration: u64) -> u32 {
    let duration = if duration > 0 { duration - 1 } else { 0 };
    clock + duration as u32
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
