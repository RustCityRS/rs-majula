//! Hunt config packers — two flavours sharing one 2004scape-style text
//! format:
//!
//! 1. **Server cache (legacy)**: [`pack_hunts`] reads `.hunt` text and
//!    writes `data/pack/server/hunt.dat`/`hunt.idx` via `PackedData`. This
//!    is the SERVER cache the engine reads at startup; binary output is
//!    unchanged from prior revisions.
//! 2. **Cache-byte-perfect (Phase 1)**: [`emit_hunt_props`] + [`pack_hunt_def`]
//!    roundtrip through `HuntDefinition` for archive 2 group 2 of the cache.
//!    `pack_hunt_def(...).encode()` produces byte-perfect cache bytes.
//!
//! Both paths read the SAME `.hunt` text format, mirroring 2004scape's
//! `engine/tools/pack/config/HuntConfig.ts`. Hunt is server-only in the
//! reference (no client decoder); we still write a byte-perfect server
//! cache binary for parity with 2004scape's `hunt.dat`.
//!
//! Cross-references resolve through `PackRegistry`: NPC/obj/loc/category
//! get human-readable names; varp (op 8) and varn (op 9) are emitted with
//! a leading `%` prefix per 2004scape convention; on pack the `%` is
//! stripped before lookup.

use std::path::Path;

use crate::cache647::HuntType;
use anyhow::Result;
use tracing::info;

use super::{collect_config_files, parse_config_sections};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

