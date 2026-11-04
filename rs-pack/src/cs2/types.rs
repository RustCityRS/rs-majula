#![allow(clippy::needless_range_loop)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Type-aware name resolution for CS2 decompilation.
//!
//! Maps integer constants to named references (^obj_name, etc.) based on
//! opcode parameter type annotations and pack file lookups.

use std::collections::HashMap;
use std::path::Path;

/// Which semantic type an integer argument represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArgType {
    Int,
    Obj,
    Npc,
    Loc,
    Stat,
    Inv,
    Seq,
    Spotanim,
    Component,
    Coord,
    Interface,
    Struct,
    Param,
    Enum,
    Synth,
    Graphic,
    FontMetrics,
    Model,
}

/// Pack-file backed name registry for type-aware constant resolution.
pub struct TypeResolver {
    pub objs: HashMap<i32, String>,
    pub npcs: HashMap<i32, String>,
    pub locs: HashMap<i32, String>,
    pub stats: HashMap<i32, String>,
    pub invs: HashMap<i32, String>,
    pub seqs: HashMap<i32, String>,
    pub structs: HashMap<i32, String>,
    pub params: HashMap<i32, String>,
    pub enums: HashMap<i32, String>,
    pub interfaces: HashMap<i32, String>,
    pub components: HashMap<i32, String>,
    pub synths: HashMap<i32, String>,
    pub spotanims: HashMap<i32, String>,
}

impl TypeResolver {
    pub fn load(pack_dir: &Path) -> Self {
        Self {
            objs: load_pack(pack_dir, "obj.pack"),
            npcs: load_pack(pack_dir, "npc.pack"),
            locs: load_pack(pack_dir, "loc.pack"),
            stats: load_pack(pack_dir, "stat.pack"),
            invs: load_pack(pack_dir, "inv.pack"),
            seqs: load_pack(pack_dir, "seq.pack"),
            structs: load_pack(pack_dir, "struct.pack"),
            params: load_pack(pack_dir, "param.pack"),
            enums: load_pack(pack_dir, "enum.pack"),
            interfaces: load_pack(pack_dir, "interface.pack"),
            components: load_pack(pack_dir, "component.pack"),
            synths: load_pack(pack_dir, "synth.pack"),
            spotanims: load_pack(pack_dir, "spotanim.pack"),
        }
    }

    pub fn resolve(&self, value: i32, arg_type: ArgType) -> Option<&str> {
        let map = match arg_type {
            ArgType::Obj => &self.objs,
            ArgType::Npc => &self.npcs,
            ArgType::Loc => &self.locs,
            ArgType::Stat => &self.stats,
            ArgType::Inv => &self.invs,
            ArgType::Seq => &self.seqs,
            ArgType::Struct => &self.structs,
            ArgType::Param => &self.params,
            ArgType::Enum => &self.enums,
            ArgType::Interface => &self.interfaces,
            ArgType::Component => &self.components,
            ArgType::Synth => &self.synths,
            ArgType::Spotanim => &self.spotanims,
            _ => return None,
        };
        map.get(&value).map(|s| s.as_str())
    }

