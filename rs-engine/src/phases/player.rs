use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::Engine;
use crate::engine::{cache, engine, engine_mut};
use crate::phases::shared::EntityId;
use rs_entity::MoveSpeed;
use rs_entity::interaction::InteractionTarget;
use rs_grid::CoordGrid;
use rs_info::FocusKind;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};
use rs_vm::state::{ExecutionState, QueuePriority, ScriptArgument, TimerPriority};
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::panic::{AssertUnwindSafe, catch_unwind};
use tracing::error;

impl Engine {
    /// Processes the player phase of the engine tick cycle.
    ///
    /// For each active player, within a panic-catching boundary, executes
    /// the following sub-phases in order:
    ///
    /// 1. Checks and clears the delay timer.
    /// 2. Resumes any suspended script from a previous tick.
    /// 3. Processes primary and weak queues ([`process_queues`]).
    /// 4. Fires normal and soft timers ([`process_timers`]).
    /// 5. Processes engine-internal queue entries ([`process_engine_queue`]).
    /// 6. Updates face-entity orientation.
    /// 7. Handles interactions and movement: if the player has an
    ///    interaction target, runs [`process_interaction`]; otherwise runs
    ///    [`process_movement`].
    /// 8. Recovers or depletes run energy ([`update_energy`]).
    /// 9. Updates zone membership and collision maps.
    ///
    /// Players that panic during processing are emergency-removed.
    ///
    /// # Side Effects
    ///
    /// * Executes RuneScript queue/timer/interaction scripts.
    /// * Mutates player coordinate, path, queue, and timer state.
    /// * Updates zone entity lists and collision flags.
    /// * May emergency-remove players that cause panics.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `process_queues`, `process_timers`, `process_engine_queue`,
    ///   `process_interaction`, `process_movement`, `update_energy`,
    ///   `check_zones_and_collision`
    pub(crate) fn players(&mut self) {
        let pids = self.player_list.take_pids();
        let mut start = 0;
        loop {
            let result = catch_unwind(AssertUnwindSafe(|| {
                for &pid in &pids[start..] {
                    Self::process_player(self, pid);
                }
            }));
            match result {
                Ok(()) => break,
                Err(panic) => {
                    let pid = pids[start];
                    let msg = crate::phases::shared::panic_message(&panic);
                    error!("panic during player processing for pid {pid}: {msg}");
                    self.emergency_remove_player(pid);
                    start += 1;
                }
            }
        }
        self.player_list.put_pids(pids);
    }

    #[inline(always)]
    fn process_player(&mut self, pid: u16) {
        let Some(active) = self.player_list.players[pid as usize].as_mut() else {
            return;
        };

        let prev_coord = active.player.pathing.coord;

        active.player.state.check_delay(self.clock);

        // - resume suspended script
        if !active.player.state.delayed
            && active
                .player
                .state
                .active_script
                .as_ref()
                .is_some_and(|s| s.execution == ExecutionState::Suspended)
        {
            let state = *active.player.state.active_script.take().unwrap();
            if let Err(e) = engine_mut().run_script_by_state(
                state,
                Some(ScriptSubject::Player(active.player.uid)),
                Some(true),
                Some(true),
            ) {
                error!(
                    "error resuming suspended script for player {}: {e}",
                    active.player.uid.pid()
                );
            }
        }

        // - primary queue
        // - weak queue
        Self::process_queues(active);
        // - timers
        // - soft timers
        Self::process_timers(self.clock, active, TimerPriority::Normal);
        Self::process_timers(self.clock, active, TimerPriority::Soft);
        // - engine queue
        Self::process_engine_queue(active);
        active.player.set_face_entity();
        active.player.pathing.follow_coord = active.player.pathing.last_step_coord;
        // - interactions
        // - movement
        if active.player.interaction.target.is_some() {
            Self::process_interaction(active as *mut _);
        } else {
            Self::process_movement(active);
        }

        if prev_coord != active.player.pathing.coord {
            active.player.pathing.last_movement = self.clock + 1;
        }

        // - run energy
        if active.player.update_energy() {
            // energy hit zero: push the now-disabled run state to the client orb
            active.sync_run();
        }

        Engine::check_zones_and_collision(
            &mut self.zones,
            prev_coord,
            active.player.pathing.coord,
            EntityId::Player(active.player.uid.pid()),
            active.player.pathing.size,
            active.player.block_walk,
        );
    }

