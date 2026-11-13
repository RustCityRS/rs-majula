use crate::active_player::ActivePlayer;
use crate::core::engine_mut;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::window_status::WindowStatus;
use rs_vm::ScriptError;
use rs_vm::subject::ScriptSubject;

/// Handles `WindowStatus` — rev-647 WINDOW_STATUS (opcode 69).
///
/// The client sends this on login and again on every display-mode change or
/// window resize. It is the only way the server learns which gameframe the
/// client is actually rendering, and `getwindowmode` reads straight off it.
///
/// Leaving it undispatched was not harmless. `window_mode` stayed at its `1`
/// (fixed) default forever, so `[proc,open_game_frame]` always took the fixed
/// branch and mounted every tab — inventory included — as a subinterface of
/// `toplevel` (548). A client in resizable mode renders `toplevel_fullscreen`
/// (746) instead, so those subinterfaces were bound to a parent that was never
/// on screen: interface 149 never appeared in the drawn set, and no inventory
/// component could be hovered, clicked or used as a drag target.
///
/// Mode values are the client's own: 1 = fixed, 2 = resizable, 3 = fullscreen.
/// Content only distinguishes 1 from everything else, matching the `= 1` test
/// in the login scripts.
///
/// # Side Effects
///
/// * Sets `player.window_mode`, `player.window_width` and `player.window_height`.
/// * Runs `[proc,switch_window_mode]` when the mode changes after the gameframe
///   is already up, which re-roots every tab and orb under the new toplevel.
impl ClientGameHandler for WindowStatus {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let previous = active.player.window_mode;
        active.player.window_mode = self.mode;
        active.player.window_width = self.width;
        active.player.window_height = self.height;

        // `toplevel` is zero until IF_OPENTOP has run, which means login has not
        // reached `~open_game_frame` yet. It will read the mode we just stored,
        // so switching here would be redundant at best and would run against a
        // half-initialised player at worst.
        if previous == self.mode || active.player.toplevel == 0 {
            return Ok(());
        }

        let uid = active.player.uid;
        engine_mut().run_script_by_name(
            // Not `open_game_frame`: that re-opens every subinterface from
            // scratch, throwing away whatever the player had on screen. The
            // content proc moves them across with `if_movesub`, which is what
            // the live game does when you toggle display mode.
            // Compiled scripts are named by their header.
            "[proc,switch_window_mode]",
            Some(ScriptSubject::Player(uid)),
            None,
            None,
            None,
            None,
        )
    }
}
