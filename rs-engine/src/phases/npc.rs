use crate::active_npc::ActiveNpc;
use crate::engine::Engine;
use crate::engine::{cache, engine, engine_mut};
use crate::phases::shared::EntityId;
use rs_entity::{Direction, InteractionTarget, PathingEntity};
use rs_grid::CoordGrid;
use rs_info::FocusKind;
use rs_pack::cache::CacheStore;
use rs_pack::cache::hunt::{HuntType, check_hunt_condition};
use rs_pack::types::*;
use rs_vm::engine::ScriptEngine;
use rs_vm::state::{ExecutionState, ScriptArgument};
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use rs_zone::zone_map::ZoneMap;
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::panic::{AssertUnwindSafe, catch_unwind};
use tracing::error;

impl Engine {
    /// Processes the NPC phase of the engine tick cycle.
    ///
    /// For each active NPC, within a panic-catching boundary, executes the
    /// following sub-phases in order:
    ///
    /// 1. Checks and clears the delay timer.
    /// 2. Resumes any `NpcSuspended` script from a previous tick.
    /// 3. Handles respawn if the NPC is dead and its respawn timer has
    ///    elapsed ([`respawn_npc`]).
    /// 4. Reverts the NPC's type if a temporary type-change has expired.
    /// 5. If the NPC is alive and not delayed, processes:
    ///    - Hunt target acquisition ([`npc_process_hunt`],
    ///      [`npc_consume_hunt_target`]).
    ///    - Stat regeneration ([`npc_process_regen`]).
    ///    - AI timer scripts ([`npc_process_timers`]).
    ///    - Script queue ([`npc_process_queue`]).
    ///    - Face-entity orientation.
    ///    - Movement and mode-based AI
    ///      ([`npc_process_movement_interaction`]).
    /// 6. Updates zone membership and collision maps.
    ///
    /// NPCs that panic during processing are emergency-deactivated.
    ///
    /// # Side Effects
    ///
    /// * Executes RuneScript AI scripts (timers, queues, hunt, modes).
    /// * Mutates NPC coordinate, path, interaction, and stat state.
    /// * Updates zone entity lists and collision flags.
    /// * May respawn dead NPCs and run their spawn scripts.
    /// * May emergency-deactivate NPCs that cause panics.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `npc_process_hunt`, `npc_consume_hunt_target`,
    ///   `npc_process_regen`, `npc_process_timers`, `npc_process_queue`,
    ///   `npc_process_movement_interaction`, `check_zones_and_collision`
    pub(crate) fn npcs(&mut self) {
        let nids = self.npc_list.take_nids();
        let mut start = 0;
        loop {
            let result = catch_unwind(AssertUnwindSafe(|| {
                for &nid in &nids[start..] {
                    Self::process_npc(self, nid);
                }
            }));
            match result {
                Ok(()) => break,
                Err(panic) => {
                    let nid = nids[start];
                    let msg = crate::phases::shared::panic_message(&panic);
                    error!("panic during npc processing for nid {nid}: {msg}");
                    self.emergency_deactivate_npc(nid);
                    start += 1;
                }
            }
        }
        self.npc_list.put_nids(nids);
    }

    #[inline(always)]
    fn process_npc(&mut self, nid: u16) {
        let Some(active) = self.npc_list.npcs[nid as usize].as_mut() else {
            return;
        };

        let prev_coord = active.npc.pathing.coord;

        if active.npc.active {
            active.npc.state.check_delay(self.clock);

            if !active.npc.state.delayed
                && active
                    .npc
                    .state
                    .active_script
                    .as_ref()
                    .is_some_and(|s| s.execution == ExecutionState::NpcSuspended)
            {
                let state = *active.npc.state.active_script.take().unwrap();
                if let Err(e) = engine_mut().run_script_by_state(
                    state,
                    Some(ScriptSubject::Npc(active.npc.uid)),
                    None,
                    None,
                ) {
                    error!(
                        "error resuming suspended script for npc {}: {e}",
                        active.npc.uid.nid()
                    );
                }
            }
        }

        if !active.npc.state.delayed && !active.npc.active {
            if let Some(remaining) = active.npc.respawn_at {
                if remaining <= 1 {
                    active.npc.respawn_at = None;
                    Self::respawn_npc(&mut self.zones, active, self.cache);
                    engine_mut().ai_spawn(active.npc.uid, active.npc.base_type);
                } else {
                    active.npc.respawn_at = Some(remaining - 1);
                }
            }
        }

        if !active.npc.state.delayed && active.npc.active {
            if let Some(remaining) = active.npc.revert_at {
                if remaining <= 1 {
                    active.revert_type();
                } else {
                    active.npc.revert_at = Some(remaining - 1);
                }
            }
        }

        if active.npc.state.delayed || !active.npc.active {
            return;
        }

        Self::npc_process_hunt(active);
        Self::npc_consume_hunt_target(active);
        Self::npc_process_regen(active);
        Self::npc_process_timers(active);
        Self::npc_process_queue(active);
        active.npc.set_face_entity();
        Self::npc_process_movement_interaction(active);

        if prev_coord != active.npc.pathing.coord {
            active.npc.pathing.last_movement = self.clock + 1;
        }

        Engine::check_zones_and_collision(
            &mut self.zones,
            prev_coord,
            active.npc.pathing.coord,
            EntityId::Npc(nid),
            active.npc.pathing.size,
            active.block_walk(),
        );
    }

    // ----

    /// Respawns a dead NPC at its original spawn coordinate.
    ///
    /// Resets the NPC's state to its initial configuration:
    ///
    /// * Moves the NPC back to its spawn coordinate.
    /// * Restores base combat levels from the NPC type definition.
    /// * Resets the pathing entity, animation, variables, and defaults
    ///   (mode, hunt, timer).
    /// * Re-registers the NPC in the zone map and updates collision flags
    ///   according to the NPC's `block_walk` setting.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * Adds the NPC to its spawn zone.
    /// * Updates collision maps (`rsmod::change_npc` / `change_player`).
    /// * Resets all NPC state fields.
    #[inline(always)]
    fn respawn_npc(zones: &mut ZoneMap, active: *mut ActiveNpc, cache: &CacheStore) {
        let active = unsafe { &mut *active };
        active.npc.pathing.coord = active.npc.spawn_coord;
        active.npc.active = true;
        active.npc.respawn_at = None;
        active.npc.state.delayed = false;
        active.npc.state.delayed_until = 0;
        active.npc.state.active_script = None;

        let npc_type = cache.npcs.get_by_id(active.npc.base_type);
        if let Some(npc_type) = npc_type {
            active.npc.stats.base_levels[NpcStat::Attack as usize] = npc_type.attack;
            active.npc.stats.base_levels[NpcStat::Defence as usize] = npc_type.defence;
            active.npc.stats.base_levels[NpcStat::Strength as usize] = npc_type.strength;
            active.npc.stats.base_levels[NpcStat::Hitpoints as usize] = npc_type.hitpoints;
            active.npc.stats.base_levels[NpcStat::Ranged as usize] = npc_type.ranged;
            active.npc.stats.base_levels[NpcStat::Magic as usize] = npc_type.magic;
        }

        active.npc.reset_pathing_entity(true);
        active.anim(None, 0);
        active
            .npc
            .vars
            .reset(cache.varns.types.iter().map(|v| v.var_type));
        active.npc.reset_defaults(
            npc_type.map(|t| t.defaultmode).unwrap_or(NpcMode::None),
            npc_type.and_then(|t| t.huntmode),
            npc_type.map(|t| t.huntrange).unwrap_or(0),
            npc_type.and_then(|t| t.timer),
        );

        zones
            .zone_mut(
                active.npc.pathing.coord.x(),
                active.npc.pathing.coord.y(),
                active.npc.pathing.coord.z(),
            )
            .add_npc(active.npc.uid.nid());

        let bw = active.block_walk();
        match bw {
            BlockWalk::Npc => {
                rsmod::change_npc(
                    active.npc.pathing.coord.x(),
                    active.npc.pathing.coord.z(),
                    active.npc.pathing.coord.y(),
                    active.npc.pathing.size,
                    true,
                );
            }
            BlockWalk::All => {
                rsmod::change_npc(
                    active.npc.pathing.coord.x(),
                    active.npc.pathing.coord.z(),
                    active.npc.pathing.coord.y(),
                    active.npc.pathing.size,
                    true,
                );
                rsmod::change_player(
                    active.npc.pathing.coord.x(),
                    active.npc.pathing.coord.z(),
                    active.npc.pathing.coord.y(),
                    active.npc.pathing.size,
                    true,
                );
            }
            _ => {}
        }
    }

