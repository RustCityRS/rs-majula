use crate::ScriptError;

/// Identifies which entity pointer slot is active within a [`ScriptState`](crate::state::ScriptState).
///
/// Each variant corresponds to one of the VM's entity reference fields
/// (e.g. `active_player`, `active_npc2`). The discriminant values are
/// used as bit indices inside [`ScriptPointerSet`] to form a compact
/// bitset that tracks which pointers are currently valid.
///
/// Primary variants (no `2` suffix) refer to the subject entity, while
/// secondary variants (`2` suffix) refer to the target entity.
/// `Protected` variants guard player pointers that must not be invalidated
/// by nested script calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ScriptPointer {
    ActivePlayer = 0,
    ActivePlayer2 = 1,
    ProtectedActivePlayer = 2,
    ProtectedActivePlayer2 = 3,
    ActiveNpc = 4,
    ActiveNpc2 = 5,
    ActiveLoc = 6,
    ActiveLoc2 = 7,
    ActiveObj = 8,
    ActiveObj2 = 9,
}

impl ScriptPointer {
    /// Returns the human-readable name of this pointer variant.
    ///
    /// Secondary pointer names are prefixed with `"."` (e.g. `".active_player"`),
    /// and protected pointer names are prefixed with `"p_"` (e.g. `"p_active_player"`).
    ///
    /// # Returns
    ///
    /// A `&'static str` label suitable for diagnostic messages and error reporting.
    ///
    /// **Called by:** [`ScriptPointerSet::check`] for error messages.
    pub const fn name(self) -> &'static str {
        match self {
            Self::ActivePlayer => "active_player",
            Self::ActivePlayer2 => ".active_player",
            Self::ProtectedActivePlayer => "p_active_player",
            Self::ProtectedActivePlayer2 => ".p_active_player",
            Self::ActiveNpc => "active_npc",
            Self::ActiveNpc2 => ".active_npc",
            Self::ActiveLoc => "active_loc",
            Self::ActiveLoc2 => ".active_loc",
            Self::ActiveObj => "active_obj",
            Self::ActiveObj2 => ".active_obj",
        }
    }
}

/// A compact bitset that tracks which [`ScriptPointer`] slots are currently
/// active in a [`ScriptState`](crate::state::ScriptState).
///
/// Internally stores a `u32` where each bit position corresponds to a
/// [`ScriptPointer`] discriminant value (0..=9). This allows O(1) set/test/clear
/// operations on pointer availability using bitwise arithmetic.
///
/// Used by the VM to validate that a required entity pointer (e.g. `active_player`)
/// is present before executing an opcode that references it.
#[derive(Clone, Copy, Default, Debug)]
pub struct ScriptPointerSet(u32);

impl ScriptPointerSet {
    /// Creates an empty pointer set with no active pointers.
    ///
    /// # Returns
    ///
    /// A `ScriptPointerSet` with all bits cleared (internal value `0`).
    ///
    /// **Called by:** [`ScriptState::new`](crate::state::ScriptState::new) during VM state construction.
    #[inline(always)]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Marks a pointer as active by setting its corresponding bit.
    ///
    /// This is idempotent -- adding an already-active pointer has no effect.
    ///
    /// # Arguments
    ///
    /// * `ptr` - The [`ScriptPointer`] to activate.
    ///
    /// # Side Effects
    ///
    /// Sets bit `ptr as u8` in the internal `u32`.
    ///
    /// **Called by:** [`ScriptState::sync_pointers`](crate::state::ScriptState) after entity binding.
    #[inline(always)]
    pub const fn add(&mut self, ptr: ScriptPointer) {
        self.0 |= 1u32 << (ptr as u8);
    }

    /// Marks a pointer as inactive by clearing its corresponding bit.
    ///
    /// This is idempotent -- removing an already-inactive pointer has no effect.
    ///
    /// # Arguments
    ///
    /// * `ptr` - The [`ScriptPointer`] to deactivate.
    ///
    /// # Side Effects
    ///
    /// Clears bit `ptr as u8` in the internal `u32`.
    ///
    /// **Called by:** [`remove_all`](Self::remove_all), and engine opcode handlers
    /// that invalidate entity references.
    #[inline(always)]
    pub const fn remove(&mut self, ptr: ScriptPointer) {
        self.0 &= !(1u32 << (ptr as u8));
    }

