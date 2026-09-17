/// Stack mode for a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackMode {
    /// Stack based on item's stackable property (normal inventory).
    #[default]
    Normal,
    /// Always stack all items (bank, shop).
    Always,
    /// Never stack (equipment, trade).
    Never,
}

/// Maximum count that can live in a single stacked slot.
pub const STACK_LIMIT: u32 = 0x7FFF_FFFF; // 2147483647

/// An item container (inventory, bank, equipment).
#[derive(Debug, Clone, Default)]
pub struct Inventory {
    pub capacity: usize,
    pub slots: Vec<Option<Item>>,
    pub stack_mode: StackMode,
    pub dirty: bool,
    pub dirty_slots: Vec<u16>,
    pub stockobj: Box<[u16]>,
}

impl Inventory {
    /// Creates a new inventory with the given capacity and default `Normal` stack mode.
    ///
    /// All slots are initialized to `None` (empty). The `dirty` flag starts as `false`.
    ///
    /// # Arguments
    /// * `capacity` - The number of slots in this inventory (e.g. 28 for player backpack, 800 for bank).
    ///
    /// # Returns
    /// A new `Inventory` with `StackMode::Normal` and all empty slots.
    ///
    /// # Call Stack
    /// **Called by:** `Engine::get_shared_inv`, player save loading, tests.
    /// **Calls:** nothing.
    pub fn new(capacity: usize) -> Self {
        Self::with_stack_mode(capacity, StackMode::Normal)
    }

    /// Creates a new inventory with the given capacity and explicit stack mode.
    ///
    /// Use this for containers that need non-default stacking behavior:
    /// `StackMode::Always` for banks/shops, `StackMode::Never` for equipment/trade.
    ///
    /// # Arguments
    /// * `capacity` - The number of slots in this inventory.
    /// * `stack_mode` - The stacking policy (`Normal`, `Always`, or `Never`).
    ///
    /// # Returns
    /// A new `Inventory` with the specified stack mode and all empty slots.
    ///
    /// # Call Stack
    /// **Called by:** `Engine::get_shared_inv`, player save loading (`player_save.rs`),
    /// VM `INV_TRANSMIT` op.
    /// **Calls:** nothing.
    pub fn with_stack_mode(capacity: usize, stack_mode: StackMode) -> Self {
        Self {
            capacity,
            slots: vec![None; capacity],
            stack_mode,
            dirty: false,
            dirty_slots: Vec::new(),
            stockobj: Box::default(),
        }
    }

    /// Determines whether an item should be stacked based on the inventory's stack mode.
    ///
    /// In `Normal` mode, defers to the item's own `stackable` property.
    /// In `Always` mode, unconditionally returns `true` (bank/shop behavior).
    /// In `Never` mode, unconditionally returns `false` (equipment/trade behavior).
    ///
    /// # Arguments
    /// * `stackable` - The item's inherent stackable property from its object definition.
    ///
    /// # Returns
    /// `true` if items should merge into existing stacks, `false` if each item occupies its own slot.
    ///
    /// # Call Stack
    /// **Called by:** `Inventory::add`.
    /// **Calls:** nothing.
    const fn should_stack(&self, stackable: bool) -> bool {
        match self.stack_mode {
            StackMode::Normal => stackable,
            StackMode::Always => true,
            StackMode::Never => false,
        }
    }

    /// Records that a single slot changed: sets `dirty` and appends the slot index.
    ///
    /// Duplicate indices are allowed (multiple mutations to the same slot in a tick);
    /// [`Inventory::collect_dirty`] deduplicates on read.
    #[inline]
    fn mark_dirty(&mut self, slot: u16) {
        self.dirty = true;
        self.dirty_slots.push(slot);
    }

