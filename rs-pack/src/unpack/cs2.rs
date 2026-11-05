use anyhow::Result;
use rs_dat2::CacheStore;
use std::io::Write;
use std::path::Path;
use tracing::info;

const ARCHIVE_CLIENTSCRIPT: u8 = 12;

struct Cs2Script {
    name: String,
    local_int_count: u16,
    local_string_count: u16,
    local_long_count: u16,
    int_param_count: u16,
    string_param_count: u16,
    long_param_count: u16,
    instructions: Vec<Cs2Instruction>,
    switches: Vec<Vec<(i32, i32)>>,
}

struct Cs2Instruction {
    opcode: u16,
    operand: Cs2Operand,
}

enum Cs2Operand {
    Int(i32),
    String(String),
    Long(i64),
    Byte(u8),
}

fn decode_cs2(data: &[u8]) -> Result<Cs2Script> {
    if data.len() < 18 {
        anyhow::bail!("script too short: {} bytes", data.len());
    }

    let len = data.len();
    let switch_section_size = u16::from_be_bytes([data[len - 2], data[len - 1]]) as usize;
    let meta_start = len - switch_section_size - 16 - 2;

    let instruction_count = i32::from_be_bytes([
        data[meta_start],
        data[meta_start + 1],
        data[meta_start + 2],
        data[meta_start + 3],
    ]) as usize;
    let local_int_count = u16::from_be_bytes([data[meta_start + 4], data[meta_start + 5]]);
    let local_string_count = u16::from_be_bytes([data[meta_start + 6], data[meta_start + 7]]);
    let local_long_count = u16::from_be_bytes([data[meta_start + 8], data[meta_start + 9]]);
    let int_param_count = u16::from_be_bytes([data[meta_start + 10], data[meta_start + 11]]);
    let string_param_count = u16::from_be_bytes([data[meta_start + 12], data[meta_start + 13]]);
    let long_param_count = u16::from_be_bytes([data[meta_start + 14], data[meta_start + 15]]);

    // Switch tables
    let mut spos = meta_start + 16;
    let switch_count = data[spos] as usize;
    spos += 1;
    let mut switches = Vec::with_capacity(switch_count);
    for _ in 0..switch_count {
        let case_count = u16::from_be_bytes([data[spos], data[spos + 1]]) as usize;
        spos += 2;
        let mut cases = Vec::with_capacity(case_count);
        for _ in 0..case_count {
            let key =
                i32::from_be_bytes([data[spos], data[spos + 1], data[spos + 2], data[spos + 3]]);
            spos += 4;
            let target =
                i32::from_be_bytes([data[spos], data[spos + 1], data[spos + 2], data[spos + 3]]);
            spos += 4;
            cases.push((key, target));
        }
        switches.push(cases);
    }

    // Name string at position 0
    let mut pos: usize = 0;
    let name = read_nullstr(data, &mut pos);

    // Instructions
    let mut instructions = Vec::with_capacity(instruction_count);
    while pos < meta_start {
        let opcode = u16::from_be_bytes([data[pos], data[pos + 1]]);
        pos += 2;

        let operand = if opcode == 3 {
            Cs2Operand::String(read_nullstr(data, &mut pos))
        } else if opcode == 54 {
            let v = i64::from_be_bytes([
                data[pos],
                data[pos + 1],
                data[pos + 2],
                data[pos + 3],
                data[pos + 4],
                data[pos + 5],
                data[pos + 6],
                data[pos + 7],
            ]);
            pos += 8;
            Cs2Operand::Long(v)
        } else if opcode >= 150 || opcode == 21 || opcode == 38 || opcode == 39 {
            let v = data[pos];
            pos += 1;
            Cs2Operand::Byte(v)
        } else {
            let v = i32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
            pos += 4;
            Cs2Operand::Int(v)
        };

        instructions.push(Cs2Instruction { opcode, operand });
    }

    Ok(Cs2Script {
        name,
        local_int_count,
        local_string_count,
        local_long_count,
        int_param_count,
        string_param_count,
        long_param_count,
        instructions,
        switches,
    })
}

