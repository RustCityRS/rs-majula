use crate::core::{Engine, engine_mut};
use rs_pack::types::HuntModeType;
use rs_vm::state::{ExecutionState, ScriptState};

impl Engine {
    /// Processes the world phase of the engine tick cycle.
    ///
    /// Executes global world-level logic that is not tied to any single
    /// player or NPC:
    ///
    /// 1. Drains and executes delayed world scripts from the world queue.
    /// 2. Spawns delayed objects whose timers have expired.
    /// 3. Runs NPC-hunts-players logic (player-type hunts that must run
    ///    before the per-NPC phase so that newly observed players are
    ///    visible to NPC AI).
    ///
    /// # Side Effects
    ///
    /// * Executes RuneScript world scripts, which may suspend into player
    ///   or NPC active scripts.
    /// * Adds objects to zones via `add_obj`.
    /// * Sets `hunt_target` on NPCs whose hunt type is `Player`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `process_world_queue`, `process_obj_delayed_queue`,
    ///   `process_npc_hunt_players`
    pub(crate) fn world(&mut self) {
        // - world queue
        self.process_world_queue();
        // - add objs delayed
        self.process_obj_delayed_queue();
        // - npc hunt players
        self.process_npc_hunt_players();
    }

    /// Drains the world script queue and executes any scripts whose delay
    /// has reached zero.
    ///
    /// Each entry's delay counts down once per pass ([`tick_down`]). When it
    /// is due, the script is unlinked from the queue and executed. Post-execution,
    /// the result is dispatched:
    ///
    /// * `WorldSuspended` -- re-enqueues the script with a new delay popped
    ///   from the script stack.
    /// * `Suspended` -- parks the script as the active script on its
    ///   associated player.
    /// * `NpcSuspended` -- parks the script as the active script on its
    ///   associated NPC.
    /// * All other states are discarded (script finished).
    ///
    /// # Side Effects
    ///
    /// * Executes RuneScript VM instructions.
    /// * Releases any protected player access the script took, so a
    ///   `world_delay` resume must `p_finduid` again.
    /// * May modify player or NPC `active_script` state.
    fn process_world_queue(&mut self) {
        let mut h = self.world_queue.head();
        while let Some(idx) = h {
            if tick_down(&mut self.world_queue.get_mut(idx).delay) {
                let script = self.world_queue.unlink(idx);
                self.run_world_script(script);
            }

            h = self.world_queue.next();
        }
    }

    /// Runs a script that belongs to no player or npc, and parks it where
    /// it suspends: back in the world queue, or on the player or npc it
    /// suspended on.
    pub(crate) fn run_world_script(&mut self, mut script: ScriptState) {
        let result = engine_mut().runescript_vm_execute(&mut script);
        self.release_protected_players(&mut script);

        match result {
            ExecutionState::WorldSuspended => {
                let delay = script.pop_int();
                engine_mut().enqueue_world_script(script, delay);
            }
            ExecutionState::Suspended => {
                if let Some(uid) = script.active_player {
                    if let Some(active) = &mut self.player_list.players[uid.pid() as usize] {
                        active.player.state.active_script = Some(Box::new(script));
                    }
                }
            }
            ExecutionState::NpcSuspended => {
                if let Some(uid) = script.active_npc {
                    if let Some(active) = &mut self.npc_list.npcs[uid.nid() as usize] {
                        active.npc.state.active_script = Some(Box::new(script));
                    }
                }
            }
            _ => {}
        }
    }

    /// Drains the delayed-object queue and spawns objects whose timers have
    /// expired.
    ///
    /// Each entry's delay counts down once per pass ([`tick_down`]). When it
    /// is due, the object is constructed and added to the world via `add_obj`
    /// with the configured receiver and despawn duration.
    ///
    /// # Side Effects
    ///
    /// * Adds new [`Obj`] entities to zones.
    fn process_obj_delayed_queue(&mut self) {
        let mut h = self.obj_delayed_queue.head();
        while let Some(idx) = h {
            if !tick_down(&mut self.obj_delayed_queue.get_mut(idx).delay) {
                h = self.obj_delayed_queue.next();
                continue;
            }

            h = self.obj_delayed_queue.next();
            let request = self.obj_delayed_queue.unlink(idx);
            engine_mut().add_item_obj(
                request.coord,
                request.item,
                request.receiver,
                request.duration,
            );
        }
    }

