use crate::core::Engine;
use rs_protocol::network::game::server::update_reboot_timer::UpdateRebootTimer;
use tracing::{error, info};

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
    pub(super) fn reboot_timer_clocks(&self) -> u16 {
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

    /// Calls off a scheduled reboot and clears every player's countdown.
    /// Returns `false` if none is scheduled, or it has already begun (players
    /// are being logged out), when it can no longer be stopped.
    pub fn cancel_reboot(&mut self) -> bool {
        if !self.is_pending_shutdown() || self.is_shutdown() {
            return false;
        }
        self.shutdown_clock = None;
        for active in self.player_list.players.iter_mut().flatten() {
            active.write(UpdateRebootTimer { clocks: 0 });
        }
        info!("Scheduled reboot cancelled");
        true
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
    /// signaling [`Engine::cycle`] to return `CycleResult::Shutdown` so the
    /// driver can flush database saves and exit the process.
    pub(super) fn process_shutdown(&mut self) -> bool {
        for pid in self.player_list.pids() {
            let Some(active) = self.player_list.players[pid as usize].as_mut() else {
                continue;
            };
            if active.player.bot {
                self.remove_player(pid);
            } else if !active.player.logout_sent {
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
