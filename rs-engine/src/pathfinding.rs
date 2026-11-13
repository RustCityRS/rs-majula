use crate::active_player::ActivePlayer;
use crate::core::{Engine, engine, engine_mut};
use rs_entity::PlayerUid;
use rs_grid::CoordGrid;
use rsmod::{PathJob, PathPool, PathRequest};
use std::io;

#[derive(Clone, Copy, Default, Debug)]
pub struct PathStats {
    pub requests: usize,
    pub searches: usize,
    pub batches: usize,
}

#[derive(Default)]
pub(crate) struct InputPaths {
    pub current: Option<PlayerUid>,
    pub stats: PathStats,
    pool: Option<PathPool>,
    jobs: Vec<PathJob>,
    players: Vec<(PlayerUid, u32)>,
}

pub(crate) struct PendingPath {
    request: PathRequest,
    revision: u32,
}

impl ActivePlayer {
    pub(crate) fn request_path(&mut self, request: PathRequest) {
        if engine().input_paths.current.map(|uid| uid.packed()) != Some(self.player.uid.packed()) {
            self.pending_path = None;
            self.player.pathing.queue_waypoints(request.find_path());
            return;
        }
        engine_mut().input_paths.stats.requests += 1;
        self.player.pathing.queue_waypoint(CoordGrid::new(
            request.dest_x,
            request.y,
            request.dest_z,
        ));
        self.pending_path = Some(PendingPath {
            request,
            revision: self.player.pathing.path_revision,
        });
    }

    pub(crate) fn finish_input_path(&mut self) {
        let Some(pending) = self.pending_path.take() else {
            return;
        };
        let pathing = &mut self.player.pathing;
        let request = pending.request;
        if pathing.path_revision != pending.revision {
            return;
        }
        if pathing.coord != CoordGrid::new(request.src_x, request.y, request.src_z) {
            pathing.clear_waypoints();
            return;
        }
        let paths = &mut engine_mut().input_paths;
        if paths.pool.is_none() {
            paths.stats.searches += 1;
            pathing.queue_waypoints(request.find_path());
        } else {
            paths.jobs.push(PathJob::new(request));
            paths.players.push((self.player.uid, pending.revision));
        }
    }

    pub(crate) fn cancel_input_path(&mut self) {
        if let Some(pending) = self.pending_path.take()
            && self.player.pathing.path_revision == pending.revision
        {
            self.player.pathing.clear_waypoints();
        }
    }
}

impl Engine {
    /// Configures search workers at startup, before raising the tick priority.
    /// One thread runs searches directly on the engine thread.
    pub fn set_pathfinder_threads(&mut self, threads: usize) -> io::Result<()> {
        self.flush_input_paths();
        self.input_paths.pool = if threads == 1 {
            None
        } else {
            Some(PathPool::new(threads)?)
        };
        Ok(())
    }

    pub fn pathfinding_stats(&self) -> PathStats {
        self.input_paths.stats
    }

    /// Finishes earlier players' searches before a script or other input can
    /// observe their routes or change collision. The current player's request
    /// stays replaceable until all of their input has been handled.
    pub(crate) fn flush_input_paths(&mut self) {
        let paths = &mut self.input_paths;
        if paths.jobs.is_empty() {
            return;
        }
        unsafe { paths.pool.as_ref().unwrap().find_paths(&mut paths.jobs) };
        paths.stats.searches += paths.jobs.len();
        paths.stats.batches += 1;
        for (job, (uid, revision)) in paths.jobs.drain(..).zip(paths.players.drain(..)) {
            if let Some(active) = self.player_list.players[uid.pid() as usize].as_mut()
                && active.player.uid.packed() == uid.packed()
                && active.player.pathing.path_revision == revision
            {
                active.player.pathing.queue_waypoints(job.waypoints());
            }
        }
    }
}
