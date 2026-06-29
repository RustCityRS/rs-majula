use crate::PlayerUid;
use crate::state::{LocRef, NpcRef, ObjRef, QueuePriority, ScriptArgument, TimerPriority};
use rs_grid::CoordGrid;
use rs_inv::Inventory;
use rs_pack::cache::script::Script;
use rs_pack::cache::{CacheStore, VarValue};
use rs_util::random::JavaRandom;
use std::cell::Cell;
use std::sync::Arc;

/// Engine-level operations available to the script VM.
///
/// `ScriptEngine` is the primary interface between the bytecode interpreter and
/// the game world. It exposes clock information, cache access, entity lookups,
/// zone queries, object/loc/NPC mutation, RNG, and membership status.
///
/// Implementors represent the game engine itself and are stored in thread-local
/// storage during script execution via [`with_engine`].
pub trait ScriptEngine {
    /// Returns the current game clock tick.
    ///
    /// # Returns
    /// The monotonically increasing tick counter (`u32`).
    fn clock(&self) -> u32;

    /// Returns the experience multiplier of the engine.
    ///
    /// # Returns
    /// The experience multiplier as defined in the environment args on the engine.
    fn multi_experience(&self) -> u8;

    /// Returns a reference to the global cache store.
    ///
    /// # Returns
    /// A shared reference to the [`CacheStore`] containing all loaded game definitions.
    fn cache(&self) -> &CacheStore;

    /// Looks up a compiled script by its numeric identifier.
    ///
    /// # Arguments
    /// * `id` - The script identifier to look up.
    ///
    /// # Returns
    /// `Some(&Arc<Script>)` if the script exists, `None` otherwise.
    fn get_script(&self, id: i32) -> Option<&Arc<Script>>;

    /// Returns the number of players currently in the world.
    ///
    /// # Returns
    /// The total online player count.
    fn playercount(&self) -> usize;

    /// Retrieves a shared (global) inventory, creating it if it does not exist.
    ///
    /// # Arguments
    /// * `id` - The shared inventory identifier.
    /// * `size` - The number of slots to allocate if the inventory is created.
    /// * `stack_mode` - The stacking behavior to use if the inventory is created.
    ///
    /// # Returns
    /// A mutable reference to the shared inventory.
    fn get_shared_inv(
        &mut self,
        id: u16,
        size: usize,
        stack_mode: rs_inv::StackMode,
    ) -> &mut Inventory;

    /// Retrieves an existing shared (global) inventory without creating one.
    ///
    /// # Arguments
    /// * `id` - The shared inventory identifier.
    ///
    /// # Returns
    /// `Some(&mut Inventory)` if the inventory exists, `None` otherwise.
    fn get_shared_inv_mut(&mut self, id: u16) -> Option<&mut Inventory>;

    /// Looks up a player by their slot index (immutable).
    ///
    /// # Arguments
    /// * `pid` - The player slot index.
    ///
    /// # Returns
    /// `Some` with an immutable reference to the player, or `None` if the slot is empty.
    fn get_player(&self, pid: u16) -> Option<&impl ScriptPlayer>
    where
        Self: Sized;

    /// Looks up a player by their slot index (mutable).
    ///
    /// # Arguments
    /// * `pid` - The player slot index.
    ///
    /// # Returns
    /// `Some` with a mutable reference to the player, or `None` if the slot is empty.
    fn get_player_mut(&mut self, pid: u16) -> Option<&mut impl ScriptPlayer>
    where
        Self: Sized;

    /// Looks up an NPC by its slot index (immutable).
    ///
    /// # Arguments
    /// * `nid` - The NPC slot index.
    ///
    /// # Returns
    /// `Some` with an immutable reference to the NPC, or `None` if the slot is empty.
    fn get_npc(&self, nid: u16) -> Option<&impl ScriptNpc>
    where
        Self: Sized;

    /// Looks up an NPC by its slot index (mutable).
    ///
    /// # Arguments
    /// * `nid` - The NPC slot index.
    ///
    /// # Returns
    /// `Some` with a mutable reference to the NPC, or `None` if the slot is empty.
    fn get_npc_mut(&mut self, nid: u16) -> Option<&mut impl ScriptNpc>
    where
        Self: Sized;

    /// Finds a player by their Base37-encoded username.
    ///
    /// # Arguments
    /// * `user37` - The Base37-encoded username to search for.
    ///
    /// # Returns
    /// `Some(PlayerUid)` if a matching player is online, `None` otherwise.
    fn find_player_by_user37(&self, user37: u64) -> Option<PlayerUid>
    where
        Self: Sized;

    /// Returns all NPCs present in the specified zone.
    ///
    /// # Arguments
    /// * `x` - The zone X coordinate.
    /// * `y` - The zone level (height plane).
    /// * `z` - The zone Z coordinate.
    ///
    /// # Returns
    /// A `Vec<NpcRef>` of NPC references in the zone.
    fn get_zone_npcs(&self, x: u16, y: u8, z: u16) -> Vec<NpcRef>;

    /// Returns the packed coordinates of all players in the specified zone.
    ///
    /// # Arguments
    /// * `x` - The zone X coordinate.
    /// * `y` - The zone level (height plane).
    /// * `z` - The zone Z coordinate.
    ///
    /// # Returns
    /// A `Vec<u32>` of packed `CoordGrid` values for each player in the zone.
    fn get_zone_player_coords(&self, x: u16, y: u8, z: u16) -> Vec<u32>;

    /// Returns the player slot indices for all players in the specified zone.
    ///
    /// # Arguments
    /// * `x` - The zone X coordinate.
    /// * `y` - The zone level (height plane).
    /// * `z` - The zone Z coordinate.
    ///
    /// # Returns
    /// A slice of player slot indices (`&[u16]`).
    fn get_zone_player_pids(&self, x: u16, y: u8, z: u16) -> &[u16];

    /// Spawns a new NPC at the given coordinate with a limited lifetime.
    ///
    /// # Arguments
    /// * `coord` - The packed coordinate where the NPC should appear.
    /// * `id` - The NPC type identifier.
    /// * `duration` - The tick at which the NPC should be removed.
    ///
    /// # Returns
    /// `Some(NpcUid)` with the unique identifier of the spawned NPC, or `None`
    /// if no free slot is available.
    fn add_npc_spawned(&mut self, coord: u32, id: u16, duration: u64) -> Option<crate::NpcUid>;

    /// Removes an NPC from the world by its slot index.
    ///
    /// # Arguments
    /// * `nid` - The NPC slot index to remove.
    fn remove_npc(&mut self, nid: u16);

    /// Adds a ground object at the given coordinate.
    ///
    /// # Arguments
    /// * `coord` - The packed coordinate where the object should appear.
    /// * `id` - The object type identifier.
    /// * `count` - The stack count for the object.
    /// * `receiver37` - Optional Base37-encoded username of the only player
    ///   who may see the object. `None` makes it visible to everyone.
    /// * `duration` - The tick at which the object should be removed.
    fn add_obj(&mut self, coord: u32, id: u16, count: u32, receiver37: Option<u64>, duration: u64);

    /// Adds a ground object that becomes visible after a delay.
    ///
    /// # Arguments
    /// * `coord` - The packed coordinate where the object should appear.
    /// * `id` - The object type identifier.
    /// * `count` - The stack count for the object.
    /// * `receiver37` - Optional Base37-encoded username restricting visibility.
    /// * `duration` - The tick at which the object should be removed.
    /// * `delay` - The number of ticks to wait before the object becomes visible.
    fn add_obj_delayed(
        &mut self,
        coord: u32,
        id: u16,
        count: u32,
        receiver37: Option<u64>,
        duration: u64,
        delay: u64,
    );

    /// Removes a ground object from the given coordinate.
    ///
    /// # Arguments
    /// * `coord` - The packed coordinate of the object.
    /// * `id` - The object type identifier.
    /// * `receiver37` - Optional Base37-encoded username that originally owned
    ///   the object.
    /// * `duration` - The remaining duration used to identify the specific object
    ///   instance to remove.
    fn remove_obj(&mut self, coord: u32, id: u16, receiver37: Option<u64>, duration: u64);

    /// Finds a ground object at the given coordinate by type ID.
    ///
    /// If `receiver37` is `Some`, matches objects visible to that player
    /// (receiver-owned or globally visible). If `None`, matches only
    /// globally visible objects.
    fn find_obj(&self, coord: u32, id: u16, receiver37: Option<u64>) -> Option<ObjRef>;

    /// Returns all ground objects present in the specified zone.
    fn get_zone_objs(&self, x: u16, y: u8, z: u16) -> Vec<ObjRef>;

