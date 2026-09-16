// The cursor-based iteration intentionally reproduces a quirk of the original:
// head()/next() cache the successor at call time. If a node is appended while
// iterating *past* the former tail, the cached cursor already points to the
// sentinel, so the new node is skipped. But if there are still nodes ahead,
// the cursor chain naturally reaches the new node - giving it an early
// execution ("speedup bug") that is authentic server behavior.

use crate::Entry;

const SENTINEL: usize = 0;

/// Arena-based singly-linked list with sentinel node and cursor-based iteration.
///
/// All nodes are stored in a contiguous `Vec<Entry<T>>`. Index `0` is the
/// sentinel whose `prev`/`next` pointers close the list into a circular doubly-linked
/// ring. Callers never see the sentinel; public handles always refer to indices >= 1.
///
/// Removed slots are pushed onto an internal free list and reused by subsequent
/// insertions, keeping the arena compact without reallocation.
///
/// A single `cursor` field supports forward (`head`/`next`) and reverse
/// (`tail`/`prev`) iteration. Because the cursor caches the successor at call
/// time, it is safe to `unlink` the current node mid-iteration.
pub struct LinkList<T> {
    entries: Vec<Entry<T>>,
    free: Vec<usize>,
    cursor: usize,
}

#[allow(clippy::should_implement_trait)]
impl<T> LinkList<T> {
    /// Creates a new, empty `LinkList`.
    ///
    /// No heap allocation is performed. The sentinel entry at index 0 is
    /// materialized by the first [`alloc`](Self::alloc); until then the arena is
    /// empty and every read path treats it as an empty circular ring.
    ///
    /// # Returns
    ///
    /// A `LinkList<T>` with no user-visible elements.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::new` (rs-engine/src/engine.rs),
    /// `QueueSet::new` (rs-queue/src/lib.rs)
    pub fn new() -> Self {
        LinkList {
            entries: Vec::new(),
            free: Vec::new(),
            cursor: SENTINEL,
        }
    }

    /// Returns the sentinel's `next` pointer, or `SENTINEL` if the arena has not
    /// been allocated yet.
    ///
    /// An unallocated arena is indistinguishable from an empty ring, so this
    /// lets the read paths stay branch-light without forcing an allocation.
    ///
    /// # Returns
    ///
    /// The handle of the head node, or `SENTINEL` when the list is empty.
    #[inline]
    fn sentinel_next(&self) -> usize {
        match self.entries.first() {
            Some(sentinel) => sentinel.next,
            None => SENTINEL,
        }
    }

    /// Returns the sentinel's `prev` pointer, or `SENTINEL` if the arena has not
    /// been allocated yet.
    ///
    /// # Returns
    ///
    /// The handle of the tail node, or `SENTINEL` when the list is empty.
    #[inline]
    fn sentinel_prev(&self) -> usize {
        match self.entries.first() {
            Some(sentinel) => sentinel.prev,
            None => SENTINEL,
        }
    }

    /// Allocates a slot for `value` and returns its arena index.
    ///
    /// If the free list is non-empty, the most recently freed slot is reused.
    /// Otherwise a new entry is pushed onto the backing `Vec`, growing the arena.
    /// The returned entry has `prev` and `next` set to `SENTINEL`; the caller is
    /// responsible for linking it into the ring.
    ///
    /// # Arguments
    ///
    /// * `value` - The element to store in the newly allocated slot.
    ///
    /// # Returns
    ///
    /// The arena index of the allocated slot (always >= 1).
    ///
    /// # Side Effects
    ///
    /// * Pops from `self.free` when a recycled slot is available.
    /// * Pushes onto `self.entries` when no free slot exists.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `add_tail`, `add_head`
    fn alloc(&mut self, value: T) -> usize {
        if self.entries.is_empty() {
            self.entries.push(Entry {
                value: None,
                prev: SENTINEL,
                next: SENTINEL,
            });
        }
        if let Some(idx) = self.free.pop() {
            self.entries[idx] = Entry {
                value: Some(value),
                prev: SENTINEL,
                next: SENTINEL,
            };
            idx
        } else {
            let idx = self.entries.len();
            self.entries.push(Entry {
                value: Some(value),
                prev: SENTINEL,
                next: SENTINEL,
            });
            idx
        }
    }

