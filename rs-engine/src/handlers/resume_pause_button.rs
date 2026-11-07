use crate::active_player::ActivePlayer;
use crate::engine::engine_mut;
use crate::handlers::ClientGameHandler;
use crate::watch::Op;
use rs_protocol::network::game::client::resume_pause_button::ResumePauseButton;
use rs_vm::ScriptError;
use rs_vm::state::ExecutionState;
use rs_vm::subject::ScriptSubject;

/// Handles the `ResumePauseButton` client protocol message.
///
/// Resumes execution of the player's active script that was paused waiting for a
/// generic button press (e.g., "click here to continue" dialogs). Validates that
/// the player has an active script in the `PauseButton` execution state before
/// resuming it.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success.
/// * `Err(ScriptError::Client)` if the player has no active script or the script
///   is not in the `PauseButton` state.
///
/// # Side Effects
///
/// * Records the clicked component in `last_com` (parity with `IfButton`).
/// * Resumes the paused script execution via `run_script_by_state`.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `engine_mut().run_script_by_state`
impl ClientGameHandler for ResumePauseButton {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        active.watch_op(Op::Other);
        active.watch_note("Click here to continue");
        #[cfg(before_647)]
        {
            active.player.last_com = Some(self.com);
        }
        let Some(state) = &active.player.state.active_script else {
            return Err(ScriptError::Client(
                "Player does not have an active script!".to_string(),
            ));
        };

        if state.execution != ExecutionState::PauseButton {
            return Err(ScriptError::Client(
                "Player does not have an active pause button!".to_string(),
            ));
        }
        let state = (**state).clone();

        // The 647 client names the pause button it clicked. A dialogue of
        // several -- a choice, an item to make -- reads that as `last_com`;
        // left alone it would still hold whatever button came before.
        #[cfg(since_647)]
        {
            let iface = (self.com >> 16) as u16;
            if !active.player.is_interface_open(iface) {
                return Err(ScriptError::Client(format!(
                    "Interface is not visible: {iface}"
                )));
            }
            active.player.last_com = self.com as i32;
        }

        engine_mut().run_script_by_state(
            state,
            Some(ScriptSubject::Player(active.player.uid)),
            Some(true),
            Some(true),
        )
    }
}
