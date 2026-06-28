use crate::engine::{ScriptEngine, ScriptNpc, ScriptPlayer, cache, engine, engine_mut};
use crate::state::{LocRef, ObjRef, ScriptState};
use crate::{NpcUid, PlayerUid, Result, ScriptError};
use rs_inv::{Inventory, STACK_LIMIT, StackMode};
use rs_pack::ParamValue;
use rs_pack::cache::dbrow::DbRowType;
use rs_pack::cache::r#enum::EnumType;
use rs_pack::cache::font::FontType;
use rs_pack::cache::idk::IdkType;
use rs_pack::cache::inv::{InvScope, InvType};
use rs_pack::cache::loc::LocType;
#[cfg(rev = "225")]
use rs_pack::cache::midi::MidiType;
use rs_pack::cache::npc::NpcType;
use rs_pack::cache::obj::ObjType;
use rs_pack::cache::param::ParamType;
use rs_pack::cache::provider::CacheType;
use rs_pack::cache::script::Script;
use rs_pack::cache::seq::SeqType;
use rs_pack::cache::spotanim::SpotAnimType;
use rs_pack::cache::r#struct::StructType;
use std::sync::Arc;

pub const LOOTDROP_DURATION: u64 = (200 * 3) >> 1;

/// Enforces the protected-access rule shared by the inventory opcodes: a
/// `protect` inventory may only be mutated while the matching protected
/// active-player pointer is held, unless the inventory is shared.
///
/// `idx` selects which protected active-player pointer is required (`0` for the
/// primary active player, `1` for the secondary).
///
/// # Errors
/// Returns [`ScriptError::Runtime`] when access is not permitted.
pub(crate) fn require_inv_access(state: &ScriptState, inv: &InvType, idx: usize) -> Result<()> {
    if !state
        .pointers
        .has(ScriptState::PROTECTED_ACTIVE_PLAYER[idx])
        && inv.protect
        && inv.scope != InvScope::Shared
    {
        return Err(ScriptError::Runtime(format!(
            "Inv: {:?} requires protected access!",
            inv.debugname()
        )));
    }
    Ok(())
}

/// Drops `count` of object `id` on the ground at `coord`, splitting the way the
/// inventory and obj opcodes expect: a non-stackable object (or a lone item) is
/// dropped as individual single-count piles, while a stackable amount is dropped
/// as one combined pile.
pub(crate) fn add_obj_split<E: ScriptEngine + 'static>(
    coord: u32,
    id: u16,
    count: u32,
    stackable: bool,
    receiver37: Option<u64>,
    duration: u64,
) {
    if !stackable || count == 1 {
        for _ in 0..count {
            engine_mut::<E>().add_obj(coord, id, 1, receiver37, duration);
        }
    } else {
        engine_mut::<E>().add_obj(coord, id, count, receiver37, duration);
    }
}

/// Returns a mutable reference to the active player entity from the global engine.
///
/// # Arguments
/// * `state` - The current script execution state, which holds the active player UID.
/// * `secondary` - If `true`, uses the secondary active player (`active_player2`);
///   otherwise uses the primary (`active_player`).
///
/// # Returns
/// A mutable reference to the player implementing [`ScriptPlayer`].
///
/// # Panics / Errors
/// Returns `ScriptError::Runtime` if no active player is set for the chosen slot,
/// or if the player's slot in the engine is empty.
///
/// # Call Stack
/// **Called by:** player opcode handlers via `active_player_mut!` and utility functions.
/// **Calls:** [`engine_mut`], [`ScriptEngine::get_player_mut`]
#[allow(clippy::mut_from_ref)]
pub(crate) fn get_active_player_mut<E: ScriptEngine + 'static>(
    state: &ScriptState,
    secondary: bool,
) -> Result<&mut (impl ScriptPlayer + use<E>)> {
    let uid = if secondary {
        state.active_player2
    } else {
        state.active_player
    }
    .ok_or_else(|| {
        ScriptError::Runtime(format!(
            "no {} active_player",
            if secondary { "secondary" } else { "primary" }
        ))
    })?;
    engine_mut::<E>()
        .get_player_mut(uid.pid())
        .ok_or_else(|| ScriptError::Runtime(format!("active player slot empty: {}", uid.pid())))
}

