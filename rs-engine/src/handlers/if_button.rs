use crate::active_player::ActivePlayer;
#[cfg(since_530)]
use crate::core::engine;
use crate::core::engine_mut;
use crate::handlers::ClientGameHandler;
#[cfg(all(since_530, before_647))]
use crate::handlers::op_common::obj_at;
use crate::handlers::op_common::{component, component_protected, require_held, transmitted_inv};
#[cfg(since_530)]
use crate::handlers::op_common::{component_events, other_inv};
#[cfg(before_647)]
use crate::watch::Op;
#[cfg(before_647)]
use rs_pack::types::IfButtonType;
#[cfg(before_647)]
use rs_protocol::network::game::client::{
    if_button::IfButton, inv_button1::InvButton1, inv_button2::InvButton2, inv_button3::InvButton3,
    inv_button4::InvButton4, inv_button5::InvButton5,
};
#[cfg(since_530)]
use rs_protocol::network::game::client::{
    if_button1::IfButton1, if_button2::IfButton2, if_button3::IfButton3, if_button4::IfButton4,
    if_button5::IfButton5, if_button6::IfButton6, if_button7::IfButton7, if_button8::IfButton8,
    if_button9::IfButton9, if_button10::IfButton10,
};
use rs_ver::TypeId;
#[cfg(since_530)]
use rs_vm::engine::ScriptEngine;
use rs_vm::state::ExecutionState;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ComId, ScriptError};

#[cfg(before_647)]
impl ClientGameHandler for IfButton {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        #[cfg(before_530)]
        let com = self.com.into();
        #[cfg(since_530)]
        let com = self.com;
        active.watch_button(com);
        handle(0, com, u16::MAX, TypeId::NULL, false, active)
    }
}

#[cfg(before_647)]
macro_rules! impl_inv_button_handler {
    ($($ty:ty => $op:expr),* $(,)?) => {
        $(
            impl ClientGameHandler for $ty {
                fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
                    #[cfg(before_530)]
                    let com = self.com.into();
                    #[cfg(since_530)]
                    let com = self.com;
                    active.watch_op(Op::InvButton {
                        op: $op, obj: self.obj, slot: self.slot, com,
                    });
                    handle($op, com, self.slot, self.obj, false, active)
                }
            }
        )*
    };
}

/// IF_BUTTON1-10. The 530 client leaves the obj out: it is the one in the
/// slot of the inventory transmitted to the component.
#[cfg(since_530)]
macro_rules! impl_if_button_handler {
    ($($ty:ty => $op:expr),* $(,)?) => {
        $(
            impl ClientGameHandler for $ty {
                fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
                    #[cfg(all(since_530, before_647))]
                    let obj = obj_at(active, self.com, self.slot);
                    #[cfg(since_647)]
                    let obj = self.obj;
                    handle($op, self.com, self.slot, obj, true, active)
                }
            }
        )*
    };
}

#[cfg(before_647)]
impl_inv_button_handler!(
    InvButton1 => 1, InvButton2 => 2, InvButton3 => 3, InvButton4 => 4, InvButton5 => 5,
);
#[cfg(since_530)]
impl_if_button_handler!(
    IfButton1 => 1, IfButton2 => 2, IfButton3 => 3, IfButton4 => 4, IfButton5 => 5,
    IfButton6 => 6, IfButton7 => 7, IfButton8 => 8, IfButton9 => 9, IfButton10 => 10,
);

