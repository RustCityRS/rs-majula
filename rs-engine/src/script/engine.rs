use crate::active_npc::ActiveNpc;
#[cfg(since_647)]
use crate::clients::client_db::DbRequest;
use crate::core::{Engine, ObjDelayedRequest};
use crate::counters::{self, COUNTERS};
#[cfg(since_647)]
use crate::region::REGION_X;
use crate::script::errors::{Context, Subject};
use rs_entity::StaffModLevel;
use rs_entity::{EntityLifeTime, Loc, ObjSerial};
use rs_entity::{NpcUid, PlayerUid};
use rs_grid::CoordGrid;
use rs_inv::{Inventory, Item, StackMode};
use rs_pack::cache::VarValue;
use rs_pack::cache::category::CategoryTypeProvider;
use rs_pack::cache::dbrow::DbRowTypeProvider;
use rs_pack::cache::dbtable::{DbTableIndex, DbTableTypeProvider};
use rs_pack::cache::r#enum::EnumTypeProvider;
use rs_pack::cache::flo::FloTypeProvider;
use rs_pack::cache::font::FontTypeProvider;
use rs_pack::cache::hunt::HuntTypeProvider;
use rs_pack::cache::idk::IdkTypeProvider;
use rs_pack::cache::r#if::IfTypeProvider;
use rs_pack::cache::inv::InvTypeProvider;
use rs_pack::cache::loc::LocTypeProvider;
use rs_pack::cache::mesanim::MesAnimTypeProvider;
#[cfg(rev = "225")]
use rs_pack::cache::midi::MidiType;
use rs_pack::cache::npc::NpcTypeProvider;
use rs_pack::cache::obj::ObjTypeProvider;
use rs_pack::cache::param::ParamTypeProvider;
use rs_pack::cache::script::Script;
use rs_pack::cache::seq::SeqTypeProvider;
use rs_pack::cache::spotanim::SpotAnimTypeProvider;
use rs_pack::cache::r#struct::StructTypeProvider;
#[cfg(since_254)]
use rs_pack::cache::varbit::VarbitTypeProvider;
#[cfg(since_647)]
use rs_pack::cache::varcon::VarConTypeProvider;
use rs_pack::cache::varn::VarnTypeProvider;
#[cfg(since_647)]
use rs_pack::cache::varobj::VarObjTypeProvider;
use rs_pack::cache::varp::VarPlayerTypeProvider;
use rs_pack::cache::vars::VarsTypeProvider;
use rs_pack::cache::wordenc::WordEncProvider;
use rs_pack::types::{LocAngle, LocShape};
#[cfg(since_647)]
use rs_protocol::network::game::server::loc_merge::LocMerge;
#[cfg(since_647)]
use rs_protocol::network::game::server::map_projanim_halfsq::MapProjAnimHalfsq;
#[cfg(since_289)]
use rs_protocol::network::game::server::sound_area::SoundArea;
#[cfg(since_647)]
use rs_protocol::network::game::server::sound_vorbis_area::SoundVorbisArea;
#[cfg(since_647)]
use rs_protocol::network::game::server::text_coord::TextCoord;
use rs_util::random::JavaRandom;
use rs_var::VarSet;
use rs_vm::ScriptError;
#[cfg(since_647)]
use rs_vm::engine::ProjAnimHalfsq;
use rs_vm::engine::{ScriptEngine, ScriptFault, ScriptNpc, ScriptPlayer};
use rs_vm::state::*;
#[cfg(since_647)]
use rs_vm::subject::ScriptSubject;
#[cfg(since_647)]
use rs_vm::trigger::ServerTriggerType;
#[cfg(since_647)]
use rs_vm::{ControllerUid, RegionUid};
#[cfg(since_647)]
use rs_zone::ZoneEventType;
use rs_zone::ZoneMessage;
use rsmod::rsmod::flag::collision_flag::CollisionFlag;
use rsmod::rsmod::flag::zone_flag::ZoneFlag;
use std::sync::Arc;

impl ScriptEngine for Engine {
    /// Returns the current game clock tick.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** reads `self.clock`
    fn clock(&self) -> u32 {
        self.clock
    }

    /// Returns this cycle's wall-clock timestamp in Unix-epoch milliseconds.
    ///
    /// # Call Stack
    ///
    /// **Called by:** the `DATE_MINUTES` / `DATE_RUNEDAY` ops via `ScriptEngine`
    /// **Calls:** reads `self.cycle_date_ms`
    fn date_millis(&self) -> i64 {
        self.cycle_date_ms
    }

    /// Returns the experience multiplier of the engine.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** reads `self.multi_xp`
    fn multi_experience(&self) -> u8 {
        self.multi_xp
    }

    fn objs(&self) -> &ObjTypeProvider {
        &self.cache.objs
    }

    fn invs(&self) -> &InvTypeProvider {
        &self.cache.invs
    }

    fn varps(&self) -> &VarPlayerTypeProvider {
        &self.cache.varps
    }

    #[cfg(since_254)]
    fn varbits(&self) -> &VarbitTypeProvider {
        &self.cache.varbits
    }

    #[cfg(since_647)]
    fn varobjs(&self) -> &VarObjTypeProvider {
        &self.cache.varobjs
    }

    #[cfg(since_647)]
    fn varcons(&self) -> &VarConTypeProvider {
        &self.cache.varcons
    }

    #[cfg(since_647)]
    fn varconbits(&self) -> &VarbitTypeProvider {
        &self.cache.varconbits
    }

    fn dbrows(&self) -> &DbRowTypeProvider {
        &self.cache.dbrows
    }

    fn dbtables(&self) -> &DbTableTypeProvider {
        &self.cache.dbtables
    }

    fn dbindex(&self) -> &DbTableIndex {
        &self.cache.db_index
    }

    fn enums(&self) -> &EnumTypeProvider {
        &self.cache.enums
    }

    fn flos(&self) -> &FloTypeProvider {
        &self.cache.flos
    }

    fn hunts(&self) -> &HuntTypeProvider {
        &self.cache.hunts
    }

    fn idks(&self) -> &IdkTypeProvider {
        &self.cache.idks
    }