    /// Returns all locations (locs) present in the specified zone.
    ///
    /// # Arguments
    /// * `x` - The zone X coordinate.
    /// * `y` - The zone level (height plane).
    /// * `z` - The zone Z coordinate.
    ///
    /// # Returns
    /// A `Vec<LocRef>` of location references in the zone.
    fn get_zone_locs(&self, x: u16, y: u8, z: u16) -> Vec<LocRef>;

    /// Finds a specific location at the given tile.
    ///
    /// # Arguments
    /// * `x` - The tile X coordinate.
    /// * `z` - The tile Z coordinate.
    /// * `y` - The level (height plane).
    /// * `id` - The location type identifier.
    ///
    /// # Returns
    /// `Some(LocRef)` if a matching location exists, `None` otherwise.
    fn find_loc(&self, x: u16, z: u16, y: u8, id: u16) -> Option<LocRef>;

    /// Adds a new location or changes an existing one at the given coordinate.
    ///
    /// # Arguments
    /// * `coord` - The packed coordinate.
    /// * `id` - The location type identifier.
    /// * `shape` - The location shape (e.g. wall, centrepiece, ground decor).
    /// * `angle` - The rotation angle (0--3).
    /// * `duration` - How many ticks until the change reverts (0 = permanent).
    /// * `create_if_missing` - When `true` (LOC_ADD), a loc is created if none
    ///   exists on the layer. When `false` (LOC_CHANGE), a missing loc is left
    ///   untouched so a deleted dynamic loc is never resurrected.
    fn add_or_change_loc(
        &mut self,
        coord: u32,
        id: u16,
        shape: u8,
        angle: u8,
        duration: u64,
        create_if_missing: bool,
    );

    /// Merges a location so that it is only visible to one player.
    ///
    /// # Arguments
    /// * `coord` - The packed coordinate.
    /// * `shape` - The location shape.
    /// * `angle` - The rotation angle (0--3).
    /// * `id` - The location type identifier.
    /// * `start` - The start cycle of the merge.
    /// * `end` - The end cycle of the merge.
    /// * `pid` - The player slot index that should see the merged loc.
    /// * `south` - Southern boundary of the merge area.
    /// * `east` - Eastern boundary of the merge area.
    /// * `north` - Northern boundary of the merge area.
    /// * `west` - Western boundary of the merge area.
    #[allow(clippy::too_many_arguments)]
    fn merge_loc(
        &mut self,
        coord: u32,
        shape: u8,
        angle: u8,
        id: u16,
        start: u16,
        end: u16,
        pid: u16,
        south: u16,
        east: u16,
        north: u16,
        west: u16,
    );

    /// Removes a location from the given coordinate and collision layer.
    ///
    /// # Arguments
    /// * `coord` - The packed coordinate.
    /// * `layer` - The collision layer of the location to remove.
    /// * `duration` - The tick at which the removal should revert.
    fn remove_loc(&mut self, coord: u32, layer: u8, duration: u64);

    /// Plays a sequence animation on a location.
    ///
    /// # Arguments
    /// * `coord` - The packed coordinate of the location.
    /// * `id` - The location type identifier.
    /// * `seq` - The sequence (animation) identifier to play.
    fn anim_loc(&mut self, coord: u32, id: u16, seq: u16);

    /// Creates a projectile animation between two map positions.
    ///
    /// # Arguments
    /// * `y` - The level (height plane).
    /// * `x` - The source tile X coordinate.
    /// * `z` - The source tile Z coordinate.
    /// * `dst_x` - The destination tile X coordinate.
    /// * `dst_z` - The destination tile Z coordinate.
    /// * `target` - The target index (positive = player, negative = NPC, 0 = none).
    /// * `id` - The spotanim (graphic) identifier.
    /// * `src_height` - The height offset at the source.
    /// * `dst_height` - The height offset at the destination.
    /// * `start` - The start delay in client ticks.
    /// * `end` - The end delay in client ticks.
    /// * `peak` - The peak arc height.
    /// * `arc` - The arc curve factor.
    #[allow(clippy::too_many_arguments)]
    fn map_proj_anim(
        &mut self,
        y: u8,
        x: u16,
        z: u16,
        dst_x: u16,
        dst_z: u16,
        target: i16,
        id: u16,
        src_height: u8,
        dst_height: u8,
        start: u16,
        end: u16,
        peak: u8,
        arc: u8,
    );

    /// Plays a spot animation (graphic) on the map at the given tile.
    ///
    /// # Arguments
    /// * `y` - The level (height plane).
    /// * `x` - The tile X coordinate.
    /// * `z` - The tile Z coordinate.
    /// * `spotanim` - The spotanim identifier to display.
    /// * `height` - The height offset above the tile.
    /// * `delay` - The delay in client ticks before the animation plays.
    fn anim_map(&mut self, y: u8, x: u16, z: u16, spotanim: u16, height: u8, delay: u16);

    /// Checks whether adding a location at the given coordinate is unsafe.
    ///
    /// # Arguments
    /// * `coord` - The coordinate to test.
    ///
    /// # Returns
    /// `true` if the coordinate is flagged as unsafe for location placement.
    fn locaddunsafe(&self, coord: CoordGrid) -> bool;

    /// Checks whether an active loc occupies the given coordinate.
    ///
    /// Like [`locaddunsafe`](Self::locaddunsafe), but invisible wall locs still
    /// count (they are not skipped).
    ///
    /// # Arguments
    /// * `coord` - The coordinate to test.
    ///
    /// # Returns
    /// `true` if an active loc covers the coordinate.
    #[cfg(since_274)]
    fn map_loc(&self, coord: CoordGrid) -> bool;

    /// Returns a mutable reference to the engine's random number generator.
    ///
    /// # Returns
    /// A mutable reference to the [`JavaRandom`] instance used for
    /// deterministic pseudo-random number generation.
    fn random(&mut self) -> &mut JavaRandom;

    /// Indicates whether the server is running in members mode.
    ///
    /// # Returns
    /// `true` if the server is a members world, `false` for free-to-play.
    fn members(&self) -> bool;

    /// Indicates if there is "line of sight" between these two coords.
    ///
    /// # Returns
    /// `true` if there is "line of sight" from the "src" coord to the "dst" coord.
    /// `false` if there is not "line of sight" from the "src" coord to the "dst" coord.
    fn lineofsight(&self, src: CoordGrid, dst: CoordGrid) -> bool;

    /// Indicates if there is "line of walk" between these two coords.
    ///
    /// # Returns
    /// `true` if there is "line of walk" from the "src" coord to the "dst" coord.
    /// `false` if there is not "line of walk" from the "src" coord to the "dst" coord.
    fn lineofwalk(&self, src: CoordGrid, dst: CoordGrid) -> bool;

    /// Indicates if this coord has a `CollisionFlag::WalkBlocked` on it.
    ///
    /// # Returns
    /// `true` if there is `CollisionFlag::WalkBlocked` on it.
    /// `false` if there is not `CollisionFlag::WalkBlocked` on it.
    fn map_blocked(&self, coord: CoordGrid) -> bool;

    /// Indicates if this coord has a `CollisionFlag::Roof` collision flag on it.
    ///
    /// # Returns
    /// `true` if there is `CollisionFlag::Roof` on it.
    /// `false` if there is not `CollisionFlag::Roof` on it.
    fn map_indoors(&self, coord: CoordGrid) -> bool;

    /// Reads a shared variable (vars) by its definition ID.
    ///
    /// # Arguments
    /// * `id` - The vars definition ID.
    ///
    /// # Returns
    /// The current [`VarValue`] of the variable.
    fn get_var(&self, id: u16) -> VarValue;

    /// Writes a shared variable (vars).
    ///
    /// # Arguments
    /// * `id` - The vars definition ID.
    /// * `value` - The new value to set.
    fn set_var(&mut self, id: u16, value: VarValue);
}

/// Player-level operations available to the script VM.
///
/// `ScriptPlayer` defines the contract for every operation a script may perform
/// on a player entity -- reading state (coordinates, stats, vars, inventories),
/// sending client updates (animations, interfaces, messages, sounds), and
/// controlling movement and interaction targets.
///
/// The active player is resolved from the VM's [`ScriptState`](crate::state::ScriptState)
/// via its `active_player` / `active_player2` fields.
pub trait ScriptPlayer {
    /// Returns this player's unique identifier.
    ///
    /// # Returns
    /// The [`PlayerUid`] that uniquely identifies this player across the session.
    fn uid(&self) -> PlayerUid;

    /// Returns the player's current packed coordinate.
    ///
    /// # Returns
    /// A packed `u32` coordinate encoding level, X, and Z.
    fn coord(&self) -> u32;

    /// Returns the component ID of the last interface button the player clicked.
    ///
    /// # Returns
    /// The interface component ID, or a sentinel if none.
    fn last_com(&self) -> i32;