/// Returns an immutable reference to the active player entity from the global engine.
///
/// # Arguments
/// * `state` - The current script execution state, which holds the active player UID.
/// * `secondary` - If `true`, uses the secondary active player (`active_player2`);
///   otherwise uses the primary (`active_player`).
///
/// # Returns
/// An immutable reference to the player implementing [`ScriptPlayer`].
///
/// # Panics / Errors
/// Returns `ScriptError::Runtime` if no active player is set for the chosen slot,
/// or if the player's slot in the engine is empty.
///
/// # Call Stack
/// **Called by:** player opcode handlers via `active_player!` and utility functions.
/// **Calls:** [`engine`], [`ScriptEngine::get_player`]
pub(crate) fn get_active_player<E: ScriptEngine + 'static>(
    state: &ScriptState,
    secondary: bool,
) -> Result<&(impl ScriptPlayer + use<E>)> {
    let uid = if secondary {
        state.active_player2
    } else {
        state.active_player
    }
    .ok_or_else(|| {
        ScriptError::Runtime(format!(
            "no {} active_player",
            if secondary { "secondary" } else { "primary" }
        ))
    })?;
    engine::<E>()
        .get_player(uid.pid())
        .ok_or_else(|| ScriptError::Runtime(format!("active player slot empty: {}", uid.pid())))
}

/// Sets the active player pointer on the script state and updates the pointer flags.
///
/// # Arguments
/// * `state` - The script state to update.
/// * `uid` - The [`PlayerUid`] of the player to make active.
/// * `secondary` - If `true`, sets the secondary slot (`active_player2`);
///   otherwise sets the primary (`active_player`).
///
/// # Side Effects
/// Adds the corresponding `ACTIVE_PLAYER` pointer flag to `state.pointers`.
pub(crate) fn set_active_player(state: &mut ScriptState, uid: PlayerUid, secondary: bool) {
    if secondary {
        state.active_player2 = Some(uid);
    } else {
        state.active_player = Some(uid);
    }
    state
        .pointers
        .add(ScriptState::ACTIVE_PLAYER[secondary as usize]);
}

/// Returns a mutable reference to the active NPC entity from the global engine.
///
/// # Arguments
/// * `state` - The current script execution state, which holds the active NPC UID.
/// * `secondary` - If `true`, uses the secondary active NPC (`active_npc2`);
///   otherwise uses the primary (`active_npc`).
///
/// # Returns
/// A mutable reference to the NPC implementing [`ScriptNpc`].
///
/// # Panics / Errors
/// Returns `ScriptError::Runtime` if no active NPC is set for the chosen slot,
/// or if the NPC's slot in the engine is empty.
///
/// # Call Stack
/// **Called by:** NPC opcode handlers via `active_npc_mut!` and utility functions.
/// **Calls:** [`engine_mut`], [`ScriptEngine::get_npc_mut`]
pub(crate) fn get_active_npc_mut<E: ScriptEngine + 'static>(
    state: &ScriptState,
    secondary: bool,
) -> Result<&mut (impl ScriptNpc + use<E>)> {
    let uid = if secondary {
        state.active_npc2
    } else {
        state.active_npc
    }
    .ok_or_else(|| {
        ScriptError::Runtime(format!(
            "no {} active_npc",
            if secondary { "secondary" } else { "primary" }
        ))
    })?;
    engine_mut::<E>()
        .get_npc_mut(uid.nid())
        .ok_or_else(|| ScriptError::Runtime(format!("active npc slot empty: {}", uid.nid())))
}

/// Returns an immutable reference to the active NPC entity from the global engine.
///
/// # Arguments
/// * `state` - The current script execution state, which holds the active NPC UID.
/// * `secondary` - If `true`, uses the secondary active NPC (`active_npc2`);
///   otherwise uses the primary (`active_npc`).
///
/// # Returns
/// An immutable reference to the NPC implementing [`ScriptNpc`].
///
/// # Panics / Errors
/// Returns `ScriptError::Runtime` if no active NPC is set for the chosen slot,
/// or if the NPC's slot in the engine is empty.
///
/// # Call Stack
/// **Called by:** NPC opcode handlers via `active_npc!` and utility functions.
/// **Calls:** [`engine`], [`ScriptEngine::get_npc`]
pub(crate) fn get_active_npc<E: ScriptEngine + 'static>(
    state: &ScriptState,
    secondary: bool,
) -> Result<&(impl ScriptNpc + use<E>)> {
    let uid = if secondary {
        state.active_npc2
    } else {
        state.active_npc
    }
    .ok_or_else(|| {
        ScriptError::Runtime(format!(
            "no {} active_npc",
            if secondary { "secondary" } else { "primary" }
        ))
    })?;
    engine::<E>()
        .get_npc(uid.nid())
        .ok_or_else(|| ScriptError::Runtime(format!("active npc slot empty: {}", uid.nid())))
}