/// Walk a hunt's emissions and produce the (key, value) pairs matching
/// 2004scape's HuntConfig text format. `DebugName` is dropped because
/// the `[section]` header carries it.
///
/// Per-opcode text keys (preserves discriminator identity for ops that
/// would otherwise collide):
/// - op 1 `type` (off/player/npc/obj/scenery)
/// - op 2 `check_vis` (off/lineofsight/lineofwalk)
/// - op 3 `check_nottoostrong` (off/outside_wilderness)
/// - op 4 `check_notbusy=on`
/// - op 5 `find_keephunting=on`
/// - op 6 `find_newmode=<NpcMode_name>`
/// - op 7 `nobodynear` (keephunting/pausehunt)
/// - op 8 `check_notcombat=%<varp>` (varp ref)
/// - op 9 `check_notcombat_self=%<varn>` (varn ref)
/// - op 10 `check_afk=off`
/// - op 11 `rate=<u16>`
/// - op 12 `check_category=<name>`
/// - op 13 `check_npc=<name>`
/// - op 14 `check_obj=<name>`
/// - op 15 `check_loc=<name>`
/// - op 16 `check_inv_obj=<inv>,<obj>,<cond>,<val>`
/// - op 17 `check_inv_param=<inv>,<param>,<cond>,<val>`
/// - op 18/19/20 `check_var_op<N>=<varp>,<cond>,<val>` (N = opcode-18)
pub fn emit_hunt_props(def: &HuntType, registry: &PackRegistry) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();
    if def.hunt_type != 0 {
        let s = match def.hunt_type {
            0 => "off",
            1 => "player",
            2 => "npc",
            3 => "obj",
            4 => "scenery",
            v => {
                props.push(("type".into(), v.to_string()));
                // skip the push below
                ""
            }
        };
        if !s.is_empty() {
            props.push(("type".into(), s.into()));
        }
    }
    if def.check_vis != 0 {
        let s = match def.check_vis {
            0 => "off",
            1 => "lineofsight",
            2 => "lineofwalk",
            v => {
                props.push(("check_vis".into(), v.to_string()));
                ""
            }
        };
        if !s.is_empty() {
            props.push(("check_vis".into(), s.into()));
        }
    }
    if def.check_not_too_strong != 0 {
        let s = match def.check_not_too_strong {
            0 => "off",
            1 => "outside_wilderness",
            v => {
                props.push(("check_nottoostrong".into(), v.to_string()));
                ""
            }
        };
        if !s.is_empty() {
            props.push(("check_nottoostrong".into(), s.into()));
        }
    }
    if def.check_not_busy {
        props.push(("check_notbusy".into(), "on".into()));
    }
    if def.find_keep_hunting {
        props.push(("find_keephunting".into(), "on".into()));
    }
    if def.find_new_mode != 0 {
        let name =
            npc_mode_name(def.find_new_mode).unwrap_or_else(|| def.find_new_mode.to_string());
        props.push(("find_newmode".into(), name));
    }
    if def.nobody_near != 0 {
        let s = match def.nobody_near {
            0 => "keephunting",
            _ => "pausehunt",
        };
        props.push(("nobodynear".into(), s.into()));
    }
    if def.check_not_combat >= 0 {
        let v = def.check_not_combat as u32;
        let name = registry
            .varp
            .get_by_id(v)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("varp_{v}"));
        props.push(("check_notcombat".into(), format!("%{name}")));
    }
    if def.check_not_combat_self >= 0 {
        let v = def.check_not_combat_self as u32;
        let name = registry
            .varn
            .get_by_id(v)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("varn_{v}"));
        props.push(("check_notcombat_self".into(), format!("%{name}")));
    }
    if !def.check_afk {
        props.push(("check_afk".into(), "off".into()));
    }
    if def.rate != 1 {
        props.push(("rate".into(), def.rate.to_string()));
    }
    if def.check_category >= 0 {
        let v = def.check_category as u32;
        let name = registry
            .category
            .get_by_id(v)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("category_{v}"));
        props.push(("check_category".into(), name));
    }
    if def.check_npc >= 0 {
        let v = def.check_npc as u32;
        let name = registry
            .npc
            .get_by_id(v)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("npc_{v}"));
        props.push(("check_npc".into(), name));
    }
    if def.check_obj >= 0 {
        let v = def.check_obj as u32;
        let name = registry
            .obj
            .get_by_id(v)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("obj_{v}"));
        props.push(("check_obj".into(), name));
    }
    if def.check_loc >= 0 {
        let v = def.check_loc as u32;
        let name = registry
            .loc
            .get_by_id(v)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("loc_{v}"));
        props.push(("check_loc".into(), name));
    }
    if def.check_inv_opcode == 16 {
        let inv = registry
            .inv
            .get_by_id(def.check_inv as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("inv_{}", def.check_inv));
        let obj = registry
            .obj
            .get_by_id(def.check_inv_obj as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("obj_{}", def.check_inv_obj));
        props.push((
            "check_inv_obj".into(),
            format!(
                "{inv},{obj},{},{}",
                def.check_inv_condition, def.check_inv_val
            ),
        ));
    }
    if def.check_inv_opcode == 17 {
        let inv = registry
            .inv
            .get_by_id(def.check_inv as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("inv_{}", def.check_inv));
        let param = registry
            .param
            .get_by_id(def.check_obj_param as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("param_{}", def.check_obj_param));
        props.push((
            "check_inv_param".into(),
            format!(
                "{inv},{param},{},{}",
                def.check_inv_condition, def.check_inv_val
            ),
        ));
    }
    for (opcode, var_id, condition, value) in &def.check_vars {
        let n = opcode.saturating_sub(18);
        let var_name = registry
            .varp
            .get_by_id(*var_id as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("varp_{var_id}"));
        props.push((
            format!("check_var_op{n}"),
            format!("{var_name},{condition},{value}"),
        ));
    }
    props
}