    /// Tests whether the given pointer is currently active.
    ///
    /// # Arguments
    ///
    /// * `ptr` - The [`ScriptPointer`] to test.
    ///
    /// # Returns
    ///
    /// `true` if the bit for `ptr` is set, `false` otherwise.
    ///
    /// **Called by:** [`check`](Self::check) and engine opcode handlers that
    /// conditionally access entity references.
    #[inline(always)]
    pub const fn has(self, ptr: ScriptPointer) -> bool {
        (self.0 & (1u32 << (ptr as u8))) != 0
    }

    /// Asserts that the given pointer is active, returning a runtime error if not.
    ///
    /// This is the primary guard used before accessing an entity reference in
    /// the VM. If the pointer is missing, the error message includes the
    /// pointer's [`name`](ScriptPointer::name) for diagnostics.
    ///
    /// # Arguments
    ///
    /// * `ptr` - The [`ScriptPointer`] that must be active.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the pointer is set.
    /// * `Err(ScriptError::Runtime)` with the pointer name if the pointer is not set.
    ///
    /// # Call Stack
    ///
    /// **Calls:** [`has`](Self::has), [`ScriptPointer::name`].
    ///
    /// **Called by:** Engine opcode handlers that require a specific entity pointer
    /// before executing (e.g. player, NPC, loc, obj operations).
    #[inline(always)]
    pub fn check(self, ptr: ScriptPointer) -> crate::Result<()> {
        if self.has(ptr) {
            Ok(())
        } else {
            Err(ScriptError::Runtime(format!(
                "required pointer not set: {}",
                ptr.name()
            )))
        }
    }

    /// Removes all pointers in the given slice from this set.
    ///
    /// Iterates over `ptrs` and clears each corresponding bit. Passing an
    /// empty slice is a no-op.
    ///
    /// # Arguments
    ///
    /// * `ptrs` - A slice of [`ScriptPointer`] values to deactivate.
    ///
    /// # Side Effects
    ///
    /// Clears the bit for each pointer in `ptrs`.
    ///
    /// # Call Stack
    ///
    /// **Calls:** [`remove`](Self::remove) for each pointer.
    ///
    /// **Called by:** Engine opcode handlers that invalidate groups of related
    /// pointers (e.g. clearing both primary and secondary player pointers).
    pub fn remove_all(&mut self, ptrs: &[ScriptPointer]) {
        for &ptr in ptrs {
            self.remove(ptr);
        }
    }

