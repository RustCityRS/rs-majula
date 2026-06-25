mod renderer;

pub use renderer::{NpcRenderer, PlayerRenderer};
use rs_protocol::network::game::info_prot::{NpcInfoProt, PlayerInfoProt};

/// Discriminator for selecting the correct protocol mask bits when setting
/// face-entity or face-coord updates on an entity.
///
/// NPC and Player info protocols use different bitmask values for the same
/// logical operations (e.g., `FaceEntity` is `0x4` for both, but `FaceCoord`
/// is `0x80` for NPCs and `0x20` for Players). This enum abstracts over
/// that difference so that shared logic in [`EntityMasks`] can operate
/// generically on either entity type.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusKind {
    /// The entity is an NPC; maps to [`NpcInfoProt`] mask values.
    Npc,
    /// The entity is a Player; maps to [`PlayerInfoProt`] mask values.
    Player,
}

impl FocusKind {
    /// Returns the protocol bitmask for a `FaceEntity` info update
    /// corresponding to this entity kind.
    ///
    /// # Returns
    ///
    /// * `NpcInfoProt::FaceEntity as u16` (`0x4`) when `self` is [`FocusKind::Npc`].
    /// * `PlayerInfoProt::FaceEntity as u16` (`0x4`) when `self` is [`FocusKind::Player`].
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`EntityMasks::set_face_entity_check`],
    /// [`EntityMasks::mark_face_entity`], [`EntityMasks::clear_face_entity`].
    pub const fn face_entity_mask(self) -> u16 {
        match self {
            FocusKind::Npc => NpcInfoProt::FaceEntity as u16,
            FocusKind::Player => PlayerInfoProt::FaceEntity as u16,
        }
    }

    /// Returns the protocol bitmask for a `FaceCoord` info update
    /// corresponding to this entity kind.
    ///
    /// # Returns
    ///
    /// * `NpcInfoProt::FaceCoord as u16` (`0x80`) when `self` is [`FocusKind::Npc`].
    /// * `PlayerInfoProt::FaceCoord as u16` (`0x20`) when `self` is [`FocusKind::Player`].
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`EntityMasks::focus`].
    pub const fn face_coord_mask(self) -> u16 {
        match self {
            FocusKind::Npc => NpcInfoProt::FaceCoord as u16,
            FocusKind::Player => PlayerInfoProt::FaceCoord as u16,
        }
    }
}

/// Visibility level of an entity, controlling how it appears (or does not
/// appear) to other players during the info rendering phase.
///
/// This value is persistent across engine ticks -- [`EntityMasks::reset`]
/// does **not** clear it.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
pub enum Visibility {
    /// Normal visibility -- the entity is rendered with the standard info protocol.
    Default,
    /// Soft-hidden -- the entity is hidden but may be revealed by certain game conditions.
    Soft,
    /// Hard-hidden -- the entity is unconditionally hidden from all observers.
    Hard,
}

/// Holds every info-update mask flag and its associated payload data for a
/// single entity (Player or NPC).
///
/// Each engine tick, the game logic sets fields on this struct to describe
/// what has changed (animation played, entity faced, damage dealt, etc.).
/// The info renderer ([`PlayerRenderer`] / [`NpcRenderer`]) then reads
/// these fields, encodes them into the appropriate protocol messages, and
/// sends them to observing clients. At the end of the tick the cleanup
/// phase calls [`EntityMasks::reset`] to clear all *temporary* fields
/// while preserving *persistent* state.
///
/// # Field categories
///
/// **Persistent** (survive [`reset`](EntityMasks::reset)):
/// `appearance`, `last_appearance`, `last_appearance_info`, `readyanim`,
/// `turnanim`, `walkanim`, `walkanim_b`, `walkanim_l`, `walkanim_r`,
/// `runanim`, `face_entity`, `orientation_x`, `orientation_z`,
/// `anim_protect`, `vis`.
///
/// **Temporary** (cleared by [`reset`](EntityMasks::reset)):
/// `masks`, `face_x`, `face_z`, `anim_id`, `anim_delay`, `say`,
/// `damage_taken`, `damage_type`, `damage_current`, `damage_base`,
/// `chat_bytes`, `chat_colour`, `chat_effects`, `chat_ignored`,
/// `spotanim`, `spotanim_height`, `spotanim_delay`,
/// `exactmove_start_x`, `exactmove_start_z`, `exactmove_end_x`,
/// `exactmove_end_z`, `exactmove_begin`, `exactmove_finish`,
/// `exactmove_dir`, `changetype`.
pub struct EntityMasks {
    pub masks: u16,
    pub appearance: Option<u16>,
    pub last_appearance: Option<u32>,
    pub last_appearance_info: Option<Box<[u8]>>,
    pub readyanim: Option<u16>,
    pub turnanim: Option<u16>,
    pub walkanim: Option<u16>,
    pub walkanim_b: Option<u16>,
    pub walkanim_l: Option<u16>,
    pub walkanim_r: Option<u16>,
    pub runanim: Option<u16>,
    pub face_entity: Option<u16>,
    pub face_x: Option<u16>,
    pub face_z: Option<u16>,
    pub orientation_x: Option<u16>,
    pub orientation_z: Option<u16>,
    pub anim_id: Option<u16>,
    pub anim_delay: Option<u8>,
    pub anim_protect: bool,
    pub say: Option<Box<str>>,
    pub damage_taken: Option<u8>,
    pub damage_type: Option<u8>,
    pub damage_current: Option<u8>,
    pub damage_base: Option<u8>,
    #[cfg(since_244)]
    pub damage2_taken: Option<u8>,
    #[cfg(since_244)]
    pub damage2_type: Option<u8>,
    #[cfg(since_244)]
    pub damage2_current: Option<u8>,
    #[cfg(since_244)]
    pub damage2_base: Option<u8>,
    #[cfg(since_244)]
    pub damage_slot: u8,
    pub chat_bytes: Option<Box<[u8]>>,
    pub chat_colour: Option<u8>,
    pub chat_effects: Option<u8>,
    pub chat_ignored: Option<u8>,
    pub spotanim: Option<u16>,
    pub spotanim_height: Option<u16>,
    pub spotanim_delay: Option<u16>,
    pub exactmove_start_x: Option<u16>,
    pub exactmove_start_z: Option<u16>,
    pub exactmove_end_x: Option<u16>,
    pub exactmove_end_z: Option<u16>,
    pub exactmove_begin: Option<u16>,
    pub exactmove_finish: Option<u16>,
    pub exactmove_dir: Option<u8>,
    pub changetype: Option<u16>,
    pub vis: Visibility,
}