/// Sets the active NPC pointer on the script state and updates the pointer flags.
///
/// # Arguments
/// * `state` - The script state to update.
/// * `uid` - The [`NpcUid`] of the NPC to make active.
/// * `secondary` - If `true`, sets the secondary slot (`active_npc2`);
///   otherwise sets the primary (`active_npc`).
///
/// # Side Effects
/// Adds the corresponding `ACTIVE_NPC` pointer flag to `state.pointers`.
pub(crate) fn set_active_npc(state: &mut ScriptState, uid: NpcUid, secondary: bool) {
    if secondary {
        state.active_npc2 = Some(uid);
    } else {
        state.active_npc = Some(uid);
    }
    state
        .pointers
        .add(ScriptState::ACTIVE_NPC[secondary as usize]);
}

/// Returns the active location reference from the script state.
///
/// # Arguments
/// * `state` - The current script execution state, which holds the active location ref.
/// * `secondary` - If `true`, uses the secondary active location (`active_loc2`);
///   otherwise uses the primary (`active_loc`).
///
/// # Returns
/// The [`LocRef`] of the active location.
///
/// # Panics / Errors
/// Returns `ScriptError::Runtime` if no active location is set for the chosen slot.
pub(crate) fn get_active_loc(state: &ScriptState, secondary: bool) -> Result<LocRef> {
    if secondary {
        state.active_loc2
    } else {
        state.active_loc
    }
    .ok_or_else(|| {
        ScriptError::Runtime(format!(
            "no {} active_loc",
            if secondary { "secondary" } else { "primary" }
        ))
    })
}

/// Sets the active location pointer on the script state and updates the pointer flags.
///
/// # Arguments
/// * `state` - The script state to update.
/// * `loc` - The [`LocRef`] to make active.
/// * `secondary` - If `true`, sets the secondary slot (`active_loc2`);
///   otherwise sets the primary (`active_loc`).
///
/// # Side Effects
/// Adds the corresponding `ACTIVE_LOC` pointer flag to `state.pointers`.
pub(crate) fn set_active_loc(state: &mut ScriptState, loc: LocRef, secondary: bool) {
    if secondary {
        state.active_loc2 = Some(loc);
    } else {
        state.active_loc = Some(loc);
    }
    state
        .pointers
        .add(ScriptState::ACTIVE_LOC[secondary as usize]);
}

/// Returns the active ground object reference from the script state.
///
/// # Arguments
/// * `state` - The current script execution state, which holds the active object ref.
/// * `secondary` - If `true`, uses the secondary active object (`active_obj2`);
///   otherwise uses the primary (`active_obj`).
///
/// # Returns
/// The [`ObjRef`] of the active ground object.
///
/// # Panics / Errors
/// Returns `ScriptError::Runtime` if no active object is set for the chosen slot.
#[allow(unused)]
pub(crate) fn get_active_obj(state: &ScriptState, secondary: bool) -> Result<ObjRef> {
    if secondary {
        state.active_obj2
    } else {
        state.active_obj
    }
    .ok_or_else(|| {
        ScriptError::Runtime(format!(
            "no {} active_obj",
            if secondary { "secondary" } else { "primary" }
        ))
    })
}

/// Sets the active ground object pointer on the script state and updates the pointer flags.
///
/// # Arguments
/// * `state` - The script state to update.
/// * `obj` - The [`ObjRef`] to make active.
/// * `secondary` - If `true`, sets the secondary slot (`active_obj2`);
///   otherwise sets the primary (`active_obj`).
///
/// # Side Effects
/// Adds the corresponding `ACTIVE_OBJ` pointer flag to `state.pointers`.
pub(crate) fn set_active_obj(state: &mut ScriptState, obj: ObjRef, secondary: bool) {
    if secondary {
        state.active_obj2 = Some(obj);
    } else {
        state.active_obj = Some(obj);
    }
    state
        .pointers
        .add(ScriptState::ACTIVE_OBJ[secondary as usize]);
}

/// Validates that the active player pointer is set for the current operand slot.
///
/// Checks the script state's pointer flags to ensure the active player (primary or
/// secondary, determined by [`ScriptState::int_operand`]) was properly established
/// before the current opcode attempts to access it.
///
/// # Arguments
/// * `state` - The current script execution state.
///
/// # Panics / Errors
/// Returns a `ScriptError` if the required active player pointer flag is not set.
///
/// # Call Stack
/// **Called by:** `active_player!`, `active_player_mut!` macros.
/// **Calls:** [`ScriptState::int_operand`], pointer flag check.
pub(crate) fn require_active_player(state: &ScriptState) -> Result<()> {
    state
        .pointers
        .check(ScriptState::ACTIVE_PLAYER[state.int_operand() as usize])
}

