use crate::active_player::ActivePlayer;
use crate::core::{engine, engine_mut};
use rs_entity::{InteractionTarget, Loc, Obj};
use rs_grid::CoordGrid;
use rs_inv::Inventory;
use rs_pack::cache::r#if::IfType;
#[cfg(since_647)]
use rs_pack::types::IfEvents;
use rs_pack::types::InvScope;
use rs_vm::engine::ScriptEngine;
use rs_vm::{ComId, ScriptError};

const BUILD_AREA_RADIUS: i32 = 52;

/// `ComActionTarget` bits: which kinds of target a spell component may be cast on.
///
/// Used by the `OpXxxT` handlers to reject components the client should never
/// have been able to aim at the target in question.
pub mod action_target {
    pub const OBJ: u16 = 0x1;
    pub const NPC: u16 = 0x2;
    pub const LOC: u16 = 0x4;
    pub const PLAYER: u16 = 0x8;
    #[cfg(before_647)]
    pub const HELD: u16 = 0x10;
    #[cfg(since_647)]
    pub const HELD: u16 = 0x20;
    #[cfg(since_647)]
    pub const COORD: u16 = 0x40;
}

pub fn component(active: &ActivePlayer, com: ComId) -> Result<&'static IfType, ScriptError> {
    let interface = engine()
        .interfaces()
        .get_by_id(com)
        .ok_or_else(|| ScriptError::Client(format!("No interface with id: {com}")))?;
    #[cfg(before_647)]
    let visible = active.player.is_interface_visible(interface.root_layer);
    #[cfg(since_647)]
    let visible = active.player.is_interface_open((com as u32 >> 16) as u16);
    if !visible {
        return Err(ScriptError::Client(format!(
            "Interface is not visible: {}",
            interface.root_layer
        )));
    }
    Ok(interface)
}

pub fn component_protected(active: &ActivePlayer, com: ComId) -> bool {
    #[cfg(before_647)]
    {
        let _ = active;
        engine()
            .interfaces()
            .get_by_id(com)
            .and_then(|interface| engine().interfaces().get_by_id(interface.root_layer))
            .is_some_and(|root| !root.overlay)
    }
    #[cfg(since_647)]
    {
        let mut iface = (com as u32 >> 16) as u16;
        for _ in 0..16 {
            if [
                active.player.modal_main,
                active.player.modal_side,
                active.player.modal_chat,
            ]
            .contains(&Some(iface))
            {
                return true;
            }
            let Some(parent) = active
                .player
                .open_subs
                .iter()
                .find(|(_, (sub, _))| *sub == iface)
                .map(|(&parent, _)| (parent as u32 >> 16) as u16)
            else {
                break;
            };
            iface = parent;
        }
        false
    }
}

#[cfg(since_647)]
pub fn component_events(active: &ActivePlayer, com: ComId, slot: u16) -> Option<IfEvents> {
    let interface = component(active, com).ok()?;
    active
        .player
        .if_events
        .get(&(com, slot))
        .copied()
        .or_else(|| (slot == u16::MAX).then_some(interface.active_properties))
        .map(IfEvents)
}

/// Returns `true` if the tile is inside the player's build area.
///
/// # Arguments
///
/// * `active` - The active player whose build area is being tested.
/// * `x` - The X coordinate of the tile.
/// * `z` - The Z coordinate of the tile.
///
/// # Returns
///
/// `true` if the tile is within [`BUILD_AREA_RADIUS`] of the build-area origin
/// on both axes, `false` otherwise (i.e. the tile is not visible on the client).
pub fn in_build_area(active: &ActivePlayer, x: u16, z: u16) -> bool {
    let origin_x = active.player.build_area.origin.x() as i32;
    let origin_z = active.player.build_area.origin.z() as i32;
    let (x, z) = (x as i32, z as i32);
    x >= origin_x - BUILD_AREA_RADIUS
        && x <= origin_x + BUILD_AREA_RADIUS
        && z >= origin_z - BUILD_AREA_RADIUS
        && z <= origin_z + BUILD_AREA_RADIUS
}

/// Looks up a placed location in the zone that contains the given tile.
///
/// # Arguments
///
/// * `x` - The X coordinate of the location.
/// * `y` - The level of the location.
/// * `z` - The Z coordinate of the location.
/// * `id` - The location type ID to match.
///
/// # Returns
///
/// The matching [`Loc`], or `None` if the zone is not loaded or holds no such
/// location.
pub fn zone_loc(x: u16, y: u8, z: u16, id: u16) -> Option<&'static Loc> {
    engine().zones.loc_at(x, y, z, id)
}

