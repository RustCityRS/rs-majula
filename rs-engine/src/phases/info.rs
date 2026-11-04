use crate::engine::Engine;
use crate::info::{NpcSnapshot, PlayerSnapshot};
use rs_entity::InteractionTarget;
use rs_grid::CoordGrid;
use rs_info::Visibility;
use rs_protocol::network::game::info_prot::PlayerInfoProt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use tracing::error;

impl Engine {
    /// Processes the info phase of the engine tick cycle.
    ///
    /// Computes and stores per-entity info update masks for both players
    /// and NPCs, which the output phase will encode into client packets.
    ///
    /// Returns early if no players are online.
    ///
    /// # Side Effects
    ///
    /// * Rebuilds player appearance data when the appearance mask is set.
    /// * Writes computed info into `self.player_renderer` and
    ///   `self.npc_renderer`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `process_player_info`, `process_npc_info`
    pub(crate) fn infos(&mut self) {
        if self.player_list.count() == 0 {
            return;
        }
        self.player_snapshots.fill(PlayerSnapshot::ABSENT);
        self.npc_snapshots.fill(NpcSnapshot::ABSENT);
        self.process_player_info();
        self.process_npc_info();
    }

    #[inline(always)]
    fn process_player_info(&mut self) {
        let pids = self.player_list.take_pids();
        let mut start = 0;
        loop {
            let result = catch_unwind(AssertUnwindSafe(|| {
                for &pid in &pids[start..] {
                    Self::compute_player_info(self, pid);
                    start += 1;
                }
            }));
            match result {
                Ok(()) => break,
                Err(panic) => {
                    let pid = pids[start];
                    let msg = crate::phases::shared::panic_message(&panic);
                    error!("panic during player info for pid {pid}: {msg}");
                    self.emergency_remove_player(pid);
                    start += 1;
                }
            }
        }
        self.player_list.put_pids(pids);
    }

    #[inline(always)]
    fn compute_player_info(&mut self, pid: u16) {
        let target = match self.player_list.players[pid as usize].as_ref() {
            Some(active) => active.player.interaction.target,
            None => return,
        };
        let pathing_face = self.resolve_pathing_face(target);

        let Some(active) = self.player_list.players[pid as usize].as_mut() else {
            return;
        };

        active.player.reorient(pathing_face);
        #[cfg(since_647)]
        active.player.sync_move_types();
        active.rebuild_normal(false);
        if active.player.info.masks & PlayerInfoProt::Appearance as u16 != 0 {
            active.generateappearance(self.clock);
        }
        #[cfg(before_647)]
        self.player_renderer
            .compute_info(active.player.uid.pid() as usize, &active.player.info);
        // Rev-647: the FaceCoord block carries a precomputed angle, derived
        // from the player's current tile at encode time.
        #[cfg(since_647)]
        self.player_renderer.compute_info(
            active.player.uid.pid() as usize,
            &active.player.info,
            active.player.pathing.coord,
        );

        // Snapshot exactly the fields `PlayerInfo::write_players` branches on.
        // Movement/visibility are finalized before the info phase and unchanged
        // through the output phase, so these copies are value-identical.
        let len = self.player_renderer.highdefinitions(pid) as u16;
        let p = &active.player;
        let mut flags = PlayerSnapshot::PRESENT;
        if p.active {
            flags |= PlayerSnapshot::ACTIVE;
        }
        if p.pathing.tele {
            flags |= PlayerSnapshot::TELE;
        }
        if p.info.vis == Visibility::Hard {
            flags |= PlayerSnapshot::VIS_HARD;
        }
        // Record whether an ExactMove block is present so write_players can
        // skip the cold ActivePlayer deref unless the tail must be appended.
        // Read from the same final masks write_players' highdefinition tests.
        if p.info.masks & PlayerInfoProt::ExactMove as u16 != 0 {
            flags |= PlayerSnapshot::HAS_EXACTMOVE;
        }
        self.player_snapshots[pid as usize] = PlayerSnapshot {
            coord: p.pathing.coord.packed(),
            len,
            run_dir: p.pathing.run_dir,
            walk_dir: p.pathing.walk_dir,
            flags,
        };
    }

    #[inline(always)]
    fn process_npc_info(&mut self) {
        let nids = self.npc_list.take_nids();
        let mut start = 0;
        loop {
            let result = catch_unwind(AssertUnwindSafe(|| {
                for &nid in &nids[start..] {
                    Self::compute_npc_info(self, nid);
                    start += 1;
                }
            }));
            match result {
                Ok(()) => break,
                Err(panic) => {
                    let nid = nids[start];
                    let msg = crate::phases::shared::panic_message(&panic);
                    error!("panic during npc info for nid {nid}: {msg}");
                    self.emergency_deactivate_npc(nid);
                    start += 1;
                }
            }
        }
        self.npc_list.put_nids(nids);
    }

    #[inline(always)]
    fn compute_npc_info(&mut self, nid: u16) {
        let target = match self.npc_list.npcs[nid as usize].as_ref() {
            Some(active) => active.npc.interaction.target,
            None => return,
        };
        let pathing_face = self.resolve_pathing_face(target);

        let Some(active) = self.npc_list.npcs[nid as usize].as_mut() else {
            return;
        };

        active.npc.reorient(pathing_face);
        self.npc_renderer
            .compute_info(active.npc.uid.nid() as usize, &active.npc.info);

        // Snapshot exactly the fields `NpcInfo::write_npcs` branches on.
        let len = self.npc_renderer.highdefinitions(nid) as u16;
        let n = &active.npc;
        let mut flags = NpcSnapshot::PRESENT;
        if n.active {
            flags |= NpcSnapshot::ACTIVE;
        }
        if n.pathing.tele {
            flags |= NpcSnapshot::TELE;
        }
        self.npc_snapshots[nid as usize] = NpcSnapshot {
            coord: n.pathing.coord.packed(),
            len,
            run_dir: n.pathing.run_dir,
            walk_dir: n.pathing.walk_dir,
            flags,
        };
    }

    /// Resolves the live fine (sub-tile) coordinate of a pathing-entity interaction
    /// target, or `None` for non-pathing targets and despawned entities.
    ///
    /// `InteractionTarget::{Npc, Player}` store only an index, so their facing coordinate
    /// can't be derived from the target alone -- it must be read from the entity's current
    /// pathing position here, where both registries are in scope. Consumed by `reorient`
    /// to re-face a moving target each tick.
    fn resolve_pathing_face(&self, target: Option<InteractionTarget>) -> Option<(u16, u16)> {
        match target? {
            InteractionTarget::Player { pid } => {
                let p = &self.player_list.players.get(pid as usize)?.as_ref()?.player;
                Some((
                    CoordGrid::fine(p.pathing.coord.x(), p.pathing.size),
                    CoordGrid::fine(p.pathing.coord.z(), p.pathing.size),
                ))
            }
            InteractionTarget::Npc { nid } => {
                let n = &self.npc_list.npcs.get(nid as usize)?.as_ref()?.npc;
                Some((
                    CoordGrid::fine(n.pathing.coord.x(), n.pathing.size),
                    CoordGrid::fine(n.pathing.coord.z(), n.pathing.size),
                ))
            }
            _ => None,
        }
    }
}
