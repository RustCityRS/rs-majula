use rs_entity::{EntityLifeTime, Npc, NpcUid};
use rs_grid::CoordGrid;
use rs_info::FocusKind;
use rs_pack::cache::CacheStore;
use rs_pack::types::{BlockWalk, MoveRestrict, NpcStat};
use rs_protocol::network::game::info_prot::NpcInfoProt;
use rs_var::VarSet;
use rs_vm::engine::cache;

/// Wraps an [`Npc`] entity with engine-level behavior such as animation,
/// combat, type morphing, and timer management.
///
/// `ActiveNpc` is the primary handle through which the game engine interacts
/// with a live NPC instance on the world map.
pub struct ActiveNpc {
    pub npc: Npc,
}

impl ActiveNpc {
    /// Creates a new active NPC from its type definition, initializing combat
    /// stats, hunt behavior, and movement restrictions from the cache.
    ///
    /// # Arguments
    /// * `id` - The NPC type ID used to look up stats and config in the cache.
    /// * `nid` - The unique NPC index (slot) within the world NPC array.
    /// * `coord` - The spawn coordinate on the world grid.
    /// * `size` - The NPC's tile footprint size.
    /// * `vars` - The variable set for this NPC instance (script variables).
    /// * `store` - Reference to the game cache for NPC type lookups.
    ///
    /// # Returns
    /// A fully initialized `ActiveNpc` with stats, hunt config, and timer set
    /// from the NPC type definition.
    ///
    /// # Side Effects
    /// * Sets the NPC's timer if the type definition specifies a timer interval.
    ///
    /// # Call Stack
    /// **Calls:** [`Npc::new`], [`ActiveNpc::set_timer`]
    pub fn new(
        id: u16,
        nid: u16,
        coord: CoordGrid,
        size: u8,
        vars: VarSet,
        store: &CacheStore,
    ) -> Self {
        let mut npc = Npc::new(id, nid, coord, size, vars);

        if let Some(npc_type) = store.npcs.get_by_id(id) {
            npc.stats.levels[NpcStat::Attack as usize] = npc_type.attack as u8;
            npc.stats.levels[NpcStat::Defence as usize] = npc_type.defence as u8;
            npc.stats.levels[NpcStat::Strength as usize] = npc_type.strength as u8;
            npc.stats.levels[NpcStat::Hitpoints as usize] = npc_type.hitpoints as u8;
            npc.stats.levels[NpcStat::Ranged as usize] = npc_type.ranged as u8;
            npc.stats.levels[NpcStat::Magic as usize] = npc_type.magic as u8;
            npc.stats.base_levels = npc.stats.levels;
            npc.interaction.target_op = Some(npc_type.defaultmode as u8);
            npc.hunt_mode = npc_type.huntmode;
            npc.hunt_range = npc_type.huntrange;
            npc.pathing.move_restrict = npc_type.moverestrict;
        }

        let mut active = Self { npc };
        if let Some(npc_type) = store.npcs.get_by_id(id) {
            active.set_timer(npc_type.timer);
        }
        active
    }

    /// Plays an animation (sequence) on this NPC, respecting priority.
    ///
    /// If the NPC already has an animation playing, the new animation only
    /// replaces it when its sequence priority is equal to or higher than the
    /// current one.
    ///
    /// # Arguments
    /// * `id` - The sequence (anim) ID to play, or `None` to clear.
    /// * `delay` - The tick delay before the animation begins.
    ///
    /// # Side Effects
    /// * Updates `npc.info` animation fields and sets the `NpcInfoProt::Anim` mask.
    pub fn anim(&mut self, id: Option<u16>, delay: u8) {
        let cur_pri = self
            .npc
            .info
            .anim_id
            .and_then(|a| cache().seqs.get_by_id(a))
            .map(|s| s.priority as u16);
        let new_pri = id
            .and_then(|a| cache().seqs.get_by_id(a))
            .map(|s| s.priority as u16);
        self.npc
            .info
            .set_anim(id, delay, NpcInfoProt::Anim as u16, cur_pri, new_pri);
    }