    /// Clears all pointer bits, marking every pointer as inactive.
    ///
    /// # Side Effects
    ///
    /// Resets the internal `u32` to `0`.
    ///
    /// **Called by:** [`ScriptState::sync_pointers`](crate::state::ScriptState)
    /// before re-populating the set from the current entity fields.
    #[inline(always)]
    pub const fn clear(&mut self) {
        self.0 = 0u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_names() {
        assert_eq!(ScriptPointer::ActivePlayer.name(), "active_player");
        assert_eq!(ScriptPointer::ActivePlayer2.name(), ".active_player");
        assert_eq!(
            ScriptPointer::ProtectedActivePlayer.name(),
            "p_active_player"
        );
        assert_eq!(
            ScriptPointer::ProtectedActivePlayer2.name(),
            ".p_active_player"
        );
        assert_eq!(ScriptPointer::ActiveNpc.name(), "active_npc");
        assert_eq!(ScriptPointer::ActiveNpc2.name(), ".active_npc");
        assert_eq!(ScriptPointer::ActiveLoc.name(), "active_loc");
        assert_eq!(ScriptPointer::ActiveLoc2.name(), ".active_loc");
        assert_eq!(ScriptPointer::ActiveObj.name(), "active_obj");
        assert_eq!(ScriptPointer::ActiveObj2.name(), ".active_obj");
    }

    #[test]
    fn pointer_set_new_is_empty() {
        let set = ScriptPointerSet::new();
        assert!(!set.has(ScriptPointer::ActivePlayer));
        assert!(!set.has(ScriptPointer::ActiveNpc));
    }

    #[test]
    fn add_and_has() {
        let mut set = ScriptPointerSet::new();
        set.add(ScriptPointer::ActivePlayer);
        assert!(set.has(ScriptPointer::ActivePlayer));
        assert!(!set.has(ScriptPointer::ActivePlayer2));
    }

    #[test]
    fn add_multiple() {
        let mut set = ScriptPointerSet::new();
        set.add(ScriptPointer::ActivePlayer);
        set.add(ScriptPointer::ActiveNpc);
        set.add(ScriptPointer::ActiveLoc);
        assert!(set.has(ScriptPointer::ActivePlayer));
        assert!(set.has(ScriptPointer::ActiveNpc));
        assert!(set.has(ScriptPointer::ActiveLoc));
        assert!(!set.has(ScriptPointer::ActiveObj));
    }

    #[test]
    fn remove_pointer() {
        let mut set = ScriptPointerSet::new();
        set.add(ScriptPointer::ActivePlayer);
        set.add(ScriptPointer::ActiveNpc);
        set.remove(ScriptPointer::ActivePlayer);
        assert!(!set.has(ScriptPointer::ActivePlayer));
        assert!(set.has(ScriptPointer::ActiveNpc));
    }

    #[test]
    fn remove_nonexistent_no_effect() {
        let mut set = ScriptPointerSet::new();
        set.add(ScriptPointer::ActivePlayer);
        set.remove(ScriptPointer::ActiveNpc);
        assert!(set.has(ScriptPointer::ActivePlayer));
    }

    #[test]
    fn clear_removes_all() {
        let mut set = ScriptPointerSet::new();
        set.add(ScriptPointer::ActivePlayer);
        set.add(ScriptPointer::ActiveNpc);
        set.add(ScriptPointer::ActiveLoc);
        set.clear();
        assert!(!set.has(ScriptPointer::ActivePlayer));
        assert!(!set.has(ScriptPointer::ActiveNpc));
        assert!(!set.has(ScriptPointer::ActiveLoc));
    }

    #[test]
    fn check_returns_ok_when_set() {
        let mut set = ScriptPointerSet::new();
        set.add(ScriptPointer::ActivePlayer);
        assert!(set.check(ScriptPointer::ActivePlayer).is_ok());
    }

    #[test]
    fn check_returns_err_when_not_set() {
        let set = ScriptPointerSet::new();
        assert!(set.check(ScriptPointer::ActivePlayer).is_err());
    }

    #[test]
    fn check_error_contains_pointer_name() {
        let set = ScriptPointerSet::new();
        let err = set.check(ScriptPointer::ActiveNpc).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("active_npc"), "Error message: {msg}");
    }

    #[test]
    fn remove_all_removes_listed_pointers() {
        let mut set = ScriptPointerSet::new();
        set.add(ScriptPointer::ActivePlayer);
        set.add(ScriptPointer::ActivePlayer2);
        set.add(ScriptPointer::ActiveNpc);
        set.remove_all(&[ScriptPointer::ActivePlayer, ScriptPointer::ActivePlayer2]);
        assert!(!set.has(ScriptPointer::ActivePlayer));
        assert!(!set.has(ScriptPointer::ActivePlayer2));
        assert!(set.has(ScriptPointer::ActiveNpc));
    }

    #[test]
    fn remove_all_empty_slice_no_effect() {
        let mut set = ScriptPointerSet::new();
        set.add(ScriptPointer::ActivePlayer);
        set.remove_all(&[]);
        assert!(set.has(ScriptPointer::ActivePlayer));
    }

    #[test]
    fn add_all_pointers() {
        let mut set = ScriptPointerSet::new();
        let all = [
            ScriptPointer::ActivePlayer,
            ScriptPointer::ActivePlayer2,
            ScriptPointer::ProtectedActivePlayer,
            ScriptPointer::ProtectedActivePlayer2,
            ScriptPointer::ActiveNpc,
            ScriptPointer::ActiveNpc2,
            ScriptPointer::ActiveLoc,
            ScriptPointer::ActiveLoc2,
            ScriptPointer::ActiveObj,
            ScriptPointer::ActiveObj2,
        ];
        for &ptr in &all {
            set.add(ptr);
        }
        for &ptr in &all {
            assert!(set.has(ptr));
        }
    }

    #[test]
    fn double_add_idempotent() {
        let mut set = ScriptPointerSet::new();
        set.add(ScriptPointer::ActivePlayer);
        set.add(ScriptPointer::ActivePlayer);
        assert!(set.has(ScriptPointer::ActivePlayer));
        set.remove(ScriptPointer::ActivePlayer);
        assert!(!set.has(ScriptPointer::ActivePlayer));
    }