/// Looks up a ground object in the zone that contains the given tile.
///
/// # Arguments
///
/// * `x` - The X coordinate of the ground object.
/// * `y` - The level of the ground object.
/// * `z` - The Z coordinate of the ground object.
/// * `id` - The object type ID to match.
/// * `receiver37` - The base-37 username of the player looking, so that
///   receiver-only objects belonging to someone else stay invisible.
///
/// # Returns
///
/// The matching [`Obj`], or `None` if the zone is not loaded or holds no such
/// object visible to this receiver.
pub fn zone_obj(x: u16, y: u8, z: u16, id: u16, receiver37: u64) -> Option<&'static Obj> {
    engine().zones.obj_at(x, y, z, id, Some(receiver37))
}

/// Builds the [`InteractionTarget::Loc`] for a placed location.
///
/// Id, dimensions, shape, angle, layer and serial all come from the placed
/// location itself. The zone lookup already matched on the loc's *current* id, and every
/// add/change path keeps the packed width/length in sync with that id, so no loc
/// type lookup is needed here.
///
/// # Arguments
///
/// * `coord` - The coordinate the location occupies.
/// * `loc` - The placed location, as found in the zone.
///
/// # Returns
///
/// The interaction target describing this location.
pub fn loc_target(coord: CoordGrid, loc: &Loc) -> InteractionTarget {
    InteractionTarget::Loc {
        coord,
        id: loc.id(),
        width: loc.width(),
        length: loc.length(),
        shape: loc.shape(),
        angle: loc.angle(),
        layer: loc.layer(),
        serial: loc.serial(),
    }
}

/// Returns `true` if a component may be used as the source of a "use item on
/// target" interaction.
///
/// # Arguments
///
/// * `active` - The active player whose open interfaces are checked.
/// * `com` - The interface component ID the client claims to have used.
///
/// # Returns
///
/// `true` if the component exists, is flagged usable, and is currently visible
/// to the player; `false` otherwise (a bad or lagging client).
#[cfg(before_647)]
pub fn use_component_ok(active: &ActivePlayer, com: ComId, _: u16, _: u16) -> bool {
    component(active, com).is_ok_and(|interface| interface.usable)
}

#[cfg(since_647)]
pub fn use_component_ok(active: &ActivePlayer, com: ComId, slot: u16, target: u16) -> bool {
    component_events(active, com, slot).is_some_and(|events| events.allows_target(target))
}

/// Returns `true` if a spell component may be cast on the given kind of target.
///
/// # Arguments
///
/// * `active` - The active player whose open interfaces are checked.
/// * `com` - The spell component ID the client claims to have cast.
/// * `target` - The [`action_target`] bit for the kind of target being aimed at.
///
/// # Returns
///
/// `true` if the component exists, accepts this target kind, and is currently
/// visible to the player; `false` otherwise (a bad or lagging client).
#[cfg(before_647)]
pub fn spell_component_ok(active: &ActivePlayer, com: ComId, _: u16, target: u16) -> bool {
    component(active, com).is_ok_and(|interface| interface.action_target & target != 0)
}

#[cfg(since_647)]
pub fn spell_component_ok(active: &ActivePlayer, com: ComId, slot: u16, target: u16) -> bool {
    use_component_ok(active, com, slot, target)
}

pub fn held_component_ok(active: &ActivePlayer, com: ComId, slot: u16) -> bool {
    #[cfg(before_647)]
    {
        use_component_ok(active, com, slot, action_target::HELD)
    }
    #[cfg(since_647)]
    {
        component_events(active, com, slot).is_some_and(IfEvents::is_targetable)
    }
}

pub fn inventory(active: &mut ActivePlayer, inv_id: u16) -> Option<&mut Inventory> {
    if engine()
        .invs()
        .get_by_id(inv_id)
        .is_some_and(|inv| inv.scope == InvScope::Shared)
    {
        engine_mut().get_shared_inv_mut(inv_id)
    } else {
        active.player.invs.get_mut(&inv_id)
    }
}

