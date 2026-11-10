use crate::MAX_NPCS;
use crate::active_npc::ActiveNpc;
use crate::core::{Engine, next_free_id};
use rs_datastruct::HashTable;
use rs_entity::EntityLifeTime;
use rs_entity::NpcUid;
use rs_pack::cache::script::Script;
use rs_pack::types::BlockWalk;
use rs_vm::trigger::ServerTriggerType;
use std::ops::Deref;
use std::slice::Iter;
use std::sync::Arc;

/// Which trigger a queued [`NpcEvent`] runs (the reference engine's
/// `NpcEventType`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NpcEventKind {
    Spawn,
    Despawn,
}

/// An `ai_spawn` or `ai_despawn` script waiting to run (the reference
/// engine's `NpcEventRequest`), queued on `Engine::npc_events`.
pub struct NpcEvent {
    pub kind: NpcEventKind,
    pub uid: NpcUid,
    pub script: Arc<Script>,
}

/// The nids in processing order as [`NpcList::take_nids`] hands them out,
/// tagged with the `processing` revision they were read at, so an unchanged
/// order is reused without a refill. Reads as a `[u16]`.
pub struct NpcOrder {
    revision: u64,
    values: Vec<u16>,
}

impl Deref for NpcOrder {
    type Target = [u16];
    fn deref(&self) -> &[u16] {
        &self.values
    }
}

impl<'a> IntoIterator for &'a NpcOrder {
    type Item = &'a u16;
    type IntoIter = Iter<'a, u16>;
    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

/// The NPCs in the world, held in `Engine::npc_list`: a boxed slot per nid,
/// the order the phases process them in, and `despawn_pending`, the nids
/// whose slots the cleanup phase frees.
pub struct NpcList {
    pub npcs: Vec<Option<Box<ActiveNpc>>>,
    pub processing: HashTable<u16>,
    node_map: Vec<usize>,
    cursor: u16,
    nid_scratch: Option<NpcOrder>,
    pub despawn_pending: Vec<u16>,
}

impl NpcList {
    /// An empty list with every nid slot free, set so the first
    /// [`next_nid`](Self::next_nid) hands out nid 0.
    pub fn new() -> Self {
        let mut npcs = Vec::with_capacity(MAX_NPCS);
        npcs.resize_with(MAX_NPCS, || None);
        Self {
            npcs,
            processing: HashTable::new(8),
            node_map: vec![0; MAX_NPCS],
            cursor: (MAX_NPCS - 2) as u16,
            nid_scratch: Some(NpcOrder {
                revision: u64::MAX,
                values: Vec::with_capacity(MAX_NPCS),
            }),
            despawn_pending: Vec::new(),
        }
    }

    /// Takes the reusable nid buffer, filled with the current processing-order
    /// nids. Return it with [`put_nids`](Self::put_nids) to reuse the
    /// allocation. See `PlayerList::take_pids`.
    pub fn take_nids(&mut self) -> NpcOrder {
        let revision = self.processing.revision();
        let mut order = self.nid_scratch.take().unwrap_or_else(|| NpcOrder {
            revision: !revision,
            values: Vec::with_capacity(self.processing.len()),
        });
        if order.revision != revision {
            order.values.clear();
            order.values.extend(self.processing.iter().copied());
            order.revision = revision;
        }
        order
    }
    /// Returns the order taken by [`take_nids`](Self::take_nids) for reuse.
    pub fn put_nids(&mut self, order: NpcOrder) {
        self.nid_scratch = Some(order);
    }

    /// The next free nid, searched round robin from just after the last nid
    /// added; `None` when the list is full. Nids run from 0 to `MAX_NPCS - 2`.
    pub fn next_nid(&self) -> Option<u16> {
        next_free_id(self.cursor, (MAX_NPCS - 1) as u16, 0, |i| {
            self.npcs[i as usize].is_none()
        })
    }

    /// Puts `active` in slot `nid`, boxed, and links it into the processing
    /// order under `key`. The next [`next_nid`](Self::next_nid) search starts
    /// after `nid`.
    pub fn add(&mut self, nid: u16, active: ActiveNpc, key: i64) {
        self.cursor = nid;
        let node_idx = self.processing.put(key, nid);
        self.node_map[nid as usize] = node_idx;
        self.npcs[nid as usize] = Some(Box::new(active));
    }

    /// Empties slot `nid` and returns its NPC, unlinked from the processing
    /// order; `None` if the slot was empty.
    pub fn remove(&mut self, nid: u16) -> Option<Box<ActiveNpc>> {
        if self.npcs[nid as usize].is_some() {
            self.processing.unlink(self.node_map[nid as usize]);
        }
        self.npcs[nid as usize].take()
    }