    // ----

    /// Fires all player timers of the given priority whose interval has
    /// elapsed.
    ///
    /// For each timer entry, if the current clock minus the timer's last
    /// fire clock meets or exceeds the configured interval, the associated
    /// RuneScript is executed. Normal-priority timers require the player to
    /// be accessible (not in a modal); soft timers fire unconditionally.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias on the parameter -- script execution
    /// through `engine_mut()` aliases the same player state, and noalias
    /// lets LLVM cache field values across those calls in release builds.
    ///
    /// # Side Effects
    ///
    /// * Updates `timer.clock` to the current tick.
    /// * Executes RuneScript timer scripts.
    #[inline(always)]
    fn process_timers(clock: u32, active: *mut ActivePlayer, priority: TimerPriority) {
        let active = unsafe { &mut *active };
        if active.player.logout_sent {
            return;
        }

        let uid = active.player.uid;
        let can_access = active.can_access();
        let accessible = priority == TimerPriority::Soft || can_access;

        let timers = match priority {
            TimerPriority::Normal => &mut active.player.state.timers.normal,
            TimerPriority::Soft => &mut active.player.state.timers.soft,
        };

        for timer in timers.values_mut() {
            if clock < timer.clock + timer.interval as u32 || !accessible {
                continue;
            }

            timer.clock = clock;

            if let Some(script) = engine().get_script(timer.script_id).cloned() {
                let state = engine_mut().build_state(
                    script,
                    Some(ScriptSubject::Player(uid)),
                    None,
                    timer.args.clone(),
                );
                if let Err(e) = engine_mut().run_script_by_state(
                    state,
                    Some(ScriptSubject::Player(uid)),
                    Some(priority == TimerPriority::Normal),
                    None,
                ) {
                    error!("error running timer script for player {}: {e}", uid.pid());
                }
            }
        }
    }

    /// Processes both primary and weak script queues for a player.
    ///
    /// Before draining queues, scans the primary queue for any `Strong`
    /// priority entry. If one exists, sets `request_modal_close` so that
    /// the modal interface is closed before scripts run. Then delegates to
    /// [`process_queue`] and [`process_weak_queue`].
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    ///
    /// # Side Effects
    ///
    /// * May close the player's modal interface.
    /// * Drains and executes primary and weak queue scripts.
    #[inline(always)]
    fn process_queues(active: *mut ActivePlayer) {
        let active = unsafe { &mut *active };
        let mut h = active.player.state.queues.queue.head();
        while let Some(idx) = h {
            if active.player.state.queues.queue[idx].priority == QueuePriority::Strong {
                active.player.request_modal_close = true;
                break;
            }
            h = active.player.state.queues.queue.next();
        }

        if active.player.request_modal_close {
            active.player.request_modal_close = false;
            if let Err(e) = active.close_modal(true) {
                error!(
                    "error closing modal for player {}: {e}",
                    active.player.uid.pid()
                );
            }
        }

        Self::process_queue(active);
        Self::process_weak_queue(active);
    }

