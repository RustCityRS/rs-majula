use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::core::engine;
use crate::counters::COUNTERS;
#[cfg(since_647)]
use crate::region::REGION_X;
use rs_cam::CamKind;
use rs_db::transfer;
use rs_entity::PlayerUid;
use rs_entity::{InteractionTarget, ObjSerial};
use rs_grid::CoordGrid;
use rs_inv::{Inventory, StackMode};
use rs_pack::cache::VarValue;
use rs_pack::types::{LocAngle, LocShape, PlayerStat};
use rs_protocol::network::game::MAX_PACKET_SIZE;
use rs_protocol::network::game::info_prot::PlayerInfoProt;
#[cfg(since_647)]
use rs_protocol::network::game::server::loc_prefetch::LocPrefetch;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};
use rs_vm::state::*;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ComId, ScriptError};
use rsmod::PathRequest;
use rsmod::rsmod::collision::collision_strategy::CollisionType;

impl ScriptPlayer for ActivePlayer {
    /// Returns this player's unique identifier.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.uid`
    fn uid(&self) -> PlayerUid {
        self.player.uid
    }

    /// The name the player is drawn with (the appearance block): the
    /// display name they claimed, else their login name as 289 shows it.
    #[cfg(since_647)]
    fn display_name(&self) -> String {
        self.player.shown_name()
    }