// ─── Legacy server-cache packer (data/pack/server/hunt.dat) ──────────────
//
// Same .hunt text format as the cache packer above; binary output is the
// rs-server's startup-loadable PackedData blob. Kept verbatim for
// engine compatibility.
pub fn pack_hunts(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &std::collections::HashMap<String, String>,
    output_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    let files = collect_config_files(source_dir, "hunt");
    info!("  Found {} .hunt files", files.len());

    let configs = parse_config_sections(&files, constants);
    info!("  Parsed {} hunt configs", configs.len());

    // The client's hunt group is the cache's own (`_unpack/all.hunt`, whose
    // entries are empty): content's hunt modes are the server's alone, so
    // they stay out of it, and server-only ones past the cache's last are not
    // in it at all.
    // The cache's sections are empty, and empty sections parse to nothing, so
    // the count comes from the file's section headers.
    let all_file = source_dir.join("_unpack").join("all.hunt");
    let (client_configs, client_count) = if all_file.exists() {
        let count = std::fs::read_to_string(&all_file)?
            .lines()
            .filter_map(|l| l.strip_prefix('[')?.strip_suffix(']'))
            .filter_map(|name| registry.hunt.get_by_name(name))
            .max()
            .map_or(0, |m| m + 1);
        (parse_config_sections(&[all_file], constants), count)
    } else {
        (std::collections::HashMap::new(), registry.hunt.max)
    };

    let mut packed = PackedData::new(registry.hunt.max);
    let mut client = PackedData::new(client_count);

    for id in 0..registry.hunt.max {
        let name = registry.hunt.get_by_id(id);
        packed.start_entry();
        if let Some(props) = name.and_then(|n| configs.get(n)) {
            encode_hunt(props, registry, &mut packed);
            packed.p1(250);
            packed.pjstr(name.unwrap());
        }
        packed.finish_entry();

        if id < client_count {
            client.start_entry();
            if let Some(props) = name.and_then(|n| client_configs.get(n)) {
                encode_hunt(props, registry, &mut client);
            }
            client.finish_entry();
        }
    }

    packed.save(output_dir, "hunt")?;
    config_blob.add_packed_data("hunt", client.dat, client.idx);
    Ok(())
}

/// One hunt config's opcodes, in 2004scape's `HuntConfig` order.
fn encode_hunt(props: &[(String, String)], registry: &PackRegistry, out: &mut PackedData) {
    for (key, value) in props {
        match key.as_str() {
            "type" => {
                let v = match value.as_str() {
                    "player" => 1u8,
                    "npc" => 2,
                    "obj" => 3,
                    "scenery" => 4,
                    _ => 0, // off
                };
                if v != 0 {
                    out.p1(1);
                    out.p1(v);
                }
            }
            "check_vis" => {
                let v = match value.as_str() {
                    "lineofsight" => 1u8,
                    "lineofwalk" => 2,
                    _ => 0,
                };
                if v != 0 {
                    out.p1(2);
                    out.p1(v);
                }
            }
            "check_nottoostrong" => {
                let v = match value.as_str() {
                    "outside_wilderness" => 1u8,
                    _ => 0,
                };
                if v != 0 {
                    out.p1(3);
                    out.p1(v);
                }
            }
            "check_notbusy" => {
                if value == "on" {
                    out.p1(4);
                }
            }
            "find_keephunting" => {
                if value == "on" {
                    out.p1(5);
                }
            }
            "find_newmode" => {
                let mode = parse_npc_mode(value);
                if mode != 0 {
                    out.p1(6);
                    out.p1(mode);
                }
            }
            "nobodynear" => {
                let v = match value.as_str() {
                    "keephunting" => 0u8,
                    _ => 1, // pausehunt
                };
                out.p1(7);
                out.p1(v);
            }
            "check_notcombat" => {
                let varp_id = resolve_varp(value, registry);
                if varp_id >= 0 {
                    out.p1(8);
                    out.p2(varp_id as u16);
                }
            }
            "check_notcombat_self" => {
                let varn_id = resolve_varn(value, registry);
                if varn_id >= 0 {
                    out.p1(9);
                    out.p2(varn_id as u16);
                }
            }
            "check_afk" => {
                if value == "off" {
                    out.p1(10);
                }
            }
            "rate" => {
                if let Ok(rate) = value.parse::<u16>() {
                    if rate != 1 {
                        out.p1(11);
                        out.p2(rate);
                    }
                }
            }
            "check_category" => {
                if let Some(cat_id) = registry.category.get_by_name(value) {
                    out.p1(12);
                    out.p2(cat_id as u16);
                }
            }
            "check_npc" => {
                if let Some(npc_id) = registry.npc.get_by_name(value) {
                    out.p1(13);
                    out.p2(npc_id as u16);
                }
            }
            "check_obj" => {
                if let Some(obj_id) = registry.obj.get_by_name(value) {
                    out.p1(14);
                    out.p2(obj_id as u16);
                }
            }
            "check_loc" => {
                if let Some(loc_id) = registry.loc.get_by_name(value) {
                    out.p1(15);
                    out.p2(loc_id as u16);
                }
            }
            _ => {}
        }
    }
}