    /// Get the argument types for an opcode. Returns a slice where
    /// index 0 = first arg popped (last pushed), etc.
    pub fn opcode_arg_types(opcode: u16) -> &'static [ArgType] {
        use super::opcodes::*;
        match opcode {
            // oc_* — first arg is obj
            OC_NAME | OC_COST | OC_STACKABLE | OC_CERT | OC_UNCERT | OC_MEMBERS | OC_WEIGHT
            | OC_WEARPOS | OC_WEARPOS2 => &[ArgType::Obj],
            OC_OP | OC_IOP => &[ArgType::Obj, ArgType::Int],
            OC_PARAM => &[ArgType::Obj, ArgType::Param],

            // cc_setobject(obj, count)
            CC_SETOBJECT | CC_SETOBJECT_NONUM => &[ArgType::Obj, ArgType::Int],
            1208 | 1212 => &[ArgType::Obj, ArgType::Int], // cc_setobject variants
            // if_setobject(obj, count, component)
            IF_SETOBJECT | IF_SETOBJECT_NONUM => &[ArgType::Obj, ArgType::Int],
            2208 | 2212 => &[ArgType::Obj, ArgType::Int, ArgType::Int],

            // inv_* — first arg is inv
            INV_GETOBJ | INV_GETNUM | INV_TOTAL => &[ArgType::Inv, ArgType::Obj],
            INV_SIZE => &[ArgType::Inv],
            3313 | 3331 => &[ArgType::Inv, ArgType::Int], // inv_alwaysnoted variants

            // stat — arg is stat
            STAT | STAT_BASE | STAT_XP => &[ArgType::Stat],
            3330 => &[ArgType::Int], // player_getlevel_byxp

            // nc_param(npc, param), lc_param(loc, param), struct_param(struct, param)
            NC_PARAM | 4300 => &[ArgType::Npc, ArgType::Param],
            LC_PARAM => &[ArgType::Loc, ArgType::Param],
            STRUCT_PARAM | 4500 => &[ArgType::Struct, ArgType::Param],

            // enum(input_type, output_type, enum, key)
            ENUM => &[ArgType::Int, ArgType::Int, ArgType::Enum, ArgType::Int],
            ENUM_STRING => &[ArgType::Int, ArgType::Int, ArgType::Enum],
            ENUM_GETOUTPUTCOUNT => &[ArgType::Enum],

            // seq
            1110 => &[ArgType::Seq], // cc_setmodelanim

            // sound
            SOUND_SYNTH => &[ArgType::Synth, ArgType::Int, ArgType::Int],

            _ => &[],
        }
    }

    /// Infer the semantic type of an integer value by checking pack files.
    /// Returns the most specific type that matches, with priority ordering
    /// to avoid false positives.
    pub fn infer_int_type(&self, value: i32) -> ArgType {
        if value < 0 {
            return ArgType::Int;
        }
        // Component hashes: interface_id << 16 | component_id
        // Valid interfaces are typically 0-2000, components 0-500
        let iface = (value >> 16) & 0xFFFF;
        let comp = value & 0xFFFF;
        if iface > 0 && iface < 2000 && comp < 500 {
            if self.interfaces.contains_key(&iface) {
                return ArgType::Component;
            }
        }
        // Stat IDs are 0-24, but skip 0-1 (too common as bool/count)
        if (2..=30).contains(&value) && self.stats.contains_key(&value) {
            return ArgType::Stat;
        }
        // Don't guess obj/npc/loc from values alone — the pack files
        // are too dense (20K+ obj entries) and cause false positives.
        // These types are only reliably determined by caller analysis.
        ArgType::Int
    }

    /// Infer return types for a script by analyzing captured return values.
    /// Returns a vec of ArgTypes for each int return position.
    pub fn infer_return_types(
        &self,
        returns: &[(Vec<super::ir::Expr>, Vec<super::ir::Expr>)],
    ) -> Vec<ArgType> {
        if returns.is_empty() {
            return Vec::new();
        }
        // Find the max int return count
        let int_count = returns
            .iter()
            .map(|(ints, _)| ints.len())
            .max()
            .unwrap_or(0);
        let mut types = vec![ArgType::Int; int_count];

        for pos in 0..int_count {
            // Collect all constant values at this position across returns
            let mut constants: Vec<i32> = Vec::new();
            let mut has_non_constant = false;
            for (ints, _) in returns {
                if let Some(expr) = ints.get(pos) {
                    match expr {
                        super::ir::Expr::IntConstant(v) => constants.push(*v),
                        _ => has_non_constant = true,
                    }
                }
            }
            if constants.is_empty() {
                continue;
            }
            // Check if most constants at this position share a type
            let mut type_counts: HashMap<ArgType, usize> = HashMap::new();
            for &v in &constants {
                let ty = self.infer_int_type(v);
                *type_counts.entry(ty).or_default() += 1;
            }
            // Use the most common non-Int type if it covers >50% of constants
            let best = type_counts
                .iter()
                .filter(|(ty, _)| **ty != ArgType::Int)
                .max_by_key(|(_, count)| **count);
            if let Some((ty, count)) = best {
                let ty = *ty;
                let count = *count;
                let total = constants.len();
                // If the type matches most returns (allowing for -1 sentinel values)
                let _int_count_at_pos = type_counts.get(&ArgType::Int).copied().unwrap_or(0);
                let sentinel_count = constants.iter().filter(|&&v| v == -1 || v == 0).count();
                if count + sentinel_count >= total || (!has_non_constant && count > total / 2) {
                    types[pos] = ty;
                }
            }
        }
        types
    }
}

