//! Rev-647 targeted ops: an item or a spell used on a loc, npc, ground obj or
//! player.
//!
//! 225/289 send a separate packet per source: `OpLocU` for an item, `OpLocT`
//! for a spell, and so on. 647 sends one packet per *target* (`OpLocT`,
//! `OpNpcT`, `OpObjT`, `OpPlayerT`) that carries whichever source the player
//! had selected (`Js5TextureSource.set_target`). The source decides which
//! trigger family answers, so the rules here are the pre-647 `OpXU` and
//! `OpXT` handlers' own:
//!
//! * an **item** drives `[ap|op]<target>u`, with `last_useitem` and
//!   `last_useslot` recorded, after checking the player's transmitted
//!   inventory really holds it at that slot;
//! * a **spell** drives `[ap|op]<target>t`, keyed on the spell's component.
//!
//! Not yet checked here, as the pre-647 handlers check it: that the source
//! component accepts this kind of target. 647 grants that per component with
//! IF_SETEVENTS, which the server does not keep yet.

use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::{engine, engine_mut};
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{loc_target, zone_loc, zone_obj};
use rs_entity::InteractionTarget;
use rs_grid::CoordGrid;
use rs_pack::types::InvScope;
use rs_protocol::network::game::client_prot::{OpLocT, OpNpcT, OpObjT, OpPlayerT};
use rs_vm::ScriptError;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};
use rs_vm::trigger::ServerTriggerType;

/// The `object_id` the client sends for a source component that holds no
/// item: `AClass3.anInt123` is the component's `object_id`, -1 unless an
/// item is in it, written with `p2`.
const NO_OBJ: u16 = u16::MAX;

/// What the player had selected before clicking the target.
enum Source {
    Item { obj: u16, slot: u16, com: i32 },
    Spell { com: i32 },
}

/// Resolves the selected source. `None` when an item source is not what the
/// player's transmitted inventory holds at that slot of that component --
/// a stale or forged click, dropped as the pre-647 `OpXU` handlers drop it.
fn source(
    active: &mut ActivePlayer,
    from_com: u32,
    from_slot: u16,
    from_obj: u16,
) -> Option<Source> {
    let com = from_com as i32;
    // bad client or lag: the source's interface is not on screen
    if !active.player.is_interface_open((from_com >> 16) as u16) {
        return None;
    }
    if from_obj == NO_OBJ {
        return Some(Source::Spell { com });
    }

    // bad client or lag: inventory is not transmitted to client
    let inv_id = active
        .player
        .inv_transmits
        .iter()
        .find(|(_, coms)| coms.contains(&com))
        .map(|(id, _)| *id)?;
    let shared = engine()
        .cache
        .invs
        .get_by_id(inv_id)
        .is_some_and(|t| t.scope == InvScope::Shared);
    let inventory = if shared {
        engine_mut().get_shared_inv_mut(inv_id)
    } else {
        active.player.invs.get_mut(&inv_id)
    }?;

    // bad client: real inventory is smaller; or lag: the item has moved
    (inventory.valid_slot(from_slot) && inventory.has_at(from_slot, from_obj)).then_some(
        Source::Item {
            obj: from_obj,
            slot: from_slot,
            com,
        },
    )
}

/// Starts the interaction a resolved source drives.
///
/// `subject_is_item` keys an item's trigger on the item itself, as
/// `[opplayeru,<obj>]` is: a player target has no type of its own to key on.
fn interact(
    active: &mut ActivePlayer,
    target: InteractionTarget,
    source: Source,
    item_trigger: ServerTriggerType,
    spell_trigger: ServerTriggerType,
    subject_is_item: bool,
) -> Result<(), ScriptError> {
    active.clear_pending_action()?;
    match source {
        Source::Item { obj, slot, com } => {
            if engine()
                .cache
                .objs
                .get_by_id(obj)
                .is_some_and(|o| o.members)
                && !engine().members
            {
                active.message_game("To use this item please login to a members' server.");
                active.unset_map_flag();
                return Ok(());
            }
            active.player.last_use_item = Some(obj);
            active.player.last_use_slot = Some(slot);
            // The item's component, which the interaction checks it is
            // still held in until it runs (`use_item_held`).
            active.player.last_use_com = Some(com);
            active
                .player
                .set_interaction(target, item_trigger as u8, true);
            if subject_is_item {
                active.player.interaction.target_subject_com = Some(obj as i32);
            }
        }
        Source::Spell { com } => {
            active
                .player
                .set_interaction(target, spell_trigger as u8, true);
            active.player.interaction.target_subject_com = Some(com);
        }
    }
    active.player.opcalled = true;
    Ok(())
}