    /// Returns the inventory slot index from the last interaction.
    ///
    /// # Returns
    /// The slot index as `i32`.
    fn last_slot(&self) -> i32;

    /// Returns the inventory slot index of the item used in an item-on-X interaction.
    ///
    /// # Returns
    /// The use-slot index as `i32`.
    fn last_useslot(&self) -> i32;

    /// Returns the target slot index of the last item-on-item interaction.
    ///
    /// # Returns
    /// The target slot index as `i32`.
    fn last_targetslot(&self) -> i32;

    /// Returns the item type ID from the last interaction.
    ///
    /// # Returns
    /// The item (obj) type ID as `i32`.
    fn last_item(&self) -> i32;

    /// Returns the item type ID of the item used in an item-on-X interaction.
    ///
    /// # Returns
    /// The use-item type ID as `i32`.
    fn last_useitem(&self) -> i32;

    /// Indicates whether the player's client is running in low-memory mode.
    ///
    /// # Returns
    /// `true` if the client reported low-memory mode.
    fn lowmem(&self) -> bool;

    /// Indicates whether the player has members status.
    ///
    /// # Returns
    /// `true` if the player is a member.
    fn member(&self) -> bool;

    /// Returns the player's staff moderator level.
    ///
    /// # Returns
    /// `0` for regular players, higher values for increasing staff privileges.
    fn staffmodlevel(&self) -> u8;

    /// Checks whether the player currently has access rights for privileged
    /// operations.
    ///
    /// # Returns
    /// `true` if the player passes the access check.
    fn can_access(&self) -> bool;

    /// Indicates whether the player is currently busy (e.g. in a script-driven
    /// interaction that blocks new actions).
    ///
    /// # Returns
    /// `true` if the player is busy.
    fn busy(&self) -> bool;

    /// Indicates whether the player has initiated a logout.
    ///
    /// # Returns
    /// `true` if the player is in the process of logging out.
    fn logging_out(&self) -> bool;

    /// Checks whether the player currently has an active interaction target.
    ///
    /// # Returns
    /// `true` if an interaction (with an NPC, loc, obj, or player) is in progress.
    fn has_interaction(&self) -> bool;

    /// Checks whether the player has pending movement waypoints.
    ///
    /// # Returns
    /// `true` if the waypoint queue is non-empty.
    fn has_waypoints(&self) -> bool;

    /// Reads a player variable (varp) by its definition ID.
    ///
    /// # Arguments
    /// * `id` - The varp definition ID.
    ///
    /// # Returns
    /// The current [`VarValue`] of the variable.
    fn get_var(&self, id: u16) -> VarValue;

    /// Writes a player variable (varp) and optionally transmits the change to
    /// the client.
    ///
    /// # Arguments
    /// * `id` - The varp definition ID.
    /// * `value` - The new value to set.
    /// * `transmit` - Whether to send the update to the client immediately.
    fn set_var(&mut self, id: u16, value: VarValue, transmit: bool);

    /// Consumes and returns the pending AFK event flag.
    ///
    /// # Returns
    /// `true` if the player triggered an AFK event since the last check; the
    /// flag is cleared after this call.
    fn afk_event(&mut self) -> bool;

    /// Sets whether the player may open the character-design (appearance) screen.
    ///
    /// # Arguments
    /// * `allow` - `true` to permit redesigning, `false` to forbid it.
    fn set_allow_design(&mut self, allow: bool);

    /// Returns the player's current run energy.
    ///
    /// # Returns
    /// Run energy as a `u16` (typically 0--10000, representing 0.0--100.0%).
    fn runenergy(&self) -> u16;

    /// Adds run energy to the player, clamping the result to the valid range.
    ///
    /// # Arguments
    /// * `amount` - The energy to add, in hundredths of a percent (100 = 1%).
    ///   May be negative; the resulting energy is clamped to `0..=10000`.
    fn healenergy(&mut self, amount: i32);

    /// Returns the player's current carried weight.
    ///
    /// # Returns
    /// The carried weight in kilograms (may be negative when weight-reducing
    /// items are equipped).
    fn weight(&self) -> i32;

    /// Reports whether the player's outgoing buffered-packet queue has reached
    /// the soft bandwidth limit.
    ///
    /// This is a developer-facing throttle so scripts can avoid degrading the
    /// player experience by flooding bandwidth.
    ///
    /// # Returns
    /// `true` if the combined size of all queued buffered packets is at least
    /// 5000 bytes.
    fn buffer_full(&self) -> bool;

    /// Makes the player say a message as overhead forced chat.
    ///
    /// # Arguments
    /// * `msg` - The message text to display above the player.
    fn say(&mut self, msg: &str);

    /// Sends the last-login info to the client (welcome screen): days since the
    /// previous login, recovery prompt code, and unread message count.
    fn last_login_info(&mut self);

    /// Returns the player's current (boosted/drained) level in the given stat.
    ///
    /// # Arguments
    /// * `stat` - The stat index.
    ///
    /// # Returns
    /// The effective level as `u16`.
    fn stat(&self, stat: usize) -> u16;

    /// Returns the player's base (unboosted) level in the given stat.
    ///
    /// # Arguments
    /// * `stat` - The stat index.
    ///
    /// # Returns
    /// The base level as `u16`.
    fn stat_base(&self, stat: usize) -> u16;

    /// Returns the sum of all base (unboosted) stat levels.
    fn stat_total(&self) -> i32;

    /// Awards experience in a stat.
    ///
    /// # Arguments
    /// * `stat` - The stat index.
    /// * `xp` - The amount of experience to add (in tenths).
    fn add_xp(&mut self, stat: usize, xp: i32);

    /// Raises a stat's current level by a flat amount plus a percentage of
    /// the base level, capped at base + constant.
    ///
    /// # Arguments
    /// * `stat` - The stat index.
    /// * `constant` - The flat amount to add.
    /// * `percent` - The percentage of the base level to add.
    fn stat_add(&mut self, stat: usize, constant: i32, percent: i32);

    /// Boosts a stat's current level by a flat amount plus a percentage of
    /// the base level, allowed to exceed the base level.
    ///
    /// # Arguments
    /// * `stat` - The stat index.
    /// * `constant` - The flat amount to add.
    /// * `percent` - The percentage of the base level to add.
    fn stat_boost(&mut self, stat: usize, constant: i32, percent: i32);

    /// Heals a stat's current level by a flat amount plus a percentage of
    /// the base level, capped at the base level.
    ///
    /// # Arguments
    /// * `stat` - The stat index.
    /// * `constant` - The flat amount to restore.
    /// * `percent` - The percentage of the base level to restore.
    fn stat_heal(&mut self, stat: usize, constant: i32, percent: i32);

    /// Lowers a stat's current level by a flat amount plus a percentage of
    /// the current level, floored at zero.
    ///
    /// # Arguments
    /// * `stat` - The stat index.
    /// * `constant` - The flat amount to subtract.
    /// * `percent` - The percentage of the current level to subtract.
    fn stat_sub(&mut self, stat: usize, constant: i32, percent: i32);

    /// Drains a stat's current level by a flat amount plus a percentage of
    /// the current level, floored at zero, and also lowers the base level.
    ///
    /// # Arguments
    /// * `stat` - The stat index.
    /// * `constant` - The flat amount to drain.
    /// * `percent` - The percentage of the current level to drain.
    fn stat_drain(&mut self, stat: usize, constant: i32, percent: i32);

    /// Marks a stat as changed so its updated value is transmitted to the client.
    ///
    /// # Arguments
    /// * `stat` - The stat index to flag for retransmission.
    fn change_stat(&mut self, stat: usize);

    /// Awards hero points to a player for contributing damage to an entity.
    ///
    /// # Arguments
    /// * `user37` - The Base37-encoded username of the player receiving points.
    /// * `points` - The number of hero points to award.
    fn heropoints(&mut self, user37: u64, points: i32);

    /// Applies damage to the player and displays a hitsplat.
    ///
    /// # Arguments
    /// * `amount` - The amount of damage to deal.
    /// * `damage_type` - The hitmark type identifier.
    fn damage(&mut self, amount: u8, damage_type: u8);

    /// Finds the player with the most hero points on this entity.
    ///
    /// # Returns
    /// `Some(u64)` with the Base37-encoded username of the top contributor,
    /// or `None` if no hero points have been recorded.
    fn findhero(&self) -> Option<u64>;

    /// Sets whether the player's animation is protected from being overridden.
    ///
    /// # Arguments
    /// * `protect` - `true` to protect the current animation, `false` to allow overrides.
    fn animprotect(&mut self, protect: bool);

    /// Sets the player's ready (idle/stand) animation.
    ///
    /// # Arguments
    /// * `id` - The sequence ID for the ready animation.
    fn readyanim(&mut self, id: u16);

