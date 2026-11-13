use crate::active_player::ActivePlayer;
use crate::core::engine_mut;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::tut_clickside::TutClickSide;
use rs_vm::ScriptError;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `TutClickSide` client protocol message.
///
/// Sent when the player clicks one of the side tabs while the tutorial is active.
/// Validates the tab index is in range (0..=13), then runs the `Tutorial` server
/// trigger script so content can react to the click. If no tutorial script is
/// registered, the handler is a no-op.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success, if the tab index is out of range, or if no tutorial
///   trigger is registered.
/// * `Err(ScriptError)` if the tutorial script raises an execution error.
///
/// # Side Effects
///
/// * Runs the `ServerTriggerType::Tutorial` script for this player.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `engine_mut().run_script_by_trigger`
impl ClientGameHandler for TutClickSide {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if self.tab > 13 {
            // bad client: tab index is out of range
            return Ok(());
        }

        let result = engine_mut().run_script_by_trigger(
            (ServerTriggerType::Tutorial, None, None),
            Some(ScriptSubject::Player(active.player.uid)),
            None,
            Some(true),
            None,
            None,
        );

        match result {
            Err(ScriptError::TriggerNotFound(_)) => {}
            other => other?,
        }

        Ok(())
    }
}