fn resolve_varp(value: &str, registry: &PackRegistry) -> i32 {
    if let Some(name) = value.strip_prefix('%') {
        registry
            .varp
            .get_by_name(name)
            .map(|id| id as i32)
            .unwrap_or(-1)
    } else {
        value.parse().unwrap_or(-1)
    }
}

fn resolve_varn(value: &str, registry: &PackRegistry) -> i32 {
    if let Some(name) = value.strip_prefix('%') {
        registry
            .varn
            .get_by_name(name)
            .map(|id| id as i32)
            .unwrap_or(-1)
    } else {
        value.parse().unwrap_or(-1)
    }
}

fn parse_npc_mode(value: &str) -> u8 {
    match value {
        "none" | "null" => 0,
        "wander" => 1,
        "patrol" => 2,
        "playerescape" => 3,
        "playerfollow" => 4,
        "playerface" => 5,
        "playerfaceclose" => 6,
        "opplayer1" => 7,
        "opplayer2" => 8,
        "opplayer3" => 9,
        "opplayer4" => 10,
        "opplayer5" => 11,
        "applayer1" => 12,
        "applayer2" => 13,
        "applayer3" => 14,
        "applayer4" => 15,
        "applayer5" => 16,
        "oploc1" => 17,
        "oploc2" => 18,
        "oploc3" => 19,
        "oploc4" => 20,
        "oploc5" => 21,
        "aploc1" => 22,
        "aploc2" => 23,
        "aploc3" => 24,
        "aploc4" => 25,
        "aploc5" => 26,
        "opobj1" => 27,
        "opobj2" => 28,
        "opobj3" => 29,
        "opobj4" => 30,
        "opobj5" => 31,
        "apobj1" => 32,
        "apobj2" => 33,
        "apobj3" => 34,
        "apobj4" => 35,
        "apobj5" => 36,
        "opnpc1" => 37,
        "opnpc2" => 38,
        "opnpc3" => 39,
        "opnpc4" => 40,
        "opnpc5" => 41,
        "apnpc1" => 42,
        "apnpc2" => 43,
        "apnpc3" => 44,
        "apnpc4" => 45,
        "apnpc5" => 46,
        _ => value.parse().unwrap_or(0),
    }
}

/// Inverse of `parse_npc_mode`. Returns `None` if `v` is out of the known
/// range — emit will then fall back to the literal numeric value.
fn npc_mode_name(v: u8) -> Option<String> {
    let s = match v {
        0 => "none",
        1 => "wander",
        2 => "patrol",
        3 => "playerescape",
        4 => "playerfollow",
        5 => "playerface",
        6 => "playerfaceclose",
        7 => "opplayer1",
        8 => "opplayer2",
        9 => "opplayer3",
        10 => "opplayer4",
        11 => "opplayer5",
        12 => "applayer1",
        13 => "applayer2",
        14 => "applayer3",
        15 => "applayer4",
        16 => "applayer5",
        17 => "oploc1",
        18 => "oploc2",
        19 => "oploc3",
        20 => "oploc4",
        21 => "oploc5",
        22 => "aploc1",
        23 => "aploc2",
        24 => "aploc3",
        25 => "aploc4",
        26 => "aploc5",
        27 => "opobj1",
        28 => "opobj2",
        29 => "opobj3",
        30 => "opobj4",
        31 => "opobj5",
        32 => "apobj1",
        33 => "apobj2",
        34 => "apobj3",
        35 => "apobj4",
        36 => "apobj5",
        37 => "opnpc1",
        38 => "opnpc2",
        39 => "opnpc3",
        40 => "opnpc4",
        41 => "opnpc5",
        42 => "apnpc1",
        43 => "apnpc2",
        44 => "apnpc3",
        45 => "apnpc4",
        46 => "apnpc5",
        _ => return None,
    };
    Some(s.to_string())
}