    /// Runs player-type hunt checks for all active NPCs.
    ///
    /// Only processes NPCs that are active, have a hunt mode configured,
    /// and have at least one observer. Player hunts are handled here in the
    /// world phase (rather than in the per-NPC phase) so that every NPC
    /// sees a consistent snapshot of player positions before individual NPC
    /// processing begins.
    ///
    /// Returns early if no players are online.
    ///
    /// # Side Effects
    ///
    /// * Sets `hunt_target` on NPCs that successfully find a player target.
    fn process_npc_hunt_players(&mut self) {
        if self.player_list.count() == 0 {
            return;
        }
        for &nid in self.npc_list.processing.iter() {
            let Some(active) = self.npc_list.npcs[nid as usize].as_mut() else {
                continue;
            };
            if !active.npc.active {
                continue;
            }
            let Some(hunt_id) = active.npc.hunt_mode else {
                continue;
            };
            if active.npc.observers == 0 {
                continue;
            }
            let Some(hunt) = self.cache.hunts.get_by_id(hunt_id) else {
                continue;
            };
            if hunt.hunt_type != HuntModeType::Player {
                continue;
            }
            Self::npc_hunt_all(&mut **active, hunt);
        }
    }
}

/// Counts a queued delay down for one world pass, like the reference engine's
/// `const delay = request.delay--; if (delay > 0) continue;`. Returns `true`
/// when the entry is due: its delay was 0 or less before this pass.
#[inline(always)]
fn tick_down(delay: &mut i32) -> bool {
    let before = *delay;
    *delay = before.saturating_sub(1);
    before <= 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_datastruct::LinkList;

    /// A queued entry, and the `world_delay` it calls when it runs.
    struct Entry {
        name: &'static str,
        delay: i32,
        then: Option<i32>,
    }

    /// Runs `passes` world passes over `queue`, the way `process_world_queue`
    /// walks it, and returns `(pass, name)` for each entry that ran.
    fn run(queue: &mut LinkList<Entry>, passes: u32) -> Vec<(u32, &'static str)> {
        let mut ran = Vec::new();
        for pass in 1..=passes {
            let mut h = queue.head();
            while let Some(idx) = h {
                if tick_down(&mut queue.get_mut(idx).delay) {
                    let entry = queue.unlink(idx);
                    ran.push((pass, entry.name));
                    if let Some(delay) = entry.then {
                        // `enqueue_world_script`.
                        queue.add_tail(Entry {
                            name: entry.name,
                            delay: delay.saturating_add(1),
                            then: None,
                        });
                    }
                }
                h = queue.next();
            }
        }
        ran
    }

    /// A script runs in the player phase of tick T, after that tick's world
    /// pass, so world pass N is tick T+N.
    fn first_pass(stored: i32) -> u32 {
        let mut queue = LinkList::new();
        queue.add_tail(Entry {
            name: "a",
            delay: stored,
            then: None,
        });
        run(&mut queue, 100)[0].0
    }

    /// `inv_dropitem_delayed` stores the delay as given (InvOps.ts:208), so
    /// delay N lands on tick T+N+1 and delay 0 or less on T+1. Before, 0
    /// underflowed a u64 and the item never landed.
    #[test]
    fn delayed_obj_lands_like_upstream() {
        for (delay, pass) in [(-3, 1), (-1, 1), (0, 1), (1, 2), (5, 6)] {
            assert_eq!(
                first_pass(delay),
                pass,
                "inv_dropitem_delayed delay {delay}"
            );
        }
    }

    /// `world_delay(d)` stores d + 1 (World.ts:1250), so it resumes on tick
    /// T+d+2, and on T+1 for d below 0.
    #[test]
    fn world_delay_resumes_like_upstream() {
        for (delay, pass) in [(-2, 1), (-1, 1), (0, 2), (1, 3), (4, 6)] {
            assert_eq!(first_pass(delay + 1), pass, "world_delay({delay})");
        }
    }

    /// A world-queue script that calls `world_delay(0)` again is reached the
    /// same pass when entries follow it, so it resumes a pass sooner than
    /// when it is the last entry. Upstream's `LinkList.all()` does the same.
    #[test]
    fn requeued_world_delay_depends_on_what_follows() {
        let mut alone = LinkList::new();
        alone.add_tail(Entry {
            name: "a",
            delay: 1,
            then: Some(0),
        });
        assert_eq!(run(&mut alone, 4), [(2, "a"), (4, "a")]);

        let mut followed = LinkList::new();
        followed.add_tail(Entry {
            name: "a",
            delay: 1,
            then: Some(0),
        });
        followed.add_tail(Entry {
            name: "b",
            delay: 9,
            then: None,
        });
        assert_eq!(run(&mut followed, 4), [(2, "a"), (3, "a")]);
    }
}
