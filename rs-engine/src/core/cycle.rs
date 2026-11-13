use crate::core::Engine;
use crate::phases::shared::panic_message;
use rs_vm::engine::with_engine;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;
use tracing::{error, info};

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

            engine.cycle_date_ms =
                    engine.date_origin_ms + engine.date_origin.elapsed().as_millis() as i64;

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

            engine.gather_output = true;
            let _ = phase!("fence", engine.fence());
            let world = phase!("world", engine.world());
            let _ = phase!("fence", engine.fence());
            let input = phase!("in", engine.inputs());
            let npcs = phase!("npcs", engine.npcs());
            #[cfg(since_647)]
            let _ = phase!("controllers", engine.process_controllers());
            let _ = phase!("fence", engine.fence());
            let players = phase!("players", engine.players());
            let logouts = phase!("logouts", engine.logouts());
            let autosave = phase!("autosave", engine.autosave());
            let logins = phase!("logins", engine.logins());
            let ether = phase!("ether", engine.ether());
            let saves = phase!("saves", engine.saves());
            let zones = phase!("zones", engine.zones());
            let info = phase!("info", engine.infos());
            let out = phase!("out", engine.outputs());
            engine.gather_output = false;
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