    /// Makes this NPC display an overhead chat message.
    ///
    /// # Arguments
    /// * `msg` - The text to display above the NPC.
    ///
    /// # Side Effects
    /// * Sets `npc.info.say` and enables the `NpcInfoProt::Say` mask for the
    ///   next info update cycle.
    pub fn say(&mut self, msg: &str) {
        self.npc.info.say = Some(msg.into());
        self.npc.info.masks |= NpcInfoProt::Say as u16;
    }

    /// Applies damage to this NPC, clamping hitpoints to zero if the damage
    /// exceeds the current value.
    ///
    /// # Arguments
    /// * `amount` - The amount of damage to deal.
    /// * `damage_type` - The damage type identifier (e.g. melee, ranged, magic).
    ///
    /// # Side Effects
    /// * Reduces `npc.levels[Hitpoints]` by `amount` (saturating at 0).
    /// * Populates the damage info fields (`damage_taken`, `damage_type`,
    ///   `damage_current`, `damage_base`) and sets the `NpcInfoProt::Damage`
    ///   mask for the next info update.
    pub fn damage(&mut self, amount: u8, damage_type: u8) {
        let current = self.npc.stats.levels[NpcStat::Hitpoints as usize];
        let taken = if current.saturating_sub(amount) == 0 {
            self.npc.stats.levels[NpcStat::Hitpoints as usize] = 0;
            current
        } else {
            self.npc.stats.levels[NpcStat::Hitpoints as usize] = current.saturating_sub(amount);
            amount
        };
        let remaining = self.npc.stats.levels[NpcStat::Hitpoints as usize];
        let base = self.npc.stats.base_levels[NpcStat::Hitpoints as usize];

        #[cfg(since_244)]
        if self.npc.info.apply_damage2(
            taken,
            damage_type,
            remaining,
            base,
            NpcInfoProt::Damage2 as u16,
        ) {
            return;
        }

        self.npc.info.apply_damage(
            taken,
            damage_type,
            remaining,
            base,
            NpcInfoProt::Damage as u16,
        );
    }

    /// Configures a recurring timer on this NPC.
    ///
    /// # Arguments
    /// * `interval` - The timer interval in ticks, or `None` to leave unchanged.
    ///
    /// # Side Effects
    /// * Sets `npc.timer_interval` and resets the timer clock to 0.
    ///
    /// # Call Stack
    /// **Called by:** [`ActiveNpc::new`]
    pub fn set_timer(&mut self, interval: Option<u16>) {
        if let Some(interval) = interval {
            self.npc.timer_interval = Some(interval);
            self.npc.timer_clock = 0;
        }
    }

    /// Returns the movement restriction mode for this NPC's current type.
    ///
    /// # Returns
    /// The [`MoveRestrict`] from the NPC type definition, or
    /// [`MoveRestrict::Normal`] if the type is not found in the cache.
    pub fn move_restrict(&self) -> MoveRestrict {
        cache()
            .npcs
            .get_by_id(self.npc.uid.id())
            .map(|t| t.moverestrict)
            .unwrap_or(MoveRestrict::Normal)
    }

    /// Returns the block-walk collision mode for this NPC's current type.
    ///
    /// # Returns
    /// The [`BlockWalk`] from the NPC type definition, or
    /// [`BlockWalk::Npc`] if the type is not found in the cache.
    pub fn block_walk(&self) -> BlockWalk {
        cache()
            .npcs
            .get_by_id(self.npc.uid.id())
            .map(|t| t.blockwalk)
            .unwrap_or(BlockWalk::Npc)
    }