impl EntityMasks {
    /// Creates a new `EntityMasks` with every field in its default/empty state.
    ///
    /// All `Option` fields are `None`, `masks` is `0`, `anim_protect` is
    /// `false`, and `vis` is [`Visibility::Default`].
    ///
    /// This is a `const fn` so it can be used in static or const contexts
    /// (e.g., initializing entity pool entries at compile time).
    ///
    /// # Returns
    ///
    /// A fully zeroed `EntityMasks` instance ready for use.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Player::new`, `Npc::new` (rs-entity).
    pub const fn new() -> Self {
        Self {
            masks: 0,
            appearance: None,
            last_appearance: None,
            last_appearance_info: None,
            readyanim: None,
            turnanim: None,
            walkanim: None,
            walkanim_b: None,
            walkanim_l: None,
            walkanim_r: None,
            runanim: None,
            face_entity: None,
            face_x: None,
            face_z: None,
            orientation_x: None,
            orientation_z: None,
            anim_id: None,
            anim_delay: None,
            anim_protect: false,
            say: None,
            damage_taken: None,
            damage_type: None,
            damage_current: None,
            damage_base: None,
            #[cfg(since_244)]
            damage2_taken: None,
            #[cfg(since_244)]
            damage2_type: None,
            #[cfg(since_244)]
            damage2_current: None,
            #[cfg(since_244)]
            damage2_base: None,
            #[cfg(since_244)]
            damage_slot: 0,
            chat_bytes: None,
            chat_colour: None,
            chat_effects: None,
            chat_ignored: None,
            spotanim: None,
            spotanim_height: None,
            spotanim_delay: None,
            exactmove_start_x: None,
            exactmove_start_z: None,
            exactmove_end_x: None,
            exactmove_end_z: None,
            exactmove_begin: None,
            exactmove_finish: None,
            exactmove_dir: None,
            changetype: None,
            vis: Visibility::Default,
        }
    }

    /// Writes a hit into the primary `Damage` slot and flags `bit` for the
    /// next info update.
    ///
    /// `bit` is the rev-specific `Damage` mask bit (`PlayerInfoProt::Damage`
    /// for players, `NpcInfoProt::Damage` for npcs).
    pub fn apply_damage(&mut self, taken: u8, damage_type: u8, current: u8, base: u8, bit: u16) {
        self.damage_taken = Some(taken);
        self.damage_type = Some(damage_type);
        self.damage_current = Some(current);
        self.damage_base = Some(base);
        self.masks |= bit;
    }

    /// Routes a hit to the second-hitmark (`Damage2`) slot on the odd-numbered
    /// hit of a tick. `damage_slot` alternates on every call and is reset to `0`
    /// each tick by [`reset`](EntityMasks::reset), so the first hit lands in
    /// `Damage` and the second in `Damage2`. Returns `true` when this hit filled
    /// `Damage2`, in which case the caller skips the primary `Damage` write.
    ///
    /// `bit` is the rev-specific `Damage2` mask bit (`PlayerInfoProt::Damage2`
    /// for players, `NpcInfoProt::Damage2` for npcs).
    #[cfg(since_244)]
    pub fn apply_damage2(
        &mut self,
        taken: u8,
        damage_type: u8,
        current: u8,
        base: u8,
        bit: u16,
    ) -> bool {
        let slot = self.damage_slot;
        self.damage_slot = slot.wrapping_add(1);
        if slot % 2 == 1 {
            self.damage2_taken = Some(taken);
            self.damage2_type = Some(damage_type);
            self.damage2_current = Some(current);
            self.damage2_base = Some(base);
            self.masks |= bit;
            true
        } else {
            false
        }
    }

