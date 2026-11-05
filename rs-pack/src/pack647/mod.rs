#![allow(clippy::type_complexity, clippy::needless_range_loop)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Pack — text config files → server binary .dat/.idx.
//! Mirrors 2004scape's `engine/tools/pack/config/PackShared.ts`.

pub mod anim;
#[path = "../pack/config/category.rs"]
pub mod category;
pub mod config_generic;
pub mod dbrow;
pub mod dbtable;
pub mod enum_pack;
pub mod flo;
pub mod flu;
pub mod hunt;
pub mod idk;
pub mod inv;
pub mod loc;
pub mod map_spawns;
pub mod mesanim;
pub mod npc;
pub mod obj;
pub mod param;
pub mod seq;
pub mod spotanim;
pub mod struct_pack;
pub mod varbit;
pub mod varn;
pub mod varp;
pub mod vars;

pub use super::{pack_registry, packed_data};

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use rs_dat2::archive::CONFIG_UNKNOWN_GROUPS;
use tracing::info;

use crate::pack_registry::PackRegistry;

pub fn pack_all(
    source_dir: &Path,
    pack_dir: &Path,
    server_dir: &Path,
    client_dir: &Path,
    verify: bool,
    force: bool,
) -> Result<()> {
    std::fs::create_dir_all(server_dir)?;
    std::fs::create_dir_all(client_dir)?;

    let mut config_blob = crate::config_blob::ConfigBlob::new();

    // Validate pack files: auto-register names from content files.
    // Matches 2004scape revalidatePack() → validateConfigPack() in PackFile.ts.
    validate_config_packs(source_dir, pack_dir, verify)?;

    let registry = PackRegistry::load(pack_dir)?;
    let constants = load_constants(source_dir);
    info!("  Loaded {} constants", constants.len());

    category::pack_categories(&registry)?.save(server_dir, "category")?;

    // Macro: skip if output is up-to-date (matching 2004scape shouldBuild pattern).
    macro_rules! pack_if_needed {
        ($ext:expr, $label:expr, $call:expr) => {
            let out = server_dir.join(concat!($ext, ".dat"));
            if force || should_build(source_dir, $ext, &out) {
                info!("Packing {} configs...", $label);
                $call;
            } else {
                tracing::debug!("{} up to date, skipping", $label);
            }
        };
    }

    // Pack in dependency order (matching 2004scape packConfigs):
    pack_if_needed!(
        "param",
        "param",
        param::pack_params(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );
    pack_if_needed!(
        "dbtable",
        "dbtable",
        dbtable::pack_dbtables(source_dir, &registry, &constants, server_dir)?
    );
    pack_if_needed!(
        "dbrow",
        "dbrow",
        dbrow::pack_dbrows(source_dir, &registry, &constants, server_dir)?
    );
    pack_if_needed!(
        "enum",
        "enum",
        enum_pack::pack_enums(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );
    pack_if_needed!(
        "inv",
        "inv",
        inv::pack_invs(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );
    pack_if_needed!(
        "mesanim",
        "mesanim",
        mesanim::pack_mesanims(source_dir, &registry, &constants, server_dir)?
    );
    pack_if_needed!(
        "struct",
        "struct",
        struct_pack::pack_structs(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );
    pack_if_needed!(
        "seq",
        "seq",
        seq::pack_seqs(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );
    pack_if_needed!(
        "loc",
        "loc",
        loc::pack_locs(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );
    pack_if_needed!(
        "npc",
        "npc",
        npc::pack_npcs(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );
    pack_if_needed!(
        "obj",
        "obj",
        obj::pack_objs(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );
    pack_if_needed!(
        "varp",
        "varp",
        varp::pack_varps(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );
    pack_if_needed!(
        "hunt",
        "hunt",
        hunt::pack_hunts(source_dir, &registry, &constants, server_dir)?
    );
    pack_if_needed!(
        "varn",
        "varn",
        varn::pack_varns(source_dir, &registry, &constants, server_dir)?
    );
    pack_if_needed!(
        "vars",
        "vars",
        vars::pack_vars_configs(source_dir, &registry, &constants, server_dir)?
    );

    // Transmitted-only types (client cache, no server overlays)
    pack_if_needed!(
        "flo",
        "flo",
        flo::pack_flos(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );
    pack_if_needed!(
        "flu",
        "flu",
        flu::pack_flus(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );
    pack_if_needed!(
        "varbit",
        "varbit",
        varbit::pack_varbits(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );
    pack_if_needed!(
        "spotanim",
        "spotanim",
        spotanim::pack_spotanims(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );
    pack_if_needed!(
        "idk",
        "idk",
        idk::pack_idks(
            source_dir,
            &registry,
            &constants,
            server_dir,
            &mut config_blob
        )?
    );

    // Generic config types — client-only, no server overlays.
    // Known types (with client-src decoders):
    for &type_name in &[
        "light",
        "bas",
        "cursor",
        "msi",
        "quest",
        "mel",
        "hitmark",
        "varclansetting",
    ] {
        let all_file = source_dir.join("_unpack").join(format!("all.{type_name}"));
        if all_file.exists() {
            info!("Packing {type_name} (generic)...");
            config_generic::pack_generic_config(source_dir, type_name, &mut config_blob)?;
        }
    }
    // Unknown groups (raw binary passthrough):
    for group_id in CONFIG_UNKNOWN_GROUPS {
        let type_name = format!("config{group_id}");
        let all_file = source_dir.join("_unpack").join(format!("all.{type_name}"));
        if all_file.exists() {
            info!("Packing {type_name} (raw)...");
            config_generic::pack_generic_config(source_dir, &type_name, &mut config_blob)?;
        }
    }

    // Save the config blob as a single file
    config_blob.save(&client_dir.join("config"))?;

    info!("Pack complete.");
    Ok(())
}

/// Discover all files with a given extension recursively under a directory.
/// Returns sorted paths for deterministic output.
pub fn collect_config_files(dir: &Path, ext: &str) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    collect_recursive(dir, ext, &mut files);
    files.sort();
    files
}

fn collect_recursive(dir: &Path, ext: &str, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut sorted: Vec<std::path::PathBuf> = entries.flatten().map(|e| e.path()).collect();
    sorted.sort();
    for path in sorted {
        if path.is_dir() {
            collect_recursive(&path, ext, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some(ext) {
            out.push(path);
        }
    }
}

/// Config pack type: transmitted (client cache) vs server-only.
/// Matches 2004scape's `transmitted` parameter in `validateConfigPack()`.
struct ConfigPackDef {
    ext: &'static str,
    /// True if this type is transmitted to the client (IDs must be stable
    /// across cache versions). In verify mode, missing IDs are errors.
    /// In non-verify mode, auto-registration still happens.
    transmitted: bool,
}

/// All config types, matching 2004scape PackFile.ts declarations.
const CONFIG_PACKS: &[ConfigPackDef] = &[
    // Server-only (never transmitted to client)
    ConfigPackDef {
        ext: "hunt",
        transmitted: false,
    },
    ConfigPackDef {
        ext: "varn",
        transmitted: false,
    },
    ConfigPackDef {
        ext: "vars",
        transmitted: false,
    },
    ConfigPackDef {
        ext: "mesanim",
        transmitted: false,
    },
    ConfigPackDef {
        ext: "struct",
        transmitted: false,
    },
    ConfigPackDef {
        ext: "param",
        transmitted: false,
    },
    ConfigPackDef {
        ext: "inv",
        transmitted: false,
    },
    ConfigPackDef {
        ext: "enum",
        transmitted: false,
    },
    ConfigPackDef {
        ext: "dbtable",
        transmitted: false,
    },
    ConfigPackDef {
        ext: "dbrow",
        transmitted: false,
    },
    // Transmitted to client (IDs must match cache)
    ConfigPackDef {
        ext: "npc",
        transmitted: true,
    },
    ConfigPackDef {
        ext: "obj",
        transmitted: true,
    },
    ConfigPackDef {
        ext: "loc",
        transmitted: true,
    },
    ConfigPackDef {
        ext: "seq",
        transmitted: true,
    },
    ConfigPackDef {
        ext: "varp",
        transmitted: true,
    },
    ConfigPackDef {
        ext: "spotanim",
        transmitted: true,
    },
    ConfigPackDef {
        ext: "idk",
        transmitted: true,
    },
    ConfigPackDef {
        ext: "flo",
        transmitted: true,
    },
];

/// Validate pack files against content.
///
/// - **Non-transmitted types** (server-only: hunt, varn, vars, mesanim,
///   struct, param, inv, controller, …): auto-register any `[name]` not
///   in pack. The server is the source of truth for these ids.
/// - **Transmitted types** (npc, obj, loc, seq, …): NEVER auto-register.
///   Their ids must match the client cache, so inventing ids past the
///   cache range produces silently-broken state. Missing names log a
///   warning; in `verify` mode they error.
/// - In verify mode: also error on orphaned names (in pack but not
///   referenced by any content file) for transmitted types.
fn validate_config_packs(source_dir: &Path, pack_dir: &Path, verify: bool) -> Result<()> {
    for def in CONFIG_PACKS {
        let ext = def.ext;
        let pack_path = pack_dir.join(format!("{ext}.pack"));
        let mut pack = crate::pack_registry::PackFile::load(&pack_path)?;

        // Crawl all [name] sections from content files
        let config_names = crawl_config_names(source_dir, ext);

        // Auto-register only for non-transmitted (server-only) types.
        // Transmitted-type ids are bound to the cache and must come from
        // `rs-pack discover symbols/npc`, never from validate.
        if !def.transmitted {
            let mut added = 0u32;
            for name in &config_names {
                if pack.get_by_name(name).is_none() && !name.starts_with("cert_") {
                    let new_id = pack.max;
                    pack.id_to_name.insert(new_id, name.to_string());
                    pack.name_to_id.insert(name.to_string(), new_id);
                    pack.max = new_id + 1;
                    added += 1;
                }
            }
            if added > 0 {
                save_pack_file(&pack_path, &pack)?;
                info!(
                    "  {ext}.pack: registered {added} new names (total: {})",
                    pack.max
                );
            }
        }

        // Missing names: error in verify mode, warn otherwise (transmitted types only —
        // non-transmitted already auto-registered above).
        if def.transmitted {
            let mut missing: Vec<&str> = Vec::new();
            for name in &config_names {
                if pack.get_by_name(name).is_none() && !name.starts_with("cert_") {
                    missing.push(name);
                }
            }
            if !missing.is_empty() {
                let list = missing
                    .iter()
                    .take(10)
                    .map(|n| format!("  [{n}]"))
                    .collect::<Vec<_>>()
                    .join("\n");
                if verify {
                    anyhow::bail!(
                        "{ext}.pack: {} config(s) missing pack IDs:\n{list}\n\
                         Run `rs-pack discover {ext}` (or symbols) to repopulate, \
                         then edit content/pack/{ext}.pack if needed.",
                        missing.len()
                    );
                } else {
                    tracing::warn!(
                        "{ext}.pack: {} config name(s) missing from pack — entries will be skipped at pack time:\n{list}",
                        missing.len()
                    );
                }
            }
        }

        // Verify transmitted: no orphaned IDs (names in pack not found in content)
        if verify && def.transmitted {
            let config_set: std::collections::HashSet<&str> =
                config_names.iter().map(|s| s.as_str()).collect();
            let mut orphaned: Vec<&str> = Vec::new();
            for name in pack.name_to_id.keys() {
                if !config_set.contains(name.as_str()) && !name.starts_with("cert_") {
                    orphaned.push(name);
                }
            }
            if !orphaned.is_empty() {
                tracing::warn!(
                    "{ext}.pack: {} orphaned name(s) not found in any .{ext} file (first 5: {:?})",
                    orphaned.len(),
                    &orphaned[..orphaned.len().min(5)]
                );
            }
        }
    }
    Ok(())
}

/// Check if any source file with the given extension is newer than the output file.
/// Matches 2004scape `shouldBuild(path, ext, out)`.
/// Returns true if output doesn't exist or any source is newer.
pub fn should_build(source_dir: &Path, ext: &str, output_file: &Path) -> bool {
    let Ok(out_meta) = std::fs::metadata(output_file) else {
        return true; // output doesn't exist
    };
    let Ok(out_mtime) = out_meta.modified() else {
        return true;
    };

    // A rebuilt server can change *packer logic* (e.g. enum symbol resolution)
    // with no change to content files; treat the binary as an input so the .dat
    // is rebuilt after a rebuild, not just after a content edit.
    if let Some(exe_mtime) = current_exe_mtime() {
        if exe_mtime > out_mtime {
            return true;
        }
    }

    let files = collect_config_files(source_dir, ext);
    for path in &files {
        if let Ok(meta) = std::fs::metadata(path) {
            if let Ok(src_mtime) = meta.modified() {
                if src_mtime > out_mtime {
                    return true;
                }
            }
        }
    }
    false
}

/// Modification time of the running executable. Folded into the up-to-date
/// inputs so a rebuilt binary (changed *packer logic*, not content) forces a
/// repack. (Inlined from the source repo's `startup::current_exe_mtime`.)
fn current_exe_mtime() -> Option<std::time::SystemTime> {
    std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::metadata(p).ok())
        .and_then(|m| m.modified().ok())
}

/// Compute CRC32 of a byte slice. Used for verifying packed output matches
/// known-good values (matching 2004scape `Packet.checkcrc()`).
pub fn crc32(data: &[u8]) -> i32 {
    rs_io::crc::getcrc(data, 0, data.len())
}

/// Crawl all `[name]` sections from config files of a given extension.
/// Matches 2004scape `crawlConfigNames()`.
fn crawl_config_names(source_dir: &Path, ext: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let files = collect_config_files(source_dir, ext);
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for line in text.lines() {
            let line = line.split("//").next().unwrap_or("").trim();
            if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                if seen.insert(name.to_string()) {
                    names.push(name.to_string());
                }
            }
        }
    }
    names
}

/// Write a pack file back to disk (sorted by ID).
fn save_pack_file(path: &Path, pack: &crate::pack_registry::PackFile) -> Result<()> {
    let mut lines: Vec<(u32, &str)> = pack
        .id_to_name
        .iter()
        .map(|(&id, name)| (id, name.as_str()))
        .collect();
    lines.sort_by_key(|(id, _)| *id);
    let content: String = lines
        .iter()
        .map(|(id, name)| format!("{id}={name}\n"))
        .collect();
    std::fs::write(path, content)?;
    Ok(())
}

/// Load all `.constant` files from a directory tree and return name→value map.
/// Constants are referenced as `^NAME` in config values.
pub fn load_constants(source_dir: &Path) -> HashMap<String, String> {
    let mut constants = HashMap::new();
    let files = collect_config_files(source_dir, "constant");
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for line in text.lines() {
            let line = line.split("//").next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if let Some((name, value)) = line.split_once('=') {
                let name = name.trim().strip_prefix('^').unwrap_or(name.trim());
                constants.insert(name.to_string(), value.trim().to_string());
            }
        }
    }
    constants
}

/// Parse all config sections from a set of text files.
/// Returns name → Vec<(key, value)> for each `[section_name]`.
///
/// Matching 2004scape `readConfigs()`:
/// - Constants (`^NAME`) are substituted in values
/// - Within a single file, duplicate `[name]` sections are an error (logged and skipped)
///
/// At 647 a section can also come from two layers: the cache's, unpacked into
/// `_unpack/all.*` and never edited by hand, and an authored file. They are
/// merged by [`overlay_section`]. Two *authored* files defining one section is
/// still a hard error: neither would be the obvious winner.
pub fn parse_config_sections(
    files: &[std::path::PathBuf],
    constants: &HashMap<String, String>,
) -> HashMap<String, Vec<(String, String)>> {
    // One map per layer, so an authored overlay can land on the cache's
    // section whichever order the files are read in.
    let mut layers: [HashMap<String, Vec<(String, String)>>; 2] = Default::default();
    let mut section_source: [HashMap<String, std::path::PathBuf>; 2] = Default::default();

    for path in files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let layer = usize::from(!is_unpacked(path));
        let mut current_name: Option<String> = None;
        let mut seen_in_file: std::collections::HashSet<String> = std::collections::HashSet::new();

        for (line_no, raw_line) in text.lines().enumerate() {
            let line = raw_line.trim_start().trim_end_matches('\r');
            let line = if let Some(idx) = find_comment_start(line) {
                &line[..idx]
            } else {
                line
            };
            let trimmed_for_header = line.trim_end();
            if trimmed_for_header.is_empty() {
                continue;
            }

            if let Some(name) = trimmed_for_header
                .strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
            {
                if !seen_in_file.insert(name.to_string()) {
                    tracing::warn!(
                        "{}:{}: Duplicate config [{}] in same file",
                        path.display(),
                        line_no + 1,
                        name
                    );
                }
                if let Some(prev_path) = section_source[layer].get(name) {
                    if prev_path != path {
                        panic!(
                            "Duplicate config [{}] across files:\n  first: {}\n  second: {}:{}\n\
                             Two authored files cannot define one section; merge them.",
                            name,
                            prev_path.display(),
                            path.display(),
                            line_no + 1,
                        );
                    }
                }
                current_name = Some(name.to_string());
                section_source[layer].insert(name.to_string(), path.clone());
                layers[layer].insert(name.to_string(), Vec::new());
                continue;
            }

            if let Some((key, raw_value)) = line.split_once('=') {
                if let Some(ref name) = current_name {
                    let value = substitute_constants(raw_value, constants);
                    layers[layer]
                        .get_mut(name)
                        .unwrap()
                        .push((key.trim().to_string(), value));
                }
            }
        }
    }

    let [mut configs, authored] = layers;
    for (name, overlay) in authored {
        let merged = match configs.remove(&name) {
            Some(base) => overlay_section(base, overlay),
            None => overlay,
        };
        configs.insert(name, merged);
    }
    configs
}

/// Is `path` cache output (`.../_unpack/...`) rather than authored content?
fn is_unpacked(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == "_unpack")
}

/// Merge an authored section onto the cache's section of the same name.
///
/// * An authored section that restates **every** key the cache's has is a
///   full definition and replaces it, in its own line order -- e.g. the
///   woodcutting trees in `skill_woodcutting/configs/trees/`.
/// * Otherwise it is a patch. Each key it names replaces all of the cache's
///   lines for that key, at the position of the first one; keys the cache
///   does not have are appended. That is how server-only data the cache
///   cannot carry -- a barrier's `category`, an obj's `wearpos` -- sits on top
///   of a cache definition without the cache file ever being edited.
///
/// Neither form can delete a key, and no authored content needs to.
pub fn overlay_section(
    base: Vec<(String, String)>,
    overlay: Vec<(String, String)>,
) -> Vec<(String, String)> {
    use std::collections::HashSet;
    let named: HashSet<&str> = overlay.iter().map(|(k, _)| k.as_str()).collect();
    if base.iter().all(|(k, _)| named.contains(k.as_str())) {
        return overlay;
    }
    let mut out = Vec::with_capacity(base.len() + overlay.len());
    let mut placed: HashSet<String> = HashSet::new();
    for (key, value) in &base {
        if !named.contains(key.as_str()) {
            out.push((key.clone(), value.clone()));
        } else if placed.insert(key.clone()) {
            out.extend(overlay.iter().filter(|(k, _)| k == key).cloned());
        }
    }
    out.extend(overlay.iter().filter(|(k, _)| !placed.contains(k)).cloned());
    out
}

/// Find the start index of a `//` line comment, preserving `//` inside
/// values. Returns `Some(idx)` if the line has a comment to strip, or `None`
/// to keep the line as-is.
///
/// Heuristic: a `//` is a comment iff it's at the start of the line (after
/// indentation, which the caller already stripped) OR preceded by whitespace.
/// This lets `val=k,http://...` survive while still stripping
/// `key=value  // note`.
fn find_comment_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'/' {
            if i == 0 || bytes[i - 1].is_ascii_whitespace() {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Replace `^CONSTANT_NAME` references in a value string.
fn substitute_constants(value: &str, constants: &HashMap<String, String>) -> String {
    if !value.contains('^') {
        return value.to_string();
    }
    let mut result = value.to_string();
    // Simple single-pass: find ^word and replace
    while let Some(start) = result.find('^') {
        let rest = &result[start + 1..];
        let end = rest
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        let name = &rest[..end];
        if let Some(replacement) = constants.get(name) {
            result = format!("{}{}{}", &result[..start], replacement, &rest[end..]);
        } else {
            break;
        }
    }
    result
}

/// Reorder emitted properties to match the original opcode decode order.
///
/// `opcode_order` is the sequence of opcodes as they appeared in the binary.
/// `props` is a list of (opcode, key, value) triples built by the emitter.
/// Returns just (key, value) pairs in the correct order.
///
/// If `opcode_order` is empty (hand-written config), preserves the original
/// insertion order (which is the fixed emitter order).
pub fn reorder_by_opcode(
    opcode_order: &[u8],
    props: Vec<(u8, String, String)>,
) -> Vec<(String, String)> {
    reorder_by_opcode_with_raw(opcode_order, props, &[])
}

/// Reorder emitted properties to match the original opcode decode order,
/// using raw_opcodes to reconstruct earlier values for duplicate opcodes.
///
/// When an opcode appears N times in `opcode_order` but the emitter only
/// produced M entries (because the struct field stores only the last value),
/// the first (N-M) occurrences are reconstructed from `raw_opcodes`.
/// The emitter's entries are used for the LAST occurrence.
///
/// Uses the generic `decode_raw_to_text` decoder. For type-specific decoding,
/// use `reorder_by_opcode_with_raw_fn`.
pub fn reorder_by_opcode_with_raw(
    opcode_order: &[u8],
    props: Vec<(u8, String, String)>,
    raw_opcodes: &[(u8, Vec<u8>)],
) -> Vec<(String, String)> {
    reorder_by_opcode_with_raw_fn(opcode_order, props, raw_opcodes, decode_raw_to_text)
}

/// Reorder emitted properties using a custom raw-to-text decoder function.
pub fn reorder_by_opcode_with_raw_fn(
    opcode_order: &[u8],
    props: Vec<(u8, String, String)>,
    raw_opcodes: &[(u8, Vec<u8>)],
    decoder: fn(u8, &[u8]) -> Option<(String, String)>,
) -> Vec<(String, String)> {
    if opcode_order.is_empty() {
        return props.into_iter().map(|(_, k, v)| (k, v)).collect();
    }

    // Group emitted props by opcode, preserving insertion order within each group.
    let mut by_op: HashMap<u8, Vec<(String, String)>> = HashMap::new();
    for (op, k, v) in props {
        by_op.entry(op).or_default().push((k, v));
    }

    // Count how many times each opcode appears in the original order.
    let mut op_count_in_order: HashMap<u8, usize> = HashMap::new();
    for &op in opcode_order {
        *op_count_in_order.entry(op).or_default() += 1;
    }

    // For opcodes that appear more times in opcode_order than the emitter
    // produced, build raw-decoded text entries from raw_opcodes.
    // The emitter's entries correspond to the LAST occurrence (since struct
    // fields store the last decoded value). We need entries for the earlier
    // occurrences from raw_opcodes.
    let mut raw_extras: HashMap<u8, Vec<(String, String)>> = HashMap::new();
    if !raw_opcodes.is_empty() {
        for (&op, &needed) in &op_count_in_order {
            let have = by_op.get(&op).map(|v| v.len()).unwrap_or(0);
            if needed > have {
                // Collect all raw entries for this opcode
                let raw_entries: Vec<&Vec<u8>> = raw_opcodes
                    .iter()
                    .filter(|(o, _)| *o == op)
                    .map(|(_, data)| data)
                    .collect();
                // The emitter produced `have` entries using the struct's LAST
                // value. The raw_entries list has ALL occurrences in order.
                // The emitter's entries correspond to raw_entries[needed-have..].
                // We need raw_entries[0..needed-have] as extras.
                let extras_needed = needed - have;
                let mut extras = Vec::new();
                for i in 0..extras_needed.min(raw_entries.len()) {
                    if let Some(text) = decoder(op, raw_entries[i]) {
                        extras.push(text);
                    }
                }
                if !extras.is_empty() {
                    raw_extras.insert(op, extras);
                }
            }
        }
    }

    // Build queues: raw extras first (for earlier occurrences), then emitter entries.
    let mut by_op_queues: HashMap<u8, std::collections::VecDeque<(String, String)>> =
        HashMap::new();
    for (op, entries) in &by_op {
        by_op_queues.insert(*op, entries.iter().cloned().collect());
    }
    for (op, extras) in &raw_extras {
        let queue = by_op_queues.entry(*op).or_default();
        for (i, entry) in extras.iter().enumerate() {
            queue.insert(i, entry.clone());
        }
    }

    // For each opcode, decide how many text entries to drain per occurrence.
    // - If queue.len() == occurrences in opcode_order: 1 entry per occurrence
    //   (duplicate opcode or multi-instance like seq op19/op20).
    // - If queue.len() > occurrences: batch opcode (e.g. recol produces
    //   recol1s, recol1d, recol2s, recol2d for a single op40 emission).
    //   Compute entries_per = queue.len() / occurrences (integer division).
    //   Drain `entries_per` entries per occurrence, remainder on last.
    let mut result = Vec::new();
    let mut op_occurrence: HashMap<u8, usize> = HashMap::new();

    for &op in opcode_order {
        let queue = match by_op_queues.get_mut(&op) {
            Some(q) if !q.is_empty() => q,
            _ => continue,
        };

        let occ_idx = op_occurrence.entry(op).or_insert(0);
        let total_occ = *op_count_in_order.get(&op).unwrap_or(&1);
        let remaining_occ = total_occ - *occ_idx;

        if remaining_occ <= 1 {
            // Last (or only) occurrence: drain everything remaining
            result.extend(queue.drain(..));
        } else if queue.len() == remaining_occ {
            // 1:1: one entry per remaining occurrence
            if let Some(entry) = queue.pop_front() {
                result.push(entry);
            }
        } else if queue.len() > remaining_occ {
            // Batch: multiple entries per occurrence.
            // Drain a proportional share, rounding down (last gets remainder).
            let per = queue.len() / remaining_occ;
            for _ in 0..per {
                if let Some(entry) = queue.pop_front() {
                    result.push(entry);
                }
            }
        } else {
            // Fewer entries than occurrences (shouldn't happen with raw extras)
            if let Some(entry) = queue.pop_front() {
                result.push(entry);
            }
        }

        *occ_idx += 1;
    }

    result
}

/// Decode raw opcode bytes back to a (key, value) text pair.
/// This handles the common single-value opcodes that appear as duplicates.
/// Returns None for complex/batch opcodes that can't be trivially decoded.
pub fn decode_raw_to_text(opcode: u8, raw: &[u8]) -> Option<(String, String)> {
    // Helper to read big-endian u16
    let r_u16 = |data: &[u8], pos: usize| -> u16 {
        if pos + 1 < data.len() {
            u16::from_be_bytes([data[pos], data[pos + 1]])
        } else {
            0
        }
    };
    let r_i8 =
        |data: &[u8], pos: usize| -> i8 { if pos < data.len() { data[pos] as i8 } else { 0 } };
    let r_u8 = |data: &[u8], pos: usize| -> u8 { if pos < data.len() { data[pos] } else { 0 } };
    let _r_i32 = |data: &[u8], pos: usize| -> i32 {
        if pos + 3 < data.len() {
            i32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
        } else {
            0
        }
    };
    let r_i16 = |data: &[u8], pos: usize| -> i16 {
        if pos + 1 < data.len() {
            i16::from_be_bytes([data[pos], data[pos + 1]])
        } else {
            0
        }
    };
    // Read NUL-terminated string
    let r_str = |data: &[u8], pos: usize| -> String {
        let mut end = pos;
        while end < data.len() && data[end] != 0 {
            end += 1;
        }
        String::from_utf8_lossy(&data[pos..end]).to_string()
    };

    match opcode {
        // ── NPC opcodes ──
        // op2: name (string)
        2 => Some(("name".into(), r_str(raw, 0))),
        // op12: size (u8)
        12 => Some(("size".into(), r_u8(raw, 0).to_string())),
        // op30..34: op1..op5 (string) — slot encoded in opcode
        30..=34 => Some((format!("op{}", opcode - 29), r_str(raw, 0))),
        // op93: minimap=no (flag, no payload)
        93 => Some(("minimap".into(), "no".into())),
        // op95: vislevel (u16)
        95 => {
            let v = r_u16(raw, 0);
            if v == 0 {
                Some(("vislevel".into(), "hide".into()))
            } else {
                Some(("vislevel".into(), v.to_string()))
            }
        }
        // op97: resizeh (u16)
        97 => Some(("resizeh".into(), r_u16(raw, 0).to_string())),
        // op98: resizev (u16)
        98 => Some(("resizev".into(), r_u16(raw, 0).to_string())),
        // op99: alwaysontop (flag)
        99 => Some(("alwaysontop".into(), "yes".into())),
        // op100: ambient (i8)
        100 => Some(("ambient".into(), r_i8(raw, 0).to_string())),
        // op101: contrast (i8, raw byte not * 5)
        101 => Some(("contrast".into(), r_i8(raw, 0).to_string())),
        // op102: headicon (u16)
        102 => Some(("headicon".into(), r_u16(raw, 0).to_string())),
        // op103: turnspeed (u16)
        103 => Some(("turnspeed".into(), r_u16(raw, 0).to_string())),

        // ── LOC opcodes ──
        // op14: width (u8)
        14 => Some(("width".into(), r_u8(raw, 0).to_string())),
        // op15: length (u8)
        15 => Some(("length".into(), r_u8(raw, 0).to_string())),
        // op17: blockwalk=no (flag)
        17 => Some(("blockwalk".into(), "no".into())),
        // op18: blockrange=no (flag)
        18 => Some(("blockrange".into(), "no".into())),
        // op19: active (u8) — Note: for LOC this is a single u8, for SEQ it's multi-instance
        19 => Some(("active".into(), r_u8(raw, 0).to_string())),
        // op21: hillskew=yes (flag)
        21 => Some(("hillskew".into(), "yes".into())),
        // op24: anim (u16, 0xFFFF = null)
        24 => {
            let v = r_u16(raw, 0);
            if v == 0xFFFF {
                Some(("anim".into(), "null".into()))
            } else {
                Some(("anim".into(), v.to_string()))
            }
        }
        // op29: ambient (i8) — LOC version
        29 => Some(("ambient".into(), r_i8(raw, 0).to_string())),
        // op39: contrast (i8) — LOC version
        39 => Some(("contrast".into(), r_i8(raw, 0).to_string())),
        // op62: mirror (flag)
        62 => Some(("mirror".into(), "yes".into())),
        // op64: shadow=no (flag)
        64 => Some(("shadow".into(), "no".into())),
        // op65: resizex (u16)
        65 => Some(("resizex".into(), r_u16(raw, 0).to_string())),
        // op66: resizey (u16)
        66 => Some(("resizey".into(), r_u16(raw, 0).to_string())),
        // op67: resizez (u16)
        67 => Some(("resizez".into(), r_u16(raw, 0).to_string())),
        // op69: forceapproach (u8)
        69 => Some(("forceapproach".into(), r_u8(raw, 0).to_string())),
        // op70: offsetx (u16)
        70 => Some(("offsetx".into(), r_u16(raw, 0).to_string())),
        // op71: offsety (u16)
        71 => Some(("offsety".into(), r_u16(raw, 0).to_string())),
        // op72: offsetz (u16)
        72 => Some(("offsetz".into(), r_u16(raw, 0).to_string())),
        // op73: forcedecor (flag)
        73 => Some(("forcedecor".into(), "yes".into())),
        // op74: breakroutefinding (flag)
        74 => Some(("breakroutefinding".into(), "yes".into())),
        // op75: raiseobject (u8)
        75 => Some(("raiseobject".into(), r_u8(raw, 0).to_string())),

        // ── OBJ opcodes ──
        // op1: model (u16) — for OBJ
        1 => Some(("model".into(), r_u16(raw, 0).to_string())),
        // op4: 2dzoom (u16)
        4 => Some(("2dzoom".into(), r_u16(raw, 0).to_string())),
        // op5: 2dxan (u16)
        5 => Some(("2dxan".into(), r_u16(raw, 0).to_string())),
        // op6: 2dyan (u16)
        6 => Some(("2dyan".into(), r_u16(raw, 0).to_string())),
        // op7: 2dxof (i16)
        7 => Some(("2dxof".into(), r_i16(raw, 0).to_string())),
        // op8: 2dyof (i16)
        8 => Some(("2dyof".into(), r_i16(raw, 0).to_string())),
        // op11: stackable (flag)
        11 => Some(("stackable".into(), "yes".into())),
        // op16: members (flag)
        16 => Some(("members".into(), "yes".into())),
        // op23: manwear (u16)
        23 => Some(("manwear".into(), r_u16(raw, 0).to_string())),
        // op25: womanwear (u16)
        25 => Some(("womanwear".into(), r_u16(raw, 0).to_string())),

        // ── Shared opcodes with i32 payload ──
        // op12 for OBJ: cost (i32) — conflicts with NPC op12 (size u8)
        // We can't distinguish here; the caller knows the type.
        // For safety, these ambiguous opcodes should be handled by
        // the type-specific emitter, not here.

        // ── LOC cursor opcodes ──
        // op99 (LOC): cursor1 (u8, u16) — conflicts with NPC op99 (alwaysontop flag)
        // op100 (LOC): cursor2 (u8, u16) — conflicts with NPC op100 (ambient i8)

        // ── Op with u16 payload ──
        // op101 (LOC): mapfunction (u8) — shadowed by NPC op101 (contrast)
        // above; the type-specific emitter handles LOC mapfunction.
        // op122 (NPC): hitbar (u16)
        122 => Some(("hitbar".into(), r_u16(raw, 0).to_string())),
        // op123 (NPC): height (u16)
        123 => Some(("height".into(), r_u16(raw, 0).to_string())),
        // op125 (NPC): respawndir (i8)
        125 => Some(("respawndir".into(), r_i8(raw, 0).to_string())),
        // op127 (NPC): readyanim (u16)
        127 => Some(("readyanim".into(), r_u16(raw, 0).to_string())),
        // op128 (NPC): unknown128 (u8)
        128 => Some(("unknown128".into(), r_u8(raw, 0).to_string())),

        // ── SEQ opcodes (multi-instance, already handled correctly) ──
        // op19 (SEQ): idx:val pair — already pushed to Vec, handled by emitter
        // op20 (SEQ): idx:a:b triple — already pushed to Vec, handled by emitter

        // ── LOC misc ──
        // op82: hidden (flag)
        82 => Some(("hidden".into(), "yes".into())),
        // op88: nominimap_visible (flag)
        88 => Some(("nominimap_visible".into(), "yes".into())),
        // op89: noblocks_npcs (flag)
        89 => Some(("noblocks_npcs".into(), "yes".into())),
        // op90: op90 (flag)
        90 => Some(("op90".into(), "yes".into())),
        // op91: members_only (flag)
        91 => Some(("members_only".into(), "yes".into())),
        // op104: interaction_sides (u8)
        104 => Some(("interaction_sides".into(), r_u8(raw, 0).to_string())),
        // op105: op105 (flag)
        105 => Some(("op105".into(), "yes".into())),
        // op107: mapelement (u16)
        107 => Some(("mapelement".into(), r_u16(raw, 0).to_string())),

        // op164..167: i16/u16 values
        164 => Some(("op164".into(), r_i16(raw, 0).to_string())),
        165 => Some(("op165".into(), r_i16(raw, 0).to_string())),
        166 => Some(("op166".into(), r_i16(raw, 0).to_string())),
        167 => Some(("op167".into(), r_u16(raw, 0).to_string())),
        // op168..169: flags
        168 => Some(("op168".into(), "yes".into())),
        169 => Some(("op169".into(), "yes".into())),
        // op177: flag
        177 => Some(("op177".into(), "yes".into())),
        // op178: u8
        178 => Some(("op178".into(), r_u8(raw, 0).to_string())),

        // ── Complex/batch opcodes: can't trivially decode ──
        _ => None,
    }
}