    fn locs(&self) -> &LocTypeProvider {
        &self.cache.locs
    }

    fn mesanims(&self) -> &MesAnimTypeProvider {
        &self.cache.mesanims
    }

    fn npcs(&self) -> &NpcTypeProvider {
        &self.cache.npcs
    }

    fn params(&self) -> &ParamTypeProvider {
        &self.cache.params
    }

    fn seqs(&self) -> &SeqTypeProvider {
        &self.cache.seqs
    }

    fn spotanims(&self) -> &SpotAnimTypeProvider {
        &self.cache.spotanims
    }

    fn structs(&self) -> &StructTypeProvider {
        &self.cache.structs
    }

    fn varns(&self) -> &VarnTypeProvider {
        &self.cache.varns
    }

    fn varss(&self) -> &VarsTypeProvider {
        &self.cache.varss
    }

    fn fonts(&self) -> &FontTypeProvider {
        &self.cache.fonts
    }

    fn categories(&self) -> &CategoryTypeProvider {
        &self.cache.categories
    }

    fn interfaces(&self) -> &IfTypeProvider {
        &self.cache.interfaces
    }

    fn wordenc(&self) -> &WordEncProvider {
        &self.cache.wordenc
    }

    #[cfg(since_254)]
    fn midi_tick_length(&self, id: i32) -> Option<u16> {
        self.cache
            .midi_tick_lengths
            .get(id as usize)
            .copied()
            .flatten()
    }

    #[cfg(rev = "225")]
    fn jingle_by_name(&self, name: &str) -> Option<&MidiType> {
        self.cache.jingles.get_by_name(name)
    }

    #[cfg(rev = "225")]
    fn song_by_name(&self, name: &str) -> Option<&MidiType> {
        self.cache.songs.get_by_name(name)
    }

    #[cfg(all(since_244, before_254))]
    fn midi_id(&self, name: &str) -> Option<u16> {
        self.cache.midi_ids.get(name).copied()
    }

    /// Looks up a compiled script by its numeric identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - The script identifier to look up.
    ///
    /// # Returns
    ///
    /// `Some(&Arc<Script>)` if the script exists, `None` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ScriptProvider::get_by_id`
    fn get_script(&self, id: i32) -> Option<&Arc<Script>> {
        self.scripts.get_by_id(id)
    }