    /// Appends `value` to the tail of the list.
    ///
    /// The new node is inserted between the current tail and the sentinel,
    /// making it the last element visited by forward iteration.
    ///
    /// # Arguments
    ///
    /// * `value` - The element to append.
    ///
    /// # Side Effects
    ///
    /// * Allocates a slot via `alloc`.
    /// * Updates `prev`/`next` pointers of the former tail and the sentinel.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::queue_script`, `Engine::add_obj_delayed`
    /// (rs-engine/src/engine.rs)
    /// **Calls:** `alloc`
    pub fn add_tail(&mut self, value: T) {
        let idx = self.alloc(value);
        let prev = self.entries[SENTINEL].prev;
        self.entries[idx].prev = prev;
        self.entries[idx].next = SENTINEL;
        self.entries[prev].next = idx;
        self.entries[SENTINEL].prev = idx;
    }

    /// Prepends `value` to the head of the list.
    ///
    /// The new node is inserted between the sentinel and the current head,
    /// making it the first element visited by forward iteration.
    ///
    /// # Arguments
    ///
    /// * `value` - The element to prepend.
    ///
    /// # Side Effects
    ///
    /// * Allocates a slot via `alloc`.
    /// * Updates `prev`/`next` pointers of the sentinel and the former head.
    ///
    /// # Call Stack
    ///
    /// **Calls:** `alloc`
    pub fn add_head(&mut self, value: T) {
        let idx = self.alloc(value);
        let next = self.entries[SENTINEL].next;
        self.entries[idx].prev = SENTINEL;
        self.entries[idx].next = next;
        self.entries[SENTINEL].next = idx;
        self.entries[next].prev = idx;
    }

    /// Removes the head node and returns its value.
    ///
    /// If the list is empty, returns `None`. Otherwise unlinks the first
    /// user-visible node (the one immediately after the sentinel) and returns
    /// its value, pushing the slot onto the free list.
    ///
    /// # Returns
    ///
    /// `Some(T)` with the former head's value, or `None` if the list is empty.
    ///
    /// # Side Effects
    ///
    /// * Unlinks the head node and recycles its slot via `unlink`.
    ///
    /// # Call Stack
    ///
    /// **Calls:** `unlink`
    pub fn remove_head(&mut self) -> Option<T> {
        let idx = self.sentinel_next();
        if idx == SENTINEL {
            return None;
        }
        Some(self.unlink(idx))
    }

    /// Begins forward iteration and returns the handle of the head node.
    ///
    /// Sets the internal cursor to the successor of the head node so that
    /// a subsequent call to [`next`](Self::next) advances to the second element.
    /// If the list is empty, the cursor is reset to the sentinel and `None`
    /// is returned.
    ///
    /// Because the cursor caches the head's successor at call time, it is safe
    /// to `unlink` the returned handle before calling `next`.
    ///
    /// # Returns
    ///
    /// `Some(handle)` for the head node, or `None` if the list is empty.
    ///
    /// # Side Effects
    ///
    /// * Sets `self.cursor` to the node after the head (or `SENTINEL` if empty).
    ///
    /// # Call Stack
    ///
    /// **Called by:** iteration loops in phases/npc.rs, phases/player.rs,
    /// phases/world.rs
    pub fn head(&mut self) -> Option<usize> {
        let idx = self.sentinel_next();
        if idx == SENTINEL {
            self.cursor = SENTINEL;
            return None;
        }
        self.cursor = self.entries[idx].next;
        Some(idx)
    }

