use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::{Engine, engine_mut};
use crate::phases::shared::EntityId;
use rs_vm::engine::ScriptPlayer;
use rs_vm::trigger::ServerTriggerType;
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
    ///    [`check_zones_and_collision`].
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
    ///   `check_zones_and_collision`
    pub(crate) fn inputs(&mut self) {
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
                    let pid = pids[start];
                    let msg = crate::phases::shared::panic_message(&panic);
                    error!("panic during input processing for pid {pid}: {msg}");
                    self.emergency_remove_player(pid);
                    start += 1;
                }
            }
        }
        self.player_list.put_pids(pids);
    }

    #[inline(always)]
    fn process_input(&mut self, pid: u16) {
        let Some(active) = self.player_list.players[pid as usize].as_mut() else {
            return;
        };

        let prev_coord = active.player.pathing.coord;

        Self::check_afk(self.clock, active);
        if active.decode() {
            active.player.last_response = self.clock;
        }
        if active.user_limit > 0 {
            active.player.last_active = self.clock;
        }
        Self::post_process(active, self.client_pathfinder);

        Engine::check_zones_and_collision(
            &mut self.zones,
            prev_coord,
            active.player.pathing.coord,
            EntityId::Player(active.player.uid.pid()),
            active.player.pathing.size,
            active.player.block_walk,
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
            let chance = if active.player.last_afk_zone == 1000 {
                AFK_CHANCE2
            } else {
                AFK_CHANCE1
            };
            active.player.afk_event_ready = engine_mut().random.next_double() < chance;
        }
    }

    /// Post-processes decoded client input for a single player.
    ///
    /// If the player has an active path or a pending op-call and is not in
    /// a delayed state, computes a server-side path to the interaction
    /// target. Clears waypoints if the player is currently delayed.
    ///
    /// Skips players that are following another player (ApPlayer3 /
    /// OpPlayer3 triggers), since their pathing is handled during the
    /// interaction phase.
    ///
    /// # Side Effects
    ///
    /// * May modify the player's waypoint queue via [`path_to_target`].
    /// * Clears waypoints when the player is delayed.
    #[inline(always)]
    fn post_process(active: &mut ActivePlayer, client_pathfinder: bool) {
        let has_path = active.player.path.as_ref().is_some_and(|p| !p.is_empty());

        if !has_path && !active.player.opcalled {
            return;
        }

        if active.player.state.delayed {
            active.clear_waypoints();
            return;
        }

        let target_op = active.player.interaction.target_op;
        let following = target_op == Some(ServerTriggerType::ApPlayer3 as u8)
            || target_op == Some(ServerTriggerType::OpPlayer3 as u8);

        active.player.move_request = active.busy() || !active.player.opcalled;

        if !following && active.player.opcalled && (!has_path || !client_pathfinder) {
            Self::path_to_target(active, client_pathfinder);
        }
    }
}