    /// The NPC in slot `nid`, if any.
    pub fn get(&self, nid: u16) -> Option<&ActiveNpc> {
        self.npcs.get(nid as usize)?.as_deref()
    }

    /// The NPC in slot `nid`, mutably, if any.
    pub fn get_mut(&mut self, nid: u16) -> Option<&mut ActiveNpc> {
        self.npcs.get_mut(nid as usize)?.as_deref_mut()
    }

    /// Every nid in processing order, as a new `Vec`;
    /// [`take_nids`](Self::take_nids) reuses one snapshot instead.
    pub fn nids(&self) -> Vec<u16> {
        self.processing.iter().copied().collect()
    }

    /// How many slots hold an NPC, inactive ones included.
    pub fn count(&self) -> usize {
        self.processing.len()
    }
}

impl Engine {
    /// Puts an NPC into the world in the next free nid slot and queues its
    /// `ai_spawn` script.
    ///
    /// # Returns
    ///
    /// The NPC's uid, or `None` when every nid is taken.
    ///
    /// # Side Effects
    ///
    /// - Rewrites `npc.uid` with the allocated nid, keeping its type id, and
    ///   sets `pathing.presence` to its coord.
    /// - Fills the nid's slot through [`NpcList::add`], keyed in the
    ///   processing order by its packed coord, and registers the nid in the
    ///   zone at its coord.
    /// - Queues its `ai_spawn` script on `npc_events` if its type or category
    ///   has one ([`Engine::ai_spawn`]).
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::new` (the map's NPC spawns),
    /// `ScriptEngine::add_npc_spawned` (`npc_add`, `::npcadd`).
    /// **Calls:** [`NpcList::next_nid`], [`NpcList::add`], `ai_spawn`.
    pub fn add_npc(&mut self, mut active: ActiveNpc) -> Option<NpcUid> {
        let nid = self.npc_list.next_nid()?;
        let id = active.npc.uid.id();
        active.npc.uid = NpcUid::new(id, nid);
        let uid = active.npc.uid;
        let type_id = active.npc.base_type;
        let key = active.npc.pathing.coord.packed() as i64;
        active.npc.pathing.presence = active.npc.pathing.coord;
        self.npc_list.add(nid, active, key);
        if let Some(active) = self.get_npc(nid) {
            self.zones
                .zone_mut(
                    active.npc.pathing.coord.x(),
                    active.npc.pathing.coord.y(),
                    active.npc.pathing.coord.z(),
                )
                .add_npc(nid);
        }

        self.ai_spawn(uid, type_id);

        Some(uid)
    }

    /// The NPC's `ai_spawn` / `ai_despawn` script, if one is bound to its
    /// type or category.
    fn npc_trigger(&self, trigger: ServerTriggerType, type_id: u16) -> Option<Arc<Script>> {
        let category = self
            .cache
            .npcs
            .get_by_id(type_id)
            .and_then(|t| t.category)
            .map(|c| c as i32);
        self.script_by_key(trigger, Some(type_id as i32), category)
    }

    /// Queues the NPC's `ai_spawn` script, if it has one. Like the reference
    /// engine's `addNpc`, it doesn't run here: the NPC phase runs queued
    /// events before any NPC turns (see `Engine::process_npc_events`), so a
    /// spawn during input runs the same tick and a later one the next tick.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `add_npc`, the NPC phase's respawn.
    pub fn ai_spawn(&mut self, uid: NpcUid, type_id: u16) {
        if let Some(script) = self.npc_trigger(ServerTriggerType::AiSpawn, type_id) {
            self.npc_events.add_tail(NpcEvent {
                kind: NpcEventKind::Spawn,
                uid,
                script,
            });
        }
    }

    /// An `npc_add` NPC's duration ran out: takes it out of the world now and
    /// queues its `ai_despawn` script, which runs before the NPCs next tick,
    /// as in the reference engine. The slot stays taken (and the NPC
    /// readable) until that script has run; with no script, cleanup frees it
    /// this tick. `npc_del` never fires `ai_despawn`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::process_npc` (`phases/npc.rs`).
    /// **Calls:** `deactivate_npc_common`.
    pub(crate) fn despawn_npc(&mut self, nid: u16) {
        let Some((uid, type_id)) = self
            .npc_list
            .get(nid)
            .filter(|active| active.npc.active)
            .map(|active| (active.npc.uid, active.npc.base_type))
        else {
            return;
        };
        let script = self.npc_trigger(ServerTriggerType::AiDespawn, type_id);
        if !self.deactivate_npc_common(nid) {
            return;
        }
        match script {
            Some(script) => self.npc_events.add_tail(NpcEvent {
                kind: NpcEventKind::Despawn,
                uid,
                script,
            }),
            None => self.npc_list.despawn_pending.push(nid),
        }
    }

