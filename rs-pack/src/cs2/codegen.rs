//! Code generator — produces readable CS2 source from structured constructs.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use crate::cs2::cfa::{Construct, is_empty_construct};
use crate::cs2::ir::*;
use crate::cs2::opcodes;

use crate::cs2::types::{ArgType, TypeResolver};

thread_local! {
    static SCRIPT_NAMES: RefCell<Option<HashMap<u32, String>>> = const { RefCell::new(None) };
    static TYPE_RESOLVER: RefCell<Option<TypeResolver>> = const { RefCell::new(None) };
    static CURRENT_RETURN_TYPES: RefCell<Vec<ArgType>> = const { RefCell::new(Vec::new()) };
}

/// Generate human-readable CS2 source from a structured construct tree.
pub fn generate(construct: &Construct, function: &Function) -> String {
    generate_with_names(construct, function, None, None)
}

/// Generate with optional script name lookup for proc calls.
pub fn set_type_resolver(resolver: TypeResolver) {
    TYPE_RESOLVER.with(|r| *r.borrow_mut() = Some(resolver));
}

pub fn generate_with_names(
    construct: &Construct,
    function: &Function,
    script_names: Option<&std::collections::HashMap<u32, String>>,
    script_meta: Option<&super::ScriptMeta>,
) -> String {
    let mut out = String::new();

    CURRENT_RETURN_TYPES.with(|rt| {
        *rt.borrow_mut() = script_meta
            .map(|m| m.int_return_types.clone())
            .unwrap_or_default();
    });

    if let Some(names) = script_names {
        SCRIPT_NAMES.with(|n| *n.borrow_mut() = Some(names.clone()));
    }

    let sig = script_names
        .and_then(|names| names.get(&function.id))
        .cloned()
        .unwrap_or_else(|| format!("[clientscript,script_{}]", function.id));

    let mut params = Vec::new();
    for i in 0..function.int_param_count {
        params.push(format!("int $int{}", i));
    }
    for i in 0..function.string_param_count {
        params.push(format!("string $str{}", i));
    }
    for i in 0..function.long_param_count {
        params.push(format!("long $long{}", i));
    }

    let return_type = infer_return_type_with_meta(construct, script_meta);
    let return_sig = if return_type.is_empty() {
        String::new()
    } else {
        format!("({})", return_type.join(", "))
    };

    if params.is_empty() {
        writeln!(out, "{sig}{return_sig}").unwrap();
    } else {
        writeln!(out, "{sig}({}){return_sig}", params.join(", ")).unwrap();
    }

    // Track declared local variables (params are pre-declared)
    let mut declared: HashSet<String> = HashSet::new();
    for i in 0..function.int_param_count {
        declared.insert(format!("int:{i}"));
    }
    for i in 0..function.string_param_count {
        declared.insert(format!("string:{i}"));
    }
    for i in 0..function.long_param_count {
        declared.insert(format!("long:{i}"));
    }

    emit_construct(&mut out, construct, 0, &mut declared);
    out
}

