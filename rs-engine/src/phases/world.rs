use crate::engine::{Engine, engine_mut};
use rs_entity::{EntityLifeTime, Obj};
use rs_pack::types::HuntModeType;
use rs_vm::state::ExecutionState;

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
    /// Each entry's delay is decremented every tick. When it reaches zero,
    /// the script is unlinked from the queue and executed. Post-execution,
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
    /// * May modify player or NPC `active_script` state.
    fn process_world_queue(&mut self) {
        let mut h = self.world_queue.head();
        while let Some(idx) = h {
            let entry = self.world_queue.get_mut(idx);
            entry.delay -= 1;
            if entry.delay > 0 {
                h = self.world_queue.next();
                continue;
            }

            h = self.world_queue.next();
            let mut script = self.world_queue.unlink(idx);
            let result = engine_mut().runescript_vm_execute(&mut script);

            match result {
                ExecutionState::WorldSuspended => {
                    let delay = script.pop_int() as u16;
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
    }

    /// Drains the delayed-object queue and spawns objects whose timers have
    /// expired.
    ///
    /// Each entry's delay is decremented every tick. When it reaches zero,
    /// the object is constructed and added to the world via `add_obj` with
    /// the configured receiver and despawn duration.
    ///
    /// # Side Effects
    ///
    /// * Adds new [`Obj`] entities to zones.
    fn process_obj_delayed_queue(&mut self) {
        let mut h = self.obj_delayed_queue.head();
        while let Some(idx) = h {
            let entry = self.obj_delayed_queue.get_mut(idx);
            entry.delay -= 1;
            if entry.delay > 0 {
                h = self.obj_delayed_queue.next();
                continue;
            }

            h = self.obj_delayed_queue.next();
            let request = self.obj_delayed_queue.unlink(idx);
            let coord = request.coord;
            let obj = Obj::new(coord, EntityLifeTime::Despawn, request.id, request.count);
            engine_mut().add_obj(coord, obj, request.receiver37, request.duration);
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