    /// Shared teardown for the NPC deactivation paths.
    ///
    /// Marks the NPC inactive, clears its script state, removes it from its
    /// zone, the collision map, the renderer and the snapshot list, and
    /// schedules a respawn for `Respawn` lifecycle NPCs. Callers are
    /// responsible for their own pre-checks and for releasing the slot of a
    /// `Despawn` lifecycle NPC, which each path does differently.
    ///
    /// Clearing `active_script`/`delayed`/`delayed_until` here is load-bearing,
    /// not tidiness: `EntityState::check_delay` only runs for *active* NPCs (see
    /// `phases::npc::process_npc`), while the respawn countdown is gated on
    /// `!delayed`. An NPC deactivated mid-delay would otherwise keep `delayed`
    /// set with nothing left able to clear it, so its countdown would never run
    /// and the spawn would be lost for good.
    ///
    /// # Arguments
    ///
    /// * `nid` -- The NPC id to tear down.
    ///
    /// # Returns
    ///
    /// `true` when the NPC has a `Despawn` lifecycle and the caller must still
    /// release its slot; `false` when a respawn was scheduled instead, or when
    /// the slot was already empty.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `deactivate_npc`, `despawn_npc`, `emergency_deactivate_npc`.
    /// **Calls:** `rsmod::change_npc`, `rsmod::change_player`.
    fn deactivate_npc_common(&mut self, nid: u16) -> bool {
        // Its type's respawn rate, scaled down by the player count like the
        // reference engine's `removeNpc`.
        let Some(rate) = self.npc_list.get(nid).map(|active| {
            self.cache
                .npcs
                .get_by_id(active.npc.uid.id())
                .map_or(100, |t| t.respawnrate as u64)
        }) else {
            return false;
        };
        let respawn = self.scale_by_player_count(rate) as u32;
        let Some(active) = self.npc_list.npcs[nid as usize].as_mut() else {
            return false;
        };
        active.npc.active = false;
        active.npc.state.active_script = None;
        active.npc.state.delayed = false;
        active.npc.state.delayed_until = 0;

        let coord = active.npc.pathing.presence;
        self.zones
            .zone_mut(coord.x(), coord.y(), coord.z())
            .remove_npc(nid);

        let block_walk = active.block_walk();
        let size = active.npc.pathing.size;
        match block_walk {
            BlockWalk::Npc => {
                rsmod::change_npc(coord.x(), coord.z(), coord.y(), size, false);
            }
            BlockWalk::All => {
                rsmod::change_npc(coord.x(), coord.z(), coord.y(), size, false);
                rsmod::change_player(coord.x(), coord.z(), coord.y(), size, false);
            }
            _ => {}
        }

        self.npc_renderer.remove_permanent(nid);
        self.npc_snapshots[nid as usize].clear();

        if active.npc.lifecycle == EntityLifeTime::Respawn {
            active.npc.lifecycle_at = Some(respawn);
            false
        } else {
            true
        }
    }

    /// Deactivates an NPC, removing it from the world without freeing its slot.
    ///
    /// Marks the NPC as inactive, removes it from its zone, clears its
    /// collision flags, and removes it from the renderer. If the NPC has a
    /// `Respawn` lifecycle, sets `lifecycle_at` so the NPC phase can
    /// re-activate it later. Doesn't fire `ai_despawn`: only an `npc_add`
    /// NPC's despawn does, in the NPC phase.
    ///
    /// # Arguments
    ///
    /// * `nid` -- The NPC id to deactivate.
    ///
    /// # Side Effects
    ///
    /// - Sets `active.npc.active = false`.
    /// - Clears `active_script`, `delayed` and `delayed_until`.
    /// - Removes the NPC from its zone and from the collision map.
    /// - Removes the nid from the NPC renderer.
    /// - Sets `lifecycle_at` for `Respawn` lifecycle NPCs.
    /// - Queues `Despawn` lifecycle NPCs on `despawn_pending` so the
    ///   cleanup phase frees their slots without a full-list scan.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptEngine::remove_npc` (`npc_del`), `free_region`.
    /// **Calls:** `deactivate_npc_common`.
    pub fn deactivate_npc(&mut self, nid: u16) {
        if !self.npc_list.npcs[nid as usize]
            .as_ref()
            .is_some_and(|active| active.npc.active)
        {
            return;
        }

        if self.deactivate_npc_common(nid) {
            self.npc_list.despawn_pending.push(nid);
        }
    }