    /// Temporarily morphs this NPC into a different type for a given duration.
    ///
    /// When `reset` is true, combat stat levels are recalculated relative to
    /// the new type (preserving any level deltas from buffs/debuffs). The NPC
    /// will automatically revert to its base type after `duration` ticks unless
    /// it is already being set back to its base type with a `Respawn` lifecycle.
    ///
    /// # Arguments
    /// * `new_type` - The NPC type ID to morph into.
    /// * `duration` - How many ticks the morph lasts. If less than 1, the call
    ///   is a no-op.
    /// * `reset` - Whether to recalculate combat stats from the new type.
    ///
    /// # Side Effects
    /// * Updates `npc.current_type`, `npc.uid`, and sets the
    ///   `NpcInfoProt::ChangeType` mask.
    /// * When `reset` is true, adjusts `npc.levels` and `npc.base_levels` to
    ///   match the new type's stats.
    /// * Schedules a revert `duration` ticks from now (a relative countdown
    ///   stored in `npc.revert_at`, decremented only while the NPC is processed).
    pub fn change_type(&mut self, new_type: u16, duration: u64, reset: bool) {
        if duration < 1 {
            return;
        }

        self.npc.uid = NpcUid::new(new_type, self.npc.uid.nid());
        self.npc.info.changetype = Some(new_type);
        self.npc.info.masks |= NpcInfoProt::ChangeType as u16;

        if reset && let Some(npc_type) = cache().npcs.get_by_id(new_type) {
            let stats = [
                npc_type.attack as u8,
                npc_type.defence as u8,
                npc_type.strength as u8,
                npc_type.hitpoints as u8,
                npc_type.ranged as u8,
                npc_type.magic as u8,
            ];
            for (i, &base) in stats.iter().enumerate() {
                let delta = self.npc.stats.levels[i] as i16 - self.npc.stats.base_levels[i] as i16;
                self.npc.stats.levels[i] = (base as i16 + delta).max(0) as u8;
                self.npc.stats.base_levels[i] = base;
            }
        }

        if new_type == self.npc.base_type && self.npc.lifecycle == EntityLifeTime::Respawn {
            self.npc.revert_at = None;
        } else {
            self.npc.revert_at = Some(duration as u32);
            self.npc.revert_reset = reset;
        }
    }

    /// Reverts this NPC back to its original base type after a temporary morph.
    ///
    /// If `revert_reset` was set during [`change_type`](ActiveNpc::change_type),
    /// all combat stats and levels are fully restored from the base type
    /// definition and the hero points map is cleared.
    ///
    /// # Side Effects
    /// * Resets `npc.current_type` and `npc.uid` to the base type.
    /// * Sets the `NpcInfoProt::ChangeType` mask.
    /// * Clears `npc.revert_at`.
    /// * When `revert_reset` is true, restores all stat levels and clears hero
    ///   points.
    pub fn revert_type(&mut self) {
        self.npc.revert_at = None;

        self.npc.uid = NpcUid::new(self.npc.base_type, self.npc.uid.nid());
        self.npc.info.changetype = Some(self.npc.base_type);
        self.npc.info.masks |= NpcInfoProt::ChangeType as u16;

        if self.npc.revert_reset {
            if let Some(npc_type) = cache().npcs.get_by_id(self.npc.base_type) {
                self.npc.stats.base_levels[NpcStat::Attack as usize] = npc_type.attack as u8;
                self.npc.stats.base_levels[NpcStat::Defence as usize] = npc_type.defence as u8;
                self.npc.stats.base_levels[NpcStat::Strength as usize] = npc_type.strength as u8;
                self.npc.stats.base_levels[NpcStat::Hitpoints as usize] = npc_type.hitpoints as u8;
                self.npc.stats.base_levels[NpcStat::Ranged as usize] = npc_type.ranged as u8;
                self.npc.stats.base_levels[NpcStat::Magic as usize] = npc_type.magic as u8;
                self.npc.stats.reset();
            }
            self.npc.hero_points.clear();
        }
    }

    /// Teleports this NPC to the given coordinate.
    ///
    /// The teleport is silently ignored if the target zone is not allocated
    /// in the collision map. If the destination is on a
    /// different level, a jump flag is set.
    ///
    /// # Arguments
    /// * `coord` - The destination coordinate.
    ///
    /// # Side Effects
    /// * Sets `npc.pathing.coord` and marks `npc.pathing.tele = true` so the
    ///   info encoder transmits the NPC as removed then re-added.
    /// * Sets `npc.jump = true` when changing levels.
    pub fn tele(&mut self, coord: CoordGrid) {
        match self.npc.pathing.teleport(coord) {
            None => {}
            Some((look_x, look_z)) => {
                self.npc.info.focus(
                    FocusKind::Npc,
                    CoordGrid::fine(look_x, self.npc.pathing.size),
                    CoordGrid::fine(look_z, self.npc.pathing.size),
                    false,
                );
            }
        }
    }
}
