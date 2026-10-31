use crate::active_player::EnginePlayer;
use crate::engine::Engine;
use rs_vm::engine::ScriptPlayer;
use rs_vm::pointer::ScriptPointer;
use rs_vm::state::{QueuePriority, ScriptArgument, ScriptState};
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use tracing::{error, info};

/// Ticks a disconnected player remains in-world before a logout is requested
/// (30 seconds). The grace window exists so a client that lost its connection
/// can reconnect (login type 18) and adopt the session instead of being
/// logged out -- mirroring the reference engine's `TIMEOUT_NO_CONNECTION`.
/// The request path respects logout prevention (e.g. combat), and is
/// re-requested every tick after the window while the player stays
/// disconnected.
const TIMEOUT_NO_CONNECTION: u32 = 50;

/// Ticks without any inbound packet before a player is force-logged-out
/// (60 seconds), bypassing logout prevention`. This is the backstop for silently dead
/// connections (no FIN/RST ever arrives, so no disconnect signal fires): a
/// live client always sends keepalives well within this window. Bots are
/// exempt (they never send packets).
pub(crate) const TIMEOUT_NO_RESPONSE: u32 = 100;

/// Ticks without any real player action (a successful `UserEvent`: movement,
/// interaction, button, chat, ...) before an idle logout is *requested*
/// (5 minutes). This is a server-authoritative backstop that does NOT trust
/// the client's `IdleTimer` packet: a connected client spamming only
/// keepalives/passive packets never refreshes `last_active`, so it is still
/// logged out here.
///
/// The authentic client self-reports idle at ~90s of zero input (4500 cycles
/// at 50 Hz, `Client.idleCycles > 4500` -> sends `IdleTimer`), but it resets
/// on bare mouse movement. The server cannot count mouse movement, because the
/// mouse/tracking packets are `ClientEvent`s a modified client could spoof to
/// stay alive -- the exact evasion this closes -- so `last_active` keys on real
/// actions only. That makes the server timer stricter per event, hence a
/// window wider than 90s: 5 minutes sits above typical non-combat
/// power-skilling gaps between actions while still being a real control.
///
/// Unlike [`TIMEOUT_NO_RESPONSE`] this respects logout prevention (it routes
/// through `logout_idle_requested`, so a player in combat or another gated
/// auto-action is not yanked out), and it re-requests every tick while idle.
/// Honest clients still self-logout earlier via their own `IdleTimer`; this
/// only catches clients that suppress it. Tune here if it is too aggressive.
const TIMEOUT_NO_INPUT: u32 = 500;

impl Engine {
    /// Processes the logout phase of the engine tick cycle.
    ///
    /// Iterates over every active player and handles pending disconnection
    /// or voluntary logout:
    ///
    /// 1. Checks the player's `disconnect_rx` channel and records the
    ///    disconnect tick in `disconnected_at`. The player lingers in-world
    ///    for [`TIMEOUT_NO_CONNECTION`] ticks (reconnect grace window)
    ///    before a logout is requested.
    /// 2. Players with no inbound traffic for [`TIMEOUT_NO_RESPONSE`] ticks
    ///    are force-logged-out, bypassing logout prevention.
    /// 3. If a logout is already in progress (`logout_sent`), the player is
    ///    marked for removal.
    /// 4. If a logout is requested but temporarily prevented (e.g. in
    ///    combat), the prevention message is shown and the request is
    ///    cleared.
    /// 5. Otherwise, calls `active.logout()` to begin the logout sequence.
    ///
    /// For each player marked for removal, the engine:
    ///
    /// * Closes any open modal interface.
    /// * Checks whether the player's queues allow immediate removal.
    /// * Executes the `Logout` server trigger script.
    /// * Persists the player profile, notifying the ether service once the
    ///   save lands (see [`Engine::save_logout`]).
    /// * Removes the player from the engine.
    ///
    /// # Side Effects
    ///
    /// * Sends save requests to the database.
    /// * Sends logout notifications to the ether service.
    /// * Removes player slots from `self.players` and `self.active_players`.
    /// * Executes the `Logout` RuneScript trigger.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `remove_player`, `runescript_vm_execute`, `extract_profile`,
    ///   `save_binary`
    pub(crate) fn logouts(&mut self) {
        let mut removals = Vec::new();

        for &pid in self.player_list.processing.iter() {
            let Some(active) = self.player_list.players[pid as usize].as_mut() else {
                continue;
            };

            if active.player.disconnected_at.is_none()
                && active.handle.disconnect_rx.try_recv().is_ok()
            {
                active.player.disconnected_at = Some(self.clock);
            }

            let mut force = false;
            if !active.player.bot {
                force = self.clock - active.player.last_response >= TIMEOUT_NO_RESPONSE;
                if let Some(at) = active.player.disconnected_at
                    && self.clock - at >= TIMEOUT_NO_CONNECTION
                {
                    active.player.logout_idle_requested = true;
                }
                if self.clock.saturating_sub(active.player.last_active) >= TIMEOUT_NO_INPUT {
                    active.player.logout_idle_requested = true;
                }
            }

            if active.player.logout_sent {
                removals.push(pid);
            } else if force {
                active.logout();
            } else if active.player.logout_requested || active.player.logout_idle_requested {
                if let Some(until) = active.player.logout_prevented_until
                    && self.clock < until
                {
                    if active.player.logout_requested
                        && let Some(msg) = active.player.logout_prevented_message.take()
                    {
                        active.message_game(&msg);
                    }
                    active.player.logout_requested = false;
                    active.player.logout_idle_requested = false;
                    continue;
                }
                active.logout();
            }
        }

        if removals.is_empty() {
            return;
        }

        for &index in &removals {
            let idx = index as usize;

            let (uid, user37, can_access, queue_discard, no_head) = {
                let Some(active) = &mut self.player_list.players[idx] else {
                    continue;
                };
                if let Err(e) = active.close_modal(true) {
                    error!(
                        "error closing modal during logout for player {}: {e}",
                        active.player.uid.pid()
                    );
                }
                let mut queue_discard = true;
                let mut h = active.player.state.queues.queue.head();
                while let Some(idx) = h {
                    if active.player.state.queues.queue[idx].priority == QueuePriority::Long {
                        if let Some(ScriptArgument::Int(1)) = active.player.state.queues.queue[idx]
                            .args
                            .as_ref()
                            .and_then(|a| a.first())
                        {
                            h = active.player.state.queues.queue.next();
                            continue;
                        }
                    }
                    queue_discard = false;
                    break;
                }
                (
                    active.player.uid,
                    active.uid().username37(),
                    active.can_access(),
                    queue_discard,
                    active.player.state.queues.engine.head().is_none(),
                )
            };

            if can_access && no_head && queue_discard {
                match self.script_by_key(ServerTriggerType::Logout, None, None) {
                    Some(script) => {
                        let mut state =
                            ScriptState::init(script, Some(ScriptSubject::Player(uid)), None, None);
                        state.pointers.add(ScriptPointer::ProtectedActivePlayer);
                        self.runescript_vm_execute(&mut state);

                        if self.player_list.players[idx].is_some() {
                            self.save_logout(index);
                        } else {
                            self.release_logout(user37);
                        }
                        self.remove_player(index);
                        info!(
                            "Player '{}' disconnected, removed from uid={:?}, pid={}",
                            uid.username(),
                            uid,
                            uid.pid()
                        );
                    }
                    None => error!("LOGOUT TRIGGER IS BROKEN!"),
                }
            }
        }
    }
}
