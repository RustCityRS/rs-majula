use rs_pack::cache::{ScriptVarType, VarValue};

/// Type-aware variable storage container backed by a [`Vec<VarValue>`].
///
/// Each slot is initialized to a type-appropriate default via
/// [`VarValue::default_for`]: `0` for [`ScriptVarType::Int`], `""` for
/// [`ScriptVarType::String`], and `-1` for reference types such as
/// [`ScriptVarType::Obj`], [`ScriptVarType::Npc`],
/// [`ScriptVarType::Coord`], and [`ScriptVarType::Boolean`].
///
/// `VarSet` is used in two contexts:
/// - **Player variables (varps):** stored on each [`Player`] and accessed by
///   the VM through the `PlayerEngine::get_var` / `PlayerEngine::set_var`
///   trait methods.
/// - **NPC variables (varns):** stored on each [`ActiveNpc`] and accessed by
///   the VM through the `NpcEngine::get_var` / `NpcEngine::set_var` trait
///   methods.
pub struct VarSet(Vec<VarValue>);

impl VarSet {
    /// Creates a new `VarSet` by mapping each [`ScriptVarType`] to its
    /// type-appropriate default value via [`VarValue::default_for`].
    ///
    /// # Arguments
    /// * `types` - An iterator of [`ScriptVarType`] entries that defines the
    ///   number of slots and the default value for each slot.
    ///
    /// # Returns
    /// A `VarSet` whose length equals the number of items yielded by `types`,
    /// with every slot initialized to its type-specific default.
    ///
    /// # Call Stack
    /// **Called by:**
    /// - `ActivePlayer::new` (rs-engine/src/active_player.rs) -- player varps
    ///   from `cache.varps` type definitions.
    /// - `Engine::add_npc_spawned` (rs-engine/src/engine.rs) -- NPC varns from
    ///   `cache.varns` type definitions.
    /// - `GameMap` NPC loading (rs-engine/src/game_map.rs) -- NPC varns during
    ///   map deserialization.
    /// - `Player::new` (rs-engine/src/player_save.rs) -- player varps during
    ///   player construction.
    ///
    /// **Calls:** [`VarValue::default_for`] for each type.
    pub fn new(types: impl Iterator<Item = ScriptVarType>) -> Self {
        VarSet(types.map(VarValue::default_for).collect())
    }

    /// Returns a reference to the variable value at the given index.
    ///
    /// # Arguments
    /// * `id` - Zero-based variable index, cast to `usize` for the lookup.
    ///
    /// # Returns
    /// A reference to the [`VarValue`] stored at position `id`.
    ///
    /// # Panics
    /// Panics if `id as usize` is out of bounds (i.e. `id >= self.len()`).
    ///
    /// # Call Stack
    /// **Called by:**
    /// - `PlayerEngine::get_var` (rs-engine/src/engine.rs) -- reads a player
    ///   varp for the VM.
    /// - `NpcEngine::get_var` (rs-engine/src/engine.rs) -- reads an NPC varn
    ///   for the VM.
    /// - `ActivePlayer::set_varp` / varp transmission
    ///   (rs-engine/src/active_player.rs) -- reads varp values for client sync.
    /// - NPC hunt phase checks (rs-engine/src/phases/npc.rs) -- reads varps
    ///   and varns during hunt condition evaluation.
    /// - Player save serialization (rs-engine/src/player_save.rs) -- reads
    ///   each permanent varp for persistence.
    #[inline]
    pub fn get(&self, id: u16) -> &VarValue {
        &self.0[id as usize]
    }

    /// Overwrites the variable value at the given index.
    ///
    /// # Arguments
    /// * `id` - Zero-based variable index, cast to `usize` for the lookup.
    /// * `value` - The new [`VarValue`] to store. No type-checking is
    ///   performed against the original [`ScriptVarType`]; the caller is
    ///   responsible for supplying a compatible value.
    ///
    /// # Side Effects
    /// Replaces the existing value in-place. When called through
    /// `ActivePlayer::set_varp`, the new value is also transmitted to the
    /// client.
    ///
    /// # Panics
    /// Panics if `id as usize` is out of bounds (i.e. `id >= self.len()`).
    ///
    /// # Call Stack
    /// **Called by:**
    /// - `ActivePlayer::set_varp` (rs-engine/src/active_player.rs) -- sets a
    ///   player varp and optionally transmits it to the client.
    /// - `NpcEngine::set_var` (rs-engine/src/engine.rs) -- sets an NPC varn
    ///   from within the VM.
    /// - `PlayerEngine::set_var` (rs-engine/src/engine.rs) -- delegates to
    ///   `ActivePlayer::set_varp`.
    #[inline]
    pub fn set(&mut self, id: u16, value: VarValue) {
        self.0[id as usize] = value;
    }