/// Returns `true` if an NPC is a valid interaction target for the player.
///
/// # Arguments
///
/// * `active` - The active player whose build area is checked.
/// * `nid` - The NPC instance ID (slot index in the engine's NPC list).
///
/// # Returns
///
/// `true` if the NPC exists, is not delayed, and is visible on the player's
/// client; `false` otherwise.
pub fn npc_target_ok(active: &ActivePlayer, nid: u16) -> bool {
    let Some((delayed, id)) = engine()
        .get_npc(nid)
        .map(|n| (n.npc.state.delayed, n.npc.uid.id()))
    else {
        return false;
    };
    !delayed && active.player.build_area.npcs.contains(nid) && active.shown_npc(id).is_some()
}

/// Returns `true` if another player is a valid interaction target.
///
/// # Arguments
///
/// * `active` - The active player whose build area is checked.
/// * `pid` - The player ID (slot index) of the target player.
///
/// # Returns
///
/// `true` if the target player exists and is visible on the active player's
/// client; `false` otherwise.
pub fn player_target_ok(active: &ActivePlayer, pid: u16) -> bool {
    engine_mut().get_player(pid).is_some() && active.player.build_area.players.contains(pid)
}

/// Rejects members-only items on a free-to-play world, notifying the player.
///
/// # Arguments
///
/// * `active` - The active player attempting to use the item.
/// * `obj` - The object type ID of the item being used.
///
/// # Returns
///
/// `true` if the interaction was blocked, `false` if it may continue.
///
/// # Side Effects
///
/// When blocked, sends the members-server game message and unsets the map flag.
pub fn members_blocked(active: &mut ActivePlayer, obj: u16) -> bool {
    let engine = engine();
    if engine.objs().get_by_id(obj).is_some_and(|o| o.members) && !engine.members {
        active.message_game("To use this item please login to a members' server.");
        active.unset_map_flag();
        return true;
    }
    false
}

/// The inventory transmitted to component `com`, if any.
pub fn transmitted_inv(active: &ActivePlayer, com: ComId) -> Option<u16> {
    active
        .player
        .inv_transmits
        .iter()
        .find(|(_, coms)| coms.contains(&com))
        .map(|(id, _)| *id)
}

/// Outcome of [`check_held`].
///
/// The `U` handler family treats every failure the same way; the `OpHeld*`
/// family reports each one with its own client error, which is why the failure
/// cases stay distinguishable here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeldCheck {
    Ok,
    NoTransmit,
    NoInv(u16),
    InvalidSlot,
    NotHeld,
}

/// Verifies that the player really holds `obj` at `slot` in the inventory that
/// is transmitted to interface component `com`.
///
/// Resolves the component to a transmitted inventory, picks the shared or the
/// per-player copy according to the inventory type scope, then bounds-checks the
/// slot and confirms its contents.
///
/// # Arguments
///
/// * `active` - The active player whose inventories are checked.
/// * `com` - The interface component ID the item was operated on through.
/// * `slot` - The inventory slot index the client claimed.
/// * `obj` - The object type ID the client claimed is in that slot.
///
/// # Returns
///
/// [`HeldCheck::Ok`] when the item is really there, otherwise the specific
/// reason the claim was rejected.
pub fn check_held(active: &mut ActivePlayer, com: ComId, slot: u16, obj: u16) -> HeldCheck {
    let Some(inv_id) = transmitted_inv(active, com) else {
        return HeldCheck::NoTransmit;
    };

    let Some(inventory) = inventory(active, inv_id) else {
        return HeldCheck::NoInv(inv_id);
    };

    if !inventory.valid_slot(slot) {
        return HeldCheck::InvalidSlot;
    }

    if !inventory.has_at(slot, obj) {
        return HeldCheck::NotHeld;
    }

    HeldCheck::Ok
}

pub fn require_held(
    active: &mut ActivePlayer,
    com: ComId,
    slot: u16,
    obj: u16,
) -> Result<bool, ScriptError> {
    match check_held(active, com, slot, obj) {
        HeldCheck::Ok => Ok(true),
        HeldCheck::NotHeld => Ok(false),
        HeldCheck::NoTransmit => Err(ScriptError::Client(format!(
            "No inv transmit for interface with id: {com}"
        ))),
        HeldCheck::NoInv(inv) => Err(ScriptError::Client(format!(
            "Inv {inv} not found for com: {com}"
        ))),
        HeldCheck::InvalidSlot => Err(ScriptError::Client(format!("Invalid slot: {slot}"))),
    }
}
