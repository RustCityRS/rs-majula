use crate::engine::{ScriptEngine, ScriptNpc, ScriptPlayer, cache, engine, engine_mut};
use crate::register::OpsRegistry;
use crate::state::{ExecutionState, ScriptState};
use crate::util::*;
use crate::{Result, ScriptError, handlers, none};
use rs_pack::cache::ScriptVarType;
use rs_pack::cache::VarValue;
use rs_pack::cache::provider::CacheType;
use rs_pack::cache::script::*;

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
            let varp = cache()
                .varps
                .get_by_id(id)
                .ok_or(ScriptError::Runtime(format!("Varp with id: {id} not found!")))?;
            if !s.pointers.has(ScriptState::PROTECTED_ACTIVE_PLAYER[((operand >> 16) & 0x1) as usize]) && varp.protect {
                return Err(ScriptError::Runtime(format!("Varp: {:?} requires protected access!", varp.debugname())))
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
            let varn = cache()
                .varns
                .get_by_id(id)
                .ok_or(ScriptError::Runtime(format!("Varn with id: {id} not found!")))?;
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
            let vars = cache()
                .varss
                .get_by_id(id)
                .ok_or(ScriptError::Runtime(format!("Vars with id: {id} not found!")))?;
            let value = if vars.var_type == ScriptVarType::String {
                VarValue::String(s.pop_string())
            } else {
                VarValue::from_int(vars.var_type, s.pop_int())
            };
            engine_mut::<E>().set_var(id, value);
        });

        // 21
        none!(m, RETURN => |s| {
            if s.gsfsp == 0 {
                s.execution = ExecutionState::Finished;
            } else {
                s.pop_frame();
            }
        });

        // 22
        none!(m, GOSUB => |s| {
            if s.gsfsp >= 50 {
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
            let varbit = cache()
                .varbits
                .get_by_id(id)
                .ok_or(ScriptError::Runtime(format!("Varbit with id: {id} not found!")))?;
            let start = varbit.start_bit as u32;
            let width = varbit.end_bit as u32 - start + 1;
            let mask = if width >= 32 { u32::MAX } else { (1u32 << width) - 1 };
            let base = get_active_player::<E>(s, secondary)?.get_var(varbit.basevar).as_int() as u32;
            s.push_int(((base >> start) & mask) as i32);
        });

        // 27
        #[cfg(since_254)]
        none!(m, POP_VARBIT => |s| {
            let operand = s.int_operand();
            let secondary = ((operand >> 16) & 0x1) != 0;
            let id = (operand & 0xFFFF) as u16;
            let varbit = cache()
                .varbits
                .get_by_id(id)
                .ok_or(ScriptError::Runtime(format!("Varbit with id: {id} not found!")))?;
            let basevar = cache()
                .varps
                .get_by_id(varbit.basevar)
                .ok_or(ScriptError::Runtime(format!("Varp with id: {} not found!", varbit.basevar)))?;
            if !s.pointers.has(ScriptState::PROTECTED_ACTIVE_PLAYER[((operand >> 16) & 0x1) as usize]) && basevar.protect {
                return Err(ScriptError::Runtime(format!("Varbit: {:?} requires protected access!", varbit.debugname())))
            }
            let start = varbit.start_bit as u32;
            let width = varbit.end_bit as u32 - start + 1;
            let value_mask = if width >= 32 { u32::MAX } else { (1u32 << width) - 1 };
            let mut value = s.pop_int();
            if value < 0 || value as i64 > value_mask as i64 {
                value = 0;
            }
            let shifted_mask = value_mask << start;
            let player = get_active_player_mut::<E>(s, secondary)?;
            let current = player.get_var(varbit.basevar).as_int() as u32;
            let new_value = (shifted_mask & ((value as u32) << start)) | (current & !shifted_mask);
            player.set_var(varbit.basevar, VarValue::from_int(basevar.var_type, new_value as i32), basevar.transmit);
        });

        // 31
        m.insert(BRANCH_LESS_THAN_OR_EQUALS, |s| branch_if(s, |a, b| a <= b));

        // 32
        m.insert(BRANCH_GREATER_THAN_OR_EQUALS, |s| branch_if(s, |a, b| a >= b));

        // 33
        none!(m, PUSH_INT_LOCAL => |s| {
            s.push_int(s.int_locals[s.int_operand() as usize]);
        });

        // 34
        none!(m, POP_INT_LOCAL => |s| {
            let operand = s.int_operand() as usize;
            s.int_locals[operand] = s.pop_int();
        });

        // 35
        none!(m, PUSH_STRING_LOCAL => |s| {
            let idx = s.int_operand() as usize;
            s.push_string_local(idx);
        });

        // 36
        none!(m, POP_STRING_LOCAL => |s| {
            let operand = s.int_operand() as usize;
            s.string_locals[operand] = s.pop_string();
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
            if s.gsfsp >= 50 {
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
