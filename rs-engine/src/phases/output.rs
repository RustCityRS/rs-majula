#[cfg(since_647)]
use crate::active_player::ActivePlayer;
use crate::active_player::EnginePlayer;
use crate::core::Engine;
use crate::phases::shared::panic_message;
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
    /// Each player is encoded in their slot. Only a replay keyframe takes
    /// them out of it, and they always go back, even after a panic. Players
    /// that panic are emergency-removed.
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
        #[cfg(since_647)]
        self.broadcast_public_chats();
        let pids = self.player_list.take_pids();
        let mut start = 0;
        // Held out here, so a panic mid-keyframe can't drop the player: they
        // go back in their slot for the emergency removal to save.
        let mut taken = None;
        loop {
            let result = catch_unwind(AssertUnwindSafe(|| {
                for &pid in &pids[start..] {
                    if self.process_output(pid) {
                        taken = self.player_list.players[pid as usize].take();
                        #[cfg(before_647)]
                        if let Some(active) = &mut taken {
                            self.replay_keyframe(active);
                        }
                        self.player_list.players[pid as usize] = taken.take();
                    }
                    start += 1;
                }
            }));
            match result {
                Ok(()) => break,
                Err(panic) => {
                    let pid = pids[start];
                    if let Some(active) = taken.take() {
                        self.player_list.players[pid as usize] = Some(active);
                    }
                    let msg = panic_message(&panic);
                    error!("panic during output processing for pid {pid}: {msg}");
                    self.emergency_remove_player(pid);
                    start += 1;
                }
            }
        }
        self.player_list.put_pids(pids);
    }

    /// Rev-647 public chat: deliver each pending message as MESSAGE_PUBLIC
    /// packets to every player in the speaker's tracked (in-view) set — the
    /// GPI system keeps `build_area.players` current each tick.
    #[cfg(since_647)]
    fn broadcast_public_chats(&mut self) {
        use rs_grid::CoordGrid;

        let pids = self.player_list.take_pids();
        let mut chats: Vec<(u16, CoordGrid, u16, u8, usize, Vec<u8>)> = Vec::new();
        for &pid in &pids {
            if let Some(active) = self.player_list.players[pid as usize].as_mut()
                && let Some(chat) = active.player.pending_public_chat.take()
            {
                chats.push((
                    pid,
                    active.player.pathing.coord,
                    chat.0,
                    chat.1,
                    chat.2,
                    chat.3,
                ));
            }
        }

        let mut receivers: Vec<u16> = Vec::new();
        for (sender_pid, sender_coord, effects, rights, char_count, huffman_data) in chats {
            let Some(packet) =
                ActivePlayer::public_chat(sender_pid, effects, rights, char_count, &huffman_data)
            else {
                continue;
            };
            receivers.clear();
            match self.player_list.players[sender_pid as usize].as_ref() {
                Some(sender) => receivers.extend(sender.player.build_area.players.iter()),
                None => continue,
            }
            for &recv_pid in &receivers {
                if let Some(receiver) = self.player_list.players[recv_pid as usize].as_mut() {
                    let in_range = CoordGrid::in_distance(
                        &receiver.player.pathing.coord,
                        sender_coord,
                        receiver.player.build_area.view_distance,
                    );
                    if in_range {
                        receiver.message_public(&packet.data);
                    }
                }
            }
        }
        self.player_list.put_pids(pids);
    }
    /// Encodes player `pid`'s output in their slot. Whether a replay
    /// keyframe is due.
    #[inline(always)]
    fn process_output(&mut self, pid: u16) -> bool {
        let Some((active, others)) = self.player_list.split(pid) else {
            return false;
        };
        let dx = (active.player.pathing.last_coord.x() as i32
            - active.player.pathing.coord.x() as i32)
            .abs();
        let dz = (active.player.pathing.last_coord.z() as i32
            - active.player.pathing.coord.z() as i32)
            .abs();
        let rebuild = active.player.pathing.last_coord.y() != active.player.pathing.coord.y();

        let player_info = self.player_info.encode(
            &mut self.player_renderer,
            &self.player_snapshots,
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
        active.player_info_parts(player_info.into());
        active.npc_info_parts(npc_info.into());
        active.update_zones(&self.zones, &self.zones_tracking, self.clock);
        active.update_invs(&mut self.invs);
        active.update_other_invs(others);
        active.update_stats();
        active.player.update_afk_zones();
        active.encode();
        active.watch.keyframe_due()
    }
}