    /// Returns the number of variable slots in this set.
    ///
    /// # Returns
    /// The total count of variables, equal to the number of types supplied
    /// to [`VarSet::new`].
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if this set contains no variable slots.
    ///
    /// # Returns
    /// `true` when `self.len() == 0`, i.e. the set was constructed from an
    /// empty iterator.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Resets existing variable slots back to their type-appropriate defaults.
    ///
    /// Iterates over `types` in order and overwrites each corresponding slot
    /// with [`VarValue::default_for`]. Slots beyond the length of `types` are
    /// left unchanged, and extra types beyond the current length are ignored.
    ///
    /// # Arguments
    /// * `types` - An iterator of [`ScriptVarType`] entries. Each entry is
    ///   paired positionally with the internal values vector.
    ///
    /// # Side Effects
    /// Overwrites up to `min(types.count(), self.len())` slots in place with
    /// their default values.
    ///
    /// # Call Stack
    /// **Called by:** player login / save loading paths to clear temporary
    /// varps back to defaults before applying persisted values.
    ///
    /// **Calls:** [`VarValue::default_for`] for each type.
    pub fn reset(&mut self, types: impl Iterator<Item = ScriptVarType>) {
        for (i, var_type) in types.enumerate() {
            if i < self.0.len() {
                self.0[i] = VarValue::default_for(var_type);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_empty_iterator() {
        let vs = VarSet::new(std::iter::empty());
        assert_eq!(vs.len(), 0);
        assert!(vs.is_empty());
    }

    #[test]
    fn new_with_single_int() {
        let vs = VarSet::new([ScriptVarType::Int].into_iter());
        assert_eq!(vs.len(), 1);
        assert!(!vs.is_empty());
        assert_eq!(vs.get(0).as_int(), 0);
    }

    #[test]
    fn new_with_single_string() {
        let vs = VarSet::new([ScriptVarType::String].into_iter());
        assert_eq!(vs.len(), 1);
        if let VarValue::String(s) = vs.get(0) {
            assert_eq!(s, "");
        } else {
            panic!("Expected String variant");
        }
    }

    #[test]
    fn new_with_boolean_defaults_to_minus_one() {
        let vs = VarSet::new([ScriptVarType::Boolean].into_iter());
        assert_eq!(vs.get(0).as_int(), -1);
    }

    #[test]
    fn new_with_obj_defaults_to_minus_one() {
        let vs = VarSet::new([ScriptVarType::Obj].into_iter());
        assert_eq!(vs.get(0).as_int(), -1);
    }

    #[test]
    fn new_with_multiple_types() {
        let types = [
            ScriptVarType::Int,
            ScriptVarType::String,
            ScriptVarType::Boolean,
        ];
        let vs = VarSet::new(types.into_iter());
        assert_eq!(vs.len(), 3);
        assert_eq!(vs.get(0).as_int(), 0);
        assert_eq!(vs.get(2).as_int(), -1);
    }

    #[test]
    fn set_and_get_int() {
        let mut vs = VarSet::new([ScriptVarType::Int].into_iter());
        vs.set(0, VarValue::Int(42));
        assert_eq!(vs.get(0).as_int(), 42);
    }

    #[test]
    fn set_and_get_string() {
        let mut vs = VarSet::new([ScriptVarType::String].into_iter());
        vs.set(0, VarValue::String("hello".into()));
        if let VarValue::String(s) = vs.get(0) {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected String variant");
        }
    }

    #[test]
    fn set_overwrites_previous() {
        let mut vs = VarSet::new([ScriptVarType::Int].into_iter());
        vs.set(0, VarValue::Int(10));
        vs.set(0, VarValue::Int(20));
        assert_eq!(vs.get(0).as_int(), 20);
    }

    #[test]
    fn set_specific_index() {
        let types = [ScriptVarType::Int, ScriptVarType::Int, ScriptVarType::Int];
        let mut vs = VarSet::new(types.into_iter());
        vs.set(1, VarValue::Int(99));
        assert_eq!(vs.get(0).as_int(), 0);
        assert_eq!(vs.get(1).as_int(), 99);
        assert_eq!(vs.get(2).as_int(), 0);
    }

    #[test]
    fn len_matches_type_count() {
        let types = vec![ScriptVarType::Int; 50];
        let vs = VarSet::new(types.into_iter());
        assert_eq!(vs.len(), 50);
    }

    #[test]
    fn is_empty_false_with_values() {
        let vs = VarSet::new([ScriptVarType::Int].into_iter());
        assert!(!vs.is_empty());
    }

    #[test]
    fn various_types_default_values() {
        let types = [
            ScriptVarType::Int,
            ScriptVarType::Coord,
            ScriptVarType::Npc,
            ScriptVarType::Loc,
            ScriptVarType::Enum,
            ScriptVarType::Stat,
        ];
        let vs = VarSet::new(types.into_iter());
        assert_eq!(vs.len(), 6);
        assert_eq!(vs.get(0).as_int(), 0); // Int defaults to 0
        // Most ref types default to -1
        for i in 1..6 {
            assert_eq!(vs.get(i as u16).as_int(), -1);
        }
    }

    #[test]
    fn from_int_and_set() {
        let mut vs = VarSet::new([ScriptVarType::Obj].into_iter());
        let val = VarValue::from_int(ScriptVarType::Obj, 100);
        vs.set(0, val);
        assert_eq!(vs.get(0).as_int(), 100);
    }

    #[test]
    fn large_varset() {
        let types = vec![ScriptVarType::Int; 1000];
        let mut vs = VarSet::new(types.into_iter());
        assert_eq!(vs.len(), 1000);
        vs.set(999, VarValue::Int(12345));
        assert_eq!(vs.get(999).as_int(), 12345);
        assert_eq!(vs.get(0).as_int(), 0);
    }

    #[test]
    fn set_int_then_read_as_int() {
        let mut vs = VarSet::new([ScriptVarType::Int].into_iter());
        vs.set(0, VarValue::from_int(ScriptVarType::Int, 42));
        assert_eq!(vs.get(0).as_int(), 42);
    }

    #[test]
    fn string_varp_pattern() {
        // Engine does: if var_type == String { VarValue::String(s) } else { from_int }
        let mut vs = VarSet::new([ScriptVarType::String, ScriptVarType::Int].into_iter());
        vs.set(0, VarValue::String("hello".into()));
        vs.set(1, VarValue::from_int(ScriptVarType::Int, 100));

        if let VarValue::String(s) = vs.get(0) {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected String");
        }
        assert_eq!(vs.get(1).as_int(), 100);
    }

    #[test]
    fn boolean_varp_as_int() {
        let mut vs = VarSet::new([ScriptVarType::Boolean].into_iter());
        assert_eq!(vs.get(0).as_int(), -1); // default
        vs.set(0, VarValue::from_int(ScriptVarType::Boolean, 1));
        assert_eq!(vs.get(0).as_int(), 1);
    }

    #[test]
    fn coord_varp() {
        let mut vs = VarSet::new([ScriptVarType::Coord].into_iter());
        assert_eq!(vs.get(0).as_int(), -1); // default for coord
        vs.set(0, VarValue::from_int(ScriptVarType::Coord, 3200));
        assert_eq!(vs.get(0).as_int(), 3200);
    }

    #[test]
    fn multiple_varp_modifications() {
        let types = vec![ScriptVarType::Int; 10];
        let mut vs = VarSet::new(types.into_iter());
        for i in 0..10 {
            vs.set(i, VarValue::Int(i as i32 * 100));
        }
        for i in 0..10 {
            assert_eq!(vs.get(i).as_int(), i as i32 * 100);
        }
    }

    #[test]
    fn obj_varp_default_minus_one() {
        let vs = VarSet::new([ScriptVarType::Obj].into_iter());
        assert_eq!(vs.get(0).as_int(), -1);
    }

    #[test]
    fn npc_varp_default_minus_one() {
        let vs = VarSet::new([ScriptVarType::Npc].into_iter());
        assert_eq!(vs.get(0).as_int(), -1);
    }

    #[test]
    fn stat_varp() {
        let mut vs = VarSet::new([ScriptVarType::Stat].into_iter());
        vs.set(0, VarValue::from_int(ScriptVarType::Stat, 5));
        assert_eq!(vs.get(0).as_int(), 5);
    }
}