    /// Returns the number of players currently online.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    fn playercount(&self) -> usize {
        self.player_list.count()
    }

    /// Retrieves a shared (global) inventory, creating it with stock defaults if it does not exist.
    ///
    /// If the inventory type defines `stockobj` and `stockcount` in the cache, the newly
    /// created inventory is pre-populated with those entries.
    ///
    /// # Arguments
    ///
    /// * `id` - The shared inventory identifier.
    /// * `size` - The number of slots to allocate if the inventory is created.
    /// * `stack_mode` - The stacking behavior to use if the inventory is created.
    ///
    /// # Returns
    ///
    /// A mutable reference to the shared inventory.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `FxHashMap::entry`, `Inventory::with_stack_mode`, `Inventory::set`
    fn get_shared_inv(&mut self, id: u16, size: usize, stack_mode: StackMode) -> &mut Inventory {
        self.invs.entry(id).or_insert_with(|| {
            let mut inv = Inventory::with_stack_mode(size, stack_mode);
            if let Some(inv_type) = self.cache.invs.get_by_id(id) {
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

    /// Retrieves an existing shared (global) inventory without creating one.
    ///
    /// # Arguments
    ///
    /// * `id` - The shared inventory identifier.
    ///
    /// # Returns
    ///
    /// `Some(&mut Inventory)` if the inventory exists, `None` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `FxHashMap::get_mut`
    fn get_shared_inv_mut(&mut self, id: u16) -> Option<&mut Inventory> {
        self.invs.get_mut(&id)
    }

    /// Looks up a player by their slot index (immutable).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    fn get_player(&self, pid: u16) -> Option<&impl ScriptPlayer> {
        self.player_list.get(pid)
    }

    fn get_player_mut(&mut self, pid: u16) -> Option<&mut impl ScriptPlayer> {
        self.player_list.get_mut(pid)
    }

    fn get_npc(&self, nid: u16) -> Option<&impl ScriptNpc> {
        self.npc_list.get(nid)
    }

    fn get_npc_mut(&mut self, nid: u16) -> Option<&mut impl ScriptNpc> {
        self.npc_list.get_mut(nid)
    }

    /// Finds a player by their Base37-encoded username.
    ///
    /// # Arguments
    ///
    /// * `user37` - The Base37-encoded username to search for.
    ///
    /// # Returns
    ///
    /// `Some(PlayerUid)` if a matching player is online, `None` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::find_pid_by_user37`
    fn find_player_by_user37(&self, user37: u64) -> Option<PlayerUid> {
        self.find_pid_by_user37(user37)
            .and_then(|pid| self.player_list.get(pid).map(|p| p.player.uid))
    }

    #[cfg(since_647)]
    fn run_opheld(&mut self, uid: PlayerUid, op: i32, obj_id: u16, slot: i32) {
        // Rev-647 has no opheld beyond 5; inventory ops 6-10 are handled inline
        // by the if_button{op} content, not a per-item opheld trigger.
        let trigger = match op {
            1 => ServerTriggerType::OpHeld1,
            2 => ServerTriggerType::OpHeld2,
            3 => ServerTriggerType::OpHeld3,
            4 => ServerTriggerType::OpHeld4,
            5 => ServerTriggerType::OpHeld5,
            _ => return,
        };
        let category = self
            .cache
            .objs
            .get_by_id(obj_id)
            .and_then(|o| o.category)
            .map(|c| c as i32)
            .unwrap_or(-1);
        // Bind the held-item context the opheld script reads (last_item/last_slot).
        // A caller on a modal (the stats screen's backpack) already holds
        // protected access: its handler ran it protected. The item's op runs
        // within that access, as it does from the backpack tab, whose caller
        // took it with p_finduid; skipped as "busy" otherwise.
        let held = match self.player_list.get_mut(uid.pid()) {
            Some(active) => {
                active.player.last_item = obj_id as i32;
                active.player.last_slot = slot;
                active.player.state.protect
            }
            None => false,
        };
        if let Err(e) = self.run_script_by_trigger(
            (trigger, Some(obj_id as i32), Some(category)),
            Some(ScriptSubject::Player(uid)),
            None,
            Some(true),
            Some(held),
            None,
        ) && !matches!(e, ScriptError::TriggerNotFound(_))
        {
            tracing::warn!(error = %e, op, obj_id, "[P_OPHELD] opheld script error");
        }
        // The nested run clears the flag it found set; the caller still holds it.
        if held && let Some(active) = self.player_list.get_mut(uid.pid()) {
            active.player.state.protect = true;
        }
    }

    #[cfg(since_647)]
    fn request_set_display_name(&mut self, uid: PlayerUid, name: String) {
        if let Some(tx) = &self.db_tx {
            let _ = tx.send(DbRequest::SetDisplayName {
                user37: uid.username37(),
                display_name: name,
            });
        }
    }

    #[cfg(since_647)]
    fn request_more_suggestions(&mut self, uid: PlayerUid) {
        if let Some(tx) = &self.db_tx {
            let _ = tx.send(DbRequest::MoreSuggestions {
                user37: uid.username37(),
            });
        }
    }

    /// Appends every NPC in the zone containing `coord` to `out`.
    ///
    /// Iterates the zone's NPC slot list and resolves each to an `NpcRef`
    /// containing the slot index, current type ID, and coordinate.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ZoneMap::zone`
    fn extend_zone_npcs(&self, coord: CoordGrid, out: &mut Vec<NpcRef>) {
        let Some(zone) = self.zones.zone(coord.x(), coord.y(), coord.z()) else {
            return;
        };
        out.extend(zone.npcs.iter().filter_map(|&nid| {
            let active = self.npc_list.get(nid)?;
            Some(NpcRef {
                nid,
                id: active.npc.uid.id(),
                coord: active.npc.pathing.coord,
            })
        }));
    }

    /// Returns the packed coordinates of all players in the specified zone.
    ///
    /// # Arguments
    ///
    /// * `coord` - A coordinate in the zone.
    ///
    /// # Returns
    ///
    /// A `Vec<CoordGrid>` with the coordinate of each player in the zone.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ZoneMap::zone`
    fn get_zone_player_coords(&self, coord: CoordGrid) -> Vec<CoordGrid> {
        let Some(zone) = self.zones.zone(coord.x(), coord.y(), coord.z()) else {
            return Vec::new();
        };
        zone.players
            .iter()
            .filter_map(|&pid| Some(self.player_list.get(pid)?.player.pathing.coord))
            .collect()
    }

    /// Returns the player slot indices for all players in the specified zone.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ZoneMap::zone`
    fn get_zone_player_pids(&self, coord: CoordGrid) -> &[u16] {
        self.zones
            .zone(coord.x(), coord.y(), coord.z())
            .map(|z| z.players.as_slice())
            .unwrap_or(&[])
    }

    /// Spawns a new NPC at the given coordinate with a despawn lifecycle.
    ///
    /// Creates an `ActiveNpc` with `EntityLifeTime::Despawn`, allocates a free NPC slot,
    /// and returns the unique identifier.
    ///
    /// # Arguments
    ///
    /// * `coord` - The packed coordinate where the NPC should appear.
    /// * `id` - The NPC type identifier.
    /// * `duration` - Ticks until the NPC despawns and fires `ai_despawn`. Like
    ///   the reference engine's `setLifeCycle`, anything below 1 counts as 1,
    ///   except that rev 647 keeps an npc added in region space with 0 until
    ///   its region is freed, as Jagex's KBD instance adds its dragon.
    ///
    /// # Returns
    ///
    /// `Some(NpcUid)` with the unique identifier, or `None` if no free slot is available.
    ///
    /// # Side Effects
    ///
    /// Adds the NPC to the world, collision map, and zone tracking.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ActiveNpc::new`, `Engine::add_npc`
    fn add_npc_spawned(
        &mut self,
        coord: CoordGrid,
        id: u16,
        duration: i32,
    ) -> rs_vm::Result<Option<NpcUid>> {
        let npc_type = self
            .cache
            .npcs
            .get_by_id(id)
            .ok_or(ScriptError::NpcNotFound(id as i32))?;
        let size = npc_type.size.max(1) as u16;
        let (max_x, max_z) = (coord.x() + size - 1, coord.z() + size - 1);
        for zone_x in CoordGrid::zone(coord.x())..=CoordGrid::zone(max_x) {
            for zone_z in CoordGrid::zone(coord.z())..=CoordGrid::zone(max_z) {
                if !rsmod::is_zone_allocated(zone_x << 3, zone_z << 3, coord.y()) {
                    return Err(ScriptError::Runtime(format!(
                        "Zone does not exist at coord: {:?}",
                        CoordGrid::new(zone_x << 3, coord.y(), zone_z << 3)
                    )));
                }
            }
        }
        let vars = VarSet::new(self.cache.varns.types.iter().map(|v| v.var_type));
        let mut active = ActiveNpc::new(id, 0, coord, npc_type.size, vars, self.cache);
        active.npc.lifecycle = EntityLifeTime::Despawn;
        active.npc.lifecycle_at = Some(duration.max(1) as u32);
        #[cfg(since_647)]
        if duration == 0 && coord.x() >= REGION_X {
            active.npc.lifecycle_at = None;
        }
        Ok(self.add_npc(active))
    }

    /// Removes an NPC from the world by its slot index.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::deactivate_npc`
    fn remove_npc(&mut self, nid: u16) {
        self.deactivate_npc(nid);
    }

    /// Adds a ground object at the given coordinate with a despawn lifecycle.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::add_item_obj`
    fn add_obj(
        &mut self,
        coord: CoordGrid,
        item: Item,
        receiver: Option<PlayerUid>,
        duration: u64,
    ) -> rs_vm::Result<ObjSerial> {
        if !rsmod::is_zone_allocated(coord.x(), coord.z(), coord.y()) {
            return Err(ScriptError::Runtime(format!(
                "Zone does not exist at coord: {:?}",
                coord
            )));
        }
        Ok(self.add_item_obj(coord, item, receiver, duration))
    }

    /// Enqueues a ground object to be spawned after a delay.
    ///
    /// The request is placed on the `obj_delayed_queue` and processed during the world phase.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `LinkList::add_tail`
    fn add_obj_delayed(
        &mut self,
        coord: CoordGrid,
        item: Item,
        receiver: Option<PlayerUid>,
        duration: u64,
        delay: i32,
    ) {
        self.obj_delayed_queue.add_tail(ObjDelayedRequest {
            coord,
            item,
            receiver,
            duration,
            delay,
        });
    }

    /// Removes a ground object from the given coordinate.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::remove_obj` (inherent)
    fn remove_obj(
        &mut self,
        coord: CoordGrid,
        id: u16,
        serial: ObjSerial,
        duration: u64,
    ) -> Option<u32> {
        self.remove_obj(coord, id, serial, duration)
    }

    /// The count of a ground object that can be counted or taken.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::valid_obj`
    fn valid_obj_count(
        &self,
        coord: CoordGrid,
        id: u16,
        serial: ObjSerial,
        receiver37: Option<u64>,
    ) -> Option<u32> {
        let zone = self.zones.zone(coord.x(), coord.y(), coord.z())?;
        let idx = zone.valid_obj(coord.x(), coord.z(), id, serial, receiver37)?;
        Some(zone.objs[idx].count())
    }

    /// Who a ground object was dropped for.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::obj_dropped_for`
    fn obj_dropped_for(&self, coord: CoordGrid, id: u16, serial: ObjSerial) -> Option<u64> {
        self.zones
            .zone(coord.x(), coord.y(), coord.z())?
            .obj_dropped_for(coord.x(), coord.z(), id, serial)
    }

    #[cfg(since_647)]
    fn obj_var(&self, coord: CoordGrid, id: u16, serial: ObjSerial) -> i32 {
        self.zones
            .zone(coord.x(), coord.y(), coord.z())
            .map_or(0, |zone| zone.obj_var(coord.x(), coord.z(), id, serial))
    }

    #[cfg(since_647)]
    fn set_obj_var(&mut self, coord: CoordGrid, id: u16, serial: ObjSerial, var: i32) -> bool {
        self.zones.zone(coord.x(), coord.y(), coord.z()).is_some()
            && self
                .zones
                .zone_mut(coord.x(), coord.y(), coord.z())
                .set_obj_var(coord.x(), coord.z(), id, serial, var)
    }

    /// Whether a staff level is kept apart from other players.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::isolates`
    fn isolates_staff(&self, staffmodlevel: u8) -> bool {
        self.isolates(StaffModLevel::from_u8(staffmodlevel))
    }

    /// Finds a ground object at the given coordinate by type ID and optional
    /// receiver ownership.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ZoneMap::find_obj`
    fn find_obj(&self, coord: CoordGrid, id: u16, receiver37: Option<u64>) -> Option<ObjRef> {
        let (zone, idx) = self
            .zones
            .find_obj(coord.x(), coord.y(), coord.z(), id, receiver37)?;
        let obj = &zone.objs[idx];
        Some(ObjRef {
            coord: obj.world_coord(zone.coord),
            id: obj.id(),
            serial: obj.serial(),
        })
    }

    /// Returns the valid ground objects in the specified zone, any player's,
    /// newest first.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ZoneMap::zone`
    fn get_zone_objs(&self, coord: CoordGrid) -> Vec<ObjRef> {
        let Some(zone) = self.zones.zone(coord.x(), coord.y(), coord.z()) else {
            return Vec::new();
        };
        zone.objs
            .iter()
            .rev()
            .filter(|obj| obj.is_valid())
            .map(|obj| ObjRef {
                coord: obj.world_coord(zone.coord),
                id: obj.id(),
                serial: obj.serial(),
            })
            .collect()
    }

    /// Returns all visible locations in the specified zone.
    ///
    /// Filters out hidden (removed/reverted) locations and maps each to a `LocRef`.
    ///
    /// # Arguments
    ///
    /// * `coord` - A coordinate in the zone.
    ///
    /// # Returns
    ///
    /// A `Vec<LocRef>` of location references in the zone.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait, `Engine::npc_hunt_locs`
    /// **Calls:** `ZoneMap::zone`, `Loc::visible`, `loc_ref`
    fn get_zone_locs(&self, coord: CoordGrid) -> Vec<LocRef> {
        let Some(zone) = self.zones.zone(coord.x(), coord.y(), coord.z()) else {
            return Vec::new();
        };
        zone.locs
            .iter()
            .filter(|loc| loc.visible())
            .map(|loc| loc_ref(loc.world_coord(zone.coord), loc))
            .collect()
    }

    /// Finds a specific location at the given tile by type ID.
    ///
    /// # Arguments
    ///
    /// * `coord` - The tile to search.
    /// * `id` - The location type identifier.
    ///
    /// # Returns
    ///
    /// `Some(LocRef)` if a matching location exists, `None` otherwise.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `ZoneMap::find_loc`
    fn find_loc(&self, coord: CoordGrid, id: u16) -> Option<LocRef> {
        let (zone, idx) = self.zones.find_loc(coord.x(), coord.y(), coord.z(), id)?;
        let loc = &zone.locs[idx];
        Some(loc_ref(loc.world_coord(zone.coord), loc))
    }

    /// Adds a new location or changes the one already on the shape's layer at
    /// the given coordinate.
    ///
    /// Looks the loc type up, checks its footprint's zones exist and delegates to
    /// the inherent `Engine::add_or_change_loc`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::add_or_change_loc` (inherent)
    fn add_or_change_loc(
        &mut self,
        coord: CoordGrid,
        id: u16,
        shape: LocShape,
        angle: LocAngle,
        duration: u64,
    ) -> rs_vm::Result<LocRef> {
        let (width, length, blockwalk, blockrange) = self.placeable_loc_type(coord, id, angle)?;
        let loc = self.add_or_change_loc(
            coord, id, shape, angle, width, length, blockwalk, blockrange, duration,
        );
        Ok(loc_ref(coord, &loc))
    }

    /// Changes the loc `loc` names to type `id`, keeping its shape and angle.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::change_loc` (inherent)
    fn change_loc(&mut self, loc: LocRef, id: u16, duration: u64) -> rs_vm::Result<Option<LocRef>> {
        let live = self.live_loc(loc);
        let (width, length, blockwalk, blockrange) =
            self.placeable_loc_type(loc.coord, id, live.angle)?;
        let changed = self.change_loc(
            loc.coord, loc.layer, loc.serial, id, width, length, blockwalk, blockrange, duration,
        );
        Ok(changed.map(|changed| loc_ref(loc.coord, &changed)))
    }

    /// Returns `loc` as the loc it names is now, or unchanged once it is gone.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::loc_index`
    fn live_loc(&self, loc: LocRef) -> LocRef {
        let (x, y, z) = (loc.coord.x(), loc.coord.y(), loc.coord.z());
        self.zones
            .zone(x, y, z)
            .and_then(|zone| {
                let idx = zone.loc_index(x, z, loc.layer, loc.serial)?;
                Some(loc_ref(loc.coord, &zone.locs[idx]))
            })
            .unwrap_or(loc)
    }

    /// Merges a location so that it is only visible to one player within a bounded area.
    ///
    /// Finds the existing location at the given coordinate and layer, then queues
    /// a `LocMerge` zone message with relative boundary offsets.
    ///
    /// # Side Effects
    ///
    /// Queues a zone message and marks the zone as dirty.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::merge_loc`, `Engine::track_zone`
    #[cfg(before_647)]
    fn merge_loc(
        &mut self,
        coord: CoordGrid,
        shape: LocShape,
        angle: LocAngle,
        id: u16,
        start: u16,
        end: u16,
        pid: u16,
        south: u16,
        east: u16,
        north: u16,
        west: u16,
    ) {
        let layer = shape.layer();
        let (x, y, z) = (coord.x(), coord.y(), coord.z());

        let existing = self
            .zones
            .zone_mut(x, y, z)
            .locs
            .iter()
            .position(|l| l.is_at(x, z) && l.layer() == layer);

        if let Some(idx) = existing {
            let coord = CoordGrid::packed_zone_coord(x, z);
            let shape_angle = ((shape as u8) << 2) | (angle as u8 & 3);
            let message =
                ZoneMessage::LocMerge(rs_protocol::network::game::server::loc_merge::LocMerge {
                    coord,
                    shape_angle,
                    id,
                    start,
                    end,
                    pid,
                    east: east.wrapping_sub(x) as i8,
                    south: south.wrapping_sub(z) as i8,
                    west: west.wrapping_sub(x) as i8,
                    north: north.wrapping_sub(z) as i8,
                });
            self.zones.zone_mut(x, y, z).merge_loc(idx, message);
            self.track_zone(x, y, z);
        }
    }

    /// Rev 647: does nothing, because 647 has no way to merge a loc into a
    /// player model. 225/289 send LOC_MERGE, and the client keeps the merged loc on the player and
    /// splices it into the player's model while drawing. The 647 client
    /// has none of that: `PlayerEntity` holds no loc model and builds its
    /// model from appearance alone, no server prot carries the merge, and
    /// `ZoneProt` ends at 14 -- a sub-prot 15 inside an enclosed zone update
    /// indexes past `ZoneProt.toSerialArray()` and throws mid-packet.
    /// `void-634` has no merge either. So `p_locmerge` does nothing here,
    /// and the obstacle plays as the exact move it travels with.
    #[cfg(since_647)]
    fn merge_loc(
        &mut self,
        _coord: CoordGrid,
        _shape: LocShape,
        _angle: LocAngle,
        _id: u16,
        _start: u16,
        _end: u16,
        _pid: u16,
        _south: u16,
        _east: u16,
        _north: u16,
        _west: u16,
    ) {
    }

    /// Removes the loc `loc` names, if it is still in the world.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::remove_loc` (inherent)
    fn remove_loc(&mut self, loc: LocRef, duration: u64) {
        self.remove_loc(loc.coord, loc.layer, loc.serial, duration);
    }

    /// Plays a sequence animation on the loc `loc` names.
    ///
    /// The reference engine animates the loc whether or not it is shown, so a
    /// removed map loc still gets the event; a loc a script added that is gone
    /// does not.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::loc_index`, `Zone::anim_loc`, `Engine::track_zone`
    fn anim_loc(&mut self, loc: LocRef, seq: u16) {
        let (x, y, z) = (loc.coord.x(), loc.coord.y(), loc.coord.z());
        let zone = self.zones.zone_mut(x, y, z);
        if let Some(idx) = zone.loc_index(x, z, loc.layer, loc.serial) {
            zone.anim_loc(idx, seq);
            self.track_zone(x, y, z);
        }
    }

    /// Starts every client observing `coord`'s zone downloading the models
    /// loc type `id` has for `shape`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::prefetch_loc`, `Engine::track_zone`
    #[cfg(since_647)]
    fn loc_prefetchall(&mut self, coord: CoordGrid, id: u16, shape: u8) {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        self.zones.zone_mut(x, y, z).prefetch_loc(id, shape);
        self.track_zone(x, y, z);
    }

    /// Merges a look into the loc `loc` names, if it is still shown. The
    /// client looks the lists' lengths up by the loc's type and the shape its
    /// models are filed under.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::loc_index`, `Zone::merge_loc`, `Engine::track_zone`
    #[cfg(since_647)]
    fn loc_merge(
        &mut self,
        loc: LocRef,
        models: Option<Box<[u16]>>,
        recol: Option<Box<[u16]>>,
        retex: Option<Box<[u16]>>,
    ) {
        let (x, y, z) = (loc.coord.x(), loc.coord.y(), loc.coord.z());
        let zone = self.zones.zone_mut(x, y, z);
        let Some(idx) = zone.loc_index(x, z, loc.layer, loc.serial) else {
            return;
        };
        let live = zone.locs[idx];
        if !live.visible() {
            return;
        }
        let merge = LocMerge {
            coord: live.packed_zone_coord(),
            id: live.id(),
            shape_angle: (live.shape().model_shape() as u8) << 2 | live.angle() as u8,
            models,
            recol,
            retex,
        };
        zone.merge_loc(idx, merge);
        self.track_zone(x, y, z);
    }

    /// Creates a projectile animation between two map positions.
    ///
    /// Computes relative deltas from source to destination and queues a
    /// `MapProjAnim` zone message. A script's height `h` means `h * 4` units
    /// above the ground. Clients from 254 multiply the byte by 4 themselves,
    /// so they're sent `h`; earlier ones take the byte as it is, so they're
    /// sent `h * 4`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::map_proj_anim`, `Engine::track_zone`
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
    ) {
        let coord = CoordGrid::packed_zone_coord(x, z);
        let dx = (dst_x as i32 - x as i32) as i8;
        let dz = (dst_z as i32 - z as i32) as i8;
        #[cfg(before_254)]
        let (src_height, dst_height) = (src_height << 2, dst_height << 2);
        #[cfg(since_647)]
        let arc = u16::from(arc);
        let message = ZoneMessage::MapProjAnim(
            rs_protocol::network::game::server::map_projanim::MapProjAnim {
                coord,
                dx,
                dz,
                target,
                spotanim: id,
                src_height,
                dst_height,
                start_delay: start,
                end_delay: end,
                peak,
                arc,
            },
        );
        self.zones.zone_mut(x, y, z).map_proj_anim(message);
        self.track_zone(x, y, z);
    }

    /// Plays a spot animation (graphic) on the map at the given tile.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::anim_map`, `Engine::track_zone`
    fn anim_map(&mut self, y: u8, x: u16, z: u16, spotanim: u16, height: u8, delay: u16) {
        let coord = CoordGrid::packed_zone_coord(x, z);
        let message = ZoneMessage::MapAnim(rs_protocol::network::game::server::map_anim::MapAnim {
            coord,
            spotanim,
            height,
            delay,
        });
        self.zones.zone_mut(x, y, z).anim_map(message);
        self.track_zone(x, y, z);
    }

    /// Plays a sound effect anchored to a tile, broadcast to every player
    /// observing the zone.
    ///
    /// `range` (audible radius, `0..=15`) is packed into the high nibble of the
    /// wire `info` byte and `loops` (`0..=7`) into the low bits, matching the
    /// client decode (`range = info >> 4 & 0xF`, `loops = info & 0x7`).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::sound_area`, `Engine::track_zone`
    #[cfg(since_289)]
    fn sound_area(&mut self, y: u8, x: u16, z: u16, sound: u16, range: u8, loops: u8, delay: u8) {
        let coord = CoordGrid::packed_zone_coord(x, z);
        let info = ((range & 0xF) << 4) | (loops & 0x7);
        self.zones
            .zone_mut(x, y, z)
            .sound_area(ZoneMessage::SoundArea(SoundArea {
                coord,
                sound,
                info,
                delay,
                #[cfg(since_647)]
                volume: 255,
                #[cfg(since_647)]
                speed: 256,
            }));
        self.track_zone(x, y, z);
    }

    /// Plays a vorbis sample anchored to a tile, as `sound_area` plays a synth.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::sound_area`, `Engine::track_zone`
    #[cfg(since_647)]
    fn sound_vorbis_area(&mut self, coord: CoordGrid, sound: u16, range: u8, loops: u8, delay: u8) {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        let message = ZoneMessage::SoundVorbisArea(SoundVorbisArea {
            coord: CoordGrid::packed_zone_coord(x, z),
            sound,
            info: ((range & 0xF) << 4) | (loops & 0x7),
            delay,
            volume: 255,
            speed: 256,
        });
        self.zones.zone_mut(x, y, z).sound_area(message);
        self.track_zone(x, y, z);
    }

    /// Shows text above a tile to every player observing its zone.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::queue_event`, `Engine::track_zone`
    #[cfg(since_647)]
    fn text_coord(&mut self, coord: CoordGrid, text: &str, colour: i32, height: u8, duration: u16) {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        let message = ZoneMessage::TextCoord(TextCoord {
            coord: CoordGrid::packed_zone_coord(x, z),
            duration,
            height,
            colour,
            text: text.to_owned(),
        });
        self.zones
            .zone_mut(x, y, z)
            .queue_event(None, ZoneEventType::Enclosed, None, message);
        self.track_zone(x, y, z);
    }

    /// Queues a half-square projectile in the zone holding its start, whose
    /// 16 by 16 half squares the coord byte indexes.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Zone::map_proj_anim`, `Engine::track_zone`
    #[cfg(since_647)]
    fn map_proj_anim_halfsq(&mut self, projectile: ProjAnimHalfsq) {
        let (x, y, z) = (projectile.x >> 1, projectile.level, projectile.z >> 1);
        let message = ZoneMessage::MapProjAnimHalfsq(MapProjAnimHalfsq {
            coord: ((projectile.x & 0xF) << 4 | projectile.z & 0xF) as u8,
            ground: false,
            wearpos: projectile.wearpos,
            dx: projectile.dx,
            dz: projectile.dz,
            source: projectile.source,
            target: projectile.target,
            spotanim: projectile.spotanim,
            src_height: projectile.src_height,
            dst_height: projectile.dst_height,
            start_delay: projectile.start,
            end_delay: projectile.end,
            peak: projectile.peak,
            arc: projectile.arc,
        });
        self.zones.zone_mut(x, y, z).map_proj_anim(message);
        self.track_zone(x, y, z);
    }

    /// Checks whether adding a location at the given coordinate is unsafe, i.e.
    /// an active loc already occupies the tile. Invisible wall locs are skipped.
    ///
    /// # Arguments
    ///
    /// * `coord` - The coordinate to test.
    ///
    /// # Returns
    ///
    /// `true` if the coordinate is occupied by an active location.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `Engine::loc_occupies`
    fn locaddunsafe(&self, coord: CoordGrid) -> bool {
        self.loc_occupies(coord, true)
    }

    /// Checks whether an active loc occupies the given coordinate. Unlike
    /// [`locaddunsafe`](Self::locaddunsafe), invisible wall locs still count.
    ///
    /// # Arguments
    ///
    /// * `coord` - The coordinate to test.
    ///
    /// # Returns
    ///
    /// `true` if an active loc covers the coordinate.
    #[cfg(since_274)]
    fn map_loc(&self, coord: CoordGrid) -> bool {
        self.loc_occupies(coord, false)
    }

    /// Returns a mutable reference to the engine's random number generator.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** reads `self.random`
    fn random(&mut self) -> &mut JavaRandom {
        &mut self.random
    }

    /// Indicates whether the server is running in members mode.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** reads `self.members`
    fn members(&self) -> bool {
        self.members
    }

    /// Indicates if there is "line of sight" between these coords.
    ///
    /// # Call Stack
    ///
    /// **Calls:** reads `rsmod::has_line_of_sight()`
    fn lineofsight(&self, src: CoordGrid, dst: CoordGrid) -> rs_vm::Result<bool> {
        let (sx, sy, sz) = (src.x(), src.y(), src.z());
        let (dx, dy, dz) = (dst.x(), dst.y(), dst.z());
        if !rsmod::is_zone_allocated(sx, sz, sy) {
            return Err(ScriptError::Runtime(format!(
                "Zone does not exist at coord: {:?}",
                src
            )));
        }
        if !rsmod::is_zone_allocated(dx, dz, dy) {
            return Err(ScriptError::Runtime(format!(
                "Zone does not exist at coord: {:?}",
                dst
            )));
        }
        if sy != dy {
            return Ok(false);
        }
        if !self.members && !rsmod::is_zone_flagged(dx, dz, dy, ZoneFlag::Free as u8) {
            return Ok(false);
        }
        Ok(rsmod::has_line_of_sight(sy, sx, sz, dx, dz, 1, 1, 1, 1, 0))
    }

    /// Indicates if there is "line of walk" between these coords.
    ///
    /// # Call Stack
    ///
    /// **Calls:** reads `rsmod::has_line_of_walk()`
    fn lineofwalk(&self, src: CoordGrid, dst: CoordGrid) -> rs_vm::Result<bool> {
        let (sx, sy, sz) = (src.x(), src.y(), src.z());
        let (dx, dy, dz) = (dst.x(), dst.y(), dst.z());
        if !rsmod::is_zone_allocated(sx, sz, sy) {
            return Err(ScriptError::Runtime(format!(
                "Zone does not exist at coord: {:?}",
                src
            )));
        }
        if !rsmod::is_zone_allocated(dx, dz, dy) {
            return Err(ScriptError::Runtime(format!(
                "Zone does not exist at coord: {:?}",
                dst
            )));
        }
        if sy != dy {
            return Ok(false);
        }
        if !self.members && !rsmod::is_zone_flagged(dx, dz, dy, ZoneFlag::Free as u8) {
            return Ok(false);
        }
        Ok(rsmod::has_line_of_walk(sy, sx, sz, dx, dz, 1, 1, 1, 1, 0))
    }

    /// Indicates if this coord has a `CollisionFlag::WalkBlocked` on it. On a free
    /// world, members tiles count as blocked.
    ///
    /// # Call Stack
    ///
    /// **Calls:** reads `rsmod::is_flagged()`
    fn map_blocked(&self, coord: CoordGrid) -> rs_vm::Result<bool> {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        if !rsmod::is_zone_allocated(x, z, y) {
            return Err(ScriptError::Runtime(format!(
                "Zone does not exist at coord: {:?}",
                coord
            )));
        }
        if !self.members && !rsmod::is_zone_flagged(x, z, y, ZoneFlag::Free as u8) {
            return Ok(true);
        }
        Ok(rsmod::is_flagged(
            x,
            z,
            y,
            CollisionFlag::WalkBlocked as u32,
        ))
    }

    /// Indicates if this coord has a `CollisionFlag::Roof` collision flag on it.
    ///
    /// # Call Stack
    ///
    /// **Calls:** reads `rsmod::is_flagged()`
    fn map_indoors(&self, coord: CoordGrid) -> rs_vm::Result<bool> {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        if !rsmod::is_zone_allocated(x, z, y) {
            return Err(ScriptError::Runtime(format!(
                "Zone does not exist at coord: {:?}",
                coord
            )));
        }
        Ok(rsmod::is_flagged(x, z, y, CollisionFlag::Roof as u32))
    }

    /// Indicates if this coord has a `ZoneFlag::Free` collision flag on it.
    ///
    /// # Call Stack
    ///
    /// **Calls:** reads `rsmod::is_zone_flagged()`
    fn map_f2p(&self, coord: CoordGrid) -> rs_vm::Result<bool> {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        if !rsmod::is_zone_allocated(x, z, y) {
            return Err(ScriptError::Runtime(format!(
                "Zone does not exist at coord: {:?}",
                coord
            )));
        }
        if !self.members && !rsmod::is_zone_flagged(x, z, y, ZoneFlag::Free as u8) {
            return Ok(false);
        }
        Ok(true)
    }

    /// Indicates if this coord is multi-combat, see [`Engine::multiway`].
    fn map_multiway(&self, coord: CoordGrid) -> rs_vm::Result<bool> {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        if !rsmod::is_zone_allocated(x, z, y) {
            return Err(ScriptError::Runtime(format!(
                "Zone does not exist at coord: {:?}",
                coord
            )));
        }
        Ok(self.multiway(coord))
    }

    #[cfg(since_647)]
    fn inarea(&self, area: u16, coord: CoordGrid) -> bool {
        let coord = self.area_tile(coord);
        area != u16::MAX
            && self
                .cache
                .area_map
                .get(coord.y(), coord.x(), coord.z())
                .contains(&area)
    }

    #[cfg(since_647)]
    fn region_find(&self, coord: CoordGrid) -> Option<RegionUid> {
        self.regions
            .find(coord.x(), coord.z())
            .map(|region| region.uid)
    }

    #[cfg(since_647)]
    fn region_exists(&self, uid: RegionUid) -> bool {
        self.regions.get(uid).is_some()
    }

    #[cfg(since_647)]
    fn region_coord(&self, uid: RegionUid, level: i32, x: i32, z: i32) -> rs_vm::Result<CoordGrid> {
        Engine::region_coord(self, uid, level, x, z)
    }

    #[cfg(since_647)]
    fn region_create(&mut self, levels: i32, east: i32, north: i32) -> rs_vm::Result<RegionUid> {
        Engine::region_create(self, levels, east, north)
    }

    #[cfg(since_647)]
    fn region_set(
        &mut self,
        uid: RegionUid,
        level: i32,
        east: i32,
        north: i32,
        source: CoordGrid,
        rotation: i32,
    ) -> rs_vm::Result<()> {
        Engine::region_set(self, uid, level, east, north, source, rotation)
    }

    #[cfg(since_647)]
    fn region_set_exit(&mut self, uid: RegionUid, exit: CoordGrid) -> rs_vm::Result<()> {
        Engine::region_set_exit(self, uid, exit)
    }

    #[cfg(since_647)]
    fn region_allows(&self, from: CoordGrid, to: CoordGrid, selected: Option<RegionUid>) -> bool {
        Engine::region_allows(self, from, to, selected)
    }

    #[cfg(since_647)]
    fn controller_add(
        &mut self,
        coord: CoordGrid,
        id: u16,
        duration: i32,
    ) -> rs_vm::Result<ControllerUid> {
        Engine::controller_add(self, coord, id, duration)
    }

    #[cfg(since_647)]
    fn controller_coord(&self, uid: ControllerUid) -> rs_vm::Result<CoordGrid> {
        Ok(self.controller(uid)?.coord)
    }

    #[cfg(since_647)]
    fn controller_del(&mut self, uid: ControllerUid) {
        self.controllers.remove(uid);
    }

    #[cfg(since_647)]
    fn controller_findexact(&self, coord: CoordGrid, id: u16) -> Option<ControllerUid> {
        self.controllers.find_exact(coord, id)
    }

    #[cfg(since_647)]
    fn controller_queue(
        &mut self,
        uid: ControllerUid,
        arg: i32,
        delay: u16,
        source: Option<u16>,
    ) -> rs_vm::Result<()> {
        Engine::controller_queue(self, uid, arg, delay, source)
    }

    #[cfg(since_647)]
    fn controller_queue_script(
        &mut self,
        uid: ControllerUid,
        script: i32,
        args: Vec<ScriptArgument>,
        delay: u16,
        source: Option<u16>,
    ) -> rs_vm::Result<()> {
        Engine::controller_queue_script(self, uid, script, args, delay, source)
    }

    #[cfg(since_647)]
    fn controller_settimer(&mut self, uid: ControllerUid, interval: i32) -> rs_vm::Result<()> {
        Engine::controller_settimer(self, uid, interval)
    }

    #[cfg(since_647)]
    fn controller_var(&self, uid: ControllerUid, id: u16) -> rs_vm::Result<VarValue> {
        Engine::controller_var(self, uid, id)
    }

    #[cfg(since_647)]
    fn set_controller_var(
        &mut self,
        uid: ControllerUid,
        id: u16,
        value: VarValue,
    ) -> rs_vm::Result<()> {
        Engine::set_controller_var(self, uid, id, value)
    }

    /// Reads a shared variable (vars) by its definition ID.
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `VarSet::get`
    fn get_var(&self, id: u16) -> VarValue {
        self.vars.get(id).clone()
    }

    /// Writes a shared variable (vars).
    ///
    /// # Call Stack
    ///
    /// **Called by:** VM ops via `ScriptEngine` trait
    /// **Calls:** `VarSet::set`
    fn set_var(&mut self, id: u16, value: VarValue) {
        self.vars.set(id, value.clone());
    }

    /// Gathers a script error for staff (`ScriptErrors`), and marks it in
    /// the recording of the player it ran on.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `rs_vm::vm::report_error` via `ScriptEngine`
    /// **Calls:** `Watch::script_error`, `ScriptErrors::add`
    fn script_error(&mut self, fault: ScriptFault) {
        counters::add(&COUNTERS.script_errors, 1);
        let clock = self.clock;
        let player = fault.player.and_then(|uid| {
            self.player_list
                .get_mut(uid.pid())
                .filter(|a| a.player.uid.packed() == uid.packed())
        });
        let context = if let Some(active) = player {
            active
                .watch
                .script_error(clock, &fault.script, &fault.message);
            let coord = active.player.pathing.coord;
            let replay_id = active.watch.recording_id();
            Context {
                subject: if active.player.bot {
                    Subject::World
                } else {
                    Subject::Player(active.player.uid.username37())
                },
                npc: None,
                x: coord.x() as i16,
                z: coord.z() as i16,
                y: i16::from(coord.y()),
                replay_id,
                replay_ms: replay_id.map(|_| active.watch.ms() as i32),
            }
        } else if let Some(active) = fault.npc.and_then(|uid| {
            self.npc_list
                .get(uid.nid())
                .filter(|a| a.npc.uid.packed() == uid.packed())
        }) {
            let coord = active.coord();
            let id = active.npc.uid.id();
            Context {
                subject: Subject::Npc(id),
                npc: Some(
                    <Self as ScriptEngine>::npcs(self)
                        .get_by_id(id)
                        .and_then(|t| t.debugname())
                        .map_or_else(|| format!("npc_{id}"), str::to_owned),
                ),
                x: coord.x() as i16,
                z: coord.z() as i16,
                y: i16::from(coord.y()),
                replay_id: None,
                replay_ms: None,
            }
        } else {
            Context {
                subject: Subject::World,
                npc: None,
                x: 0,
                z: 0,
                y: 0,
                replay_id: None,
                replay_ms: None,
            }
        };
        // Without a database there's nowhere to keep them.
        if self.db_tx.is_some() {
            self.script_errors.add(&fault, context, self.cycle_date_ms);
        }
    }
}

/// Builds the [`LocRef`] for the stored loc `loc` at `coord`, as it is now.
fn loc_ref(coord: CoordGrid, loc: &Loc) -> LocRef {
    LocRef {
        coord,
        id: loc.id(),
        shape: loc.shape(),
        angle: loc.angle(),
        layer: loc.layer(),
        serial: loc.serial(),
    }
}