    /// Sets the active animation on this entity, subject to priority and
    /// protection checks.
    ///
    /// If `anim_protect` is `true`, the call is a no-op -- the current
    /// animation is locked and cannot be overridden. Otherwise, when both
    /// `current_priority` and `new_priority` are `Some`, the new animation is
    /// applied only when its priority is greater than or equal to the current
    /// one (an equal-priority anim overrides). A `None` priority -- no current
    /// anim, or clearing the anim -- always applies.
    ///
    /// When the animation is accepted, `anim_id` and `anim_delay` are
    /// written and the given `mask_bit` is OR-ed into `masks` so the info
    /// renderer knows to encode this update.
    ///
    /// # Arguments
    ///
    /// * `id` - The animation sequence ID to play, or `None` to clear.
    /// * `delay` - Client-side delay (in ticks) before the animation begins.
    /// * `mask_bit` - The protocol-specific bitmask for the Anim info update
    ///   (differs between `NpcInfoProt::Anim` and `PlayerInfoProt::Anim`).
    /// * `current_priority` - Priority of the currently playing animation, if
    ///   any.
    /// * `new_priority` - Priority of the animation being requested, if any.
    ///
    /// # Side Effects
    ///
    /// * Sets `self.anim_id` and `self.anim_delay` when accepted.
    /// * OR-s `mask_bit` into `self.masks`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptPlayer::anim` and `ScriptNpc::anim` (rs-script).
    pub fn set_anim(
        &mut self,
        id: Option<u16>,
        delay: u8,
        mask_bit: u16,
        current_priority: Option<u16>,
        new_priority: Option<u16>,
    ) {
        if self.anim_protect {
            return;
        }
        if let (Some(cur), Some(new)) = (current_priority, new_priority)
            && new < cur
        {
            return;
        }
        self.anim_id = id;
        self.anim_delay = Some(delay);
        self.masks |= mask_bit;
    }

    /// Clears all temporary per-tick fields, resetting them for the next
    /// engine cycle, while preserving persistent state.
    ///
    /// **Cleared (temporary):** `masks`, `face_x`, `face_z`, `anim_id`,
    /// `anim_delay`, `say`, `damage_taken`, `damage_type`,
    /// `damage_current`, `damage_base`, `chat_bytes`, `chat_colour`,
    /// `chat_effects`, `chat_ignored`, `spotanim`, `spotanim_height`,
    /// `spotanim_delay`, `exactmove_start_x`, `exactmove_start_z`,
    /// `exactmove_end_x`, `exactmove_end_z`, `exactmove_begin`,
    /// `exactmove_finish`, `exactmove_dir`, `changetype`.
    ///
    /// **Preserved (persistent):** `appearance`, `last_appearance`,
    /// `last_appearance_info`, `readyanim`, `turnanim`, `walkanim`,
    /// `walkanim_b`, `walkanim_l`, `walkanim_r`, `runanim`,
    /// `face_entity`, `orientation_x`, `orientation_z`, `anim_protect`,
    /// `vis`.
    ///
    /// # Side Effects
    ///
    /// * All temporary fields are set to `None` or `0`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** The engine cleanup phase at the end of each tick.
    pub fn reset(&mut self) {
        self.masks = 0;
        self.face_x = None;
        self.face_z = None;
        self.anim_id = None;
        self.anim_delay = None;
        self.say = None;
        self.damage_taken = None;
        self.damage_type = None;
        self.damage_current = None;
        self.damage_base = None;
        #[cfg(since_244)]
        {
            self.damage2_taken = None;
            self.damage2_type = None;
            self.damage2_current = None;
            self.damage2_base = None;
            self.damage_slot = 0;
        }
        self.chat_bytes = None;
        self.chat_colour = None;
        self.chat_effects = None;
        self.chat_ignored = None;
        self.spotanim = None;
        self.spotanim_height = None;
        self.spotanim_delay = None;
        self.exactmove_start_x = None;
        self.exactmove_start_z = None;
        self.exactmove_end_x = None;
        self.exactmove_end_z = None;
        self.exactmove_begin = None;
        self.exactmove_finish = None;
        self.exactmove_dir = None;
        self.changetype = None;
    }

