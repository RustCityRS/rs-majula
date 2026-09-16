use crate::active_player::ActivePlayer;
use crate::engine::{engine, engine_mut};
use rs_entity::{InteractionTarget, Loc, Obj};
use rs_grid::CoordGrid;
use rs_pack::types::InvScope;
use rs_vm::engine::ScriptEngine;

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
    pub const HELD: u16 = 0x10;
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
    let zone = engine().zones.zone(x, y, z)?;
    let idx = zone.get_loc(x, z, id)?;
    Some(&zone.locs[idx])
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
    let zone = engine().zones.zone(x, y, z)?;
    let idx = zone.get_obj(x, z, id, Some(receiver37))?;
    Some(&zone.objs[idx])
}

/// Builds the [`InteractionTarget::Loc`] for a placed location.
///
/// Dimensions come from the location type when it is known, falling back to a
/// 1x1 footprint when it is not; shape, angle and layer come from the placed
/// location itself.
///
/// # Arguments
///
/// * `id` - The location type ID.
/// * `coord` - The coordinate the location occupies.
/// * `loc` - The placed location, as found in the zone.
///
/// # Returns
///
/// The interaction target describing this location.
pub fn loc_target(id: u16, coord: CoordGrid, loc: &Loc) -> InteractionTarget {
    let loc_type = engine().locs().get_by_id(id);
    InteractionTarget::Loc {
        coord,
        id,
        width: loc_type.map(|lt| lt.width).unwrap_or(1),
        length: loc_type.map(|lt| lt.length).unwrap_or(1),
        shape: loc.shape(),
        angle: loc.angle(),
        layer: loc.layer(),
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
pub fn use_component_ok(active: &ActivePlayer, com: u16) -> bool {
    let Some(interface) = engine().interfaces().get_by_id(com) else {
        return false;
    };
    interface.usable && active.player.is_interface_visible(interface.root_layer)
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
pub fn spell_component_ok(active: &ActivePlayer, com: u16, target: u16) -> bool {
    let Some(interface) = engine().interfaces().get_by_id(com) else {
        return false;
    };
    interface.action_target & target != 0
        && active.player.is_interface_visible(interface.root_layer)
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
    let Some(delayed) = engine().get_npc(nid).map(|n| n.npc.state.delayed) else {
        return false;
    };
    !delayed && active.player.build_area.npcs.contains(nid)
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
pub fn check_held(active: &mut ActivePlayer, com: u16, slot: u16, obj: u16) -> HeldCheck {
    let inv_id = active
        .player
        .inv_transmits
        .iter()
        .find(|(_, coms)| coms.contains(&com))
        .map(|(id, _)| *id);

    let Some(inv_id) = inv_id else {
        return HeldCheck::NoTransmit;
    };

    let shared = engine()
        .invs()
        .get_by_id(inv_id)
        .is_some_and(|t| t.scope == InvScope::Shared);

    let Some(inventory) = (if shared {
        engine_mut().get_shared_inv_mut(inv_id)
    } else {
        active.player.invs.get_mut(&inv_id)
    }) else {
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