fn emit_construct(
    out: &mut String,
    construct: &Construct,
    indent: usize,
    declared: &mut HashSet<String>,
) {
    match construct {
        Construct::Seq(instrs, next) => {
            for instr in instrs {
                emit_instruction(out, instr, indent, declared);
            }
            if let Some(next) = next {
                emit_construct(out, next, indent, declared);
            }
        }
        Construct::If {
            condition,
            then_body,
            else_body,
            next,
        } => {
            // If then-body is empty and else-body exists, negate and swap
            let (cond, primary, secondary) = if is_empty_construct(then_body) && else_body.is_some()
            {
                let neg = crate::cs2::cfa::negate_condition_expr(condition.clone());
                (neg, else_body.as_ref().unwrap().as_ref(), None)
            } else {
                (condition.clone(), then_body.as_ref(), else_body.as_deref())
            };
            write_indent(out, indent);
            writeln!(out, "if ({}) {{", format_expr(&cond)).unwrap();
            emit_construct(out, primary, indent + 1, declared);
            if let Some(else_body) = secondary {
                write_indent(out, indent);
                writeln!(out, "}} else {{").unwrap();
                emit_construct(out, else_body, indent + 1, declared);
            }
            write_indent(out, indent);
            writeln!(out, "}}").unwrap();
            if let Some(next) = next {
                emit_construct(out, next, indent, declared);
            }
        }
        Construct::While {
            condition,
            body,
            next,
        } => {
            write_indent(out, indent);
            writeln!(out, "while ({}) {{", format_expr(condition)).unwrap();
            emit_construct(out, body, indent + 1, declared);
            write_indent(out, indent);
            writeln!(out, "}}").unwrap();
            if let Some(next) = next {
                emit_construct(out, next, indent, declared);
            }
        }
        Construct::Switch {
            expr,
            cases,
            default,
            next,
        } => {
            write_indent(out, indent);
            writeln!(out, "switch ({}) {{", format_expr(expr)).unwrap();
            for (keys, body) in cases {
                for key in keys {
                    write_indent(out, indent + 1);
                    writeln!(out, "case {key}:").unwrap();
                }
                emit_construct(out, body, indent + 2, declared);
            }
            if let Some(default) = default {
                write_indent(out, indent + 1);
                writeln!(out, "default:").unwrap();
                emit_construct(out, default, indent + 2, declared);
            }
            write_indent(out, indent);
            writeln!(out, "}}").unwrap();
            if let Some(next) = next {
                emit_construct(out, next, indent, declared);
            }
        }
    }
}

fn emit_instruction(
    out: &mut String,
    instr: &Instruction,
    indent: usize,
    declared: &mut HashSet<String>,
) {
    match instr {
        Instruction::Label(lbl) => {
            write_indent(out, indent.saturating_sub(1));
            writeln!(out, "label_{lbl}:").unwrap();
        }
        Instruction::Assignment { targets, expr } => {
            write_indent(out, indent);
            if targets.len() == 1 {
                let target = &targets[0];
                let (key, prefix) = match target {
                    Variable::LocalInt(i) => {
                        let k = format!("int:{i}");
                        if declared.insert(k.clone()) {
                            (k, format!("def_int $int{i}"))
                        } else {
                            (k, format!("$int{i}"))
                        }
                    }
                    Variable::LocalString(i) => {
                        let k = format!("string:{i}");
                        if declared.insert(k.clone()) {
                            (k, format!("def_string $str{i}"))
                        } else {
                            (k, format!("$str{i}"))
                        }
                    }
                    Variable::LocalLong(i) => {
                        let k = format!("long:{i}");
                        if declared.insert(k.clone()) {
                            (k, format!("def_long $long{i}"))
                        } else {
                            (k, format!("$long{i}"))
                        }
                    }
                    _ => (String::new(), format!("{target}")),
                };
                let _ = key;
                writeln!(out, "{prefix} = {};", format_expr(expr)).unwrap();
            } else {
                let targets_str: Vec<String> = targets.iter().map(|t| format!("{t}")).collect();
                writeln!(out, "({}) = {};", targets_str.join(", "), format_expr(expr)).unwrap();
            }
        }
        Instruction::Return {
            int_values,
            string_values,
        } => {
            write_indent(out, indent);
            if int_values.is_empty() && string_values.is_empty() {
                writeln!(out, "return;").unwrap();
            } else {
                // Deduplicate multi-return proc calls: if all values
                // are the same Proc expression, emit it once.
                let all_same_proc = !int_values.is_empty()
                    && !string_values.is_empty()
                    && int_values
                        .iter()
                        .chain(string_values.iter())
                        .all(|e| format_expr(e) == format_expr(&int_values[0]))
                    && matches!(&int_values[0], Expr::Proc { .. });
                if all_same_proc {
                    writeln!(out, "return({});", format_expr(&int_values[0])).unwrap();
                } else {
                    let return_types = CURRENT_RETURN_TYPES.with(|rt| rt.borrow().clone());
                    let mut parts: Vec<String> = int_values
                        .iter()
                        .enumerate()
                        .map(|(i, e)| {
                            let ty = return_types.get(i).copied();
                            format_typed_arg(e, ty)
                        })
                        .collect();
                    parts.extend(string_values.iter().map(format_expr));
                    writeln!(out, "return({});", parts.join(", ")).unwrap();
                }
            }
        }
        Instruction::Branch { condition, target } => {
            write_indent(out, indent);
            writeln!(out, "if ({}) goto label_{target};", format_expr(condition)).unwrap();
        }
        Instruction::Goto(target) => {
            write_indent(out, indent);
            writeln!(out, "goto label_{target};").unwrap();
        }
        Instruction::Switch { expr, cases } => {
            write_indent(out, indent);
            writeln!(out, "switch ({}) {{", format_expr(expr)).unwrap();
            for (key, target) in cases {
                write_indent(out, indent + 1);
                writeln!(out, "case {key}: goto label_{target};").unwrap();
            }
            write_indent(out, indent);
            writeln!(out, "}}").unwrap();
        }
        Instruction::Expression(expr) => {
            write_indent(out, indent);
            writeln!(out, "{};", format_expr(expr)).unwrap();
        }
    }
}

