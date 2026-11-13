use crate::core::{Engine, PendingZoneEvent, lifecycle_clock};
use rs_entity::PlayerUid;
use rs_entity::StaffModLevel;
use rs_entity::{EntityLifeTime, Obj, ObjSerial, REVEAL_TICKS};
use rs_grid::CoordGrid;
use rs_inv::{Item, STACK_LIMIT};
use rs_protocol::network::game::server::obj_count::ObjCount;
use rs_zone::{ZoneEventType, ZoneMessage};
use std::collections::BTreeMap;
use std::time::Instant;

/// A request to spawn a ground object after a delay (measured in game ticks).
///
/// Queued into `Engine::obj_delayed_queue` and processed during the world phase.
/// Once the `delay` expires, `item` is put down at `coord` with the specified
/// `receiver` and `duration` (despawn lifetime).
pub struct ObjDelayedRequest {
    pub coord: CoordGrid,
    pub item: Item,
    pub receiver: Option<PlayerUid>,
    pub duration: u64,
    pub delay: i32,
}

impl Engine {
    /// Whether players of staff level `level` are kept apart from others
    /// on this world (see [`Engine::staff_isolation`]).
    pub fn isolates(&self, level: StaffModLevel) -> bool {
        self.staff_isolation && level.is_isolated()
    }

    /// Whether `uid` is an online player whose drops stay theirs. Free when
    /// no isolated staff are online.
    fn isolated_owner(&self, uid: PlayerUid) -> bool {
        if !self.staff_isolation || self.player_list.isolated == 0 {
            return false;
        }
        let user37 = uid.username37();
        let owner = self
            .player_list
            .get(uid.pid())
            .filter(|p| p.player.uid.username37() == user37)
            // A delayed drop's owner may have relogged into another slot.
            .or_else(|| self.player_list.get(self.find_pid_by_user37(user37)?));
        owner.is_some_and(|p| p.player.staff_mod_level.is_isolated())
    }