    #[test]
    fn default_is_empty() {
        let set = ScriptPointerSet::default();
        assert!(!set.has(ScriptPointer::ActivePlayer));
    }

    #[test]
    fn copy_semantics() {
        let mut set = ScriptPointerSet::new();
        set.add(ScriptPointer::ActivePlayer);
        let copy = set;
        assert!(copy.has(ScriptPointer::ActivePlayer));
        set.remove(ScriptPointer::ActivePlayer);
        assert!(copy.has(ScriptPointer::ActivePlayer)); // copy is independent
    }

    #[test]
    fn script_state_active_player_constants() {
        use crate::state::ScriptState;
        assert_eq!(ScriptState::ACTIVE_PLAYER[0], ScriptPointer::ActivePlayer);
        assert_eq!(ScriptState::ACTIVE_PLAYER[1], ScriptPointer::ActivePlayer2);
    }

    #[test]
    fn script_state_protected_active_player_constants() {
        use crate::state::ScriptState;
        assert_eq!(
            ScriptState::PROTECTED_ACTIVE_PLAYER[0],
            ScriptPointer::ProtectedActivePlayer
        );
        assert_eq!(
            ScriptState::PROTECTED_ACTIVE_PLAYER[1],
            ScriptPointer::ProtectedActivePlayer2
        );
    }

    #[test]
    fn script_state_active_npc_constants() {
        use crate::state::ScriptState;
        assert_eq!(ScriptState::ACTIVE_NPC[0], ScriptPointer::ActiveNpc);
        assert_eq!(ScriptState::ACTIVE_NPC[1], ScriptPointer::ActiveNpc2);
    }

    #[test]
    fn script_state_active_loc_constants() {
        use crate::state::ScriptState;
        assert_eq!(ScriptState::ACTIVE_LOC[0], ScriptPointer::ActiveLoc);
        assert_eq!(ScriptState::ACTIVE_LOC[1], ScriptPointer::ActiveLoc2);
    }

    #[test]
    fn script_state_active_obj_constants() {
        use crate::state::ScriptState;
        assert_eq!(ScriptState::ACTIVE_OBJ[0], ScriptPointer::ActiveObj);
        assert_eq!(ScriptState::ACTIVE_OBJ[1], ScriptPointer::ActiveObj2);
    }

    #[test]
    fn pointer_set_check_error_message_format() {
        let set = ScriptPointerSet::new();
        let err = set.check(ScriptPointer::ProtectedActivePlayer).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("p_active_player"), "got: {msg}");
    }

    #[test]
    fn pointer_set_remove_then_check_fails() {
        let mut set = ScriptPointerSet::new();
        set.add(ScriptPointer::ActivePlayer);
        assert!(set.check(ScriptPointer::ActivePlayer).is_ok());
        set.remove(ScriptPointer::ActivePlayer);
        assert!(set.check(ScriptPointer::ActivePlayer).is_err());
    }

    #[test]
    fn pointer_set_clear_then_add() {
        let mut set = ScriptPointerSet::new();
        set.add(ScriptPointer::ActivePlayer);
        set.add(ScriptPointer::ActiveNpc);
        set.clear();
        set.add(ScriptPointer::ActiveLoc);
        assert!(!set.has(ScriptPointer::ActivePlayer));
        assert!(!set.has(ScriptPointer::ActiveNpc));
        assert!(set.has(ScriptPointer::ActiveLoc));
    }

    #[test]
    fn pointer_repr_values() {
        assert_eq!(ScriptPointer::ActivePlayer as u8, 0);
        assert_eq!(ScriptPointer::ActivePlayer2 as u8, 1);
        assert_eq!(ScriptPointer::ProtectedActivePlayer as u8, 2);
        assert_eq!(ScriptPointer::ProtectedActivePlayer2 as u8, 3);
        assert_eq!(ScriptPointer::ActiveNpc as u8, 4);
        assert_eq!(ScriptPointer::ActiveNpc2 as u8, 5);
        assert_eq!(ScriptPointer::ActiveLoc as u8, 6);
        assert_eq!(ScriptPointer::ActiveLoc2 as u8, 7);
        assert_eq!(ScriptPointer::ActiveObj as u8, 8);
        assert_eq!(ScriptPointer::ActiveObj2 as u8, 9);
    }
}