    /// Drains the player's primary script queue.
    ///
    /// Iterates each queue entry, decrementing its delay. When a delay
    /// reaches zero and the player is accessible, the entry is unlinked
    /// and its RuneScript is executed. `Long`-priority queue entries that
    /// have a leading integer argument of `0` are force-expired during
    /// logout. The first argument is stripped from `Long` entries before
    /// execution.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    ///
    /// # Side Effects
    ///
    /// * Executes RuneScript queue scripts.
    /// * Modifies queue entry delays.
    #[inline(always)]
    fn process_queue(active: *mut ActivePlayer) {
        let active = unsafe { &mut *active };
        let uid = active.player.uid;
        let mut h = active.player.state.queues.queue.head();
        while let Some(idx) = h {
            if active.player.logout_sent
                && active.player.state.queues.queue[idx].priority == QueuePriority::Long
            {
                if let Some(ScriptArgument::Int(0)) = active.player.state.queues.queue[idx]
                    .args
                    .as_ref()
                    .and_then(|a| a.first())
                {
                    active.player.state.queues.queue[idx].delay = 0;
                }
            }

            let delay = active.player.state.queues.queue[idx].delay;
            active.player.state.queues.queue[idx].delay = delay.saturating_sub(1);

            if active.can_access() && delay == 0 {
                let mut request = active.player.state.queues.queue.unlink(idx);

                if request.priority == QueuePriority::Long {
                    if let Some(args) = &mut request.args {
                        args.remove(0);
                    }
                }

                if let Some(script) = engine().get_script(request.script_id).cloned() {
                    let state = engine_mut().build_state(
                        script,
                        Some(ScriptSubject::Player(uid)),
                        None,
                        request.args,
                    );
                    if let Err(e) = engine_mut().run_script_by_state(
                        state,
                        Some(ScriptSubject::Player(uid)),
                        Some(true),
                        None,
                    ) {
                        error!("error running queue script for player {}: {e}", uid.pid());
                    }
                }
            }

            h = active.player.state.queues.queue.next();
        }
    }

    /// Drains the player's weak script queue.
    ///
    /// Behaves like [`process_queue`] but operates on the weak queue, which
    /// holds lower-priority scripts that do not interrupt strong actions.
    /// Each entry's delay is decremented, and entries whose delay reaches
    /// zero are executed when the player is accessible.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    ///
    /// # Side Effects
    ///
    /// * Executes RuneScript weak-queue scripts.
    #[inline(always)]
    fn process_weak_queue(active: *mut ActivePlayer) {
        let active = unsafe { &mut *active };
        let uid = active.player.uid;
        let mut h = active.player.state.queues.weak.head();
        while let Some(idx) = h {
            let delay = active.player.state.queues.weak[idx].delay;
            active.player.state.queues.weak[idx].delay = delay.saturating_sub(1);

            if active.can_access() && delay == 0 {
                let request = active.player.state.queues.weak.unlink(idx);

                if let Some(script) = engine().get_script(request.script_id).cloned() {
                    let state = engine_mut().build_state(
                        script,
                        Some(ScriptSubject::Player(uid)),
                        None,
                        request.args,
                    );
                    if let Err(e) = engine_mut().run_script_by_state(
                        state,
                        Some(ScriptSubject::Player(uid)),
                        Some(true),
                        None,
                    ) {
                        error!(
                            "error running weak queue script for player {}: {e}",
                            uid.pid()
                        );
                    }
                }
            }

            h = active.player.state.queues.weak.next();
        }
    }

    /// Drains the player's engine-internal script queue.
    ///
    /// Engine queue entries are system-generated (not player-initiated) and
    /// follow the same delay-decrement-then-execute pattern as the primary
    /// queue. They are used for engine-driven actions such as interface
    /// callbacks.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    ///
    /// # Side Effects
    ///
    /// * Executes RuneScript engine-queue scripts.
    #[inline(always)]
    fn process_engine_queue(active: *mut ActivePlayer) {
        let active = unsafe { &mut *active };
        let uid = active.player.uid;
        let mut h = active.player.state.queues.engine.head();
        while let Some(idx) = h {
            let delay = active.player.state.queues.engine[idx].delay;
            active.player.state.queues.engine[idx].delay = delay.saturating_sub(1);

            if active.can_access() && delay == 0 {
                let request = active.player.state.queues.engine.unlink(idx);

                if let Some(script) = engine().get_script(request.script_id).cloned() {
                    let state = engine_mut().build_state(
                        script,
                        Some(ScriptSubject::Player(uid)),
                        None,
                        request.args,
                    );
                    if let Err(e) = engine_mut().run_script_by_state(
                        state,
                        Some(ScriptSubject::Player(uid)),
                        Some(true),
                        None,
                    ) {
                        error!(
                            "error running engine queue script for player {}: {e}",
                            uid.pid()
                        );
                    }
                }
            }

            h = active.player.state.queues.engine.next();
        }
    }