/// A button on an if1 component (IF_BUTTON and INV_BUTTON1-5, before 647)
/// or on an if3 one (IF_BUTTON1-10, from 530); 530 has both.
fn handle(
    op: u8,
    com: ComId,
    slot: u16,
    obj: TypeId,
    if3: bool,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    #[cfg(before_647)]
    if !if3 && op != 0 && active.player.state.delayed {
        return Ok(());
    }

    let _interface = component(active, com)?;
    #[cfg(before_647)]
    if !if3 {
        if op == 0 {
            if _interface.button_type == IfButtonType::None {
                return Err(ScriptError::Client(format!(
                    "Interface has no button type: {com}"
                )));
            }
        } else if _interface
            .iop
            .as_ref()
            .and_then(|ops| ops.get(op as usize - 1))
            .is_none_or(Option::is_none)
        {
            return Err(ScriptError::Client(format!(
                "No iop option {op} for interface with id: {com}"
            )));
        }
    }
    #[cfg(since_530)]
    if if3 && component_events(active, com, slot).is_none_or(|events| !events.allows_op(op)) {
        return Err(ScriptError::Client(format!(
            "Interface option {op} is disabled: {com}, slot {slot}"
        )));
    }

    let inventory_click = if if3 {
        transmitted_inv(active, com).is_some() && slot != u16::MAX
    } else {
        op != 0
    };
    #[cfg(since_530)]
    if if3 && !obj.is_null() && active.player.state.delayed {
        return Ok(());
    }
    if inventory_click && (active.player.state.delayed || !require_held(active, com, slot, obj)?) {
        return Ok(());
    }
    #[cfg(since_530)]
    if if3 && !inventory_click && slot != u16::MAX {
        if active.player.inv_other_transmits.contains_key(&com) {
            let held = other_inv(active, com)
                .is_some_and(|inv| inv.valid_slot(slot) && inv.has_at(slot, obj));
            if !held {
                return Ok(());
            }
        }
    }
    // A slot can draw an obj with no inventory behind it: a clientscript's `cc_setobject` (the
    // pouches of Summoning Pouch Creation, 672:16, drawn by clientscript 794), and the client
    // sends that obj with the click. Nothing on the server vouches for it, so the click runs
    // with the slot alone and `last_item` stays null.
    #[cfg(since_530)]
    let obj = if if3
        && !inventory_click
        && slot != u16::MAX
        && !active.player.inv_other_transmits.contains_key(&com)
    {
        TypeId::NULL
    } else {
        obj
    };
    #[cfg(since_530)]
    if if3 && !obj.is_null() && engine().objs().get_by_id(obj).is_none() {
        return Err(ScriptError::Client(format!("Unknown obj: {obj}")));
    }

    active.player.last_com = Some(com);
    if op != 0 {
        active.player.last_slot = (slot != u16::MAX).then_some(slot);
        active.player.last_item = obj.non_null();
    }

    let uid = active.player.uid;
    if (if3 || op <= 1)
        && active
            .player
            .resume_buttons
            .as_ref()
            .is_some_and(|buttons| buttons.contains(&com))
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

    let trigger = match op {
        0 => Some(ServerTriggerType::IfButton),
        _ => ServerTriggerType::inv_button(op),
    };
    let Some(trigger) = trigger else {
        return Err(ScriptError::Client(format!("Invalid interface op: {op}")));
    };
    let protect = component_protected(active, com);
    let result = engine_mut().run_script_by_trigger(
        (trigger, Some(com), None),
        Some(ScriptSubject::Player(uid)),
        None,
        Some(protect),
        None,
        None,
    );
    #[cfg(since_530)]
    let result = if if3 && op == 1 && matches!(result, Err(ScriptError::TriggerNotFound(_))) {
        engine_mut().run_script_by_trigger(
            (ServerTriggerType::IfButton, Some(com), None),
            Some(ScriptSubject::Player(uid)),
            None,
            Some(protect),
            None,
            None,
        )
    } else {
        result
    };

    match result {
        Err(ScriptError::TriggerNotFound(_)) => {
            #[cfg(debug_assertions)]
            {
                let trigger = match op {
                    0 => "if_button".to_owned(),
                    _ if if3 => format!("if_button{op}"),
                    _ => format!("inv_button{op}"),
                };
                active.message_game(&format!(
                    "No trigger for [{trigger},{}]",
                    active.component_name(com),
                ));
            }
        }
        other => other?,
    }
    Ok(())
}