    /// Sets the player's turn-on-spot animation.
    ///
    /// # Arguments
    /// * `id` - The sequence ID for the turn animation.
    fn turnanim(&mut self, id: u16);

    /// Opens a tutorial interface and tracks it as the active tutorial modal.
    ///
    /// # Arguments
    /// * `com` - The interface component ID to open as the tutorial.
    fn tut_open(&mut self, com: u16);

    /// Flashes a tutorial tab to draw the player's attention.
    ///
    /// # Arguments
    /// * `tab` - The tab index to flash.
    fn tut_flash(&mut self, tab: u8);

    /// Closes the active tutorial interface, firing its `IfClose` trigger.
    ///
    /// # Returns
    /// `Ok(())` on success, or a [`ScriptError`](crate::ScriptError) if the
    /// close trigger script fails.
    fn tut_close(&mut self) -> crate::Result<()>;

    /// Sets the player's forward walk animation.
    ///
    /// # Arguments
    /// * `id` - The sequence ID for the forward walk animation.
    fn walkanim(&mut self, id: u16);

    /// Sets the player's backward walk animation.
    ///
    /// # Arguments
    /// * `id` - The sequence ID for the backward walk animation.
    fn walkanim_b(&mut self, id: u16);

    /// Sets the player's left strafe walk animation.
    ///
    /// # Arguments
    /// * `id` - The sequence ID for the left walk animation.
    fn walkanim_l(&mut self, id: u16);

    /// Sets the player's right strafe walk animation.
    ///
    /// # Arguments
    /// * `id` - The sequence ID for the right walk animation.
    fn walkanim_r(&mut self, id: u16);

    /// Sets the player's run animation.
    ///
    /// # Arguments
    /// * `id` - The sequence ID for the run animation.
    fn runanim(&mut self, id: u16);

    /// Configures which interface buttons are valid resume targets for a
    /// paused script dialog.
    ///
    /// # Arguments
    /// * `buttons` - `Some(Vec<i32>)` with the allowed component IDs, or
    ///   `None` to clear the resume button set.
    #[cfg(before_254)]
    fn if_setresumebuttons(&mut self, buttons: Option<Vec<i32>>);

    /// Appends a single component ID to the player's resume button set, lazily
    /// creating the set if it does not yet exist.
    ///
    /// This is the incremental counterpart to `if_setresumebuttons`: the resume
    /// set is reset whenever a suspended dialog script is cleared, so a dialog
    /// accumulates its buttons one `if_addresumebutton` call at a time.
    ///
    /// # Arguments
    /// * `button` - The component ID to add as a valid resume target.
    #[cfg(since_254)]
    fn if_addresumebutton(&mut self, button: i32);

    /// Initiates a player logout.
    ///
    /// # Arguments
    /// * `requested` - `true` if the player requested the logout themselves,
    ///   `false` if forced by the server.
    fn logout(&mut self, requested: bool);

    /// Prevents the player from logging out until the specified tick.
    ///
    /// # Arguments
    /// * `message` - The message to display if the player attempts to log out.
    /// * `until` - The game tick after which logout is permitted again.
    fn prevent_logout(&mut self, message: &str, until: u32);

    /// Sends a game message to the player's chatbox.
    ///
    /// # Arguments
    /// * `msg` - The message text to display.
    fn mes(&mut self, msg: &str);

    /// Sends a game message to the player's chatbox with automatic line wrapping.
    ///
    /// # Arguments
    /// * `msg` - The message text to display (may be wrapped across multiple lines).
    fn message_game_wrapped(&mut self, msg: &str);

    /// Plays an animation on the player's character model.
    ///
    /// # Arguments
    /// * `id` - The sequence ID to play, or `None` to clear the current animation.
    /// * `delay` - The delay in client ticks before the animation starts.
    fn anim(&mut self, id: Option<u16>, delay: u8);

    /// Rebuilds the player's appearance from their worn equipment inventory.
    ///
    /// # Arguments
    /// * `inv` - The inventory ID to read equipment from.
    fn buildappearance(&mut self, inv: u16);

    /// Sets the player's skin color (appearance color slot 4).
    ///
    /// The change is not visible until the appearance is rebuilt.
    ///
    /// # Arguments
    /// * `skin` - The skin color index.
    fn setskincolour(&mut self, skin: u8);

    /// Sets the player's color for a specified idk slot.
    ///
    /// The change is not visible until the appearance is rebuilt.
    ///
    /// # Arguments
    /// * `slot` - The idk index.
    /// * `colour` - The color to set.
    fn setidkcolour(&mut self, slot: u8, colour: u8) -> crate::Result<()>;

    /// Sets an identity-kit (idk) body part and its color from the player's
    /// character design.
    ///
    /// The body slot is derived from the idk's body type (adjusted for gender);
    /// the color is applied to the matching appearance color slot
    /// (hair/torso/legs/feet).
    ///
    /// # Arguments
    /// * `idk_type` - The idk body type index (0-6 male, 7-13 female).
    /// * `idk_id` - The identity-kit ID to apply to the body slot.
    /// * `colour` - The color index to apply to the matching color slot.
    fn setidkit(&mut self, idk_type: u8, idk_id: u16, colour: u8);

    /// Sets the player's gender, converting each appearance body part through
    /// the male/female identity-kit mapping.
    ///
    /// # Arguments
    /// * `gender` - The new gender (`0` = male, `1` = female).
    fn setgender(&mut self, gender: u8);

    /// Resets the player's camera to its default position and orientation.
    fn cam_reset(&mut self);

    /// Sends a hint arrow pointing at the NPC with the given world index.
    ///
    /// # Arguments
    /// * `nid` - The world index of the NPC to highlight.
    fn hint_npc(&mut self, nid: u16);

    /// Sends a hint arrow hovering over the given tile.
    ///
    /// # Arguments
    /// * `offset` - The arrow position type (`2`--`6`) selecting where over the tile the
    ///   arrow hovers.
    /// * `x` - The absolute tile X coordinate.
    /// * `z` - The absolute tile Z coordinate.
    /// * `height` - The vertical offset of the arrow above the tile.
    fn hint_tile(&mut self, offset: u8, x: u16, z: u16, height: u8);

    /// Sends a hint arrow pointing at the player with the given index.
    ///
    /// # Arguments
    /// * `slot` - The player index (pid) to highlight.
    fn hint_player(&mut self, slot: u16);

    /// Clears any active hint arrow on the player's client.
    fn stop_hint(&mut self);

    /// Returns the player's gender.
    ///
    /// # Returns
    /// `0` for male, `1` for female.
    fn gender(&self) -> u8;

    /// Returns the currently set walk trigger script ID.
    ///
    /// # Returns
    /// The script ID that triggers when the player walks, or a sentinel if unset.
    fn getwalktrigger(&self) -> i32;

    /// Sets the walk trigger script ID that fires when the player moves.
    ///
    /// # Arguments
    /// * `trigger` - The script ID to set as the walk trigger.
    fn walktrigger(&mut self, trigger: i32);

    /// Returns the player's current overhead head icon bitfield.
    ///
    /// # Returns
    /// The head icon flags as `u8`.
    fn headicons_get(&self) -> u8;

    /// Sets the player's overhead head icon bitfield.
    ///
    /// # Arguments
    /// * `headicons` - The new head icon flags.
    fn headicons_set(&mut self, headicons: u8);

    /// Closes any open modal interface.
    ///
    /// # Arguments
    /// * `clear` - Whether to also clear the interface state on the client.
    ///
    /// # Returns
    /// `Ok(())` on success, or a [`ScriptError`](crate::ScriptError) if the
    /// close triggers an error.
    fn if_close(&mut self, clear: bool) -> crate::Result<()>;

    /// Opens a chatbox interface (e.g. a dialog).
    ///
    /// # Arguments
    /// * `id` - The interface component ID to open in the chatbox area.
    fn if_openchat(&mut self, id: u16);

    /// Opens a main-area interface alongside a side-panel interface.
    ///
    /// # Arguments
    /// * `com` - The main interface component ID.
    /// * `side` - The side-panel interface component ID.
    fn if_openmain_side(&mut self, com: u16, side: u16);

    /// Opens a full-screen main-area interface.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    fn if_openmain(&mut self, com: u16);

    /// Opens a side-panel interface.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    fn if_openside(&mut self, com: u16);

    /// Sets the animation displayed on an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `seq` - The sequence (animation) ID to display.
    fn if_setanim(&mut self, com: u16, seq: u16);

    /// Sets the colour of an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `colour` - The RGB15 colour value.
    fn if_setcolour(&mut self, com: u16, colour: u16);

