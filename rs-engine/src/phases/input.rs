use crate::active_player::{ActivePlayer, EnginePlayer};
#[cfg(since_647)]
use crate::core::engine;
use crate::core::{Engine, engine_mut};
use crate::handlers::client_cheat::bot_tick;
use crate::phases::shared::{EntityId, panic_message};
use rs_vm::engine::ScriptPlayer;
use std::panic::{AssertUnwindSafe, catch_unwind};
use tracing::error;

/// Probability of an AFK random event firing per 500-tick check when the
/// player is in a normal zone. Equivalent to once per ~120 seconds on average.
const AFK_CHANCE1: f64 = 1f64 / (120f64 / 5f64);

/// Probability of an AFK random event firing per 500-tick check when the
/// player is in an accelerated AFK zone (zone 1000). Equivalent to once per
/// ~60 seconds on average.
const AFK_CHANCE2: f64 = 1f64 / (60f64 / 5f64);

impl Engine {
    /// Processes the input phase of the engine tick cycle.
    ///
    /// For each active player, within a panic-catching boundary:
    ///
    /// 1. Records the player's previous coordinate.
    /// 2. Rolls the AFK random-event check ([`check_afk`]).
    /// 3. Decodes incoming client packets ([`ActivePlayer::decode`]).
    /// 4. Post-processes the decoded input, potentially pathing toward an
    ///    interaction target ([`post_process`]).
    /// 5. Updates zone membership and collision maps via
    ///    [`refresh_presence`].
    ///
    /// If any player panics during processing, that player is
    /// emergency-removed to prevent cascading failures.
    ///
    /// # Side Effects
    ///
    /// * Mutates each player's coordinate, path, and interaction state
    ///   based on decoded client messages.
    /// * Updates zone entity lists and collision flags when a player moves
    ///   across zone boundaries.
    /// * May emergency-remove players that cause panics.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `check_afk`, `ActivePlayer::decode`, `post_process`,
    ///   `refresh_presence`
    pub(crate) fn inputs(&mut self) {
        self.input_paths.stats = Default::default();
        let pids = self.player_list.take_pids();
        let mut start = 0;
        loop {
            let result = catch_unwind(AssertUnwindSafe(|| {
                for &pid in &pids[start..] {
                    Self::process_input(self, pid);
                    start += 1;
                }
            }));
            match result {
                Ok(()) => break,
                Err(panic) => {
                    self.input_paths.current = None;
                    let pid = pids[start];
                    let msg = panic_message(&panic);
                    error!("panic during input processing for pid {pid}: {msg}");
                    self.flush_input_paths();
                    self.emergency_remove_player(pid);
                    start += 1;
                }
            }
        }
        self.player_list.put_pids(pids);
        self.input_paths.current = None;
        self.flush_input_paths();
    }

    #[inline(always)]
    fn process_input(&mut self, pid: u16) {
        let Some(active) = self.player_list.players[pid as usize].as_mut() else {
            return;
        };
        self.input_paths.current = Some(active.player.uid);

        Self::check_afk(self.clock, active);
        if active.decode() {
            active.player.last_response = self.clock;
        }
        if active.user_limit > 0 {
            active.player.last_active = self.clock;
        }
        if active.player.bot
            && let Err(e) = bot_tick(active)
        {
            error!("bot {pid} tick: {e}");
        }
        Self::post_process(active);
        active.finish_input_path();
        self.input_paths.current = None;

        if active.player.pathing.presence != active.player.pathing.coord {
            engine_mut().flush_input_paths();
        }

        let block_walk = active.player.block_walk;
        Engine::refresh_presence(
            &mut self.zones,
            &mut active.player.pathing,
            EntityId::Player(active.player.uid.pid()),
            block_walk,
        );
    }

    /// Determines whether an AFK random event should fire for the given player.
    ///
    /// Checked once every 500 ticks. The probability depends on whether the
    /// player is in an accelerated AFK zone ([`AFK_CHANCE2`]) or a normal
    /// zone ([`AFK_CHANCE1`]). Sets `afk_event_ready` on the player when
    /// the roll succeeds.
    ///
    /// # Side Effects
    ///
    /// * Sets `active.player.afk_event_ready` to `true` or `false`.
    #[inline(always)]
    fn check_afk(clock: u32, active: &mut ActivePlayer) {
        if clock.is_multiple_of(500) {
            let chance = if active.player.zones_afk() {
                AFK_CHANCE2
            } else {
                AFK_CHANCE1
            };
            active.player.afk_event_ready = engine_mut().random.next_double() < chance;
        }
    }

    /// Post-processes decoded client input for a single player, like the
    /// reference engine's `processClientsIn`.
    ///
    /// After a move click or an op this tick, records whether it counts as a
    /// move request: anything but an op made while not busy. A busy player
    /// with a move request and a queued script can't walk (see
    /// `process_movement`). Clears waypoints if the player is delayed.
    ///
    /// The move click handlers already queued the waypoints. Before 647 an
    /// op's route comes from the move click the client sends with it; the
    /// player phase re-paths only to a moving target. 647's client sends an
    /// op alone (its run flag is in the op packet), so the server paths to
    /// the target here, on the tick the op arrives.
    ///
    /// # Side Effects
    ///
    /// * Sets `active.player.move_request`.
    /// * Clears waypoints when the player is delayed.
    /// * At 647, queues the path to an op's target.
    #[inline(always)]
    fn post_process(active: &mut ActivePlayer) {
        if !active.player.move_clicked && !active.player.opcalled {
            return;
        }

        if active.player.state.delayed {
            active.clear_waypoints();
            return;
        }

        active.player.move_request = active.busy() || !active.player.opcalled;

        #[cfg(since_647)]
        {
            use rs_vm::trigger::ServerTriggerType;
            let target_op = active.player.interaction.target_op;
            let following = target_op == Some(ServerTriggerType::ApPlayer3 as u8)
                || target_op == Some(ServerTriggerType::OpPlayer3 as u8);
            let client_pathfinder = engine().client_pathfinder;
            if !following
                && active.player.opcalled
                && (!active.player.pathing.has_waypoints() || !client_pathfinder)
            {
                Self::path_to_target(active as *mut ActivePlayer, client_pathfinder);
            }
        }
    }
}