/// Infer return types by analyzing how callers USE return values.
/// Scans all scripts' IR for proc calls whose return values flow
/// into typed opcode parameters.
pub fn infer_return_types_from_callers(
    all_instructions: &HashMap<u32, Vec<super::ir::Instruction>>,
    meta: &mut HashMap<u32, super::ScriptMeta>,
) {
    use super::ir::{Expr, Instruction};

    // For each script, scan its instructions for typed usage of proc returns
    let mut inferred: HashMap<u32, Vec<(usize, ArgType)>> = HashMap::new();

    for instrs in all_instructions.values() {
        for instr in instrs {
            if let Instruction::Expression(Expr::Operation { opcode, args }) = instr {
                let type_sig = TypeResolver::opcode_arg_types(*opcode);
                for (i, arg) in args.iter().enumerate() {
                    if let Some(&ty) = type_sig.get(i) {
                        if ty != ArgType::Int {
                            // This arg has a known type. Check if it's a Proc return.
                            if let Expr::Proc { script_id, .. } = arg {
                                // Single-return proc used in typed position →
                                // return position 0 is this type
                                inferred.entry(*script_id as u32).or_default().push((0, ty));
                            }
                        }
                    }
                }
            }
            // Also check assignments where RHS is a typed operation using a proc
            if let Instruction::Assignment { expr, .. } = instr {
                check_expr_for_typed_proc_usage(expr, &mut inferred);
            }
        }
    }

    // Apply inferred types to metadata
    for (script_id, positions) in &inferred {
        if let Some(m) = meta.get_mut(script_id) {
            for &(pos, ty) in positions {
                if pos < m.int_return_types.len() && m.int_return_types[pos] == ArgType::Int {
                    m.int_return_types[pos] = ty;
                }
            }
        }
    }
}

fn check_expr_for_typed_proc_usage(
    expr: &super::ir::Expr,
    inferred: &mut HashMap<u32, Vec<(usize, ArgType)>>,
) {
    use super::ir::Expr;
    if let Expr::Operation { opcode, args } = expr {
        let type_sig = TypeResolver::opcode_arg_types(*opcode);
        for (i, arg) in args.iter().enumerate() {
            if let Some(&ty) = type_sig.get(i) {
                if ty != ArgType::Int {
                    if let Expr::Proc { script_id, .. } = arg {
                        inferred.entry(*script_id as u32).or_default().push((0, ty));
                    }
                }
            }
            // Recurse into nested expressions
            check_expr_for_typed_proc_usage(arg, inferred);
        }
    }
}

fn load_pack(dir: &Path, name: &str) -> HashMap<i32, String> {
    let path = dir.join(name);
    let mut map = HashMap::new();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let line = line.trim();
            if let Some((id_str, name)) = line.split_once('=') {
                if let Ok(id) = id_str.parse::<i32>() {
                    map.insert(id, name.to_string());
                }
            }
        }
    }
    map
}