    /// Clears the dirty flag and the set of changed slots.
    ///
    /// Called once per tick (after all viewers have transmitted) by the engine
    /// cleanup phase, so the next tick starts with a clean change set.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
        self.dirty_slots.clear();
    }

    /// Returns the deduplicated, sorted set of changed slots paired with their
    /// current contents, suitable for a partial inventory update.
    pub fn collect_dirty(&self) -> Vec<(u16, Option<(u16, i32)>)> {
        let mut slots = self.dirty_slots.clone();
        slots.sort_unstable();
        slots.dedup();
        slots
            .into_iter()
            .filter(|&s| (s as usize) < self.capacity)
            .map(|s| (s, self.get(s).map(|i| i.as_pair())))
            .collect()
    }

    /// Add an item. Stackable items merge into existing stacks.
    /// Returns the count that could NOT be added (0 = full success).
    pub fn add(&mut self, item_id: u16, count: u32, stackable: bool) -> u32 {
        if count == 0 {
            return 0;
        }

        if self.should_stack(stackable) {
            if let Some(slot) = self
                .slots
                .iter()
                .position(|s| s.as_ref().is_some_and(|i| i.obj == item_id))
            {
                let item = self.slots[slot].as_mut().unwrap();
                let new_count = item.num as u64 + count as u64;
                if new_count > STACK_LIMIT as u64 {
                    let can_add = STACK_LIMIT - item.num;
                    item.num = STACK_LIMIT;
                    self.mark_dirty(slot as u16);
                    return count - can_add;
                }
                item.num = new_count as u32;
                self.mark_dirty(slot as u16);
                return 0;
            }
            if let Some(slot) = self.slots.iter().position(|s| s.is_none()) {
                let actual = count.min(STACK_LIMIT);
                self.slots[slot] = Some(Item {
                    obj: item_id,
                    num: actual,
                });
                self.mark_dirty(slot as u16);
                return count - actual;
            }
            count
        } else {
            let mut remaining = count;
            for i in 0..self.slots.len() {
                if remaining == 0 {
                    break;
                }
                if self.slots[i].is_none() {
                    self.slots[i] = Some(Item {
                        obj: item_id,
                        num: 1,
                    });
                    self.mark_dirty(i as u16);
                    remaining -= 1;
                }
            }
            remaining
        }
    }

    /// Remove up to `count` of an item by ID. Returns the amount actually removed.
    ///
    /// If the item id is one of this inventory's [`stockobj`](Self::stockobj) entries
    /// (i.e. a shop's base stock), a slot emptied to count 0 is kept occupied at
    /// count 0 rather than cleared, so the item shows as out-of-stock and restocks
    /// over time. Non-stock items are cleared from their slot as usual.
    pub fn delete(&mut self, item_id: u16, count: u32) -> u32 {
        let stock_obj = self.stockobj.contains(&item_id);
        let mut remaining = count;
        for i in 0..self.slots.len() {
            if remaining == 0 {
                break;
            }
            let Some(item) = self.slots[i].as_ref() else {
                continue;
            };
            if item.obj != item_id {
                continue;
            }
            let num = item.num;
            let remove_count = num.min(remaining);
            remaining -= remove_count;
            let new_count = num - remove_count;
            if new_count == 0 && !stock_obj {
                self.slots[i] = None;
            } else {
                self.slots[i].as_mut().unwrap().num = new_count;
            }
            self.mark_dirty(i as u16);
        }
        count - remaining
    }

    /// Remove `count` items from a specific slot. Returns true on success.
    pub fn remove(&mut self, slot: u16, count: u32) -> bool {
        if let Some(item) = self.slots.get_mut(slot as usize).and_then(|s| s.as_mut()) {
            if item.num <= count {
                self.slots[slot as usize] = None;
            } else {
                item.num -= count;
            }
            self.mark_dirty(slot);
            true
        } else {
            false
        }
    }

    /// Removes all items from every slot in the inventory.
    ///
    /// Every slot is set to `None`. Always marks the inventory as dirty, even
    /// if it was already empty.
    ///
    /// # Side Effects
    /// * Sets all slots to `None`.
    /// * Sets `self.dirty = true`.
    ///
    /// # Call Stack
    /// **Called by:** VM `INV_CLEAR` op.
    /// **Calls:** nothing.
    pub fn clear(&mut self) {
        self.slots.fill(None);
        self.dirty = true;
        self.dirty_slots.extend(0..self.capacity as u16);
    }

    /// Places an item directly into a specific slot, overwriting any existing content.
    ///
    /// If the slot index is out of bounds, the call is silently ignored (no panic).
    /// This bypasses all stacking logic -- the item is placed exactly as specified.
    ///
    /// # Arguments
    /// * `slot` - The slot index to write to.
    /// * `id` - The item (object) ID to place.
    /// * `count` - The quantity to place in the slot.
    ///
    /// # Side Effects
    /// * Overwrites the contents of `slots[slot]`.
    /// * Sets `self.dirty = true` on success.
    ///
    /// # Call Stack
    /// **Called by:** VM `INV_SETSLOT` op, `Engine::get_shared_inv` (stock initialization),
    /// player save loading, shop restock (`cleanup.rs`), `Inventory::move_to_slot_to`.
    /// **Calls:** nothing.
    pub fn set(&mut self, slot: u16, id: u16, count: u32) {
        if let Some(s) = self.slots.get_mut(slot as usize) {
            *s = Some(Item {
                obj: id,
                num: count,
            });
            self.mark_dirty(slot);
        }
    }

    /// Swaps the contents of two slots within the same inventory.
    ///
    /// If either slot index is out of bounds, the call is silently ignored.
    /// Works correctly when one or both slots are empty (the empty `None` is
    /// simply swapped to the other position).
    ///
    /// # Arguments
    /// * `a` - The first slot index.
    /// * `b` - The second slot index.
    ///
    /// # Side Effects
    /// * Swaps `slots[a]` and `slots[b]`.
    /// * Sets `self.dirty = true` on success.
    ///
    /// # Call Stack
    /// **Called by:** VM `INV_MOVETOSLOT` op (when source and destination inventory are the same).
    /// **Calls:** `Inventory::valid_slot`.
    pub fn move_to_slot(&mut self, a: u16, b: u16) {
        if !self.valid_slot(a) || !self.valid_slot(b) {
            return;
        }
        self.slots.swap(a as usize, b as usize);
        self.mark_dirty(a);
        self.mark_dirty(b);
    }

    /// Clears a specific slot, removing whatever item was there.
    ///
    /// If the slot index is out of bounds, the call is silently ignored.
    ///
    /// # Arguments
    /// * `slot` - The slot index to clear.
    ///
    /// # Side Effects
    /// * Sets `slots[slot]` to `None`.
    /// * Sets `self.dirty = true` on success.
    ///
    /// # Call Stack
    /// **Called by:** VM `INV_DELSLOT` op, `Inventory::move_from_slot`,
    /// `Inventory::move_from_slot_to`, `Inventory::move_to_slot_to`.
    /// **Calls:** nothing.
    pub fn delete_slot(&mut self, slot: u16) {
        if let Some(s) = self.slots.get_mut(slot as usize) {
            *s = None;
            self.mark_dirty(slot);
        }
    }

    /// Returns the number of empty (unoccupied) slots in the inventory.
    ///
    /// # Returns
    /// The count of slots that are `None`.
    ///
    /// # Call Stack
    /// **Called by:** VM `INV_FREESPACE` op, `inv_itemspace` helper (for capacity checks).
    /// **Calls:** nothing.
    pub fn freespace(&self) -> usize {
        self.slots.iter().filter(|s| s.is_none()).count()
    }

    /// Returns `true` if every slot in the inventory is occupied.
    ///
    /// Equivalent to `self.freespace() == 0`.
    ///
    /// # Returns
    /// `true` if no slot is `None`, `false` otherwise.
    ///
    /// # Call Stack
    /// **Called by:** tests.
    /// **Calls:** nothing.
    pub fn is_full(&self) -> bool {
        self.slots.iter().all(|s| s.is_some())
    }

    /// Total count of `item_id` across all slots.
    pub fn total(&self, item_id: u16) -> u32 {
        self.slots
            .iter()
            .filter_map(|s| s.as_ref())
            .filter(|i| i.obj == item_id)
            .map(|i| i.num)
            .fold(0u32, |a, b| a.saturating_add(b))
    }

    /// Returns a reference to the item in the given slot, or `None` if the slot
    /// is empty or out of bounds.
    ///
    /// # Arguments
    /// * `slot` - The slot index to look up.
    ///
    /// # Returns
    /// `Some(&Item)` if the slot contains an item, `None` otherwise.
    ///
    /// # Call Stack
    /// **Called by:** VM `INV_GETNUM` / `INV_GETOBJ` / `INV_DROPSLOT` ops,
    /// handler validation (`inv_button.rs`, `inv_buttond.rs`, `opheld.rs`, `opheldu.rs`),
    /// shop restock (`cleanup.rs`), `Inventory::has_at`, `Inventory::move_from_slot`,
    /// `Inventory::move_from_slot_to`, `Inventory::move_to_slot_to`,
    /// `Inventory::collect_slots_at`.
    /// **Calls:** nothing.
    pub fn get(&self, slot: u16) -> Option<&Item> {
        self.slots.get(slot as usize)?.as_ref()
    }

    /// Checks whether a specific slot contains a specific item ID.
    ///
    /// Returns `false` if the slot is empty, out of bounds, or holds a different item.
    ///
    /// # Arguments
    /// * `slot` - The slot index to check.
    /// * `item_id` - The expected item (object) ID.
    ///
    /// # Returns
    /// `true` if `slots[slot]` holds an item with `obj == item_id`.
    ///
    /// # Call Stack
    /// **Called by:** handler validation (`inv_button.rs`, `opheld.rs`, `opheldu.rs`).
    /// **Calls:** `Inventory::get`.
    pub fn has_at(&self, slot: u16, item_id: u16) -> bool {
        self.get(slot).is_some_and(|item| item.obj == item_id)
    }

    /// Checks whether a slot index is within the inventory's capacity.
    ///
    /// # Arguments
    /// * `slot` - The slot index to validate.
    ///
    /// # Returns
    /// `true` if `slot < capacity`, `false` otherwise.
    ///
    /// # Call Stack
    /// **Called by:** handler validation (`inv_button.rs`, `inv_buttond.rs`, `opheld.rs`,
    /// `opheldu.rs`), `Inventory::move_to_slot`, `Inventory::move_from_slot`,
    /// `Inventory::move_from_slot_to`, `Inventory::move_to_slot_to`.
    /// **Calls:** nothing.
    pub fn valid_slot(&self, slot: u16) -> bool {
        (slot as usize) < self.capacity
    }

    /// Removes and returns the item in `slot`, clearing the slot.
    ///
    /// Returns `None` (leaving the inventory untouched) if `slot` is out of
    /// bounds or empty. On success the slot is cleared via [`delete_slot`](Self::delete_slot)
    /// before the item is returned, matching the read-then-clear order used by
    /// the `move_from_slot*` helpers.
    fn take_slot(&mut self, slot: u16) -> Option<Item> {
        if !self.valid_slot(slot) {
            return None;
        }
        let item = self.get(slot).copied()?;
        self.delete_slot(slot);
        Some(item)
    }

    /// Remove `count` of `obj_id`, add to self as `dest_obj_id`.
    /// Returns overflow (count that couldn't be added).
    /// Take item from `slot`, re-add it to self.
    /// Returns `(overflow, obj_id)` or `None` if slot was empty/invalid.
    pub fn move_from_slot(&mut self, slot: u16, stackable: bool) -> u32 {
        match self.take_slot(slot) {
            Some(item) => self.add(item.obj, item.num, stackable),
            None => 0,
        }
    }

    /// Take item from `slot` in self and add to `dest`.
    /// Returns `(overflow, obj_id)` or `None` if slot was empty/invalid.
    pub fn move_from_slot_to(&mut self, dest: &mut Inventory, slot: u16, stackable: bool) -> u32 {
        match self.take_slot(slot) {
            Some(item) => dest.add(item.obj, item.num, stackable),
            None => 0,
        }
    }

    /// Swap items between `self[a]` and `self[b]` across two inventories
    /// by reading both, then writing each to the other's slot.
    pub fn move_to_slot_to(&mut self, dest: &mut Inventory, from_slot: u16, to_slot: u16) {
        if !self.valid_slot(from_slot) || !dest.valid_slot(to_slot) {
            return;
        }
        let from_item = self.get(from_slot).copied();
        let to_item = dest.get(to_slot).copied();
        match from_item {
            Some(item) => dest.set(to_slot, item.obj, item.num),
            None => dest.delete_slot(to_slot),
        }
        match to_item {
            Some(item) => self.set(from_slot, item.obj, item.num),
            None => self.delete_slot(from_slot),
        }
    }

    /// Collects all slots into a vector of `(item_id, count)` pairs for network transmission.
    ///
    /// Empty slots are represented as `None`. The count is cast to `i32` for
    /// compatibility with the client protocol format.
    ///
    /// # Returns
    /// A `Vec` with one entry per slot. Each entry is `Some((obj, num as i32))`
    /// for occupied slots or `None` for empty slots.
    ///
    /// # Call Stack
    /// **Called by:** `ActivePlayer` inventory transmit logic (`active_player.rs`).
    /// **Calls:** nothing.
    pub fn collect_slots(&self) -> Vec<Option<(u16, i32)>> {
        self.slots
            .iter()
            .map(|s| s.map(|item| item.as_pair()))
            .collect()
    }

    /// Collects specific slots into a vector of `(item_id, count)` pairs for partial
    /// network transmission.
    ///
    /// Like [`collect_slots`](Inventory::collect_slots) but only reads the slots at the
    /// given indices, producing a smaller payload for partial inventory updates.
    /// Out-of-bounds indices yield `None`.
    ///
    /// # Arguments
    /// * `slots` - The slot indices to read.
    ///
    /// # Returns
    /// A `Vec` with one entry per requested index. Each entry is `Some((obj, num as i32))`
    /// for occupied slots or `None` for empty/out-of-bounds slots.
    ///
    /// # Call Stack
    /// **Called by:** `InvButtonD` handler (`inv_buttond.rs`) for partial transmit after drag.
    /// **Calls:** `Inventory::get`.
    pub fn collect_slots_at(&self, slots: &[u16]) -> Vec<Option<(u16, i32)>> {
        slots
            .iter()
            .map(|&slot| self.get(slot).map(|item| item.as_pair()))
            .collect()
    }
}