fn read_nullstr(data: &[u8], pos: &mut usize) -> String {
    let start = *pos;
    while *pos < data.len() && data[*pos] != 0 {
        *pos += 1;
    }
    let s = String::from_utf8_lossy(&data[start..*pos]).to_string();
    if *pos < data.len() {
        *pos += 1; // skip null terminator
    }
    s
}

fn opcode_name(opcode: u16) -> &'static str {
    match opcode {
        0 => "push_constant_int",
        1 => "push_var",
        2 => "pop_var",
        3 => "push_constant_string",
        6 => "branch",
        7 => "branch_not",
        8 => "branch_equals",
        9 => "branch_less_than",
        10 => "branch_greater_than",
        21 => "return",
        25 => "push_varbit",
        27 => "pop_varbit",
        31 => "branch_less_than_or_equals",
        32 => "branch_greater_than_or_equals",
        33 => "push_int_local",
        34 => "pop_int_local",
        35 => "push_string_local",
        36 => "pop_string_local",
        37 => "join_string",
        38 => "pop_int_discard",
        39 => "pop_string_discard",
        40 => "gosub_with_params",
        42 => "push_varc_int",
        43 => "pop_varc_int",
        44 => "define_array",
        45 => "push_array_int",
        46 => "pop_array_int",
        47 => "push_varc_string_old",
        48 => "pop_varc_string_old",
        51 => "switch",
        54 => "push_constant_long",
        55 => "pop_long_discard",
        66 => "push_long_local",
        67 => "pop_long_local",
        68 => "branch_not_long",
        69 => "branch_equals_long",
        70 => "branch_less_than_long",
        71 => "branch_greater_than_long",
        72 => "branch_less_than_or_equals_long",
        73 => "branch_greater_than_or_equals_long",
        86 => "branch_if_true",
        87 => "branch_if_false",
        100 => "cc_create",
        101 => "cc_delete",
        102 => "cc_deleteall",
        112 => "push_long_varc",
        113 => "pop_long_varc",
        114 => "push_long_local2",
        115 => "pop_long_local2",
        150 => "cc_setposition",
        200 => "cc_find",
        201 => "if_find",
        1000 => "cc_setposition2",
        1001 => "cc_setsize",
        1003 => "cc_sethide",
        1100 => "cc_setscrollpos",
        1101 => "cc_setcolour",
        1102 => "cc_setfill",
        1103 => "cc_settrans",
        1105 => "cc_setgraphic",
        1108 => "cc_setmodel",
        1109 => "cc_setmodelangle",
        1110 => "cc_setmodelanim",
        1112 => "cc_settext",
        1113 => "cc_settextfont",
        1114 => "cc_settextalign",
        1115 => "cc_settextshadow",
        1120 => "cc_setscrollsize",
        1200 => "cc_setobject",
        1205 => "cc_setobject_nonum",
        1300 => "cc_setop",
        1301 => "cc_setdraggable",
        1305 => "cc_setopbase",
        1306 => "cc_settargetverb",
        1307 => "cc_clearops",
        1350 => "cc_setopkey",
        1400 => "cc_setonclick",
        1401 => "cc_setonhold",
        1402 => "cc_setonrelease",
        1403 => "cc_setonmouseover",
        1404 => "cc_setonmouseleave",
        1405 => "cc_setondrag",
        1407 => "cc_setonvartransmit",
        1408 => "cc_setontimer",
        1409 => "cc_setonop",
        1410 => "cc_setondragcomplete",
        1411 => "cc_setonclickrepeat",
        1412 => "cc_setonmouserepeat",
        1414 => "cc_setoninvtransmit",
        1415 => "cc_setonstattransmit",
        1416 => "cc_setontargetenter",
        1417 => "cc_setonscrollwheel",
        1418 => "cc_setonchattransmit",
        1419 => "cc_setonkey",
        1500 => "cc_getx",
        1501 => "cc_gety",
        1502 => "cc_getwidth",
        1503 => "cc_getheight",
        1504 => "cc_gethide",
        1505 => "cc_getlayer",
        1600 => "cc_getscrollx",
        1601 => "cc_getscrolly",
        1602 => "cc_gettext",
        1700 => "cc_getinvobject",
        1701 => "cc_getinvcount",
        1702 => "cc_getid",
        2000 => "if_setposition",
        2001 => "if_setsize",
        2003 => "if_sethide",
        2100 => "if_setscrollpos",
        2101 => "if_setcolour",
        2105 => "if_setgraphic",
        2112 => "if_settext",
        2113 => "if_settextfont",
        2114 => "if_settextalign",
        2200 => "if_setobject",
        2300 => "if_setop",
        2500 => "if_getx",
        2501 => "if_gety",
        2502 => "if_getwidth",
        2503 => "if_getheight",
        2504 => "if_gethide",
        2602 => "if_gettext",
        2700 => "if_getinvobject",
        2701 => "if_getinvcount",
        3100 => "mes",
        3101 => "anim",
        3103 => "if_close",
        3104 => "resume_countdialog",
        3105 => "resume_namedialog",
        3106 => "resume_stringdialog",
        3107 => "opplayer",
        3200 => "sound_synth",
        3201 => "sound_song",
        3202 => "sound_jingle",
        3300 => "clientclock",
        3301 => "inv_getobj",
        3302 => "inv_getnum",
        3303 => "inv_total",
        3304 => "inv_size",
        3305 => "stat",
        3306 => "stat_base",
        3307 => "stat_xp",
        3308 => "coord",
        3312 => "map_members",
        3316 => "staffmodlevel",
        3317 => "reboottimer",
        3400 => "enum_string",
        3408 => "enum",
        3411 => "enum_getoutputcount",
        3600 => "friend_count",
        3601 => "friend_getname",
        3602 => "friend_getworld",
        3609 => "friend_test",
        4000 => "add",
        4001 => "sub",
        4002 => "multiply",
        4003 => "div",
        4004 => "random",
        4005 => "randominc",
        4006 => "interpolate",
        4007 => "addpercent",
        4008 => "setbit",
        4009 => "clearbit",
        4010 => "testbit",
        4011 => "mod",
        4014 => "and",
        4015 => "or",
        4018 => "scale",
        4100 => "append_num",
        4101 => "append",
        4102 => "append_signnum",
        4103 => "lowercase",
        4106 => "tostring",
        4107 => "compare",
        4108 => "paraheight",
        4109 => "parawidth",
        4110 => "text_switch",
        4117 => "string_length",
        4200 => "oc_name",
        4201 => "oc_op",
        4202 => "oc_iop",
        4203 => "oc_cost",
        4204 => "oc_stackable",
        4205 => "oc_cert",
        4206 => "oc_uncert",
        4207 => "oc_members",
        5000 => "chat_getfilter_public",
        5001 => "chat_setfilter",
        5015 => "chat_playername",
        5020 => "docheat",
        5306 => "getwindowmode",
        5307 => "setwindowmode",
        5504 => "cam_forceangle",
        5630 => "logout",
        6200 => "viewport_setfov",
        6500 => "worldlist_fetch",
        6501 => "worldlist_start",
        6502 => "worldlist_next",
        6513 => "nc_param",
        6514 => "lc_param",
        6515 => "oc_param",
        6516 => "struct_param",
        6518 => "on_mobile",
        6519 => "clienttype",
        _ => "",
    }
}

