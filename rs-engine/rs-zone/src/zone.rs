use crate::zone_event::ZoneEvent;
use crate::zone_event_type::ZoneEventType;
use crate::zone_message::ZoneMessage;
use rs_entity::{EntityLifeTime, Loc, Obj};
use rs_grid::ZoneCoordGrid;
use rs_io::Packet;
use rs_pack::types::LocLayer;
use rs_protocol::network::game::server::loc_add_change::LocAddChange;
use rs_protocol::network::game::server::loc_anim::LocAnim;
use rs_protocol::network::game::server::loc_del::LocDel;
use rs_protocol::network::game::server::obj_add::ObjAdd;
use rs_protocol::network::game::server::obj_del::ObjDel;
use rs_protocol::network::game::server::obj_reveal::ObjReveal;
use rustc_hash::FxHashMap;

/// An 8x8 tile region that manages all entities and events within its bounds.
///
/// Each `Zone` tracks the players, NPCs, locs (map objects/scenery), and objs
/// (ground items) occupying its 8x8 tile area. It also maintains a queue of
/// [`ZoneEvent`]s representing state changes that must be communicated to
/// observing clients.
///
/// # Event model
///
/// When entities are added, removed, or modified, the zone queues events that
/// are consumed during the zone output phase of each engine tick. Events are
/// categorized as:
///
/// - **Enclosed** -- broadcast to all players observing the zone.
/// - **Follows** -- sent only to a specific player (e.g., privately visible objs).
///
/// Enclosed events are batched into a shared byte buffer via [`compute_shared`](Self::compute_shared)
/// for efficient delivery.
///
/// # Lifecycle
///
/// Entities with [`EntityLifeTime::Despawn`] are removed from storage when deleted.
/// Entities with [`EntityLifeTime::Respawn`] remain in storage but become invisible
/// until their respawn clock expires.
#[derive(Debug)]
pub struct Zone {
    pub coord: ZoneCoordGrid,
    pub players: Vec<u16>,
    pub npcs: Vec<u16>,
    pub locs: Vec<Loc>,
    pub objs: Vec<Obj>,
    receivers: FxHashMap<u64, u64>,
    events: Vec<ZoneEvent>,
    shared: Vec<u8>,
}

impl Zone {
    /// Maximum number of objs (respawn + despawn) retained in a zone. Once this
    /// total is reached, the next add evicts the oldest *despawn* obj to make room;
    /// respawn (static) objs count toward the total but are never evicted.
    /// `((8 * 8(<<3)) << 1) - 1 = 127`.
    pub const MAX_OBJS: usize = ((8 << 3) << 1) - 1;

    /// Maximum number of locs (respawn + despawn) retained in a zone. Once this
    /// total is reached, the next add evicts the oldest *despawn* loc to make room;
    /// respawn (static) locs count toward the total but are never evicted.
    /// `((8 * 8(<<3)) << 2) - 1 = 255`.
    pub const MAX_LOCS: usize = ((8 << 3) << 2) - 1;

    /// Creates a new, empty zone at the given coordinate.
    ///
    /// # Arguments
    ///
    /// * `coord` -- The zone's position in the zone grid.
    ///
    /// # Returns
    ///
    /// A `Zone` with empty entity lists and no queued events.
    ///
    /// **Called by:** [`ZoneMap::zone_mut`](crate::zone_map::ZoneMap::zone_mut) when
    /// lazily creating zones on first access.
    #[inline]
    pub fn new(coord: ZoneCoordGrid) -> Self {
        Self {
            coord,
            players: Vec::new(),
            npcs: Vec::new(),
            locs: Vec::new(),
            objs: Vec::new(),
            receivers: FxHashMap::default(),
            events: Vec::new(),
            shared: Vec::new(),
        }
    }

    /// Registers a player as present in this zone.
    ///
    /// No-op if the player is already registered. Uses a linear search since
    /// the player count per zone is expected to be small.
    ///
    /// # Arguments
    ///
    /// * `player` -- The player's PID (player index).
    ///
    /// # Side Effects
    ///
    /// Appends `player` to `self.players` if not already present.
    ///
    /// **Called by:** `ActivePlayer::update_zones` when a player enters a zone.
    #[inline]
    pub fn add_player(&mut self, player: u16) {
        if !self.players.contains(&player) {
            self.players.push(player);
        }
    }

    /// Unregisters a player from this zone.
    ///
    /// Uses `swap_remove` for O(1) removal. No-op if the player is not present.
    ///
    /// # Arguments
    ///
    /// * `player` -- The player's PID to remove.
    ///
    /// # Side Effects
    ///
    /// Removes `player` from `self.players` via swap-remove (order is not preserved).
    ///
    /// **Called by:** `ActivePlayer::update_zones` when a player leaves a zone.
    #[inline]
    pub fn remove_player(&mut self, player: u16) {
        if let Some(i) = self.players.iter().position(|&p| p == player) {
            self.players.swap_remove(i);
        }
    }

    /// Registers an NPC as present in this zone.
    ///
    /// No-op if the NPC is already registered.
    ///
    /// # Arguments
    ///
    /// * `npc` -- The NPC's index.
    ///
    /// # Side Effects
    ///
    /// Appends `npc` to `self.npcs` if not already present.
    ///
    /// **Called by:** `Engine` methods when an NPC enters this zone.
    #[inline]
    pub fn add_npc(&mut self, npc: u16) {
        if !self.npcs.contains(&npc) {
            self.npcs.push(npc);
        }
    }

    /// Unregisters an NPC from this zone.
    ///
    /// Uses `swap_remove` for O(1) removal. No-op if the NPC is not present.
    ///
    /// # Arguments
    ///
    /// * `npc` -- The NPC's index to remove.
    ///
    /// # Side Effects
    ///
    /// Removes `npc` from `self.npcs` via swap-remove.
    ///
    /// **Called by:** `Engine` methods when an NPC leaves this zone.
    #[inline]
    pub fn remove_npc(&mut self, npc: u16) {
        if let Some(i) = self.npcs.iter().position(|&n| n == npc) {
            self.npcs.swap_remove(i);
        }
    }

    /// Adds a static (map-defined) loc to this zone during world loading.
    ///
    /// Static locs are loaded from the game cache at startup. Unlike dynamic locs
    /// added via [`add_loc`](Self::add_loc), no zone event is queued because
    /// players receive static locs through the map loading protocol.
    ///
    /// # Arguments
    ///
    /// * `loc` -- The loc to add.
    ///
    /// # Side Effects
    ///
    /// Appends `loc` to `self.locs`.
    ///
    /// **Called by:** `GameMap::load` during server startup.
    #[inline]
    pub fn add_static_loc(&mut self, loc: Loc) {
        self.locs.push(loc);
    }

    /// Adds a static (map-defined) obj to this zone during world loading.
    ///
    /// Static objs are loaded from the game cache at startup. Unlike dynamic objs
    /// added via [`add_obj`](Self::add_obj), no zone event is queued because
    /// players receive static objs through the zone rebuild protocol.
    ///
    /// # Arguments
    ///
    /// * `obj` -- The obj to add.
    ///
    /// # Side Effects
    ///
    /// Assigns the obj an instance slot (so its [`oid`](Obj::oid) is zone-unique)
    /// and appends it to `self.objs`.
    ///
    /// The obj is dropped if its `(tile, id)` slot space is already full (256 objs),
    /// since admitting it would alias another obj's oid. This is unreachable with
    /// legitimate map data (it would require 256 identical static objs on one tile).
    ///
    /// **Called by:** `GameMap::load` during server startup.
    #[inline]
    pub fn add_static_obj(&mut self, mut obj: Obj) {
        let Some(slot) = self.assign_obj_slot(obj.local_x(), obj.local_z(), obj.id()) else {
            return;
        };
        obj.set_slot(slot);
        self.objs.push(obj);
    }

    // ---- zone events ----

    /// Appends a zone event to the pending event queue.
    ///
    /// Events remain in the queue until the next call to [`reset`](Self::reset),
    /// which occurs at the start of each engine tick. Enclosed events are
    /// serialized into a shared buffer via [`compute_shared`](Self::compute_shared).
    ///
    /// # Arguments
    ///
    /// * `id` -- Optional entity identifier (oid or lid) used for event cancellation
    ///   via [`clear_queued_events`](Self::clear_queued_events). `None` for non-entity
    ///   events like map animations.
    /// * `event_type` -- Whether this event is broadcast (`Enclosed`) or targeted (`Follows`).
    /// * `receiver37` -- The target player UID (lower 37 bits) for `Follows` events,
    ///   or `None` for `Enclosed` events.
    /// * `message` -- The protocol message payload.
    ///
    /// # Side Effects
    ///
    /// Pushes a new [`ZoneEvent`] onto `self.events`.
    ///
    /// **Called by:** `add_loc`, `change_loc`, `remove_loc`, `respawn_loc`, `anim_loc`,
    /// `merge_loc`, `add_obj`, `reveal_obj`, `remove_obj_at`, `respawn_obj`,
    /// `anim_map`, `map_proj_anim`.
    pub fn queue_event(
        &mut self,
        id: Option<u64>,
        event_type: ZoneEventType,
        receiver37: Option<u64>,
        message: ZoneMessage,
    ) {
        self.events.push(ZoneEvent {
            event_type,
            receiver37,
            message,
            id,
        });
    }