/// An item in a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub obj: u16,
    pub num: u32,
}

impl Item {
    /// Returns this item as the `(obj_id, count)` pair used for network
    /// transmission, with the count cast to `i32` for the client protocol.
    #[inline]
    pub fn as_pair(&self) -> (u16, i32) {
        (self.obj, self.num as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Construction ---

    #[test]
    fn new_creates_empty_inventory() {
        let inv = Inventory::new(28);
        assert_eq!(inv.capacity, 28);
        assert_eq!(inv.slots.len(), 28);
        assert_eq!(inv.freespace(), 28);
        assert!(!inv.dirty);
        assert_eq!(inv.stack_mode, StackMode::Normal);
    }

    #[test]
    fn with_stack_mode_always() {
        let inv = Inventory::with_stack_mode(10, StackMode::Always);
        assert_eq!(inv.stack_mode, StackMode::Always);
        assert_eq!(inv.capacity, 10);
    }

    #[test]
    fn with_stack_mode_never() {
        let inv = Inventory::with_stack_mode(5, StackMode::Never);
        assert_eq!(inv.stack_mode, StackMode::Never);
    }

    // --- Add: Normal stacking ---

    #[test]
    fn add_stackable_item_stacks() {
        let mut inv = Inventory::new(28);
        assert_eq!(inv.add(1, 5, true), 0);
        assert_eq!(inv.add(1, 10, true), 0);
        assert_eq!(inv.total(1), 15);
        assert_eq!(inv.freespace(), 27);
    }

    #[test]
    fn add_non_stackable_uses_individual_slots() {
        let mut inv = Inventory::new(5);
        assert_eq!(inv.add(1, 3, false), 0);
        assert_eq!(inv.freespace(), 2);
        assert_eq!(inv.total(1), 3);
    }

    #[test]
    fn add_non_stackable_overflow() {
        let mut inv = Inventory::new(3);
        let overflow = inv.add(1, 5, false);
        assert_eq!(overflow, 2);
        assert_eq!(inv.total(1), 3);
        assert!(inv.is_full());
    }

    #[test]
    fn add_stackable_overflow_at_stack_limit() {
        let mut inv = Inventory::new(28);
        inv.add(1, STACK_LIMIT, true);
        let overflow = inv.add(1, 100, true);
        assert_eq!(overflow, 100);
        assert_eq!(inv.total(1), STACK_LIMIT);
    }

    #[test]
    fn add_stackable_partial_overflow_at_limit() {
        let mut inv = Inventory::new(28);
        inv.add(1, STACK_LIMIT - 50, true);
        let overflow = inv.add(1, 100, true);
        assert_eq!(overflow, 50);
        assert_eq!(inv.total(1), STACK_LIMIT);
    }

    #[test]
    fn add_stackable_new_slot_capped_at_limit() {
        let mut inv = Inventory::new(28);
        let overflow = inv.add(1, STACK_LIMIT + 100, true);
        assert_eq!(overflow, 100);
        assert_eq!(inv.total(1), STACK_LIMIT);
    }

    #[test]
    fn add_stackable_full_inventory_returns_count() {
        let mut inv = Inventory::new(1);
        inv.add(1, 5, true);
        let overflow = inv.add(2, 10, true);
        assert_eq!(overflow, 10);
    }

    #[test]
    fn add_zero_count() {
        let mut inv = Inventory::new(28);
        assert_eq!(inv.add(1, 0, true), 0);
        assert!(!inv.dirty);
    }

    #[test]
    fn add_sets_dirty() {
        let mut inv = Inventory::new(28);
        inv.add(1, 1, true);
        assert!(inv.dirty);
    }

    // --- Add: Always stack mode ---

    #[test]
    fn always_stack_mode_stacks_non_stackable() {
        let mut inv = Inventory::with_stack_mode(28, StackMode::Always);
        inv.add(1, 5, false);
        inv.add(1, 10, false);
        assert_eq!(inv.total(1), 15);
        assert_eq!(inv.freespace(), 27);
    }

    // --- Add: Never stack mode ---

    #[test]
    fn never_stack_mode_doesnt_stack_stackable() {
        let mut inv = Inventory::with_stack_mode(5, StackMode::Never);
        inv.add(1, 3, true);
        assert_eq!(inv.freespace(), 2);
        assert_eq!(inv.total(1), 3);
        // each item in separate slot with num=1
        for i in 0..3 {
            let item = inv.get(i).unwrap();
            assert_eq!(item.num, 1);
        }
    }

    // --- Delete ---

    #[test]
    fn delete_removes_items() {
        let mut inv = Inventory::new(28);
        inv.add(1, 100, true);
        let removed = inv.delete(1, 50);
        assert_eq!(removed, 50);
        assert_eq!(inv.total(1), 50);
    }

    #[test]
    fn delete_more_than_available() {
        let mut inv = Inventory::new(28);
        inv.add(1, 30, true);
        let removed = inv.delete(1, 100);
        assert_eq!(removed, 30);
        assert_eq!(inv.total(1), 0);
        assert_eq!(inv.freespace(), 28);
    }

    #[test]
    fn delete_non_existent_item() {
        let mut inv = Inventory::new(28);
        inv.add(1, 10, true);
        let removed = inv.delete(2, 5);
        assert_eq!(removed, 0);
    }

    #[test]
    fn delete_across_multiple_slots() {
        let mut inv = Inventory::with_stack_mode(28, StackMode::Never);
        inv.add(1, 5, false);
        let removed = inv.delete(1, 3);
        assert_eq!(removed, 3);
        assert_eq!(inv.total(1), 2);
    }

    #[test]
    fn delete_partial_from_stacked() {
        let mut inv = Inventory::new(28);
        inv.add(1, 100, true);
        inv.delete(1, 30);
        let item = inv.get(0).unwrap();
        assert_eq!(item.num, 70);
    }

    #[test]
    fn delete_keeps_stock_obj_slot_at_zero() {
        let mut inv = Inventory::new(28);
        inv.stockobj = Box::from([10u16, 20]);
        inv.set(0, 10, 5); // stock object
        inv.set(1, 99, 3); // not a stock object

        // Buying out a stock object keeps its slot occupied at count 0.
        inv.delete(10, 5);
        assert_eq!(inv.get(0), Some(&Item { obj: 10, num: 0 }));

        // A non-stock object is cleared from its slot as usual.
        inv.delete(99, 3);
        assert_eq!(inv.get(1), None);
    }

    #[test]
    fn delete_clears_slot_without_stock_list() {
        // No stockobj configured -> a fully removed item clears its slot.
        let mut inv = Inventory::new(28);
        inv.set(0, 10, 5);
        inv.delete(10, 5);
        assert_eq!(inv.get(0), None);
    }

    // --- Remove (slot-based) ---

    #[test]
    fn remove_from_slot() {
        let mut inv = Inventory::new(28);
        inv.add(1, 100, true);
        assert!(inv.remove(0, 40));
        assert_eq!(inv.get(0).unwrap().num, 60);
    }

    #[test]
    fn remove_entire_slot() {
        let mut inv = Inventory::new(28);
        inv.add(1, 50, true);
        assert!(inv.remove(0, 50));
        assert!(inv.get(0).is_none());
    }

    #[test]
    fn remove_more_than_slot_clears_slot() {
        let mut inv = Inventory::new(28);
        inv.add(1, 30, true);
        assert!(inv.remove(0, 100));
        assert!(inv.get(0).is_none());
    }

    #[test]
    fn remove_empty_slot_returns_false() {
        let mut inv = Inventory::new(28);
        assert!(!inv.remove(0, 1));
    }

    #[test]
    fn remove_invalid_slot_returns_false() {
        let mut inv = Inventory::new(5);
        assert!(!inv.remove(10, 1));
    }

    // --- Clear ---

    #[test]
    fn clear_empties_all_slots() {
        let mut inv = Inventory::new(28);
        inv.add(1, 10, true);
        inv.add(2, 20, true);
        inv.clear();
        assert_eq!(inv.freespace(), 28);
        assert!(inv.dirty);
    }

    // --- Set ---

    #[test]
    fn set_places_item_at_slot() {
        let mut inv = Inventory::new(28);
        inv.set(5, 42, 100);
        assert_eq!(inv.get(5).unwrap().obj, 42);
        assert_eq!(inv.get(5).unwrap().num, 100);
    }

    #[test]
    fn set_overwrites_existing() {
        let mut inv = Inventory::new(28);
        inv.set(0, 1, 50);
        inv.set(0, 2, 100);
        assert_eq!(inv.get(0).unwrap().obj, 2);
        assert_eq!(inv.get(0).unwrap().num, 100);
    }

    #[test]
    fn set_invalid_slot_no_panic() {
        let mut inv = Inventory::new(5);
        inv.set(10, 1, 50); // should not panic
    }

    // --- Move to slot (within same inventory) ---

    #[test]
    fn move_to_slot_swaps() {
        let mut inv = Inventory::new(28);
        inv.set(0, 1, 10);
        inv.set(1, 2, 20);
        inv.move_to_slot(0, 1);
        assert_eq!(inv.get(0).unwrap().obj, 2);
        assert_eq!(inv.get(1).unwrap().obj, 1);
    }

    #[test]
    fn move_to_slot_with_empty() {
        let mut inv = Inventory::new(28);
        inv.set(0, 1, 10);
        inv.move_to_slot(0, 5);
        assert!(inv.get(0).is_none());
        assert_eq!(inv.get(5).unwrap().obj, 1);
    }

    #[test]
    fn move_to_slot_invalid_noop() {
        let mut inv = Inventory::new(5);
        inv.set(0, 1, 10);
        inv.move_to_slot(0, 10); // invalid b
        assert_eq!(inv.get(0).unwrap().obj, 1);
    }

    // --- Delete slot ---

    #[test]
    fn delete_slot_clears() {
        let mut inv = Inventory::new(28);
        inv.set(3, 1, 50);
        inv.delete_slot(3);
        assert!(inv.get(3).is_none());
    }

    // --- Query methods ---

    #[test]
    fn freespace_and_is_full() {
        let mut inv = Inventory::new(3);
        assert_eq!(inv.freespace(), 3);
        assert!(!inv.is_full());
        inv.add(1, 3, false);
        assert_eq!(inv.freespace(), 0);
        assert!(inv.is_full());
    }

    #[test]
    fn total_across_modes() {
        let mut inv = Inventory::with_stack_mode(28, StackMode::Never);
        inv.add(1, 5, false);
        assert_eq!(inv.total(1), 5);
        assert_eq!(inv.total(2), 0);
    }

    #[test]
    fn has_at_checks_specific_slot() {
        let mut inv = Inventory::new(28);
        inv.set(0, 1, 10);
        inv.set(1, 2, 20);
        assert!(inv.has_at(0, 1));
        assert!(!inv.has_at(0, 2));
        assert!(inv.has_at(1, 2));
        assert!(!inv.has_at(5, 1));
    }

    #[test]
    fn valid_slot_checks() {
        let inv = Inventory::new(10);
        assert!(inv.valid_slot(0));
        assert!(inv.valid_slot(9));
        assert!(!inv.valid_slot(10));
        assert!(!inv.valid_slot(100));
    }

    // --- Move item (within same inventory, converting IDs) ---

    // --- Move from slot ---

    #[test]
    fn move_from_slot_restacks() {
        let mut inv = Inventory::new(28);
        inv.set(5, 1, 10);
        inv.set(0, 1, 5);
        let overflow = inv.move_from_slot(5, true);
        assert_eq!(overflow, 0);
        assert!(inv.get(5).is_none());
        assert_eq!(inv.total(1), 15);
    }

    #[test]
    fn move_from_slot_empty() {
        let mut inv = Inventory::new(28);
        let overflow = inv.move_from_slot(0, true);
        assert_eq!(overflow, 0);
    }

    #[test]
    fn move_from_slot_invalid() {
        let mut inv = Inventory::new(5);
        let overflow = inv.move_from_slot(10, true);
        assert_eq!(overflow, 0);
    }

    // --- Move from slot to ---

    #[test]
    fn move_from_slot_to_dest() {
        let mut src = Inventory::new(28);
        let mut dest = Inventory::new(28);
        src.set(3, 1, 25);
        let overflow = src.move_from_slot_to(&mut dest, 3, true);
        assert_eq!(overflow, 0);
        assert!(src.get(3).is_none());
        assert_eq!(dest.total(1), 25);
    }

    // --- Move to slot to (cross-inventory swap) ---

    #[test]
    fn move_to_slot_to_swaps() {
        let mut a = Inventory::new(28);
        let mut b = Inventory::new(28);
        a.set(0, 1, 10);
        b.set(0, 2, 20);
        a.move_to_slot_to(&mut b, 0, 0);
        assert_eq!(a.get(0).unwrap().obj, 2);
        assert_eq!(b.get(0).unwrap().obj, 1);
    }

    #[test]
    fn move_to_slot_to_with_empty_src() {
        let mut a = Inventory::new(28);
        let mut b = Inventory::new(28);
        b.set(0, 2, 20);
        a.move_to_slot_to(&mut b, 0, 0);
        assert!(a.get(0).is_some());
        assert!(b.get(0).is_none());
    }

    #[test]
    fn move_to_slot_to_with_empty_dest() {
        let mut a = Inventory::new(28);
        let mut b = Inventory::new(28);
        a.set(0, 1, 10);
        a.move_to_slot_to(&mut b, 0, 0);
        assert!(a.get(0).is_none());
        assert_eq!(b.get(0).unwrap().obj, 1);
    }

    // --- Collect slots ---

    #[test]
    fn collect_slots_represents_inventory() {
        let mut inv = Inventory::new(3);
        inv.set(0, 1, 10);
        inv.set(2, 3, 30);
        let collected = inv.collect_slots();
        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0], Some((1, 10)));
        assert_eq!(collected[1], None);
        assert_eq!(collected[2], Some((3, 30)));
    }