    /// Processes hitpoint regeneration for an NPC.
    ///
    /// Decrements the NPC's `regen_clock` each tick. When it reaches zero,
    /// adjusts the NPC's current hitpoints toward its maximum by +1 or -1
    /// and resets the clock to the NPC type's configured `regenrate`. Does
    /// nothing if the NPC type has a regen rate of zero.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * Modifies `active.npc.levels[Hitpoints]`.
    /// * Resets `active.npc.regen_clock`.
    #[inline(always)]
    fn npc_process_regen(active: *mut ActiveNpc) {
        let active = unsafe { &mut *active };
        let Some(npc_type) = cache().npcs.get_by_id(active.npc.uid.id()) else {
            return;
        };
        if npc_type.regenrate == 0 {
            return;
        }

        active.npc.regen_clock -= 1;
        if active.npc.regen_clock > 0 {
            return;
        }

        active.npc.regen_clock = npc_type.regenrate as i16;

        if active.npc.stats.levels[NpcStat::Hitpoints as usize]
            < active.npc.stats.base_levels[NpcStat::Hitpoints as usize]
        {
            active.npc.stats.levels[NpcStat::Hitpoints as usize] += 1;
        } else if active.npc.stats.levels[NpcStat::Hitpoints as usize]
            > active.npc.stats.base_levels[NpcStat::Hitpoints as usize]
        {
            active.npc.stats.levels[NpcStat::Hitpoints as usize] -= 1;
        }
    }

    /// Fires the NPC's AI timer script when its interval has elapsed.
    ///
    /// Increments `timer_clock` each tick. When it reaches the configured
    /// `timer_interval`, looks up and executes the `AiTimer` trigger script
    /// for the NPC's current type and category. Resets the timer clock
    /// after execution.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * Executes a RuneScript `AiTimer` trigger.
    /// * Resets `active.npc.timer_clock` to 0.
    #[inline(always)]
    fn npc_process_timers(active: *mut ActiveNpc) {
        let active = unsafe { &mut *active };
        let interval = match active.npc.timer_interval {
            Some(i) if i > 0 => i,
            _ => return,
        };

        active.npc.timer_clock += 1;
        if active.npc.timer_clock < interval {
            return;
        }

        let uid = active.npc.uid;
        let type_id = uid.id();
        let category = cache()
            .npcs
            .get_by_id(type_id)
            .and_then(|t| t.category)
            .map(|c| c as i32);
        let trigger = (ServerTriggerType::AiTimer, Some(type_id), category);
        if engine()
            .script_by_key(trigger.0, trigger.1, trigger.2)
            .is_some()
        {
            if let Err(e) = engine_mut().run_script_by_trigger(
                trigger,
                Some(ScriptSubject::Npc(uid)),
                None,
                None,
                None,
                None,
            ) {
                error!("error running timer script for npc {}: {e}", uid.nid());
            }
            active.npc.timer_clock = 0;
        }
    }

    /// Drains and executes the NPC's script queue.
    ///
    /// Each queue entry's delay is decremented per tick (unless the NPC is
    /// delayed). When the delay reaches zero, the entry is unlinked, the
    /// trigger type is derived from the script ID, and the corresponding
    /// `AiQueue` RuneScript trigger is looked up and executed with the
    /// entry's arguments. The `last_int` value from the arguments is
    /// passed through for use by the script.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * Executes RuneScript `AiQueue` trigger scripts.
    /// * Removes entries from the NPC's script queue.
    #[inline(always)]
    fn npc_process_queue(active: *mut ActiveNpc) {
        let active = unsafe { &mut *active };

        if !active.npc.active {
            return;
        }

        let uid = active.npc.uid;

        let mut h = active.npc.state.queues.queue.head();
        while let Some(idx) = h {
            if !active.npc.state.delayed {
                let delay = active.npc.state.queues.queue[idx].delay;
                active.npc.state.queues.queue[idx].delay = delay.saturating_sub(1);
            }

            if !active.npc.state.delayed && active.npc.state.queues.queue[idx].delay == 0 {
                let request = active.npc.state.queues.queue.unlink(idx);
                let type_id = active.npc.uid.id();
                let category = cache()
                    .npcs
                    .get_by_id(type_id)
                    .and_then(|t| t.category)
                    .map(|c| c as i32);

                let last_int = request.args.as_ref().and_then(|args| {
                    args.iter().find_map(|a| match a {
                        ScriptArgument::Int(v) => Some(*v),
                        _ => None,
                    })
                });

                let trigger = ServerTriggerType::try_from(request.script_id as u8);
                if let Ok(trigger) = trigger {
                    if let Some(script) = engine().script_by_key(trigger, Some(type_id), category) {
                        let mut state = engine_mut().build_state(
                            script,
                            Some(ScriptSubject::Npc(uid)),
                            None,
                            request.args,
                        );
                        state.last_int = last_int;
                        if let Err(e) = engine_mut().run_script_by_state(
                            state,
                            Some(ScriptSubject::Npc(uid)),
                            None,
                            None,
                        ) {
                            error!("error running queue script for npc {}: {e}", uid.nid());
                        }
                    }
                }

                // Update the raw pointer for subsequent iterations
                h = active.npc.state.queues.queue.next();
                continue;
            }

            h = active.npc.state.queues.queue.next();
        }
    }

    /// Runs the NPC's hunt logic to search for a new target.
    ///
    /// Skips processing when:
    /// * The NPC has no hunt mode set.
    /// * The hunt's `nobodynear` policy is `PauseHunt` and there are no
    ///   observers (and it is not a player hunt).
    ///
    /// Player-type hunts are handled separately in the world phase
    /// ([`process_npc_hunt_players`]), so they are skipped here. All other
    /// hunt types (NPC, Obj, Scenery) are processed via [`npc_hunt_all`].
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * Sets `active.npc.hunt_target` if a target is found.
    /// * Increments `active.npc.hunt_clock`.
    #[inline(always)]
    fn npc_process_hunt(active: *mut ActiveNpc) {
        let active = unsafe { &mut *active };
        let Some(hunt_id) = active.npc.hunt_mode else {
            return;
        };
        let Some(hunt) = cache().hunts.get_by_id(hunt_id) else {
            return;
        };

        let should_process = hunt.nobodynear != HuntNobodyNear::PauseHunt
            || active.npc.observers > 0
            || hunt.hunt_type == HuntModeType::Player;

        if !should_process {
            return;
        }

        // PLAYER hunts are processed in the world phase (process_npc_hunt_players).
        if hunt.hunt_type != HuntModeType::Player {
            Self::npc_hunt_all(active, hunt);
        }

        active.npc.hunt_clock += 1;
    }

    /// Searches for a hunt target of the configured type within the NPC's
    /// hunt range.
    ///
    /// Dispatches to the type-specific hunt scanner:
    ///
    /// * [`npc_hunt_players`] for `Player` hunts.
    /// * [`npc_hunt_npcs`] for `Npc` hunts.
    /// * [`npc_hunt_objs`] for `Obj` hunts.
    /// * [`npc_hunt_locs`] for `Scenery` hunts.
    ///
    /// Respects the hunt's `rate` to throttle how frequently the NPC scans.
    /// Sets `active.npc.hunt_target` to the selected target (chosen via
    /// reservoir sampling for uniform random selection).
    ///
    /// # References
    ///
    /// * <https://x.com/JagexAsh/status/1821236327150710829>
    /// * <https://x.com/JagexAsh/status/1799793914595131463>
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * Sets `active.npc.hunt_target`.
    pub(crate) fn npc_hunt_all(active: *mut ActiveNpc, hunt: &HuntType) {
        let active = unsafe { &mut *active };
        active.npc.hunt_target = None;

        if active.npc.hunt_clock < hunt.rate.saturating_sub(1) {
            return;
        }

        if hunt.hunt_type == HuntModeType::Off || active.npc.hunt_range < 1 {
            return;
        }

        let coord = active.npc.pathing.coord;
        let distance = active.npc.hunt_range as i32;
        let vis = hunt.check_vis;

        let target = match hunt.hunt_type {
            HuntModeType::Player => Self::npc_hunt_players(active, hunt, coord, distance, vis),
            HuntModeType::Npc => {
                Self::npc_hunt_npcs(coord, distance, vis, hunt.check_npc, hunt.check_category)
            }
            HuntModeType::Obj => {
                Self::npc_hunt_objs(coord, distance, vis, hunt.check_obj, hunt.check_category)
            }
            HuntModeType::Scenery => {
                Self::npc_hunt_locs(coord, distance, vis, hunt.check_loc, hunt.check_category)
            }
            HuntModeType::Off => None,
        };

        active.npc.hunt_target = target;
    }

