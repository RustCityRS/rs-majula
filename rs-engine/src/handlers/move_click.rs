use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_grid::CoordGrid;
use rs_protocol::network::game::client::move_gameclick::MoveGameClick;
use rs_protocol::network::game::client::move_minimapclick::MoveMinimapClick;
#[cfg(before_647)]
use rs_protocol::network::game::client::move_opclick::MoveOpClick;
#[cfg(since_647)]
use rs_protocol::network::game::client::pack_coord;
use rs_protocol::network::game::client::unpack_coord;
use rs_vm::ScriptError;
use rsmod::rsmod::collision::collision_strategy::CollisionType;

/// Handles the `MoveGameClick` client protocol message.
///
/// Processes a movement request initiated by clicking on the game viewport.
/// This is a non-operation click (i.e., a pure movement without an associated
/// entity interaction).
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
#[cfg(before_647)]
impl ClientGameHandler for MoveGameClick {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        active.watch_walk(&self.path, false);
        handle(self.path, self.ctrl, false, active)
    }
}

/// Handles the rev-647 `MoveGameClick` client protocol message.
///
/// The rev-647 client sends only the destination tile (no waypoint list);
/// the server pathfinds to it. Mapped onto the shared [`handle`] as a
/// single-coordinate path.
#[cfg(since_647)]
impl ClientGameHandler for MoveGameClick {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(vec![pack_coord(self.x, self.z)], self.ctrl, false, active)
    }
}

/// Handles the `MoveMinimapClick` client protocol message.
///
/// Processes a movement request initiated by clicking on the minimap.
/// This is a non-operation click (i.e., a pure movement without an associated
/// entity interaction).
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
#[cfg(before_647)]
impl ClientGameHandler for MoveMinimapClick {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        active.watch_walk(&self.path, true);
        active.watch_minimap(&self.path, &self.trailer);
        handle(self.path, self.ctrl, false, active)
    }
}

/// Handles the rev-647 `MoveMinimapClick` client protocol message.
///
/// Destination-only, same as [`MoveGameClick`].
#[cfg(since_647)]
impl ClientGameHandler for MoveMinimapClick {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(vec![pack_coord(self.x, self.z)], self.ctrl, false, active)
    }
}

/// Handles the `MoveOpClick` client protocol message.
///
/// Processes a movement request initiated as part of an entity operation
/// (e.g., walking to an NPC or object before interacting). Unlike game/minimap
/// clicks, this does not clear pending actions, as the movement is a precursor
/// to an interaction.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
#[cfg(before_647)]
impl ClientGameHandler for MoveOpClick {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(self.path, self.ctrl, true, active)
    }
}

/// Shared handler for all movement click types (game click, minimap click, operation click),
/// following the reference engine's `MoveClickHandler`.
///
/// Validates the path is non-empty and within 104 tiles of the player, then
/// queues waypoints for the player's pathing system. When the engine uses
/// client-side pathfinding, the full client path is used directly and the walk
/// trigger runs; otherwise the server computes a path to the final destination
/// using the rsmod pathfinder.
///
/// Every click sets temporary run (ctrl-click to run); only non-operation clicks
/// clear pending actions.
///
/// # Arguments
///
/// * `path` - The packed coordinate path from the client.
/// * `ctrl` - Whether the ctrl key was held (used for run toggling).
/// * `op` - Whether this movement is part of an operation click (`true`) or a
///   pure movement click (`false`).
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success or if the path is empty/out of range.
///
/// # Side Effects
///
/// * Updates the player's waypoint queue and sets `move_clicked`, which the
///   input phase reads (see `Engine::post_process`).
/// * Sets the temp run flag. For non-op clicks, clears pending actions.
/// * With client-side pathfinding, processes the walk trigger.
///
/// # Call Stack
///
/// **Called by:** `MoveGameClick::handle`, `MoveMinimapClick::handle`, `MoveOpClick::handle`
/// **Calls:** `ActivePlayer::clear_pending_action`, `ActivePlayer::process_walktrigger`,
/// `rsmod::find_path` (server-side)
fn handle(
    path: Vec<u32>,
    ctrl: bool,
    op: bool,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    if path.is_empty() {
        return Ok(());
    }

    if active.player.state.delayed {
        active.unset_map_flag();
        return Ok(());
    }

    let y = active.player.pathing.coord.y();

    let (x, z) = unpack_coord(path[0]);
    if !active
        .player
        .pathing
        .coord
        .in_distance(CoordGrid::new(x, y, z), 104)
    {
        active.clear_waypoints();
        active.player.move_clicked = false;
        return Ok(());
    }

    // Not for op clicks: the op packet that follows clears and sets the
    // interaction itself.
    if !op {
        active.clear_pending_action()?;
    }

    // ctrl-click temporarily runs for this move; a normal click leaves
    // temp-run off (so movement follows the persistent run toggle). Low
    // energy (<100) cancels a ctrl temp-run.
    #[cfg(before_647)]
    {
        active.player.temprun = if active.player.runenergy < 100 && ctrl {
            false
        } else {
            ctrl
        };
    }
    // Rev-647: ctrl flips the run orb for this walk; a plain click follows it.
    #[cfg(since_647)]
    active.player.click_temprun(ctrl);

    active.player.move_clicked = true;

    if engine().client_pathfinder {
        let path: Vec<u32> = path
            .into_iter()
            .map(|p| {
                let (px, pz) = unpack_coord(p);
                CoordGrid::new(px, y, pz).packed()
            })
            .collect();
        active.player.pathing.queue_waypoints(&path);
        active.process_walktrigger();
    } else {
        let (dest_x, dest_z) = unpack_coord(path[path.len() - 1]);
        active.player.pathing.queue_waypoints(rsmod::find_path(
            y,
            active.player.pathing.coord.x(),
            active.player.pathing.coord.z(),
            dest_x,
            dest_z,
            1,
            1,
            1,
            0,
            -1,
            true,
            0,
            25,
            CollisionType::Normal,
        ));
    }

    Ok(())
}
