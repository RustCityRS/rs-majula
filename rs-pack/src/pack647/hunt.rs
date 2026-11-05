use std::path::Path;

use anyhow::{Context, Result, ensure};
use tracing::info;

use super::{collect_config_files, parse_config_sections};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

pub fn pack_hunts(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &std::collections::HashMap<String, String>,
    output_dir: &Path,
) -> Result<()> {
    let files = collect_config_files(source_dir, "hunt");
    info!("  Found {} .hunt files", files.len());

    let configs = parse_config_sections(&files, constants);
    info!("  Parsed {} hunt configs", configs.len());

    let mut packed = PackedData::new(registry.hunt.max);

    for id in 0..registry.hunt.max {
        let name = registry.hunt.get_by_id(id);
        packed.start_entry();
        if let Some(props) = name.and_then(|n| configs.get(n)) {
            encode_hunt(props, registry, &mut packed)?;
            packed.p1(250);
            packed.pjstr(name.unwrap());
        }
        packed.finish_entry();
    }

    packed.save(output_dir, "hunt")?;
    Ok(())
}

/// One hunt config's opcodes, in 2004scape's `HuntConfig` order.
fn encode_hunt(
    props: &[(String, String)],
    registry: &PackRegistry,
    out: &mut PackedData,
) -> Result<()> {
    let mut extracheck_var = 0;
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
            "check_inv" | "check_invparam" | "check_invcat" => {
                let parts = value.splitn(3, ',').map(str::trim).collect::<Vec<_>>();
                ensure!(parts.len() == 3, "Invalid {key} value: {value}");
                let (opcode, pack) = match key.as_str() {
                    "check_inv" => (16, &registry.obj),
                    "check_invparam" => (17, &registry.param),
                    _ => (18, &registry.category),
                };
                let inv = registry
                    .inv
                    .get_by_name(parts[0])
                    .with_context(|| format!("Unknown inventory '{}'", parts[0]))?;
                let target = pack
                    .get_by_name(parts[1])
                    .with_context(|| format!("Unknown {key} target '{}'", parts[1]))?;
                let (condition, value) = parse_condition(parts[2])?;
                out.p1(opcode);
                out.p2(inv as u16);
                out.p2(target as u16);
                out.pjstr(condition);
                out.p4(value);
            }
            "extracheck_var" => {
                ensure!(extracheck_var < 3, "Too many extracheck_var entries");
                let (var, check) = value
                    .split_once(',')
                    .context("Invalid extracheck_var value")?;
                let var = resolve_varp(var.trim(), registry);
                ensure!(var >= 0, "Unknown extracheck_var variable: {value}");
                let (condition, value) = parse_condition(check.trim())?;
                out.p1(19 + extracheck_var);
                out.p2(var as u16);
                out.pjstr(condition);
                out.p4(value);
                extracheck_var += 1;
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_condition(value: &str) -> Result<(&str, i32)> {
    let (condition, value) = value
        .split_at_checked(1)
        .context("Missing hunt condition")?;
    ensure!(
        matches!(condition, "=" | ">" | "<" | "!" | "&" | "|"),
        "Invalid hunt condition: {condition}"
    );
    Ok((condition, value.parse()?))
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