    /// Shows or hides an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `hide` - `true` to hide, `false` to show.
    fn if_sethide(&mut self, com: u16, hide: bool);

    /// Sets the model displayed on an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `model` - The model ID to display.
    fn if_setmodel(&mut self, com: u16, model: u16);

    /// Sets an NPC chathead model on an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `npc` - The NPC type ID whose head model to display.
    fn if_setnpchead(&mut self, com: u16, npc: u16);

    /// Displays an object model on an interface component at the given zoom level.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `obj` - The object (item) type ID.
    /// * `zoom` - The zoom level for the model display.
    fn if_setobject(&mut self, com: u16, obj: u16, zoom: u16);

    /// Sets the player's own chathead model on an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    fn if_setplayerhead(&mut self, com: u16);

    /// Sets the position of an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `x` - The X position in interface coordinates.
    /// * `y` - The Y position in interface coordinates.
    fn if_setposition(&mut self, com: u16, x: u16, y: u16);

    /// Sets the scrollbar position on an interface component.
    #[cfg(since_245_2)]
    fn if_setscrollpos(&mut self, com: u16, y: u16);

    /// Opens a modal overlay interface component.
    #[cfg(since_244)]
    fn if_openoverlay(&mut self, com: u16);

    /// Sets (or clears, when `value` is empty) a right-click option shown on
    /// this player's context menu for other players.
    ///
    /// # Arguments
    /// * `op` - The option slot index.
    /// * `value` - The option text (empty clears the slot).
    /// * `primary` - Whether this is a primary (left-click) option.
    #[cfg(since_254)]
    fn set_player_op(&mut self, op: u8, value: &str, primary: u8);

    /// Sets the client minimap state.
    ///
    /// # Arguments
    /// * `minimap_type` - `0` normal, `1` disable click, `2` blacked out.
    #[cfg(since_274)]
    fn minimap_toggle(&mut self, minimap_type: u8);

    /// Sets the player's appearance "skill level", shown by the client
    /// alongside the combat level.
    ///
    /// # Arguments
    /// * `level` - The skill level value (`0` = unset).
    #[cfg(since_274)]
    fn set_skill_level(&mut self, level: u16);

    /// Recolors an interface component model, remapping one color to another.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `src` - The source color to replace.
    /// * `dst` - The destination color to apply.
    #[cfg(before_245_2)]
    fn if_setrecol(&mut self, com: u16, src: u16, dst: u16);

    /// Assigns an interface component to a tab slot.
    ///
    /// # Arguments
    /// * `tab` - The tab index.
    /// * `com` - The interface component ID to place in the tab.
    fn if_settab(&mut self, tab: u16, com: u8);

    /// Switches the client's currently selected (active) tab.
    ///
    /// # Arguments
    /// * `tab` - The tab index to make active.
    fn if_settabactive(&mut self, tab: u8);

    /// Sets the text content of an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `text` - The text string to display.
    fn if_settext(&mut self, com: u16, text: &str);

    /// Plays a MIDI jingle (short musical effect) for the player.
    ///
    /// # Arguments
    /// * `length` - The playback length in client ticks.
    /// * `data` - The raw MIDI data bytes.
    #[cfg(rev = "225")]
    fn midi_jingle(&mut self, length: u16, data: &[u8]);

    /// Plays a MIDI jingle (short musical effect) for the player.
    ///
    /// # Arguments
    /// * `id` - The id of the midi from idx(3).
    /// * `delay` - The playback delay.
    #[cfg(since_244)]
    fn midi_jingle(&mut self, id: u16, delay: u16);

    /// Starts playing a MIDI song (background music) for the player.
    ///
    /// # Arguments
    /// * `name` - The song name.
    /// * `crc` - The CRC checksum of the song data.
    /// * `len` - The length of the song data.
    #[cfg(rev = "225")]
    fn midi_song(&mut self, name: &str, crc: i32, len: i32);

    /// Starts playing a MIDI song (background music) for the player.
    ///
    /// # Arguments
    /// * `id` - The id of the midi from idx(3).
    #[cfg(since_244)]
    fn midi_song(&mut self, id: u16);

    /// Makes the player face a specific tile.
    ///
    /// # Arguments
    /// * `x` - The tile X coordinate to face.
    /// * `z` - The tile Z coordinate to face.
    fn facesquare(&mut self, x: u16, z: u16);

    /// Plays a synthesized sound effect for the player.
    ///
    /// # Arguments
    /// * `synth` - The sound effect identifier.
    /// * `loops` - The number of times to loop the sound.
    /// * `delay` - The delay in client ticks before playback starts.
    fn sound_synth(&mut self, synth: u16, loops: u8, delay: u16);

    /// Clears the player's pending action, if any.
    ///
    /// # Returns
    /// `Ok(())` on success, or a [`ScriptError`](crate::ScriptError) if clearing
    /// fails.
    fn clearpendingaction(&mut self) -> crate::Result<()>;

    /// Suspends the player's currently running script for a number of ticks.
    ///
    /// # Arguments
    /// * `delay` - The number of game ticks to suspend execution.
    fn delay(&mut self, delay: u32);

    /// Records an arrive-delay timestamp so movement completion can be checked.
    ///
    /// # Arguments
    /// * `clock` - The game tick at which the player arrives.
    ///
    /// # Returns
    /// `true` if a delay was actually applied (the player moved this/last tick), in which
    /// case the caller should suspend the script; `false` if it was a no-op and the script
    /// should continue this tick.
    fn arrivedelay(&mut self, clock: u32) -> bool;

    /// Opens a count-dialog input prompt on the client.
    fn countdialog(&mut self);

    /// Sets the player's run mode.
    ///
    /// # Arguments
    /// * `run` - The run mode flag (`0` = walk, `1` = run, `2` = temporary run).
    fn run(&mut self, run: u8);

    /// Stops the player's current action and clears interaction state.
    ///
    /// # Returns
    /// `Ok(())` on success, or a [`ScriptError`](crate::ScriptError) if stopping
    /// fails.
    fn stopaction(&mut self) -> crate::Result<()>;

    /// Instantly moves the player to a coordinate without walking (no
    /// intermediate tiles are processed).
    ///
    /// # Arguments
    /// * `coord` - The packed destination coordinate.
    ///
    /// # Side Effects
    /// Clears the waypoint queue and updates the player's zone membership.
    fn telejump(&mut self, coord: u32);

    /// Teleports the player to a coordinate, processing zone transitions.
    ///
    /// # Arguments
    /// * `coord` - The packed destination coordinate.
    ///
    /// # Side Effects
    /// Clears the waypoint queue, updates the player's zone membership, and
    /// sends map reload data to the client if necessary.
    fn teleport(&mut self, coord: u32);

    /// Plays an exact-move animation that linearly interpolates the player
    /// between two positions.
    ///
    /// # Arguments
    /// * `start_x` - The starting tile X coordinate.
    /// * `start_z` - The starting tile Z coordinate.
    /// * `end_x` - The ending tile X coordinate.
    /// * `end_z` - The ending tile Z coordinate.
    /// * `begin` - The start time in client ticks.
    /// * `finish` - The end time in client ticks.
    /// * `direction` - The facing direction during the move.
    #[allow(clippy::too_many_arguments)]
    fn exactmove(
        &mut self,
        start_x: u16,
        start_z: u16,
        end_x: u16,
        end_z: u16,
        begin: u16,
        finish: u16,
        direction: u8,
    );

    /// Displays a spot animation (graphic) attached to the player.
    ///
    /// # Arguments
    /// * `spotanim` - The spotanim identifier.
    /// * `height` - The height offset above the player model.
    /// * `delay` - The delay in client ticks before the animation plays.
    fn spotanim(&mut self, spotanim: u16, height: u16, delay: u16);

    /// Enqueues a script for deferred execution on this player.
    ///
    /// # Arguments
    /// * `script_id` - The compiled script identifier to enqueue.
    /// * `priority` - The queue priority determining execution order and
    ///   cancellation semantics.
    /// * `delay` - The number of game ticks to wait before the script runs.
    /// * `args` - Optional script arguments to pass when execution starts.
    ///
    /// # Returns
    /// `Ok(())` on success, or a [`ScriptError`](crate::ScriptError) if the
    /// script cannot be found.
    fn queue(
        &mut self,
        script_id: i32,
        priority: QueuePriority,
        delay: u16,
        args: Option<Vec<ScriptArgument>>,
    ) -> crate::Result<()>;

    /// Sets a recurring timer that fires a script at a fixed interval.
    ///
    /// # Arguments
    /// * `script_id` - The compiled script identifier to run on each tick.
    /// * `priority` - The timer priority (`Normal` or `Soft`).
    /// * `interval` - The number of game ticks between each firing.
    /// * `clock` - The current game tick used to calculate the first firing.
    /// * `args` - Optional script arguments passed on each invocation.
    fn settimer(
        &mut self,
        script_id: i32,
        priority: TimerPriority,
        interval: u16,
        clock: u32,
        args: Option<Vec<ScriptArgument>>,
    );