/// Validates that the active NPC pointer is set for the current operand slot.
///
/// # Arguments
/// * `state` - The current script execution state.
///
/// # Panics / Errors
/// Returns a `ScriptError` if the required active NPC pointer flag is not set.
///
/// # Call Stack
/// **Called by:** `active_npc!`, `active_npc_mut!` macros.
pub(crate) fn require_active_npc(state: &ScriptState) -> Result<()> {
    state
        .pointers
        .check(ScriptState::ACTIVE_NPC[state.int_operand() as usize])
}

/// Validates that the active location pointer is set for the current operand slot.
///
/// # Arguments
/// * `state` - The current script execution state.
///
/// # Panics / Errors
/// Returns a `ScriptError` if the required active location pointer flag is not set.
///
/// # Call Stack
/// **Called by:** `active_loc!`, `active_loc_mut!` macros.
pub(crate) fn require_active_loc(state: &ScriptState) -> Result<()> {
    state
        .pointers
        .check(ScriptState::ACTIVE_LOC[state.int_operand() as usize])
}

/// Validates that the active ground object pointer is set for the current operand slot.
///
/// # Arguments
/// * `state` - The current script execution state.
///
/// # Panics / Errors
/// Returns a `ScriptError` if the required active object pointer flag is not set.
///
/// # Call Stack
/// **Called by:** `active_obj!`, `active_obj_mut!` macros.
pub(crate) fn require_active_obj(state: &ScriptState) -> Result<()> {
    state
        .pointers
        .check(ScriptState::ACTIVE_OBJ[state.int_operand() as usize])
}

/// Validates that the protected active player pointer is set for the current operand slot.
///
/// Similar to [`require_active_player`] but checks the `PROTECTED_ACTIVE_PLAYER`
/// flag instead, which is a stricter pointer guard used for opcodes that require
/// protection against stale player references.
///
/// # Arguments
/// * `state` - The current script execution state.
///
/// # Panics / Errors
/// Returns a `ScriptError` if the required protected active player pointer flag is not set.
///
/// # Call Stack
/// **Called by:** `protected_active_player!`, `protected_active_player_mut!` macros.
pub(crate) fn require_protected_active_player(state: &ScriptState) -> Result<()> {
    state
        .pointers
        .check(ScriptState::PROTECTED_ACTIVE_PLAYER[state.int_operand() as usize])
}

/// Pops an integer from the script stack and validates it as a non-negative count.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped.
///
/// # Returns
/// The count as a `u32`, clamped to `[0, i32::MAX]`.
///
/// # Panics / Errors
/// Returns `ScriptError::Runtime` if the popped value is negative.
pub(crate) fn pop_count(state: &mut ScriptState) -> Result<u32> {
    let count = state.pop_int();
    if count < 0 {
        return Err(ScriptError::Runtime(format!(
            "count is out of range: {}",
            count
        )));
    }
    Ok(count.clamp(0, i32::MAX) as u32)
}

/// Pops an integer from the script stack and looks up the corresponding [`EnumType`] from the cache.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped for the enum ID.
///
/// # Returns
/// A static reference to the [`EnumType`] definition.
///
/// # Panics / Errors
/// Returns `ScriptError::EnumNotFound` if no enum exists with the popped ID.
pub(crate) fn pop_enum(state: &mut ScriptState) -> Result<&'static EnumType> {
    let id = state.pop_int();
    cache()
        .enums
        .get_by_id(id as u16)
        .ok_or(ScriptError::EnumNotFound(id))
}

/// Pops an integer from the script stack and looks up the corresponding [`DbRowType`] from the cache.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped for the database row ID.
///
/// # Returns
/// A static reference to the [`DbRowType`] definition.
///
/// # Panics / Errors
/// Returns `ScriptError::DbRowNotFound` if no database row exists with the popped ID.
pub(crate) fn pop_dbrow(state: &mut ScriptState) -> Result<&'static DbRowType> {
    let id = state.pop_int();
    cache()
        .dbrows
        .get_by_id(id as u16)
        .ok_or(ScriptError::DbRowNotFound(id))
}