    /// Scans nearby zones for player hunt targets.
    ///
    /// Iterates over all players in zones within the hunt range, applying
    /// the full suite of hunt condition checks:
    ///
    /// * Distance and line-of-sight / line-of-walk visibility.
    /// * Not-busy, not-AFK, not-too-strong filters.
    /// * Multi-combat zone and recent-combat varp checks.
    /// * Extra variable conditions and inventory checks.
    ///
    /// Uses reservoir sampling (`next_int_bound(count)`) to select a
    /// uniformly random target from all qualifying players.
    ///
    /// # Returns
    ///
    /// An `InteractionTarget::Player` for the chosen player, or `None` if
    /// no valid target was found.
    #[inline(always)]
    fn npc_hunt_players(
        active: &ActiveNpc,
        hunt: &HuntType,
        coord: CoordGrid,
        distance: i32,
        vis: HuntCheckVis,
    ) -> Option<InteractionTarget> {
        let center_zx = CoordGrid::zone(coord.x()) as i32;
        let center_zz = CoordGrid::zone(coord.z()) as i32;
        let radius = 1 + (distance >> 3);
        let mut chosen: Option<u16> = None;
        let mut count: u32 = 0;

        let engine = engine();
        let cache = cache();
        let clock = engine.clock as i32;

        let npc_type = cache.npcs.get_by_id(active.npc.uid.id());

        for zx in ((center_zx - radius)..=(center_zx + radius)).rev() {
            for zz in ((center_zz - radius)..=(center_zz + radius)).rev() {
                if zx < 0 || zz < 0 {
                    continue;
                }
                let zone_x = (zx as u16) << 3;
                let zone_z = (zz as u16) << 3;
                let Some(zone) = engine.zones.zone(zone_x, coord.y(), zone_z) else {
                    continue;
                };

                for &pid in &zone.players {
                    let Some(player) = engine.get_player(pid) else {
                        continue;
                    };
                    let player_coord = player.player.pathing.coord;
                    if coord.distance(player_coord) > distance {
                        continue;
                    }
                    match vis {
                        HuntCheckVis::LineOfSight => {
                            if !engine.lineofsight(coord, player_coord) {
                                continue;
                            }
                        }
                        HuntCheckVis::LineOfWalk => {
                            if !engine.lineofwalk(coord, player_coord) {
                                continue;
                            }
                        }
                        HuntCheckVis::Off => {}
                    }

                    if hunt.check_notbusy == HuntCheckNotBusy::On && player.player.busy() {
                        continue;
                    }

                    if hunt.check_afk == HuntCheckAfk::On && player.player.afk_event_ready {
                        continue;
                    }

                    if hunt.check_nottoostrong == HuntCheckNotTooStrong::OutsideWilderness {
                        if let Some(nt) = npc_type {
                            let vislevel = nt.vislevel.unwrap_or(0);
                            if !player_coord.is_in_wilderness()
                                && player.player.combat_level as u16 > vislevel * 2
                            {
                                continue;
                            }
                        }
                    }

                    let is_current_target = matches!(
                        active.npc.interaction.target,
                        Some(InteractionTarget::Player { pid: tp }) if tp == pid
                    );

                    if !is_current_target
                        && !cache.is_multi(player_coord.x(), player_coord.z(), player_coord.y())
                    {
                        if let Some(varp_id) = hunt.check_notcombat {
                            if player.player.vars.len() > varp_id as usize {
                                let last_combat = player.player.vars.get(varp_id).as_int();
                                if last_combat + 8 > clock {
                                    continue;
                                }
                            }
                        }
                        if let Some(varn_id) = hunt.check_notcombat_self {
                            if active.npc.vars.len() > varn_id as usize {
                                let npc_last_combat = active.npc.vars.get(varn_id).as_int();
                                if npc_last_combat + 8 > clock {
                                    continue;
                                }
                            }
                        }
                    }

                    if !hunt.extracheck_vars.is_empty() {
                        let mut pass = true;
                        for check_var in &hunt.extracheck_vars {
                            if player.player.vars.len() > check_var.varp as usize {
                                let val = player.player.vars.get(check_var.varp).as_int();
                                if !check_hunt_condition(val, &check_var.condition, check_var.value)
                                {
                                    pass = false;
                                    break;
                                }
                            }
                        }
                        if !pass {
                            continue;
                        }
                    }

                    if let Some(ref inv_check) = hunt.check_inv {
                        let quantity = player
                            .player
                            .invs
                            .get(&inv_check.inv)
                            .map(|inv| inv.total(inv_check.obj) as i32)
                            .unwrap_or(0);
                        if !check_hunt_condition(quantity, &inv_check.condition, inv_check.value) {
                            continue;
                        }
                    }

                    if let Some(ref param_check) = hunt.check_invparam {
                        let quantity = player
                            .player
                            .invs
                            .get(&param_check.inv)
                            .map(|inv| {
                                let param = cache.params.get_by_id(param_check.param);
                                inv.slots
                                    .iter()
                                    .filter_map(|s| s.as_ref())
                                    .map(|item| {
                                        let value = param
                                            .and_then(|p| {
                                                cache
                                                    .objs
                                                    .get_by_id(item.obj)
                                                    .and_then(|o| o.params.as_ref())
                                                    .and_then(|ps| {
                                                        ps.get(&(param_check.param as i32))
                                                    })
                                                    .map(|v| match v {
                                                        ParamValue::Int(i) => *i,
                                                        _ => p.default_int,
                                                    })
                                            })
                                            .unwrap_or(0);
                                        (item.num as i32).wrapping_mul(value)
                                    })
                                    .fold(0i32, |a, b| a.wrapping_add(b))
                            })
                            .unwrap_or(0);
                        if !check_hunt_condition(
                            quantity,
                            &param_check.condition,
                            param_check.value,
                        ) {
                            continue;
                        }
                    }

                    count += 1;
                    if engine_mut().random.next_int_bound(count as i32) == 0 {
                        chosen = Some(pid);
                    }
                }
            }
        }

        chosen.map(|pid| InteractionTarget::Player { pid })
    }

    /// Scans nearby zones for NPC hunt targets.
    ///
    /// Searches active NPCs within the hunt range, filtering by optional
    /// NPC type ID and/or category. Applies distance and visibility checks.
    /// Uses reservoir sampling for uniform random selection.
    ///
    /// # Returns
    ///
    /// An `InteractionTarget::Npc` for the chosen NPC, or `None` if no
    /// valid target was found.
    #[inline(always)]
    fn npc_hunt_npcs(
        coord: CoordGrid,
        distance: i32,
        vis: HuntCheckVis,
        check_npc: Option<u16>,
        check_category: Option<u16>,
    ) -> Option<InteractionTarget> {
        let center_zx = CoordGrid::zone(coord.x()) as i32;
        let center_zz = CoordGrid::zone(coord.z()) as i32;
        let radius = 1 + (distance >> 3);
        let mut chosen: Option<u16> = None;
        let mut count: u32 = 0;

        let engine = engine();
        let cache = cache();

        for zx in ((center_zx - radius)..=(center_zx + radius)).rev() {
            for zz in ((center_zz - radius)..=(center_zz + radius)).rev() {
                if zx < 0 || zz < 0 {
                    continue;
                }
                let Some(zone) = engine
                    .zones
                    .zone((zx as u16) << 3, coord.y(), (zz as u16) << 3)
                else {
                    continue;
                };
                for &nid in &zone.npcs {
                    let Some(active_npc) = engine.get_npc(nid) else {
                        continue;
                    };
                    if !active_npc.npc.active {
                        continue;
                    }
                    let npc_type_id = active_npc.npc.uid.id();
                    let npc_coord = active_npc.npc.pathing.coord;
                    if let Some(id) = check_npc
                        && npc_type_id != id
                    {
                        continue;
                    }
                    if coord.distance(npc_coord) > distance {
                        continue;
                    }
                    if let Some(cat) = check_category {
                        let npc_cat = cache.npcs.get_by_id(npc_type_id).and_then(|t| t.category);
                        if npc_cat != Some(cat) {
                            continue;
                        }
                    }
                    match vis {
                        HuntCheckVis::LineOfSight => {
                            if !engine.lineofsight(coord, npc_coord) {
                                continue;
                            }
                        }
                        HuntCheckVis::LineOfWalk => {
                            if !engine.lineofwalk(coord, npc_coord) {
                                continue;
                            }
                        }
                        HuntCheckVis::Off => {}
                    }
                    count += 1;
                    if engine_mut().random.next_int_bound(count as i32) == 0 {
                        chosen = Some(nid);
                    }
                }
            }
        }

        chosen.map(|nid| InteractionTarget::Npc { nid })
    }

