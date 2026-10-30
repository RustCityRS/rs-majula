use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::Engine;
use std::panic::{AssertUnwindSafe, catch_unwind};
use tracing::error;

impl Engine {
    /// Processes the output phase of the engine tick cycle.
    ///
    /// For each active player, within a panic-catching boundary, encodes
    /// and flushes all outbound data accumulated during the tick:
    ///
    /// 1. Computes coordinate deltas and level-change flags.
    /// 2. Encodes player info (nearby player updates, appearance, movement).
    /// 3. Encodes NPC info (nearby NPC updates, movement, animations).
    /// 4. Sends map rebuild packets if the player changed levels.
    /// 5. Writes player info and NPC info packets.
    /// 6. Sends zone update packets for zones the player observes.
    /// 7. Sends inventory change packets for modified inventories.
    /// 8. Sends stat change packets for modified skills.
    /// 9. Updates AFK zone tracking.
    /// 10. Flushes the player's output buffer to the network channel.
    ///
    /// The player is temporarily `take()`-n from the slot for the duration
    /// of encoding and always restored afterward, even after a panic.
    /// Players that panic are emergency-removed.
    ///
    /// # Side Effects
    ///
    /// * Writes encoded packets to each player's output buffer.
    /// * Flushes output buffers to the network transport.
    /// * May emergency-remove players that cause panics.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `player_info`, `npc_info`, `update_map`, `update_zones`,
    ///   `update_invs`, `update_stats`, `update_afk_zones`, `encode`
    pub(crate) fn outputs(&mut self) {
        let pids = self.player_list.take_pids();
        let mut start = 0;
        // Held out here, so a panic mid-encode can't drop the player: they
        // go back in their slot for the emergency removal to save.
        let mut taken = None;
        loop {
            let result = catch_unwind(AssertUnwindSafe(|| {
                for &pid in &pids[start..] {
                    taken = self.player_list.players[pid as usize].take();
                    if let Some(active) = &mut taken {
                        self.process_output(active);
                    }
                    self.player_list.players[pid as usize] = taken.take();
                    start += 1;
                }
            }));
            match result {
                Ok(()) => break,
                Err(panic) => {
                    let pid = pids[start];
                    self.player_list.players[pid as usize] = taken.take();
                    let msg = crate::phases::shared::panic_message(&panic);
                    error!("panic during output processing for pid {pid}: {msg}");
                    self.emergency_remove_player(pid);
                    start += 1;
                }
            }
        }
        self.player_list.put_pids(pids);
    }

    #[inline(always)]
    fn process_output(&mut self, active: &mut ActivePlayer) {
        let dx = (active.player.pathing.last_coord.x() as i32
            - active.player.pathing.coord.x() as i32)
            .abs();
        let dz = (active.player.pathing.last_coord.z() as i32
            - active.player.pathing.coord.z() as i32)
            .abs();
        let rebuild = active.player.pathing.last_coord.y() != active.player.pathing.coord.y();

        let player_info = self.player_info.encode(
            &mut self.player_renderer,
            &self.player_list.players,
            &self.player_snapshots[..],
            &self.zones,
            active,
            dx,
            dz,
            rebuild,
        );

        let npc_info = self.npc_info.encode(
            &mut self.npc_renderer,
            &mut self.npc_list.npcs,
            &self.npc_snapshots[..],
            &self.zones,
            active,
            dx,
            dz,
            rebuild,
        );

        active.update_map();
        active.player_info(player_info);
        active.npc_info(npc_info);
        active.update_zones(&self.zones, &self.zones_tracking, self.clock);
        active.update_invs(&mut self.invs);
        active.update_other_invs(&self.player_list.players);
        active.update_stats();
        active.player.update_afk_zones();
        active.encode();
        if active.watch.keyframe_due() {
            self.replay_keyframe(active);
        }
    }
}