/// Format an expression as a human-readable string.
pub fn format_expr(expr: &Expr) -> String {
    match expr {
        Expr::IntConstant(v) => v.to_string(),
        Expr::StringConstant(s) => {
            format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
        }
        Expr::LongConstant(v) => format!("{v}L"),
        Expr::Var(v) => format!("{v}"),
        Expr::Operation { opcode, args } => format_operation(*opcode, args),
        Expr::Proc { script_id, args } => {
            let proc_name = SCRIPT_NAMES
                .with(|n| {
                    n.borrow().as_ref().and_then(|names| {
                        names.get(&(*script_id as u32)).and_then(|s| {
                            s.strip_prefix('[')
                                .and_then(|s| s.strip_suffix(']'))
                                .and_then(|s| s.split_once(','))
                                .map(|(_, name)| name.to_string())
                        })
                    })
                })
                .unwrap_or_else(|| format!("script_{script_id}"));

            if args.is_empty() {
                format!("~{proc_name}")
            } else {
                let args_str: Vec<String> = args.iter().map(format_expr).collect();
                format!("~{proc_name}({})", args_str.join(", "))
            }
        }
        Expr::StringConcat(parts) => {
            if parts.is_empty() {
                "\"\"".to_string()
            } else {
                let parts_str: Vec<String> = parts.iter().map(format_expr).collect();
                parts_str.join(" + ")
            }
        }
        Expr::Compound(exprs) => {
            let exprs_str: Vec<String> = exprs.iter().map(format_expr).collect();
            exprs_str.join(", ")
        }
    }
}

fn format_operation(opcode: u16, args: &[Expr]) -> String {
    if args.len() == 2 {
        if opcodes::is_branch(opcode) {
            let op = opcodes::branch_operator(opcode);
            return format!("{} {op} {}", format_expr(&args[0]), format_expr(&args[1]));
        }
        if opcodes::is_calc_arithmetic(opcode) {
            let this_expr = Expr::Operation {
                opcode,
                args: args.to_vec(),
            };
            if opcodes::needs_calc(&this_expr) {
                return format!("calc({})", format_calc_inner(&this_expr));
            }
        }
    }

    let name = opcodes::opcode_name(opcode);
    let func_name = if name.is_empty() {
        format!("op{opcode}")
    } else {
        name.to_string()
    };

    if args.is_empty() {
        func_name
    } else {
        let type_sig = TypeResolver::opcode_arg_types(opcode);
        let args_str: Vec<String> = args
            .iter()
            .enumerate()
            .map(|(i, a)| format_typed_arg(a, type_sig.get(i).copied()))
            .collect();
        format!("{func_name}({})", args_str.join(", "))
    }
}