    /// Scans nearby zones for ground-item (Obj) hunt targets.
    ///
    /// Searches visible objects within the hunt range, filtering by
    /// optional object type ID and/or category. Only considers objects
    /// whose visibility clock has passed. Applies distance and visibility
    /// checks. Uses reservoir sampling for uniform random selection.
    ///
    /// # Returns
    ///
    /// An `InteractionTarget::Obj` for the chosen object, or `None` if no
    /// valid target was found.
    #[inline(always)]
    fn npc_hunt_objs(
        coord: CoordGrid,
        distance: i32,
        vis: HuntCheckVis,
        check_obj: Option<u16>,
        check_category: Option<u16>,
    ) -> Option<InteractionTarget> {
        let center_zx = CoordGrid::zone(coord.x()) as i32;
        let center_zz = CoordGrid::zone(coord.z()) as i32;
        let radius = 1 + (distance >> 3);
        let mut chosen: Option<(CoordGrid, u16, u32)> = None;
        let mut count: u32 = 0;

        let engine = engine();
        let cache = cache();
        let clock = engine.clock();

        for zx in ((center_zx - radius)..=(center_zx + radius)).rev() {
            for zz in ((center_zz - radius)..=(center_zz + radius)).rev() {
                if zx < 0 || zz < 0 {
                    continue;
                }
                let zone_x = (zx as u16) << 3;
                let zone_z = (zz as u16) << 3;
                let Some(zone) = engine.zones.zone(zone_x, coord.y(), zone_z) else {
                    continue;
                };
                for obj in &zone.objs {
                    if !obj.visible(clock) {
                        continue;
                    }
                    if let Some(id) = check_obj
                        && obj.id() != id
                    {
                        continue;
                    }
                    let obj_coord = obj.world_coord(zone.coord);
                    if coord.distance(obj_coord) > distance {
                        continue;
                    }
                    if let Some(cat) = check_category {
                        let obj_cat = cache.objs.get_by_id(obj.id()).and_then(|t| t.category);
                        if obj_cat != Some(cat) {
                            continue;
                        }
                    }
                    match vis {
                        HuntCheckVis::LineOfSight => {
                            if !engine.lineofsight(coord, obj_coord) {
                                continue;
                            }
                        }
                        HuntCheckVis::LineOfWalk => {
                            if !engine.lineofwalk(coord, obj_coord) {
                                continue;
                            }
                        }
                        HuntCheckVis::Off => {}
                    }
                    count += 1;
                    if engine_mut().random.next_int_bound(count as i32) == 0 {
                        chosen = Some((obj_coord, obj.id(), obj.count()));
                    }
                }
            }
        }

        chosen.map(|(c, id, count)| InteractionTarget::Obj {
            coord: c,
            id,
            count,
        })
    }

    /// Scans nearby zones for scenery (Loc) hunt targets.
    ///
    /// Searches location entities within the hunt range, filtering by
    /// optional loc type ID and/or category. Applies distance and
    /// visibility checks. Resolves the loc's width, length, shape, angle,
    /// and layer for the resulting interaction target. Uses reservoir
    /// sampling for uniform random selection.
    ///
    /// # Returns
    ///
    /// An `InteractionTarget::Loc` for the chosen location, or `None` if
    /// no valid target was found.
    #[inline(always)]
    fn npc_hunt_locs(
        coord: CoordGrid,
        distance: i32,
        vis: HuntCheckVis,
        check_loc: Option<u16>,
        check_category: Option<u16>,
    ) -> Option<InteractionTarget> {
        let center_zx = CoordGrid::zone(coord.x()) as i32;
        let center_zz = CoordGrid::zone(coord.z()) as i32;
        let radius = 1 + (distance >> 3);
        let mut chosen: Option<InteractionTarget> = None;
        let mut count: u32 = 0;

        let engine = engine();
        let cache = cache();

        for zx in ((center_zx - radius)..=(center_zx + radius)).rev() {
            for zz in ((center_zz - radius)..=(center_zz + radius)).rev() {
                if zx < 0 || zz < 0 {
                    continue;
                }
                let locs = engine.get_zone_locs((zx as u16) << 3, coord.y(), (zz as u16) << 3);
                for loc_ref in locs {
                    if let Some(id) = check_loc
                        && loc_ref.id != id
                    {
                        continue;
                    }
                    let loc_coord = CoordGrid::from(loc_ref.coord);
                    if coord.distance(loc_coord) > distance {
                        continue;
                    }
                    let loc_type = cache.locs.get_by_id(loc_ref.id);
                    if let Some(cat) = check_category {
                        let loc_cat = loc_type.and_then(|t| t.category);
                        if loc_cat != Some(cat) {
                            continue;
                        }
                    }
                    match vis {
                        HuntCheckVis::LineOfSight => {
                            if !engine.lineofsight(coord, loc_coord) {
                                continue;
                            }
                        }
                        HuntCheckVis::LineOfWalk => {
                            if !engine.lineofwalk(coord, loc_coord) {
                                continue;
                            }
                        }
                        HuntCheckVis::Off => {}
                    }
                    let width = loc_type.map(|lt| lt.width).unwrap_or(1);
                    let length = loc_type.map(|lt| lt.length).unwrap_or(1);
                    count += 1;
                    if engine_mut().random.next_int_bound(count as i32) == 0 {
                        chosen = Some(InteractionTarget::Loc {
                            coord: loc_coord,
                            id: loc_ref.id,
                            width,
                            length,
                            shape: LocShape::try_from(loc_ref.shape)
                                .unwrap_or(LocShape::CentrepieceStraight),
                            angle: LocAngle::try_from(loc_ref.angle).unwrap_or(LocAngle::North),
                            layer: LocLayer::try_from(loc_ref.layer).unwrap_or(LocLayer::Ground),
                        });
                    }
                }
            }
        }

        chosen
    }