/// Pops an integer from the script stack and looks up the corresponding [`FontType`] from the cache.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped for the font ID.
///
/// # Returns
/// A static reference to the [`FontType`] definition.
///
/// # Panics / Errors
/// Returns `ScriptError::FontNotFound` if no font exists with the popped ID.
pub(crate) fn pop_font(state: &mut ScriptState) -> Result<&'static FontType> {
    let id = state.pop_int();
    cache()
        .fonts
        .get_by_id(id as u16)
        .ok_or(ScriptError::FontNotFound(id))
}

/// Pops an integer from the script stack and looks up the corresponding [`InvType`] from the cache.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped for the inventory type ID.
///
/// # Returns
/// A static reference to the [`InvType`] definition.
///
/// # Panics / Errors
/// Returns `ScriptError::InvNotFound` if no inventory type exists with the popped ID.
pub(crate) fn pop_inv(state: &mut ScriptState) -> Result<&'static InvType> {
    let id = state.pop_int();
    cache()
        .invs
        .get_by_id(id as u16)
        .ok_or(ScriptError::InvNotFound(id))
}

/// Pops an integer from the script stack and looks up the corresponding [`IdkType`] from the cache.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped for the identity kit type ID.
///
/// # Returns
/// A static reference to the [`IdkType`] definition.
///
/// # Panics / Errors
/// Returns `ScriptError::IdkNotFound` if no inventory type exists with the popped ID.
pub(crate) fn pop_idk(state: &mut ScriptState) -> Result<&'static IdkType> {
    let id = state.pop_int();
    cache()
        .idks
        .get_by_id(id as u16)
        .ok_or(ScriptError::IdkNotFound(id))
}

/// Pops a string from the script stack and looks up the corresponding jingle [`MidiType`] by name.
///
/// # Arguments
/// * `state` - The script state whose string stack is popped for the jingle name.
///
/// # Returns
/// A static reference to the jingle [`MidiType`] definition.
///
/// # Panics / Errors
/// Returns `ScriptError::JingleNotFoundName` if no jingle exists with the popped name.
#[cfg(rev = "225")]
pub(crate) fn pop_jingle(state: &mut ScriptState) -> Result<&'static MidiType> {
    let name = state.pop_string();
    cache()
        .jingles
        .get_by_name(&name)
        .ok_or(ScriptError::JingleNotFoundName(name))
}

/// Pops a string from the script stack, normalizes it (lowercase, spaces to underscores),
/// and looks up the corresponding song [`MidiType`] by name.
///
/// # Arguments
/// * `state` - The script state whose string stack is popped for the song name.
///
/// # Returns
/// A static reference to the song [`MidiType`] definition.
///
/// # Safety
/// Uses `unsafe` to mutate the string bytes in-place for performance. This is safe
/// because the string is owned and the transformation (ASCII lowercase + space-to-underscore)
/// preserves valid UTF-8.
///
/// # Panics / Errors
/// Returns `ScriptError::SongNotFoundName` if no song exists with the normalized name.
#[cfg(rev = "225")]
pub(crate) fn pop_song(state: &mut ScriptState) -> Result<&'static MidiType> {
    let mut name = state.pop_string();
    unsafe {
        for b in name.as_bytes_mut() {
            if *b == b' ' {
                *b = b'_';
            }
            b.make_ascii_lowercase();
        }
    }
    cache()
        .songs
        .get_by_name(&name)
        .ok_or(ScriptError::SongNotFoundName(name))
}

#[cfg(all(since_244, before_254))]
fn normalize_song_name(name: &str) -> String {
    name.chars()
        .map(|c| c.to_ascii_lowercase())
        .map(|c| if c == ' ' { '_' } else { c })
        .filter(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '-'))
        .collect()
}

#[cfg(all(since_244, before_254))]
pub(crate) fn song_midi_id(name: &str) -> Option<u16> {
    cache()
        .midi_ids
        .get(normalize_song_name(name).as_str())
        .copied()
}

#[cfg(since_244)]
pub(crate) fn jingle_midi_id(name: &str) -> Option<u16> {
    cache()
        .midi_ids
        .get(name.to_ascii_lowercase().as_str())
        .copied()
}

/// Pops an integer from the script stack and looks up the corresponding [`NpcType`] from the cache.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped for the NPC type ID.
///
/// # Returns
/// A static reference to the [`NpcType`] definition.
///
/// # Panics / Errors
/// Returns `ScriptError::NpcNotFound` if no NPC type exists with the popped ID.
pub(crate) fn pop_npc(state: &mut ScriptState) -> Result<&'static NpcType> {
    let id = state.pop_int();
    cache()
        .npcs
        .get_by_id(id as u16)
        .ok_or(ScriptError::NpcNotFound(id))
}