fn format_operand(instr: &Cs2Instruction) -> String {
    match &instr.operand {
        Cs2Operand::Int(v) => v.to_string(),
        Cs2Operand::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Cs2Operand::Long(v) => format!("{}L", v),
        Cs2Operand::Byte(v) => v.to_string(),
    }
}

pub fn unpack_cs2_scripts(cache_dir: &Path, output_dir: &Path) -> Result<u32> {
    let mut store = CacheStore::open(cache_dir)?;

    let group_count = store.group_count(ARCHIVE_CLIENTSCRIPT).unwrap_or(0);
    if group_count == 0 {
        info!("  Archive 12 (clientscript) absent — skipping");
        return Ok(0);
    }

    let out_dir = output_dir.join("clientscript").join("_unpack");
    std::fs::create_dir_all(&out_dir)?;

    let mut total = 0u32;
    let mut errors = 0u32;

    for gid in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_CLIENTSCRIPT, gid)? else {
            continue;
        };

        for entry in &group.entries {
            if entry.data.is_empty() {
                continue;
            }

            let script_id = gid;
            let out_path = out_dir.join(format!("{}.cs2", script_id));

            match decode_cs2(&entry.data) {
                Ok(script) => {
                    let mut file = std::fs::File::create(&out_path)?;

                    // Header
                    if !script.name.is_empty() {
                        writeln!(file, "; name: {}", script.name)?;
                    }
                    writeln!(file, "; id: {}", script_id)?;
                    writeln!(file, "int_param_count={}", script.int_param_count)?;
                    writeln!(file, "string_param_count={}", script.string_param_count)?;
                    writeln!(file, "long_param_count={}", script.long_param_count)?;
                    writeln!(file, "local_int_count={}", script.local_int_count)?;
                    writeln!(file, "local_string_count={}", script.local_string_count)?;
                    writeln!(file, "local_long_count={}", script.local_long_count)?;
                    writeln!(file)?;

                    // Instructions
                    for (i, instr) in script.instructions.iter().enumerate() {
                        let name = opcode_name(instr.opcode);
                        let operand = format_operand(instr);
                        if !name.is_empty() {
                            writeln!(file, "{:>5}: {} {}", i, name, operand)?;
                        } else {
                            writeln!(file, "{:>5}: op{} {}", i, instr.opcode, operand)?;
                        }
                    }

                    // Switch tables
                    if !script.switches.is_empty() {
                        writeln!(file)?;
                        for (i, table) in script.switches.iter().enumerate() {
                            writeln!(file, "switch_{}:", i)?;
                            for (key, target) in table {
                                writeln!(file, "  {} => {}", key, target)?;
                            }
                        }
                    }

                    total += 1;
                }
                Err(e) => {
                    errors += 1;
                    if errors <= 5 {
                        tracing::warn!("  script {}: {}", script_id, e);
                    }
                }
            }
        }
    }

    if errors > 0 {
        tracing::warn!("  {} scripts failed to decode", errors);
    }
    info!(
        "  {} cs2 scripts disassembled to {}",
        total,
        out_dir.display()
    );
    Ok(total)
}