    /// Begins reverse iteration and returns the handle of the tail node.
    ///
    /// Sets the internal cursor to the predecessor of the tail node so that
    /// a subsequent call to [`prev`](Self::prev) advances toward the head.
    /// If the list is empty, the cursor is reset to the sentinel and `None`
    /// is returned.
    ///
    /// # Returns
    ///
    /// `Some(handle)` for the tail node, or `None` if the list is empty.
    ///
    /// # Side Effects
    ///
    /// * Sets `self.cursor` to the node before the tail (or `SENTINEL` if empty).
    pub fn tail(&mut self) -> Option<usize> {
        let idx = self.sentinel_prev();
        if idx == SENTINEL {
            self.cursor = SENTINEL;
            return None;
        }
        self.cursor = self.entries[idx].prev;
        Some(idx)
    }

    /// Advances the forward cursor and returns the next node's handle.
    ///
    /// Returns the handle stored in `self.cursor` and advances the cursor to
    /// that node's `next` pointer. When the cursor reaches the sentinel, the
    /// iteration is exhausted and `None` is returned.
    ///
    /// # Returns
    ///
    /// `Some(handle)` for the current cursor node, or `None` if iteration
    /// has reached the sentinel.
    ///
    /// # Side Effects
    ///
    /// * Advances `self.cursor` to the successor of the returned node.
    ///
    /// # Call Stack
    ///
    /// **Called by:** iteration loops in phases/npc.rs, phases/player.rs,
    /// phases/world.rs
    pub fn next(&mut self) -> Option<usize> {
        let idx = self.cursor;
        if idx == SENTINEL {
            return None;
        }
        self.cursor = self.entries[idx].next;
        Some(idx)
    }

    /// Advances the reverse cursor and returns the previous node's handle.
    ///
    /// Returns the handle stored in `self.cursor` and moves the cursor to
    /// that node's `prev` pointer. When the cursor reaches the sentinel, the
    /// iteration is exhausted and `None` is returned.
    ///
    /// # Returns
    ///
    /// `Some(handle)` for the current cursor node, or `None` if iteration
    /// has reached the sentinel.
    ///
    /// # Side Effects
    ///
    /// * Moves `self.cursor` to the predecessor of the returned node.
    pub fn prev(&mut self) -> Option<usize> {
        let idx = self.cursor;
        if idx == SENTINEL {
            return None;
        }
        self.cursor = self.entries[idx].prev;
        Some(idx)
    }

    /// Returns an immutable reference to the value stored at `handle`.
    ///
    /// # Arguments
    ///
    /// * `handle` - Arena index obtained from `head`, `tail`, `next`, or `prev`.
    ///
    /// # Returns
    ///
    /// A shared reference to the value at the given slot.
    ///
    /// # Panics
    ///
    /// Panics with `"invalid handle"` if the slot is vacant (i.e., the node has
    /// been unlinked or was never allocated).
    pub fn get(&self, handle: usize) -> &T {
        self.entries[handle].value.as_ref().expect("invalid handle")
    }

    /// Returns a mutable reference to the value stored at `handle`.
    ///
    /// # Arguments
    ///
    /// * `handle` - Arena index obtained from `head`, `tail`, `next`, or `prev`.
    ///
    /// # Returns
    ///
    /// An exclusive reference to the value at the given slot.
    ///
    /// # Panics
    ///
    /// Panics with `"invalid handle"` if the slot is vacant (i.e., the node has
    /// been unlinked or was never allocated).
    pub fn get_mut(&mut self, handle: usize) -> &mut T {
        self.entries[handle].value.as_mut().expect("invalid handle")
    }

