use crate::active_player::ActivePlayer;
use crate::engine::{engine, engine_mut};
use crate::handlers::ClientGameHandler;
use rs_pack::types::IfButtonType;
use rs_protocol::network::game::client::if_button::IfButton;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;
use rs_vm::state::ExecutionState;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `IfButton` client protocol message.
///
/// Processes a click on a non-inventory interface button. Validates the interface
/// exists, has a button type, and is currently visible. If the player has an active
/// script paused on a button wait and the clicked component is in the resume button
/// set, the paused script is resumed. Otherwise, the handler looks up and runs the
/// `ServerTriggerType::IfButton` trigger script for the clicked component.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success or if no trigger is found (a game message is sent instead).
/// * `Err(ScriptError)` if the interface does not exist, has no button type,
///   is not visible, or a script execution error occurs.
///
/// # Side Effects
///
/// * Sets `last_com` on the player to the clicked component ID.
/// * May resume a paused script if the button matches `resume_buttons`.
/// * May run the `IfButton` trigger script for the component.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `engine_mut().run_script_by_state` (resume), `engine_mut().run_script_by_trigger`
impl ClientGameHandler for IfButton {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let interfaces = engine().interfaces();

        let Some(interface) = interfaces.get_by_id(self.com) else {
            return Err(ScriptError::Client(format!(
                "No interface with id: {}",
                self.com
            )));
        };

        if interface.button_type == IfButtonType::None {
            return Err(ScriptError::Client(format!(
                "Interface has no button type: {}",
                self.com
            )));
        }

        if !active.player.is_interface_visible(interface.root_layer) {
            return Err(ScriptError::Client(format!(
                "Interface is not visible: {}",
                interface.root_layer
            )));
        }

        active.player.last_com = Some(self.com);

        let uid = active.player.uid;
        if active
            .player
            .resume_buttons
            .as_ref()
            .is_some_and(|b| b.contains(&(self.com as i32)))
        {
            if let Some(state) = &active.player.state.active_script
                && state.execution == ExecutionState::PauseButton
            {
                engine_mut().run_script_by_state(
                    (**state).clone(),
                    Some(ScriptSubject::Player(uid)),
                    Some(true),
                    Some(true),
                )?;
            }
        } else {
            let protect = interfaces
                .get_by_id(interface.root_layer as u16)
                .is_some_and(|root| !root.overlay);

            let result = engine_mut().run_script_by_trigger(
                (ServerTriggerType::IfButton, Some(self.com), None),
                Some(ScriptSubject::Player(uid)),
                None,
                Some(protect),
                None,
                None,
            );

            match result {
                Err(ScriptError::TriggerNotFound(_)) => {
                    #[cfg(debug_assertions)]
                    active.message_game(&format!(
                        "No trigger for [if_button,{}]",
                        interface
                            .com_name
                            .as_deref()
                            .unwrap_or(&self.com.to_string())
                    ));
                }
                other => other?,
            }
        }

        Ok(())
    }
}