    /// Adds a ground object to the world at `coord`.
    ///
    /// For stackable items owned by a specific receiver, attempts to merge
    /// the count into an existing stack via [`Engine::merge_obj`] before
    /// creating a new entity. If a receiver is specified, the object starts
    /// as receiver-only and a `ObjReveal` event is scheduled to make it
    /// globally visible after [`REVEAL_TICKS`]. An `ObjDelete` event is
    /// always scheduled to despawn the object after `duration` ticks.
    ///
    /// # Arguments
    ///
    /// * `coord` -- The tile to put the object on.
    /// * `obj` -- The ground object to place.
    /// * `receiver` -- Optional player who can see the object before it is
    ///   revealed globally.
    /// * `duration` -- Lifetime in game ticks before automatic removal.
    ///
    /// # Returns
    ///
    /// The serial the object was given, or [`ObjSerial::NONE`] when
    /// it merged into a stack already there: a script's reference to the drop
    /// then finds nothing, as the drop itself never reached the floor.
    ///
    /// # Side Effects
    ///
    /// - Inserts the object into the zone's obj list.
    /// - Schedules `ObjReveal` and/or `ObjDelete` zone events.
    /// - Marks the zone as dirty via [`Engine::track_zone`].
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptEngine::add_obj`, world phase obj-delayed processing.
    /// **Calls:** `merge_obj`, `schedule_zone_event`, `track_zone`.
    pub fn add_obj(
        &mut self,
        coord: CoordGrid,
        mut obj: Obj,
        receiver: Option<PlayerUid>,
        duration: u64,
    ) -> ObjSerial {
        let receiver37 = receiver.map(|uid| uid.username37());
        let obj_type = self.cache.objs.get_by_id(obj.id());

        let stackable = obj_type.is_some_and(|t| t.stackable);

        if stackable
            && obj.lifetime() == EntityLifeTime::Despawn
            && let Some(r) = receiver37
            && self.merge_obj(coord, &obj, r, duration)
        {
            return ObjSerial::NONE;
        }

        let clock = lifecycle_clock(self.clock, duration);
        obj.set_last_clock(clock);
        if let Some(uid) = receiver {
            let r = uid.username37();
            let reveal_clock = lifecycle_clock(self.clock, REVEAL_TICKS as u64);

            // Isolated staff's drops (their deaths and kills too) stay theirs.
            let revealable = obj_type
                .is_some_and(|t| obj_revealable(t.tradeable, t.members, self.members))
                && !self.isolated_owner(uid);

            if revealable && reveal_clock < clock {
                self.schedule_zone_event(
                    reveal_clock,
                    PendingZoneEvent::ObjReveal {
                        coord,
                        id: obj.id(),
                        receiver37: r,
                    },
                );
            }
        }

        self.schedule_zone_event(
            clock,
            PendingZoneEvent::ObjDelete {
                coord,
                id: obj.id(),
                clock,
            },
        );

        let (ox, oy, oz) = (coord.x(), coord.y(), coord.z());
        self.obj_appeared.insert((coord, obj.id()), Instant::now());
        let zone = self.zones.zone_mut(ox, oy, oz);
        let serial = zone.add_obj(obj, receiver37);
        self.track_zone(ox, oy, oz);
        serial
    }

    /// Puts `item` down as a despawning obj, its varobjs with it.
    pub fn add_item_obj(
        &mut self,
        coord: CoordGrid,
        item: Item,
        receiver: Option<PlayerUid>,
        duration: u64,
    ) -> ObjSerial {
        let obj = Obj::new(coord, EntityLifeTime::Despawn, item.obj, item.num);
        let serial = self.add_obj(coord, obj, receiver, duration);
        #[cfg(since_647)]
        if item.var != 0 {
            self.zones
                .zone_mut(coord.x(), coord.y(), coord.z())
                .set_obj_var(coord.x(), coord.z(), item.obj, serial, item.var);
        }
        serial
    }

    /// Forgets objs that appeared too long ago for a reaction to them.
    pub(crate) fn forget_appeared_objs(&mut self) {
        const KEEP: std::time::Duration = std::time::Duration::from_secs(15);
        if !self.obj_appeared.is_empty() {
            let now = Instant::now();
            self.obj_appeared
                .retain(|_, at| now.saturating_duration_since(*at) < KEEP);
        }
    }

    /// Attempts to merge a stackable object's count into an existing stack
    /// at the same coordinate, owned by the same receiver.
    ///
    /// Only merges if the existing object is also a `Despawn` entity and the
    /// combined count does not exceed [`STACK_LIMIT`]. On success, sends an
    /// `ObjCount` zone message to update clients and reschedules the deletion
    /// event.
    ///
    /// # Arguments
    ///
    /// * `obj` -- The new object whose count should be merged.
    /// * `receiver` -- Base37 username of the receiver to match.
    /// * `duration` -- New despawn lifetime (resets the deletion timer).
    ///
    /// # Returns
    ///
    /// `true` if the merge succeeded (caller should *not* create a new obj),
    /// `false` if no compatible stack was found or the merge would exceed the
    /// stack limit.
    ///
    /// # Side Effects
    ///
    /// - Mutates the existing obj's `count` and `last_clock` in the zone.
    /// - Queues an `ObjCount` zone event for client updates.
    /// - Schedules a new `ObjDelete` event for the merged stack.
    /// - Marks the zone as dirty.
    fn merge_obj(&mut self, coord: CoordGrid, obj: &Obj, receiver: u64, duration: u64) -> bool {
        let clock = lifecycle_clock(self.clock, duration);
        let zone = self.zones.zone_mut(coord.x(), coord.y(), coord.z());
        let Some(idx) = zone.get_obj_of_receiver(coord.x(), coord.z(), obj.id(), receiver) else {
            return false;
        };
        if zone.objs[idx].lifetime() != EntityLifeTime::Despawn {
            return false;
        }
        let next_count = obj.count() + zone.objs[idx].count();
        if next_count > STACK_LIMIT {
            return false;
        }

        let old_count = zone.objs[idx].count();
        zone.objs[idx].set_count(next_count);
        zone.objs[idx].set_last_clock(clock);

        let oid = zone.objs[idx].oid();
        let message = ZoneMessage::ObjCount(ObjCount {
            coord: obj.packed_zone_coord(),
            id: obj.id(),
            old_count: old_count.clamp(0, 65535) as u16,
            new_count: next_count.clamp(0, 65535) as u16,
        });
        zone.queue_event(Some(oid), ZoneEventType::Follows, Some(receiver), message);
        self.track_zone(coord.x(), coord.y(), coord.z());

        self.schedule_zone_event(
            clock,
            PendingZoneEvent::ObjDelete {
                coord,
                id: obj.id(),
                clock,
            },
        );
        true
    }

    /// Removes a ground object from the world, optionally scheduling a respawn.
    ///
    /// If `duration` is non-zero the respawn delay is scaled by the current
    /// player count (via [`Engine::scale_by_player_count`]) and an `ObjAdd`
    /// event is scheduled to re-create the object after that period.
    ///
    /// # Arguments
    ///
    /// * `coord` -- World coordinate of the object.
    /// * `id` -- Object type id.
    /// * `serial` -- The object's [`serial`](Obj::serial).
    /// * `duration` -- Respawn delay in game ticks (0 = no respawn).
    ///
    /// # Returns
    ///
    /// The count removed, or `None` if that object wasn't there.
    ///
    /// # Side Effects
    ///
    /// - Removes the object from the zone's obj list.
    /// - Schedules an `ObjAdd` respawn event if `duration > 0` and the object
    ///   respawns.
    /// - Marks the zone as dirty.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptEngine::remove_obj`, obj interaction scripts.
    /// **Calls:** `scale_by_player_count`, `schedule_zone_event`, `track_zone`.
    pub fn remove_obj(
        &mut self,
        coord: CoordGrid,
        id: u16,
        serial: ObjSerial,
        duration: u64,
    ) -> Option<u32> {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        let respawn_at = if duration > 0 {
            let scaled = self.scale_by_player_count(duration);
            Some(lifecycle_clock(self.clock, scaled))
        } else {
            None
        };
        let (obj, receiver37) = self
            .zones
            .zone_mut(x, y, z)
            .remove_obj(x, z, id, serial, respawn_at)?;
        // Only a private obj has a reveal pending.
        if let Some(receiver) = receiver37 {
            cancel_one_pending_obj_reveal(&mut self.pending_zone_events, coord, id, receiver);
        }
        // A dropped obj never comes back; the event would bring back a taken
        // map spawn of the same id on this tile early.
        if let Some(clock) = respawn_at
            && obj.lifetime() == EntityLifeTime::Respawn
        {
            self.schedule_zone_event(clock, PendingZoneEvent::ObjAdd { coord, id });
        }
        self.track_zone(x, y, z);
        Some(obj.count())
    }
}

