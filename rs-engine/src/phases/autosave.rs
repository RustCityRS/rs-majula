use crate::clients::client_db::DbRequest;
use crate::engine::Engine;
use crate::player_save::{extract_profile, save_binary};
use rs_vm::engine::ScriptPlayer;
use tracing::info;

/// Number of ticks between autosave cycles. At 600ms per tick this is
/// approximately 150 seconds (~2.5 minutes).
const AUTOSAVE_INTERVAL: u32 = 250;

/// Number of ticks between `world_status` heartbeats (~10 seconds). The
/// website treats a world as offline once its heartbeat is 30 seconds old.
const STATUS_INTERVAL: u32 = 17;

impl Engine {
    /// Processes the autosave phase of the engine tick cycle.
    ///
    /// Every tick, increments each active player's `playtime` counter.
    ///
    /// Every [`STATUS_INTERVAL`] ticks, reports the number of (non-bot)
    /// players online via [`DbRequest::WorldStatus`], with the players this
    /// world has online or held mid-login/mid-logout, so the database can
    /// square its session claims with them.
    ///
    /// Every [`AUTOSAVE_INTERVAL`] ticks (excluding tick 0), extracts and
    /// Serializes every active player's profile and sends it to the
    /// database via [`DbRequest::Save`]. This provides periodic
    /// durability so that player progress is not lost on a crash.
    ///
    /// # Side Effects
    ///
    /// * Increments `playtime` on every active player every tick.
    /// * Sends `DbRequest::WorldStatus` at the status interval.
    /// * Sends `DbRequest::Save` for every active player at the autosave
    ///   interval.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `extract_profile`, `save_binary`, `DbRequest::Save`
    pub(crate) fn autosave(&mut self) {
        let mut players = 0;
        for &pid in self.player_list.processing.iter() {
            if let Some(active) = self.player_list.players[pid as usize].as_mut() {
                if !active.player.bot {
                    active.player.playtime += 1;
                    players += 1;
                }
            }
        }

        if self.clock.is_multiple_of(STATUS_INTERVAL)
            && let Some(tx) = &self.db_tx
        {
            let online = self
                .player_list
                .processing
                .iter()
                .filter_map(|&pid| self.player_list.players[pid as usize].as_ref())
                .filter(|active| !active.player.bot)
                .map(|active| active.uid().username37())
                .collect();
            let held = self
                .pending_logins
                .iter()
                .map(|p| p.user37)
                .chain(self.logout_saves.keys().copied())
                .collect();
            let _ = tx.send(DbRequest::WorldStatus {
                players,
                online,
                held,
            });
        }

        if !self.clock.is_multiple_of(AUTOSAVE_INTERVAL) || self.clock == 0 {
            return;
        }

        let mut count = 0;
        for &pid in self.player_list.processing.iter() {
            if let Some(active) = self.player_list.players[pid as usize].as_ref() {
                if active.player.bot {
                    continue;
                }
                if let Some(tx) = &self.db_tx {
                    let profile = extract_profile(&active.player, self.cache);
                    let binary = save_binary(&profile, self.cache);
                    let _ = tx.send(DbRequest::Save {
                        user37: active.uid().username37(),
                        username: active.uid().username(),
                        profile: Box::new(profile),
                        binary,
                        logout: false,
                    });
                }
                count += 1;
            }
        }

        if count > 0 {
            info!("Autosaved {} player(s)", count);
        }
    }
}
