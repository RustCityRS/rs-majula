use crate::active_player::EnginePlayer;
use crate::core::Engine;
use crate::phases::logout::TIMEOUT_NO_RESPONSE;
use crate::phases::shared::panic_message;
use std::panic::{AssertUnwindSafe, catch_unwind};
use tracing::{error, info};

impl Engine {
    /// Runs the lobby's players, who are in no world phase: drops each whose
    /// client left or went quiet for [`TIMEOUT_NO_RESPONSE`] ticks, then reads
    /// the rest's requests and sends what they have waiting. A lobby player
    /// is never saved: the lobby changes nothing that lasts.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`, after every phase that can write to a
    /// lobby player (logins and their `[login,_]` run in `logins` and `saves`).
    pub(crate) fn lobbies(&mut self) {
        let clock = self.clock;
        for pid in self.player_list.lobby.clone() {
            let result = catch_unwind(AssertUnwindSafe(|| {
                let Some(active) = self.player_list.players[pid as usize].as_mut() else {
                    return false;
                };
                if active.handle.disconnect_rx.try_recv().is_ok()
                    || active.player.disconnected_at.is_some()
                    || clock - active.player.last_response >= TIMEOUT_NO_RESPONSE
                {
                    return true;
                }
                if active.decode() {
                    active.player.last_response = clock;
                }
                active.encode();
                false
            }));
            let leave = result.unwrap_or_else(|panic| {
                error!(
                    "panic in the lobby for pid {pid}: {}",
                    panic_message(&panic)
                );
                true
            });
            if leave && let Some(active) = self.player_list.remove(pid) {
                info!(
                    "Player '{}' left the lobby (pid={pid})",
                    active.player.uid.username()
                );
            }
        }
    }
}