    /// Removes the node at `handle` from the list and returns its value.
    ///
    /// The node's neighbors are patched to skip it, the node's `prev`/`next`
    /// are reset to `SENTINEL`, and the slot is pushed onto the free list for
    /// reuse. The value is taken out of the entry's `Option` via `take`.
    ///
    /// It is safe to call `unlink` on the current node during a `head`/`next`
    /// or `tail`/`prev` iteration because the cursor has already cached the
    /// successor or predecessor.
    ///
    /// # Arguments
    ///
    /// * `handle` - Arena index of the node to remove.
    ///
    /// # Returns
    ///
    /// The value that was stored in the node.
    ///
    /// # Side Effects
    ///
    /// * Patches `prev`/`next` pointers of neighboring nodes.
    /// * Resets the unlinked entry's pointers to `SENTINEL`.
    /// * Takes the value out of the entry (sets it to `None`).
    /// * Pushes `handle` onto `self.free`.
    ///
    /// # Panics
    ///
    /// Panics with `"double unlink"` if the slot's value has already been taken,
    /// indicating the handle was used after the node was freed.
    ///
    /// # Call Stack
    ///
    /// **Called by:** npc phase (rs-engine/src/phases/npc.rs), player phase
    /// (rs-engine/src/phases/player.rs), world phase (rs-engine/src/phases/world.rs)
    pub fn unlink(&mut self, handle: usize) -> T {
        let prev = self.entries[handle].prev;
        let next = self.entries[handle].next;
        self.entries[prev].next = next;
        self.entries[next].prev = prev;
        self.entries[handle].prev = SENTINEL;
        self.entries[handle].next = SENTINEL;
        let value = self.entries[handle].value.take().expect("double unlink");
        self.free.push(handle);
        value
    }

    /// Returns `true` if the list contains no user-visible elements.
    ///
    /// The check is performed by testing whether the sentinel's `next` pointer
    /// refers back to itself.
    ///
    /// # Returns
    ///
    /// `true` when the list is empty, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.sentinel_next() == SENTINEL
    }

    /// Returns a forward iterator over the list's live values, head to tail,
    /// without using or disturbing the internal traversal cursor.
    ///
    /// Unlike [`head`](Self::head)/[`next`](Self::next), this walks the node
    /// chain directly, so it is safe to call while a cursor-based iteration is
    /// already in progress (e.g. from a queued script during the queue drain).
    ///
    /// # Returns
    ///
    /// An iterator yielding `&T` for each element in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        let mut idx = self.sentinel_next();
        std::iter::from_fn(move || {
            if idx == SENTINEL {
                return None;
            }
            let entry = &self.entries[idx];
            idx = entry.next;
            entry.value.as_ref()
        })
    }

    /// Resets the list to its empty state, recycling all allocated slots.
    ///
    /// The sentinel's pointers are reset to itself, the cursor is cleared,
    /// and every non-sentinel entry has its value dropped and its index
    /// pushed onto the free list. The backing `Vec` is not shrunk, so
    /// subsequent insertions can reuse the existing capacity without
    /// reallocation.
    ///
    /// # Side Effects
    ///
    /// * Drops all stored values.
    /// * Resets `self.cursor` to `SENTINEL`.
    /// * Clears and repopulates `self.free` with indices `1..entries.len()`.
    /// * Resets the sentinel's `prev`/`next` to `SENTINEL`.
    pub fn clear(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.entries[SENTINEL].prev = SENTINEL;
        self.entries[SENTINEL].next = SENTINEL;
        self.cursor = SENTINEL;
        self.free.clear();
        for idx in 1..self.entries.len() {
            self.entries[idx].value = None;
            self.free.push(idx);
        }
    }
}

/// Provides a [`Default`] implementation that delegates to [`LinkList::new`].
impl<T> Default for LinkList<T> {
    /// Creates an empty `LinkList` by calling [`LinkList::new`].
    ///
    /// # Returns
    ///
    /// A `LinkList<T>` with no user-visible elements.
    ///
    /// # Call Stack
    ///
    /// **Calls:** `LinkList::new`
    fn default() -> Self {
        Self::new()
    }
}

/// Allows indexing a `LinkList` with `list[handle]` syntax.
impl<T> std::ops::Index<usize> for LinkList<T> {
    type Output = T;

    /// Returns an immutable reference to the value at `handle`.
    ///
    /// This delegates to [`LinkList::get`].
    ///
    /// # Arguments
    ///
    /// * `handle` - Arena index of the node to access.
    ///
    /// # Panics
    ///
    /// Panics with `"invalid handle"` if the slot is vacant.
    ///
    /// # Call Stack
    ///
    /// **Calls:** `LinkList::get`
    fn index(&self, handle: usize) -> &T {
        self.get(handle)
    }
}