/// Pops an integer from the script stack and looks up the corresponding [`ObjType`] from the cache.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped for the object/item type ID.
///
/// # Returns
/// A static reference to the [`ObjType`] definition.
///
/// # Panics / Errors
/// Returns `ScriptError::ObjNotFound` if no object type exists with the popped ID.
pub(crate) fn pop_obj(state: &mut ScriptState) -> Result<&'static ObjType> {
    let id = state.pop_int();
    cache()
        .objs
        .get_by_id(id as u16)
        .ok_or(ScriptError::ObjNotFound(id))
}

/// Pops an integer from the script stack and looks up the corresponding [`LocType`] from the cache.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped for the location/loc type ID.
///
/// # Returns
/// A static reference to the [`LocType`] definition.
///
/// # Panics / Errors
/// Returns `ScriptError::ObjNotFound` if no object type exists with the popped ID.
pub(crate) fn pop_loc(state: &mut ScriptState) -> Result<&'static LocType> {
    let id = state.pop_int();
    cache()
        .locs
        .get_by_id(id as u16)
        .ok_or(ScriptError::ObjNotFound(id))
}

/// Pops an integer from the script stack and looks up the corresponding [`ParamType`] from the cache.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped for the param type ID.
///
/// # Returns
/// A static reference to the [`ParamType`] definition.
///
/// # Panics / Errors
/// Returns `ScriptError::ParamNotFound` if no param type exists with the popped ID.
pub(crate) fn pop_param(state: &mut ScriptState) -> Result<&'static ParamType> {
    let id = state.pop_int();
    cache()
        .params
        .get_by_id(id as u16)
        .ok_or(ScriptError::ParamNotFound(id))
}

/// Pops an integer from the script stack and looks up the corresponding [`Script`] from the engine.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped for the script ID.
///
/// # Returns
/// A static reference to the [`Arc<Script>`] definition.
///
/// # Panics / Errors
/// Returns `ScriptError::ScriptNotFound` if no script exists with the popped ID.
///
/// # Call Stack
/// **Calls:** [`ScriptEngine::get_script`] via the global engine accessor.
pub(crate) fn pop_script<E: ScriptEngine + 'static>(
    state: &mut ScriptState,
) -> Result<&'static Arc<Script>> {
    let id = state.pop_int();
    engine::<E>()
        .get_script(id)
        .ok_or(ScriptError::ScriptNotFound(id))
}

/// Pops an integer from the script stack and looks up the corresponding [`SeqType`] from the cache.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped for the sequence/animation ID.
///
/// # Returns
/// A static reference to the [`SeqType`] (animation sequence) definition.
///
/// # Panics / Errors
/// Returns `ScriptError::SeqNotFound` if no sequence exists with the popped ID.
pub(crate) fn pop_seq(state: &mut ScriptState) -> Result<&'static SeqType> {
    let id = state.pop_int();
    cache()
        .seqs
        .get_by_id(id as u16)
        .ok_or(ScriptError::SeqNotFound(id))
}

/// Pops an integer from the script stack and looks up the corresponding [`SpotAnimType`] from the cache.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped for the spot animation ID.
///
/// # Returns
/// A static reference to the [`SpotAnimType`] (graphical effect) definition.
///
/// # Panics / Errors
/// Returns `ScriptError::SpotanimNotFound` if no spot animation exists with the popped ID.
pub(crate) fn pop_spotanim(state: &mut ScriptState) -> Result<&'static SpotAnimType> {
    let id = state.pop_int();
    cache()
        .spotanims
        .get_by_id(id as u16)
        .ok_or(ScriptError::SpotanimNotFound(id))
}

/// Pops an integer from the script stack and looks up the corresponding [`StructType`] from the cache.
///
/// # Arguments
/// * `state` - The script state whose integer stack is popped for the struct type ID.
///
/// # Returns
/// A static reference to the [`StructType`] definition (a param-keyed data container).
///
/// # Panics / Errors
/// Returns `ScriptError::StructNotFound` if no struct exists with the popped ID.
pub(crate) fn pop_struct(state: &mut ScriptState) -> Result<&'static StructType> {
    let id = state.pop_int();
    cache()
        .structs
        .get_by_id(id as u16)
        .ok_or(ScriptError::StructNotFound(id))
}

// ── Inventory helpers ───────────────────────────────────────────────