    /// Processes a player's current interaction target.
    ///
    /// Orchestrates the core interact-or-move decision loop:
    ///
    /// 1. Validates the interaction target is still reachable.
    /// 2. Fires `walktrigger` unless the player is following.
    /// 3. Attempts to interact with the target ([`try_interact`]).
    /// 4. If interaction is not possible yet, paths toward the target,
    ///    moves the player, and retries after movement.
    /// 5. Shows "I can't reach that!" if the player is out of waypoints
    ///    with zero steps taken and no interaction occurred.
    /// 6. Propagates `next_target` set by scripts (e.g. `p_oploc`) for the
    ///    next cycle, or clears the interaction if completed.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    ///
    /// # Side Effects
    ///
    /// * Executes RuneScript op/ap trigger scripts.
    /// * Moves the player along waypoints.
    /// * May clear or replace the interaction target.
    #[inline(always)]
    fn process_interaction(active: *mut ActivePlayer) {
        let active = unsafe { &mut *active };
        if active.player.interaction.target.is_none() {
            return;
        }

        active.player.next_target = None;

        let target_op = active.player.interaction.target_op;
        let following = target_op == Some(ServerTriggerType::ApPlayer3 as u8)
            || target_op == Some(ServerTriggerType::OpPlayer3 as u8);

        let mut interacted = false;

        if active.player.interaction.target.is_some() && active.can_access() {
            if !Self::validate_target(active) {
                active.player.clear_interaction();
                active.clear_waypoints();
                return;
            }

            if !following {
                active.process_walktrigger();
            }

            interacted = Self::try_interact(active, false);
        }

        if !interacted {
            Self::path_to_pathing_target(active as *mut _);

            let active = unsafe { &mut *(active as *mut ActivePlayer) };

            if active.player.pathing.has_waypoints() && active.can_access() {
                active.process_walktrigger();
            }

            if !active.player.pathing.has_waypoints() && following {
                active.player.clear_interaction();
            }

            Self::process_movement(active);

            if active.player.interaction.target.is_some() && active.can_access() && !following {
                interacted = Self::try_interact(active, active.player.pathing.steps_taken == 0);

                if !interacted
                    && !active.player.interaction.ap_range_called
                    && !active.player.pathing.has_waypoints()
                    && active.player.pathing.steps_taken == 0
                {
                    active.message_game("I can't reach that!");
                    active.player.clear_interaction();
                }
            }
        }

        // If a script called p_oploc/p_opobj/etc., next_target was set for the next cycle
        if active.player.next_target.is_some() {
            active.player.interaction.target = active.player.next_target.take();
        } else if interacted && !active.player.interaction.ap_range_called {
            active.player.clear_interaction();
        }

        if !active.player.pathing.has_waypoints() && active.player.pathing.steps_taken > 0 {
            active.clear_waypoints();
        }
    }