    // --- Edge cases ---

    #[test]
    fn multiple_different_items() {
        let mut inv = Inventory::new(28);
        inv.add(1, 10, true);
        inv.add(2, 20, true);
        inv.add(3, 30, true);
        assert_eq!(inv.total(1), 10);
        assert_eq!(inv.total(2), 20);
        assert_eq!(inv.total(3), 30);
        assert_eq!(inv.freespace(), 25);
    }

    #[test]
    fn add_same_non_stackable_item_fills_slots() {
        let mut inv = Inventory::new(5);
        inv.add(1, 5, false);
        assert!(inv.is_full());
        for i in 0..5 {
            let item = inv.get(i).unwrap();
            assert_eq!(item.obj, 1);
            assert_eq!(item.num, 1);
        }
    }

    #[test]
    fn dirty_flag_reset_manually() {
        let mut inv = Inventory::new(28);
        inv.add(1, 10, true);
        assert!(inv.dirty);
        inv.dirty = false;
        inv.delete(1, 5);
        assert!(inv.dirty);
    }

    // --- STACK_LIMIT boundary tests (from engine inv_itemspace logic) ---

    #[test]
    fn add_stackable_exactly_at_stack_limit() {
        let mut inv = Inventory::new(28);
        assert_eq!(inv.add(1, STACK_LIMIT, true), 0);
        assert_eq!(inv.total(1), STACK_LIMIT);
        // adding 1 more should overflow by 1
        assert_eq!(inv.add(1, 1, true), 1);
        assert_eq!(inv.total(1), STACK_LIMIT);
    }