/// Determines the [`StackMode`] for an inventory based on its configuration.
///
/// # Arguments
/// * `inv` - The inventory type definition from the cache.
///
/// # Returns
/// [`StackMode::Always`] if `inv.stackall` is `true`, otherwise [`StackMode::Normal`]
/// (items stack only if their `ObjType` is marked as stackable).
fn stackmode(inv: &InvType) -> StackMode {
    if inv.stackall {
        StackMode::Always
    } else {
        StackMode::Normal
    }
}

/// Retrieves an immutable reference to an inventory, creating it if it does not yet exist.
///
/// Routes to the shared engine inventory for `InvScope::Shared`, or to the player's
/// personal inventory for `Temp`/`Perm` scopes.
///
/// # Arguments
/// * `inv` - The inventory type definition (determines scope, size, and stack mode).
/// * `player` - The player whose personal inventories to access (unused for shared scope).
///
/// # Returns
/// An immutable reference to the [`Inventory`].
///
/// # Call Stack
/// **Calls:** [`stackmode`], [`engine_mut`] (for shared), [`ScriptPlayer::get_or_create_inv`].
pub(crate) fn get_inv<'a, E: ScriptEngine + 'static>(
    inv: &InvType,
    player: &'a mut impl ScriptPlayer,
) -> Result<&'a Inventory> {
    let stackmode = stackmode(inv);
    let size = inv.size as usize;
    match inv.scope {
        InvScope::Shared => Ok(engine_mut::<E>().get_shared_inv(inv.id, size, stackmode)),
        InvScope::Temp | InvScope::Perm => Ok(player.get_or_create_inv(inv.id, size, stackmode)),
    }
}

/// Retrieves a mutable reference to an inventory by ID, creating it if it does not yet exist.
///
/// Looks up the [`InvType`] from the cache, then routes to the shared engine inventory
/// or the player's personal inventory depending on scope.
///
/// # Arguments
/// * `inv_id` - The inventory type ID to look up in the cache.
/// * `player` - The player whose personal inventories to access (unused for shared scope).
///
/// # Returns
/// A mutable reference to the [`Inventory`].
///
/// # Panics / Errors
/// Returns `ScriptError::InvNotFound` if the inventory type ID is not in the cache.
///
/// # Call Stack
/// **Calls:** [`cache`], [`stackmode`], [`engine_mut`] (for shared), [`ScriptPlayer::get_or_create_inv`].
pub(crate) fn get_inv_mut<E: ScriptEngine + 'static>(
    inv_id: u16,
    player: &mut impl ScriptPlayer,
) -> Result<&mut Inventory> {
    let inv = cache()
        .invs
        .get_by_id(inv_id)
        .ok_or(ScriptError::InvNotFound(inv_id as i32))?;
    let stackmode = stackmode(inv);
    let size = inv.size as usize;
    match inv.scope {
        InvScope::Shared => Ok(engine_mut::<E>().get_shared_inv(inv.id, size, stackmode)),
        InvScope::Temp | InvScope::Perm => Ok(player.get_or_create_inv(inv.id, size, stackmode)),
    }
}

/// Retrieves mutable references to two distinct inventories simultaneously.
///
/// Both inventories are ensured to exist before borrowing them as a pair. This avoids
/// double-mutable-borrow issues when an opcode needs to transfer items between two
/// inventories.
///
/// # Arguments
/// * `inv_a` - The first inventory type ID.
/// * `inv_b` - The second inventory type ID (must differ from `inv_a`).
/// * `player` - The player whose personal inventories to access.
///
/// # Returns
/// A tuple of mutable references `(&mut Inventory, &mut Inventory)`.
///
/// # Panics / Errors
/// Returns `ScriptError::InvNotFound` if either inventory type ID is not in the cache
/// or if the pair borrow fails.
///
/// # Call Stack
/// **Calls:** [`cache`], [`stackmode`], [`ScriptPlayer::get_or_create_inv`],
/// [`ScriptPlayer::get_inv_pair_mut`].
pub(crate) fn get_inv_pair_mut(
    inv_a: u16,
    inv_b: u16,
    player: &mut impl ScriptPlayer,
) -> Result<(&mut Inventory, &mut Inventory)> {
    let cache = cache();
    let a = cache
        .invs
        .get_by_id(inv_a)
        .ok_or(ScriptError::InvNotFound(inv_a as i32))?;
    let b = cache
        .invs
        .get_by_id(inv_b)
        .ok_or(ScriptError::InvNotFound(inv_b as i32))?;
    let sa = stackmode(a);
    let sb = stackmode(b);
    // ensure both exist
    player.get_or_create_inv(a.id, a.size as usize, sa);
    player.get_or_create_inv(b.id, b.size as usize, sb);
    player
        .get_inv_pair_mut(inv_a, inv_b)
        .ok_or(ScriptError::InvNotFound(inv_a as i32))
}