/// Whether a target tile is within the 104x104 scene the client has loaded.
fn in_scene(active: &ActivePlayer, x: u16, z: u16) -> bool {
    let origin_x = active.player.build_area.origin.x() as i32;
    let origin_z = active.player.build_area.origin.z() as i32;
    (origin_x - 52..=origin_x + 52).contains(&(x as i32))
        && (origin_z - 52..=origin_z + 52).contains(&(z as i32))
}

/// Drops the click: clears the client's destination flag and does nothing.
fn drop_click(active: &mut ActivePlayer) -> Result<(), ScriptError> {
    active.unset_map_flag();
    Ok(())
}

impl ClientGameHandler for OpLocT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed {
            // normal: cannot interact while delayed
            return drop_click(active);
        }
        if !in_scene(active, self.x, self.z) {
            // bad client: tile is not visible on client
            return drop_click(active);
        }

        let y = active.player.pathing.coord.y();
        let Some(loc) = zone_loc(self.x, y, self.z, self.loc_id) else {
            // bad client or lag: loc does not exist
            return drop_click(active);
        };
        let target = loc_target(CoordGrid::new(self.x, y, self.z), loc);

        let Some(source) = source(active, self.from_com, self.from_slot, self.from_obj) else {
            return drop_click(active);
        };
        interact(
            active,
            target,
            source,
            ServerTriggerType::ApLocU,
            ServerTriggerType::ApLocT,
            false,
        )
    }
}

impl ClientGameHandler for OpNpcT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed {
            // normal: cannot interact while delayed
            return drop_click(active);
        }

        let Some(source) = source(active, self.from_com, self.from_slot, self.from_obj) else {
            return drop_click(active);
        };

        let Some(npc_delayed) = engine().get_npc(self.nid).map(|n| n.npc.state.delayed) else {
            // bad client or lag: npc does not exist
            return drop_click(active);
        };
        if npc_delayed {
            // normal: cannot interact with delayed npcs
            return drop_click(active);
        }
        if !active.player.build_area.npcs.contains(self.nid) {
            // bad client or lag: npc is not visible on client
            return drop_click(active);
        }

        interact(
            active,
            InteractionTarget::Npc { nid: self.nid },
            source,
            ServerTriggerType::ApNpcU,
            ServerTriggerType::ApNpcT,
            false,
        )
    }
}

impl ClientGameHandler for OpObjT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed {
            // normal: cannot interact while delayed
            return drop_click(active);
        }
        if !in_scene(active, self.x, self.z) {
            // bad client: tile is not visible on client
            return drop_click(active);
        }

        let y = active.player.pathing.coord.y();
        let receiver = active.uid().username37();
        let Some(obj) = zone_obj(self.x, y, self.z, self.obj_id, receiver) else {
            // bad client or lag: obj does not exist
            return drop_click(active);
        };
        let target = InteractionTarget::Obj {
            coord: CoordGrid::new(self.x, y, self.z),
            id: self.obj_id,
            serial: obj.serial(),
        };

        let Some(source) = source(active, self.from_com, self.from_slot, self.from_obj) else {
            return drop_click(active);
        };
        interact(
            active,
            target,
            source,
            ServerTriggerType::ApObjU,
            ServerTriggerType::ApObjT,
            false,
        )
    }
}

impl ClientGameHandler for OpPlayerT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed {
            // normal: cannot interact while delayed
            return drop_click(active);
        }

        let Some(source) = source(active, self.from_com, self.from_slot, self.from_obj) else {
            return drop_click(active);
        };

        if engine_mut().get_player(self.pid).is_none() {
            // bad client or lag: player does not exist
            return drop_click(active);
        }
        if !active.player.build_area.players.contains(self.pid) {
            // bad client or lag: player is not visible on client
            return drop_click(active);
        }

        interact(
            active,
            InteractionTarget::Player { pid: self.pid },
            source,
            ServerTriggerType::ApPlayerU,
            ServerTriggerType::ApPlayerT,
            true,
        )
    }
}
