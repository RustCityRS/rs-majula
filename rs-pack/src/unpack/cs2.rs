use crate::archive;
use crate::cs2::codegen::set_type_resolver;
use crate::cs2::interpreter::interpret_with_params;
use crate::cs2::ir::{Expr, Instruction};
use crate::cs2::names::{load_script_names, load_script_names_from_cache_names};
use crate::cs2::script::Script;
use crate::cs2::types::{ArgType, TypeResolver, infer_return_types_from_callers};
use crate::cs2::{ScriptMeta, decompile_with_names};
use anyhow::{Context, Result};
use rs_io::js5::Js5Store;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use tracing::info;

pub fn unpack_cs2_scripts(cache_dir: &Path, output_dir: &Path, pack_dir: &Path) -> Result<u32> {
    let mut store = Js5Store::open(cache_dir)?;
    let names = crate::pack::names::AssetNames::load(pack_dir)?;
    let dir = output_dir.join("clientscript");
    fs::create_dir_all(&dir)?;
    let groups = store
        .reference_table(archive::CLIENTSCRIPT)
        .context("Missing clientscript table")?
        .groups
        .clone();
    for info in &groups {
        let data = store
            .read_group_raw_decompressed(archive::CLIENTSCRIPT, info.id)?
            .context("Missing clientscript")?;
        let text = crate::cs2::assembly::unpack(&data, &names)?;
        anyhow::ensure!(
            crate::cs2::assembly::pack(&text, &names)? == data,
            "Clientscript {} text round trip changed bytes",
            info.id
        );
        fs::write(
            dir.join(format!("{}.cs2", names.name("clientscript", info.id))),
            text,
        )?;
    }
    Ok(groups.len() as u32)
}

pub fn scan_cs2_varp_refs(store: &mut Js5Store) -> Result<(HashSet<u32>, HashSet<u32>)> {
    let mut varps = HashSet::new();
    let mut varbits = HashSet::new();
    let groups = store
        .reference_table(archive::CLIENTSCRIPT)
        .context("Missing clientscript table")?
        .groups
        .clone();
    for info in groups {
        let data = store
            .read_group_raw_decompressed(archive::CLIENTSCRIPT, info.id)?
            .context("Missing clientscript")?;
        let script = Script::decode(&data)?;
        for (&opcode, &operand) in script.opcodes.iter().zip(&script.int_operands) {
            match opcode {
                1 | 2 => {
                    varps.insert(operand as u32);
                }
                25 | 27 => {
                    varbits.insert(operand as u32);
                }
                _ => {}
            }
        }
    }
    Ok((varps, varbits))
}

pub fn decompile_cs2_scripts(
    cache_dir: &Path,
    output_dir: &Path,
    names_file: Option<&Path>,
) -> Result<u32> {
    // Load pack files for named constant resolution
    let pack_dir = Path::new(crate::PACK_DIR);
    if pack_dir.exists() {
        let resolver = TypeResolver::load(pack_dir);
        set_type_resolver(resolver);
    }

    let mut store = Js5Store::open(cache_dir)?;

    let group_count = store.group_count(archive::CLIENTSCRIPT).unwrap_or(0);
    if group_count == 0 {
        info!("  Archive 12 (clientscript) absent — skipping");
        return Ok(0);
    }

    // Load script names if available
    let script_names = if let Some(path) = names_file {
        if path.exists() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "tsv" {
                let first_line = fs::read_to_string(path)?
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let col_count = first_line.split('\t').count();
                if col_count >= 5 {
                    load_script_names_from_cache_names(path).ok()
                } else {
                    load_script_names(path).ok()
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Collect all script data (needed for two-pass: params then decompile)
    let mut scripts: Vec<(u32, Vec<u8>)> = Vec::new();
    for gid in 0..group_count {
        let Some(group) = store.read_group(archive::CLIENTSCRIPT, gid)? else {
            continue;
        };
        for entry in &group.entries {
            if !entry.data().is_empty() {
                scripts.push((gid, entry.data().to_vec()));
            }
        }
    }

    // Pass 1: collect parameter counts from metadata trailers
    let mut script_meta: HashMap<u32, ScriptMeta> = HashMap::new();
    for &(gid, ref data) in &scripts {
        if let Ok((ip, sp, lp)) = Script::decode_params(data) {
            script_meta.insert(
                gid,
                ScriptMeta {
                    int_params: ip,
                    string_params: sp,
                    long_params: lp,
                    ..Default::default()
                },
            );
        }
    }
    info!(
        "  Collected param metadata for {} scripts",
        script_meta.len()
    );

    // Load type resolver for return type inference
    let type_resolver = if pack_dir.exists() {
        Some(TypeResolver::load(pack_dir))
    } else {
        None
    };

    // Pass 2: decompile to infer return counts and semantic types
    for &(gid, ref data) in &scripts {
        if let Ok(script) = Script::decode(data) {
            let function = interpret_with_params(&script, gid, Some(&script_meta));
            // Collect ALL return instructions for type inference
            let mut returns: Vec<(Vec<Expr>, Vec<Expr>)> = Vec::new();
            let mut int_ret = 0u16;
            let mut str_ret = 0u16;
            for instr in &function.instructions {
                if let Instruction::Return {
                    int_values,
                    string_values,
                } = instr
                {
                    if !int_values.is_empty() || !string_values.is_empty() {
                        if int_ret == 0 && str_ret == 0 {
                            int_ret = int_values.len() as u16;
                            str_ret = string_values.len() as u16;
                        }
                        returns.push((int_values.clone(), string_values.clone()));
                    }
                }
            }
            if let Some(meta) = script_meta.get_mut(&gid) {
                meta.int_returns = int_ret;
                meta.string_returns = str_ret;
                if let Some(resolver) = &type_resolver {
                    meta.int_return_types = resolver.infer_return_types(&returns);
                }
            }
        }
    }
    let with_returns = script_meta
        .values()
        .filter(|m| m.int_returns > 0 || m.string_returns > 0)
        .count();
    info!("  Inferred return types for {} scripts", with_returns);

    // Pass 2b: caller-based return type inference — re-decompile with
    // return counts to get proc call expressions, then analyze how callers
    // use each proc's return values in typed opcode arguments.
    {
        let mut all_instrs: HashMap<u32, Vec<Instruction>> = HashMap::new();
        for &(gid, ref data) in &scripts {
            if let Ok(script) = Script::decode(data) {
                let function = interpret_with_params(&script, gid, Some(&script_meta));
                all_instrs.insert(gid, function.instructions);
            }
        }
        infer_return_types_from_callers(&all_instrs, &mut script_meta);
        let with_typed = script_meta
            .values()
            .filter(|m| m.int_return_types.iter().any(|t| *t != ArgType::Int))
            .count();
        info!("  Caller-inferred typed returns for {} scripts", with_typed);
    }

    let out_dir = output_dir.join("clientscript").join("_decompiled");
    fs::create_dir_all(&out_dir)?;

    let mut total = 0u32;
    let mut errors = 0u32;

    // Pass 3: final decompile with full metadata (params + return counts)
    for &(gid, ref data) in &scripts {
        let out_path = out_dir.join(format!("{}.cs2", gid));

        match decompile_with_names(data, gid, script_names.as_ref(), Some(&script_meta)) {
            Ok(source) => {
                fs::write(&out_path, source)?;
                total += 1;
            }
            Err(_) => {
                errors += 1;
            }
        }
    }

    if errors > 0 {
        tracing::warn!("  {} scripts failed to decompile", errors);
    }
    info!(
        "  {} cs2 scripts decompiled to {}",
        total,
        out_dir.display()
    );
    Ok(total)
}