    #[test]
    fn total_saturating_add_multiple_slots_never_mode() {
        // In Never mode, each item gets its own slot with num=1
        let mut inv = Inventory::with_stack_mode(100, StackMode::Never);
        inv.add(1, 50, true);
        assert_eq!(inv.total(1), 50);
    }

    #[test]
    fn total_with_always_mode_single_stack() {
        let mut inv = Inventory::with_stack_mode(10, StackMode::Always);
        inv.add(1, 1000, false);
        assert_eq!(inv.total(1), 1000);
        assert_eq!(inv.freespace(), 9);
    }

    #[test]
    fn freespace_minus_size_calculation() {
        // Engine does: (count - (freespace - (inv.size - size))).max(0)
        // for non-stackable items. Test freespace accurately.
        let mut inv = Inventory::new(10);
        inv.add(1, 3, false);
        assert_eq!(inv.freespace(), 7);
        inv.add(2, 4, false);
        assert_eq!(inv.freespace(), 3);
        assert!(!inv.is_full());
    }

    // --- Cert/uncert move patterns (from INV_MOVEITEM_CERT/UNCERT) ---

    #[test]
    fn cert_conversion_stackable_to_stackable() {
        let mut inv = Inventory::new(28);
        inv.add(100, 50, true);
        let removed = inv.delete(100, 20);
        let overflow = inv.add(101, removed, true);
        assert_eq!(overflow, 0);
        assert_eq!(inv.total(100), 30);
        assert_eq!(inv.total(101), 20);
    }