/// Calculates how many items of the given type cannot fit into an inventory.
///
/// For stackable items (or inventories with `stackall`), computes the overflow based
/// on the `STACK_LIMIT` and the current total of that item. For non-stackable items,
/// computes overflow based on free slot count.
///
/// # Arguments
/// * `inv` - The inventory type definition.
/// * `obj` - The object/item type definition (checked for stackability and cert status).
/// * `inventory` - The current inventory state.
/// * `count` - The number of items attempting to be added.
/// * `size` - The total intended inventory size (may differ from `inv.size` in some contexts).
///
/// # Returns
/// The number of items that would not fit (overflow). Returns `0` if all items fit.
///
/// # Call Stack
/// **Calls:** [`uncert`], [`Inventory::total`], [`Inventory::freespace`].
pub(crate) fn inv_itemspace(
    inv: &InvType,
    obj: &ObjType,
    inventory: &Inventory,
    count: i32,
    size: i32,
) -> i32 {
    let uncert_id = uncert(obj);

    if obj.stackable || uncert_id != obj.id || inv.stackall {
        let stock_obj = inv.stockobj.as_ref().is_some_and(|s| s.contains(&obj.id));
        let total = inventory.total(obj.id) as i32;
        if total == 0 && inventory.freespace() == 0 && !stock_obj {
            return count;
        }
        return (count - (STACK_LIMIT as i32 - total)).max(0);
    }

    (count - (inventory.freespace() as i32 - (inv.size as i32 - size))).max(0)
}

/// Returns the un-certificated (noted) item ID for the given object type.
///
/// If the object has a `certtemplate` (meaning it is a certificate/noted form) and a
/// `certlink`, returns the linked real item ID. Otherwise returns the object's own ID.
///
/// # Arguments
/// * `obj` - The object/item type definition.
///
/// # Returns
/// The un-certificated item ID, or the object's own ID if it is not a certificate.
pub(crate) fn uncert(obj: &ObjType) -> u16 {
    if obj.certtemplate.is_some()
        && let Some(certlink) = obj.certlink
    {
        certlink
    } else {
        obj.id
    }
}

/// Returns the certificated (noted) item ID for the given object type.
///
/// If the object does not have a `certtemplate` (meaning it is the real/un-noted form)
/// and has a `certlink`, returns the linked certificate item ID. Otherwise returns the
/// object's own ID.
///
/// # Arguments
/// * `obj` - The object/item type definition.
///
/// # Returns
/// The certificated item ID, or the object's own ID if no certificate form exists.
pub(crate) fn cert(obj: &ObjType) -> u16 {
    if obj.certtemplate.is_none()
        && let Some(certlink) = obj.certlink
    {
        certlink
    } else {
        obj.id
    }
}

/// Sums a parameter value across all items in an inventory.
///
/// For each occupied slot, looks up the item's param value (falling back to the param's
/// default if not defined on the item). If `stack` is `true`, the param value is
/// multiplied by the item's stack count before accumulating. Uses wrapping arithmetic
/// to match the original engine behavior.
///
/// # Arguments
/// * `inv` - The inventory type definition.
/// * `param` - The parameter type whose values to sum.
/// * `stack` - If `true`, multiplies each param value by the item's stack count.
/// * `player` - The player whose inventory to read.
///
/// # Returns
/// The total (wrapping) sum of the param values across all occupied inventory slots.
///
/// # Call Stack
/// **Calls:** [`get_inv`], [`cache`] for `ObjType` lookups.
pub(crate) fn inv_total_param<E: ScriptEngine + 'static>(
    inv: &InvType,
    param: &ParamType,
    stack: bool,
    player: &mut impl ScriptPlayer,
) -> Result<i32> {
    let inventory = get_inv::<E>(inv, player)?;
    let c = cache();
    let mut total: i32 = 0;
    for slot in &inventory.slots {
        let Some(item) = slot else { continue };
        let Some(obj_type) = c.objs.get_by_id(item.obj) else {
            continue;
        };
        let value = obj_type
            .params
            .as_ref()
            .and_then(|p| p.get(&(param.id as i32)))
            .map(|v| match v {
                ParamValue::Int(i) => *i,
                _ => param.default_int,
            })
            .unwrap_or(param.default_int);
        if stack {
            total = total.wrapping_add((item.num as i32).wrapping_mul(value));
        } else {
            total = total.wrapping_add(value);
        }
    }
    Ok(total)
}