    /// Clears the player's timer for the given script, regardless of priority
    /// (both the normal and soft lanes are cleared).
    ///
    /// # Arguments
    /// * `script_id` - The script identifier of the timer(s) to clear.
    fn cleartimer(&mut self, script_id: i32);

    /// Removes all queued (normal and weak) scripts matching the given script.
    ///
    /// # Arguments
    /// * `script_id` - The script identifier to unlink from the queues.
    fn clearqueue(&mut self, script_id: i32);

    /// Counts the player's queued (normal and weak) scripts matching the given
    /// script.
    ///
    /// # Arguments
    /// * `script_id` - The script identifier to count.
    ///
    /// # Returns
    /// The number of matching queued scripts.
    fn getqueue(&self, script_id: i32) -> i32;

    /// Returns the clock of the player's timer for the given script.
    ///
    /// # Arguments
    /// * `script_id` - The timer script identifier to look up.
    /// * `now` - The current game tick.
    ///
    /// # Returns
    /// The clock of the timer for the given script, or `-1` if the
    /// player has no timer for that script.
    fn gettimer(&self, script_id: i32) -> i32;

    fn cam_lookat(&mut self, x: u16, z: u16, height: u16, rate: u8, rate2: u8)
    -> crate::Result<()>;

    fn cam_moveto(&mut self, x: u16, z: u16, height: u16, rate: u8, rate2: u8)
    -> crate::Result<()>;

    fn cam_shake(&mut self, direction: u8, jitter: u8, amplitude: u8, frequency: u8);

    /// Returns an immutable reference to a player inventory.
    ///
    /// # Arguments
    /// * `id` - The inventory definition ID.
    ///
    /// # Returns
    /// `Some(&Inventory)` if the inventory exists, `None` otherwise.
    fn get_inv(&mut self, id: u16) -> Option<&Inventory>;

    /// Returns a mutable reference to a player inventory.
    ///
    /// # Arguments
    /// * `id` - The inventory definition ID.
    ///
    /// # Returns
    /// `Some(&mut Inventory)` if the inventory exists, `None` otherwise.
    fn get_inv_mut(&mut self, id: u16) -> Option<&mut Inventory>;

    /// Returns mutable references to two distinct player inventories simultaneously.
    ///
    /// # Arguments
    /// * `a` - The first inventory definition ID.
    /// * `b` - The second inventory definition ID (must differ from `a`).
    ///
    /// # Returns
    /// `Some((&mut Inventory, &mut Inventory))` if both exist, `None` otherwise.
    fn get_inv_pair_mut(&mut self, a: u16, b: u16) -> Option<(&mut Inventory, &mut Inventory)>;

    /// Retrieves a player inventory, creating it if it does not yet exist.
    ///
    /// # Arguments
    /// * `id` - The inventory definition ID.
    /// * `size` - The number of slots to allocate if the inventory is created.
    /// * `stack_mode` - The stacking behavior if the inventory is created.
    ///
    /// # Returns
    /// A mutable reference to the inventory.
    fn get_or_create_inv(
        &mut self,
        id: u16,
        size: usize,
        stack_mode: rs_inv::StackMode,
    ) -> &mut Inventory;

    /// Registers an inventory for automatic transmission to the client via
    /// the specified interface component.
    ///
    /// # Arguments
    /// * `inv_id` - The inventory definition ID.
    /// * `com` - The interface component ID that will display the inventory.
    fn add_inv_transmit(&mut self, inv_id: u16, com: u16);

    /// Checks whether a given interface component has a transmit binding.
    ///
    /// # Arguments
    /// * `com` - The interface component ID to check.
    ///
    /// # Returns
    /// `Some(inv_id)` if the component is bound, `None` otherwise.
    fn has_inv_transmit(&self, com: u16) -> Option<u16>;

    /// Removes all transmit bindings for the given inventory.
    ///
    /// # Arguments
    /// * `inv_id` - The inventory definition ID whose transmit bindings
    ///   should be cleared.
    fn clear_inv_transmits(&mut self, inv_id: u16);

    /// Registers a cross-player inventory listener: mirrors another player's
    /// inventory onto an interface component (`invother_transmit`).
    ///
    /// # Arguments
    /// * `com` - The interface component ID that will display the inventory.
    /// * `inv_id` - The inventory definition ID to display.
    /// * `uid` - The script-uid of the source player whose inventory is shown.
    fn add_inv_other_transmit(&mut self, com: u16, inv_id: u16, uid: i32);

    /// Returns the cross-player inventory listener bound to a component, if any.
    ///
    /// # Arguments
    /// * `com` - The interface component ID to check.
    ///
    /// # Returns
    /// `Some((uid, inv_id))` if `com` mirrors another player's inventory, else `None`.
    fn has_inv_other_transmit(&self, com: u16) -> Option<(i32, u16)>;

    /// Appends a waypoint to the player's movement queue.
    ///
    /// # Arguments
    /// * `x` - The destination tile X coordinate.
    /// * `z` - The destination tile Z coordinate.
    fn queue_waypoint(&mut self, x: u16, z: u16);

    /// Starts the player walking toward the given tile, replacing any
    /// existing waypoints.
    ///
    /// # Arguments
    /// * `dest_x` - The destination tile X coordinate.
    /// * `dest_z` - The destination tile Z coordinate.
    fn walk(&mut self, dest_x: u16, dest_z: u16);

    /// Clears all pending movement waypoints.
    fn clear_waypoints(&mut self);

    /// Sets the player's interaction target to a location.
    ///
    /// # Arguments
    /// * `coord` - The packed coordinate of the target location.
    /// * `id` - The location type identifier.
    /// * `width` - The location width in tiles.
    /// * `length` - The location length in tiles.
    /// * `shape` - The location shape.
    /// * `angle` - The rotation angle (0--3).
    /// * `layer` - The collision layer.
    /// * `op` - The interaction opcode.
    #[allow(clippy::too_many_arguments)]
    fn set_interaction_loc(
        &mut self,
        coord: u32,
        id: u16,
        width: u8,
        length: u8,
        shape: u8,
        angle: u8,
        layer: u8,
        op: u8,
    );

    /// Sets the player's interaction target to an NPC.
    ///
    /// # Arguments
    /// * `nid` - The NPC slot index.
    /// * `op` - The interaction opcode.
    fn set_interaction_npc(&mut self, nid: u16, op: u8);

    /// Sets the player's interaction target to a ground object.
    ///
    /// # Arguments
    /// * `coord` - The packed coordinate of the target object.
    /// * `id` - The object type identifier.
    /// * `count` - The stack count used to identify the specific object instance.
    /// * `op` - The interaction opcode.
    fn set_interaction_obj(&mut self, coord: u32, id: u16, count: u32, op: u8);

    /// Sets the player's interaction target to another player.
    ///
    /// # Arguments
    /// * `pid` - The target player slot index.
    /// * `op` - The interaction opcode.
    fn set_interaction_player(&mut self, pid: u16, op: u8);

    /// Records a spell/use component as the subject of the player's current
    /// interaction, overriding the trigger-lookup type id.
    ///
    /// Must be called *after* one of the `set_interaction_*` methods, which reset
    /// the interaction subject.
    ///
    /// # Arguments
    /// * `com` - The spell or use component id to key the interaction trigger on.
    fn set_interaction_spell(&mut self, com: u16);

    /// Checks whether the player is within operable distance of a location.
    ///
    /// # Arguments
    /// * `coord` - The packed coordinate of the location.
    /// * `width` - The location width in tiles.
    /// * `length` - The location length in tiles.
    /// * `shape` - The location shape.
    /// * `angle` - The rotation angle (0--3).
    /// * `forceapproach` - Approach-override flags.
    ///
    /// # Returns
    /// `true` if the player can operate on the location from their current position.
    fn in_operable_distance_loc(
        &self,
        coord: u32,
        width: u8,
        length: u8,
        shape: u8,
        angle: u8,
        forceapproach: u8,
    ) -> bool;

    /// Sets the approach range for the player's current interaction.
    ///
    /// # Arguments
    /// * `range` - The maximum distance (in tiles) at which the player can
    ///   interact with their target.
    fn aprange(&mut self, range: i32);
}