    /// Computes a path toward the player's current interaction target when
    /// that target is a moving entity (player or NPC).
    ///
    /// Handles three cases:
    ///
    /// * **Following:** If the player is on the last (or no) waypoint and
    ///   is following another entity, queues a single waypoint toward the
    ///   target's last known coordinate.
    /// * **Client pathfinder + intersection:** If the client-side
    ///   pathfinder is enabled and the player currently intersects the
    ///   target, uses a naive (straight-line) pathfinder to step away.
    /// * **Otherwise:** Delegates to [`path_to_target`] for a full A*
    ///   pathfind when the player is on the last or no waypoint.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    ///
    /// # Side Effects
    ///
    /// * Queues waypoints on the player's pathing entity.
    #[inline(always)]
    fn path_to_pathing_target(active: *mut ActivePlayer) {
        let active = unsafe { &mut *active };
        let Some(target) = &active.player.interaction.target else {
            return;
        };

        if !target.is_pathing_entity() {
            return;
        }

        let target_op = active.player.interaction.target_op;
        let following = target_op == Some(ServerTriggerType::ApPlayer3 as u8)
            || target_op == Some(ServerTriggerType::OpPlayer3 as u8);

        if active.player.pathing.is_last_or_no_waypoint() && following {
            if let InteractionTarget::Player { pid } = target {
                if let Some(target_player) = engine().get_player(*pid) {
                    active.player.pathing.queue_waypoint(
                        target_player.player.pathing.follow_coord.x(),
                        target_player.player.pathing.follow_coord.z(),
                    );
                }
            }
            return;
        }

        if !active.can_access() {
            return;
        }

        let client_pathfinder = engine().client_pathfinder;
        let x = active.player.pathing.coord.x();
        let z = active.player.pathing.coord.z();

        if client_pathfinder {
            let intersects = match target {
                InteractionTarget::Player { pid } => engine().get_player(*pid).is_some_and(|p| {
                    CoordGrid::intersects(
                        x,
                        z,
                        1,
                        1,
                        p.player.pathing.coord.x(),
                        p.player.pathing.coord.z(),
                        1,
                        1,
                    )
                }),
                InteractionTarget::Npc { nid } => engine().get_npc(*nid).is_some_and(|n| {
                    let s = n.npc.pathing.size as u16;
                    CoordGrid::intersects(
                        x,
                        z,
                        1,
                        1,
                        n.npc.pathing.coord.x(),
                        n.npc.pathing.coord.z(),
                        s,
                        s,
                    )
                }),
                _ => false,
            };

            if intersects {
                let y = active.player.pathing.coord.y();
                match target {
                    InteractionTarget::Player { pid } => {
                        if let Some(p) = engine().get_player(*pid) {
                            active
                                .player
                                .pathing
                                .queue_waypoints(rsmod::find_naive_path(
                                    y,
                                    x,
                                    z,
                                    p.player.pathing.coord.x(),
                                    p.player.pathing.coord.z(),
                                    1,
                                    1,
                                    1,
                                    1,
                                    0,
                                    CollisionType::Normal,
                                ));
                        }
                    }
                    InteractionTarget::Npc { nid } => {
                        if let Some(n) = engine().get_npc(*nid) {
                            let s = n.npc.pathing.size;
                            active
                                .player
                                .pathing
                                .queue_waypoints(rsmod::find_naive_path(
                                    y,
                                    x,
                                    z,
                                    n.npc.pathing.coord.x(),
                                    n.npc.pathing.coord.z(),
                                    1,
                                    1,
                                    s,
                                    s,
                                    0,
                                    CollisionType::Normal,
                                ));
                        }
                    }
                    _ => {}
                }
                return;
            }
        }

        if active.player.pathing.is_last_or_no_waypoint() {
            Self::path_to_target(active, client_pathfinder);
        }
    }

    /// Computes a path from the player's current position to its interaction
    /// target using the shared [`entity_path_to_target`] utility.
    ///
    /// This is the primary entry point for server-side pathfinding toward
    /// an interaction target during the player phase.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    ///
    /// # Side Effects
    ///
    /// * Queues waypoints on the player's pathing entity.
    pub(crate) fn path_to_target(active: *mut ActivePlayer, client_pathfinder: bool) {
        let active = unsafe { &mut *active };
        if let Some(target) = &active.player.interaction.target {
            let src = active.player.pathing.coord.packed();
            let dst = Self::target_coord(target).packed();
            if active.player.pathing.is_last_or_no_waypoint()
                && src == active.player.interaction.last_path_src
                && dst == active.player.interaction.last_path_dst
            {
                return;
            }
            active.player.interaction.last_path_src = src;
            active.player.interaction.last_path_dst = dst;
            Self::entity_path_to_target(&mut active.player.pathing, target, client_pathfinder);
        };
    }

