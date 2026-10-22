use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_grid::CoordGrid;
use rs_protocol::network::game::client::move_gameclick::MoveGameClick;
use rs_protocol::network::game::client::move_minimapclick::MoveMinimapClick;
use rs_protocol::network::game::client::move_opclick::MoveOpClick;
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
impl ClientGameHandler for MoveGameClick {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        active.watch_walk(&self.path, false);
        handle(self.path, self.ctrl, false, active)
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
impl ClientGameHandler for MoveMinimapClick {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        active.watch_walk(&self.path, true);
        active.watch_minimap(&self.path, &self.trailer);
        handle(self.path, self.ctrl, false, active)
    }
}

/// Handles the `MoveOpClick` client protocol message.
///
/// Processes a movement request initiated as part of an entity operation
/// (e.g., walking to an NPC or object before interacting). Unlike game/minimap
/// clicks, this does not clear pending actions or process walk triggers, as the
/// movement is a precursor to an interaction.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for MoveOpClick {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(self.path, self.ctrl, true, active)
    }
}

/// Shared handler for all movement click types (game click, minimap click, operation click).
///
/// Validates the path is non-empty and within 104 tiles of the player, then
/// queues waypoints for the player's pathing system. When the engine uses
/// client-side pathfinding, the full client path is used directly; otherwise
/// the server computes a path to the final destination using the rsmod pathfinder.
///
/// For non-operation clicks, this also clears pending actions, manages temporary
/// run state (ctrl-click to run), and processes walk triggers.
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
/// * Updates the player's waypoint queue.
/// * For non-op clicks: clears pending actions, sets temp run flag, processes walk triggers.
///
/// # Call Stack
///
/// **Called by:** `MoveGameClick::handle`, `MoveMinimapClick::handle`, `MoveOpClick::handle`
/// **Calls:** [`path_to_move_click`], `ActivePlayer::clear_pending_action`, `ActivePlayer::process_walktrigger`
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
        active.clear_waypoints();
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
        return Ok(());
    }

    let client_pathfinder = engine().client_pathfinder;

    if client_pathfinder {
        if path.len() != 1
            || x != active.player.pathing.coord.x()
            || z != active.player.pathing.coord.z()
        {
            active.player.path = Some(
                path.iter()
                    .map(|&p| {
                        let (px, pz) = unpack_coord(p);
                        CoordGrid::new(px, y, pz)
                    })
                    .collect(),
            );
        } else {
            active.clear_waypoints();
        }
    } else {
        let (dest_x, dest_z) = unpack_coord(path[path.len() - 1]);
        active.player.path = Some(vec![CoordGrid::new(dest_x, y, dest_z)]);
    }

    path_to_move_click(active, client_pathfinder);

    if !op {
        active.clear_pending_action()?;
        // ctrl-click temporarily runs for this move; a normal click leaves
        // temp-run off (so movement follows the persistent run toggle). Low
        // energy (<100) cancels a ctrl temp-run.
        active.player.temprun = if active.player.runenergy < 100 && ctrl {
            false
        } else {
            ctrl
        };
        if active.player.pathing.has_waypoints() {
            active.process_walktrigger();
        }
    }

    Ok(())
}

/// Converts the player's pending path into waypoints for the pathing system.
///
/// When server-side pathfinding is used (`client_pathfinder` is `false`), this
/// runs the rsmod `find_path` algorithm from the player's current position to
/// the destination coordinate, generating up to 25 waypoints with normal
/// collision. When client-side pathfinding is used, the packed path coordinates
/// from the client are directly queued as waypoints (up to 25).
///
/// # Arguments
///
/// * `active` - The active player whose path is being processed.
/// * `client_pathfinder` - Whether the engine is configured to trust client-side paths.
///
/// # Side Effects
///
/// * Consumes the player's `path` field (takes ownership via `take()`).
/// * Queues waypoints into the player's pathing system.
///
/// # Call Stack
///
/// **Called by:** [`handle`]
/// **Calls:** `rsmod::find_path` (server-side), `player.pathing.queue_waypoints`
fn path_to_move_click(active: &mut ActivePlayer, client_pathfinder: bool) {
    let Some(path) = active.player.path.take() else {
        return;
    };
    if path.is_empty() {
        return;
    }
    if !client_pathfinder {
        let coord = path[0];
        active.player.pathing.queue_waypoints(rsmod::find_path(
            coord.y(),
            active.player.pathing.coord.x(),
            active.player.pathing.coord.z(),
            coord.x(),
            coord.z(),
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
    } else {
        let mut packed = [0; 25];
        let len = path.len().min(25);
        for (i, coord) in path.iter().enumerate().take(len) {
            packed[i] = coord.packed();
        }
        active.player.pathing.queue_waypoints(&packed[..len]);
    }
}