/// NPC-level operations available to the script VM.
///
/// `ScriptNpc` defines the contract for every operation a script may perform on
/// a non-player character -- reading state (coordinates, stats, vars), sending
/// visual updates (animations, overhead text), controlling movement, managing
/// interaction targets, and scheduling deferred work (queues, timers).
///
/// The active NPC is resolved from the VM's [`ScriptState`](crate::state::ScriptState)
/// via its `active_npc` / `active_npc2` fields.
pub trait ScriptNpc {
    /// Returns this NPC's unique identifier.
    ///
    /// # Returns
    /// The [`NpcUid`](crate::NpcUid) that uniquely identifies this NPC across
    /// the current session.
    fn uid(&self) -> crate::NpcUid;

    /// Returns the NPC's current packed coordinate.
    ///
    /// # Returns
    /// A packed `u32` coordinate encoding level, X, and Z.
    fn coord(&self) -> u32;

    /// Returns the NPC's collision size in tiles.
    ///
    /// # Returns
    /// The size as `u8` (e.g. `1` for a 1x1 NPC, `2` for 2x2).
    fn size(&self) -> u8;

    /// Reads an NPC variable (varn) by its definition ID.
    ///
    /// # Arguments
    /// * `id` - The NPC variable definition ID.
    ///
    /// # Returns
    /// The current [`VarValue`] of the variable.
    fn get_var(&self, id: u16) -> VarValue;

    /// Writes an NPC variable (varn).
    ///
    /// # Arguments
    /// * `id` - The NPC variable definition ID.
    /// * `value` - The new value to set.
    fn set_var(&mut self, id: u16, value: VarValue);

    /// Returns the opcode of the NPC's current interaction target, if any.
    ///
    /// # Returns
    /// `Some(op)` with the interaction opcode, or `None` if no interaction
    /// is active.
    fn target_op(&self) -> Option<u8>;

    /// Sets the NPC's AI mode.
    ///
    /// # Arguments
    /// * `mode` - The AI mode to activate, or `None` to clear the current mode.
    fn set_mode(&mut self, mode: Option<u8>);

    /// Clears the NPC's current interaction target.
    fn clear_interaction(&mut self);

    /// Resets the NPC's patrol progress (point index, stuck timer, and delay
    /// countdown) so it restarts its route from the beginning.
    fn clear_patrol(&mut self);

    /// Resets the NPC to its default spawn state, clearing temporary
    /// transformations, variables, and interaction state.
    fn reset_defaults(&mut self);

    /// Sets the NPC's interaction target to another NPC.
    ///
    /// # Arguments
    /// * `nid` - The slot index of the target NPC.
    /// * `op` - The interaction opcode.
    fn set_interaction_npc(&mut self, nid: u16, op: u8);

    /// Sets the NPC's interaction target to a player.
    ///
    /// # Arguments
    /// * `pid` - The slot index of the target player.
    /// * `op` - The interaction opcode.
    fn set_interaction_player(&mut self, pid: u16, op: u8);

    /// Sets the NPC's interaction target to a location.
    ///
    /// # Arguments
    /// * `coord` - The packed coordinate of the target location.
    /// * `id` - The location type identifier.
    /// * `width` - The location width in tiles.
    /// * `length` - The location length in tiles.
    /// * `shape` - The location shape.
    /// * `angle` - The rotation angle (0--3).
    /// * `layer` - The collision layer.
    /// * `op` - The interaction opcode.
    #[allow(clippy::too_many_arguments)]
    fn set_interaction_loc(
        &mut self,
        coord: u32,
        id: u16,
        width: u8,
        length: u8,
        shape: u8,
        angle: u8,
        layer: u8,
        op: u8,
    );

    /// Sets the NPC's interaction target to a ground object.
    ///
    /// # Arguments
    /// * `coord` - The packed coordinate of the target object.
    /// * `id` - The object type identifier.
    /// * `count` - The stack count used to identify the specific object instance.
    /// * `op` - The interaction opcode.
    fn set_interaction_obj(&mut self, coord: u32, id: u16, count: u32, op: u8);

    /// Plays an animation on the NPC's model.
    ///
    /// # Arguments
    /// * `id` - The sequence ID to play, or `None` to clear the current animation.
    /// * `delay` - The delay in client ticks before the animation starts.
    fn anim(&mut self, id: Option<u16>, delay: u8);

    /// Displays overhead chat text above the NPC.
    ///
    /// # Arguments
    /// * `msg` - The message text to display.
    fn say(&mut self, msg: &str);

    /// Returns the NPC's current level in the given stat.
    ///
    /// # Arguments
    /// * `stat` - The NPC stat index.
    ///
    /// # Returns
    /// The effective level as `u16`.
    fn stat(&self, stat: usize) -> u16;

    /// Returns the NPC's base (unmodified) level in the given stat.
    fn basestat(&self, stat: usize) -> u16;

    /// Applies damage to the NPC and displays a hitsplat.
    ///
    /// # Arguments
    /// * `amount` - The damage amount.
    /// * `damage_type` - The type of hitsplat to display (e.g. melee, ranged, poison).
    fn damage(&mut self, amount: u8, damage_type: u8);

    /// Awards hero points to a player for contributing damage to this NPC.
    ///
    /// # Arguments
    /// * `user37` - The Base37-encoded username of the player receiving points.
    /// * `points` - The number of hero points to award.
    fn heropoints(&mut self, user37: u64, points: i32);

    /// Finds the player with the most hero points on this NPC.
    ///
    /// # Returns
    /// `Some(u64)` with the Base37-encoded username of the top contributor,
    /// or `None` if no hero points have been recorded.
    fn findhero(&self) -> Option<u64>;

    /// Enqueues a script for deferred execution on this NPC.
    ///
    /// # Arguments
    /// * `script_id` - The compiled script identifier to enqueue.
    /// * `delay` - The number of game ticks to wait before the script runs.
    /// * `args` - Optional script arguments to pass when execution starts.
    ///
    /// # Returns
    /// `Ok(())` on success, or a [`ScriptError`](crate::ScriptError) if the
    /// script cannot be found.
    fn queue(
        &mut self,
        script_id: i32,
        delay: u16,
        args: Option<Vec<ScriptArgument>>,
    ) -> crate::Result<()>;

    /// Sets or clears the NPC's recurring timer interval.
    ///
    /// # Arguments
    /// * `interval` - `Some(ticks)` to set the timer interval, or `None` to
    ///   disable the timer.
    fn settimer(&mut self, interval: Option<u16>);

    /// Returns the game tick of the NPC's last movement.
    ///
    /// # Returns
    /// The tick at which the NPC last moved.
    fn last_movement(&self) -> u32;

    /// Suspends the NPC's currently running script for a number of ticks.
    ///
    /// # Arguments
    /// * `delay` - The number of game ticks to suspend execution.
    fn delay(&mut self, delay: u32);

    /// Teleports the NPC to a new coordinate.
    ///
    /// # Arguments
    /// * `coord` - The packed destination coordinate.
    ///
    /// # Side Effects
    /// Updates the NPC's zone membership and collision map.
    fn tele(&mut self, coord: u32);

    /// Makes the NPC face a specific tile.
    ///
    /// # Arguments
    /// * `x` - The tile X coordinate to face.
    /// * `z` - The tile Z coordinate to face.
    fn facesquare(&mut self, x: u16, z: u16);

    /// Walks the NPC toward the given tile using pathfinding.
    ///
    /// # Arguments
    /// * `x` - The destination tile X coordinate.
    /// * `z` - The destination tile Z coordinate.
    fn walk(&mut self, x: u16, z: u16);

    /// Sets the maximum hunt range for this NPC's AI.
    ///
    /// # Arguments
    /// * `range` - The hunt range in tiles.
    fn set_hunt_range(&mut self, range: u8);

    /// Sets or clears the NPC's hunt mode.
    ///
    /// # Arguments
    /// * `mode` - `Some(hunt_type_id)` to set a hunt mode, or `None` to
    ///   disable hunting.
    fn set_hunt_mode(&mut self, mode: Option<u16>);

    /// Temporarily transforms the NPC to a different type.
    ///
    /// # Arguments
    /// * `new_type` - The NPC type ID to transform into.
    /// * `duration` - How many ticks the transformation lasts before reverting.
    /// * `reset` - Whether to reset the NPC's state upon transformation.
    fn change_type(&mut self, new_type: u16, duration: u64, reset: bool);

    /// Returns whether the NPC's current target is within its max range.
    fn inrange(&self) -> bool;

    /// Adds to the NPC's current stat level: `current + (constant + base * percent / 100)`,
    /// clamped to `[0, 255]`.
    fn statadd(&mut self, stat: usize, constant: i32, percent: i32);

    /// Subtracts from the NPC's current stat level: `current - (constant + base * percent / 100)`,
    /// floored at 0.
    fn statsub(&mut self, stat: usize, constant: i32, percent: i32);

    /// Heals the NPC's current stat level: `current + (constant + base * percent / 100)`,
    /// capped at the base level.
    fn statheal(&mut self, stat: usize, constant: i32, percent: i32);

