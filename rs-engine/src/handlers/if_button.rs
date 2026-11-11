use crate::active_player::ActivePlayer;
#[cfg(since_647)]
use crate::core::engine;
use crate::core::engine_mut;
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{component, component_protected, require_held};
#[cfg(since_647)]
use crate::handlers::op_common::{component_events, transmitted_inv};
#[cfg(before_647)]
use crate::watch::Op;
#[cfg(before_647)]
use rs_pack::types::IfButtonType;
#[cfg(before_647)]
use rs_protocol::network::game::client::{
    if_button::IfButton, inv_button1::InvButton1, inv_button2::InvButton2, inv_button3::InvButton3,
    inv_button4::InvButton4, inv_button5::InvButton5,
};
#[cfg(since_647)]
use rs_protocol::network::game::client::{
    if_button1::IfButton1, if_button2::IfButton2, if_button3::IfButton3, if_button4::IfButton4,
    if_button5::IfButton5, if_button6::IfButton6, if_button7::IfButton7, if_button8::IfButton8,
    if_button9::IfButton9, if_button10::IfButton10,
};
#[cfg(since_647)]
use rs_vm::engine::ScriptEngine;
use rs_vm::state::ExecutionState;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ComId, ScriptError};

#[cfg(before_647)]
impl ClientGameHandler for IfButton {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        active.watch_button(self.com);
        handle(0, self.com.into(), u16::MAX, u16::MAX, active)
    }
}

macro_rules! impl_button_handler {
    ($($ty:ty => $op:expr),* $(,)?) => {
        $(
            impl ClientGameHandler for $ty {
                fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
                    #[cfg(before_647)]
                    active.watch_op(Op::InvButton {
                        op: $op, obj: self.obj, slot: self.slot, com: self.com,
                    });
                    #[cfg(before_647)]
                    let com = self.com.into();
                    #[cfg(since_647)]
                    let com = self.com;
                    handle($op, com, self.slot, self.obj, active)
                }
            }
        )*
    };
}

#[cfg(before_647)]
impl_button_handler!(
    InvButton1 => 1, InvButton2 => 2, InvButton3 => 3, InvButton4 => 4, InvButton5 => 5,
);
#[cfg(since_647)]
impl_button_handler!(
    IfButton1 => 1, IfButton2 => 2, IfButton3 => 3, IfButton4 => 4, IfButton5 => 5,
    IfButton6 => 6, IfButton7 => 7, IfButton8 => 8, IfButton9 => 9, IfButton10 => 10,
);

fn handle(
    op: u8,
    com: ComId,
    slot: u16,
    obj: u16,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    #[cfg(before_647)]
    if op != 0 && active.player.state.delayed {
        return Ok(());
    }

    let _interface = component(active, com)?;
    #[cfg(before_647)]
    {
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
    #[cfg(since_647)]
    if component_events(active, com, slot).is_none_or(|events| !events.allows_op(op)) {
        return Err(ScriptError::Client(format!(
            "Interface option {op} is disabled: {com}, slot {slot}"
        )));
    }

    #[cfg(before_647)]
    let inventory_click = op != 0;
    #[cfg(since_647)]
    let inventory_click = transmitted_inv(active, com).is_some() && slot != u16::MAX;
    #[cfg(since_647)]
    if obj != u16::MAX && active.player.state.delayed {
        return Ok(());
    }
    if inventory_click && (active.player.state.delayed || !require_held(active, com, slot, obj)?) {
        return Ok(());
    }
    #[cfg(since_647)]
    if !inventory_click && slot != u16::MAX {
        if let Some(&(uid, inv)) = active.player.inv_other_transmits.get(&com) {
            let held = engine()
                .get_player((uid & 0x7FF) as u16)
                .filter(|source| source.player.uid.packed() as i32 == uid)
                .and_then(|source| source.player.invs.get(&inv))
                .is_some_and(|inv| inv.valid_slot(slot) && inv.has_at(slot, obj));
            if !held {
                return Ok(());
            }
        } else if obj != u16::MAX {
            return Err(ScriptError::Client(format!(
                "No inv transmit for interface with id: {com}"
            )));
        }
    }
    #[cfg(since_647)]
    if obj != u16::MAX && engine().objs().get_by_id(obj).is_none() {
        return Err(ScriptError::Client(format!("Unknown obj: {obj}")));
    }

    active.player.last_com = Some(com);
    if op != 0 {
        active.player.last_slot = (slot != u16::MAX).then_some(slot);
        active.player.last_item = (obj != u16::MAX).then_some(obj);
    }

    let uid = active.player.uid;
    // A 647 component sends whichever of its ten ops was clicked, and a resume button is not
    // always op 1: quest_intro's "Click here to show" is op 10.
    #[cfg(before_647)]
    let resumes = op <= 1;
    #[cfg(since_647)]
    let resumes = true;
    if resumes
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
        0 => ServerTriggerType::IfButton,
        1 => ServerTriggerType::InvButton1,
        2 => ServerTriggerType::InvButton2,
        3 => ServerTriggerType::InvButton3,
        4 => ServerTriggerType::InvButton4,
        5 => ServerTriggerType::InvButton5,
        #[cfg(since_647)]
        6 => ServerTriggerType::InvButton6,
        #[cfg(since_647)]
        7 => ServerTriggerType::InvButton7,
        #[cfg(since_647)]
        8 => ServerTriggerType::InvButton8,
        #[cfg(since_647)]
        9 => ServerTriggerType::InvButton9,
        #[cfg(since_647)]
        10 => ServerTriggerType::InvButton10,
        _ => return Err(ScriptError::Client(format!("Invalid interface op: {op}"))),
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
    #[cfg(since_647)]
    let result = if op == 1 && matches!(result, Err(ScriptError::TriggerNotFound(_))) {
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
                    #[cfg(before_647)]
                    _ => format!("inv_button{op}"),
                    #[cfg(since_647)]
                    _ => format!("if_button{op}"),
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