/// Whether a privately-dropped obj should ever be revealed to all players.
///
/// Untradeable objs are never revealed (they stay receiver-only until they
/// despawn); members objs are revealed only on a members world.
const fn obj_revealable(tradeable: bool, members: bool, world_members: bool) -> bool {
    tradeable && (!members || world_members)
}

/// Cancels the earliest pending [`PendingZoneEvent::ObjReveal`] scheduled for the
/// given `(coord, id, receiver37)`, if one exists.
///
/// Called when a privately-dropped obj is removed (e.g. picked up) before its
/// reveal fires. Unlike `ObjDelete` -- which `remove_obj_by_clock` clock-guards
/// so a stale event cannot delete a re-dropped obj -- `reveal_obj` matches only
/// coord/id/receiver37, so a leftover reveal would reveal a later same-tile/id
/// re-drop by the same receiver too early. Exactly ONE reveal is removed so a
/// multi-drop (two identical non-stackable objs on one tile, each with its own
/// scheduled reveal) keeps one reveal per remaining obj.
fn cancel_one_pending_obj_reveal(
    pending: &mut BTreeMap<u32, Vec<PendingZoneEvent>>,
    coord: CoordGrid,
    id: u16,
    receiver37: u64,
) {
    let target = PendingZoneEvent::ObjReveal {
        coord,
        id,
        receiver37,
    };
    for events in pending.values_mut() {
        if let Some(pos) = events.iter().position(|e| *e == target) {
            events.remove(pos);
            return;
        }
    }
}