    /// Returns the player's current coordinate.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    fn coord(&self) -> CoordGrid {
        self.player.pathing.coord
    }

    /// Returns the component ID of the last interface button clicked, or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::map`, `Option::unwrap_or`
    #[cfg(before_647)]
    fn last_com(&self) -> i32 {
        self.player.last_com.map(|v| v as i32).unwrap_or(-1)
    }

    /// Returns the 32-bit component hash of the last interface button clicked,
    /// or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    #[cfg(since_647)]
    fn last_com(&self) -> i32 {
        self.player.last_com
    }

    /// Returns the inventory slot index from the last interaction, or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::map`, `Option::unwrap_or`
    #[cfg(before_647)]
    fn last_slot(&self) -> i32 {
        self.player.last_slot.map(|v| v as i32).unwrap_or(-1)
    }

    /// Rev-647: slot carried by the IfButton1-10 packet; -1 = none.
    #[cfg(since_647)]
    fn last_slot(&self) -> i32 {
        self.player.last_slot
    }

    /// Returns the use-slot index from an item-on-X interaction, or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::map`, `Option::unwrap_or`
    fn last_useslot(&self) -> i32 {
        self.player.last_use_slot.map(|v| v as i32).unwrap_or(-1)
    }

    /// Returns the target slot index from an item-on-item interaction, or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::map`, `Option::unwrap_or`
    fn last_targetslot(&self) -> i32 {
        self.player.last_target_slot.map(|v| v as i32).unwrap_or(-1)
    }

    /// Returns the component the last drag ended on, or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    #[cfg(since_647)]
    fn last_targetcom(&self) -> i32 {
        self.player.last_target_com.unwrap_or(-1)
    }

    /// Returns the item type ID from the last interaction, or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::map`, `Option::unwrap_or`
    #[cfg(before_647)]
    fn last_item(&self) -> i32 {
        self.player.last_item.map(|v| v as i32).unwrap_or(-1)
    }

    /// Rev-647: obj id carried by the IfButton1-10 packet; -1 = none.
    #[cfg(since_647)]
    fn last_item(&self) -> i32 {
        self.player.last_item
    }

    /// Returns the use-item type ID from an item-on-X interaction, or `-1` if none.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::map`, `Option::unwrap_or`
    fn last_useitem(&self) -> i32 {
        self.player.last_use_item.map(|v| v as i32).unwrap_or(-1)
    }

    /// Indicates whether the player's client is running in low-memory mode.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.low_memory`
    fn lowmem(&self) -> bool {
        self.player.low_memory
    }

    /// Indicates whether the player has members status.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.is_member`
    fn member(&self) -> bool {
        self.player.is_member
    }

    /// Returns the player's staff moderator level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.staff_mod_level`
    fn staffmodlevel(&self) -> u8 {
        self.player.staff_mod_level as u8
    }

    /// Checks whether the player has access rights for privileged operations.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::can_access`
    fn can_access(&self) -> bool {
        self.player.can_access()
    }

    /// Indicates whether the player is currently busy.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::busy`
    fn busy(&self) -> bool {
        self.player.busy()
    }

    /// Indicates whether the player has initiated a logout.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.logout_sent`
    fn logging_out(&self) -> bool {
        self.player.logout_sent
    }

    /// Checks whether the player currently has an active interaction target.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::has_interaction`
    fn has_interaction(&self) -> bool {
        self.player.has_interaction()
    }

    /// Checks whether the player has pending movement waypoints.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Pathing::has_waypoints`
    fn has_waypoints(&self) -> bool {
        self.player.pathing.has_waypoints()
    }

    /// Reads a player variable (varp) by its definition ID.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `VarSet::get`
    fn get_var(&self, id: u16) -> VarValue {
        self.player.vars.get(id).clone()
    }

    /// Writes a player variable (varp) and optionally transmits the change to the client.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::set_varp`
    fn set_var(&mut self, id: u16, value: VarValue, transmit: bool) {
        self.set_varp(id, value, transmit);
    }

    /// Consumes and returns the pending AFK event flag.
    ///
    /// Returns `true` if the player triggered an AFK event since the last check;
    /// the flag is cleared after this call. Never in a region: a random event
    /// keeps the coord it returns the player to, which no region outlives.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads and resets `self.player.afk_event_ready`
    fn afk_event(&mut self) -> bool {
        let ready = self.player.afk_event_ready;
        self.player.afk_event_ready = false;
        #[cfg(since_647)]
        if self.player.pathing.coord.x() >= REGION_X {
            return false;
        }
        ready
    }

    /// Sets whether the player may open the character-design (appearance) screen.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** writes `self.player.allow_design`
    fn set_allow_design(&mut self, allow: bool) {
        self.player.allow_design = allow;
    }

    /// Returns the player's current run energy.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.runenergy`
    fn runenergy(&self) -> u16 {
        self.player.runenergy
    }

    /// Adds run energy to the player, clamped to `0..=10000`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** writes `self.player.runenergy`
    fn healenergy(&mut self, amount: i32) {
        self.player.runenergy = (self.player.runenergy as i32 + amount).clamp(0, 10000) as u16;
    }

    /// Returns the player's current carried weight.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.runweight`
    fn weight(&self) -> i32 {
        self.player.runweight
    }

    /// Reports whether the buffered-packet queue has reached the soft
    /// bandwidth limit of [`MAX_PACKET_SIZE`].
    ///
    /// The queue holds each packet's on-wire bytes (opcode, frame header,
    /// payload) back to back, so its length is the total pending for this
    /// player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    fn buffer_full(&self) -> bool {
        self.buffered.pos >= MAX_PACKET_SIZE
    }

    /// Makes the player say a message as overhead forced chat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.say` and the `PlayerInfoProt::Say` mask
    fn say(&mut self, msg: &str) {
        self.player.info.say = Some(msg.into());
        self.player.info.masks |= PlayerInfoProt::Say as u16;
    }

    /// Logs a `session_log` event for moderators.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::log_session`
    fn session_log(&mut self, event_type: i32, event: &str) {
        self.log_session(event_type, event);
    }

    /// Logs a `wealth_event` for moderators.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Counters::wealth`, `ActivePlayer::log_wealth`
    fn wealth_event(&mut self, event_type: i32, item: &str, count: i32, value: i32) {
        if !self.player.bot {
            COUNTERS.wealth(event_type, count, value);
        }
        self.log_wealth(event_type, item, count, value);
    }

    /// Logs a handover to another player: a trade, a duel stake, or any
    /// other inventory moved straight across.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `BOTH_MOVEINV` via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::log_transfer`
    fn log_handover(&mut self, inv: &str, obj: u16, count: i32, to37: u64) {
        let event_type = match inv {
            "tradeoffer" => transfer::TRADE,
            "dueloffer" => transfer::STAKE,
            _ => transfer::GIVE,
        };
        self.log_transfer(event_type, self.coord(), obj, count, Some(to37));
    }

    /// Logs a drop on the floor, or a pickup from it.
    ///
    /// # Call Stack
    ///
    /// **Called by:** the drop ops and `OBJ_TAKEITEM` via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::log_transfer`
    fn log_floor(&mut self, taken: bool, coord: CoordGrid, obj: u16, count: i32) {
        let event_type = if taken {
            transfer::TAKE
        } else {
            transfer::DROP
        };
        self.log_transfer(event_type, coord, obj, count, None);
    }

    /// Sends the last-login info packet (welcome screen).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::last_login_info`
    fn last_login_info(&mut self) {
        self.last_login_info();
    }

    /// Returns the player's current (boosted/drained) level in the given stat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Stats::level`
    fn stat(&self, stat: usize) -> u16 {
        self.player.stats.level(stat)
    }

    /// Returns the player's base (unboosted) level in the given stat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Stats::base_level`
    fn stat_base(&self, stat: usize) -> u16 {
        self.player.stats.base_level(stat)
    }

    /// Returns the sum of all base stat levels.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    fn stat_total(&self) -> i32 {
        self.player.stats.total()
    }

    /// Awards experience in a stat, recalculating levels and combat if needed.
    ///
    /// Delegates XP math to `Stats::add_xp`. If the base level increases
    /// (a level-up), triggers `change_stat`, enqueues the `AdvanceStat` script
    /// for the stat, and rebuilds the player's appearance if their combat level
    /// changes.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Stats::add_xp`, `change_stat`, `Engine::script_by_key`, `ScriptQueue::add`,
    /// `recalc_combat_and_appearance`
    fn add_xp(&mut self, stat: usize, xp: i32) {
        #[cfg(since_647)]
        let before = self.player.stats.base_level(stat);
        if !self.player.stats.add_xp(stat, xp) {
            return;
        }
        self.change_stat(stat);
        // Rev-647: the level-up script runs once for the award and is told how many levels it
        // crossed, for 2011's "You've just advanced 2 Mining levels!" (levelup.rs2).
        #[cfg(since_647)]
        let args = Some(vec![ScriptArgument::Int(i32::from(
            self.player.stats.base_level(stat).saturating_sub(before),
        ))]);
        #[cfg(before_647)]
        let args = None;
        if let Some(script) =
            engine().script_by_key(ServerTriggerType::AdvanceStat, Some(stat as i32), None)
        {
            let _ = self
                .player
                .state
                .queues
                .add(QueuePriority::Engine, script.id, 0, args);
        }
        self.recalc_combat_and_appearance();
    }

    /// Raises a stat's current level via `Stats::add`.
    ///
    /// Clears hero points if Hitpoints reaches or exceeds the base level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Stats::add`, `change_stat`
    fn stat_add(&mut self, stat: usize, constant: i32, percent: i32) {
        let prev = self.player.stats.level(stat);
        self.player.stats.add(stat, constant, percent);
        if stat == PlayerStat::Hitpoints as usize
            && self.player.stats.level(PlayerStat::Hitpoints as usize)
                >= self.player.stats.base_level(PlayerStat::Hitpoints as usize)
        {
            self.player.hero_points.clear();
        }
        if self.player.stats.level(stat) != prev {
            self.change_stat(stat);
        }
    }

    /// Boosts a stat's current level via `Stats::boost`.
    ///
    /// Clears hero points if Hitpoints reaches or exceeds the base level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Stats::boost`, `change_stat`
    fn stat_boost(&mut self, stat: usize, constant: i32, percent: i32) {
        let prev = self.player.stats.level(stat);
        self.player.stats.boost(stat, constant, percent);
        if stat == PlayerStat::Hitpoints as usize
            && self.player.stats.level(PlayerStat::Hitpoints as usize)
                >= self.player.stats.base_level(PlayerStat::Hitpoints as usize)
        {
            self.player.hero_points.clear();
        }
        if self.player.stats.level(stat) != prev {
            self.change_stat(stat);
        }
    }

    /// Heals a stat's current level via `Stats::heal`.
    ///
    /// Clears hero points if Hitpoints reaches or exceeds the base level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Stats::heal`, `change_stat`
    fn stat_heal(&mut self, stat: usize, constant: i32, percent: i32) {
        let prev = self.player.stats.level(stat);
        self.player.stats.heal(stat, constant, percent);
        if stat == PlayerStat::Hitpoints as usize
            && self.player.stats.level(PlayerStat::Hitpoints as usize)
                >= self.player.stats.base_level(PlayerStat::Hitpoints as usize)
        {
            self.player.hero_points.clear();
        }
        if self.player.stats.level(stat) != prev {
            self.change_stat(stat);
        }
    }

    /// Lowers a stat's current level via `Stats::sub`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Stats::sub`, `change_stat`
    fn stat_sub(&mut self, stat: usize, constant: i32, percent: i32) {
        let prev = self.player.stats.level(stat);
        self.player.stats.sub(stat, constant, percent);
        if self.player.stats.level(stat) != prev {
            self.change_stat(stat);
        }
    }

    /// Drains a stat's current level via `Stats::drain`.
    ///
    /// Unlike `stat_sub`, the percentage is based on the current level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Stats::drain`, `change_stat`
    fn stat_drain(&mut self, stat: usize, constant: i32, percent: i32) {
        let prev = self.player.stats.level(stat);
        self.player.stats.drain(stat, constant, percent);
        if self.player.stats.level(stat) != prev {
            self.change_stat(stat);
        }
    }

    /// Enqueues the `ChangeStat` trigger script for the given stat.
    ///
    /// Looks up the trigger script by stat index and adds it to the player's
    /// engine-priority queue for deferred execution.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `add_xp`, `stat_add`, `stat_boost`, `stat_heal`, `stat_sub`, `stat_drain`
    /// **Calls:** `Engine::script_by_key`, `ScriptQueue::add`
    fn change_stat(&mut self, stat: usize) {
        if let Some(script) =
            engine().script_by_key(ServerTriggerType::ChangeStat, Some(stat as i32), None)
        {
            let _ = self
                .player
                .state
                .queues
                .add(QueuePriority::Engine, script.id, 0, None);
        }
    }

    /// Awards hero points to a player for contributing damage.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `HeroPoints::add_hero`
    fn heropoints(&mut self, user37: u64, points: i32) {
        self.player.hero_points.add_hero(user37, points);
    }

    /// Applies damage to the player and displays a hitsplat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::damage`
    fn damage(&mut self, amount: u8, damage_type: u8) {
        self.damage(amount, damage_type);
    }

    /// Paints a 647 hitsplat.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::hit`
    #[cfg(since_647)]
    fn hit(&mut self, amount: u16, hitmark: u8, soak: u16, healthbar: u8, source: Option<u16>) {
        self.hit(amount, hitmark, soak, healthbar, source);
    }

    /// Finds the player with the most hero points on this entity.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `HeroPoints::find_hero`
    fn findhero(&self) -> Option<u64> {
        self.player.hero_points.find_hero()
    }

    /// Sets whether the player's animation is protected from being overridden.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.anim_protect`
    fn animprotect(&mut self, protect: bool) {
        self.player.info.anim_protect = protect;
    }

    /// Sets the player's ready (idle/stand) animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.readyanim`
    fn readyanim(&mut self, id: u16) {
        self.player.info.readyanim = Some(id);
    }

    /// Sets the player's turn-on-spot animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.turnanim`
    fn turnanim(&mut self, id: u16) {
        self.player.info.turnanim = Some(id);
    }

    /// Opens a tutorial interface and tracks it as the active tutorial modal.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::open_tutorial`
    fn tut_open(&mut self, com: u16) {
        self.open_tutorial(com);
    }

    /// Flashes a tutorial tab to draw the player's attention.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::tut_flash`
    fn tut_flash(&mut self, tab: u8) {
        self.tut_flash(tab);
    }

    /// Closes the active tutorial interface, firing its `IfClose` trigger.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::close_tutorial`
    fn tut_close(&mut self) -> rs_vm::Result<()> {
        self.close_tutorial()
    }

    /// Sets the player's forward walk animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.walkanim`
    fn walkanim(&mut self, id: u16) {
        self.player.info.walkanim = Some(id);
    }

    /// Sets the player's backward walk animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.walkanim_b`
    fn walkanim_b(&mut self, id: u16) {
        self.player.info.walkanim_b = Some(id);
    }
    /// Sets the player's left strafe walk animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.walkanim_l`
    fn walkanim_l(&mut self, id: u16) {
        self.player.info.walkanim_l = Some(id);
    }

    /// Sets the player's right strafe walk animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.walkanim_r`
    fn walkanim_r(&mut self, id: u16) {
        self.player.info.walkanim_r = Some(id);
    }

    /// Sets the player's run animation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.runanim`
    fn runanim(&mut self, id: u16) {
        self.player.info.runanim = Some(id);
    }

    /// Configures which interface buttons are valid resume targets for a paused script dialog.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.resume_buttons`
    #[cfg(before_254)]
    fn if_setresumebuttons(&mut self, buttons: Option<Vec<i32>>) {
        self.player.resume_buttons = buttons;
    }

    /// Appends a single interface button to the set of valid resume targets for
    /// a paused script dialog, lazily allocating the set on first use.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** pushes onto `self.player.resume_buttons`
    #[cfg(since_254)]
    fn if_addresumebutton(&mut self, button: i32) {
        self.player
            .resume_buttons
            .get_or_insert_with(Vec::new)
            .push(button);
    }

    /// Initiates a player logout.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.logout_requested`
    fn logout(&mut self, requested: bool) {
        self.player.logout_requested = requested;
    }

    /// Prevents the player from logging out until the specified tick.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.logout_prevented_message` and `logout_prevented_until`
    fn prevent_logout(&mut self, message: &str, until: u32) {
        if self.player.logout_prevented_message.as_deref() != Some(message) {
            self.player.logout_prevented_message = Some(message.into());
        }
        self.player.logout_prevented_until = Some(until);
    }

    /// Sends a game message to the player's chatbox.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::message_game`
    fn mes(&mut self, msg: &str) {
        self.message_game(msg);
    }

    #[cfg(since_647)]
    fn window_mode(&self) -> u8 {
        self.player.window_mode
    }

    #[cfg(since_647)]
    fn if_opentop(&mut self, com: u16, mode: u8) {
        self.if_open_top(com, mode);
    }

    #[cfg(since_647)]
    fn if_opensub(&mut self, sub_id: u16, parent_hash: i32) {
        let kind = self.player.sub_kind(parent_hash);
        self.if_open_sub(sub_id, parent_hash, kind);
    }

    #[cfg(since_647)]
    fn if_closesub(&mut self, component_hash: i32) {
        self.if_close_sub(component_hash);
    }

    #[cfg(since_647)]
    fn if_movesub(&mut self, from: i32, to: i32) {
        self.if_move_sub(from, to);
    }

    #[cfg(since_647)]
    fn if_setevents(&mut self, com: i32, slot1: u16, slot2: u16, settings: i32) {
        self.if_set_events(com, slot1, slot2, settings);
    }

    #[cfg(since_647)]
    fn sendvarc(&mut self, id: u16, value: i32) {
        self.varc_transmit(id, value);
    }

    #[cfg(since_647)]
    fn sendvarc_str(&mut self, id: u16, value: &str) {
        self.varc_str_transmit(id, value);
    }

    #[cfg(since_647)]
    fn varc(&self, id: u16) -> i32 {
        self.player.varcs.get(&id).copied().unwrap_or(0)
    }

    #[cfg(since_647)]
    fn varc_str(&self, id: u16) -> &str {
        self.player.varc_strings.get(&id).map_or("", |value| value)
    }

    #[cfg(since_647)]
    fn runclientscript(&mut self, script_id: i32) {
        self.run_client_script(script_id);
    }

    #[cfg(since_647)]
    fn runclientscript_with_params(&mut self, script_id: i32, args: Vec<ScriptArgument>) {
        self.run_client_script_with_params(script_id, args);
    }

    /// Applies the character-design preview varcs (1008-1019) to the player's
    /// body kit and colours. `sendvarc` mirrors each design varc server-side;
    /// this reads them back and commits the design, the rev-647 replacement
    /// for the 2004-era IdkSaveDesign packet (the 647 client never sends one).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait (`applydesign`)
    /// **Calls:** `setidkit`, writes `self.player.colours`
    #[cfg(since_647)]
    fn applydesign(&mut self) {
        let varc = |id: u16| -> i32 { self.player.varcs.get(&id).copied().unwrap_or(0) };

        let hair = varc(1008);
        let beard = varc(1009);
        let top = varc(1010);
        let arms = varc(1011);
        let hands = varc(1012);
        let legs = varc(1013);
        let feet = varc(1014);

        let c_hair = varc(1015) as u8;
        let c_top = varc(1016) as u8;
        let c_legs = varc(1017) as u8;
        let c_feet = varc(1018) as u8;
        let c_skin = varc(1019) as u8;

        let female = self.player.gender == 1;
        let mut apply = |id: i32, colour: u8| {
            if id < 0 {
                return;
            }
            if let Some(idk) = engine().cache.idks.get_by_id(id as u16) {
                self.setidkit(idk.body_type as u8, id as u16, colour);
            }
        };

        apply(hair, c_hair);
        apply(beard, c_hair);
        apply(top, c_top);
        if arms >= 0 {
            apply(arms, c_top);
        } else {
            let default_arms = if female { 61 } else { 26 };
            apply(default_arms, c_top);
        }
        if hands >= 0 {
            apply(hands, c_top);
        } else {
            let default_hands = if female { 68 } else { 34 };
            apply(default_hands, c_top);
        }
        apply(legs, c_legs);
        apply(feet, c_feet);

        self.player.colours[0] = c_hair;
        self.player.colours[1] = c_top;
        self.player.colours[2] = c_legs;
        self.player.colours[3] = c_feet;
        self.player.colours[4] = c_skin;
    }

    #[cfg(since_647)]
    fn name_suggestions(&self) -> &[String] {
        &self.player.name_suggestions
    }

    #[cfg(since_647)]
    fn suggestion_page(&self) -> usize {
        self.player.name_suggestion_page
    }

    #[cfg(since_647)]
    fn next_suggestion_page(&mut self) {
        let max_page = self.player.name_suggestions.len().saturating_sub(1) / 3;
        if self.player.name_suggestion_page < max_page {
            self.player.name_suggestion_page += 1;
        }
    }

    #[cfg(since_647)]
    fn prev_suggestion_page(&mut self) {
        if self.player.name_suggestion_page > 0 {
            self.player.name_suggestion_page -= 1;
        }
    }

    /// Sends a game message to the player's chatbox with automatic line wrapping.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::message_game_wrapped`
    fn message_game_wrapped(&mut self, msg: &str) {
        self.message_game_wrapped(msg);
    }

    /// Plays an animation on the player's character model.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::anim`
    fn anim(&mut self, id: Option<u16>, delay: u8) {
        self.anim(id, delay);
    }

    /// Rebuilds the player's appearance from their worn equipment inventory.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::buildappearance`
    fn buildappearance(&mut self, inv: u16) {
        self.buildappearance(inv);
    }

    /// Sets the player's skin color (appearance color slot 4).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** writes `self.player.colours[4]`
    fn setskincolour(&mut self, colour: u8) {
        self.player.colours[4] = colour;
    }

    /// Sets the player's color for a specified idk slot.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** writes `self.player.colours[slot] = colour;`
    fn setidkcolour(&mut self, slot: u8, colour: u8) -> rs_vm::Result<()> {
        if slot as usize >= self.player.colours.len() {
            return Err(ScriptError::Runtime(format!("Invalid idk slot: {}", slot)));
        }
        self.player.colours[slot as usize] = colour;
        Ok(())
    }

    /// Sets an identity-kit body part and its color from the character design.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** writes `self.player.body` and `self.player.colours`
    fn setidkit(&mut self, idk_type: u8, idk_id: u16, colour: u8) {
        // Female idks occupy body types 7-13; map them onto the 7 body slots.
        let slot = if self.player.gender == 1 {
            idk_type as i32 - 7
        } else {
            idk_type as i32
        };
        if !(0..7).contains(&slot) {
            return;
        }
        let slot = slot as usize;
        self.player.body[slot] = idk_id as i32;
        // hair/jaw -> 0, torso/arms -> 1, legs -> 2, feet -> 3; hands (4) keep skin.
        let colour_slot = match slot {
            0 | 1 => Some(0),
            2 | 3 => Some(1),
            5 => Some(2),
            6 => Some(3),
            _ => None,
        };
        if let Some(cs) = colour_slot {
            self.player.colours[cs] = colour;
        }
    }

    /// Sets the player's gender, remapping every appearance body part between
    /// the male and female identity-kit sets.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** writes `self.player.body` and `self.player.gender`
    fn setgender(&mut self, gender: u8) {
        fn male_to_female(id: i32) -> i32 {
            match id {
                0 => 45,
                1 => 47,
                2 => 48,
                3 => 49,
                4 => 50,
                5 => 51,
                6 => 52,
                7 => 53,
                8 => 54,
                9 => 55,
                18..=25 => 56,
                26 => 61,
                27 | 31 => 63,
                28 => 62,
                29 => 65,
                30 => 64,
                32 => 66,
                33 => 67,
                34 => 68,
                35 => 69,
                36 => 70,
                37 => 71,
                38 => 72,
                39 => 76,
                40 => 75,
                41 => 78,
                42 => 79,
                43 => 80,
                44 => 81,
                _ => -1,
            }
        }
        fn female_to_male(id: i32) -> i32 {
            match id {
                45 | 46 => 0,
                47 => 1,
                48 => 2,
                49 => 3,
                50 => 4,
                51 => 5,
                52 => 6,
                53 => 7,
                54 => 8,
                55 => 9,
                56..=60 => 18,
                61 => 26,
                62 => 27,
                63 => 28,
                64 | 65 => 29,
                66 => 32,
                67 => 33,
                68 => 34,
                69 => 35,
                70 | 73 | 74 | 77 => 36,
                71 => 37,
                72 => 38,
                75 => 40,
                76 => 39,
                78 => 41,
                79 => 42,
                80 => 43,
                81 => 44,
                _ => -1,
            }
        }
        for i in 0..7 {
            if gender == 1 {
                self.player.body[i] = male_to_female(self.player.body[i]);
            } else if i == 1 {
                // Switching to male resets the jaw/beard slot to the default.
                self.player.body[i] = 14;
            } else {
                self.player.body[i] = female_to_male(self.player.body[i]);
            }
        }
        self.player.gender = gender;
    }

    /// Resets the player's camera to its default position and orientation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::cam_reset`
    fn cam_reset(&mut self) {
        self.cam_reset();
    }

    /// Sends a hint arrow pointing at the NPC with the given world index.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::hint_npc`
    fn hint_npc(&mut self, nid: u16) {
        self.hint_npc(nid);
    }

    /// Sends a hint arrow hovering over the given tile.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::hint_tile`
    fn hint_tile(&mut self, offset: u8, x: u16, z: u16, height: u8) {
        self.hint_tile(offset, x, z, height);
    }

    /// Sends a hint arrow pointing at the player with the given index.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::hint_player`
    fn hint_player(&mut self, slot: u16) {
        self.hint_player(slot);
    }

    /// Clears any active hint arrow on the player's client.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::stop_hint`
    fn stop_hint(&mut self) {
        self.stop_hint();
    }

    #[cfg(since_647)]
    fn hint_coord_slot(
        &mut self,
        slot: u8,
        icon: u8,
        offset: u8,
        level: u8,
        x: u16,
        z: u16,
        height: u8,
        range: u16,
        model: Option<u16>,
    ) {
        self.hint_coord_slot(slot, icon, offset, level, x, z, height, range, model);
    }

    #[cfg(since_647)]
    fn hint_stop_slot(&mut self, slot: u8) {
        self.hint_stop_slot(slot);
    }

    #[cfg(since_647)]
    fn hint_npc_slot(&mut self, nid: u16, slot: u8, icon: u8, model: Option<u16>) {
        self.hint_npc_slot(nid, slot, icon, model);
    }

    /// Returns the player's gender (`0` = male, `1` = female).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.gender`
    fn gender(&self) -> u8 {
        self.player.gender
    }

    /// Returns the currently set walk trigger script ID, or `-1` if unset.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Option::unwrap_or`
    fn getwalktrigger(&self) -> i32 {
        self.player.walktrigger.unwrap_or(-1)
    }

    /// Sets the walk trigger script ID that fires when the player moves.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.walktrigger`
    fn walktrigger(&mut self, trigger: i32) {
        self.player.walktrigger = if trigger == -1 { None } else { Some(trigger) };
    }

    /// Returns the player's current overhead head icon bitfield.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** reads `self.player.headicons`
    fn headicons_get(&self) -> u8 {
        self.player.headicons
    }

    /// Sets the player's overhead head icon bitfield.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.headicons`
    fn headicons_set(&mut self, headicons: u8) {
        self.player.headicons = headicons;
    }

    /// Closes any open modal interface.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::close_modal`
    fn if_close(&mut self, clear: bool) -> rs_vm::Result<()> {
        self.close_modal(clear)
    }

    /// Closes any open modal interface unless `com` is the only one open.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::close_modal`
    fn if_close_others(&mut self, com: u16) -> rs_vm::Result<()> {
        let open = [
            self.player.modal_main,
            self.player.modal_chat,
            self.player.modal_side,
        ];
        if open.into_iter().flatten().all(|modal| modal == com) {
            return Ok(());
        }
        self.close_modal(true)
    }

    /// Opens a chatbox interface (e.g. a dialog).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::open_chat_modal`
    fn if_openchat(&mut self, id: u16) {
        self.open_chat_modal(id);
    }

    /// Opens a main-area interface alongside a side-panel interface.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::open_main_side_modal`
    fn if_openmain_side(&mut self, com: u16, side: u16) {
        self.open_main_side_modal(com, side);
    }

    /// Opens a full-screen main-area interface.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::open_main_modal`
    fn if_openmain(&mut self, com: u16) {
        self.open_main_modal(com);
    }

    /// Opens a side-panel interface.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::open_side_modal`
    fn if_openside(&mut self, com: u16) {
        self.open_side_modal(com);
    }

    /// Sets the animation displayed on an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setanim`
    fn if_setanim(&mut self, com: i32, seq: u16) {
        self.if_setanim(com, seq);
    }

    /// Sets the color of an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setcolour`
    fn if_setcolour(&mut self, com: i32, colour: u16) {
        self.if_setcolour(com, colour);
    }

    /// Shows or hides an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_sethide`
    fn if_sethide(&mut self, com: i32, hide: bool) {
        self.if_sethide(com, hide);
    }

    /// Sets the model displayed on an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setmodel`
    fn if_setmodel(&mut self, com: i32, model: u16) {
        self.if_setmodel(com, model);
    }

    /// Sets the graphic (a sprite) displayed on an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setgraphic`
    #[cfg(since_647)]
    fn if_setgraphic(&mut self, com: i32, graphic: u16) {
        self.if_setgraphic(com, graphic);
    }

    /// Sets an NPC chathead model on an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setnpchead`
    fn if_setnpchead(&mut self, com: i32, npc: u16) {
        self.if_setnpchead(com, npc);
    }

    /// Displays an object model on an interface component at the given zoom level.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setobject`
    fn if_setobject(&mut self, com: i32, obj: u16, scale: i32) {
        self.if_setobject(com, obj, scale);
    }

    /// Sets the player's own chathead model on an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setplayerhead`
    fn if_setplayerhead(&mut self, com: i32) {
        self.if_setplayerhead(com);
    }

    /// Sets the position of an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setposition`
    fn if_setposition(&mut self, com: i32, x: u16, y: u16) {
        self.if_setposition(com, x, y);
    }

    /// Sets the scrollbar position on an interface component.
    #[cfg(since_245_2)]
    fn if_setscrollpos(&mut self, com: i32, y: u16) {
        self.if_setscrollpos(com, y);
    }

    /// Opens a modal overlay interface component.
    #[cfg(since_244)]
    fn if_openoverlay(&mut self, com: u16) {
        self.if_openoverlay(com);
    }

    /// Sets (or clears) a right-click option on this player's menu.
    #[cfg(since_254)]
    fn set_player_op(&mut self, op: u8, value: &str, primary: u8) {
        self.set_player_op(op, value, primary);
    }

    /// Sets the client minimap state.
    #[cfg(since_274)]
    fn minimap_toggle(&mut self, minimap_type: u8) {
        self.minimap_toggle(minimap_type);
    }

    /// Sets the player's appearance skill level.
    #[cfg(since_274)]
    fn set_skill_level(&mut self, level: u16) {
        self.set_skill_level(level);
    }

    /// Disguises the player as an npc, or clears the disguise when `npc` is `None`.
    #[cfg(since_254)]
    fn transmogrify(&mut self, npc: Option<u16>) {
        self.transmogrify(npc);
    }

    /// Recolors an interface component model.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_setrecol`
    #[cfg(before_245_2)]
    fn if_setrecol(&mut self, com: u16, src: u16, dst: u16) {
        self.if_setrecol(com, src, dst);
    }

    /// Assigns an interface component to a tab slot.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::set_tab`
    fn if_settab(&mut self, tab: u16, com: u8) {
        self.set_tab(tab, com);
    }

    /// Switches the client's currently selected (active) tab.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_settabactive`
    fn if_settabactive(&mut self, tab: u8) {
        self.if_settabactive(tab);
    }

    /// Sets the text content of an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::if_settext`
    fn if_settext(&mut self, com: i32, text: &str) {
        if text.contains("\\n") && text.contains('@') {
            self.if_settext(com, &preserve_settext_colors(text));
        } else {
            self.if_settext(com, text);
        }
    }

    /// Plays a MIDI jingle (short musical effect) for the player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::midi_jingle`
    #[cfg(rev = "225")]
    fn midi_jingle(&mut self, length: u16, data: &[u8]) {
        self.midi_jingle(length, data);
    }

    /// Plays a MIDI jingle (short musical effect) for the player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::midi_jingle`
    #[cfg(since_244)]
    fn midi_jingle(&mut self, id: u16, delay: u16) {
        self.midi_jingle(id, delay);
    }

    /// Starts playing a MIDI song (background music) for the player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::midi_song`
    #[cfg(rev = "225")]
    fn midi_song(&mut self, name: &str, crc: i32, len: i32) {
        self.midi_song(name, crc, len);
    }

    /// Starts playing a MIDI song (background music) for the player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::midi_song`
    #[cfg(since_244)]
    fn midi_song(&mut self, id: u16) {
        self.midi_song(id);
    }

    /// Makes the player face a specific tile.
    ///
    /// Converts tile coordinates to fine coordinates and sets the face direction
    /// info mask for the next player info update.
    ///
    /// # Side Effects
    ///
    /// Sets `face_x`, `face_z`, and the `FaceCoord` info mask.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `CoordGrid::fine`
    fn facesquare(&mut self, coord: CoordGrid) {
        let fine_x = CoordGrid::fine(coord.x(), 1);
        let fine_z = CoordGrid::fine(coord.z(), 1);
        self.player.info.face_x = Some(fine_x);
        self.player.info.face_z = Some(fine_z);
        self.player.info.masks |= PlayerInfoProt::FaceCoord as u16;
    }

    /// Plays a synthesized sound effect for the player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::synth_sound`
    fn sound_synth(&mut self, synth: u16, loops: u8, delay: u16) {
        self.synth_sound(synth, loops, delay);
    }

    /// Plays a voice-over clip for the player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::speech_sound`
    #[cfg(since_647)]
    fn sound_speech_volume(&mut self, vorbis: u16, loops: u8, delay: u16, volume: u8) {
        self.speech_sound(vorbis, loops, delay, volume);
    }

    /// Stops the player's voice-over clips.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::speech_stop`
    #[cfg(since_647)]
    fn sound_speech_stop(&mut self) {
        self.speech_stop();
    }

    /// Starts the player's client downloading the models loc type `id` has
    /// for `shape`. The standalone packet needs no zone header: the client
    /// never reads a coordinate for it.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::write`
    #[cfg(since_647)]
    fn loc_prefetch(&mut self, id: u16, shape: u8) {
        self.write(LocPrefetch { id, shape });
    }

    /// Clears the player's pending action, if any.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::clear_pending_action`
    fn clearpendingaction(&mut self) -> rs_vm::Result<()> {
        self.clear_pending_action()
    }

    /// Suspends the player's currently running script for a number of ticks.
    ///
    /// # Side Effects
    ///
    /// Sets the player's `delayed` flag and `delayed_until` tick.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.state.delayed` and `delayed_until`
    fn delay(&mut self, delay: u32) {
        self.player.state.delayed = true;
        self.player.state.delayed_until = delay;
    }

    /// Records an arrive-delay so the script resumes after movement completes.
    ///
    /// If the player's last movement happened before the given clock tick, the
    /// delay is skipped (the player has already arrived).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Self::delay`
    fn arrivedelay(&mut self, clock: u32) -> bool {
        if self.player.pathing.last_movement < clock {
            return false;
        }
        self.delay(clock + 1);
        true
    }

    /// Opens a count-dialog input prompt on the client.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::p_countdialog`
    fn countdialog(&mut self) {
        self.p_countdialog();
    }

    /// Sets the player's run mode (`0` = walk, `1` = run) and syncs the change
    /// to the client's run varp so the run orb reflects the new state.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.run`, `ActivePlayer::sync_run`
    fn run(&mut self, run: u8) {
        self.player.run = run == 1;
        self.sync_run();
    }

    /// Forces the player to run for the current movement only.
    ///
    /// Sets the same `temprun` flag a ctrl-click sets. The movement phase
    /// applies it as `MoveSpeed::Run` and clears it once the player stops
    /// moving, so the persistent run toggle and the run orb are left alone.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.temprun`
    fn temprun(&mut self) {
        self.player.temprun = true;
    }

    /// Stops the player's current action and clears interaction state.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::stop_action`
    fn stopaction(&mut self) -> rs_vm::Result<()> {
        self.stop_action()
    }

    /// Instantly moves the player to a coordinate without walking.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::tele_jump`
    fn telejump(&mut self, coord: CoordGrid) {
        self.tele_jump(coord);
    }

    /// Teleports the player to a coordinate, processing zone transitions.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::tele`
    fn teleport(&mut self, coord: CoordGrid) {
        self.tele(coord);
    }

    /// Plays an exact-move animation that linearly interpolates between two positions.
    ///
    /// Sets all exact-move info fields, enables the `ExactMove` info mask, and
    /// updates the player's coordinate to the end position with the tele flag set.
    ///
    /// # Side Effects
    ///
    /// Updates the player's coordinate, sets `tele = true`, and sets the `ExactMove` info mask.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.exactmove_*` fields
    fn exactmove(
        &mut self,
        start_x: u16,
        start_z: u16,
        end_x: u16,
        end_z: u16,
        begin: u16,
        finish: u16,
        direction: u8,
    ) {
        self.teleport(CoordGrid::new(end_x, self.player.pathing.coord.y(), end_z));
        self.player.info.exactmove_start_x = Some(start_x);
        self.player.info.exactmove_start_z = Some(start_z);
        self.player.info.exactmove_end_x = Some(end_x);
        self.player.info.exactmove_end_z = Some(end_z);
        self.player.info.exactmove_begin = Some(begin);
        self.player.info.exactmove_finish = Some(finish);
        self.player.info.exactmove_dir = Some(direction);
        self.player.info.masks |= PlayerInfoProt::ExactMove as u16;
    }

    /// Displays a spot animation (graphic) attached to the player.
    ///
    /// # Side Effects
    ///
    /// Sets the `SpotAnim` info mask for the next player info update.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.info.spotanim*` fields
    fn spotanim(&mut self, spotanim: u16, height: u16, delay: u16) {
        self.player.info.spotanim = Some(spotanim);
        self.player.info.spotanim_height = Some(height);
        self.player.info.spotanim_delay = Some(delay);
        self.player.info.masks |= PlayerInfoProt::SpotAnim as u16;
    }

    /// Enqueues a script for deferred execution on this player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ScriptQueue::add`
    fn queue(
        &mut self,
        script_id: i32,
        priority: QueuePriority,
        delay: u16,
        args: Option<Vec<ScriptArgument>>,
    ) -> rs_vm::Result<()> {
        self.player
            .state
            .queues
            .add(priority, script_id, delay, args)
    }

    #[cfg(since_647)]
    fn queue_from(
        &mut self,
        script_id: i32,
        priority: QueuePriority,
        delay: u16,
        args: Option<Vec<ScriptArgument>>,
        source: Option<u16>,
    ) -> rs_vm::Result<()> {
        self.player
            .state
            .queues
            .add_from(priority, script_id, delay, args, source)
    }

    /// Sets a recurring timer that fires a script at a fixed interval.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `TimerSet::add`
    fn settimer(
        &mut self,
        script_id: i32,
        priority: TimerPriority,
        interval: u16,
        clock: u32,
        args: Option<Vec<ScriptArgument>>,
    ) {
        self.player
            .state
            .timers
            .add(priority, script_id, interval, clock, args)
    }

    /// Clears the player's timer for the given script, regardless of priority
    /// (both the normal and soft lanes are cleared).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ScriptTimer::remove`
    fn cleartimer(&mut self, script_id: i32) {
        self.player.state.timers.remove(script_id);
    }

    /// Removes all queued (normal and weak) scripts matching the given script.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ScriptQueue::remove_any`
    fn clearqueue(&mut self, script_id: i32) {
        self.player.state.queues.remove_any(script_id);
    }

    /// Counts the player's queued (normal and weak) scripts matching the script.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ScriptQueue::count_by_script`
    fn getqueue(&self, script_id: i32) -> i32 {
        self.player.state.queues.count_by_script(script_id)
    }

    /// Returns the clock of the player's timer for the given script.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ScriptTimer::get`
    fn gettimer(&self, script_id: i32) -> i32 {
        match self.player.state.timers.get(script_id) {
            Some(timer) => timer.clock as i32,
            None => -1,
        }
    }

    fn cam_lookat(
        &mut self,
        x: u16,
        z: u16,
        height: u16,
        rate: u8,
        rate2: u8,
    ) -> rs_vm::Result<()> {
        self.player
            .cam_queue
            .add(CamKind::LookAt, x, z, height, rate, rate2)
    }

    fn cam_moveto(
        &mut self,
        x: u16,
        z: u16,
        height: u16,
        rate: u8,
        rate2: u8,
    ) -> rs_vm::Result<()> {
        self.player
            .cam_queue
            .add(CamKind::MoveTo, x, z, height, rate, rate2)
    }

    fn cam_shake(&mut self, direction: u8, jitter: u8, amplitude: u8, frequency: u8) {
        self.cam_shake(direction, jitter, amplitude, frequency)
    }

    /// Returns an immutable reference to a player inventory.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `FxHashMap::get`
    fn get_inv(&mut self, id: u16) -> Option<&Inventory> {
        self.player.invs.get(&id)
    }
    /// Returns a mutable reference to a player inventory.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `FxHashMap::get_mut`
    fn get_inv_mut(&mut self, id: u16) -> Option<&mut Inventory> {
        self.player.invs.get_mut(&id)
    }

    /// Returns mutable references to two distinct player inventories simultaneously.
    ///
    /// # Panics
    ///
    /// Panics if `a == b` (the two keys must differ).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `FxHashMap::get_mut` (via raw pointers for split borrows)
    fn get_inv_pair_mut(&mut self, a: u16, b: u16) -> Option<(&mut Inventory, &mut Inventory)> {
        assert_ne!(a, b, "get_inv_pair_mut: keys must differ");
        let pa = self.player.invs.get_mut(&a)? as *mut Inventory;
        let pb = self.player.invs.get_mut(&b)? as *mut Inventory;
        Some(unsafe { (&mut *pa, &mut *pb) })
    }

    /// Retrieves a player inventory, creating it if it does not yet exist.
    ///
    /// If the inventory type defines `stockobj` and `stockcount` in the cache, the newly
    /// created inventory is pre-populated with those entries. Stock defaults are not
    /// exclusive to shared inventories, so player-scoped invs honor them too.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `FxHashMap::entry`, `Inventory::with_stack_mode`, `Inventory::set`
    fn get_or_create_inv(&mut self, id: u16, size: usize, stack_mode: StackMode) -> &mut Inventory {
        self.player.invs.entry(id).or_insert_with(|| {
            let mut inv = Inventory::with_stack_mode(size, stack_mode);
            if let Some(inv_type) = engine().invs().get_by_id(id) {
                if let Some(stockobj) = &inv_type.stockobj {
                    inv.stockobj = stockobj.clone();
                }
                if let (Some(stockobj), Some(stockcount)) =
                    (&inv_type.stockobj, &inv_type.stockcount)
                {
                    for i in 0..stockobj.len() {
                        inv.set(i as u16, stockobj[i], stockcount[i] as u32);
                    }
                }
            }
            inv
        })
    }

    /// Registers an inventory for automatic transmission to the client via an interface component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `FxHashMap::entry`
    fn add_inv_transmit(&mut self, inv_id: u16, com: ComId) {
        self.player
            .inv_transmits
            .entry(inv_id)
            .or_default()
            .push(com);
    }

    /// Checks whether a given interface component has an inventory transmit binding.
    ///
    /// # Returns
    ///
    /// `Some(inv_id)` if the component is bound, `None` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** iterates `self.player.inv_transmits`
    fn has_inv_transmit(&self, com: ComId) -> Option<u16> {
        self.player
            .inv_transmits
            .iter()
            .find(|(_, coms)| coms.contains(&com))
            .map(|(id, _)| *id)
    }

    /// Removes all transmit bindings for the given interface component.
    ///
    /// # Side Effects
    ///
    /// Clears the bindings and sends an `UpdateInvStopTransmit` packet.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::clear_inv_transmits`, `ActivePlayer::update_inv_stop_transmit`
    fn clear_inv_transmits(&mut self, com: ComId) {
        self.player.clear_inv_transmits(com);
        self.update_inv_stop_transmit(com);
    }

    /// Registers a cross-player inventory listener on a component.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait (`INVOTHER_TRANSMIT`)
    /// **Calls:** `HashMap::insert`
    fn add_inv_other_transmit(&mut self, com: ComId, inv_id: u16, uid: i32) {
        self.player.inv_other_transmits.insert(com, (uid, inv_id));
    }

    /// Returns the cross-player inventory listener bound to a component, if any.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait (`INVOTHER_TRANSMIT`)
    /// **Calls:** `HashMap::get`
    fn has_inv_other_transmit(&self, com: ComId) -> Option<(i32, u16)> {
        self.player.inv_other_transmits.get(&com).copied()
    }

    /// Appends a waypoint to the player's movement queue.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `Pathing::queue_waypoint`
    fn queue_waypoint(&mut self, coord: CoordGrid) {
        self.player.pathing.queue_waypoint(coord);
    }

    /// Starts the player walking toward the given tile using pathfinding.
    ///
    /// Computes a path from the player's current position to the destination
    /// using `rsmod::find_path` with normal collision and queues the waypoints.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `rsmod::find_path`, `Pathing::queue_waypoints`
    fn walk(&mut self, coord: CoordGrid) {
        self.request_path(PathRequest {
            y: self.player.pathing.coord.y(),
            src_x: self.player.pathing.coord.x(),
            src_z: self.player.pathing.coord.z(),
            dest_x: coord.x(),
            dest_z: coord.z(),
            src_size: 1,
            dest_width: 1,
            dest_length: 1,
            angle: 0,
            shape: -1,
            move_near: true,
            block_access_flags: 0,
            max_waypoints: 25,
            collision: CollisionType::Normal,
        });
    }

    /// Clears all pending movement waypoints.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `ActivePlayer::clear_waypoints`
    fn clear_waypoints(&mut self) {
        self.clear_waypoints();
    }

    /// Sets the player's interaction target to a location.
    ///
    /// Builds an `InteractionTarget::Loc` from `loc` and the given `width`
    /// and `length`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::set_interaction`
    fn set_interaction_loc(&mut self, loc: LocRef, width: u8, length: u8, op: u8) {
        self.player.set_interaction(
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

    /// Sets the player's interaction target to an NPC.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::set_interaction`
    fn set_interaction_npc(&mut self, nid: u16, op: u8) {
        let target = InteractionTarget::Npc { nid };
        self.player.set_interaction(target, op, false);
    }

    /// Sets the player's interaction target to a ground object.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::set_interaction`
    fn set_interaction_obj(&mut self, coord: CoordGrid, id: u16, serial: ObjSerial, op: u8) {
        let target = InteractionTarget::Obj { coord, id, serial };
        self.player.set_interaction(target, op, false);
    }

    /// Sets the player's interaction target to another player.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `EnginePlayer::set_interaction`
    fn set_interaction_player(&mut self, pid: u16, op: u8) {
        let target = InteractionTarget::Player { pid };
        self.player.set_interaction(target, op, false);
    }

    /// Records a spell/use component as the subject of the player's current interaction.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.interaction.target_subject_com`
    fn set_interaction_spell(&mut self, com: ComId) {
        self.player.interaction.target_subject_com = Some(com);
    }

    /// Checks whether the player is within operable distance of a location.
    ///
    /// Returns `false` immediately if the player is on a different level.
    /// Otherwise delegates to `rsmod::reached` for collision-aware reachability.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** `rsmod::reached`
    fn in_operable_distance_loc(
        &self,
        coord: CoordGrid,
        width: u8,
        length: u8,
        shape: LocShape,
        angle: LocAngle,
        forceapproach: u8,
    ) -> bool {
        if coord.y() != self.player.pathing.coord.y() {
            return false;
        }
        rsmod::reached(
            self.player.pathing.coord.y(),
            self.player.pathing.coord.x(),
            self.player.pathing.coord.z(),
            coord.x(),
            coord.z(),
            width,
            length,
            1,
            angle as u8,
            shape as i8,
            forceapproach,
        )
    }

    /// Sets the approach range for the player's current interaction.
    ///
    /// # Side Effects
    ///
    /// Sets `ap_range` and marks `ap_range_called` for the interaction processor.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptPlayer` trait
    /// **Calls:** sets `self.player.interaction.ap_range` and `ap_range_called`
    fn aprange(&mut self, range: i32) {
        self.player.interaction.ap_range = Some(range as u16);
        self.player.interaction.ap_range_called = true;
    }
}

/// Reapplies the active colour tag at the start of each `\n`-delimited line so
/// colours carry across line breaks, inserting `@bla@` after a `@str@` reset.
///
/// The delimiter is the literal two-character sequence `\n` (backslash + `n`), not a newline byte.
fn preserve_settext_colors(text: &str) -> String {
    let mut lines: Vec<String> = text.split("\\n").map(str::to_string).collect();
    let mut saved_col: Option<String> = None;

    for (i, line) in lines.iter_mut().enumerate() {
        // apply the color carried over from the previous line
        if i > 0 {
            if let Some(col) = saved_col.clone() {
                if !line.is_empty() {
                    if let Some(str_index) = line.find("@str@") {
                        let after = str_index + 5;
                        if line.as_bytes().get(after..after + 5) != Some(&b"@bla@"[..]) {
                            line.insert_str(after, "@bla@");
                        }
                        saved_col = None;
                    } else {
                        *line = format!("{col}{line}");
                    }
                }
            }
        }

        // scan this line for the last color tag, tracking @str@/@bla@ resets
        let bytes = line.as_bytes();
        let mut j = 0;
        while j + 4 < bytes.len() {
            if bytes[j] == b'@' && bytes[j + 4] == b'@' {
                if &bytes[j + 1..j + 4] == b"str" {
                    saved_col = None;
                    if bytes.get(j + 5..j + 10) == Some(&b"@bla@"[..]) {
                        j += 10;
                        continue;
                    }
                } else {
                    saved_col = Some(line[j..j + 5].to_string());
                }
                j += 5;
                continue;
            }
            j += 1;
        }
    }

    lines.join("\\n")
}
