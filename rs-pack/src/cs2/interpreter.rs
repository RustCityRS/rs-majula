//! Stack-machine interpreter — transforms raw CS2 bytecode into IR.
//!
//! Simulates the CS2 stack machine instruction-by-instruction, producing
//! a flat list of `Instruction` nodes suitable for control-flow analysis.

use crate::cs2::ScriptMeta;
use crate::cs2::ir::*;
use crate::cs2::opcodes::*;
use crate::cs2::script::Script;
use std::collections::{BTreeSet, HashMap};
use std::mem::take;

/// Interpret a decoded `Script` into a flat IR `Function`.
pub fn interpret(script: &Script, script_id: u32) -> Function {
    interpret_with_params(script, script_id, None)
}

/// Interpret with optional script metadata for resolving proc call args and returns.
pub fn interpret_with_params(
    script: &Script,
    script_id: u32,
    script_meta: Option<&HashMap<u32, ScriptMeta>>,
) -> Function {
    let mut int_stack: Vec<Expr> = Vec::new();
    let mut string_stack: Vec<Expr> = Vec::new();
    let mut instructions: Vec<Instruction> = Vec::new();

    // Collect all branch/switch target PCs so we can insert labels.
    let mut label_pcs: BTreeSet<u32> = BTreeSet::new();
    for (pc, &opcode) in script.opcodes.iter().enumerate() {
        if is_branch(opcode) {
            let target = pc as i32 + script.int_operands[pc] + 1;
            if target >= 0 {
                label_pcs.insert(target as u32);
            }
        }
        if opcode == SWITCH {
            let table_idx = script.int_operands[pc] as usize;
            if let Some(table) = script.switches.get(table_idx) {
                for &(_key, offset) in table {
                    let target = pc as i32 + offset + 1;
                    if target >= 0 {
                        label_pcs.insert(target as u32);
                    }
                }
            }
        }
    }

    let num_instructions = script.opcodes.len();

    for pc in 0..num_instructions {
        // Insert label if this PC is a branch target.
        // Clear stacks at labels — different incoming paths may leave
        // different values, and linear processing would accumulate them.
        if label_pcs.contains(&(pc as u32)) {
            int_stack.clear();
            string_stack.clear();
            instructions.push(Instruction::Label(pc as u32));
        }

        let opcode = script.opcodes[pc];
        let int_operand = script.int_operands[pc];

        match opcode {
            // --- Constants ---
            PUSH_CONSTANT_INT => {
                int_stack.push(Expr::IntConstant(int_operand));
            }
            PUSH_CONSTANT_STRING => {
                let s = script.string_operands[pc].clone().unwrap_or_default();
                string_stack.push(Expr::StringConstant(s));
            }
            PUSH_CONSTANT_LONG => {
                let v = script.long_operands[pc].unwrap_or(0);
                int_stack.push(Expr::LongConstant(v));
            }

            // --- Local variable access ---
            PUSH_INT_LOCAL => {
                int_stack.push(Expr::Var(Variable::LocalInt(int_operand as u32)));
            }
            POP_INT_LOCAL => {
                let val = pop_int(&mut int_stack);
                instructions.push(Instruction::Assignment {
                    targets: vec![Variable::LocalInt(int_operand as u32)],
                    expr: val,
                });
            }
            PUSH_STRING_LOCAL => {
                string_stack.push(Expr::Var(Variable::LocalString(int_operand as u32)));
            }
            POP_STRING_LOCAL => {
                let val = pop_string(&mut string_stack);
                instructions.push(Instruction::Assignment {
                    targets: vec![Variable::LocalString(int_operand as u32)],
                    expr: val,
                });
            }
            PUSH_LONG_LOCAL => {
                int_stack.push(Expr::Var(Variable::LocalLong(int_operand as u32)));
            }
            POP_LONG_LOCAL => {
                let val = pop_int(&mut int_stack);
                instructions.push(Instruction::Assignment {
                    targets: vec![Variable::LocalLong(int_operand as u32)],
                    expr: val,
                });
            }

            // --- Varp / varbit / varc ---
            PUSH_VAR => {
                int_stack.push(Expr::Var(Variable::Varp(int_operand as u32)));
            }
            POP_VAR => {
                let val = pop_int(&mut int_stack);
                instructions.push(Instruction::Assignment {
                    targets: vec![Variable::Varp(int_operand as u32)],
                    expr: val,
                });
            }
            PUSH_VARBIT => {
                int_stack.push(Expr::Var(Variable::Varbit(int_operand as u32)));
            }
            POP_VARBIT => {
                let val = pop_int(&mut int_stack);
                instructions.push(Instruction::Assignment {
                    targets: vec![Variable::Varbit(int_operand as u32)],
                    expr: val,
                });
            }
            PUSH_VARC_INT => {
                int_stack.push(Expr::Var(Variable::VarcInt(int_operand as u32)));
            }
            POP_VARC_INT => {
                let val = pop_int(&mut int_stack);
                instructions.push(Instruction::Assignment {
                    targets: vec![Variable::VarcInt(int_operand as u32)],
                    expr: val,
                });
            }
            PUSH_VARC_STRING_OLD => {
                string_stack.push(Expr::Var(Variable::VarcString(int_operand as u32)));
            }
            POP_VARC_STRING_OLD => {
                let val = pop_string(&mut string_stack);
                instructions.push(Instruction::Assignment {
                    targets: vec![Variable::VarcString(int_operand as u32)],
                    expr: val,
                });
            }

            // --- Arrays ---
            DEFINE_ARRAY => {
                let size = pop_int(&mut int_stack);
                instructions.push(Instruction::Expression(Expr::Operation {
                    opcode,
                    args: vec![Expr::IntConstant(int_operand), size],
                }));
            }
            PUSH_ARRAY_INT => {
                let index = pop_int(&mut int_stack);
                int_stack.push(Expr::Operation {
                    opcode,
                    args: vec![Expr::IntConstant(int_operand), index],
                });
            }
            POP_ARRAY_INT => {
                let val = pop_int(&mut int_stack);
                let index = pop_int(&mut int_stack);
                instructions.push(Instruction::Assignment {
                    targets: vec![Variable::Array(int_operand as u32)],
                    expr: Expr::Operation {
                        opcode,
                        args: vec![index, val],
                    },
                });
            }

            // --- Discard ---
            POP_INT_DISCARD | POP_LONG_DISCARD => {
                let val = pop_int(&mut int_stack);
                instructions.push(Instruction::Expression(val));
            }
            POP_STRING_DISCARD => {
                let val = pop_string(&mut string_stack);
                instructions.push(Instruction::Expression(val));
            }

            // --- Branches ---
            BRANCH => {
                let target = pc as i32 + int_operand + 1;
                instructions.push(Instruction::Goto(target as u32));
            }
            BRANCH_NOT
            | BRANCH_EQUALS
            | BRANCH_LESS_THAN
            | BRANCH_GREATER_THAN
            | BRANCH_LESS_THAN_OR_EQUALS
            | BRANCH_GREATER_THAN_OR_EQUALS => {
                let rhs = pop_int(&mut int_stack);
                let lhs = pop_int(&mut int_stack);
                let cond = Expr::Operation {
                    opcode,
                    args: vec![lhs, rhs],
                };
                let target = pc as i32 + int_operand + 1;
                instructions.push(Instruction::Branch {
                    condition: cond,
                    target: target as u32,
                });
            }
            BRANCH_NOT_LONG
            | BRANCH_EQUALS_LONG
            | BRANCH_LESS_THAN_LONG
            | BRANCH_GREATER_THAN_LONG
            | BRANCH_LESS_THAN_OR_EQUALS_LONG
            | BRANCH_GREATER_THAN_OR_EQUALS_LONG => {
                let rhs = pop_int(&mut int_stack);
                let lhs = pop_int(&mut int_stack);
                let cond = Expr::Operation {
                    opcode,
                    args: vec![lhs, rhs],
                };
                let target = pc as i32 + int_operand + 1;
                instructions.push(Instruction::Branch {
                    condition: cond,
                    target: target as u32,
                });
            }
            BRANCH_IF_TRUE => {
                let val = pop_int(&mut int_stack);
                let cond = Expr::Operation {
                    opcode: BRANCH_EQUALS,
                    args: vec![val, Expr::IntConstant(1)],
                };
                let target = pc as i32 + int_operand + 1;
                instructions.push(Instruction::Branch {
                    condition: cond,
                    target: target as u32,
                });
            }
            BRANCH_IF_FALSE => {
                let val = pop_int(&mut int_stack);
                let cond = Expr::Operation {
                    opcode: BRANCH_EQUALS,
                    args: vec![val, Expr::IntConstant(0)],
                };
                let target = pc as i32 + int_operand + 1;
                instructions.push(Instruction::Branch {
                    condition: cond,
                    target: target as u32,
                });
            }

            // --- Switch ---
            SWITCH => {
                let val = pop_int(&mut int_stack);
                let table_idx = int_operand as usize;
                let mut cases = Vec::new();
                if let Some(table) = script.switches.get(table_idx) {
                    for &(key, offset) in table {
                        let target = pc as i32 + offset + 1;
                        cases.push((key, target as u32));
                    }
                }
                instructions.push(Instruction::Switch { expr: val, cases });
            }

            // --- Return ---
            RETURN => {
                let int_values: Vec<Expr> = take(&mut int_stack);
                let string_values: Vec<Expr> = take(&mut string_stack);
                instructions.push(Instruction::Return {
                    int_values,
                    string_values,
                });
            }

            // --- Gosub (procedure call) ---
            GOSUB_WITH_PARAMS => {
                let target_id = int_operand as u32;
                let mut int_args = Vec::new();
                let mut str_args = Vec::new();
                let mut int_returns = 0u16;
                let mut string_returns = 0u16;

                if let Some(meta_map) = script_meta {
                    if let Some(meta) = meta_map.get(&target_id) {
                        for _ in 0..meta.long_params {
                            int_args.push(pop_int(&mut int_stack));
                        }
                        for _ in 0..meta.string_params {
                            str_args.push(pop_string(&mut string_stack));
                        }
                        for _ in 0..meta.int_params {
                            int_args.push(pop_int(&mut int_stack));
                        }
                        int_returns = meta.int_returns;
                        string_returns = meta.string_returns;
                    }
                }

                int_args.reverse();
                str_args.reverse();
                let mut args = int_args;
                args.append(&mut str_args);

                let proc_expr = Expr::Proc {
                    script_id: int_operand,
                    args,
                };

                if int_returns == 0 && string_returns == 0 {
                    instructions.push(Instruction::Expression(proc_expr));
                } else {
                    // Push return value placeholders onto stacks so
                    // subsequent pops (e.g., branch conditions) get the
                    // proc call expression instead of a stale/default value.
                    if int_returns == 1 && string_returns == 0 {
                        int_stack.push(proc_expr);
                    } else if int_returns == 0 && string_returns == 1 {
                        string_stack.push(proc_expr);
                    } else {
                        // Multi-return: push placeholders only (no
                        // separate Expression — the return captures them).
                        for _ in 0..int_returns {
                            int_stack.push(proc_expr.clone());
                        }
                        for _ in 0..string_returns {
                            string_stack.push(proc_expr.clone());
                        }
                    }
                }
            }

            // --- String concatenation ---
            JOIN_STRING => {
                let count = int_operand as usize;
                let mut parts = Vec::with_capacity(count);
                for _ in 0..count {
                    if !string_stack.is_empty() {
                        parts.push(pop_string(&mut string_stack));
                    }
                }
                parts.reverse();
                string_stack.push(Expr::StringConcat(parts));
            }

            // --- Arithmetic / logic (2-int → 1-int) ---
            ADD | SUB | MULTIPLY | DIV | MODULO | AND | OR | SETBIT | CLEARBIT | TESTBIT
            | ADDPERCENT | POW | INVPOW | MIN | MAX => {
                let rhs = pop_int(&mut int_stack);
                let lhs = pop_int(&mut int_stack);
                int_stack.push(Expr::Operation {
                    opcode,
                    args: vec![lhs, rhs],
                });
            }

            // scale(a, b, c) — 3-int → 1-int
            SCALE | INTERPOLATE => {
                let info = opcode_info(opcode);
                let mut args = Vec::new();
                for _ in 0..info.pops_int {
                    args.push(pop_int(&mut int_stack));
                }
                args.reverse();
                int_stack.push(Expr::Operation { opcode, args });
            }

            // 1-int → 1-int
            RANDOM | RANDOMINC | STAT | STAT_BASE | STAT_XP | INV_SIZE | ENUM_GETOUTPUTCOUNT
            | FRIEND_GETWORLD | OC_COST | OC_STACKABLE | OC_CERT | OC_UNCERT | OC_MEMBERS
            | OC_WEIGHT | OC_WEARPOS | OC_WEARPOS2 | CHAR_ISPRINTABLE | CHAR_ISALPHANUMERIC
            | CHAR_ISALPHA | CHAR_ISNUMERIC | COORDX | COORDY | COORDZ | IF_GETCHILDCOUNT
            | IF_GETTRANS2 | IF_GETGRAPHIC => {
                let a = pop_int(&mut int_stack);
                int_stack.push(Expr::Operation {
                    opcode,
                    args: vec![a],
                });
            }

            // 0-int → 1-int (getters)
            CLIENTCLOCK | COORD | MAP_MEMBERS | STAFFMODLEVEL | REBOOTTIMER | FRIEND_COUNT
            | GETWINDOWMODE | WORLDLIST_FETCH | WORLDLIST_START | WORLDLIST_NEXT | ON_MOBILE
            | CLIENTTYPE | WORLDID | CC_GETX | CC_GETY | CC_GETWIDTH | CC_GETHEIGHT
            | CC_GETHIDE | CC_GETLAYER | CC_GETSCROLLX | CC_GETSCROLLY | CC_GETINVOBJECT
            | CC_GETINVCOUNT | CC_GETID | CC_GETSCROLLWIDTH | CC_GETSCROLLHEIGHT
            | CC_GETMODELZOOM | CC_GETMODELANGLE_X | CC_GETMODELANGLE_Z | CC_GETMODELANGLE_Y
            | CC_GETTRANS | IF_GETX | IF_GETY | IF_GETWIDTH | IF_GETHEIGHT | IF_GETHIDE
            | IF_GETSCROLLX | IF_GETSCROLLY | IF_GETSCROLLWIDTH | IF_GETSCROLLHEIGHT
            | IF_GETINVOBJECT | IF_GETINVCOUNT | PLAYER_ISFEMALE | PUSH_CLANSETTING_VAR => {
                int_stack.push(Expr::Operation {
                    opcode,
                    args: vec![],
                });
            }

            // 0 → 1-string
            CC_GETTEXT | IF_GETTEXT | CHAT_PLAYERNAME => {
                string_stack.push(Expr::Operation {
                    opcode,
                    args: vec![],
                });
            }

            // 1-int → 1-string
            OC_NAME | FRIEND_GETNAME | TOSTRING | PLAYER_GETDISPLAYNAME => {
                let a = pop_int(&mut int_stack);
                string_stack.push(Expr::Operation {
                    opcode,
                    args: vec![a],
                });
            }

            // 2-int → 1-string
            OC_OP | OC_IOP => {
                let b = pop_int(&mut int_stack);
                let a = pop_int(&mut int_stack);
                string_stack.push(Expr::Operation {
                    opcode,
                    args: vec![a, b],
                });
            }

            // 2-int → 1-int (lookups)
            INV_GETOBJ | INV_GETNUM | INV_TOTAL | NC_PARAM | LC_PARAM | OC_PARAM | STRUCT_PARAM
            | NC_PARAM2 | STRUCT_PARAM2 => {
                let b = pop_int(&mut int_stack);
                let a = pop_int(&mut int_stack);
                int_stack.push(Expr::Operation {
                    opcode,
                    args: vec![a, b],
                });
            }

            // 3-int → 1-string (enum_string)
            ENUM_STRING => {
                let c = pop_int(&mut int_stack);
                let b = pop_int(&mut int_stack);
                let a = pop_int(&mut int_stack);
                string_stack.push(Expr::Operation {
                    opcode,
                    args: vec![a, b, c],
                });
            }

            // 4-int → 1-int (enum)
            ENUM => {
                let d = pop_int(&mut int_stack);
                let c = pop_int(&mut int_stack);
                let b = pop_int(&mut int_stack);
                let a = pop_int(&mut int_stack);
                int_stack.push(Expr::Operation {
                    opcode,
                    args: vec![a, b, c, d],
                });
            }

            // 1-string → 1-int
            STRING_LENGTH | FRIEND_TEST => {
                let s = pop_string(&mut string_stack);
                int_stack.push(Expr::Operation {
                    opcode,
                    args: vec![s],
                });
            }

            // 2-string → 1-int
            COMPARE | STRING_INDEXOF_STRING => {
                let b = pop_string(&mut string_stack);
                let a = pop_string(&mut string_stack);
                int_stack.push(Expr::Operation {
                    opcode,
                    args: vec![a, b],
                });
            }

            // 1-string → 1-string
            LOWERCASE | ESCAPE | REMOVETAGS => {
                let s = pop_string(&mut string_stack);
                string_stack.push(Expr::Operation {
                    opcode,
                    args: vec![s],
                });
            }

            // 2-string → 1-string
            APPEND => {
                let b = pop_string(&mut string_stack);
                let a = pop_string(&mut string_stack);
                string_stack.push(Expr::Operation {
                    opcode,
                    args: vec![a, b],
                });
            }

            // 1-int + 1-string → 1-string (append_num, append_signnum)
            APPEND_NUM | APPEND_SIGNNUM => {
                let n = pop_int(&mut int_stack);
                let s = pop_string(&mut string_stack);
                string_stack.push(Expr::Operation {
                    opcode,
                    args: vec![s, n],
                });
            }

            // 1-int + 1-string → 1-int (paraheight, parawidth, string_indexof_char)
            PARAHEIGHT | PARAWIDTH | STRING_INDEXOF_CHAR => {
                let n = pop_int(&mut int_stack);
                let s = pop_string(&mut string_stack);
                int_stack.push(Expr::Operation {
                    opcode,
                    args: vec![s, n],
                });
            }

            // 2-int + 1-string → 1-string (substring)
            SUBSTRING => {
                let end = pop_int(&mut int_stack);
                let start = pop_int(&mut int_stack);
                let s = pop_string(&mut string_stack);
                string_stack.push(Expr::Operation {
                    opcode,
                    args: vec![s, start, end],
                });
            }

            // 1-int + 2-string → 1-string (text_switch)
            TEXT_SWITCH => {
                let n = pop_int(&mut int_stack);
                let b = pop_string(&mut string_stack);
                let a = pop_string(&mut string_stack);
                string_stack.push(Expr::Operation {
                    opcode,
                    args: vec![a, b, n],
                });
            }

            // --- Side-effect-only operations ---
            // 1-string → void (mes, docheat, resume_*)
            MES | DOCHEAT | RESUME_COUNTDIALOG | RESUME_NAMEDIALOG | RESUME_STRINGDIALOG => {
                let s = pop_string(&mut string_stack);
                instructions.push(Instruction::Expression(Expr::Operation {
                    opcode,
                    args: vec![s],
                }));
            }

            // 0 → void
            IF_CLOSE | LOGOUT | CC_DELETE | CC_DELETEALL | CC_CLEAROPS | IF_CLEAROPS => {
                instructions.push(Instruction::Expression(Expr::Operation {
                    opcode,
                    args: vec![],
                }));
            }

            // --- Fallback: use opcode_info metadata ---
            _ => {
                let info = opcode_info(opcode);
                let mut int_args = Vec::new();
                let mut str_args = Vec::new();
                for _ in 0..info.pops_int {
                    int_args.push(pop_int(&mut int_stack));
                }
                int_args.reverse();
                for _ in 0..info.pops_string {
                    str_args.push(pop_string(&mut string_stack));
                }
                str_args.reverse();

                // Combine args: ints first, then strings
                let mut args = int_args;
                args.append(&mut str_args);

                // If it includes the int_operand and has no pops, pass it
                if args.is_empty() && int_operand != 0 {
                    args.push(Expr::IntConstant(int_operand));
                }

                let expr = Expr::Operation {
                    opcode,
                    args: args.clone(),
                };

                if info.pushes_int > 0 {
                    int_stack.push(expr);
                } else if info.pushes_string > 0 {
                    string_stack.push(expr);
                } else {
                    // Side-effect only
                    instructions.push(Instruction::Expression(expr));
                }
            }
        }
    }

    Function {
        id: script_id,
        name: script.name.clone(),
        int_param_count: script.int_param_count,
        string_param_count: script.string_param_count,
        long_param_count: script.long_param_count,
        local_int_count: script.local_int_count,
        local_string_count: script.local_string_count,
        local_long_count: script.local_long_count,
        instructions,
    }
}

fn pop_int(stack: &mut Vec<Expr>) -> Expr {
    stack.pop().unwrap_or(Expr::IntConstant(0))
}

fn pop_string(stack: &mut Vec<Expr>) -> Expr {
    stack.pop().unwrap_or(Expr::StringConstant(String::new()))
}