    /// Removes all queued events matching the given entity identifier.
    ///
    /// Used to cancel stale events when an entity is removed or replaced before
    /// the events have been dispatched. For example, when a loc is removed, any
    /// previously queued `LocAddChange` event for that loc is cancelled.
    ///
    /// # Arguments
    ///
    /// * `id` -- The entity identifier (oid or lid) whose events should be removed.
    ///
    /// # Side Effects
    ///
    /// Retains only events whose `id` does not match `Some(id)`.
    ///
    /// **Called by:** `remove_loc`, `reveal_obj`, `remove_obj_at`.
    pub fn clear_queued_events(&mut self, id: u64) {
        self.events.retain(|e| e.id != Some(id));
    }

    /// Pre-serializes all enclosed events into a shared byte buffer.
    ///
    /// Iterates over all queued events with type [`ZoneEventType::Enclosed`],
    /// encodes each into a contiguous byte buffer, and stores the result in
    /// `self.shared`. This buffer can then be sent to every player observing
    /// the zone without re-serializing per player.
    ///
    /// If there are no enclosed events, `self.shared` is left empty.
    ///
    /// # Side Effects
    ///
    /// Fills `self.shared` with the serialized enclosed-event bytes (reusing its
    /// allocation from previous ticks), or empties it if there are none.
    ///
    /// **Called by:** `Engine::compute_zone_shared` during the zone phase.
    ///
    /// **Calls:** [`ZoneMessage::sizeof_zone`], [`ZoneMessage::encode_zone`].
    pub fn compute_shared(&mut self) {
        let len: usize = self
            .events
            .iter()
            .filter(|e| e.event_type == ZoneEventType::Enclosed)
            .map(|e| e.message.sizeof_zone())
            .sum();

        if len == 0 {
            self.shared.clear();
            return;
        }

        let mut buf = std::mem::take(&mut self.shared);
        buf.resize(len, 0);
        let mut packet = Packet::from(buf);
        for event in &self.events {
            if event.event_type != ZoneEventType::Enclosed {
                continue;
            }
            event.message.encode_zone(&mut packet);
        }
        self.shared = packet.data;
    }

    /// Returns the pre-serialized enclosed event bytes, if any were computed.
    ///
    /// # Returns
    ///
    /// `Some(&[u8])` containing the serialized enclosed events, or `None` if
    /// [`compute_shared`](Self::compute_shared) was not called or there were no
    /// enclosed events.
    ///
    /// **Called by:** `ActivePlayer::update_zones` to append shared zone data
    /// to each observing player's output buffer.
    pub fn shared_bytes(&self) -> Option<&[u8]> {
        if self.shared.is_empty() {
            None
        } else {
            Some(&self.shared)
        }
    }

    /// Returns `true` if this zone has any pending follows (player-targeted) events.
    ///
    /// Used as a fast check to skip per-player event filtering when there are
    /// no follows events.
    ///
    /// **Called by:** `ActivePlayer::update_zones` to decide whether to iterate
    /// follows events for each player.
    pub fn has_follows_events(&self) -> bool {
        self.events
            .iter()
            .any(|e| e.event_type == ZoneEventType::Follows)
    }

    /// Returns an iterator over all pending follows (player-targeted) events.
    ///
    /// # Returns
    ///
    /// An iterator yielding references to [`ZoneEvent`]s with type
    /// [`ZoneEventType::Follows`].
    ///
    /// **Called by:** [`visible_follows_events`](Self::visible_follows_events).
    pub fn follows_events(&self) -> impl Iterator<Item = &ZoneEvent> {
        self.events
            .iter()
            .filter(|e| e.event_type == ZoneEventType::Follows)
    }

    /// Returns an iterator over all pending enclosed (broadcast) events.
    ///
    /// # Returns
    ///
    /// An iterator yielding references to [`ZoneEvent`]s with type
    /// [`ZoneEventType::Enclosed`].
    pub fn enclosed_events(&self) -> impl Iterator<Item = &ZoneEvent> {
        self.events
            .iter()
            .filter(|e| e.event_type == ZoneEventType::Enclosed)
    }