/// Extract numeric IDs following a prefix (e.g. `%varp` → `%varp123` yields 123).
fn extract_var_ids(source: &str, prefix: &str, out: &mut std::collections::HashSet<u32>) {
    let mut search_from = 0;
    while let Some(pos) = source[search_from..].find(prefix) {
        let abs = search_from + pos + prefix.len();
        let digit_start = abs;
        let mut digit_end = digit_start;
        while digit_end < source.len() && source.as_bytes()[digit_end].is_ascii_digit() {
            digit_end += 1;
        }
        if digit_end > digit_start {
            if let Ok(id) = source[digit_start..digit_end].parse::<u32>() {
                out.insert(id);
            }
        }
        search_from = digit_end.max(abs);
    }
}

/// Scan decompiled CS2 source files for `%varp<N>` and `%varbit<N>` references.
///
/// Reads `*.cs2` files from `cs2_dir` (e.g. `content/clientscript/_decompiled`).
/// Returns `(varp_ids, varbit_ids)`.
pub fn scan_cs2_varp_refs(
    cs2_dir: &Path,
) -> Result<(
    std::collections::HashSet<u32>,
    std::collections::HashSet<u32>,
)> {
    use std::collections::HashSet;

    let mut varp_ids = HashSet::new();
    let mut varbit_ids = HashSet::new();

    if !cs2_dir.is_dir() {
        anyhow::bail!(
            "Decompiled CS2 directory not found: {}. Run the decompile step first.",
            cs2_dir.display()
        );
    }

    let mut file_count = 0u32;
    for entry in std::fs::read_dir(cs2_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("cs2") {
            continue;
        }
        let source = std::fs::read_to_string(&path)?;
        file_count += 1;

        extract_var_ids(&source, "%varp", &mut varp_ids);
        extract_var_ids(&source, "%varbit", &mut varbit_ids);
    }

    info!(
        "  Scanned {} decompiled CS2 files from {}",
        file_count,
        cs2_dir.display()
    );
    Ok((varp_ids, varbit_ids))
}