    /// Consumes the NPC's pending hunt target and transitions the NPC into
    /// its `find_newmode`.
    ///
    /// If the hunt's `find_newmode` maps to a queue mode (`Queue1` through
    /// `Queue20`), the corresponding `AiQueue` trigger script is executed
    /// directly. Otherwise, the NPC's interaction is set to the target
    /// with the mode's op code.
    ///
    /// After consumption, the hunt clock is reset. If the hunt's
    /// `find_keephunting` flag is off, the NPC's hunt mode is cleared so
    /// it will not hunt again until re-assigned.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * Sets the NPC's interaction target or executes a queue trigger.
    /// * Resets `hunt_clock` to 0.
    /// * May clear `hunt_mode`.
    #[inline(always)]
    fn npc_consume_hunt_target(active: *mut ActiveNpc) {
        let active = unsafe { &mut *active };
        let Some(hunt_id) = active.npc.hunt_mode else {
            return;
        };
        let Some(hunt) = cache().hunts.get_by_id(hunt_id) else {
            return;
        };
        let Some(target) = active.npc.hunt_target.take() else {
            return;
        };
        if hunt.hunt_type == HuntModeType::Off {
            return;
        }

        let find_new_mode = hunt.find_newmode;
        let keep_hunting = hunt.find_keephunting == HuntFindKeepHunting::On;

        // Queue-based hunt: run a queue trigger script instead of setting interaction
        if find_new_mode as u8 >= NpcMode::Queue1 as u8
            && find_new_mode as u8 <= NpcMode::Queue20 as u8
        {
            let queue_offset = find_new_mode as u8 - NpcMode::Queue1 as u8;
            let trigger_base = ServerTriggerType::AiQueue1 as u8;
            if let Ok(trigger) = ServerTriggerType::try_from(trigger_base + queue_offset) {
                let uid = active.npc.uid;
                let type_id = uid.id();
                let category = cache()
                    .npcs
                    .get_by_id(type_id)
                    .and_then(|t| t.category)
                    .map(|c| c as i32);
                if let Err(e) = engine_mut().run_script_by_trigger(
                    (trigger, Some(type_id), category),
                    Some(ScriptSubject::Npc(uid)),
                    None,
                    None,
                    None,
                    None,
                ) {
                    error!("error running hunt queue script for npc {}: {e}", uid.nid());
                }
            }
        } else {
            let op = find_new_mode as u8;
            active.npc.set_interaction(target, op, false);
        }

        active.npc.hunt_clock = 0;

        if !keep_hunting {
            active.npc.hunt_mode = None;
        }
    }

    /// Dispatches the NPC to its current mode handler for movement and
    /// interaction.
    ///
    /// Determines the NPC's active mode from `target_op` and delegates to
    /// the appropriate handler:
    ///
    /// * `None` -- [`npc_no_mode`] (idle, process pending waypoints).
    /// * `Wander` -- [`npc_wander_mode`] (random wandering near spawn).
    /// * `Patrol` -- [`npc_patrol_mode`] (follow a patrol route).
    /// * `PlayerEscape` -- [`npc_player_escape_mode`] (flee from player).
    /// * `PlayerFollow` -- [`npc_player_follow_mode`] (path toward player).
    /// * `PlayerFace` -- face the player with no movement.
    /// * `PlayerFaceClose` -- [`npc_player_face_close_mode`] (face if
    ///   adjacent, reset otherwise).
    /// * All other modes (Op/Ap) -- [`npc_ai_mode`] (interact or pursue).
    ///
    /// Targeted modes validate their target first and reset to defaults if
    /// the target is invalid.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * May move the NPC, execute AI scripts, or reset the NPC to
    ///   defaults.
    #[inline(always)]
    fn npc_process_movement_interaction(active: *mut ActiveNpc) {
        let active = unsafe { &mut *active };

        if active.npc.state.delayed || !active.npc.active {
            return;
        }

        let target_op = active.npc.interaction.target_op;

        // Failsafe: if target_op is somehow invalid, reset to default
        if target_op.is_none() && active.npc.interaction.target.is_none() {
            let npc_type = cache().npcs.get_by_id(active.npc.uid.id());
            let default = npc_type.map(|t| t.defaultmode).unwrap_or(NpcMode::None);
            active.npc.interaction.target_op = Some(default as u8);
        }

        let mode = active
            .npc
            .interaction
            .target_op
            .map(|x| NpcMode::try_from(x).unwrap_or(NpcMode::None));

        match mode {
            Some(NpcMode::None) => {
                Self::npc_no_mode(active);
            }
            Some(NpcMode::Wander) => {
                Self::npc_wander_mode(active);
            }
            Some(NpcMode::Patrol) => {
                Self::npc_patrol_mode(active);
            }
            _ => {
                // Targeted modes - validate target first
                if active.npc.interaction.target.is_none() || !Self::npc_validate_target(active) {
                    Self::npc_reset_defaults(active);
                    return;
                }

                match mode {
                    Some(NpcMode::PlayerEscape) => {
                        Self::npc_player_escape_mode(active);
                    }
                    Some(NpcMode::PlayerFollow) => {
                        Self::npc_player_follow_mode(active);
                    }
                    Some(NpcMode::PlayerFace) => {
                        // Just face the player, nothing else needed
                    }
                    Some(NpcMode::PlayerFaceClose) => {
                        Self::npc_player_face_close_mode(active);
                    }
                    _ => {
                        Self::npc_ai_mode(active);
                    }
                }
            }
        }
    }

    // ---- Mode implementations

    /// Handles the `None` NPC mode (idle).
    ///
    /// Simply processes any pending movement waypoints without generating
    /// new ones.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    #[inline(always)]
    fn npc_no_mode(active: *mut ActiveNpc) {
        Self::npc_process_movement(active);
    }

    /// Handles the `Wander` NPC mode (random movement near spawn).
    ///
    /// With a 1-in-8 chance per tick, picks a random destination within the
    /// NPC type's `wanderrange` of its spawn coordinate and queues a
    /// waypoint. After processing movement, increments a wander counter;
    /// if the counter exceeds 500 ticks without the NPC returning to
    /// spawn, teleports it back.
    ///
    /// Skips waypoint generation if the NPC type's `moverestrict` is
    /// `NoMove`.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * May queue a random waypoint.
    /// * May teleport the NPC back to spawn.
    #[inline(always)]
    fn npc_wander_mode(active: *mut ActiveNpc) {
        let active = unsafe { &mut *active };
        let Some(npc_type) = cache().npcs.get_by_id(active.npc.uid.id()) else {
            return;
        };

        if npc_type.moverestrict != MoveRestrict::NoMove {
            if engine_mut().random.next_int_bound(8) == 0 {
                let range = npc_type.wanderrange as i32;
                let dx = engine_mut().random.next_int_bound(range * 2 + 1) - range;
                let dz = engine_mut().random.next_int_bound(range * 2 + 1) - range;
                let dest_x = (active.npc.spawn_coord.x() as i32 + dx) as u16;
                let dest_z = (active.npc.spawn_coord.z() as i32 + dz) as u16;
                if dest_x != active.npc.pathing.coord.x() || dest_z != active.npc.pathing.coord.z()
                {
                    active.npc.pathing.queue_waypoint(dest_x, dest_z);
                }
            }
        }

        Self::npc_process_movement(active);

        let on_spawn = active.npc.pathing.coord.x() == active.npc.spawn_coord.x()
            && active.npc.pathing.coord.z() == active.npc.spawn_coord.z()
            && active.npc.pathing.coord.y() == active.npc.spawn_coord.y();

        // Npc should teleport 501 ticks after its last movement.
        let stuck = active.npc.stuck_counter;
        active.npc.stuck_counter += 1;
        if stuck > 500 {
            if !on_spawn {
                active.tele(active.npc.spawn_coord);
            }
            active.npc.stuck_counter = 0;
        }
    }

    /// Handles the `Patrol` NPC mode (follow a predefined route).
    ///
    /// Walks the NPC through the patrol points defined in its NPC type.
    /// At each waypoint, pauses for the configured delay before advancing
    /// to the next point. If the NPC fails to reach a patrol point within
    /// 32 ticks of its last movement -- or the point is on another floor --
    /// it is teleported there.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * Queues waypoints along the patrol route.
    /// * May teleport the NPC to unreached patrol points.
    /// * Advances `next_patrol_point`.
    #[inline(always)]
    fn npc_patrol_mode(active: *mut ActiveNpc) {
        let active = unsafe { &mut *active };
        let Some(npc_type) = cache().npcs.get_by_id(active.npc.uid.id()) else {
            return;
        };
        let Some(patrol) = &npc_type.patrol else {
            return;
        };

        // No patrol points configured: just process movement and bail.
        if patrol.is_empty() {
            Self::npc_process_movement(active);
            return;
        }

        let len = patrol.len();
        let point_idx = active.npc.next_patrol_point as usize % len;
        let mut dest = CoordGrid::from(patrol[point_idx].coord as u32);

        if !active.npc.pathing.has_waypoints() && active.npc.interaction.target.is_none() {
            // requeue waypoints in cases where an npc was interacting and the interaction has been cleared
            active.npc.pathing.queue_waypoint(dest.x(), dest.z());
        }

        active.npc.stuck_counter += 1;

        // Npc should teleport 32 ticks after its last movement, or if it needs to change floors.
        if active.npc.stuck_counter >= 32 || active.npc.pathing.coord.y() != dest.y() {
            active.tele(dest);
            active.npc.stuck_counter = 0;
        }

        if active.npc.pathing.coord.x() == dest.x() && active.npc.pathing.coord.z() == dest.z() {
            // If the patrol delay is uninitialized, seed it from this point's delay.
            if active.npc.patrol_delay_ticks_remaining < 0 {
                active.npc.patrol_delay_ticks_remaining = patrol[point_idx].delay as i64;
            }

            let remaining = active.npc.patrol_delay_ticks_remaining;
            active.npc.patrol_delay_ticks_remaining -= 1;
            if remaining <= 0 {
                active.npc.next_patrol_point = ((point_idx + 1) % len) as u8;
                active.npc.patrol_delay_ticks_remaining = -1;
                let next_idx = active.npc.next_patrol_point as usize % len;
                dest = CoordGrid::from(patrol[next_idx].coord as u32);
                active.npc.pathing.queue_waypoint(dest.x(), dest.z());
            }
        }

        Self::npc_process_movement(active);
    }