    /// Validates that the player's current interaction target still exists and
    /// is on the same level.
    ///
    /// Delegates to the shared [`entity_validate_target`] with the player's
    /// current Y-level and username hash.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    ///
    /// # Returns
    ///
    /// `true` if the target is still valid and reachable.
    #[inline(always)]
    fn validate_target(active: *mut ActivePlayer) -> bool {
        let active = unsafe { &mut *active };
        let Some(target) = &active.player.interaction.target else {
            return false;
        };
        Self::entity_validate_target(
            active.player.pathing.coord.y(),
            target,
            None,
            Some(active.uid().username37()),
            false,
        )
    }

    /// Resolves the RuneScript trigger for the player's current interaction
    /// at the given trigger-table offset.
    ///
    /// Computes the trigger type from `target_op + offset`, then looks up
    /// the target's type ID and category from the cache. An offset of `0`
    /// produces the approach (AP) trigger; an offset of `7` produces the
    /// operate (OP) trigger. Returns `None` if no script is registered for
    /// the resulting trigger key.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    ///
    /// # Returns
    ///
    /// The resolved `(trigger, type_id, category)` tuple, or `None` if no
    /// matching script exists.
    #[inline(always)]
    fn get_trigger(
        active: *mut ActivePlayer,
        offset: u8,
    ) -> Option<(ServerTriggerType, Option<u16>, Option<i32>)> {
        let active = unsafe { &mut *active };
        let target = active.player.interaction.target.as_ref()?;
        let base = active
            .player
            .interaction
            .target_op
            .as_ref()?
            .wrapping_add(offset);
        let trigger = ServerTriggerType::try_from(base).ok()?;

        let mut type_id: i32 = -1;
        let mut category_id: i32 = -1;

        match target {
            InteractionTarget::Obj { id, .. } => {
                if let Some(obj_type) = cache().objs.get_by_id(*id) {
                    type_id = obj_type.id as i32;
                    category_id = obj_type.category.map(|c| c as i32).unwrap_or(-1);
                }
            }
            InteractionTarget::Loc { id, .. } => {
                if let Some(loc_type) = cache().locs.get_by_id(*id) {
                    type_id = loc_type.id as i32;
                    category_id = loc_type.category.map(|c| c as i32).unwrap_or(-1);
                }
            }
            InteractionTarget::Npc { nid } => {
                if let Some(npc_active) = engine().get_npc(*nid) {
                    let npc_type_id = npc_active.npc.uid.id();
                    if let Some(npc_type) = cache().npcs.get_by_id(npc_type_id) {
                        type_id = npc_type.id as i32;
                        category_id = npc_type.category.map(|c| c as i32).unwrap_or(-1);
                    }
                }
            }
            InteractionTarget::Player { .. } => {}
        }

        if let Some(com) = active.player.interaction.target_subject_com {
            type_id = com as i32;
        }

        let t = if type_id >= 0 {
            Some(type_id as u16)
        } else {
            None
        };
        let c = if category_id >= 0 {
            Some(category_id)
        } else {
            None
        };

        if engine().script_by_key(trigger, t, c).is_some() {
            Some((trigger, t, c))
        } else {
            None
        }
    }

    /// Returns `true` if the player is within operable (melee / adjacent)
    /// distance of its current interaction target.
    ///
    /// Delegates to the shared [`entity_in_operable_distance`] helper which
    /// checks collision-map reachability.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    #[inline(always)]
    fn in_operable_distance(active: *mut ActivePlayer) -> bool {
        let active = unsafe { &mut *active };
        let Some(target) = &active.player.interaction.target else {
            return false;
        };
        Self::entity_in_operable_distance(&active.player.pathing, target)
    }

    /// Returns `true` if the player is within approach (AP) distance of its
    /// current interaction target.
    ///
    /// Uses the player's configured `ap_range` and checks both Chebyshev
    /// distance and line-of-sight via the shared [`entity_in_approach_distance`]
    /// helper. An `ap_range` of `None` means approach has been disabled (set by
    /// the default-ap branch of [`try_interact`]), so this returns `false`.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    #[inline(always)]
    fn in_approach_distance(active: *mut ActivePlayer) -> bool {
        let active = unsafe { &mut *active };
        let Some(target) = &active.player.interaction.target else {
            return false;
        };
        let Some(range) = active.player.interaction.ap_range else {
            return false;
        };
        Self::entity_in_approach_distance(&active.player.pathing, target, range as i32)
    }

