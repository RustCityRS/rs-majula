use crate::active_player::ActivePlayer;
#[cfg(debug_assertions)]
use crate::core::engine;
use crate::core::engine_mut;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::if_buttond::IfButtonD;
use rs_vm::ScriptError;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;

/// Handles `IfButtonD` — rev-647's interface drag (opcode 37).
///
/// This is what a player dragging one inventory slot onto another produces. The
/// packet was already decoded but never dispatched, so the drag was silently
/// discarded: items would not rearrange, and no message said why.
///
/// The rev-647 shape differs from the pre-647 `InvButtonD` in two ways, which is
/// why this is a separate handler rather than a cfg arm on that one:
///
/// * Components are the full 32-bit hash (`iface << 16 | com`), matching the rest
///   of the 647 interface surface, so there is no `cache().interfaces` lookup by
///   a bare component id to do here.
/// * The client sends both endpoints, `com`/`com2` and the obj in each
///   slot. That is what lets a 647 drag cross interfaces (bank to inventory);
///   pre-647 could only drag within one.
///
/// Validation is deliberately left to the trigger. The script knows which
/// containers it is willing to move between; the engine's job is to deliver the
/// four numbers and let the content decide, the same way `handle_if_button` does.
///
/// # Side Effects
///
/// * Sets `last_com`, `last_slot`, `last_target_slot`, `last_target_com`
///   and `last_item`.
/// * Runs the `InvButtonD` trigger keyed on the source component.
impl ClientGameHandler for IfButtonD {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let com = self.com as i32;
        // Both ends of the drag must be on screen, as `InvButtonD` requires
        // before 647.
        for end in [self.com, self.com2] {
            let iface = (end >> 16) as u16;
            if !active.player.is_interface_open(iface) {
                return Err(ScriptError::Client(format!(
                    "Interface is not visible: {iface}"
                )));
            }
        }
        active.player.last_com = com;
        active.player.last_slot = self.slot as i32;
        active.player.last_target_slot = Some(self.slot2);
        active.player.last_target_com = Some(self.com2 as i32);
        active.player.last_item = self.obj as i32;

        // As before 647 (`InvButtonD`: protected unless its root is an
        // overlay), a drag on a modal runs protected: the bank's tab drops
        // rearrange it. The modal is one the server opened as such, as for
        // `IfButton`.
        let iface = (self.com >> 16) as u16;
        let protect = [
            active.player.modal_main,
            active.player.modal_side,
            active.player.modal_chat,
        ]
        .contains(&Some(iface));

        let uid = active.player.uid;
        match engine_mut().run_script_by_trigger(
            (ServerTriggerType::InvButtonD, Some(com), None),
            Some(ScriptSubject::Player(uid)),
            None,
            Some(protect),
            None,
            None,
        ) {
            Err(ScriptError::TriggerNotFound(_)) => {
                #[cfg(debug_assertions)]
                active.message_game(&format!(
                    "No trigger for [inv_buttond,{}]",
                    engine().layout.component_name(com)
                ));
            }
            other => other?,
        }
        Ok(())
    }
}