    /// Handles the `PlayerEscape` NPC mode (flee from a player).
    ///
    /// Picks the escape direction (diagonally away from the player) and queues
    /// a step: the diagonal if it is walkable and stays within the NPC's
    /// `maxrange` of spawn, otherwise the X axis, otherwise the Z axis (none if
    /// all are blocked or out of range). Resets to defaults if the target
    /// player is gone or more than 25 tiles away, or if the NPC has failed to
    /// move for 5 ticks while not yet cornered at `maxrange` on both axes.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * Queues an escape waypoint and processes movement.
    /// * May reset the NPC to default mode.
    #[inline(always)]
    fn npc_player_escape_mode(active: *mut ActiveNpc) {
        let active = unsafe { &mut *active };
        let Some(InteractionTarget::Player { pid }) = active.npc.interaction.target else {
            Self::npc_reset_defaults(active);
            return;
        };

        let Some(player) = engine().get_player(pid) else {
            Self::npc_reset_defaults(active);
            return;
        };
        let player_coord = player.player.pathing.coord;

        let dist = active.npc.pathing.coord.distance(player_coord);
        if dist > 25 {
            Self::npc_reset_defaults(active);
            return;
        }

        let coord = active.npc.pathing.coord;
        let dir: Direction = if player_coord.x() >= coord.x() && player_coord.z() >= coord.z() {
            Direction::SouthWest
        } else if player_coord.x() >= coord.x() && player_coord.z() < coord.z() {
            Direction::NorthWest
        } else if player_coord.x() < coord.x() && player_coord.z() >= coord.z() {
            Direction::SouthEast
        } else {
            Direction::NorthEast
        };

        let (dx, dz) = rs_entity::dir_delta(dir as i8);
        let level = coord.y();
        let mx = (coord.x() as i32 + dx as i32) as u16;
        let mz = (coord.z() as i32 + dz as i32) as u16;

        let Some(npc_type) = cache().npcs.get_by_id(active.npc.uid.id()) else {
            return;
        };
        let maxrange = npc_type.maxrange as i32;

        let mr = active.move_restrict();
        let collision = || PathingEntity::collision_type(mr).unwrap_or(CollisionType::Normal);
        let extra_flag = PathingEntity::block_walk_extra_flag(mr);
        let members = engine().members;
        let spawn = active.npc.spawn_coord;
        let size = active.npc.pathing.size;

        // Prefer the diagonal away from the player; if it is blocked or would
        // leave maxrange, fall back to the X axis, then the Z axis.
        let diagonal_ok = rs_entity::can_travel(
            members,
            level,
            coord.x(),
            coord.z(),
            dx,
            dz,
            size,
            extra_flag,
            collision(),
        ) && CoordGrid::new(mx, level, mz).distance(spawn) <= maxrange;

        if diagonal_ok {
            active.npc.pathing.queue_waypoint(mx, mz);
        } else {
            let primary_ok = rs_entity::can_travel(
                members,
                level,
                coord.x(),
                coord.z(),
                dx,
                0,
                size,
                extra_flag,
                collision(),
            ) && CoordGrid::new(mx, level, coord.z()).distance(spawn) <= maxrange;
            let secondary_ok = rs_entity::can_travel(
                members,
                level,
                coord.x(),
                coord.z(),
                0,
                dz,
                size,
                extra_flag,
                collision(),
            ) && CoordGrid::new(coord.x(), level, mz).distance(spawn)
                <= maxrange;

            if primary_ok {
                active.npc.pathing.queue_waypoint(mx, coord.z());
            } else if secondary_ok {
                active.npc.pathing.queue_waypoint(coord.x(), mz);
            }
        }

        if !Self::npc_process_movement(active) {
            active.npc.stuck_counter += 1;
        }

        // Give up retreating only when genuinely stuck, not merely cornered at
        // the edge of maxrange on both axes.
        let after = active.npc.pathing.coord;
        let dist_x = (after.x() as i32 - spawn.x() as i32).abs();
        let dist_z = (after.z() as i32 - spawn.z() as i32).abs();
        let at_max_range_both = dist_x >= maxrange && dist_z >= maxrange;

        if active.npc.stuck_counter >= 5 && !at_max_range_both {
            Self::npc_reset_defaults(active);
            active.npc.stuck_counter = 0;
        }
    }

    /// Handles the `PlayerFollow` NPC mode (path toward a player).
    ///
    /// Computes a path to the interaction target and processes one tick of
    /// movement. Does not attempt any interaction -- purely follows.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * Queues waypoints toward the target player.
    #[inline(always)]
    fn npc_player_follow_mode(active: *mut ActiveNpc) {
        let active = unsafe { &mut *active };
        if let Some(target) = &active.npc.interaction.target {
            Self::entity_path_to_target(&mut active.npc.pathing, target, false);
        };
        Self::npc_process_movement(active);
    }

    /// Handles the `PlayerFaceClose` NPC mode.
    ///
    /// Faces the target player only while adjacent (distance <= 1). If the
    /// player moves beyond distance 1, resets the NPC to its default mode.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    #[inline(always)]
    fn npc_player_face_close_mode(active: *mut ActiveNpc) {
        let active = unsafe { &mut *active };
        let Some(target) = active.npc.interaction.target else {
            Self::npc_reset_defaults(active);
            return;
        };

        let target_coord = Self::target_coord(&target);
        let dist = active.npc.pathing.coord.distance(target_coord);

        if dist > 1 {
            Self::npc_reset_defaults(active);
        }
    }

    /// Handles generic AI modes (Op/Ap interactions with any target type).
    ///
    /// Implements the NPC's interact-or-pursue loop:
    ///
    /// 1. Resets the wander counter.
    /// 2. Attempts to interact with the target before moving.
    /// 3. If interaction fails, paths toward the target and moves.
    /// 4. After movement, clears the interaction if the NPC type's
    ///    `givechase` is `false`.
    /// 5. Retries the interaction after movement.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * Executes RuneScript Op/Ap AI trigger scripts.
    /// * Moves the NPC along waypoints.
    /// * May reset the NPC to defaults if `givechase` is false.
    #[inline(always)]
    fn npc_ai_mode(active: *mut ActiveNpc) {
        let active = unsafe { &mut *active };
        // Reset the stuck timer if the npc runs its ai mode
        active.npc.stuck_counter = 0;

        // Try to interact before moving (allow Op on scenery)
        if Self::npc_try_interact(active, true) {
            return;
        }

        if let Some(target) = &active.npc.interaction.target {
            Self::entity_path_to_target(&mut active.npc.pathing, target, false);
        };

        // Move
        let moved = Self::npc_process_movement(active);

        // Clear target if givechase=no
        if moved {
            let givechase = cache()
                .npcs
                .get_by_id(active.npc.uid.id())
                .map(|t| t.givechase)
                .unwrap_or(true);
            if !givechase {
                Self::npc_reset_defaults(active);
                return;
            }
        }

        // Try to interact again after moving
        if active.npc.interaction.target.is_some() {
            Self::npc_try_interact(active, false);
        }
    }

    // ---- Interaction helpers