    /// Attempts to execute an interaction between the player and its current
    /// target.
    ///
    /// Evaluates the following in order:
    ///
    /// 1. **OP trigger:** If an operate trigger exists, the target is a
    ///    pathing entity (or `allow_op_scenery` is set), and the player is
    ///    in operable distance, executes the OP script.
    /// 2. **AP trigger:** If an approach trigger exists and the player is
    ///    within approach distance, executes the AP script. If the script
    ///    calls `p_aprange`, waypoints are restored for continued approach.
    /// 3. **Default OP:** If no triggers exist but the player is in
    ///    operable distance, executes `default_op` (shows "Nothing
    ///    interesting happens.").
    ///
    /// After script execution, checks for `next_target` set by script
    /// commands like `p_oploc` / `p_opobj`.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    ///
    /// # Returns
    ///
    /// `true` if any interaction (OP, AP, or default) was executed.
    #[inline(always)]
    fn try_interact(active: *mut ActivePlayer, allow_op_scenery: bool) -> bool {
        let active = unsafe { &mut *active };
        if active.player.interaction.target.is_none()
            || !active.player.has_interaction()
            || !active.can_access()
        {
            return false;
        }

        let op_trigger = Self::get_trigger(active, 7);
        let ap_trigger = Self::get_trigger(active, 0);
        let has_op = op_trigger.is_some();
        let has_ap = ap_trigger.is_some();
        let is_pathing = active
            .player
            .interaction
            .target
            .as_ref()
            .is_some_and(|t| t.is_pathing_entity());
        let in_op = Self::in_operable_distance(active);

        if has_op && (is_pathing || allow_op_scenery) && in_op {
            let target = active.player.interaction.target.unwrap();
            let target_subject = Self::target_to_subject(&target);
            let uid = active.player.uid;
            let trigger = op_trigger.unwrap();

            active.player.interaction.target = None;
            active.player.clear_waypoints();

            if let Err(e) = engine_mut().run_script_by_trigger(
                trigger,
                Some(ScriptSubject::Player(uid)),
                target_subject,
                Some(true),
                None,
                None,
            ) {
                error!("error running op trigger for player {}: {e}", uid.pid());
            }

            // If p_oploc/p_opobj was called during the script, remember it for next cycle
            active.player.next_target = active.player.interaction.target;
            active.player.interaction.target = Some(target);
            return true;
        }

        if has_ap && Self::in_approach_distance(active) {
            active.player.interaction.ap_range_called = false;

            let target = active.player.interaction.target.unwrap();
            let target_subject = Self::target_to_subject(&target);
            let uid = active.player.uid;
            let trigger = ap_trigger.unwrap();

            let saved_waypoints = active.player.pathing.waypoints;
            let saved_waypoint_index = active.player.pathing.waypoint_index;

            active.player.interaction.target = None;
            active.player.clear_waypoints();

            if let Err(e) = engine_mut().run_script_by_trigger(
                trigger,
                Some(ScriptSubject::Player(uid)),
                target_subject,
                Some(true),
                None,
                None,
            ) {
                error!("error running ap trigger for player {}: {e}", uid.pid());
            }

            // If p_oploc/p_opobj was called, remember it for next cycle
            active.player.next_target = active.player.interaction.target;
            active.player.interaction.target = Some(target);

            // If p_opobj/p_oploc was called, clear waypoints for the new target
            if active.player.next_target.is_some() {
                active.player.clear_waypoints();
            } else if active.player.interaction.ap_range_called {
                active.player.pathing.waypoints = saved_waypoints;
                active.player.pathing.waypoint_index = saved_waypoint_index;
                return false;
            }
            return true;
        }

        if Self::in_approach_distance(active) {
            active.player.interaction.ap_range = None;
            return false;
        }

        if (is_pathing || allow_op_scenery) && in_op {
            Self::default_op(active, has_op, has_ap);
            return true;
        }

        false
    }

