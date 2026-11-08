//! Rev-647 `IfButtonT`: something selected used on an item in an inventory.
//!
//! 225/289 send `OpHeldU` for an item used on an item and `OpHeldT` for a
//! spell cast on one. 647 sends both as `IfButtonT`, carrying the selected
//! source (`com`, `slot`, `obj`) and the clicked target (`com2`, `slot2`, `obj2`).
//! The source decides the trigger as for the other targeted ops: an item drives
//! `[opheldu]`, a spell drives `[opheldt]` keyed on the spell's component.
//! The rules below are the pre-647 handlers' own.

use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::{engine, engine_mut};
use crate::handlers::ClientGameHandler;
use rs_pack::types::InvScope;
use rs_protocol::network::game::client_prot::IfButtonT;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;
use rs_vm::state::ScriptState;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;

/// The `object_id` the client sends for a source component that holds no
/// item, selecting a spell.
const NO_OBJ: u16 = u16::MAX;

/// Whether the inventory transmitted on `com` holds `obj` at `slot`.
fn holds(active: &mut ActivePlayer, com: i32, slot: u16, obj: u16) -> bool {
    let Some(inv_id) = active
        .player
        .inv_transmits
        .iter()
        .find(|(_, coms)| coms.contains(&com))
        .map(|(id, _)| *id)
    else {
        return false;
    };
    let shared = engine()
        .cache
        .invs
        .get_by_id(inv_id)
        .is_some_and(|t| t.scope == InvScope::Shared);
    let inventory = if shared {
        engine_mut().get_shared_inv_mut(inv_id)
    } else {
        active.player.invs.get_mut(&inv_id)
    };
    inventory.is_some_and(|inv| inv.valid_slot(slot) && inv.has_at(slot, obj))
}

impl ClientGameHandler for IfButtonT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed {
            // normal: cannot interact while delayed
            return Ok(());
        }
        for end in [self.com, self.com2] {
            let iface = (end >> 16) as u16;
            if !active.player.is_interface_open(iface) {
                return Err(ScriptError::Client(format!(
                    "Interface is not visible: {iface}"
                )));
            }
        }

        let com2 = self.com2 as i32;
        if !holds(active, com2, self.slot2, self.obj2) {
            // lag: the target has moved since the client drew it
            active.player.move_request = true;
            active.clear_pending_action()?;
            return Ok(());
        }

        if self.obj == NO_OBJ {
            spell_on_item(self, active)
        } else {
            item_on_item(self, active)
        }
    }
}

/// `[opheldt,<spell>]`, with the target as `last_item` / `last_slot`.
fn spell_on_item(packet: IfButtonT, active: &mut ActivePlayer) -> Result<(), ScriptError> {
    active.player.last_item = packet.obj2 as i32;
    active.player.last_slot = packet.slot2 as i32;
    active.clear_pending_action()?;
    active.player.info.clear_face_entity_player();

    let spell = packet.com as i32;
    let result = engine_mut().run_script_by_trigger(
        (ServerTriggerType::OpHeldT, Some(spell), None),
        Some(ScriptSubject::Player(active.player.uid)),
        None,
        Some(true),
        None,
        None,
    );
    match result {
        Err(ScriptError::TriggerNotFound(_)) => {
            #[cfg(debug_assertions)]
            active.message_game(&format!(
                "No trigger for [opheldt,{}]",
                engine().layout.component_name(spell)
            ));
            active.message_game("Nothing interesting happens.");
            Ok(())
        }
        other => other,
    }
}

/// `[opheldu]`: the target first, then the item used; then each one's
/// category. Whichever the script is keyed on becomes `last_item`, and the
/// other `last_useitem`.
fn item_on_item(packet: IfButtonT, active: &mut ActivePlayer) -> Result<(), ScriptError> {
    // Item-on-item works inside one inventory, as `OpHeldU` requires before 647.
    if packet.com != packet.com2 {
        return Ok(());
    }
    if !holds(active, packet.com as i32, packet.slot, packet.obj) {
        active.player.move_request = true;
        active.clear_pending_action()?;
        return Ok(());
    }
    let (Some(target), Some(used)) = (
        engine().cache.objs.get_by_id(packet.obj2),
        engine().cache.objs.get_by_id(packet.obj),
    ) else {
        return Err(ScriptError::Client(format!(
            "Unknown obj in ifbuttont: {} on {}",
            packet.obj, packet.obj2
        )));
    };

    active.player.last_item = target.id as i32;
    active.player.last_slot = packet.slot2 as i32;
    active.player.last_use_item = Some(used.id);
    active.player.last_use_slot = Some(packet.slot);
    active.clear_pending_action()?;
    active.player.info.clear_face_entity_player();

    if (target.members || used.members) && !engine().members {
        active.message_game("To use this item please login to a members' server.");
        return Ok(());
    }

    // Script lookup keys: trigger | (subject kind << 8) | (subject << 10),
    // kind 2 an obj and kind 1 a category -- as the pre-647 handler builds them.
    let base = ServerTriggerType::OpHeldU as i32;
    let by_obj = |id: u16| base | (0x2 << 8) | ((id as i32) << 10);
    let by_category = |cat: u16| base | (0x1 << 8) | ((cat as i32) << 10);
    let candidates = [
        (Some(by_obj(target.id)), false),
        (Some(by_obj(used.id)), true),
        (target.category.map(by_category), false),
        (used.category.map(by_category), true),
    ];
    let found = candidates.into_iter().find_map(|(key, swap)| {
        key.and_then(|k| engine_mut().scripts.get_by_lookup(k).cloned())
            .map(|script| (script, swap))
    });

    let Some((script, swap)) = found else {
        #[cfg(debug_assertions)]
        active.message_game(&format!(
            "No trigger for [opheldu,{}]",
            target.debugname().unwrap_or(&target.id.to_string())
        ));
        active.message_game("Nothing interesting happens.");
        return Ok(());
    };
    // Keyed on the used item: it is the script's subject, so it is the one
    // `last_item` names.
    if swap {
        let player = &mut active.player;
        let item = player.last_item;
        let slot = player.last_slot;
        player.last_item = used.id as i32;
        player.last_slot = packet.slot as i32;
        player.last_use_item = Some(item as u16);
        player.last_use_slot = Some(slot as u16);
    }

    let uid = active.player.uid;
    let state = ScriptState::init(script, Some(ScriptSubject::Player(uid)), None, None);
    engine_mut().run_script_by_state(state, Some(ScriptSubject::Player(uid)), Some(true), None)
}