    /// Attempts to execute an NPC AI interaction with its current target.
    ///
    /// Checks whether the NPC's current mode is an Op (operate) or Ap
    /// (approach) trigger:
    ///
    /// * **Op:** If the NPC is in operable distance and the target is a
    ///   pathing entity or `allow_op_scenery` is set, looks up and executes
    ///   the corresponding `AiOp*` trigger script.
    /// * **Ap:** If the NPC is within its attack range, looks up and
    ///   executes the corresponding `AiAp*` trigger script.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Returns
    ///
    /// `true` if an interaction script was executed.
    #[inline(always)]
    fn npc_try_interact(active: *mut ActiveNpc, allow_op_scenery: bool) -> bool {
        let active = unsafe { &mut *active };
        if active.npc.interaction.target.is_none() {
            return false;
        }

        let uid = active.npc.uid;
        let Some(target_op) = active.npc.interaction.target_op else {
            return false;
        };

        let is_op = Self::npc_is_op_trigger(target_op);
        let is_ap = Self::npc_is_ap_trigger(target_op);

        if is_op {
            let in_op = Self::entity_in_operable_distance(
                &active.npc.pathing,
                active.npc.interaction.target.as_ref().unwrap(),
            );
            let is_pathing = active
                .npc
                .interaction
                .target
                .as_ref()
                .is_some_and(|t| t.is_pathing_entity());

            if in_op && (is_pathing || allow_op_scenery) {
                let target = active.npc.interaction.target;
                let target_subject = target.as_ref().and_then(Self::target_to_subject);
                let trigger = Self::npc_mode_to_trigger(target_op);

                if let Some(trigger) = trigger {
                    let type_id = active.npc.uid.id();
                    let category = cache()
                        .npcs
                        .get_by_id(type_id)
                        .and_then(|t| t.category)
                        .map(|c| c as i32);
                    if let Err(e) = engine_mut().run_script_by_trigger(
                        (trigger, Some(type_id), category),
                        Some(ScriptSubject::Npc(uid)),
                        target_subject,
                        None,
                        None,
                        None,
                    ) {
                        error!("error running npc op script for npc {}: {e}", uid.nid());
                    }
                }
                return true;
            }
        } else if is_ap {
            let type_id = active.npc.uid.id();
            let npc_type = cache().npcs.get_by_id(type_id);
            let attackrange = npc_type.map(|t| t.attackrange).unwrap_or(0);

            if Self::entity_in_approach_distance(
                &active.npc.pathing,
                active.npc.interaction.target.as_ref().unwrap(),
                attackrange as i32,
            ) {
                let target = active.npc.interaction.target;
                let target_subject = target.as_ref().and_then(Self::target_to_subject);
                let trigger = Self::npc_mode_to_trigger(target_op);

                if let Some(trigger) = trigger {
                    let category = npc_type.and_then(|t| t.category).map(|c| c as i32);
                    if let Err(e) = engine_mut().run_script_by_trigger(
                        (trigger, Some(type_id), category),
                        Some(ScriptSubject::Npc(uid)),
                        target_subject,
                        None,
                        None,
                        None,
                    ) {
                        error!("error running npc ap script for npc {}: {e}", uid.nid());
                    }
                }
                return true;
            }
        }

        false
    }

    /// Validates that the NPC's current interaction target still exists, is
    /// on the same level, and is within the NPC's maximum range from spawn.
    ///
    /// Delegates to [`entity_validate_target`] for existence / level checks,
    /// then applies the NPC-specific max-range constraint via
    /// [`npc_target_within_max_range`].
    ///
    /// # Returns
    ///
    /// `true` if the target is still valid.
    #[inline(always)]
    fn npc_validate_target(active: &ActiveNpc) -> bool {
        let Some(target) = &active.npc.interaction.target else {
            return false;
        };
        if !Self::entity_validate_target(
            active.npc.pathing.coord.y(),
            target,
            active.npc.interaction.target_subject_type,
            None,
            true,
        ) {
            return false;
        }
        Self::npc_target_within_max_range(active)
    }

    /// Checks whether the NPC's interaction target is within the NPC type's
    /// `maxrange` from the NPC's spawn coordinate.
    ///
    /// The range calculation varies by mode:
    ///
    /// * **Op modes:** target must be within `maxrange + 1` Chebyshev
    ///   distance of spawn.
    /// * **Ap modes:** target must be within `maxrange + attackrange`.
    /// * **PlayerEscape:** both the NPC and target must be within
    ///   `maxrange` of spawn.
    /// * **PlayerFollow:** always returns `true` (unlimited range).
    /// * **Other modes:** target within `maxrange + 1`.
    ///
    /// # Returns
    ///
    /// `true` if the target is within range.
    #[inline(always)]
    pub(crate) fn npc_target_within_max_range(active: &ActiveNpc) -> bool {
        let Some(target) = &active.npc.interaction.target else {
            return true;
        };

        let Some(target_op) = active.npc.interaction.target_op else {
            return true;
        };
        if target_op == NpcMode::PlayerFollow as u8 {
            return true;
        }

        let Some(npc_type) = cache().npcs.get_by_id(active.npc.uid.id()) else {
            return true;
        };

        let maxrange = npc_type.maxrange as i32;
        let target_coord = Self::target_coord(target);
        let spawn_x = active.npc.spawn_coord.x() as i32;
        let spawn_z = active.npc.spawn_coord.z() as i32;

        if Self::npc_is_op_trigger(target_op) {
            let dx = (target_coord.x() as i32 - spawn_x).abs();
            let dz = (target_coord.z() as i32 - spawn_z).abs();
            if dx.max(dz) > maxrange + 1 {
                return false;
            }
            if dx == maxrange + 1 && dz == maxrange + 1 {
                return false;
            }
        } else if Self::npc_is_ap_trigger(target_op) {
            let dist = (target_coord.x() as i32 - spawn_x)
                .abs()
                .max((target_coord.z() as i32 - spawn_z).abs());
            if dist > maxrange + npc_type.attackrange as i32 {
                return false;
            }
        } else if target_op == NpcMode::PlayerEscape as u8 {
            let npc_size = active.npc.pathing.size as i32;
            let npc_dist = CoordGrid::distance_to(
                active.npc.pathing.coord.x() as i32,
                active.npc.pathing.coord.z() as i32,
                npc_size,
                npc_size,
                spawn_x,
                spawn_z,
                npc_size,
                npc_size,
            );
            let (target_w, target_l) = match target {
                InteractionTarget::Npc { nid } => engine()
                    .get_npc(*nid)
                    .map(|n| (n.npc.pathing.size as i32, n.npc.pathing.size as i32))
                    .unwrap_or((1, 1)),
                InteractionTarget::Loc { width, length, .. } => (*width as i32, *length as i32),
                _ => (1, 1),
            };
            let target_dist = CoordGrid::distance_to(
                target_coord.x() as i32,
                target_coord.z() as i32,
                target_w,
                target_l,
                spawn_x,
                spawn_z,
                target_w,
                target_l,
            );
            if target_dist > maxrange && npc_dist > maxrange {
                return false;
            }
        } else {
            let dist = (target_coord.x() as i32 - spawn_x)
                .abs()
                .max((target_coord.z() as i32 - spawn_z).abs());
            if dist > maxrange + 1 {
                return false;
            }
        }

        true
    }

    // ---- Movement helpers

    /// Processes one tick of movement for an NPC along its queued waypoints.
    ///
    /// Delegates directly to [`PathingEntity::process_movement`] with the
    /// NPC's info and focus kind.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Returns
    ///
    /// `true` if the NPC moved at least one tile this tick.
    #[inline(always)]
    fn npc_process_movement(active: *mut ActiveNpc) -> bool {
        let active = unsafe { &mut *active };
        let members = engine().members;
        let moved =
            active
                .npc
                .pathing
                .process_movement(members, &mut active.npc.info, FocusKind::Npc);
        if moved {
            // Reset the stuck timer whenever the npc actually moves.
            active.npc.stuck_counter = 0;
        }
        moved
    }