    /// Sets the face-entity target to `id` and marks the corresponding
    /// protocol mask bit, but only if the target has actually changed.
    ///
    /// This avoids sending redundant `FaceEntity` updates to the client
    /// when the entity is already facing the same target.
    ///
    /// # Arguments
    ///
    /// * `kind` - Whether this entity is an NPC or Player, used to select
    ///   the correct protocol mask bit via [`FocusKind::face_entity_mask`].
    /// * `id` - The entity index to face.
    ///
    /// # Side Effects
    ///
    /// * Sets `self.face_entity` to `Some(id)` when the value differs.
    /// * OR-s the `FaceEntity` mask bit into `self.masks` when changed.
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`EntityMasks::set_face_entity_player_check`],
    /// [`EntityMasks::set_face_entity_npc_check`].
    ///
    /// **Calls:** [`FocusKind::face_entity_mask`].
    pub fn set_face_entity_check(&mut self, kind: FocusKind, id: u16) {
        if self.face_entity != Some(id) {
            self.face_entity = Some(id);
            self.masks |= kind.face_entity_mask();
        }
    }

    /// Marks the `FaceEntity` protocol mask bit without modifying the
    /// current face-entity target.
    ///
    /// This is used when the entity must re-broadcast its existing
    /// `face_entity` value (e.g., when a new observer enters the viewport
    /// and needs the current facing state).
    ///
    /// # Arguments
    ///
    /// * `kind` - Whether this entity is an NPC or Player, used to select
    ///   the correct protocol mask bit via [`FocusKind::face_entity_mask`].
    ///
    /// # Side Effects
    ///
    /// * OR-s the `FaceEntity` mask bit into `self.masks`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`EntityMasks::face_entity_player`],
    /// [`EntityMasks::face_entity_npc`].
    ///
    /// **Calls:** [`FocusKind::face_entity_mask`].
    pub fn mark_face_entity(&mut self, kind: FocusKind) {
        self.masks |= kind.face_entity_mask();
    }

    /// Clears the face-entity target and marks the `FaceEntity` protocol
    /// mask bit so the client is notified that the entity is no longer
    /// facing anything.
    ///
    /// # Arguments
    ///
    /// * `kind` - Whether this entity is an NPC or Player, used to select
    ///   the correct protocol mask bit via [`FocusKind::face_entity_mask`].
    ///
    /// # Side Effects
    ///
    /// * Sets `self.face_entity` to `None`.
    /// * OR-s the `FaceEntity` mask bit into `self.masks`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`EntityMasks::clear_face_entity_player`],
    /// [`EntityMasks::clear_face_entity_npc`].
    ///
    /// **Calls:** [`FocusKind::face_entity_mask`].
    pub fn clear_face_entity(&mut self, kind: FocusKind) {
        self.face_entity = None;
        self.masks |= kind.face_entity_mask();
    }

    /// Sets the entity's internal orientation and, when `client` is `true`,
    /// also sends a `FaceCoord` update to observing clients.
    ///
    /// `orientation_x` / `orientation_z` are always set -- they represent
    /// the server-side facing direction used by pathing and script logic.
    /// When `client` is `true`, `face_x` / `face_z` are additionally set
    /// and the `FaceCoord` mask bit is OR-ed into `masks`, causing the
    /// info renderer to encode and transmit the coordinate update.
    ///
    /// # Arguments
    ///
    /// * `kind` - Whether this entity is an NPC or Player, used to select
    ///   the correct `FaceCoord` mask bit via [`FocusKind::face_coord_mask`].
    /// * `fine_x` - The X coordinate (in fine / sub-tile units) to face.
    /// * `fine_z` - The Z coordinate (in fine / sub-tile units) to face.
    /// * `client` - If `true`, the facing change is broadcast to clients via
    ///   the `FaceCoord` info update. If `false`, only the server-side
    ///   orientation is updated.
    ///
    /// # Side Effects
    ///
    /// * Always sets `self.orientation_x` and `self.orientation_z`.
    /// * When `client` is `true`: also sets `self.face_x`, `self.face_z`,
    ///   and OR-s the `FaceCoord` mask bit into `self.masks`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`EntityMasks::focus_player`],
    /// [`EntityMasks::focus_npc`].
    ///
    /// **Calls:** [`FocusKind::face_coord_mask`].
    pub fn focus(&mut self, kind: FocusKind, fine_x: u16, fine_z: u16, client: bool) {
        self.orientation_x = Some(fine_x);
        self.orientation_z = Some(fine_z);
        if client {
            self.face_x = Some(fine_x);
            self.face_z = Some(fine_z);
            self.masks |= kind.face_coord_mask();
        }
    }

    /// Player-specific convenience wrapper for [`EntityMasks::set_face_entity_check`].
    ///
    /// Delegates to `set_face_entity_check` with [`FocusKind::Player`].
    ///
    /// # Arguments
    ///
    /// * `id` - The entity index for the Player to face.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptPlayer` face-entity logic (rs-script).
    ///
    /// **Calls:** [`EntityMasks::set_face_entity_check`].
    pub fn set_face_entity_player_check(&mut self, id: u16) {
        self.set_face_entity_check(FocusKind::Player, id);
    }

    /// Player-specific convenience wrapper for [`EntityMasks::mark_face_entity`].
    ///
    /// Marks the Player `FaceEntity` protocol mask bit without changing
    /// the current face-entity target. Used to re-broadcast the existing
    /// facing target to newly observing clients.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptPlayer` face-entity logic (rs-script).
    ///
    /// **Calls:** [`EntityMasks::mark_face_entity`].
    pub fn face_entity_player(&mut self) {
        self.mark_face_entity(FocusKind::Player);
    }

    /// Player-specific convenience wrapper for [`EntityMasks::clear_face_entity`].
    ///
    /// Clears the Player's face-entity target and marks the `FaceEntity`
    /// mask bit so observers are notified.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptPlayer` face-entity logic (rs-script).
    ///
    /// **Calls:** [`EntityMasks::clear_face_entity`].
    pub fn clear_face_entity_player(&mut self) {
        self.clear_face_entity(FocusKind::Player);
    }

    /// NPC-specific convenience wrapper for [`EntityMasks::set_face_entity_check`].
    ///
    /// Delegates to `set_face_entity_check` with [`FocusKind::Npc`].
    ///
    /// # Arguments
    ///
    /// * `id` - The entity index for the NPC to face.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptNpc` face-entity logic (rs-script).
    ///
    /// **Calls:** [`EntityMasks::set_face_entity_check`].
    pub fn set_face_entity_npc_check(&mut self, id: u16) {
        self.set_face_entity_check(FocusKind::Npc, id);
    }

    /// NPC-specific convenience wrapper for [`EntityMasks::mark_face_entity`].
    ///
    /// Marks the NPC `FaceEntity` protocol mask bit without changing
    /// the current face-entity target. Used to re-broadcast the existing
    /// facing target to newly observing clients.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptNpc` face-entity logic (rs-script).
    ///
    /// **Calls:** [`EntityMasks::mark_face_entity`].
    pub fn face_entity_npc(&mut self) {
        self.mark_face_entity(FocusKind::Npc);
    }

    /// NPC-specific convenience wrapper for [`EntityMasks::clear_face_entity`].
    ///
    /// Clears the NPC's face-entity target and marks the `FaceEntity`
    /// mask bit so observers are notified.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptNpc` face-entity logic (rs-script).
    ///
    /// **Calls:** [`EntityMasks::clear_face_entity`].
    pub fn clear_face_entity_npc(&mut self) {
        self.clear_face_entity(FocusKind::Npc);
    }

    /// Player-specific convenience wrapper for [`EntityMasks::focus`].
    ///
    /// Sets the Player's orientation and optionally broadcasts a `FaceCoord`
    /// update to observing clients.
    ///
    /// # Arguments
    ///
    /// * `fine_x` - The X coordinate (in fine / sub-tile units) to face.
    /// * `fine_z` - The Z coordinate (in fine / sub-tile units) to face.
    /// * `client` - If `true`, the facing change is broadcast to clients.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptPlayer` focus logic (rs-script), pathing system.
    ///
    /// **Calls:** [`EntityMasks::focus`].
    pub fn focus_player(&mut self, fine_x: u16, fine_z: u16, client: bool) {
        self.focus(FocusKind::Player, fine_x, fine_z, client);
    }

    /// NPC-specific convenience wrapper for [`EntityMasks::focus`].
    ///
    /// Sets the NPC's orientation and optionally broadcasts a `FaceCoord`
    /// update to observing clients.
    ///
    /// # Arguments
    ///
    /// * `fine_x` - The X coordinate (in fine / sub-tile units) to face.
    /// * `fine_z` - The Z coordinate (in fine / sub-tile units) to face.
    /// * `client` - If `true`, the facing change is broadcast to clients.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptNpc` focus logic (rs-script), pathing system.
    ///
    /// **Calls:** [`EntityMasks::focus`].
    pub fn focus_npc(&mut self, fine_x: u16, fine_z: u16, client: bool) {
        self.focus(FocusKind::Npc, fine_x, fine_z, client);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_clone_and_eq() {
        let a = Visibility::Default;
        let b = a.clone();
        assert_eq!(a, b);
        assert_ne!(Visibility::Default, Visibility::Soft);
        assert_ne!(Visibility::Soft, Visibility::Hard);
    }

    #[test]
    fn new_all_fields_none() {
        let masks = EntityMasks::new();
        assert_eq!(masks.masks, 0);
        assert!(masks.appearance.is_none());
        assert!(masks.last_appearance.is_none());
        assert!(masks.last_appearance_info.is_none());
        assert!(masks.readyanim.is_none());
        assert!(masks.turnanim.is_none());
        assert!(masks.walkanim.is_none());
        assert!(masks.walkanim_b.is_none());
        assert!(masks.walkanim_l.is_none());
        assert!(masks.walkanim_r.is_none());
        assert!(masks.runanim.is_none());
        assert!(masks.face_entity.is_none());
        assert!(masks.face_x.is_none());
        assert!(masks.face_z.is_none());
        assert!(masks.orientation_x.is_none());
        assert!(masks.orientation_z.is_none());
        assert!(masks.anim_id.is_none());
        assert!(masks.anim_delay.is_none());
        assert!(!masks.anim_protect);
        assert!(masks.say.is_none());
        assert!(masks.damage_taken.is_none());
        assert!(masks.damage_type.is_none());
        assert!(masks.damage_current.is_none());
        assert!(masks.damage_base.is_none());
        assert!(masks.chat_bytes.is_none());
        assert!(masks.chat_colour.is_none());
        assert!(masks.chat_effects.is_none());
        assert!(masks.chat_ignored.is_none());
        assert!(masks.spotanim.is_none());
        assert!(masks.spotanim_height.is_none());
        assert!(masks.spotanim_delay.is_none());
        assert!(masks.exactmove_start_x.is_none());
        assert!(masks.exactmove_start_z.is_none());
        assert!(masks.exactmove_end_x.is_none());
        assert!(masks.exactmove_end_z.is_none());
        assert!(masks.exactmove_begin.is_none());
        assert!(masks.exactmove_finish.is_none());
        assert!(masks.exactmove_dir.is_none());
        assert!(masks.changetype.is_none());
        assert_eq!(masks.vis, Visibility::Default);
    }

    #[test]
    fn reset_clears_temporary_fields() {
        let mut masks = EntityMasks::new();
        masks.masks = 0xFFFF;
        masks.face_x = Some(100);
        masks.face_z = Some(200);
        masks.anim_id = Some(50);
        masks.anim_delay = Some(3);
        masks.say = Some("hello".into());
        masks.damage_taken = Some(10);
        masks.damage_type = Some(1);
        masks.damage_current = Some(50);
        masks.damage_base = Some(100);
        masks.chat_bytes = Some(vec![1, 2, 3].into_boxed_slice());
        masks.chat_colour = Some(5);
        masks.chat_effects = Some(2);
        masks.chat_ignored = Some(0);
        masks.spotanim = Some(300);
        masks.spotanim_height = Some(200);
        masks.spotanim_delay = Some(10);
        masks.exactmove_start_x = Some(1);
        masks.exactmove_start_z = Some(2);
        masks.exactmove_end_x = Some(3);
        masks.exactmove_end_z = Some(4);
        masks.exactmove_begin = Some(5);
        masks.exactmove_finish = Some(10);
        masks.exactmove_dir = Some(7);
        masks.changetype = Some(99);

        masks.reset();

        assert_eq!(masks.masks, 0);
        assert!(masks.face_x.is_none());
        assert!(masks.face_z.is_none());
        assert!(masks.anim_id.is_none());
        assert!(masks.anim_delay.is_none());
        assert!(masks.say.is_none());
        assert!(masks.damage_taken.is_none());
        assert!(masks.damage_type.is_none());
        assert!(masks.damage_current.is_none());
        assert!(masks.damage_base.is_none());
        assert!(masks.chat_bytes.is_none());
        assert!(masks.chat_colour.is_none());
        assert!(masks.chat_effects.is_none());
        assert!(masks.chat_ignored.is_none());
        assert!(masks.spotanim.is_none());
        assert!(masks.spotanim_height.is_none());
        assert!(masks.spotanim_delay.is_none());
        assert!(masks.exactmove_start_x.is_none());
        assert!(masks.exactmove_start_z.is_none());
        assert!(masks.exactmove_end_x.is_none());
        assert!(masks.exactmove_end_z.is_none());
        assert!(masks.exactmove_begin.is_none());
        assert!(masks.exactmove_finish.is_none());
        assert!(masks.exactmove_dir.is_none());
        assert!(masks.changetype.is_none());
    }

    #[test]
    fn reset_preserves_persistent_fields() {
        let mut masks = EntityMasks::new();
        masks.appearance = Some(42);
        masks.last_appearance = Some(12345);
        masks.last_appearance_info = Some(vec![10, 20].into_boxed_slice());
        masks.readyanim = Some(100);
        masks.turnanim = Some(101);
        masks.walkanim = Some(102);
        masks.walkanim_b = Some(103);
        masks.walkanim_l = Some(104);
        masks.walkanim_r = Some(105);
        masks.runanim = Some(106);
        masks.face_entity = Some(200);
        masks.orientation_x = Some(300);
        masks.orientation_z = Some(400);
        masks.anim_protect = true;
        masks.vis = Visibility::Hard;

        masks.reset();

        assert_eq!(masks.appearance, Some(42));
        assert_eq!(masks.last_appearance, Some(12345));
        assert!(masks.last_appearance_info.is_some());
        assert_eq!(masks.readyanim, Some(100));
        assert_eq!(masks.turnanim, Some(101));
        assert_eq!(masks.walkanim, Some(102));
        assert_eq!(masks.walkanim_b, Some(103));
        assert_eq!(masks.walkanim_l, Some(104));
        assert_eq!(masks.walkanim_r, Some(105));
        assert_eq!(masks.runanim, Some(106));
        assert_eq!(masks.face_entity, Some(200));
        assert_eq!(masks.orientation_x, Some(300));
        assert_eq!(masks.orientation_z, Some(400));
        assert!(masks.anim_protect);
        assert_eq!(masks.vis, Visibility::Hard);
    }

    #[test]
    fn set_and_read_appearance() {
        let mut masks = EntityMasks::new();
        masks.appearance = Some(1234);
        assert_eq!(masks.appearance, Some(1234));
    }

    #[test]
    fn set_and_read_damage() {
        let mut masks = EntityMasks::new();
        masks.damage_taken = Some(25);
        masks.damage_type = Some(1);
        masks.damage_current = Some(60);
        masks.damage_base = Some(99);
        assert_eq!(masks.damage_taken, Some(25));
        assert_eq!(masks.damage_type, Some(1));
        assert_eq!(masks.damage_current, Some(60));
        assert_eq!(masks.damage_base, Some(99));
    }

    #[test]
    fn set_and_read_exactmove() {
        let mut masks = EntityMasks::new();
        masks.exactmove_start_x = Some(10);
        masks.exactmove_start_z = Some(20);
        masks.exactmove_end_x = Some(30);
        masks.exactmove_end_z = Some(40);
        masks.exactmove_begin = Some(0);
        masks.exactmove_finish = Some(60);
        masks.exactmove_dir = Some(2);

        assert_eq!(masks.exactmove_start_x, Some(10));
        assert_eq!(masks.exactmove_start_z, Some(20));
        assert_eq!(masks.exactmove_end_x, Some(30));
        assert_eq!(masks.exactmove_end_z, Some(40));
        assert_eq!(masks.exactmove_begin, Some(0));
        assert_eq!(masks.exactmove_finish, Some(60));
        assert_eq!(masks.exactmove_dir, Some(2));
    }

    #[test]
    fn set_and_read_say() {
        let mut masks = EntityMasks::new();
        masks.say = Some("Hello World!".into());
        assert_eq!(masks.say.as_deref(), Some("Hello World!"));
    }

    #[test]
    fn masks_bitmask_tracks_state() {
        let mut masks = EntityMasks::new();
        masks.masks = 0b1010;
        assert_eq!(masks.masks, 0b1010);
        masks.reset();
        assert_eq!(masks.masks, 0);
    }

    #[test]
    fn multiple_reset_cycles() {
        let mut masks = EntityMasks::new();
        for _ in 0..3 {
            masks.anim_id = Some(42);
            masks.say = Some("test".into());
            masks.damage_taken = Some(5);
            masks.masks = 7;
            masks.reset();
            assert!(masks.anim_id.is_none());
            assert!(masks.say.is_none());
            assert!(masks.damage_taken.is_none());
            assert_eq!(masks.masks, 0);
        }
    }

    #[test]
    fn spotanim_fields() {
        let mut masks = EntityMasks::new();
        masks.spotanim = Some(500);
        masks.spotanim_height = Some(128);
        masks.spotanim_delay = Some(10);
        assert_eq!(masks.spotanim, Some(500));
        assert_eq!(masks.spotanim_height, Some(128));
        assert_eq!(masks.spotanim_delay, Some(10));
        masks.reset();
        assert!(masks.spotanim.is_none());
        assert!(masks.spotanim_height.is_none());
        assert!(masks.spotanim_delay.is_none());
    }

    #[test]
    fn chat_fields() {
        let mut masks = EntityMasks::new();
        masks.chat_bytes = Some(vec![0x48, 0x65, 0x6C].into_boxed_slice());
        masks.chat_colour = Some(1);
        masks.chat_effects = Some(2);
        masks.chat_ignored = Some(0);
        assert!(masks.chat_bytes.is_some());
        assert_eq!(masks.chat_colour, Some(1));
        masks.reset();
        assert!(masks.chat_bytes.is_none());
        assert!(masks.chat_colour.is_none());
    }

    #[test]
    fn visibility_variants() {
        let mut masks = EntityMasks::new();
        assert_eq!(masks.vis, Visibility::Default);
        masks.vis = Visibility::Soft;
        assert_eq!(masks.vis, Visibility::Soft);
        masks.vis = Visibility::Hard;
        assert_eq!(masks.vis, Visibility::Hard);
        masks.reset();
        assert_eq!(masks.vis, Visibility::Hard); // vis is persistent
    }

    #[test]
    fn anim_protect_persists_across_reset() {
        let mut masks = EntityMasks::new();
        masks.anim_protect = true;
        masks.reset();
        assert!(masks.anim_protect);
    }

    #[test]
    fn face_entity_persists_across_reset() {
        let mut masks = EntityMasks::new();
        masks.face_entity = Some(500);
        masks.reset();
        assert_eq!(masks.face_entity, Some(500));
    }

    #[test]
    fn all_animation_fields_persist() {
        let mut masks = EntityMasks::new();
        masks.readyanim = Some(1);
        masks.turnanim = Some(2);
        masks.walkanim = Some(3);
        masks.walkanim_b = Some(4);
        masks.walkanim_l = Some(5);
        masks.walkanim_r = Some(6);
        masks.runanim = Some(7);
        masks.reset();
        assert_eq!(masks.readyanim, Some(1));
        assert_eq!(masks.turnanim, Some(2));
        assert_eq!(masks.walkanim, Some(3));
        assert_eq!(masks.walkanim_b, Some(4));
        assert_eq!(masks.walkanim_l, Some(5));
        assert_eq!(masks.walkanim_r, Some(6));
        assert_eq!(masks.runanim, Some(7));
    }

    #[test]
    fn orientation_persists_across_reset() {
        let mut masks = EntityMasks::new();
        masks.orientation_x = Some(100);
        masks.orientation_z = Some(200);
        masks.reset();
        assert_eq!(masks.orientation_x, Some(100));
        assert_eq!(masks.orientation_z, Some(200));
    }

    #[test]
    fn temporary_vs_persistent_comprehensive() {
        let mut masks = EntityMasks::new();
        // Set everything
        masks.masks = 0xFFFF;
        masks.appearance = Some(1);
        masks.face_entity = Some(2);
        masks.orientation_x = Some(3);
        masks.readyanim = Some(4);
        masks.anim_id = Some(5);
        masks.say = Some("hi".into());
        masks.damage_taken = Some(6);
        masks.spotanim = Some(7);
        masks.exactmove_start_x = Some(8);
        masks.changetype = Some(9);

        masks.reset();

        // Persistent
        assert_eq!(masks.appearance, Some(1));
        assert_eq!(masks.face_entity, Some(2));
        assert_eq!(masks.orientation_x, Some(3));
        assert_eq!(masks.readyanim, Some(4));

        // Temporary (cleared)
        assert_eq!(masks.masks, 0);
        assert!(masks.anim_id.is_none());
        assert!(masks.say.is_none());
        assert!(masks.damage_taken.is_none());
        assert!(masks.spotanim.is_none());
        assert!(masks.exactmove_start_x.is_none());
        assert!(masks.changetype.is_none());
    }

    #[test]
    fn last_appearance_persists() {
        let mut masks = EntityMasks::new();
        masks.last_appearance = Some(99999);
        masks.last_appearance_info = Some(vec![1, 2, 3].into_boxed_slice());
        masks.reset();
        assert_eq!(masks.last_appearance, Some(99999));
        assert!(masks.last_appearance_info.is_some());
    }

    #[test]
    fn changetype_is_temporary() {
        let mut masks = EntityMasks::new();
        masks.changetype = Some(42);
        masks.reset();
        assert!(masks.changetype.is_none());
    }

    #[test]
    fn face_coord_is_temporary() {
        let mut masks = EntityMasks::new();
        masks.face_x = Some(100);
        masks.face_z = Some(200);
        masks.reset();
        assert!(masks.face_x.is_none());
        assert!(masks.face_z.is_none());
    }

    #[test]
    fn all_exactmove_fields_are_temporary() {
        let mut masks = EntityMasks::new();
        masks.exactmove_start_x = Some(1);
        masks.exactmove_start_z = Some(2);
        masks.exactmove_end_x = Some(3);
        masks.exactmove_end_z = Some(4);
        masks.exactmove_begin = Some(5);
        masks.exactmove_finish = Some(6);
        masks.exactmove_dir = Some(7);
        masks.reset();
        assert!(masks.exactmove_start_x.is_none());
        assert!(masks.exactmove_start_z.is_none());
        assert!(masks.exactmove_end_x.is_none());
        assert!(masks.exactmove_end_z.is_none());
        assert!(masks.exactmove_begin.is_none());
        assert!(masks.exactmove_finish.is_none());
        assert!(masks.exactmove_dir.is_none());
    }

    #[test]
    fn all_damage_fields_are_temporary() {
        let mut masks = EntityMasks::new();
        masks.damage_taken = Some(10);
        masks.damage_type = Some(1);
        masks.damage_current = Some(50);
        masks.damage_base = Some(99);
        masks.reset();
        assert!(masks.damage_taken.is_none());
        assert!(masks.damage_type.is_none());
        assert!(masks.damage_current.is_none());
        assert!(masks.damage_base.is_none());
    }

    #[cfg(since_244)]
    #[test]
    fn damage2_fields_and_slot_are_temporary() {
        let mut masks = EntityMasks::new();
        assert_eq!(masks.damage_slot, 0);
        masks.damage2_taken = Some(7);
        masks.damage2_type = Some(2);
        masks.damage2_current = Some(40);
        masks.damage2_base = Some(99);
        masks.damage_slot = 3;
        masks.reset();
        assert!(masks.damage2_taken.is_none());
        assert!(masks.damage2_type.is_none());
        assert!(masks.damage2_current.is_none());
        assert!(masks.damage2_base.is_none());
        assert_eq!(masks.damage_slot, 0);
    }
}