    /// Handles the fallback case when no OP or AP trigger script exists for
    /// the player's current interaction target.
    ///
    /// In debug builds, prints the missing trigger name and the target's
    /// debug name to the player's chat. Always shows the generic "Nothing
    /// interesting happens." message and clears the player's waypoints.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    ///
    /// # Side Effects
    ///
    /// * Sends a game message to the player.
    /// * Clears waypoints.
    #[inline(always)]
    fn default_op(active: *mut ActivePlayer, has_op: bool, has_ap: bool) {
        let active = unsafe { &mut *active };

        if !has_op && !has_ap {
            if let Some(target) = &active.player.interaction.target
                && let Some(target_op) = active.player.interaction.target_op
            {
                let op_trigger = target_op.wrapping_add(7);
                let trigger_name = ServerTriggerType::try_from(op_trigger)
                    .map(|t| format!("{:?}", t).to_lowercase())
                    .unwrap_or_else(|_| format!("unknown_{}", op_trigger));

                let debugname = match target {
                    InteractionTarget::Obj { id, .. } => cache()
                        .objs
                        .get_by_id(*id)
                        .and_then(|t| t.debugname().map(|s| s.to_string()))
                        .unwrap_or_else(|| id.to_string()),
                    InteractionTarget::Loc { id, .. } => cache()
                        .locs
                        .get_by_id(*id)
                        .and_then(|t| t.debugname().map(|s| s.to_string()))
                        .unwrap_or_else(|| id.to_string()),
                    InteractionTarget::Npc { nid } => engine()
                        .get_npc(*nid)
                        .and_then(|n| {
                            cache()
                                .npcs
                                .get_by_id(n.npc.uid.id())
                                .and_then(|t| t.debugname().map(|s| s.to_string()))
                        })
                        .unwrap_or_else(|| nid.to_string()),
                    InteractionTarget::Player { pid } => pid.to_string(),
                };
                active.message_game(&format!("No trigger for [{},{}]", trigger_name, debugname));
            }
        }

        active.message_game("Nothing interesting happens.");
        active.player.clear_waypoints();
    }

    /// Processes one tick of movement for the player along its queued
    /// waypoints.
    ///
    /// Sets the player's move speed to `Run` or `Walk` based on the
    /// player's current run toggle, then delegates to
    /// [`PathingEntity::process_movement`].
    ///
    /// Movement is skipped entirely (returning `false`) when the player has a
    /// pending move-click while [`busy`](rs_entity::Player::busy) and still has
    /// a non-empty primary or engine queue: a player cannot walk out from under
    /// a queued action with a modal open, behavior confirmed as far back as
    /// 2005.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see [`process_timers`].
    ///
    /// # Returns
    ///
    /// `true` if the player moved at least one tile this tick.
    #[inline(always)]
    fn process_movement(active: *mut ActivePlayer) -> bool {
        let active = unsafe { &mut *active };

        // Players cannot walk if they have a pending move-click *and* are busy
        // (a modal is open or they are delayed) *and* still have something
        // queued. Confirmed as far back as 2005.
        if active.player.move_request
            && active.player.busy()
            && (!active.player.state.queues.queue.is_empty())
        {
            return false;
        }

        if active.player.run {
            active.player.pathing.move_speed = MoveSpeed::Run;
        } else {
            active.player.pathing.move_speed = MoveSpeed::Walk;
        }

        if active.player.info.runanim.is_none() {
            active.player.pathing.move_speed = MoveSpeed::Walk;
        } else if active.player.temprun {
            active.player.pathing.move_speed = MoveSpeed::Run;
        }

        let members = engine().members;
        let steps_taken = active.player.pathing.process_movement(
            members,
            &mut active.player.info,
            FocusKind::Player,
        );

        if !steps_taken {
            active.player.temprun = false;
        }

        steps_taken
    }
}
