use crate::active_player::ActivePlayer;
#[cfg(any(before_647, debug_assertions))]
use crate::core::engine;
use crate::core::engine_mut;
use crate::handlers::ClientGameHandler;
#[cfg(before_647)]
use rs_pack::types::IfButtonType;
#[cfg(before_647)]
use rs_protocol::network::game::client::if_button::IfButton;
#[cfg(since_647)]
use rs_protocol::network::game::client::{
    if_button1::IfButton1, if_button2::IfButton2, if_button3::IfButton3, if_button4::IfButton4,
    if_button5::IfButton5, if_button6::IfButton6, if_button7::IfButton7, if_button8::IfButton8,
    if_button9::IfButton9, if_button10::IfButton10,
};
use rs_vm::ScriptError;
#[cfg(before_647)]
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
#[cfg(before_647)]
impl ClientGameHandler for IfButton {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        active.watch_button(self.com);
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

        active.player.last_com = Some(self.com.into());

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
                (ServerTriggerType::IfButton, Some(self.com as i32), None),
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

/// Maps the IfButton packet op (1-10) to its `if_button{op}` server trigger.
/// Rev-647 merged OpHeld/InvButton into IfButton1-10; the engine names this
/// trigger family `InvButton{op}` (bytes 149-153, 173-177) — see
/// `ServerTriggerType`.
#[cfg(since_647)]
fn op_trigger(op: usize) -> Option<ServerTriggerType> {
    Some(match op {
        1 => ServerTriggerType::InvButton1,
        2 => ServerTriggerType::InvButton2,
        3 => ServerTriggerType::InvButton3,
        4 => ServerTriggerType::InvButton4,
        5 => ServerTriggerType::InvButton5,
        6 => ServerTriggerType::InvButton6,
        7 => ServerTriggerType::InvButton7,
        8 => ServerTriggerType::InvButton8,
        9 => ServerTriggerType::InvButton9,
        10 => ServerTriggerType::InvButton10,
        _ => return None,
    })
}

/// Handles the rev-647 `IfButton{1..10}` client protocol messages.
///
/// Rev-647 addresses interface components by a 32-bit component hash
/// (`iface << 16 | com`) and carries the clicked slot and obj in the same
/// packet. If the player has a script paused on a button wait and the
/// clicked component is in the resume-button set, the paused script is
/// resumed. Otherwise the op-specific `if_button{op}` trigger is fired for
/// the component (e.g. `[if_button1,inventory:inv]`), falling back to the
/// generic op-less `if_button` trigger for components that don't
/// distinguish ops (character creation / options use
/// `[if_button,iface:comp]`).
///
/// # Side Effects
///
/// * Sets `last_com`, `last_slot`, and `last_item` on the player.
/// * May resume a paused script if the button matches `resume_buttons`.
/// * May run the `if_button{op}` or `if_button` trigger script.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `engine_mut().run_script_by_state` (resume), `engine_mut().run_script_by_trigger`
#[cfg(since_647)]
fn handle_if_button(
    op: usize,
    com: i32,
    slot: i32,
    obj: i32,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    // As before 647: a button on an interface that is not on screen is a bad
    // client or lag, and nothing runs.
    let iface = (com as u32 >> 16) as u16;
    if !active.player.is_interface_open(iface) {
        return Err(ScriptError::Client(format!(
            "Interface is not visible: {iface}"
        )));
    }

    active.player.last_com = Some(com);
    active.player.last_slot = u16::try_from(slot).ok();
    active.player.last_item = u16::try_from(obj).ok();

    let uid = active.player.uid;

    // "Click here to continue" — resume a script paused on this button.
    if active
        .player
        .resume_buttons
        .as_ref()
        .is_some_and(|b| b.contains(&com))
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
        return Ok(());
    }

    // As before 647, a button on a modal runs protected and one on an overlay
    // does not (289: its root interface's `overlay`, true for the tabs and
    // if_openoverlay's). 2011's cache marks no interface an overlay, and the
    // server opens tabs and side modals alike as type-1 subs, so the modal is
    // one the server opened as such: main, side or chat.
    let protect = [
        active.player.modal_main,
        active.player.modal_side,
        active.player.modal_chat,
    ]
    .contains(&Some(iface));

    if let Some(trigger) = op_trigger(op) {
        match engine_mut().run_script_by_trigger(
            (trigger, Some(com), None),
            Some(ScriptSubject::Player(uid)),
            None,
            Some(protect),
            None,
            None,
        ) {
            Err(ScriptError::TriggerNotFound(_)) => {} // fall through to generic if_button
            other => return other,
        }
    }

    match engine_mut().run_script_by_trigger(
        (ServerTriggerType::IfButton, Some(com), None),
        Some(ScriptSubject::Player(uid)),
        None,
        Some(protect),
        None,
        None,
    ) {
        Err(ScriptError::TriggerNotFound(_)) => {
            #[cfg(debug_assertions)]
            active.message_game(&format!(
                "No trigger for [if_button,{}]",
                engine().layout.component_name(com)
            ));
        }
        other => other?,
    }
    Ok(())
}

#[cfg(since_647)]
macro_rules! impl_if_button_handler {
    ($($ty:ty => $op:expr),* $(,)?) => {
        $(
            impl ClientGameHandler for $ty {
                fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
                    handle_if_button($op, self.com as i32, self.slot as i32, self.obj as i32, active)
                }
            }
        )*
    };
}

#[cfg(since_647)]
impl_if_button_handler!(
    IfButton1 => 1,
    IfButton2 => 2,
    IfButton3 => 3,
    IfButton4 => 4,
    IfButton5 => 5,
    IfButton6 => 6,
    IfButton7 => 7,
    IfButton8 => 8,
    IfButton9 => 9,
    IfButton10 => 10,
);