    /// Sets the NPC's walk trigger script and argument.
    fn walktrigger(&mut self, trigger: i32, arg: i32);

    /// Plays a spot animation (graphic) on the NPC.
    fn spotanim(&mut self, id: u16, height: u16, delay: u16);

    /// Returns the NPC's current coord destination.
    fn destination(&self) -> u32;
}

thread_local! {
    static ENGINE_PTR: Cell<*mut ()> = const { Cell::new(std::ptr::null_mut()) };
    static CACHE_PTR: Cell<*const CacheStore> = const { Cell::new(std::ptr::null()) };
}

/// Stores raw engine and cache pointers into thread-local storage.
///
/// # Arguments
/// * `engine` - A type-erased mutable pointer to the engine instance.
/// * `cache` - A const pointer to the [`CacheStore`].
///
/// # Side Effects
/// Overwrites the `ENGINE_PTR` and `CACHE_PTR` thread-locals for the
/// current thread.
///
/// # Call Stack
/// **Called by:** [`with_engine`], `Restore::drop` (the RAII guard inside
/// `with_engine`).
/// **Calls:** `ENGINE_PTR.set`, `CACHE_PTR.set`.
fn set_ptrs(engine: *mut (), cache: *const CacheStore) {
    ENGINE_PTR.set(engine);
    CACHE_PTR.set(cache);
}

/// Executes a closure with the given engine installed in thread-local storage.
///
/// This is the primary entry point for running script VM code. It stores
/// `engine` (and its associated [`CacheStore`]) in thread-local cells so that
/// any function in the call tree can access them via [`cache()`],
/// [`engine()`](engine), or [`engine_mut()`](engine_mut) without passing the
/// engine explicitly.
///
/// Previous thread-local values are saved before the closure runs and
/// automatically restored when the closure returns (or unwinds), making
/// nested `with_engine` calls safe.
///
/// # Arguments
/// * `engine` - A mutable reference to the [`ScriptEngine`] implementor.
/// * `f` - The closure to execute while the engine is installed.
///
/// # Returns
/// The value returned by `f`.
///
/// # Side Effects
/// Temporarily replaces the `ENGINE_PTR` and `CACHE_PTR` thread-locals.
/// The previous values are restored via a drop guard when `f` finishes.
///
/// # Call Stack
/// **Called by:** Game engine tick loop, script execution entry points.
/// **Calls:** [`set_ptrs`], the user-provided closure `f`, and
/// `Restore::drop` (which calls [`set_ptrs`] to restore previous values).
pub fn with_engine<E: ScriptEngine, R>(engine: &mut E, f: impl FnOnce() -> R) -> R {
    let cache = engine.cache() as *const CacheStore;
    let ptr = engine as *mut E as *mut ();
    let prev_engine = ENGINE_PTR.get();
    let prev_cache = CACHE_PTR.get();
    set_ptrs(ptr, cache);
    struct Restore(*mut (), *const CacheStore);
    impl Drop for Restore {
        fn drop(&mut self) {
            set_ptrs(self.0, self.1);
        }
    }
    let _guard = Restore(prev_engine, prev_cache);
    f()
}

/// Returns a static reference to the [`CacheStore`] installed in thread-local
/// storage by [`with_engine`].
///
/// # Returns
/// A `&'static CacheStore` reference. The lifetime is tied to the enclosing
/// [`with_engine`] scope in practice, but is expressed as `'static` because
/// the pointer is stored in a thread-local cell.
///
/// # Panics
/// Debug-asserts that the pointer is non-null. If called outside a
/// [`with_engine`] scope, the assertion fires in debug builds; in release
/// builds the behavior is undefined.
///
/// # Call Stack
/// **Called by:** Opcode handlers across all op modules (core, player, npc,
/// loc, obj, inv, db, server, string), utility helpers.
/// **Calls:** `CACHE_PTR.get`, dereferences the raw pointer.
pub fn cache() -> &'static CacheStore {
    let ptr = CACHE_PTR.get();
    debug_assert!(!ptr.is_null(), "cache() called outside with_engine scope");
    unsafe { &*ptr }
}

/// Returns a typed immutable reference to the engine stored in thread-local
/// storage.
///
/// This is the safe-in-practice wrapper used within the crate. It delegates to
/// [`engine_typed`] and relies on the caller having entered a [`with_engine`]
/// scope with the matching concrete type `E`.
///
/// # Returns
/// A `&'static E` reference to the engine.
///
/// # Panics
/// Debug-asserts (via [`engine_typed`]) that the pointer is non-null.
///
/// # Call Stack
/// **Called by:** Utility helpers in `util.rs`, opcode handlers, iterators.
/// **Calls:** [`engine_typed::<E>()`](engine_typed).
pub(crate) fn engine<E: ScriptEngine + 'static>() -> &'static E {
    unsafe { engine_typed::<E>() }
}

/// Returns a typed mutable reference to the engine stored in thread-local
/// storage.
///
/// This is the safe-in-practice wrapper used within the crate. It delegates to
/// [`engine_typed_mut`] and relies on the caller having entered a
/// [`with_engine`] scope with the matching concrete type `E`.
///
/// # Returns
/// A `&'static mut E` reference to the engine.
///
/// # Panics
/// Debug-asserts (via [`engine_typed_mut`]) that the pointer is non-null.
///
/// # Call Stack
/// **Called by:** Utility helpers in `util.rs`, opcode handlers, iterators.
/// **Calls:** [`engine_typed_mut::<E>()`](engine_typed_mut).
pub(crate) fn engine_mut<E: ScriptEngine + 'static>() -> &'static mut E {
    unsafe { engine_typed_mut::<E>() }
}

/// Returns a typed immutable reference to the engine stored in thread-local
/// storage by [`with_engine`].
///
/// This is the low-level, publicly visible accessor. Most callers within the
/// crate should prefer [`engine()`](engine) instead. External code (e.g.
/// macros in `macros.rs`) calls this directly when the type parameter is
/// already known at the call site.
///
/// # Arguments
/// * `E` (type parameter) - The concrete [`ScriptEngine`] implementor that
///   was passed to [`with_engine`].
///
/// # Returns
/// A `&'static E` reference. The lifetime outlives the borrow only because
/// the pointer lives in a thread-local cell; the reference is logically
/// scoped to the enclosing [`with_engine`] call.
///
/// # Safety
/// `E` must be the concrete type passed to the enclosing `with_engine` call.
/// Calling this with a different type results in undefined behavior. The
/// caller must also ensure no mutable alias to the engine exists.
///
/// # Panics
/// Debug-asserts that the `ENGINE_PTR` thread-local is non-null.
///
/// # Call Stack
/// **Called by:** [`engine::<E>()`](engine), accessor macros in `macros.rs`.
/// **Calls:** `ENGINE_PTR.get`, dereferences the raw pointer.
pub unsafe fn engine_typed<E: ScriptEngine + 'static>() -> &'static E {
    let ptr = ENGINE_PTR.get() as *const E;
    debug_assert!(
        !ptr.is_null(),
        "engine_typed() called outside with_engine scope"
    );
    unsafe { &*ptr }
}

/// Returns a typed mutable reference to the engine stored in thread-local
/// storage by [`with_engine`].
///
/// This is the low-level, publicly visible mutable accessor. Most callers
/// within the crate should prefer [`engine_mut()`](engine_mut) instead.
/// External code (e.g. macros in `macros.rs`) calls this directly when the
/// type parameter is already known at the call site.
///
/// # Arguments
/// * `E` (type parameter) - The concrete [`ScriptEngine`] implementor that
///   was passed to [`with_engine`].
///
/// # Returns
/// A `&'static mut E` reference. The lifetime outlives the borrow only
/// because the pointer lives in a thread-local cell; the reference is
/// logically scoped to the enclosing [`with_engine`] call.
///
/// # Safety
/// `E` must be the concrete type passed to the enclosing `with_engine` call.
/// Calling this with a different type results in undefined behavior. The
/// caller must also ensure no other reference (mutable or immutable) to the
/// engine exists for the duration of the returned borrow.
///
/// # Panics
/// Debug-asserts that the `ENGINE_PTR` thread-local is non-null.
///
/// # Call Stack
/// **Called by:** [`engine_mut::<E>()`](engine_mut), accessor macros in
/// `macros.rs`.
/// **Calls:** `ENGINE_PTR.get`, dereferences the raw pointer.
pub unsafe fn engine_typed_mut<E: ScriptEngine + 'static>() -> &'static mut E {
    let ptr = ENGINE_PTR.get() as *mut E;
    debug_assert!(
        !ptr.is_null(),
        "engine_typed_mut() called outside with_engine scope"
    );
    unsafe { &mut *ptr }
}
