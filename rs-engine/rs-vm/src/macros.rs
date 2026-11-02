use crate::ScriptError;
use crate::state::{LocRef, ObjRef, ScriptState};

/// Extracts the [`LocRef`] from the current script state's active location.
///
/// Reads the int operand from the script state to determine whether to use the
/// primary or secondary active location pointer.
///
/// # Arguments
/// * `state` - The current script execution state.
///
/// # Returns
/// The [`LocRef`] of the active location on success.
///
/// # Panics / Errors
/// Returns a `ScriptError::Runtime` if no active location is set for the chosen slot.
///
/// # Call Stack
/// **Called by:** `active_loc!`, `active_loc_mut!` macros.
/// **Calls:** [`ScriptState::int_operand`]
pub(crate) fn active_loc_ref(state: &ScriptState) -> crate::Result<LocRef> {
    let secondary = state.int_operand() != 0;
    if secondary {
        state.active_loc2
    } else {
        state.active_loc
    }
    .ok_or_else(|| ScriptError::Runtime("no active_loc".into()))
}

/// Extracts the [`ObjRef`] from the current script state's active ground object.
///
/// Reads the int operand from the script state to determine whether to use the
/// primary or secondary active object pointer.
///
/// # Arguments
/// * `state` - The current script execution state.
///
/// # Returns
/// The [`ObjRef`] of the active ground object on success.
///
/// # Panics / Errors
/// Returns a `ScriptError::Runtime` if no active object is set for the chosen slot.
///
/// # Call Stack
/// **Called by:** `active_obj!`, `active_obj_mut!` macros.
/// **Calls:** [`ScriptState::int_operand`]
pub(crate) fn active_obj_ref(state: &ScriptState) -> crate::Result<ObjRef> {
    let secondary = state.int_operand() != 0;
    if secondary {
        state.active_obj2
    } else {
        state.active_obj
    }
    .ok_or_else(|| ScriptError::Runtime("no active_obj".into()))
}

#[macro_export]
macro_rules! handlers {
    (|$m:ident| $($body:tt)*) => {{
        let mut $m = $crate::register::OpsRegistry::new();
        $($body)*
        $m
    }};
}

#[macro_export]
macro_rules! none {
    ($m:ident, $op:ident => |$s:ident| $body:block) => {
        $m.insert($op, |$s| {
            $body;
            Ok(())
        });
    };
}

#[macro_export]
macro_rules! active_player {
    ($m:ident, $op:ident => |$s:ident, $player:ident| $body:block) => {
        $m.insert($op, |$s| {
            $crate::util::require_active_player($s)?;
            let $player = $crate::util::get_active_player::<E>($s, $s.int_operand() != 0)?;
            $body;
            Ok(())
        });
    };
}

#[macro_export]
macro_rules! active_player_mut {
    ($m:ident, $op:ident => |$s:ident, $player:ident| $body:block) => {
        $m.insert($op, |$s| {
            $crate::util::require_active_player($s)?;
            let $player = $crate::util::get_active_player_mut::<E>($s, $s.int_operand() != 0)?;
            $body;
            Ok(())
        });
    };
}

#[macro_export]
macro_rules! active_npc {
    ($m:ident, $op:ident => |$s:ident, $npc:ident| $body:block) => {
        $m.insert($op, |$s| {
            $crate::util::require_active_npc($s)?;
            let $npc = $crate::util::get_active_npc::<E>($s, $s.int_operand() != 0)?;
            $body;
            Ok(())
        });
    };
}

#[macro_export]
macro_rules! active_npc_mut {
    ($m:ident, $op:ident => |$s:ident, $npc:ident| $body:block) => {
        $m.insert($op, |$s| {
            $crate::util::require_active_npc($s)?;
            let $npc = $crate::util::get_active_npc_mut::<E>($s, $s.int_operand() != 0)?;
            $body;
            Ok(())
        });
    };
}

#[macro_export]
macro_rules! active_loc {
    ($m:ident, $op:ident => |$s:ident, $loc:ident| $body:block) => {
        $m.insert($op, |$s| {
            $crate::util::require_active_loc($s)?;
            let $loc = $crate::macros::active_loc_ref($s)?;
            $body;
            Ok(())
        });
    };
}

#[macro_export]
macro_rules! active_loc_mut {
    ($m:ident, $op:ident => |$s:ident, $loc:ident| $body:block) => {
        $m.insert($op, |$s| {
            $crate::util::require_active_loc($s)?;
            let $loc = $crate::macros::active_loc_ref($s)?;
            $body;
            Ok(())
        });
    };
}

#[macro_export]
macro_rules! active_obj {
    ($m:ident, $op:ident => |$s:ident, $obj:ident| $body:block) => {
        $m.insert($op, |$s| {
            $crate::util::require_active_obj($s)?;
            let $obj = $crate::macros::active_obj_ref($s)?;
            $body;
            Ok(())
        });
    };
}

#[macro_export]
macro_rules! active_obj_mut {
    ($m:ident, $op:ident => |$s:ident, $obj:ident| $body:block) => {
        $m.insert($op, |$s| {
            $crate::util::require_active_obj($s)?;
            let $obj = $crate::macros::active_obj_ref($s)?;
            $body;
            Ok(())
        });
    };
}

#[macro_export]
macro_rules! protected_active_player {
    ($m:ident, $op:ident => |$s:ident, $player:ident| $body:block) => {
        $m.insert($op, |$s| {
            $crate::util::require_protected_active_player($s)?;
            let $player = $crate::util::get_active_player::<E>($s, $s.int_operand() != 0)?;
            $body;
            Ok(())
        });
    };
}

#[macro_export]
macro_rules! protected_active_player_mut {
    ($m:ident, $op:ident => |$s:ident, $player:ident| $body:block) => {
        $m.insert($op, |$s| {
            $crate::util::require_protected_active_player($s)?;
            let $player = $crate::util::get_active_player_mut::<E>($s, $s.int_operand() != 0)?;
            $body;
            Ok(())
        });
    };
}
