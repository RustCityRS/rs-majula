/// A packed 32-bit unique identifier for an NPC, combining an NPC type ID and an NPC
/// index into a single `u32` value.
///
/// Bit layout: `(id << 16) | nid`
///
/// The upper 16 bits store the NPC type ID (the definition/config ID that identifies
/// the kind of NPC), while the lower 16 bits store the NPC index (the slot in the
/// engine's NPC array, used for direct array lookups).
///
/// Used throughout the engine for NPC identification in scripts, interactions, and
/// zone management.
#[derive(Debug, Copy, Clone)]
pub struct NpcUid(u32);

impl NpcUid {
    /// Constructs a new `NpcUid` by packing the NPC type ID and NPC index into a `u32`.
    ///
    /// # Arguments
    /// * `id` - The NPC type/config ID that identifies what kind of NPC this is.
    /// * `nid` - The NPC index (slot in the engine's NPC array). Used as a direct
    ///   array index for lookups (e.g. `npcs[nid as usize]`).
    ///
    /// # Returns
    /// An `NpcUid` whose packed representation is `(id << 16) | nid`.
    #[inline(always)]
    pub const fn new(id: u16, nid: u16) -> Self {
        Self(((id as u32) << 16) | nid as u32)
    }

    /// Returns the raw packed `u32` value containing both the NPC type ID and index.
    ///
    /// # Returns
    /// The full 32-bit packed representation: `(id << 16) | nid`.
    #[inline(always)]
    pub const fn packed(&self) -> u32 {
        self.0
    }

    /// Extracts the NPC type/config ID from the upper 16 bits of the packed value.
    ///
    /// # Returns
    /// The NPC type ID, identifying the kind of NPC (its definition/config entry).
    #[inline(always)]
    pub const fn id(&self) -> u16 {
        (self.0 >> 16 & 0xFFFF) as u16
    }

    /// Extracts the NPC index from the lower 16 bits of the packed value.
    ///
    /// # Returns
    /// The NPC index, used as a direct array index into the engine's NPC array
    /// (e.g. `npcs[nid as usize]`).
    #[inline(always)]
    pub const fn nid(&self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_accessors() {
        let uid = NpcUid::new(100, 500);
        assert_eq!(uid.id(), 100);
        assert_eq!(uid.nid(), 500);
    }

    #[test]
    fn zero_values() {
        let uid = NpcUid::new(0, 0);
        assert_eq!(uid.id(), 0);
        assert_eq!(uid.nid(), 0);
    }

    #[test]
    fn max_values() {
        let uid = NpcUid::new(u16::MAX, u16::MAX);
        assert_eq!(uid.id(), u16::MAX);
        assert_eq!(uid.nid(), u16::MAX);
    }

    #[test]
    fn packed_representation() {
        let uid = NpcUid::new(1, 2);
        assert_eq!(uid.0, (1u32 << 16) | 2);
    }

    #[test]
    fn different_ids_different_packed() {
        let a = NpcUid::new(1, 2);
        let b = NpcUid::new(2, 1);
        assert_ne!(a.0, b.0);
    }

    #[test]
    fn copy_semantics() {
        let a = NpcUid::new(10, 20);
        let b = a;
        assert_eq!(b.id(), 10);
        assert_eq!(b.nid(), 20);
    }

    #[test]
    fn various_values() {
        for id in [0u16, 1, 100, 1000, u16::MAX] {
            for nid in [0u16, 1, 100, 1000, u16::MAX] {
                let uid = NpcUid::new(id, nid);
                assert_eq!(uid.id(), id, "id mismatch for ({id},{nid})");
                assert_eq!(uid.nid(), nid, "nid mismatch for ({id},{nid})");
            }
        }
    }

    #[test]
    fn uid_packed_as_i32() {
        let uid = NpcUid::new(100, 500);
        let packed = uid.0 as i32;
        assert_eq!(((packed >> 16) & 0xFFFF) as u16, 100);
        assert_eq!((packed & 0xFFFF) as u16, 500);
    }

    #[test]
    fn nid_used_as_array_index() {
        // Engine indexes npcs[nid as usize]
        let uid = NpcUid::new(42, 8191);
        assert_eq!(uid.nid(), 8191);
    }

    #[test]
    fn id_is_npc_type() {
        let uid = NpcUid::new(999, 0);
        assert_eq!(uid.id(), 999);
    }

    #[test]
    fn many_nids_unique() {
        let mut nids = std::collections::HashSet::new();
        for nid in 0..100u16 {
            let uid = NpcUid::new(1, nid);
            nids.insert(uid.nid());
        }
        assert_eq!(nids.len(), 100);
    }

    #[test]
    fn same_nid_different_type() {
        let a = NpcUid::new(1, 100);
        let b = NpcUid::new(2, 100);
        assert_ne!(a.0, b.0);
        assert_eq!(a.nid(), b.nid());
        assert_ne!(a.id(), b.id());
    }
}