    #[test]
    fn uncert_conversion_stackable_to_nonstackable() {
        let mut inv = Inventory::new(28);
        inv.add(101, 10, true);
        let removed = inv.delete(101, 5);
        let overflow = inv.add(100, removed, false);
        assert_eq!(overflow, 0);
        assert_eq!(inv.total(101), 5);
        assert_eq!(inv.total(100), 5);
    }

    #[test]
    fn cert_cross_inventory() {
        let mut src = Inventory::new(28);
        let mut dest = Inventory::with_stack_mode(28, StackMode::Always);
        src.add(100, 50, true);
        let removed = src.delete(100, 30);
        let overflow = dest.add(101, removed, true);
        assert_eq!(overflow, 0);
        assert_eq!(src.total(100), 20);
        assert_eq!(dest.total(101), 30);
    }

    #[test]
    fn uncert_cross_inventory_with_overflow() {
        let mut src = Inventory::with_stack_mode(28, StackMode::Always);
        let mut dest = Inventory::new(3);
        src.add(101, 10, true);
        let removed = src.delete(101, 10);
        let overflow = dest.add(100, removed, false);
        assert_eq!(overflow, 7);
        assert_eq!(src.total(101), 0);
        assert_eq!(dest.total(100), 3);
    }

    // --- move_from_slot patterns (from INV_MOVEFROMSLOT) ---