    /// Takes NPC `nid` out of the world and frees its slot at once, without
    /// clearing its collision flags, scheduling a respawn or queueing a
    /// script. `npc_del` does not come here: it reaches
    /// `ScriptEngine::remove_npc`, which deactivates the NPC through
    /// [`Engine::deactivate_npc`].
    ///
    /// # Returns
    ///
    /// The removed NPC, or `None` if slot `nid` was empty.
    ///
    /// # Side Effects
    ///
    /// - Drops the nid's NPC renderer state and clears its NPC snapshot.
    /// - Removes the nid from the zone at its `presence`.
    /// - Frees slot `nid` through [`NpcList::remove`].
    ///
    /// # Call Stack
    ///
    /// **Called by:** the rs-test integration tests (`pirates_treasure`).
    /// **Calls:** `NpcRenderer::remove_permanent`, [`NpcList::remove`].
    pub fn remove_npc(&mut self, nid: u16) -> Option<Box<ActiveNpc>> {
        self.npc_renderer.remove_permanent(nid);
        self.npc_snapshots[nid as usize].clear();
        if let Some(active) = self.get_npc(nid) {
            let coord = active.npc.pathing.presence;
            self.zones
                .zone_mut(coord.x(), coord.y(), coord.z())
                .remove_npc(nid);
        }
        self.npc_list.remove(nid)
    }

    /// The NPC in slot `nid` of the NPC list, if any.
    pub fn get_npc(&self, nid: u16) -> Option<&ActiveNpc> {
        self.npc_list.get(nid)
    }

    /// The NPC in slot `nid` of the NPC list, mutably, if any.
    pub fn get_npc_mut(&mut self, nid: u16) -> Option<&mut ActiveNpc> {
        self.npc_list.get_mut(nid)
    }

    /// Forcibly deactivates an NPC without running its despawn script.
    ///
    /// Used as a last resort when an NPC's tick processing panics. Shares its
    /// teardown with [`Engine::deactivate_npc`] via
    /// `Engine::deactivate_npc_common` (zone removal, collision cleanup,
    /// renderer removal, script-state clearing, respawn scheduling) and, like
    /// it, queues no `ai_despawn` script. For `Despawn` lifecycle NPCs, the
    /// slot is freed here and now rather than deferred to the cleanup phase.
    ///
    /// # Arguments
    ///
    /// * `nid` -- The NPC id to force-deactivate.
    ///
    /// # Side Effects
    ///
    /// - Sets `active.npc.active = false`.
    /// - Clears `active_script`, `delayed` and `delayed_until`.
    /// - Removes the NPC from its zone and collision map.
    /// - Removes the nid from the NPC renderer.
    /// - For `Respawn` NPCs, sets `lifecycle_at`.
    /// - For `Despawn` NPCs, unlinks the nid from the processing order and
    ///   frees the slot.
    ///
    /// # Call Stack
    ///
    /// **Called by:** panic recovery in the NPC and info phases.
    pub fn emergency_deactivate_npc(&mut self, nid: u16) {
        let Some(active) = self.npc_list.npcs[nid as usize].as_ref() else {
            return;
        };
        if !active.npc.active {
            return;
        }
        if self.deactivate_npc_common(nid) {
            self.npc_list.remove(nid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NpcList;
    #[test]
    fn snapshots_refresh_after_same_length_remove_and_add() {
        let mut list = NpcList::new();
        let first = list.processing.put(0, 11);
        list.processing.put(8, 22);
        let old = list.take_nids();
        assert_eq!(&*old, &[11, 22]);
        list.processing.unlink(first);
        list.processing.put(16, 33);
        list.put_nids(old);
        assert_eq!(&*list.take_nids(), &[22, 33]);
    }
    #[test]
    fn returning_an_old_outer_snapshot_cannot_hide_nested_mutations() {
        let mut list = NpcList::new();
        list.processing.put(0, 11);
        let outer = list.take_nids();
        list.processing.put(8, 22);
        let nested = list.take_nids();
        assert_eq!(&*nested, &[11, 22]);
        list.put_nids(nested);
        list.put_nids(outer);
        let current = list.take_nids();
        assert_eq!(&*current, &[11, 22]);
        list.put_nids(current);
        assert_eq!(&*list.take_nids(), &[11, 22]);
    }
    #[test]
    fn value_changes_and_bucket_order_are_reflected() {
        let mut list = NpcList::new();
        list.processing.put(7, 77);
        let first = list.processing.put(0, 11);
        list.processing.put(8, 22);
        let old = list.take_nids();
        assert_eq!(&*old, &[11, 22, 77]);
        *list.processing.value_mut(first) = 33;
        list.put_nids(old);
        assert_eq!(&*list.take_nids(), &[33, 22, 77]);
    }
}
