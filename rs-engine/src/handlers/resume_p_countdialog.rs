use crate::active_player::ActivePlayer;
use crate::engine::engine_mut;
use crate::handlers::ClientGameHandler;
#[cfg(since_647)]
use rs_protocol::network::game::client::resume_p_count_dialog::ResumePCountDialog;
#[cfg(before_647)]
use rs_protocol::network::game::client::resume_p_countdialog::ResumePCountDialog;
use rs_vm::ScriptError;
use rs_vm::state::ExecutionState;
use rs_vm::subject::ScriptSubject;

/// Handles the `ResumePCountDialog` client protocol message.
///
/// Resumes execution of the player's active script that was paused waiting for
/// a numeric input dialog (e.g., "enter amount" for banking). Validates that the
/// player has an active script in the `CountDialog` execution state, stores the
/// player's input (clamped to non-negative), and resumes the script.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success.
/// * `Err(ScriptError::Client)` if the player has no active script or the script
///   is not in the `CountDialog` state.
///
/// # Side Effects
///
/// * Sets `state.last_int` to the clamped input value.
/// * Resumes the paused script execution via `run_script_by_state`.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `engine_mut().run_script_by_state`
impl ClientGameHandler for ResumePCountDialog {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        #[cfg(before_647)]
        let dialog_value = self.input;
        #[cfg(since_647)]
        let dialog_value = self.value;
        let Some(state) = &mut active.player.state.active_script else {
            return Err(ScriptError::Client(
                "Player does not have an active script!".to_string(),
            ));
        };

        if state.execution != ExecutionState::CountDialog {
            return Err(ScriptError::Client(
                "Player does not have an active count dialog!".to_string(),
            ));
        }

        state.last_int = Some(dialog_value.clamp(0, i32::MAX));
        engine_mut().run_script_by_state(
            (**state).clone(),
            Some(ScriptSubject::Player(active.player.uid)),
            Some(true),
            Some(true),
        )
    }
}
