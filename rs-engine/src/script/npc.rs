use crate::active_npc::ActiveNpc;
use crate::core::{Engine, engine};
use rs_entity::NpcUid;
use rs_entity::{InteractionTarget, ObjSerial};
use rs_grid::CoordGrid;
use rs_pack::cache::VarValue;
use rs_pack::types::{NpcMode, NpcStat};
use rs_protocol::network::game::info_prot::NpcInfoProt;
use rs_vm::engine::{ScriptEngine, ScriptNpc};
use rs_vm::state::*;

impl ScriptNpc for ActiveNpc {
    /// Returns this NPC's unique identifier.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** reads `self.npc.uid`
    fn uid(&self) -> NpcUid {
        self.npc.uid
    }

    /// Returns the NPC's current coordinate.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    fn coord(&self) -> CoordGrid {
        self.npc.pathing.coord
    }

    /// Returns the NPC's collision size in tiles.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** reads `self.npc.pathing.size`
    fn size(&self) -> u8 {
        self.npc.pathing.size
    }

    /// Reads an NPC variable (varn) by its definition ID.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `VarSet::get`
    fn get_var(&self, id: u16) -> VarValue {
        self.npc.vars.get(id).clone()
    }

    /// Writes an NPC variable (varn).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `VarSet::set`
    fn set_var(&mut self, id: u16, value: VarValue) {
        self.npc.vars.set(id, value);
    }

    /// Returns the opcode of the NPC's current interaction target, if any.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** reads `self.npc.interaction.target_op`
    fn target_op(&self) -> Option<u8> {
        self.npc.interaction.target_op
    }

    /// Sets the NPC's AI mode.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** sets `self.npc.interaction.target_op`
    fn set_mode(&mut self, mode: Option<u8>) {
        self.npc.interaction.target_op = mode;
    }

    /// Clears the NPC's current interaction target.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Npc::clear_interaction`
    fn clear_interaction(&mut self) {
        self.npc.clear_interaction();
    }

    /// Resets the NPC's patrol progress.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait (`NPC_SETMODE`)
    /// **Calls:** `Npc::clear_patrol`
    fn clear_patrol(&mut self) {
        self.npc.clear_patrol();
    }

    /// Resets the NPC to its default spawn state.
    ///
    /// Reads default mode, hunt mode, hunt range, and timer interval from the NPC type
    /// definition in the cache and applies them. Also clears the face entity.
    ///
    /// # Side Effects
    ///
    /// Resets interaction state, mode, hunt parameters, and timer.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `cache().npcs.get_by_id`, `NpcInfo::clear_face_entity_npc`, `Npc::reset_defaults`
    fn reset_defaults(&mut self) {
        let npc_type = engine().npcs().get_by_id(self.npc.uid.id());
        let default_mode = npc_type.map(|t| t.defaultmode).unwrap_or(NpcMode::None);
        let hunt_mode = npc_type.and_then(|t| t.huntmode);
        let hunt_range = npc_type.map(|t| t.huntrange).unwrap_or(0);
        let timer_interval = npc_type.and_then(|t| t.timer);
        self.npc.info.clear_face_entity_npc();
        self.npc
            .reset_defaults(default_mode, hunt_mode, hunt_range, timer_interval);
    }

    /// Sets the NPC's interaction target to another NPC.
    ///
    /// Also records the target NPC's current type for trigger resolution.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `engine().get_npc`, `Npc::set_interaction`
    fn set_interaction_npc(&mut self, nid: u16, op: u8) {
        if let Some(target_npc) = engine().get_npc(nid) {
            self.npc.interaction.target_subject_type = Some(target_npc.npc.uid.id());
        }
        self.npc
            .set_interaction(InteractionTarget::Npc { nid }, op, false);
    }

    /// Sets the NPC's interaction target to a player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Npc::set_interaction`
    fn set_interaction_player(&mut self, pid: u16, op: u8) {
        self.npc
            .set_interaction(InteractionTarget::Player { pid }, op, false);
    }

    /// Sets the NPC's interaction target to a location.
    ///
    /// Builds an `InteractionTarget::Loc` from `loc` and the given `width`
    /// and `length`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Npc::set_interaction`
    fn set_interaction_loc(&mut self, loc: LocRef, width: u8, length: u8, op: u8) {
        self.npc.set_interaction(
            InteractionTarget::Loc {
                coord: loc.coord,
                id: loc.id,
                width,
                length,
                shape: loc.shape,
                angle: loc.angle,
                layer: loc.layer,
                serial: loc.serial,
            },
            op,
            false,
        );
    }

    /// Sets the NPC's interaction target to a ground object.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Npc::set_interaction`
    fn set_interaction_obj(&mut self, coord: CoordGrid, id: u16, serial: ObjSerial, op: u8) {
        let target = InteractionTarget::Obj { coord, id, serial };
        self.npc.set_interaction(target, op, false);
    }

    /// Plays an animation on the NPC's model.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `ActiveNpc::anim`
    fn anim(&mut self, id: Option<u16>, delay: u8) {
        self.anim(id, delay);
    }

    /// Displays overhead chat text above the NPC.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `ActiveNpc::say`
    fn say(&mut self, msg: &str) {
        self.say(msg);
    }

    /// Returns the NPC's current level in the given stat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Stats::level`
    fn stat(&self, stat: usize) -> u16 {
        self.npc.stats.level(stat)
    }

    /// Returns the NPC's base (unmodified) level in the given stat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Stats::base_level`
    fn basestat(&self, stat: usize) -> u16 {
        self.npc.stats.base_level(stat)
    }

    /// Applies damage to the NPC and displays a hitsplat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `ActiveNpc::damage`
    fn damage(&mut self, amount: u16, damage_type: u8) {
        self.damage(amount, damage_type);
    }

    /// Awards hero points to a player for contributing damage to this NPC.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `HeroPoints::add_hero`
    fn heropoints(&mut self, user37: u64, points: i32) {
        self.npc.hero_points.add_hero(user37, points);
    }

    /// Finds the player with the most hero points on this NPC.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `HeroPoints::find_hero`
    fn findhero(&self) -> Option<u64> {
        self.npc.hero_points.find_hero()
    }

    /// Enqueues a script for deferred execution on this NPC.
    ///
    /// Uses `QueuePriority::Normal` for all NPC queues.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `ScriptQueue::add`
    fn queue(
        &mut self,
        script_id: i32,
        delay: u16,
        args: Option<Vec<ScriptArgument>>,
    ) -> rs_vm::Result<()> {
        self.npc
            .state
            .queues
            .add(QueuePriority::Normal, script_id, delay, args)
    }

    #[cfg(since_647)]
    fn queue_from(
        &mut self,
        script_id: i32,
        delay: u16,
        args: Option<Vec<ScriptArgument>>,
        source: Option<u16>,
    ) -> rs_vm::Result<()> {
        self.npc
            .state
            .queues
            .add_from(QueuePriority::Normal, script_id, delay, args, source)
    }

    fn queue_named(
        &mut self,
        script_id: i32,
        delay: u16,
        args: Vec<ScriptArgument>,
        source: Option<u16>,
    ) {
        self.npc
            .state
            .queues
            .add_named(script_id, delay, args, source);
    }

    #[cfg(since_647)]
    fn hit_source(&mut self, source: Option<u16>) {
        self.npc.info.set_last_hit_source(source);
    }

    /// Sets or clears the NPC's recurring timer interval.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `ActiveNpc::set_timer`
    fn settimer(&mut self, interval: Option<u16>) {
        self.set_timer(interval);
    }

    /// Returns the game tick of the NPC's last movement.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** reads `self.npc.pathing.last_movement`
    fn last_movement(&self) -> u32 {
        self.npc.pathing.last_movement
    }

    /// Suspends the NPC's currently running script for a number of ticks.
    ///
    /// # Side Effects
    ///
    /// Sets the NPC's `delayed` flag and `delayed_until` tick.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** sets `self.npc.state.delayed` and `delayed_until`
    fn delay(&mut self, delay: u32) {
        self.npc.state.delayed = true;
        self.npc.state.delayed_until = delay;
    }

    /// Teleports the NPC to a new coordinate.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `ActiveNpc::tele`
    fn tele(&mut self, coord: CoordGrid) {
        self.tele(coord);
    }

    /// Makes the NPC face a specific tile.
    ///
    /// Converts tile coordinates to fine coordinates for smooth facing.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `NpcInfo::focus_npc`, `CoordGrid::fine`
    fn facesquare(&mut self, coord: CoordGrid) {
        self.npc.info.focus_npc(
            CoordGrid::fine(coord.x(), 1),
            CoordGrid::fine(coord.z(), 1),
            true,
        );
    }

    /// Queues a waypoint for the NPC to walk toward.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Pathing::queue_waypoint`
    fn walk(&mut self, coord: CoordGrid) {
        self.npc.pathing.queue_waypoint(coord);
    }

    /// Sets the maximum hunt range for this NPC's AI.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** sets `self.npc.hunt_range`
    fn set_hunt_range(&mut self, range: u8) {
        self.npc.hunt_range = range;
    }

    /// Sets or clears the NPC's hunt mode.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** sets `self.npc.hunt_mode`
    fn set_hunt_mode(&mut self, mode: Option<u16>) {
        self.npc.hunt_mode = mode;
    }

    /// Temporarily transforms the NPC to a different type.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `ActiveNpc::change_type`
    fn change_type(&mut self, new_type: u16, duration: u64, reset: bool) {
        self.change_type(new_type, duration, reset);
    }

    /// Returns whether the NPC's current target is within its max range
    /// from its spawn point.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Engine::npc_target_within_max_range`
    fn inrange(&self) -> bool {
        Engine::npc_target_within_max_range(self)
    }

    /// Adds to the NPC's current stat level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Stats::add`
    fn statadd(&mut self, stat: usize, constant: i32, percent: i32) {
        self.npc.stats.add(stat, constant, percent);
    }

    /// Subtracts from the NPC's current stat level, floored at 0.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Stats::sub`
    fn statsub(&mut self, stat: usize, constant: i32, percent: i32) {
        self.npc.stats.sub(stat, constant, percent);
    }

    /// Heals the NPC's current stat level, capped at the base level.
    ///
    /// Clears hero points if Hitpoints is restored to or above the base level,
    /// so an NPC healed back to full HP loses its stale damage attribution.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    /// **Calls:** `Stats::heal`
    fn statheal(&mut self, stat: usize, constant: i32, percent: i32) {
        self.npc.stats.heal(stat, constant, percent);
        if stat == NpcStat::Hitpoints as usize
            && self.npc.stats.level(NpcStat::Hitpoints as usize)
                >= self.npc.stats.base_level(NpcStat::Hitpoints as usize)
        {
            self.npc.hero_points.clear();
        }
    }

    /// Sets the NPC's walk trigger script and argument.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    fn walktrigger(&mut self, trigger: i32, arg: i32) {
        self.npc.walktrigger = Some(trigger);
        self.npc.walktrigger_arg = arg;
    }

    /// Plays a spot animation (graphic) on the NPC.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    fn spotanim(&mut self, id: u16, height: u16, delay: u16) {
        self.npc.info.spotanim = Some(id);
        self.npc.info.spotanim_height = Some(height);
        self.npc.info.spotanim_delay = Some(delay);
        self.npc.info.masks |= NpcInfoProt::SpotAnim as u16;
    }

    /// Returns the NPC's current coord destination.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptNpc` trait
    fn destination(&self) -> CoordGrid {
        if !self.npc.pathing.has_waypoints() {
            return self.coord();
        }
        self.npc.pathing.waypoints[0]
    }
}