pub fn decompile_cs2_scripts(
    cache_dir: &Path,
    output_dir: &Path,
    names_file: Option<&Path>,
) -> Result<u32> {
    use std::collections::HashMap;

    // Load pack files for named constant resolution
    let pack_dir = std::path::Path::new(crate::PACK_DIR);
    if pack_dir.exists() {
        let resolver = crate::cs2::types::TypeResolver::load(pack_dir);
        crate::cs2::codegen::set_type_resolver(resolver);
    }

    let mut store = CacheStore::open(cache_dir)?;

    let group_count = store.group_count(ARCHIVE_CLIENTSCRIPT).unwrap_or(0);
    if group_count == 0 {
        info!("  Archive 12 (clientscript) absent — skipping");
        return Ok(0);
    }

    // Load script names if available
    let script_names = if let Some(path) = names_file {
        if path.exists() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "tsv" {
                let first_line = std::fs::read_to_string(path)?
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let col_count = first_line.split('\t').count();
                if col_count >= 5 {
                    crate::cs2::names::load_script_names_from_cache_names(path).ok()
                } else {
                    crate::cs2::names::load_script_names(path).ok()
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
        let Some(group) = store.read_group(ARCHIVE_CLIENTSCRIPT, gid)? else {
            continue;
        };
        for entry in &group.entries {
            if !entry.data.is_empty() {
                scripts.push((gid, entry.data.to_vec()));
            }
        }
    }

    // Pass 1: collect parameter counts from metadata trailers
    let mut script_meta: HashMap<u32, crate::cs2::ScriptMeta> = HashMap::new();
    for &(gid, ref data) in &scripts {
        if let Ok((ip, sp, lp)) = crate::cs2::script::Script::decode_params(data) {
            script_meta.insert(
                gid,
                crate::cs2::ScriptMeta {
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
        Some(crate::cs2::types::TypeResolver::load(pack_dir))
    } else {
        None
    };

    // Pass 2: decompile to infer return counts and semantic types
    for &(gid, ref data) in &scripts {
        if let Ok(script) = crate::cs2::script::Script::decode(data) {
            let function =
                crate::cs2::interpreter::interpret_with_params(&script, gid, Some(&script_meta));
            // Collect ALL return instructions for type inference
            let mut returns: Vec<(Vec<crate::cs2::ir::Expr>, Vec<crate::cs2::ir::Expr>)> =
                Vec::new();
            let mut int_ret = 0u16;
            let mut str_ret = 0u16;
            for instr in &function.instructions {
                if let crate::cs2::ir::Instruction::Return {
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
        let mut all_instrs: HashMap<u32, Vec<crate::cs2::ir::Instruction>> = HashMap::new();
        for &(gid, ref data) in &scripts {
            if let Ok(script) = crate::cs2::script::Script::decode(data) {
                let function = crate::cs2::interpreter::interpret_with_params(
                    &script,
                    gid,
                    Some(&script_meta),
                );
                all_instrs.insert(gid, function.instructions);
            }
        }
        crate::cs2::types::infer_return_types_from_callers(&all_instrs, &mut script_meta);
        let with_typed = script_meta
            .values()
            .filter(|m| {
                m.int_return_types
                    .iter()
                    .any(|t| *t != crate::cs2::types::ArgType::Int)
            })
            .count();
        info!("  Caller-inferred typed returns for {} scripts", with_typed);
    }

    let out_dir = output_dir.join("clientscript").join("_decompiled");
    std::fs::create_dir_all(&out_dir)?;

    let mut total = 0u32;
    let mut errors = 0u32;

    // Pass 3: final decompile with full metadata (params + return counts)
    for &(gid, ref data) in &scripts {
        let out_path = out_dir.join(format!("{}.cs2", gid));

        match crate::cs2::decompile_with_names(data, gid, script_names.as_ref(), Some(&script_meta))
        {
            Ok(source) => {
                std::fs::write(&out_path, source)?;
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