    /// Returns `true` if this zone has any pending events of any type.
    ///
    /// Used as a fast check to determine whether the zone needs processing
    /// during the zone output phase.
    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }

    // ---- zone output ----

    /// Returns an iterator over all objs in this zone that are visible to a given player.
    ///
    /// Visibility is determined by two criteria:
    /// 1. The obj must be visible at the given clock (i.e., not expired for despawn objs,
    ///    or past its respawn time for respawn objs).
    /// 2. The obj must either have no receiver (visible to all) or its receiver must
    ///    match `user37` (privately visible to that player only).
    ///
    /// # Arguments
    ///
    /// * `user37` -- The lower-37-bit UID of the player requesting visibility.
    /// * `clock` -- The current engine tick clock for expiry checks.
    ///
    /// # Returns
    ///
    /// An iterator over visible [`Obj`] references.
    ///
    /// **Called by:** `ActivePlayer::update_zones` to send ground item data when
    /// a player enters a new zone.
    pub fn visible_objs(&self, user37: u64, clock: u32) -> impl Iterator<Item = &Obj> {
        let receivers = &self.receivers;
        self.objs
            .iter()
            .filter(move |obj| obj.visible(clock) && matches_receiver(receivers, obj, Some(user37)))
    }

    /// Returns an iterator over follows event messages visible to a given player.
    ///
    /// Filters follows events to include only those that either have no receiver
    /// (visible to all) or match the given `user37`.
    ///
    /// # Arguments
    ///
    /// * `user37` -- The lower-37-bit UID of the player requesting events.
    ///
    /// # Returns
    ///
    /// An iterator over [`ZoneMessage`] references for matching follows events.
    ///
    /// **Called by:** `ActivePlayer::update_zones` to send player-specific zone
    /// updates (e.g., privately visible obj adds/deletes).
    ///
    /// **Calls:** [`follows_events`](Self::follows_events).
    pub fn visible_follows_events(&self, user37: u64) -> impl Iterator<Item = &ZoneMessage> {
        self.follows_events()
            .filter(move |e| e.receiver37.is_none_or(|r| r == user37))
            .map(|e| &e.message)
    }

    /// Clears all queued events and the shared byte buffer for the next tick.
    ///
    /// Called at the start of each engine tick after events from the previous
    /// tick have been dispatched to all observing players.
    ///
    /// # Side Effects
    ///
    /// - Clears `self.shared` (retaining its allocation for reuse).
    /// - Clears `self.events`.
    ///
    /// **Called by:** The engine's tick reset phase.
    pub fn reset(&mut self) {
        self.shared.clear();
        self.events.clear();
    }

    // ---- loc management ----

    /// Dynamically adds a loc to this zone and queues an enclosed `LocAddChange` event.
    ///
    /// The loc is reverted to its base state and its clock is cleared before insertion.
    /// Only locs with [`EntityLifeTime::Despawn`] are stored in `self.locs`; respawn
    /// locs are assumed to already exist in storage (as static locs loaded from the map).
    ///
    /// If the zone already holds [`Self::MAX_LOCS`] locs in total (respawn +
    /// despawn), the oldest despawn loc is evicted first (see
    /// [`make_room_for_loc`](Self::make_room_for_loc)) and returned, so the caller
    /// can drop its collision -- which is owned by the engine, not the zone.
    ///
    /// # Arguments
    ///
    /// * `loc` -- The loc to add.
    ///
    /// # Returns
    ///
    /// The evicted despawn loc when the zone was at the loc cap, otherwise `None`.
    ///
    /// # Side Effects
    ///
    /// - Calls `loc.revert()` and clears `loc.last_clock`.
    /// - Evicts the oldest despawn loc (queuing its `LocDel`) if at [`Self::MAX_LOCS`].
    /// - Pushes the loc to `self.locs` if it has `Despawn` lifetime.
    /// - Queues an enclosed `LocAddChange` event.
    ///
    /// **Called by:** `Engine` methods when scripts or game logic place a loc.
    ///
    /// **Calls:** [`make_room_for_loc`](Self::make_room_for_loc),
    /// [`queue_event`](Self::queue_event).
    pub fn add_loc(&mut self, mut loc: Loc) -> Option<Loc> {
        loc.revert();
        loc.set_last_clock(u32::MAX);
        let mut evicted = None;
        if loc.lifetime() == EntityLifeTime::Despawn {
            evicted = self.make_room_for_loc();
            self.locs.push(loc);
        }
        self.queue_event(
            Some(loc.lid()),
            ZoneEventType::Enclosed,
            None,
            loc_add_change_msg(&loc),
        );
        evicted
    }

    /// Evicts the oldest despawn loc when the zone is at the loc cap.
    ///
    /// The cap counts *all* locs (respawn + despawn). Once the zone holds
    /// [`Self::MAX_LOCS`] locs in total, the first (oldest) *despawn* loc is removed
    /// via [`remove_loc`](Self::remove_loc) -- which queues a `LocDel` so clients
    /// drop it -- and returned so the caller can clear its collision. Static
    /// (respawn) locs count toward the cap but are never evicted, so a zone
    /// saturated with respawn locs may exceed the cap. Returns `None` when there is
    /// still room or no despawn loc is available to evict.
    fn make_room_for_loc(&mut self) -> Option<Loc> {
        if self.locs.len() < Self::MAX_LOCS {
            return None;
        }
        let idx = self
            .locs
            .iter()
            .position(|l| l.lifetime() == EntityLifeTime::Despawn)?;
        let evicted = self.locs[idx];
        self.remove_loc(idx);
        Some(evicted)
    }

    /// Notifies clients that a loc at the given index has changed its type/shape/angle.
    ///
    /// The loc's `last_clock` is cleared and an enclosed `LocAddChange` event is
    /// queued with the loc's current (changed) state. The caller is responsible
    /// for having already called `loc.change(...)` on the loc before invoking this.
    ///
    /// # Arguments
    ///
    /// * `idx` -- The index into `self.locs` of the loc that changed.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds for `self.locs`.
    ///
    /// # Side Effects
    ///
    /// - Clears `self.locs[idx].last_clock`.
    /// - Queues an enclosed `LocAddChange` event.
    ///
    /// **Called by:** `Engine` methods when scripts change a loc's type.
    ///
    /// **Calls:** [`queue_event`](Self::queue_event).
    pub fn change_loc(&mut self, idx: usize) {
        let loc = &mut self.locs[idx];
        loc.set_last_clock(u32::MAX);
        let lid = loc.lid();
        let message = loc_add_change_msg(loc);
        self.events
            .retain(|e| e.id != Some(lid) || !matches!(e.message, ZoneMessage::LocAddChange(_)));
        self.queue_event(Some(lid), ZoneEventType::Enclosed, None, message);
    }

    /// Removes a loc from this zone and queues an enclosed `LocDel` event.
    ///
    /// behavior depends on the loc's lifetime:
    /// - [`EntityLifeTime::Despawn`]: The loc is removed from `self.locs` with an
    ///   order-preserving remove, so the remaining locs keep their relative order
    ///   (a zone reload re-sends locs in their original add order).
    /// - [`EntityLifeTime::Respawn`]: The loc remains in storage but is reverted to its
    ///   base state (it will be invisible until respawned).
    ///
    /// Any previously queued events for this loc are cancelled before the
    /// delete event is queued.
    ///
    /// # Arguments
    ///
    /// * `idx` -- The index into `self.locs` of the loc to remove.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds for `self.locs`.
    ///
    /// # Side Effects
    ///
    /// - Cancels prior events via [`clear_queued_events`](Self::clear_queued_events).
    /// - Removes or reverts the loc depending on its lifetime.
    /// - Queues an enclosed `LocDel` event.
    ///
    /// **Called by:** `Engine::remove_loc`, zone phase `LocDelete` processing.
    ///
    /// **Calls:** [`clear_queued_events`](Self::clear_queued_events),
    /// [`queue_event`](Self::queue_event).
    pub fn remove_loc(&mut self, idx: usize) {
        let loc = &self.locs[idx];
        let coord = loc.packed_zone_coord();
        let lid = loc.lid();
        let shape_angle = loc.packed_shape_angle();
        let lifetime = loc.lifetime();
        self.clear_queued_events(lid);
        if lifetime == EntityLifeTime::Despawn {
            self.locs.remove(idx);
        } else {
            self.locs[idx].revert();
        }
        self.queue_event(
            Some(lid),
            ZoneEventType::Enclosed,
            None,
            ZoneMessage::LocDel(LocDel { coord, shape_angle }),
        );
    }

    /// Finds the index of a visible loc at the given position with the given type id.
    ///
    /// Searches `self.locs` for a loc matching the exact x/z coordinates and type
    /// id that is currently visible (active).
    ///
    /// # Arguments
    ///
    /// * `x` -- The absolute x coordinate to match.
    /// * `z` -- The absolute z coordinate to match.
    /// * `id` -- The loc type id to match.
    ///
    /// # Returns
    ///
    /// `Some(index)` if a matching visible loc is found, `None` otherwise.
    ///
    /// **Called by:** `Engine` methods for loc lookup, zone phase processing,
    /// op handlers (`oploc`).
    pub fn get_loc(&self, x: u16, z: u16, id: u16) -> Option<usize> {
        self.locs
            .iter()
            .position(|loc| loc.is_at(x, z) && loc.id() == id && loc.visible())
    }

    /// Finds the index of a visible loc at the given position on the given layer.
    ///
    /// Searches `self.locs` for a loc matching the exact x/z coordinates and
    /// [`LocLayer`] that is currently visible.
    ///
    /// # Arguments
    ///
    /// * `x` -- The absolute x coordinate to match.
    /// * `z` -- The absolute z coordinate to match.
    /// * `layer` -- The loc layer to match (e.g., Ground, WallDecor).
    ///
    /// # Returns
    ///
    /// `Some(index)` if a matching visible loc is found, `None` otherwise.
    ///
    /// **Called by:** `Engine` shared phase for loc collision checks.
    pub fn get_loc_by_layer(&self, x: u16, z: u16, layer: LocLayer) -> Option<usize> {
        self.locs
            .iter()
            .position(|loc| loc.is_at(x, z) && loc.layer() == layer && loc.visible())
    }

    /// Respawns a previously removed loc, restoring it to its base state.
    ///
    /// Reverts the loc to its original type/shape/angle, clears its clock,
    /// and queues an enclosed `LocAddChange` event to notify clients.
    ///
    /// # Arguments
    ///
    /// * `idx` -- The index into `self.locs` of the loc to respawn.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds for `self.locs`.
    ///
    /// # Side Effects
    ///
    /// - Calls `revert()` on the loc and clears `last_clock`.
    /// - Queues an enclosed `LocAddChange` event.
    ///
    /// **Called by:** Zone phase `LocDelete` processing when a respawn timer expires.
    ///
    /// **Calls:** [`queue_event`](Self::queue_event).
    pub fn respawn_loc(&mut self, idx: usize) {
        let loc = &mut self.locs[idx];
        loc.revert();
        loc.set_last_clock(u32::MAX);
        let lid = loc.lid();
        let message = loc_add_change_msg(loc);
        self.queue_event(Some(lid), ZoneEventType::Enclosed, None, message);
    }

    /// Plays an animation sequence on a loc and queues an enclosed `LocAnim` event.
    ///
    /// # Arguments
    ///
    /// * `idx` -- The index into `self.locs` of the loc to animate.
    /// * `seq` -- The animation sequence id to play.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds for `self.locs`.
    ///
    /// # Side Effects
    ///
    /// Queues an enclosed `LocAnim` event.
    ///
    /// **Called by:** `Engine` methods when scripts trigger a loc animation.
    ///
    /// **Calls:** [`queue_event`](Self::queue_event).
    pub fn anim_loc(&mut self, idx: usize, seq: u16) {
        let loc = &self.locs[idx];
        self.queue_event(
            Some(loc.lid()),
            ZoneEventType::Enclosed,
            None,
            ZoneMessage::LocAnim(LocAnim {
                coord: loc.packed_zone_coord(),
                shape_angle: loc.packed_shape_angle(),
                seq,
            }),
        );
    }

    /// Queues a pre-built zone message for a loc merge operation.
    ///
    /// Loc merges are used for multi-tile locs that affect rendering across
    /// zone boundaries. The caller constructs the [`ZoneMessage::LocMerge`]
    /// payload externally.
    ///
    /// # Arguments
    ///
    /// * `idx` -- The index into `self.locs` of the loc being merged.
    /// * `message` -- The pre-built `LocMerge` zone message.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds for `self.locs`.
    ///
    /// # Side Effects
    ///
    /// Queues an enclosed event with the provided message.
    ///
    /// **Called by:** `Engine` methods for loc merge operations.
    ///
    /// **Calls:** [`queue_event`](Self::queue_event).
    pub fn merge_loc(&mut self, idx: usize, message: ZoneMessage) {
        let loc = &self.locs[idx];
        self.queue_event(Some(loc.lid()), ZoneEventType::Enclosed, None, message);
    }

    // ---- obj management ----

    /// Adds an obj (ground item) to this zone and queues an `ObjAdd` event.
    ///
    /// Only objs with [`EntityLifeTime::Despawn`] are stored in `self.objs`.
    /// Respawn objs are assumed to already exist in storage.
    ///
    /// The event type depends on whether the obj has a receiver:
    /// - No receiver (`None`): queues an `Enclosed` event visible to all.
    /// - With receiver (`Some`): queues a `Follows` event visible only to the receiver.
    ///
    /// # Arguments
    ///
    /// * `obj` -- The obj to add. Consumed by this method.
    /// * `receiver37` -- The player UID (lower 37 bits) who should see the obj
    ///   privately, or `None` for a publicly visible obj.
    ///
    /// If the zone already holds [`Self::MAX_OBJS`] objs in total (respawn +
    /// despawn), the oldest despawn obj is evicted first (see
    /// [`make_room_for_obj`](Self::make_room_for_obj)). Unlike locs, objs carry no
    /// collision, so eviction is fully handled here.
    ///
    /// The obj is then stamped with an instance slot (see
    /// [`assign_obj_slot`](Self::assign_obj_slot)) so its [`oid`](Obj::oid) is
    /// unique within the zone before the `ObjAdd` event is queued.
    ///
    /// # Side Effects
    ///
    /// - Evicts the oldest despawn obj (queuing its `ObjDel`) if at [`Self::MAX_OBJS`].
    /// - Stamps the obj with a zone-unique instance slot.
    /// - Pushes the obj to `self.objs` if it has `Despawn` lifetime.
    /// - Queues an `ObjAdd` event (enclosed or follows depending on receiver).
    ///
    /// **Called by:** `Engine::add_obj` when scripts or game logic drop a ground item.
    ///
    /// **Calls:** [`make_room_for_obj`](Self::make_room_for_obj),
    /// [`queue_event`](Self::queue_event).
    pub fn add_obj(&mut self, mut obj: Obj, receiver37: Option<u64>) {
        let despawn = obj.lifetime() == EntityLifeTime::Despawn;
        if despawn {
            self.make_room_for_obj();
        }
        let Some(slot) = self.assign_obj_slot(obj.local_x(), obj.local_z(), obj.id()) else {
            return;
        };
        obj.set_slot(slot);
        let oid = obj.oid();
        let message = ZoneMessage::ObjAdd(ObjAdd {
            coord: obj.packed_zone_coord(),
            id: obj.id(),
            count: clamp_count(obj.count()),
        });
        if despawn {
            if let Some(r) = receiver37 {
                obj.set_has_receiver(true);
                self.receivers.insert(oid, r);
            }
            self.objs.push(obj);
        }
        let (event_type, event_receiver) = if receiver37.is_none() {
            (ZoneEventType::Enclosed, None)
        } else {
            (ZoneEventType::Follows, receiver37)
        };
        self.queue_event(Some(oid), event_type, event_receiver, message);
    }

    /// Evicts the oldest despawn obj when the zone is at the obj cap.
    ///
    /// The cap counts *all* objs (respawn + despawn). Once the zone holds
    /// [`Self::MAX_OBJS`] objs in total, the first (oldest) *despawn* obj is removed
    /// via [`remove_obj_at`](Self::remove_obj_at), which queues an `ObjDel` so
    /// clients drop it. Static (respawn) objs count toward the cap but are never
    /// evicted, so a zone saturated with respawn objs may exceed the cap. No-op
    /// while there is still room.
    fn make_room_for_obj(&mut self) {
        if self.objs.len() < Self::MAX_OBJS {
            return;
        }
        if let Some(idx) = self
            .objs
            .iter()
            .position(|o| o.lifetime() == EntityLifeTime::Despawn)
        {
            self.remove_obj_at(idx, None);
        }
    }

    /// Returns the lowest instance slot not already used by an obj at the same
    /// local tile and type id (giving the new obj a zone-unique [`oid`](Obj::oid)),
    /// or `None` if all 256 slots at that `(tile, id)` are taken.
    ///
    /// Objs that share a tile and id -- two identical non-stackable drops, or
    /// different players' private stacks of the same item -- would otherwise share
    /// an oid; the slot disambiguates them. Slots freed by removal are reused, so
    /// values stay small. The dynamic add path is bounded by the per-zone obj cap
    /// ([`Self::MAX_OBJS`] = 127), so it always finds a slot; the static path is
    /// unbounded, so a caller that gets `None` must drop the obj rather than admit a
    /// colliding oid.
    fn assign_obj_slot(&self, local_x: u8, local_z: u8, id: u16) -> Option<u8> {
        let mut used = [0; 4];
        for obj in &self.objs {
            if obj.local_x() == local_x && obj.local_z() == local_z && obj.id() == id {
                let slot = obj.slot();
                used[(slot >> 6) as usize] |= 1 << (slot & 0x3F);
            }
        }
        for (slot, &bits) in used.iter().enumerate() {
            if bits != u64::MAX {
                return Some((((slot as u32) << 6) | (!bits).trailing_zeros()) as u8);
            }
        }
        None
    }

    /// Finds the index of an obj matching exact coordinates, type id, and receiver.
    ///
    /// Unlike [`get_obj`](Self::get_obj), this requires an exact receiver match
    /// rather than falling back to public objs.
    ///
    /// # Arguments
    ///
    /// * `x` -- The absolute x coordinate to match.
    /// * `z` -- The absolute z coordinate to match.
    /// * `id` -- The obj type id to match.
    /// * `receiver37` -- The exact receiver UID to match.
    ///
    /// # Returns
    ///
    /// `Some(index)` if a matching obj is found, `None` otherwise.
    ///
    /// **Called by:** `Engine` methods when checking for existing receiver-specific
    /// objs before merging or stacking.
    pub fn get_obj_of_receiver(&self, x: u16, z: u16, id: u16, receiver37: u64) -> Option<usize> {
        let receivers = &self.receivers;
        self.objs
            .iter()
            .position(|obj| obj.is_at(x, z) && obj.id() == id && owns(receivers, obj, receiver37))
    }

    /// Finds the index of an obj matching coordinates, type id, and optional receiver.
    ///
    /// Receiver matching behavior:
    /// - `None`: matches only public objs (those with `NO_RECEIVER`).
    /// - `Some(r)`: matches objs that are either public (`NO_RECEIVER`) or
    ///   privately owned by the given receiver.
    ///
    /// # Arguments
    ///
    /// * `x` -- The absolute x coordinate to match.
    /// * `z` -- The absolute z coordinate to match.
    /// * `id` -- The obj type id to match.
    /// * `receiver37` -- The player UID to match against, or `None` for public-only.
    ///
    /// # Returns
    ///
    /// `Some(index)` if a matching obj is found, `None` otherwise.
    ///
    /// **Called by:** `Engine` shared phase for obj interaction checks,
    /// op handlers (`opobj`).
    pub fn get_obj(&self, x: u16, z: u16, id: u16, receiver37: Option<u64>) -> Option<usize> {
        let receivers = &self.receivers;
        self.objs.iter().position(|obj| {
            obj.is_at(x, z) && obj.id() == id && matches_receiver(receivers, obj, receiver37)
        })
    }

    /// Reveals a privately-owned obj to all players and queues an enclosed `ObjReveal` event.
    ///
    /// Transitions an obj from player-private visibility to public visibility by
    /// clearing its receiver. Any previously queued events for this obj are
    /// canceled before the reveal event is queued.
    ///
    /// No-op if no matching obj is found.
    ///
    /// # Arguments
    ///
    /// * `x` -- The absolute x coordinate of the obj.
    /// * `z` -- The absolute z coordinate of the obj.
    /// * `id` -- The obj type id.
    /// * `receiver37` -- The current owner's UID (lower 37 bits) to match.
    /// * `receiver_pid` -- The PID of the player who originally owned the obj,
    ///   included in the reveal message for client rendering.
    ///
    /// # Side Effects
    ///
    /// - Sets the obj's `receiver37` to `NO_RECEIVER`.
    /// - Cancels prior events via [`clear_queued_events`](Self::clear_queued_events).
    /// - Queues an enclosed `ObjReveal` event.
    ///
    /// **Called by:** Zone phase `ObjReveal` processing when the reveal timer expires.
    ///
    /// **Calls:** [`clear_queued_events`](Self::clear_queued_events),
    /// [`queue_event`](Self::queue_event).
    pub fn reveal_obj(&mut self, x: u16, z: u16, id: u16, receiver37: u64, receiver_pid: u16) {
        let idx = {
            let receivers = &self.receivers;
            self.objs.iter().position(|obj| {
                obj.is_at(x, z) && obj.id() == id && owns(receivers, obj, receiver37)
            })
        };
        let Some(idx) = idx else {
            return;
        };
        let oid = self.objs[idx].oid();
        self.clear_queued_events(oid);
        let count = self.objs[idx].count();
        self.objs[idx].set_has_receiver(false);
        self.receivers.remove(&oid);
        self.queue_event(
            Some(oid),
            ZoneEventType::Enclosed,
            None,
            ZoneMessage::ObjReveal(ObjReveal {
                coord: self.objs[idx].packed_zone_coord(),
                id,
                count: clamp_count(count),
                receiver: receiver_pid,
            }),
        );
    }

    /// Removes an obj whose `last_clock` matches the expected clock value.
    ///
    /// This is a clock-guarded removal used for scheduled despawns. If the obj's
    /// clock has been updated since the removal was scheduled (e.g., due to a
    /// merge/stack), the removal is a no-op, preventing stale deletions.
    ///
    /// # Arguments
    ///
    /// * `x` -- The absolute x coordinate of the obj.
    /// * `z` -- The absolute z coordinate of the obj.
    /// * `id` -- The obj type id.
    /// * `expected_clock` -- The clock value the obj must have for removal to proceed.
    ///
    /// # Side Effects
    ///
    /// Delegates to [`remove_obj_at`](Self::remove_obj_at) if a match is found.
    ///
    /// **Called by:** Zone phase `ObjDelete` processing when a despawn timer fires.
    ///
    /// **Calls:** [`remove_obj_at`](Self::remove_obj_at).
    pub fn remove_obj_by_clock(&mut self, x: u16, z: u16, id: u16, expected_clock: u32) {
        let Some(idx) = self.objs.iter().position(|obj| {
            obj.is_at(x, z) && obj.id() == id && obj.last_clock() == expected_clock
        }) else {
            return;
        };
        self.remove_obj_at(idx, None);
    }

    /// Removes an obj by position, type, and receiver, with an optional respawn clock.
    ///
    /// Used for player-initiated removal (e.g., picking up a ground item) and
    /// engine-driven removal. Only matches objs that have `Despawn` lifetime or
    /// whose `last_clock` is `u64::MAX` (currently visible respawn objs).
    ///
    /// Receiver matching follows the same rules as [`get_obj`](Self::get_obj).
    ///
    /// No-op if no matching obj is found.
    ///
    /// # Arguments
    ///
    /// * `x` -- The absolute x coordinate of the obj.
    /// * `z` -- The absolute z coordinate of the obj.
    /// * `id` -- The obj type id.
    /// * `receiver37` -- The receiver to match, or `None` for public-only.
    /// * `respawn_at` -- For respawn objs, the clock tick at which the obj should
    ///   reappear. Ignored for despawn objs.
    ///
    /// # Side Effects
    ///
    /// Delegates to [`remove_obj_at`](Self::remove_obj_at) if a match is found.
    ///
    /// **Called by:** `Engine::remove_obj`, cheat handlers.
    ///
    /// **Calls:** [`remove_obj_at`](Self::remove_obj_at).
    pub fn remove_obj(
        &mut self,
        x: u16,
        z: u16,
        id: u16,
        receiver37: Option<u64>,
        respawn_at: Option<u32>,
    ) {
        let idx = {
            let receivers = &self.receivers;
            self.objs.iter().position(|obj| {
                obj.is_at(x, z)
                    && obj.id() == id
                    && (obj.lifetime() == EntityLifeTime::Despawn || obj.last_clock() == u32::MAX)
                    && matches_receiver(receivers, obj, receiver37)
            })
        };
        let Some(idx) = idx else {
            return;
        };
        self.remove_obj_at(idx, respawn_at);
    }

    /// Internal helper that removes the obj at the given index and queues an `ObjDel` event.
    ///
    /// behavior depends on the obj's lifetime:
    /// - [`EntityLifeTime::Despawn`]: The obj is removed from `self.objs` with an
    ///   order-preserving remove, so the remaining objs keep their relative order
    ///   (a zone reload re-sends ground-item stacks oldest-first).
    /// - [`EntityLifeTime::Respawn`]: The obj remains in storage but its `last_clock` is
    ///   set to `respawn_at` (or `u64::MAX` if `None`), hiding it until that tick.
    ///
    /// The event type mirrors the obj's visibility: privately-owned objs get a
    /// `Follows` event; public objs get an `Enclosed` event.
    ///
    /// # Arguments
    ///
    /// * `idx` -- The index into `self.objs` of the obj to remove.
    /// * `respawn_at` -- For respawn objs, the clock tick at which to reappear.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds for `self.objs`.
    ///
    /// # Side Effects
    ///
    /// - Cancels prior events via [`clear_queued_events`](Self::clear_queued_events).
    /// - Removes or hides the obj depending on its lifetime.
    /// - Queues an `ObjDel` event (enclosed or follows).
    ///
    /// **Called by:** [`remove_obj`](Self::remove_obj),
    /// [`remove_obj_by_clock`](Self::remove_obj_by_clock).
    ///
    /// **Calls:** [`clear_queued_events`](Self::clear_queued_events),
    /// [`queue_event`](Self::queue_event).
    fn remove_obj_at(&mut self, idx: usize, respawn_at: Option<u32>) {
        let obj = &self.objs[idx];
        let oid = obj.oid();
        let coord = obj.packed_zone_coord();
        let id = obj.id();
        let (event_type, event_receiver) = match self.receivers.get(&oid) {
            Some(&receiver37) => (ZoneEventType::Follows, Some(receiver37)),
            None => (ZoneEventType::Enclosed, None),
        };
        self.clear_queued_events(oid);
        self.receivers.remove(&oid);
        if self.objs[idx].lifetime() == EntityLifeTime::Despawn {
            self.objs.remove(idx);
        } else {
            self.objs[idx].set_last_clock(respawn_at.unwrap_or(u32::MAX));
        }
        self.queue_event(
            Some(oid),
            event_type,
            event_receiver,
            ZoneMessage::ObjDel(ObjDel { coord, id }),
        );
    }

    /// Respawns a previously removed obj, making it visible again.
    ///
    /// Only matches objs with [`EntityLifeTime::Respawn`] that are currently
    /// hidden (i.e., `last_clock != u64::MAX`). Resets the obj's clock to
    /// `u64::MAX` (permanently visible) and queues an enclosed `ObjAdd` event.
    ///
    /// No-op if no matching hidden respawn obj is found.
    ///
    /// # Arguments
    ///
    /// * `x` -- The absolute x coordinate of the obj.
    /// * `z` -- The absolute z coordinate of the obj.
    /// * `id` -- The obj type id.
    ///
    /// # Side Effects
    ///
    /// - Sets `last_clock` to `u64::MAX` on the matching obj.
    /// - Queues an enclosed `ObjAdd` event.
    ///
    /// **Called by:** Zone phase `ObjAdd` processing when a respawn timer expires.
    ///
    /// **Calls:** [`queue_event`](Self::queue_event).
    pub fn respawn_obj(&mut self, x: u16, z: u16, id: u16) {
        let Some(idx) = self.objs.iter().position(|obj| {
            obj.is_at(x, z)
                && obj.id() == id
                && obj.lifetime() == EntityLifeTime::Respawn
                && obj.last_clock() != u32::MAX
        }) else {
            return;
        };
        self.objs[idx].set_last_clock(u32::MAX);
        let count = self.objs[idx].count();
        let oid = self.objs[idx].oid();
        self.queue_event(
            Some(oid),
            ZoneEventType::Enclosed,
            None,
            ZoneMessage::ObjAdd(ObjAdd {
                coord: self.objs[idx].packed_zone_coord(),
                id,
                count: clamp_count(count),
            }),
        );
    }

    // ---- non-entity events ----

    /// Queues a map animation event (e.g., a spot animation on a tile).
    ///
    /// Map animations are not tied to any entity and are always broadcast to
    /// all players observing the zone.
    ///
    /// # Arguments
    ///
    /// * `message` -- The pre-built [`ZoneMessage::MapAnim`] payload.
    ///
    /// # Side Effects
    ///
    /// Queues an enclosed event with no entity id.
    ///
    /// **Called by:** `Engine` methods when scripts trigger tile animations.
    ///
    /// **Calls:** [`queue_event`](Self::queue_event).
    pub fn anim_map(&mut self, message: ZoneMessage) {
        self.queue_event(None, ZoneEventType::Enclosed, None, message);
    }

    /// Queues a map projectile animation event (e.g., an arrow flying between tiles).
    ///
    /// Projectile animations are not tied to any entity and are always broadcast
    /// to all players observing the zone.
    ///
    /// # Arguments
    ///
    /// * `message` -- The pre-built [`ZoneMessage::MapProjAnim`] payload.
    ///
    /// # Side Effects
    ///
    /// Queues an enclosed event with no entity id.
    ///
    /// **Called by:** `Engine` methods when scripts trigger projectile animations.
    ///
    /// **Calls:** [`queue_event`](Self::queue_event).
    pub fn map_proj_anim(&mut self, message: ZoneMessage) {
        self.queue_event(None, ZoneEventType::Enclosed, None, message);
    }
}

