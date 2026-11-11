use crate::engine::{ScriptEngine, ScriptNpc, ScriptPlayer, engine, engine_mut};
#[cfg(since_647)]
use crate::ops::controller::active_controller;
use crate::register::OpsRegistry;
use crate::state::{ExecutionState, ScriptState};
use crate::util::*;
use crate::{Result, ScriptError, handlers, none};
use rs_pack::cache::ScriptVarType;
use rs_pack::cache::VarValue;
#[cfg(since_254)]
use rs_pack::cache::provider::CacheType;
use rs_pack::cache::script::*;
#[cfg(since_254)]
use rs_util::bits::{fits_bit_range, getbit_range, setbit_range_toint};

/// Registers the core virtual machine opcodes that handle stack manipulation,
/// control flow, variable access, and subroutine management.
///
/// # Opcodes Registered
///
/// Key opcodes include:
/// - **Constants:** `PUSH_CONSTANT_INT`, `PUSH_CONSTANT_STRING`
/// - **Player variables:** `PUSH_VARP`, `POP_VARP`, `PUSH_VARBIT`, `POP_VARBIT`
/// - **NPC variables:** `PUSH_VARN`, `POP_VARN`
/// - **Shared variables:** `PUSH_VARS`, `POP_VARS`
/// - **Client variables (647):** `PUSH_VARC_INT`, `POP_VARC_INT`, `PUSH_VARC_STRING`,
///   `POP_VARC_STRING`: a write transmits, a read gets what was sent last
/// - **Controller variables (647):** `PUSH_VARCON`, `POP_VARCON`, on the active controller
/// - **Clan, obj and npc-bit variables (647):** `PUSH_VARCLAN` ...
///   `PUSH_CLANSETTING_STRING` (stubs; the server keeps no owner for them yet)
/// - **Branching:** `BRANCH`, `BRANCH_NOT`, `BRANCH_EQUALS`, `BRANCH_LESS_THAN`,
///   `BRANCH_GREATER_THAN`, `BRANCH_LESS_THAN_OR_EQUALS`, `BRANCH_GREATER_THAN_OR_EQUALS`
/// - **Flow control:** `RETURN`, `GOSUB`, `JUMP`, `SWITCH`,
///   `GOSUB_WITH_PARAMS`, `JUMP_WITH_PARAMS`
/// - **Locals:** `PUSH_INT_LOCAL`, `POP_INT_LOCAL`, `PUSH_STRING_LOCAL`, `POP_STRING_LOCAL`
/// - **Stack housekeeping:** `POP_INT_DISCARD`, `POP_STRING_DISCARD`, `JOIN_STRING`
/// - **Arrays:** `DEFINE_ARRAY`, `PUSH_ARRAY_INT`, `POP_ARRAY_INT` (stubs)
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::core::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` / `none!` macros
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 0
        none!(m, PUSH_CONSTANT_INT => |s| {
            s.push_int(s.int_operand());
        });

        // 1
        none!(m, PUSH_VARP => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let varp = get_active_player::<E>(s, secondary)?.get_var(id);
            if let VarValue::String(v) = varp {
                s.push_string(&v);
            } else {
                s.push_int(varp.as_int());
            }
        });

        // 2
        none!(m, POP_VARP => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let varp = engine::<E>()
                .varps()
                .get_by_id(id)
                .ok_or(ScriptError::VarpNotFound(id as i32))?;
            if !s.pointers.has(ScriptState::PROTECTED_ACTIVE_PLAYER[secondary as usize]) && varp.protect {
                return Err(ScriptError::Runtime(format!("Varp: {} requires protected access!", varp.debugname)))
            }
            let value = if varp.var_type == ScriptVarType::String {
                VarValue::String(s.pop_string())
            } else {
                VarValue::from_int(varp.var_type, s.pop_int())
            };
            get_active_player_mut::<E>(s, secondary)?.set_var(id, value, varp.transmit);
        });

        // 3
        none!(m, PUSH_CONSTANT_STRING => |s| {
            let operand = s.string_operand() as *const str;
            s.push_string(unsafe { &*operand });
        });

        // 4
        none!(m, PUSH_VARN => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let value = get_active_npc::<E>(s, secondary)?.get_var(id);
            if let VarValue::String(v) = value {
                s.push_string(&v);
            } else {
                s.push_int(value.as_int());
            }
        });

        // 5
        none!(m, POP_VARN => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let varn = engine::<E>()
                .varns()
                .get_by_id(id)
                .ok_or(ScriptError::VarnNotFound(id as i32))?;
            let value = if varn.var_type == ScriptVarType::String {
                VarValue::String(s.pop_string())
            } else {
                VarValue::from_int(varn.var_type, s.pop_int())
            };
            get_active_npc_mut::<E>(s, secondary)?.set_var(id, value);
        });

        // 6
        none!(m, BRANCH => |s| {
            s.pc += s.int_operand();
        });

        // 7
        m.insert(BRANCH_NOT, |s| branch_if(s, |a, b| a != b));

        // 8
        m.insert(BRANCH_EQUALS, |s| branch_if(s, |a, b| a == b));

        // 9
        m.insert(BRANCH_LESS_THAN, |s| branch_if(s, |a, b| a < b));

        // 10
        m.insert(BRANCH_GREATER_THAN, |s| branch_if(s, |a, b| a > b));

        // 11
        none!(m, PUSH_VARS => |s| {
            let operand = s.int_operand();
            let id = (operand & 0xFFFF) as u16;
            let value = engine::<E>().get_var(id);
            if let VarValue::String(v) = value {
                s.push_string(&v);
            } else {
                s.push_int(value.as_int());
            }
        });

        // 12
        none!(m, POP_VARS => |s| {
            let operand = s.int_operand();
            let id = (operand & 0xFFFF) as u16;
            let vars = engine::<E>()
                .varss()
                .get_by_id(id)
                .ok_or(ScriptError::VarsNotFound(id as i32))?;
            let value = if vars.var_type == ScriptVarType::String {
                VarValue::String(s.pop_string())
            } else {
                VarValue::from_int(vars.var_type, s.pop_int())
            };
            engine_mut::<E>().set_var(id, value);
        });

        // 21
        none!(m, RETURN => |s| {
            if s.gosub_frame_stack.is_empty() {
                s.execution = ExecutionState::Finished;
            } else {
                s.pop_frame();
            }
        });

        // 22
        none!(m, GOSUB => |s| {
            if s.gosub_frame_stack.len() >= 50 {
                return Err(ScriptError::StackOverflow);
            }
            let script = pop_script::<E>(s)?;
            s.gosub_frame(script)?;
        });

        // 23
        none!(m, JUMP => |s| {
            let script = pop_script::<E>(s)?;
            s.goto_frame(script)?;
        });

        // 24
        none!(m, SWITCH => |s| {
            let key = s.pop_int();
            let off = s.script.switch_tables
                .get(s.int_operand() as usize)
                .and_then(|table| table.get(&key))
                .copied()
                .unwrap_or(0);
            s.pc += off;
        });

        // 25
        #[cfg(since_254)]
        none!(m, PUSH_VARBIT => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let varbit = engine::<E>()
                .varbits()
                .get_by_id(id)
                .ok_or(ScriptError::VarbitNotFound(id as i32))?;
            let base = get_active_player::<E>(s, secondary)?.get_var(varbit.basevar).as_int();
            s.push_int(getbit_range(base, varbit.start_bit as i32, varbit.end_bit as i32));
        });

        // 27
        #[cfg(since_254)]
        none!(m, POP_VARBIT => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let varbit = engine::<E>()
                .varbits()
                .get_by_id(id)
                .ok_or(ScriptError::VarbitNotFound(id as i32))?;
            let basevar = engine::<E>()
                .varps()
                .get_by_id(varbit.basevar)
                .ok_or(ScriptError::VarpNotFound(varbit.basevar as i32))?;
            if !s.pointers.has(ScriptState::PROTECTED_ACTIVE_PLAYER[secondary as usize]) && basevar.protect {
                return Err(ScriptError::Runtime(format!("Varbit: {} requires protected access!", varbit.debugname())))
            }
            let (start, end) = (varbit.start_bit as i32, varbit.end_bit as i32);
            let mut value = s.pop_int();
            if !fits_bit_range(value, start, end) {
                value = 0;
            }
            let player = get_active_player_mut::<E>(s, secondary)?;
            let current = player.get_var(varbit.basevar).as_int();
            let new_value = setbit_range_toint(current, value, start, end);
            player.set_var(varbit.basevar, VarValue::from_int(basevar.var_type, new_value), basevar.transmit);
        });

        // 31
        m.insert(BRANCH_LESS_THAN_OR_EQUALS, |s| branch_if(s, |a, b| a <= b));

        // 32
        m.insert(BRANCH_GREATER_THAN_OR_EQUALS, |s| branch_if(s, |a, b| a >= b));

        // 33
        none!(m, PUSH_INT_LOCAL => |s| {
            s.push_int(s.int_locals[s.int_base + s.int_operand() as usize]);
        });

        // 34
        none!(m, POP_INT_LOCAL => |s| {
            let idx = s.int_base + s.int_operand() as usize;
            s.int_locals[idx] = s.pop_int();
        });

        // 35
        none!(m, PUSH_STRING_LOCAL => |s| {
            let idx = s.int_operand() as usize;
            s.push_string_local(idx);
        });

        // 36
        none!(m, POP_STRING_LOCAL => |s| {
            let idx = s.string_base + s.int_operand() as usize;
            s.string_locals[idx] = s.pop_string();
        });

        // 37
        none!(m, JOIN_STRING => |s| {
            let count = s.int_operand() as usize;
            s.join_strings(count);
        });

        // 38
        none!(m, POP_INT_DISCARD => |s| {
            s.isp -= 1;
        });

        // 39
        none!(m, POP_STRING_DISCARD => |s| {
            s.ssp -= 1;
        });

        // 40
        none!(m, GOSUB_WITH_PARAMS => |s| {
            if s.gosub_frame_stack.len() >= 50 {
                return Err(ScriptError::StackOverflow);
            }
            let id = s.int_operand();
            let script = engine::<E>()
                .get_script(id)
                .ok_or(ScriptError::ScriptNotFound(id))?;
            s.gosub_frame(script)?;
        });

        // 41
        none!(m, JUMP_WITH_PARAMS => |s| {
            let id = s.int_operand();
            let script = engine::<E>()
                .get_script(id)
                .ok_or(ScriptError::ScriptNotFound(id))?;
            s.goto_frame(script)?;
        });

        // 42
        #[cfg(since_647)]
        none!(m, PUSH_VARC_INT => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let value = get_active_player::<E>(s, secondary)?.varc(id);
            s.push_int(value);
        });

        // 43
        #[cfg(since_647)]
        none!(m, POP_VARC_INT => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let value = s.pop_int();
            get_active_player_mut::<E>(s, secondary)?.sendvarc(id, value);
        });

        // 44
        none!(m, DEFINE_ARRAY => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 45
        none!(m, PUSH_ARRAY_INT => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 46
        none!(m, POP_ARRAY_INT => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 47
        #[cfg(since_647)]
        none!(m, PUSH_VARC_STRING => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let value = get_active_player::<E>(s, secondary)?.varc_str(id);
            s.push_string(value);
        });

        // 48
        #[cfg(since_647)]
        none!(m, POP_VARC_STRING => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let value = s.pop_string();
            get_active_player_mut::<E>(s, secondary)?.sendvarc_str(id, &value);
        });

        // 74
        #[cfg(since_647)]
        none!(m, PUSH_VARCLAN => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 75
        #[cfg(since_647)]
        none!(m, POP_VARCLAN => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 76
        #[cfg(since_647)]
        none!(m, PUSH_VAROBJ => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 77
        #[cfg(since_647)]
        none!(m, POP_VAROBJ => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 78
        #[cfg(since_647)]
        none!(m, PUSH_VARNBIT => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 79
        #[cfg(since_647)]
        none!(m, POP_VARNBIT => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 80
        #[cfg(since_647)]
        none!(m, PUSH_VARCON => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let value = engine::<E>().controller_var(active_controller(s, secondary)?, id)?;
            if let VarValue::String(v) = value {
                s.push_string(&v);
            } else {
                s.push_int(value.as_int());
            }
        });

        // 81
        #[cfg(since_647)]
        none!(m, POP_VARCON => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let varcon = engine::<E>()
                .varcons()
                .get_by_id(id)
                .ok_or_else(|| ScriptError::Runtime(format!("Varcon not found: {id}")))?;
            let value = if varcon.var_type == ScriptVarType::String {
                VarValue::String(s.pop_string())
            } else {
                VarValue::from_int(varcon.var_type, s.pop_int())
            };
            engine_mut::<E>().set_controller_var(active_controller(s, secondary)?, id, value)?;
        });

        // 82
        #[cfg(since_647)]
        none!(m, POP_CLANSETTING_INT => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 83
        #[cfg(since_647)]
        none!(m, POP_CLANSETTING_VARBIT => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 84
        #[cfg(since_647)]
        none!(m, POP_CLANSETTING_LONG => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 85
        #[cfg(since_647)]
        none!(m, POP_CLANSETTING_STRING => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        #[cfg(since_647)]
        none!(m, PUSH_VARCONBIT => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let varbit = engine::<E>()
                .varconbits()
                .get_by_id(id)
                .ok_or(ScriptError::VarConBitNotFound(id as i32))?;
            let base = engine::<E>()
                .controller_var(active_controller(s, secondary)?, varbit.basevar)?
                .as_int();
            s.push_int(getbit_range(base, varbit.start_bit as i32, varbit.end_bit as i32));
        });

        #[cfg(since_647)]
        none!(m, POP_VARCONBIT => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let varbit = engine::<E>()
                .varconbits()
                .get_by_id(id)
                .ok_or(ScriptError::VarConBitNotFound(id as i32))?;
            let varcon = engine::<E>()
                .varcons()
                .get_by_id(varbit.basevar)
                .ok_or_else(|| ScriptError::Runtime(format!("Varcon not found: {}", varbit.basevar)))?;
            let uid = active_controller(s, secondary)?;
            let base = engine::<E>().controller_var(uid, varbit.basevar)?.as_int();
            let (start, end) = (varbit.start_bit as i32, varbit.end_bit as i32);
            let value = s.pop_int();
            let value = if fits_bit_range(value, start, end) { value } else { 0 };
            let value = VarValue::from_int(varcon.var_type, setbit_range_toint(base, value, start, end));
            engine_mut::<E>().set_controller_var(uid, varbit.basevar, value)?;
        });

        // 112
        #[cfg(since_647)]
        none!(m, PUSH_CLANSETTING_INT => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 113
        #[cfg(since_647)]
        none!(m, PUSH_CLANSETTING_VARBIT => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 114
        #[cfg(since_647)]
        none!(m, PUSH_CLANSETTING_LONG => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });

        // 115
        #[cfg(since_647)]
        none!(m, PUSH_CLANSETTING_STRING => |_s| {
            Err(ScriptError::Runtime("Not implemented".to_string()))?;
        });
    }
}

/// Pops two integers from the script stack and conditionally advances the
/// program counter by the current operand if `pred(a, b)` returns `true`.
///
/// This is the shared implementation behind all conditional branch opcodes
/// (`BRANCH_NOT`, `BRANCH_EQUALS`, `BRANCH_LESS_THAN`, `BRANCH_GREATER_THAN`,
/// `BRANCH_LESS_THAN_OR_EQUALS`, `BRANCH_GREATER_THAN_OR_EQUALS`).
fn branch_if(s: &mut ScriptState, pred: fn(i32, i32) -> bool) -> Result<()> {
    let b = s.pop_int();
    let a = s.pop_int();
    if pred(a, b) {
        s.pc += s.int_operand();
    }
    Ok(())
}