    #[test]
    fn move_from_slot_same_inv_restacks_stackable() {
        let mut inv = Inventory::new(28);
        // Put stackable item manually in slot 5
        inv.set(5, 1, 10);
        // Also have some in slot 0 already
        inv.set(0, 1, 5);
        // move_from_slot should restack
        let overflow = inv.move_from_slot(5, true);
        assert_eq!(overflow, 0);
        assert!(inv.get(5).is_none());
        assert_eq!(inv.get(0).unwrap().num, 15);
    }

    #[test]
    fn move_from_slot_same_inv_nonstackable_takes_new_slot() {
        let mut inv = Inventory::new(28);
        inv.set(10, 1, 1);
        let overflow = inv.move_from_slot(10, false);
        assert_eq!(overflow, 0);
        // slot 10 cleared, item re-added to first free slot (0)
        assert!(inv.get(10).is_none());
        assert_eq!(inv.get(0).unwrap().obj, 1);
    }

    #[test]
    fn move_from_slot_to_cross_inv_stackable() {
        let mut src = Inventory::new(28);
        let mut dest = Inventory::new(28);
        src.set(3, 1, 25);
        dest.add(1, 10, true);
        let overflow = src.move_from_slot_to(&mut dest, 3, true);
        assert_eq!(overflow, 0);
        assert!(src.get(3).is_none());
        assert_eq!(dest.total(1), 35);
    }

    #[test]
    fn move_from_slot_to_cross_inv_full_dest() {
        let mut src = Inventory::new(28);
        let mut dest = Inventory::new(1);
        src.set(0, 1, 5);
        dest.add(2, 1, true); // dest full with different item
        let overflow = src.move_from_slot_to(&mut dest, 0, true);
        assert_eq!(overflow, 5);
        assert!(src.get(0).is_none());
    }

    // --- move_to_slot patterns (from INV_MOVETOSLOT) ---

    #[test]
    fn move_to_slot_same_inv_both_occupied() {
        let mut inv = Inventory::new(28);
        inv.set(0, 1, 10);
        inv.set(5, 2, 20);
        inv.move_to_slot(0, 5);
        assert_eq!(inv.get(0).unwrap().obj, 2);
        assert_eq!(inv.get(0).unwrap().num, 20);
        assert_eq!(inv.get(5).unwrap().obj, 1);
        assert_eq!(inv.get(5).unwrap().num, 10);
    }

    #[test]
    fn move_to_slot_to_cross_inv_both_empty() {
        let mut a = Inventory::new(28);
        let mut b = Inventory::new(28);
        a.move_to_slot_to(&mut b, 0, 0);
        assert!(a.get(0).is_none());
        assert!(b.get(0).is_none());
    }

    #[test]
    fn move_to_slot_to_cross_inv_different_slots() {
        let mut a = Inventory::new(28);
        let mut b = Inventory::new(28);
        a.set(3, 10, 100);
        b.set(7, 20, 200);
        a.move_to_slot_to(&mut b, 3, 7);
        assert_eq!(a.get(3).unwrap().obj, 20);
        assert_eq!(a.get(3).unwrap().num, 200);
        assert_eq!(b.get(7).unwrap().obj, 10);
        assert_eq!(b.get(7).unwrap().num, 100);
    }

    // --- Equipment/trade patterns (Never stack mode) ---

    #[test]
    fn never_mode_individual_slot_operations() {
        let mut inv = Inventory::with_stack_mode(14, StackMode::Never);
        // Add equipment-style items
        inv.add(100, 1, true); // stackable flag ignored in Never mode
        inv.add(200, 1, true);
        inv.add(300, 1, true);
        assert_eq!(inv.freespace(), 11);
        // Each item in its own slot
        assert_eq!(inv.get(0).unwrap().obj, 100);
        assert_eq!(inv.get(1).unwrap().obj, 200);
        assert_eq!(inv.get(2).unwrap().obj, 300);
    }

    #[test]
    fn never_mode_delete_specific_items() {
        let mut inv = Inventory::with_stack_mode(14, StackMode::Never);
        inv.add(100, 3, false);
        inv.add(200, 2, false);
        let removed = inv.delete(100, 2);
        assert_eq!(removed, 2);
        assert_eq!(inv.total(100), 1);
        assert_eq!(inv.total(200), 2);
    }

    // --- Bank patterns (Always stack mode) ---

    #[test]
    fn always_mode_bank_operations() {
        let mut bank = Inventory::with_stack_mode(800, StackMode::Always);
        bank.add(1, 1000, false);
        bank.add(2, 500, false);
        bank.add(1, 500, false);
        assert_eq!(bank.total(1), 1500);
        assert_eq!(bank.total(2), 500);
        assert_eq!(bank.freespace(), 798);
    }

    #[test]
    fn always_mode_delete_partial_stack() {
        let mut bank = Inventory::with_stack_mode(800, StackMode::Always);
        bank.add(1, 1000, false);
        let removed = bank.delete(1, 600);
        assert_eq!(removed, 600);
        assert_eq!(bank.total(1), 400);
    }

    // --- Combined cross-inventory scenarios ---

    #[test]
    fn bank_to_inventory_withdraw() {
        let mut bank = Inventory::with_stack_mode(800, StackMode::Always);
        let mut inv = Inventory::new(28);
        bank.add(1, 100, false);
        let removed = bank.delete(1, 50);
        let overflow = inv.add(1, removed, false);
        assert_eq!(overflow, 22);
        assert_eq!(bank.total(1), 50);
        assert_eq!(inv.total(1), 28);
    }

    #[test]
    fn inventory_to_bank_deposit() {
        let mut inv = Inventory::new(28);
        let mut bank = Inventory::with_stack_mode(800, StackMode::Always);
        inv.add(1, 10, false);
        let removed = inv.delete(1, 10);
        let overflow = bank.add(1, removed, true);
        assert_eq!(overflow, 0);
        assert_eq!(inv.total(1), 0);
        assert_eq!(bank.total(1), 10);
    }

    // --- collect_slots for transmit ---

    #[test]
    fn collect_slots_with_mixed_items() {
        let mut inv = Inventory::new(5);
        inv.set(0, 10, 100);
        inv.set(2, 20, 200);
        inv.set(4, 30, 300);
        let slots = inv.collect_slots();
        assert_eq!(
            slots,
            vec![
                Some((10, 100)),
                None,
                Some((20, 200)),
                None,
                Some((30, 300)),
            ]
        );
    }