fn format_typed_arg(expr: &Expr, arg_type: Option<ArgType>) -> String {
    if let (Some(ty), Expr::IntConstant(v)) = (arg_type, expr) {
        if ty != ArgType::Int {
            let resolved = TYPE_RESOLVER.with(|r| {
                r.borrow()
                    .as_ref()
                    .and_then(|resolver| resolver.resolve(*v, ty).map(|s| s.to_string()))
            });
            if let Some(name) = resolved {
                return name;
            }
        }
    }
    format_expr(expr)
}

fn format_calc_inner(expr: &Expr) -> String {
    match expr {
        Expr::Operation { opcode, args }
            if args.len() == 2 && opcodes::is_calc_arithmetic(*opcode) =>
        {
            let op = opcodes::calc_operator(*opcode).unwrap();
            let left = format_calc_inner(&args[0]);
            let right = format_calc_inner(&args[1]);
            format!("{left} {op} {right}")
        }
        other => format_expr(other),
    }
}

fn write_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("    ");
    }
}

/// Walk the construct tree and build the return type signature,
/// using semantic type info from ScriptMeta when available.
fn infer_return_type_with_meta(
    construct: &Construct,
    meta: Option<&super::ScriptMeta>,
) -> Vec<String> {
    let mut returns: Vec<(usize, usize)> = Vec::new();
    collect_returns(construct, &mut returns);
    if returns.is_empty() {
        return Vec::new();
    }
    let (int_count, str_count) = returns
        .iter()
        .find(|&&(i, s)| i > 0 || s > 0)
        .copied()
        .unwrap_or((0, 0));
    if int_count == 0 && str_count == 0 {
        return Vec::new();
    }
    let mut types = Vec::new();
    for i in 0..int_count {
        let type_name = meta
            .and_then(|m| m.int_return_types.get(i))
            .map(|ty| arg_type_name(*ty))
            .unwrap_or("int");
        types.push(type_name.to_string());
    }
    for _ in 0..str_count {
        types.push("string".to_string());
    }
    types
}

fn arg_type_name(ty: ArgType) -> &'static str {
    match ty {
        ArgType::Int => "int",
        ArgType::Obj => "namedobj",
        ArgType::Npc => "npc",
        ArgType::Loc => "loc",
        ArgType::Stat => "stat",
        ArgType::Inv => "inv",
        ArgType::Seq => "seq",
        ArgType::Spotanim => "spotanim",
        ArgType::Component => "component",
        ArgType::Coord => "coord",
        ArgType::Interface => "interface",
        ArgType::Struct => "struct",
        ArgType::Param => "param",
        ArgType::Enum => "enum",
        ArgType::Synth => "synth",
        ArgType::Graphic => "graphic",
        ArgType::FontMetrics => "fontmetrics",
        ArgType::Model => "model",
    }
}

fn collect_returns(construct: &Construct, out: &mut Vec<(usize, usize)>) {
    match construct {
        Construct::Seq(instrs, next) => {
            for instr in instrs {
                if let Instruction::Return {
                    int_values,
                    string_values,
                } = instr
                {
                    out.push((int_values.len(), string_values.len()));
                }
            }
            if let Some(next) = next {
                collect_returns(next, out);
            }
        }
        Construct::If {
            then_body,
            else_body,
            next,
            ..
        } => {
            collect_returns(then_body, out);
            if let Some(eb) = else_body {
                collect_returns(eb, out);
            }
            if let Some(n) = next {
                collect_returns(n, out);
            }
        }
        Construct::While { body, next, .. } => {
            collect_returns(body, out);
            if let Some(n) = next {
                collect_returns(n, out);
            }
        }
        Construct::Switch {
            cases,
            default,
            next,
            ..
        } => {
            for (_, body) in cases {
                collect_returns(body, out);
            }
            if let Some(d) = default {
                collect_returns(d, out);
            }
            if let Some(n) = next {
                collect_returns(n, out);
            }
        }
    }
}