    /// Resets an NPC to its default mode, hunt, and timer configuration
    /// as defined by its NPC type.
    ///
    /// Clears any active interaction and restores default AI parameters.
    /// Called when a mode's target becomes invalid or unreachable.
    ///
    /// # Safety Note
    ///
    /// Takes `*mut` to avoid noalias -- see NPC processing notes.
    ///
    /// # Side Effects
    ///
    /// * Clears the NPC's interaction target and resets mode, hunt, and
    ///   timer fields.
    #[inline(always)]
    fn npc_reset_defaults(active: *mut ActiveNpc) {
        let active = unsafe { &mut *active };
        let npc_type = cache().npcs.get_by_id(active.npc.uid.id());
        let default_mode = npc_type.map(|t| t.defaultmode).unwrap_or(NpcMode::None);
        let hunt_mode = npc_type.and_then(|t| t.huntmode);
        let hunt_range = npc_type.map(|t| t.huntrange).unwrap_or(0);
        let timer_interval = npc_type.and_then(|t| t.timer);
        active
            .npc
            .reset_defaults(default_mode, hunt_mode, hunt_range, timer_interval);
    }

    // ---- Trigger mapping

    /// Returns `true` if `target_op` corresponds to an operate (Op) NPC
    /// mode (any of OpNpc1-5, OpPlayer1-5, OpLoc1-5, OpObj1-5).
    #[inline(always)]
    fn npc_is_op_trigger(target_op: u8) -> bool {
        (7..=46).contains(&target_op) && ((target_op.wrapping_sub(7) / 5) & 1) == 0
    }

    /// Returns `true` if `target_op` corresponds to an approach (Ap) NPC
    /// mode (any of ApNpc1-5, ApPlayer1-5, ApLoc1-5, ApObj1-5).
    #[inline(always)]
    fn npc_is_ap_trigger(target_op: u8) -> bool {
        (7..=46).contains(&target_op) && ((target_op.wrapping_sub(7) / 5) & 1) == 1
    }

    /// Maps an NPC mode op code to its corresponding [`ServerTriggerType`].
    ///
    /// Translates Op/Ap mode variants (e.g. `OpPlayer1` -> `AiOpPlayer1`,
    /// `ApNpc3` -> `AiApNpc3`) for script lookup. Returns `None` for modes
    /// that have no associated trigger (e.g. `Wander`, `Patrol`).
    #[inline(always)]
    fn npc_mode_to_trigger(target_op: u8) -> Option<ServerTriggerType> {
        let mode = NpcMode::try_from(target_op).ok()?;
        match mode {
            NpcMode::OpPlayer1 => Some(ServerTriggerType::AiOpPlayer1),
            NpcMode::OpPlayer2 => Some(ServerTriggerType::AiOpPlayer2),
            NpcMode::OpPlayer3 => Some(ServerTriggerType::AiOpPlayer3),
            NpcMode::OpPlayer4 => Some(ServerTriggerType::AiOpPlayer4),
            NpcMode::OpPlayer5 => Some(ServerTriggerType::AiOpPlayer5),
            NpcMode::ApPlayer1 => Some(ServerTriggerType::AiApPlayer1),
            NpcMode::ApPlayer2 => Some(ServerTriggerType::AiApPlayer2),
            NpcMode::ApPlayer3 => Some(ServerTriggerType::AiApPlayer3),
            NpcMode::ApPlayer4 => Some(ServerTriggerType::AiApPlayer4),
            NpcMode::ApPlayer5 => Some(ServerTriggerType::AiApPlayer5),
            NpcMode::OpLoc1 => Some(ServerTriggerType::AiOpLoc1),
            NpcMode::OpLoc2 => Some(ServerTriggerType::AiOpLoc2),
            NpcMode::OpLoc3 => Some(ServerTriggerType::AiOpLoc3),
            NpcMode::OpLoc4 => Some(ServerTriggerType::AiOpLoc4),
            NpcMode::OpLoc5 => Some(ServerTriggerType::AiOpLoc5),
            NpcMode::ApLoc1 => Some(ServerTriggerType::AiApLoc1),
            NpcMode::ApLoc2 => Some(ServerTriggerType::AiApLoc2),
            NpcMode::ApLoc3 => Some(ServerTriggerType::AiApLoc3),
            NpcMode::ApLoc4 => Some(ServerTriggerType::AiApLoc4),
            NpcMode::ApLoc5 => Some(ServerTriggerType::AiApLoc5),
            NpcMode::OpObj1 => Some(ServerTriggerType::AiOpObj1),
            NpcMode::OpObj2 => Some(ServerTriggerType::AiOpObj2),
            NpcMode::OpObj3 => Some(ServerTriggerType::AiOpObj3),
            NpcMode::OpObj4 => Some(ServerTriggerType::AiOpObj4),
            NpcMode::OpObj5 => Some(ServerTriggerType::AiOpObj5),
            NpcMode::ApObj1 => Some(ServerTriggerType::AiApObj1),
            NpcMode::ApObj2 => Some(ServerTriggerType::AiApObj2),
            NpcMode::ApObj3 => Some(ServerTriggerType::AiApObj3),
            NpcMode::ApObj4 => Some(ServerTriggerType::AiApObj4),
            NpcMode::ApObj5 => Some(ServerTriggerType::AiApObj5),
            NpcMode::OpNpc1 => Some(ServerTriggerType::AiOpNpc1),
            NpcMode::OpNpc2 => Some(ServerTriggerType::AiOpNpc2),
            NpcMode::OpNpc3 => Some(ServerTriggerType::AiOpNpc3),
            NpcMode::OpNpc4 => Some(ServerTriggerType::AiOpNpc4),
            NpcMode::OpNpc5 => Some(ServerTriggerType::AiOpNpc5),
            NpcMode::ApNpc1 => Some(ServerTriggerType::AiApNpc1),
            NpcMode::ApNpc2 => Some(ServerTriggerType::AiApNpc2),
            NpcMode::ApNpc3 => Some(ServerTriggerType::AiApNpc3),
            NpcMode::ApNpc4 => Some(ServerTriggerType::AiApNpc4),
            NpcMode::ApNpc5 => Some(ServerTriggerType::AiApNpc5),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_op_reference(target_op: u8) -> bool {
        matches!(
            NpcMode::try_from(target_op),
            Ok(NpcMode::OpPlayer1
                | NpcMode::OpPlayer2
                | NpcMode::OpPlayer3
                | NpcMode::OpPlayer4
                | NpcMode::OpPlayer5
                | NpcMode::OpLoc1
                | NpcMode::OpLoc2
                | NpcMode::OpLoc3
                | NpcMode::OpLoc4
                | NpcMode::OpLoc5
                | NpcMode::OpObj1
                | NpcMode::OpObj2
                | NpcMode::OpObj3
                | NpcMode::OpObj4
                | NpcMode::OpObj5
                | NpcMode::OpNpc1
                | NpcMode::OpNpc2
                | NpcMode::OpNpc3
                | NpcMode::OpNpc4
                | NpcMode::OpNpc5)
        )
    }

    fn is_ap_reference(target_op: u8) -> bool {
        matches!(
            NpcMode::try_from(target_op),
            Ok(NpcMode::ApPlayer1
                | NpcMode::ApPlayer2
                | NpcMode::ApPlayer3
                | NpcMode::ApPlayer4
                | NpcMode::ApPlayer5
                | NpcMode::ApLoc1
                | NpcMode::ApLoc2
                | NpcMode::ApLoc3
                | NpcMode::ApLoc4
                | NpcMode::ApLoc5
                | NpcMode::ApObj1
                | NpcMode::ApObj2
                | NpcMode::ApObj3
                | NpcMode::ApObj4
                | NpcMode::ApObj5
                | NpcMode::ApNpc1
                | NpcMode::ApNpc2
                | NpcMode::ApNpc3
                | NpcMode::ApNpc4
                | NpcMode::ApNpc5)
        )
    }

    #[test]
    fn op_ap_predicate_arithmetic_matches_enum() {
        for target_op in 0..=u8::MAX {
            assert_eq!(
                Engine::npc_is_op_trigger(target_op),
                is_op_reference(target_op),
                "npc_is_op_trigger mismatch at {target_op}"
            );
            assert_eq!(
                Engine::npc_is_ap_trigger(target_op),
                is_ap_reference(target_op),
                "npc_is_ap_trigger mismatch at {target_op}"
            );
            // Op and Ap are mutually exclusive.
            assert!(
                !(Engine::npc_is_op_trigger(target_op) && Engine::npc_is_ap_trigger(target_op)),
                "op and ap both true at {target_op}"
            );
        }
    }
}