/// Clamps an obj count into the `u16` range used by obj protocol messages.
///
/// Mirrors the inline `count.clamp(0, 65535) as u16` used at every obj message
/// construction site.
fn clamp_count(n: u32) -> u16 {
    n.clamp(0, 65535) as u16
}

/// Builds the `LocAddChange` zone message describing a loc's current state.
///
/// Captures the identical payload constructed when a loc is added, changed, or
/// respawned.
fn loc_add_change_msg(loc: &Loc) -> ZoneMessage {
    ZoneMessage::LocAddChange(LocAddChange {
        coord: loc.packed_zone_coord(),
        id: loc.id(),
        shape_angle: loc.packed_shape_angle(),
    })
}

/// Returns whether `obj` is privately owned by the player `receiver37`.
///
/// An obj is owned when it carries a receiver and that receiver's recorded UID
/// matches `receiver37`. Public objs (no receiver) are never owned.
fn owns(receivers: &FxHashMap<u64, u64>, obj: &Obj, receiver37: u64) -> bool {
    obj.has_receiver() && receivers.get(&obj.oid()) == Some(&receiver37)
}

/// Returns whether `obj` is visible to the player identified by `receiver37`.
///
/// - `None`: matches only public objs (those without a receiver).
/// - `Some(r)`: matches objs that are either public or privately owned by `r`.
fn matches_receiver(receivers: &FxHashMap<u64, u64>, obj: &Obj, receiver37: Option<u64>) -> bool {
    match receiver37 {
        None => !obj.has_receiver(),
        Some(r) => !obj.has_receiver() || receivers.get(&obj.oid()) == Some(&r),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_grid::CoordGrid;
    use rs_pack::types::{LocAngle, LocLayer, LocShape};

    fn zone() -> Zone {
        Zone::new(ZoneCoordGrid::new(402, 0, 402))
    }

    fn coord(x: u16, z: u16) -> CoordGrid {
        CoordGrid::new(x, 0, z)
    }

    fn despawn_obj(x: u16, z: u16, id: u16, count: u32) -> Obj {
        Obj::new(coord(x, z), EntityLifeTime::Despawn, id, count)
    }

    fn respawn_obj(x: u16, z: u16, id: u16, count: u32) -> Obj {
        Obj::new(coord(x, z), EntityLifeTime::Respawn, id, count)
    }

    fn push_private_obj(z: &mut Zone, mut obj: Obj, receiver37: u64) {
        let slot = z
            .assign_obj_slot(obj.local_x(), obj.local_z(), obj.id())
            .unwrap();
        obj.set_slot(slot);
        obj.set_has_receiver(true);
        z.objs.push(obj);
        let oid = z.objs.last().unwrap().oid();
        z.receivers.insert(oid, receiver37);
    }

    fn respawn_loc(x: u16, z: u16, id: u16) -> Loc {
        Loc::new(
            coord(x, z),
            EntityLifeTime::Respawn,
            id,
            LocShape::CentrepieceStraight,
            LocAngle::North,
            true,
            true,
            1,
            1,
        )
    }

    fn despawn_loc(x: u16, z: u16, id: u16) -> Loc {
        Loc::new(
            coord(x, z),
            EntityLifeTime::Despawn,
            id,
            LocShape::CentrepieceStraight,
            LocAngle::North,
            true,
            true,
            1,
            1,
        )
    }

    fn count_enclosed(zone: &Zone) -> usize {
        zone.enclosed_events().count()
    }

    fn count_follows(zone: &Zone) -> usize {
        zone.follows_events().count()
    }

    // ---- obj: add ----

    #[test]
    fn add_despawn_obj_pushes_to_vec() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 5);
        obj.set_last_clock(200);
        z.add_obj(obj, None);
        assert_eq!(z.objs.len(), 1);
        assert_eq!(z.objs[0].id(), 100);
    }

    #[test]
    fn add_respawn_obj_does_not_push_to_vec() {
        let mut z = zone();
        z.add_obj(respawn_obj(3222, 3222, 100, 5), None);
        assert_eq!(z.objs.len(), 0);
    }

    #[test]
    fn add_obj_without_receiver_queues_enclosed_event() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.add_obj(obj, None);
        assert_eq!(count_enclosed(&z), 1);
        assert_eq!(count_follows(&z), 0);
    }

    #[test]
    fn add_obj_with_receiver_queues_follows_event() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.add_obj(obj, Some(12345));
        assert_eq!(count_follows(&z), 1);
        assert_eq!(count_enclosed(&z), 0);
    }

    // ---- obj: visible_objs ----

    #[test]
    fn visible_objs_despawn_before_expiry() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.objs.push(obj);
        assert_eq!(z.visible_objs(0, 100).count(), 1);
    }

    #[test]
    fn visible_objs_despawn_at_expiry_hidden() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.objs.push(obj);
        assert_eq!(z.visible_objs(0, 200).count(), 0);
    }

    #[test]
    fn visible_objs_despawn_after_expiry_hidden() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.objs.push(obj);
        assert_eq!(z.visible_objs(0, 250).count(), 0);
    }

    #[test]
    fn visible_objs_respawn_no_clock_visible() {
        let mut z = zone();
        z.objs.push(respawn_obj(3222, 3222, 100, 1));
        assert_eq!(z.visible_objs(0, 0).count(), 1);
    }

    #[test]
    fn visible_objs_respawn_hidden_until_clock() {
        let mut z = zone();
        let mut obj = respawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(50);
        z.objs.push(obj);
        assert_eq!(z.visible_objs(0, 49).count(), 0);
        assert_eq!(z.visible_objs(0, 50).count(), 1);
        assert_eq!(z.visible_objs(0, 51).count(), 1);
    }

    #[test]
    fn visible_objs_receiver_filtering() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        push_private_obj(&mut z, obj, 111);
        assert_eq!(z.visible_objs(111, 50).count(), 1);
        assert_eq!(z.visible_objs(222, 50).count(), 0);
    }

    #[test]
    fn visible_objs_no_receiver_visible_to_all() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.objs.push(obj);
        assert_eq!(z.visible_objs(111, 50).count(), 1);
        assert_eq!(z.visible_objs(222, 50).count(), 1);
    }

    // ---- obj: reveal ----

    #[test]
    fn reveal_obj_clears_receiver() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 5);
        obj.set_last_clock(200);
        push_private_obj(&mut z, obj, 111);
        z.reveal_obj(3222, 3222, 100, 111, 1);
        assert!(!z.objs[0].has_receiver());
    }

    #[test]
    fn reveal_obj_makes_visible_to_all() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 5);
        obj.set_last_clock(200);
        push_private_obj(&mut z, obj, 111);
        assert_eq!(z.visible_objs(222, 50).count(), 0);
        z.reveal_obj(3222, 3222, 100, 111, 1);
        assert_eq!(z.visible_objs(222, 50).count(), 1);
    }

    #[test]
    fn reveal_obj_queues_enclosed_event() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 5);
        obj.set_last_clock(200);
        push_private_obj(&mut z, obj, 111);
        z.reveal_obj(3222, 3222, 100, 111, 1);
        assert!(
            z.enclosed_events()
                .any(|e| matches!(&e.message, ZoneMessage::ObjReveal(_)))
        );
    }

    #[test]
    fn reveal_obj_nonexistent_is_noop() {
        let mut z = zone();
        z.reveal_obj(3222, 3222, 100, 111, 1);
        assert_eq!(count_enclosed(&z), 0);
    }

    // ---- obj: remove_obj_by_clock ----

    #[test]
    fn remove_obj_by_clock_matching() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.objs.push(obj);
        z.remove_obj_by_clock(3222, 3222, 100, 200);
        assert_eq!(z.objs.len(), 0);
    }

    #[test]
    fn remove_obj_by_clock_stale_is_noop() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(300);
        z.objs.push(obj);
        z.remove_obj_by_clock(3222, 3222, 100, 200);
        assert_eq!(z.objs.len(), 1);
    }

    #[test]
    fn remove_obj_by_clock_queues_del_event() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.objs.push(obj);
        z.remove_obj_by_clock(3222, 3222, 100, 200);
        assert!(
            z.enclosed_events()
                .any(|e| matches!(&e.message, ZoneMessage::ObjDel(_)))
        );
    }

    // ---- obj: remove_obj (player pickup) ----

    #[test]
    fn remove_obj_despawn_removes_from_vec() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.objs.push(obj);
        z.remove_obj(3222, 3222, 100, None, None);
        assert_eq!(z.objs.len(), 0);
    }

    #[test]
    fn remove_obj_respawn_stays_in_vec() {
        let mut z = zone();
        z.objs.push(respawn_obj(3222, 3222, 100, 1));
        z.remove_obj(3222, 3222, 100, None, Some(50));
        assert_eq!(z.objs.len(), 1);
        assert_eq!(z.objs[0].last_clock(), 50);
    }

    #[test]
    fn remove_obj_respawn_hidden_until_respawn() {
        let mut z = zone();
        z.objs.push(respawn_obj(3222, 3222, 100, 1));
        z.remove_obj(3222, 3222, 100, None, Some(50));
        assert_eq!(z.visible_objs(0, 49).count(), 0);
        assert_eq!(z.visible_objs(0, 50).count(), 1);
    }

    #[test]
    fn remove_obj_with_receiver_matches_receiver() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        push_private_obj(&mut z, obj, 111);
        z.remove_obj(3222, 3222, 100, Some(111), None);
        assert_eq!(z.objs.len(), 0);
    }

    #[test]
    fn remove_obj_wrong_receiver_is_noop() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        push_private_obj(&mut z, obj, 111);
        z.remove_obj(3222, 3222, 100, Some(222), None);
        assert_eq!(z.objs.len(), 1);
    }

    #[test]
    fn remove_obj_revealed_then_delete_uses_enclosed() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        push_private_obj(&mut z, obj, 111);
        z.reveal_obj(3222, 3222, 100, 111, 1);
        z.reset();
        z.remove_obj(3222, 3222, 100, None, None);
        assert!(
            z.enclosed_events()
                .any(|e| matches!(&e.message, ZoneMessage::ObjDel(_)))
        );
        assert_eq!(count_follows(&z), 0);
    }

    #[test]
    fn remove_obj_unrevealed_delete_uses_follows() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        push_private_obj(&mut z, obj, 111);
        z.remove_obj(3222, 3222, 100, Some(111), None);
        assert!(
            z.follows_events()
                .any(|e| matches!(&e.message, ZoneMessage::ObjDel(_)))
        );
    }

    #[test]
    fn remove_obj_skips_already_hidden_respawn() {
        let mut z = zone();
        let obj = respawn_obj(3222, 3222, 100, 1);
        z.objs.push(obj);
        z.remove_obj(3222, 3222, 100, None, Some(50));
        z.reset();
        z.remove_obj(3222, 3222, 100, None, Some(100));
        assert_eq!(count_enclosed(&z), 0);
    }

    #[test]
    fn remove_obj_preserves_remaining_stack_order() {
        let mut z = zone();

        // Drop X on one tile, then a two-item stack A, B on another -- in order.
        for (x, zz, id) in [(3216, 3216, 10), (3220, 3220, 20), (3220, 3220, 30)] {
            let mut obj = despawn_obj(x, zz, id, 1);
            obj.set_last_clock(200);
            z.add_obj(obj, None);
        }
        assert_eq!(
            z.visible_objs(0, 100).map(|o| o.id()).collect::<Vec<_>>(),
            vec![10, 20, 30],
        );

        // Pick up X (oldest, index 0). The A,B stack must stay A-then-B.
        z.remove_obj(3216, 3216, 10, None, None);
        assert_eq!(
            z.visible_objs(0, 100).map(|o| o.id()).collect::<Vec<_>>(),
            vec![20, 30],
            "removing an obj reordered the remaining stack",
        );
    }

    // ---- obj: respawn_obj ----

    #[test]
    fn respawn_obj_clears_clock_and_queues_add() {
        let mut z = zone();
        let mut obj = respawn_obj(3222, 3222, 100, 5);
        obj.set_last_clock(50);
        z.objs.push(obj);
        z.respawn_obj(3222, 3222, 100);
        assert_eq!(z.objs[0].last_clock(), u32::MAX);
        assert!(
            z.enclosed_events()
                .any(|e| matches!(&e.message, ZoneMessage::ObjAdd(_)))
        );
    }

    #[test]
    fn respawn_obj_not_hidden_is_noop() {
        let mut z = zone();
        z.objs.push(respawn_obj(3222, 3222, 100, 5));
        z.respawn_obj(3222, 3222, 100);
        assert_eq!(count_enclosed(&z), 0);
    }

    #[test]
    fn respawn_obj_despawn_is_noop() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 5);
        obj.set_last_clock(50);
        z.objs.push(obj);
        z.respawn_obj(3222, 3222, 100);
        assert_eq!(count_enclosed(&z), 0);
    }

    // ---- obj: merge simulation ----

    #[test]
    fn merge_obj_stale_delete_ignored() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 5);
        obj.set_last_clock(100);
        z.objs.push(obj);
        z.objs[0].set_count(10);
        z.objs[0].set_last_clock(200);
        z.remove_obj_by_clock(3222, 3222, 100, 100);
        assert_eq!(z.objs.len(), 1);
        assert_eq!(z.objs[0].count(), 10);
    }

    #[test]
    fn merge_obj_new_delete_works() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 10);
        obj.set_last_clock(200);
        z.objs.push(obj);
        z.remove_obj_by_clock(3222, 3222, 100, 200);
        assert_eq!(z.objs.len(), 0);
    }

    // ---- obj: full lifecycle ----

    #[test]
    fn obj_full_lifecycle_add_reveal_delete() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 5);
        obj.set_last_clock(200);
        z.add_obj(obj, Some(111));
        assert_eq!(z.objs.len(), 1);
        assert_eq!(z.visible_objs(111, 10).count(), 1);
        assert_eq!(z.visible_objs(222, 10).count(), 0);

        z.reset();
        z.reveal_obj(3222, 3222, 100, 111, 1);
        assert_eq!(z.visible_objs(222, 60).count(), 1);

        z.reset();
        z.remove_obj_by_clock(3222, 3222, 100, 200);
        assert_eq!(z.objs.len(), 0);
    }

    #[test]
    fn obj_respawn_full_lifecycle() {
        let mut z = zone();
        z.objs.push(respawn_obj(3222, 3222, 100, 5));
        assert_eq!(z.visible_objs(0, 0).count(), 1);

        z.remove_obj(3222, 3222, 100, None, Some(50));
        assert_eq!(z.objs.len(), 1);
        assert_eq!(z.visible_objs(0, 49).count(), 0);

        z.reset();
        z.respawn_obj(3222, 3222, 100);
        assert_eq!(z.visible_objs(0, 50).count(), 1);
        assert_eq!(z.objs[0].last_clock(), u32::MAX);
    }

    // ---- loc: add ----

    #[test]
    fn add_despawn_loc_pushes_to_vec() {
        let mut z = zone();
        z.add_loc(despawn_loc(3222, 3222, 100));
        assert_eq!(z.locs.len(), 1);
    }

    #[test]
    fn add_respawn_loc_does_not_push() {
        let mut z = zone();
        z.add_loc(respawn_loc(3222, 3222, 100));
        assert_eq!(z.locs.len(), 0);
    }

    #[test]
    fn add_loc_queues_enclosed_event() {
        let mut z = zone();
        z.add_loc(despawn_loc(3222, 3222, 100));
        assert!(
            z.enclosed_events()
                .any(|e| matches!(&e.message, ZoneMessage::LocAddChange(_)))
        );
    }

    #[test]
    fn add_loc_reverts_and_clears_clock() {
        let mut z = zone();
        let mut loc = despawn_loc(3222, 3222, 100);
        loc.change(
            200,
            LocShape::CentrepieceStraight,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        loc.set_last_clock(999);
        z.add_loc(loc);
        assert!(!z.locs[0].is_changed());
        assert_eq!(z.locs[0].last_clock(), u32::MAX);
    }

    // ---- loc: active() derived state ----

    #[test]
    fn loc_active_normal_respawn() {
        let loc = respawn_loc(3222, 3222, 100);
        assert!(loc.visible());
    }

    #[test]
    fn loc_active_despawn_always() {
        let mut loc = despawn_loc(3222, 3222, 100);
        loc.set_last_clock(999);
        assert!(loc.visible());
    }

    #[test]
    fn loc_active_changed_respawn() {
        let mut loc = respawn_loc(3222, 3222, 100);
        loc.change(
            200,
            LocShape::CentrepieceStraight,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        loc.set_last_clock(999);
        assert!(loc.visible());
    }

    #[test]
    fn loc_inactive_removed_respawn() {
        let mut loc = respawn_loc(3222, 3222, 100);
        loc.set_last_clock(999);
        assert!(!loc.visible());
    }

    #[test]
    fn loc_inactive_reverted_with_clock() {
        let mut loc = respawn_loc(3222, 3222, 100);
        loc.change(
            200,
            LocShape::CentrepieceStraight,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        loc.revert();
        loc.set_last_clock(999);
        assert!(!loc.visible());
    }

    // ---- loc: change ----

    #[test]
    fn change_loc_updates_type() {
        let mut z = zone();
        z.locs.push(respawn_loc(3222, 3222, 100));
        z.locs[0].change(
            200,
            LocShape::CentrepieceStraight,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        z.change_loc(0);
        assert!(z.locs[0].is_changed());
        assert_eq!(z.locs[0].id(), 200);
    }

    #[test]
    fn change_loc_queues_add_change_event() {
        let mut z = zone();
        z.locs.push(respawn_loc(3222, 3222, 100));
        z.locs[0].change(
            200,
            LocShape::CentrepieceStraight,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        z.change_loc(0);
        assert!(
            z.enclosed_events()
                .any(|e| matches!(&e.message, ZoneMessage::LocAddChange(_)))
        );
    }

    #[test]
    fn change_loc_clears_last_clock() {
        let mut z = zone();
        z.locs.push(respawn_loc(3222, 3222, 100));
        z.locs[0].set_last_clock(999);
        z.change_loc(0);
        assert_eq!(z.locs[0].last_clock(), u32::MAX);
    }

    #[test]
    fn change_to_different_shape_updates_shape() {
        let mut loc = despawn_loc(3222, 3222, 100); // centrepiece_straight
        assert_eq!(loc.shape(), LocShape::CentrepieceStraight);
        loc.change(
            200,
            LocShape::WallDiagonal,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        assert_eq!(loc.shape(), LocShape::WallDiagonal);
        assert_eq!(loc.id(), 200);
    }

    #[test]
    fn change_to_different_shape_keeps_base_layer() {
        let mut loc = despawn_loc(3222, 3222, 100);
        let base_layer = loc.layer();
        loc.change(
            200,
            LocShape::WallDiagonal,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        assert_eq!(loc.layer(), base_layer);
    }

    #[test]
    fn revert_restores_base_shape() {
        let mut loc = respawn_loc(3222, 3222, 100); // centrepiece_straight
        loc.change(
            200,
            LocShape::WallDiagonal,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        assert_eq!(loc.shape(), LocShape::WallDiagonal);
        loc.revert();
        assert_eq!(loc.shape(), LocShape::CentrepieceStraight);
        assert!(!loc.is_changed());
    }

    #[test]
    fn loc_id_supports_full_u16() {
        let mut loc = Loc::new(
            coord(3222, 3222),
            EntityLifeTime::Despawn,
            u16::MAX,
            LocShape::CentrepieceStraight,
            LocAngle::North,
            false,
            false,
            2,
            4,
        );
        assert_eq!(loc.id(), u16::MAX);
        assert_eq!((loc.width(), loc.length()), (2, 4));
        loc.change(
            60000,
            LocShape::WallDiagonal,
            LocAngle::East,
            true,
            true,
            5,
            3,
        );
        assert_eq!(loc.id(), 60000);
        assert_eq!(loc.shape(), LocShape::WallDiagonal);
        assert_eq!(loc.angle(), LocAngle::East);
        assert_eq!((loc.width(), loc.length()), (5, 3));
        loc.revert();
        assert_eq!(loc.id(), u16::MAX);
        assert_eq!(loc.shape(), LocShape::CentrepieceStraight);
        assert_eq!((loc.width(), loc.length()), (2, 4));
    }

    #[test]
    fn change_loc_event_carries_new_shape() {
        let mut z = zone();
        z.locs.push(despawn_loc(3222, 3222, 100)); // centrepiece_straight
        z.locs[0].change(
            200,
            LocShape::WallDiagonal,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        z.change_loc(0);
        let shape = z
            .enclosed_events()
            .find_map(|e| match &e.message {
                ZoneMessage::LocAddChange(lac) => Some(lac.shape_angle >> 2),
                _ => None,
            })
            .expect("a LocAddChange event should be queued");
        assert_eq!(shape, LocShape::WallDiagonal as u8);
    }

    // ---- loc: remove ----

    #[test]
    fn remove_despawn_loc_removes_from_vec() {
        let mut z = zone();
        z.locs.push(despawn_loc(3222, 3222, 100));
        z.remove_loc(0);
        assert_eq!(z.locs.len(), 0);
    }

    #[test]
    fn remove_respawn_loc_stays_in_vec() {
        let mut z = zone();
        z.locs.push(respawn_loc(3222, 3222, 100));
        z.remove_loc(0);
        assert_eq!(z.locs.len(), 1);
    }

    #[test]
    fn remove_respawn_loc_reverts() {
        let mut z = zone();
        z.locs.push(respawn_loc(3222, 3222, 100));
        z.locs[0].change(
            200,
            LocShape::CentrepieceStraight,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        z.remove_loc(0);
        assert!(!z.locs[0].is_changed());
        assert_eq!(z.locs[0].id(), 100);
    }

    #[test]
    fn remove_loc_queues_del_event() {
        let mut z = zone();
        z.locs.push(respawn_loc(3222, 3222, 100));
        z.remove_loc(0);
        assert!(
            z.enclosed_events()
                .any(|e| matches!(&e.message, ZoneMessage::LocDel(_)))
        );
    }

    #[test]
    fn remove_loc_cancels_previous_events() {
        let mut z = zone();
        z.locs.push(respawn_loc(3222, 3222, 100));
        z.locs[0].change(
            200,
            LocShape::CentrepieceStraight,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        z.change_loc(0);
        assert_eq!(count_enclosed(&z), 1);
        z.remove_loc(0);
        let active_enclosed: Vec<_> = z.enclosed_events().collect();
        assert_eq!(active_enclosed.len(), 1);
        assert!(matches!(
            &active_enclosed[0].message,
            ZoneMessage::LocDel(_)
        ));
    }

    #[test]
    fn remove_loc_preserves_remaining_order() {
        let mut z = zone();

        // Add three dynamic locs on distinct tiles, in order.
        for (x, zz, id) in [(3216, 3216, 10), (3220, 3220, 20), (3221, 3221, 30)] {
            z.locs.push(despawn_loc(x, zz, id));
        }
        assert_eq!(
            z.locs.iter().map(|l| l.id()).collect::<Vec<_>>(),
            vec![10, 20, 30],
        );

        // Remove the oldest (index 0). The remaining locs must keep their order.
        z.remove_loc(0);
        assert_eq!(
            z.locs.iter().map(|l| l.id()).collect::<Vec<_>>(),
            vec![20, 30],
            "removing a loc reordered the remaining locs",
        );
    }

    // ---- loc: get_loc / get_loc_by_layer ----

    #[test]
    fn get_loc_finds_active() {
        let mut z = zone();
        z.locs.push(respawn_loc(3222, 3222, 100));
        assert!(z.get_loc(3222, 3222, 100).is_some());
    }

    #[test]
    fn get_loc_skips_inactive() {
        let mut z = zone();
        let mut loc = respawn_loc(3222, 3222, 100);
        loc.set_last_clock(50);
        z.locs.push(loc);
        assert!(z.get_loc(3222, 3222, 100).is_none());
    }

    #[test]
    fn get_loc_by_layer_finds_active() {
        let mut z = zone();
        z.locs.push(respawn_loc(3222, 3222, 100));
        assert!(z.get_loc_by_layer(3222, 3222, LocLayer::Ground).is_some());
    }

    #[test]
    fn get_loc_by_layer_finds_changed() {
        let mut z = zone();
        let mut loc = respawn_loc(3222, 3222, 100);
        loc.change(
            200,
            LocShape::CentrepieceStraight,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        loc.set_last_clock(999);
        z.locs.push(loc);
        assert!(z.get_loc_by_layer(3222, 3222, LocLayer::Ground).is_some());
    }

    // ---- loc: respawn ----

    #[test]
    fn respawn_loc_reverts_and_clears_clock() {
        let mut z = zone();
        let mut loc = respawn_loc(3222, 3222, 100);
        loc.set_last_clock(50);
        z.locs.push(loc);
        assert!(!z.locs[0].visible());
        z.respawn_loc(0);
        assert!(z.locs[0].visible());
        assert_eq!(z.locs[0].last_clock(), u32::MAX);
        assert!(!z.locs[0].is_changed());
    }

    #[test]
    fn respawn_loc_queues_add_change_event() {
        let mut z = zone();
        let mut loc = respawn_loc(3222, 3222, 100);
        loc.set_last_clock(50);
        z.locs.push(loc);
        z.respawn_loc(0);
        assert!(
            z.enclosed_events()
                .any(|e| matches!(&e.message, ZoneMessage::LocAddChange(_)))
        );
    }

    // ---- loc: full lifecycle ----

    #[test]
    fn loc_change_and_revert_lifecycle() {
        let mut z = zone();
        z.locs.push(respawn_loc(3222, 3222, 100));
        assert_eq!(z.locs[0].id(), 100);
        assert!(z.locs[0].visible());

        z.locs[0].change(
            200,
            LocShape::CentrepieceStraight,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        z.locs[0].set_last_clock(50);
        z.change_loc(0);
        assert_eq!(z.locs[0].id(), 200);
        assert!(z.locs[0].visible());
        assert!(z.locs[0].is_changed());

        z.reset();
        z.locs[0].revert();
        z.locs[0].set_last_clock(u32::MAX);
        z.change_loc(0);
        assert_eq!(z.locs[0].id(), 100);
        assert!(z.locs[0].visible());
        assert!(!z.locs[0].is_changed());
    }

    #[test]
    fn loc_remove_and_respawn_lifecycle() {
        let mut z = zone();
        z.locs.push(respawn_loc(3222, 3222, 100));
        assert!(z.locs[0].visible());

        z.remove_loc(0);
        z.locs[0].set_last_clock(50);
        assert!(!z.locs[0].visible());

        z.reset();
        z.respawn_loc(0);
        assert!(z.locs[0].visible());
        assert_eq!(z.locs[0].id(), 100);
    }

    #[test]
    fn loc_changed_then_removed_then_respawned() {
        let mut z = zone();
        z.locs.push(respawn_loc(3222, 3222, 100));

        z.locs[0].change(
            200,
            LocShape::CentrepieceStraight,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        z.locs[0].set_last_clock(50);
        z.change_loc(0);
        assert_eq!(z.locs[0].id(), 200);
        assert!(z.locs[0].visible());

        z.reset();
        z.remove_loc(0);
        z.locs[0].set_last_clock(100);
        assert!(!z.locs[0].visible());
        assert!(!z.locs[0].is_changed());
        assert_eq!(z.locs[0].id(), 100);

        z.reset();
        z.respawn_loc(0);
        assert!(z.locs[0].visible());
        assert_eq!(z.locs[0].id(), 100);
    }

    #[test]
    fn despawn_loc_add_then_expire() {
        let mut z = zone();
        let mut loc = despawn_loc(3222, 3222, 100);
        loc.set_last_clock(50);
        z.add_loc(loc);
        assert_eq!(z.locs.len(), 1);
        assert!(z.locs[0].visible());

        z.reset();
        z.remove_loc(0);
        assert_eq!(z.locs.len(), 0);
    }

    // ---- zone: reset ----

    #[test]
    fn reset_clears_events_and_shared() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.add_obj(obj, None);
        z.compute_shared();
        assert!(z.shared_bytes().is_some());
        z.reset();
        assert!(z.shared_bytes().is_none());
        assert!(!z.has_events());
    }

    // ---- zone: event cancellation ----

    #[test]
    fn clear_queued_events_cancels_matching() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.add_obj(obj, None);
        assert_eq!(count_enclosed(&z), 1);
        let oid = z.objs[0].oid();
        z.clear_queued_events(oid);
        assert_eq!(count_enclosed(&z), 0);
    }

    // ---- entity keys ----

    #[test]
    fn obj_entity_key_differs_by_slot() {
        let mut z = zone();
        z.add_obj(despawn_obj(3222, 3222, 100, 1), Some(111));
        z.add_obj(despawn_obj(3222, 3222, 100, 1), Some(222));
        assert_eq!((z.objs[0].slot(), z.objs[1].slot()), (0, 1));
        assert_ne!(z.objs[0].oid(), z.objs[1].oid());
    }

    #[test]
    fn obj_entity_key_differs_by_type() {
        let o1 = despawn_obj(3222, 3222, 100, 1);
        let o2 = despawn_obj(3222, 3222, 200, 1);
        assert_ne!(o1.oid(), o2.oid());
    }

    #[test]
    fn loc_entity_key_differs_by_layer() {
        let l1 = respawn_loc(3222, 3222, 100);
        let l2 = Loc::new(
            coord(3222, 3222),
            EntityLifeTime::Respawn,
            100,
            LocShape::WallDecorStraightNoOffset,
            LocAngle::North,
            true,
            true,
            1,
            1,
        );
        assert_ne!(l1.lid(), l2.lid());
    }

    // ---- multiple objs in same zone ----

    #[test]
    fn multiple_objs_same_coord_different_type() {
        let mut z = zone();
        let mut o1 = despawn_obj(3222, 3222, 100, 1);
        o1.set_last_clock(200);
        let mut o2 = despawn_obj(3222, 3222, 200, 1);
        o2.set_last_clock(200);
        z.objs.push(o1);
        z.objs.push(o2);
        z.remove_obj(3222, 3222, 100, None, None);
        assert_eq!(z.objs.len(), 1);
        assert_eq!(z.objs[0].id(), 200);
    }

    #[test]
    fn multiple_objs_same_type_different_receiver() {
        let mut z = zone();
        let mut o1 = despawn_obj(3222, 3222, 100, 1);
        o1.set_last_clock(200);
        let mut o2 = despawn_obj(3222, 3222, 100, 1);
        o2.set_last_clock(200);
        push_private_obj(&mut z, o1, 111);
        push_private_obj(&mut z, o2, 222);
        z.remove_obj(3222, 3222, 100, Some(111), None);
        assert_eq!(z.objs.len(), 1);
        assert_eq!(z.receivers.get(&z.objs[0].oid()), Some(&222));
    }

    // ---- visible_follows_events ----

    #[test]
    fn visible_follows_events_filters_by_receiver() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.add_obj(obj, Some(111));
        assert_eq!(z.visible_follows_events(111).count(), 1);
        assert_eq!(z.visible_follows_events(222).count(), 0);
    }

    // ---- compute_shared ----

    #[test]
    fn compute_shared_only_enclosed() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.add_obj(obj.clone(), None);
        let mut obj2 = despawn_obj(3222, 3222, 200, 1);
        obj2.set_last_clock(200);
        z.add_obj(obj2, Some(111));
        z.compute_shared();
        assert!(z.shared_bytes().is_some());
    }

    #[test]
    fn compute_shared_empty_when_no_enclosed() {
        let mut z = zone();
        let mut obj = despawn_obj(3222, 3222, 100, 1);
        obj.set_last_clock(200);
        z.add_obj(obj, Some(111));
        z.compute_shared();
        assert!(z.shared_bytes().is_none());
    }
}