/// Allows mutable indexing a `LinkList` with `list[handle] = value` syntax.
impl<T> std::ops::IndexMut<usize> for LinkList<T> {
    /// Returns a mutable reference to the value at `handle`.
    ///
    /// This delegates to [`LinkList::get_mut`].
    ///
    /// # Arguments
    ///
    /// * `handle` - Arena index of the node to access.
    ///
    /// # Panics
    ///
    /// Panics with `"invalid handle"` if the slot is vacant.
    ///
    /// # Call Stack
    ///
    /// **Calls:** `LinkList::get_mut`
    fn index_mut(&mut self, handle: usize) -> &mut T {
        self.get_mut(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_tail_and_remove_head() {
        let mut list = LinkList::new();
        list.add_tail(1);
        list.add_tail(2);
        list.add_tail(3);

        assert_eq!(list.remove_head(), Some(1));
        assert_eq!(list.remove_head(), Some(2));
        assert_eq!(list.remove_head(), Some(3));
        assert_eq!(list.remove_head(), None);
    }

    #[test]
    fn add_head_and_remove_head() {
        let mut list = LinkList::new();
        list.add_head(1);
        list.add_head(2);
        list.add_head(3);

        assert_eq!(list.remove_head(), Some(3));
        assert_eq!(list.remove_head(), Some(2));
        assert_eq!(list.remove_head(), Some(1));
        assert_eq!(list.remove_head(), None);
    }

    #[test]
    fn forward_iteration() {
        let mut list = LinkList::new();
        list.add_tail(10);
        list.add_tail(20);
        list.add_tail(30);

        let mut result = Vec::new();
        let mut h = list.head();
        while let Some(idx) = h {
            result.push(list[idx]);
            h = list.next();
        }
        assert_eq!(result, vec![10, 20, 30]);
    }

    #[test]
    fn reverse_iteration() {
        let mut list = LinkList::new();
        list.add_tail(10);
        list.add_tail(20);
        list.add_tail(30);

        let mut result = Vec::new();
        let mut h = list.tail();
        while let Some(idx) = h {
            result.push(list[idx]);
            h = list.prev();
        }
        assert_eq!(result, vec![30, 20, 10]);
    }

    #[test]
    fn unlink_during_iteration() {
        let mut list = LinkList::new();
        list.add_tail(1);
        list.add_tail(2);
        list.add_tail(3);

        let mut visited = Vec::new();
        let mut h = list.head();
        while let Some(idx) = h {
            let val = list[idx];
            visited.push(val);
            if val == 2 {
                list.unlink(idx);
            }
            h = list.next();
        }
        assert_eq!(visited, vec![1, 2, 3]);

        assert_eq!(list.remove_head(), Some(1));
        assert_eq!(list.remove_head(), Some(3));
        assert_eq!(list.remove_head(), None);
    }

    #[test]
    fn clear() {
        let mut list = LinkList::new();
        list.add_tail(1);
        list.add_tail(2);
        list.add_tail(3);
        list.clear();
        assert_eq!(list.remove_head(), None);
    }

    // --- speedup bug tests ---

    #[test]
    fn speedup_bug_single_element_no_speedup() {
        let mut list = LinkList::new();
        list.add_tail(1);

        let mut visited = Vec::new();
        let mut h = list.head();
        while let Some(idx) = h {
            visited.push(list[idx]);
            if list[idx] == 1 {
                list.add_tail(2);
            }
            h = list.next();
        }
        assert_eq!(visited, vec![1]);
    }

    #[test]
    fn speedup_bug_two_elements_speedup() {
        let mut list = LinkList::new();
        list.add_tail(1);
        list.add_tail(2);

        let mut visited = Vec::new();
        let mut h = list.head();
        while let Some(idx) = h {
            visited.push(list[idx]);
            if list[idx] == 1 {
                list.add_tail(3);
            }
            h = list.next();
        }
        assert_eq!(visited, vec![1, 2, 3]);
    }

    #[test]
    fn speedup_bug_add_at_last_element_no_speedup() {
        let mut list = LinkList::new();
        list.add_tail(1);
        list.add_tail(2);

        let mut visited = Vec::new();
        let mut h = list.head();
        while let Some(idx) = h {
            visited.push(list[idx]);
            if list[idx] == 2 {
                list.add_tail(3);
            }
            h = list.next();
        }
        assert_eq!(visited, vec![1, 2]);
    }

    #[test]
    fn speedup_bug_middle_insert_speedup() {
        let mut list = LinkList::new();
        list.add_tail(1);
        list.add_tail(2);
        list.add_tail(3);

        let mut visited = Vec::new();
        let mut h = list.head();
        while let Some(idx) = h {
            visited.push(list[idx]);
            if list[idx] == 2 {
                list.add_tail(4);
            }
            h = list.next();
        }
        assert_eq!(visited, vec![1, 2, 3, 4]);
    }

    #[test]
    fn empty_list_operations() {
        let mut list: LinkList<i32> = LinkList::new();
        assert!(list.is_empty());
        assert_eq!(list.head(), None);
        assert_eq!(list.tail(), None);
        assert_eq!(list.next(), None);
        assert_eq!(list.prev(), None);
        assert_eq!(list.remove_head(), None);
    }

    #[test]
    fn single_element_head_tail() {
        let mut list = LinkList::new();
        list.add_tail(42);
        let h = list.head();
        assert!(h.is_some());
        assert_eq!(list[h.unwrap()], 42);
        assert_eq!(list.next(), None);

        list.add_tail(99);
        list.clear();
        list.add_tail(7);
        let t = list.tail();
        assert!(t.is_some());
        assert_eq!(list[t.unwrap()], 7);
        assert_eq!(list.prev(), None);
    }

    #[test]
    fn index_and_index_mut() {
        let mut list = LinkList::new();
        list.add_tail(10);
        list.add_tail(20);

        let h = list.head().unwrap();
        assert_eq!(list[h], 10);
        list[h] = 100;
        assert_eq!(list[h], 100);

        let n = list.next().unwrap();
        assert_eq!(list[n], 20);
        list[n] = 200;
        assert_eq!(list[n], 200);
    }

    #[test]
    fn get_and_get_mut() {
        let mut list = LinkList::new();
        list.add_tail("hello".to_string());
        list.add_tail("world".to_string());

        let h = list.head().unwrap();
        assert_eq!(list.get(h), &"hello".to_string());
        *list.get_mut(h) = "hi".to_string();
        assert_eq!(list.get(h), &"hi".to_string());
    }

    #[test]
    fn is_empty_transitions() {
        let mut list = LinkList::new();
        assert!(list.is_empty());
        list.add_tail(1);
        assert!(!list.is_empty());
        list.remove_head();
        assert!(list.is_empty());
        list.add_head(2);
        assert!(!list.is_empty());
        list.clear();
        assert!(list.is_empty());
    }

    #[test]
    fn mixed_add_head_and_tail() {
        let mut list = LinkList::new();
        list.add_tail(2);
        list.add_head(1);
        list.add_tail(3);
        list.add_head(0);

        let mut result = Vec::new();
        let mut h = list.head();
        while let Some(idx) = h {
            result.push(list[idx]);
            h = list.next();
        }
        assert_eq!(result, vec![0, 1, 2, 3]);
    }

    #[test]
    fn unlink_head_during_iteration() {
        let mut list = LinkList::new();
        list.add_tail(1);
        list.add_tail(2);
        list.add_tail(3);

        let mut visited = Vec::new();
        let mut h = list.head();
        while let Some(idx) = h {
            let val = list[idx];
            visited.push(val);
            if val == 1 {
                list.unlink(idx);
            }
            h = list.next();
        }
        assert_eq!(visited, vec![1, 2, 3]);
        assert_eq!(list.remove_head(), Some(2));
        assert_eq!(list.remove_head(), Some(3));
        assert_eq!(list.remove_head(), None);
    }

    #[test]
    fn unlink_tail_during_iteration() {
        let mut list = LinkList::new();
        list.add_tail(1);
        list.add_tail(2);
        list.add_tail(3);

        let mut visited = Vec::new();
        let mut h = list.head();
        while let Some(idx) = h {
            let val = list[idx];
            visited.push(val);
            if val == 3 {
                list.unlink(idx);
            }
            h = list.next();
        }
        assert_eq!(visited, vec![1, 2, 3]);
        assert_eq!(list.remove_head(), Some(1));
        assert_eq!(list.remove_head(), Some(2));
        assert_eq!(list.remove_head(), None);
    }

    #[test]
    fn reuse_freed_slots() {
        let mut list = LinkList::new();
        list.add_tail(1);
        list.add_tail(2);
        list.clear();
        list.add_tail(10);
        list.add_tail(20);
        assert_eq!(list.remove_head(), Some(10));
        assert_eq!(list.remove_head(), Some(20));
        assert_eq!(list.remove_head(), None);
    }

    #[test]
    fn unlink_all_one_by_one() {
        let mut list = LinkList::new();
        list.add_tail(1);
        list.add_tail(2);
        list.add_tail(3);

        let mut handles = Vec::new();
        let mut h = list.head();
        while let Some(idx) = h {
            handles.push(idx);
            h = list.next();
        }

        for &handle in &handles {
            list.unlink(handle);
        }
        assert!(list.is_empty());
    }

    #[test]
    fn reverse_iteration_after_unlink() {
        let mut list = LinkList::new();
        list.add_tail(10);
        list.add_tail(20);
        list.add_tail(30);
        list.add_tail(40);

        // Remove 20 by finding its handle
        let mut h = list.head();
        while let Some(idx) = h {
            if list[idx] == 20 {
                list.unlink(idx);
                break;
            }
            h = list.next();
        }

        let mut result = Vec::new();
        let mut h = list.tail();
        while let Some(idx) = h {
            result.push(list[idx]);
            h = list.prev();
        }
        assert_eq!(result, vec![40, 30, 10]);
    }

    #[test]
    fn many_elements_stress() {
        let mut list = LinkList::new();
        for i in 0..1000 {
            list.add_tail(i);
        }

        let mut count = 0;
        let mut h = list.head();
        while let Some(idx) = h {
            assert_eq!(list[idx], count);
            count += 1;
            h = list.next();
        }
        assert_eq!(count, 1000);

        for _ in 0..1000 {
            list.remove_head();
        }
        assert!(list.is_empty());
    }

    #[test]
    fn default_trait() {
        let list: LinkList<i32> = Default::default();
        assert!(list.is_empty());
    }

    #[test]
    fn add_after_clear_reuses_memory() {
        let mut list = LinkList::new();
        for i in 0..100 {
            list.add_tail(i);
        }
        list.clear();
        for i in 0..50 {
            list.add_tail(i * 10);
        }
        let mut count = 0;
        let mut h = list.head();
        while let Some(idx) = h {
            assert_eq!(list[idx], count * 10);
            count += 1;
            h = list.next();
        }
        assert_eq!(count, 50);
    }

    #[test]
    fn untouched_list_owns_no_allocation() {
        let list: LinkList<i32> = LinkList::new();
        assert_eq!(list.entries.capacity(), 0);
        assert_eq!(list.free.capacity(), 0);
    }

    #[test]
    fn read_paths_on_untouched_list() {
        let mut list: LinkList<i32> = LinkList::new();
        assert!(list.is_empty());
        assert_eq!(list.iter().count(), 0);
        list.clear();
        assert_eq!(list.entries.capacity(), 0);
        assert!(list.is_empty());
    }

    #[test]
    fn untouched_list_becomes_usable_on_first_insert() {
        let mut list: LinkList<i32> = LinkList::new();
        assert!(list.is_empty());
        assert_eq!(list.head(), None);
        assert_eq!(list.tail(), None);

        list.add_tail(1);
        list.add_tail(2);
        list.add_head(0);

        assert!(!list.is_empty());
        assert_eq!(list.iter().copied().collect::<Vec<_>>(), vec![0, 1, 2]);
        assert_eq!(list.remove_head(), Some(0));
        assert_eq!(list.remove_head(), Some(1));
        assert_eq!(list.remove_head(), Some(2));
        assert_eq!(list.remove_head(), None);
        assert!(list.is_empty());
    }
}
