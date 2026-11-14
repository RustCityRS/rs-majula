use crate::active_player::ActivePlayer;
use crate::core::engine_mut;
use crate::handlers::ClientGameHandler;
#[cfg(since_530)]
use crate::handlers::op_common::component_events;
use crate::handlers::op_common::{component, component_protected, inventory, transmitted_inv};
#[cfg(since_530)]
use rs_protocol::network::game::client::if_buttond::IfButtonD;
#[cfg(before_647)]
use rs_protocol::network::game::client::inv_buttond::InvButtonD;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ComId, ScriptError};

#[cfg(before_647)]
impl ClientGameHandler for InvButtonD {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        #[cfg(before_530)]
        let com = self.com.into();
        #[cfg(since_530)]
        let com = self.com;
        handle(com, self.slot, com, self.slot2, false, active)
    }
}

#[cfg(since_530)]
impl ClientGameHandler for IfButtonD {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(self.com, self.slot, self.com2, self.slot2, true, active)
    }
}

/// A drag on an if1 inventory (INV_BUTTOND, before 647) or between if3
/// components (IF_BUTTOND, from 530); 530 has both.
fn handle(
    com: ComId,
    slot: u16,
    com2: ComId,
    slot2: u16,
    if3: bool,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    let _interface = component(active, com)?;
    component(active, com2)?;
    #[cfg(before_647)]
    if !if3 && !_interface.draggable {
        return Err(ScriptError::Client(format!(
            "Interface is not draggable: {com}"
        )));
    }
    #[cfg(since_530)]
    if if3
        && (component_events(active, com, slot).is_none_or(|events| !events.is_draggable())
            || component_events(active, com2, slot2).is_none_or(|events| !events.is_drop_target()))
    {
        return Err(ScriptError::Client(format!(
            "Interface drag is disabled: {com}, slot {slot} to {com2}, slot {slot2}"
        )));
    }

    let source_inv = transmitted_inv(active, com);
    #[cfg(before_647)]
    if !if3 && source_inv.is_none() {
        return Err(ScriptError::Client(format!(
            "No inv transmit for interface with id: {com}"
        )));
    }
    let _item = if let Some(inv) = source_inv {
        let inventory = inventory(active, inv)
            .ok_or_else(|| ScriptError::Client(format!("Inv {inv} not found for com: {com}")))?;
        if !inventory.valid_slot(slot) {
            return Err(ScriptError::Client(format!("Invalid slot: {slot}")));
        }
        let Some(item) = inventory.get(slot) else {
            resync(active, com, slot, com2, slot2);
            return Ok(());
        };
        Some(item.obj)
    } else {
        None
    };
    if let Some(inv) = transmitted_inv(active, com2) {
        #[cfg(before_530)]
        let check_slot = true;
        #[cfg(since_530)]
        let check_slot = !if3
            || component_events(active, com2, slot2).is_some_and(|events| events.is_draggable());
        if check_slot {
            let inventory = inventory(active, inv).ok_or_else(|| {
                ScriptError::Client(format!("Inv {inv} not found for com: {com2}"))
            })?;
            if !inventory.valid_slot(slot2) {
                return Err(ScriptError::Client(format!("Invalid slot: {slot2}")));
            }
        }
    }
    if active.player.state.delayed {
        resync(active, com, slot, com2, slot2);
        return Ok(());
    }

    active.player.last_slot = Some(slot);
    active.player.last_target_slot = Some(slot2);
    #[cfg(since_530)]
    {
        active.player.last_com = Some(com);
        active.player.last_target_com = Some(com2);
        active.player.last_item = _item;
    }
    let protect = component_protected(active, com);
    match engine_mut().run_script_by_trigger(
        (ServerTriggerType::InvButtonD, Some(com), None),
        Some(ScriptSubject::Player(active.player.uid)),
        None,
        Some(protect),
        None,
        None,
    ) {
        Err(ScriptError::TriggerNotFound(_)) => {
            #[cfg(debug_assertions)]
            {
                let trigger = if if3 { "if_buttond" } else { "inv_buttond" };
                active.message_game(&format!(
                    "No trigger for [{trigger},{}]",
                    active.component_name(com)
                ));
            }
        }
        other => other?,
    }
    Ok(())
}

fn resync(active: &mut ActivePlayer, com: ComId, slot: u16, com2: ComId, slot2: u16) {
    #[cfg(before_530)]
    {
        let _ = com2;
        if let Some(inv) = transmitted_inv(active, com).and_then(|id| inventory(active, id)) {
            let partial = [
                (slot, inv.get(slot).map(|item| (item.obj, item.num as i32))),
                (
                    slot2,
                    inv.get(slot2).map(|item| (item.obj, item.num as i32)),
                ),
            ];
            active.update_inv_partial(com as u16, &partial);
        }
    }
    #[cfg(since_530)]
    {
        let source = transmitted_inv(active, com);
        let target = transmitted_inv(active, com2);
        for id in [source, target.filter(|inv| Some(*inv) != source)]
            .into_iter()
            .flatten()
        {
            if let Some(inv) = inventory(active, id) {
                let mut slots: Vec<_> = [(source, slot), (target, slot2)]
                    .into_iter()
                    .filter(|&(owner, slot)| owner == Some(id) && inv.valid_slot(slot))
                    .map(|(_, slot)| (slot, inv.get(slot).map(|item| (item.obj, item.num as i32))))
                    .collect();
                slots.dedup_by_key(|(slot, _)| *slot);
                if !slots.is_empty() {
                    #[cfg(all(since_530, before_647))]
                    active.update_inv_partial(
                        if Some(id) == source { com } else { com2 },
                        id,
                        &slots,
                    );
                    #[cfg(since_647)]
                    active.update_inv_partial(id.g2(), &slots);
                }
            }
        }
    }
}