    #[test]
    fn collect_slots_empty_inventory() {
        let inv = Inventory::new(3);
        let slots = inv.collect_slots();
        assert_eq!(slots, vec![None, None, None]);
    }

    // --- delete edge cases ---

    #[test]
    fn delete_zero_count() {
        let mut inv = Inventory::new(28);
        inv.add(1, 50, true);
        let removed = inv.delete(1, 0);
        assert_eq!(removed, 0);
        assert_eq!(inv.total(1), 50);
    }

    #[test]
    fn delete_across_mixed_stacked_slots() {
        // Manually place same item in multiple slots (simulates edge case)
        let mut inv = Inventory::new(28);
        inv.set(0, 1, 30);
        inv.set(5, 1, 20);
        inv.set(10, 1, 10);
        let removed = inv.delete(1, 45);
        assert_eq!(removed, 45);
        assert_eq!(inv.total(1), 15);
    }

    // --- Dirty flag patterns ---

    #[test]
    fn dirty_flag_on_all_mutations() {
        let mut inv = Inventory::new(28);

        inv.add(1, 1, true);
        assert!(inv.dirty);
        inv.dirty = false;

        inv.delete(1, 1);
        assert!(inv.dirty);
        inv.dirty = false;

        inv.set(0, 1, 10);
        assert!(inv.dirty);
        inv.dirty = false;

        inv.delete_slot(0);
        assert!(inv.dirty);
        inv.dirty = false;

        inv.set(0, 1, 10);
        inv.set(1, 2, 20);
        inv.dirty = false;
        inv.move_to_slot(0, 1);
        assert!(inv.dirty);
        inv.dirty = false;

        inv.clear();
        assert!(inv.dirty);
    }

    #[test]
    fn dirty_slots_tracks_changed_slot() {
        let mut inv = Inventory::new(28);
        inv.clear_dirty();
        inv.set(3, 10, 100);
        assert_eq!(inv.collect_dirty(), vec![(3, Some((10, 100)))]);
    }

    #[test]
    fn dirty_slots_dedup_and_sorted() {
        let mut inv = Inventory::new(28);
        inv.set(5, 1, 1);
        inv.set(2, 2, 2);
        inv.set(5, 3, 3); // same slot twice -> deduped, reflects latest value
        let dirty = inv.collect_dirty();
        assert_eq!(dirty, vec![(2, Some((2, 2))), (5, Some((3, 3)))]);
    }

    #[test]
    fn collect_dirty_reflects_current_value() {
        let mut inv = Inventory::new(28);
        inv.set(4, 7, 70);
        inv.remove(4, 70); // slot now empty
        // Slot 4 was touched twice but should report its current (empty) contents.
        assert_eq!(inv.collect_dirty(), vec![(4, None)]);
    }

    #[test]
    fn clear_dirty_resets_change_set() {
        let mut inv = Inventory::new(28);
        inv.set(1, 1, 1);
        assert!(inv.dirty);
        assert!(!inv.dirty_slots.is_empty());
        inv.clear_dirty();
        assert!(!inv.dirty);
        assert!(inv.dirty_slots.is_empty());
        assert!(inv.collect_dirty().is_empty());
    }

    #[test]
    fn clear_marks_every_slot_dirty() {
        let mut inv = Inventory::new(5);
        inv.set(0, 1, 1);
        inv.clear_dirty();
        inv.clear();
        let dirty = inv.collect_dirty();
        assert_eq!(dirty.len(), 5);
        assert_eq!(
            dirty,
            vec![(0, None), (1, None), (2, None), (3, None), (4, None)]
        );
    }

    #[test]
    fn move_to_slot_marks_both_slots() {
        let mut inv = Inventory::new(28);
        inv.set(0, 1, 10);
        inv.set(1, 2, 20);
        inv.clear_dirty();
        inv.move_to_slot(0, 1);
        assert_eq!(
            inv.collect_dirty(),
            vec![(0, Some((2, 20))), (1, Some((1, 10)))]
        );
    }

    // --- remove edge: removing count exactly equal to item.num ---

    #[test]
    fn remove_exact_count_clears_slot() {
        let mut inv = Inventory::new(28);
        inv.set(0, 1, 42);
        assert!(inv.remove(0, 42));
        assert!(inv.get(0).is_none());
    }

    #[test]
    fn remove_one_from_one_clears() {
        let mut inv = Inventory::new(28);
        inv.set(0, 1, 1);
        assert!(inv.remove(0, 1));
        assert!(inv.get(0).is_none());
        assert_eq!(inv.freespace(), 28);
    }

    // --- collect_slots_at ---

    #[test]
    fn collect_slots_at_specific_indices() {
        let mut inv = Inventory::new(10);
        inv.set(0, 10, 100);
        inv.set(3, 20, 200);
        inv.set(7, 30, 300);
        let result = inv.collect_slots_at(&[0, 3, 7]);
        assert_eq!(
            result,
            vec![Some((10, 100)), Some((20, 200)), Some((30, 300))]
        );
    }

    #[test]
    fn collect_slots_at_empty_slots() {
        let inv = Inventory::new(10);
        let result = inv.collect_slots_at(&[0, 5, 9]);
        assert_eq!(result, vec![None, None, None]);
    }

    #[test]
    fn collect_slots_at_mixed() {
        let mut inv = Inventory::new(10);
        inv.set(2, 5, 50);
        let result = inv.collect_slots_at(&[0, 2, 4]);
        assert_eq!(result, vec![None, Some((5, 50)), None]);
    }

    #[test]
    fn collect_slots_at_out_of_bounds() {
        let inv = Inventory::new(5);
        let result = inv.collect_slots_at(&[0, 10, 100]);
        assert_eq!(result, vec![None, None, None]);
    }

    #[test]
    fn collect_slots_at_empty_input() {
        let mut inv = Inventory::new(10);
        inv.set(0, 1, 10);
        let result = inv.collect_slots_at(&[]);
        assert!(result.is_empty());
    }
}
