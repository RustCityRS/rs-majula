use crate::active_player::ActivePlayer;
use crate::engine::engine_mut;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::resume_p_string_dialog::ResumePStringDialog;
use rs_vm::ScriptError;
use rs_vm::state::ExecutionState;
use rs_vm::subject::ScriptSubject;

/// Handles the rev-647 `ResumePStringDialog` client protocol message.
///
/// The client sends the string the player typed into a name/text entry.
/// Resumes the active script suspended by `p_stringdialog`, binding the text
/// as `last_string`.
///
/// # Returns
///
/// * `Ok(())` on success.
/// * `Err(ScriptError)` if no active script is suspended on a string dialog.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `engine_mut().run_script_by_state`
impl ClientGameHandler for ResumePStringDialog {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let Some(state) = &mut active.player.state.active_script else {
            return Err(ScriptError::Client(
                "Player does not have an active script!".to_string(),
            ));
        };

        if state.execution != ExecutionState::StringDialog {
            return Err(ScriptError::Client(
                "Player does not have an active string dialog!".to_string(),
            ));
        }

        state.last_string = Some(self.text.clone());
        engine_mut().run_script_by_state(
            (**state).clone(),
            Some(ScriptSubject::Player(active.player.uid)),
            Some(true),
            Some(true),
        )
    }
}
