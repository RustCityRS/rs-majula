#[cfg(since_530)]
use crate::archive::{self, ArchiveVersions};
#[cfg(since_742)]
use crate::cache::r#if::NO_CLICK_THROUGH;
#[cfg(all(since_530, before_647))]
use crate::cache::r#if::{COMPARATORS, SCRIPT_OPS};
#[cfg(since_530)]
use crate::cache::r#if::{
    DEFAULT_COLOUR, HOOK_NAMES, IfTypeRaw, OpKey, SIZE_MODES, TARGET_MASK, TEXT_ALIGN_H,
    TEXT_ALIGN_V, X_MODES, Y_MODES, has_hook, hook_transmit_list, mode_value,
};
#[cfg(since_530)]
use crate::cache::js5::PackedCache;
#[cfg(before_530)]
use crate::config_crc;
#[cfg(since_530)]
use crate::pack::incremental::{LOBBY_COMMANDS, Stage, dir_key, game_files_under, stats_key};
#[cfg(since_530)]
use crate::pack::js5::{PreparedGroup, insert_archive, load_name_hashes, versions_dep};
#[cfg(since_530)]
use crate::pack::names::AssetNames;
#[cfg(before_530)]
use crate::pack::pack::collect_if_files;
#[cfg(before_530)]
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
#[cfg(before_530)]
use crate::pack::util::{parse_hex, verify_crc};
#[cfg(before_647)]
use crate::types::IfButtonType;
use crate::types::IfComponentType;
#[cfg(before_530)]
use crate::types::{Font, IfComparator, IfScriptOp, PlayerStat};
use anyhow::Result;
#[cfg(since_530)]
use anyhow::{Context, anyhow, bail, ensure};
#[cfg(since_530)]
use rs_io::js5::encode_group_entries;
#[cfg(since_530)]
use rs_ver::TypeId;
#[cfg(since_530)]
use runec::clientscript::{self, HookArg, component_hook};
#[cfg(since_530)]
use runec::sources::roots;
#[cfg(since_530)]
use runec::symbol::SymbolRegistry;
#[cfg(since_530)]
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
#[cfg(before_530)]
use std::str::FromStr;
#[cfg(since_530)]
use std::sync::OnceLock;
#[cfg(before_530)]
use tracing::debug;

#[cfg(before_530)]
fn get_colour(val: Option<&String>) -> &str {
    val.map(|s| s.as_str()).unwrap_or("0")
}

#[cfg(before_530)]
fn get_string(val: Option<&String>) -> &str {
    val.map(|s| s.as_str()).unwrap_or("")
}

#[cfg(before_530)]
fn get_bool(val: Option<&String>) -> bool {
    val.map(|v| v == "yes").unwrap_or(false)
}

#[cfg(before_530)]
fn get_num<T: FromStr>(val: Option<&String>, default: T) -> T {
    val.and_then(|v| v.parse::<T>().ok()).unwrap_or(default)
}

#[cfg(before_530)]
fn count_script_ops(src: &HashMap<String, String>, j: usize) -> usize {
    let mut count = 0;
    for k in 1..=20 {
        if let Some(op) = src.get(&format!("script{j}op{k}")) {
            count += 1;
            if op.is_empty() {
                continue;
            }
            let parts: Vec<&str> = op.split(',').collect();
            match parts.first().copied().unwrap_or("") {
                "stat_level" | "stat_base_level" | "stat_xp" | "stat_xp_remaining" | "pushvar" => {
                    count += 1
                }
                "inv_count" | "inv_contains" | "testbit" => count += 2,
                #[cfg(since_254)]
                "push_varbit" | "push_constant" => count += 1,
                _ => {}
            }
        }
    }
    count
}

#[cfg(before_530)]
fn parse_margin<T: FromStr>(margin: &str, defaultx: T, defaulty: T) -> (T, T) {
    let parts: Vec<&str> = margin.split(',').collect();
    (
        parts
            .first()
            .and_then(|v| v.parse::<T>().ok())
            .unwrap_or(defaultx),
        parts
            .get(1)
            .and_then(|v| v.parse::<T>().ok())
            .unwrap_or(defaulty),
    )
}

#[cfg(before_530)]
fn parse_slot_offset(slot: &str) -> (u16, u16, &str) {
    let parts: Vec<&str> = slot.split(':').collect();
    let sprite = parts.first().copied().unwrap_or("");
    let (x, y) = parts
        .get(1)
        .map(|offset| {
            let o: Vec<&str> = offset.split(',').collect();
            let x = o
                .first()
                .and_then(|v| v.parse::<i16>().ok())
                .map(|v| v as u16)
                .unwrap_or(0);
            let y = o
                .get(1)
                .and_then(|v| v.parse::<i16>().ok())
                .map(|v| v as u16)
                .unwrap_or(0);
            (x, y)
        })
        .unwrap_or((0, 0));
    (x, y, sprite)
}

#[cfg(before_530)]
fn parse_action_flags(action_target: &str) -> u16 {
    let targets: Vec<&str> = action_target.split(',').collect();
    let mut flags = 0u16;
    for target in targets {
        flags |= match target {
            "obj" => 0x1,
            "npc" => 0x2,
            "loc" => 0x4,
            "player" => 0x8,
            "heldobj" => 0x10,
            _ => 0,
        };
    }
    flags
}

#[cfg(before_530)]
struct Component {
    root: Option<String>,
    children: Vec<u16>,
    src: HashMap<String, String>,
}

#[cfg(before_530)]
pub fn pack_interfaces(
    source_dir: &Path,
    registry: &PackRegistry,
    verify: bool,
) -> Result<PackedFile> {
    let pack = &registry.interface;

    let mut components: HashMap<u16, Component> = HashMap::new();

    let interface_order = load_order(&registry.pack_dir().join("interface.order"))?;
    for &id in &interface_order {
        components.insert(
            id,
            Component {
                root: None,
                children: Vec::new(),
                src: HashMap::new(),
            },
        );
    }

    let if_files = collect_if_files(source_dir);
    debug!("  Found {} .if files", if_files.len());

    for (if_name, lines) in &if_files {
        let if_id = registry
            .interface
            .get_by_debugname(if_name)
            .ok_or_else(|| {
                panic!(
                    "Could not find name <-> ID for interface file, perhaps misnamed? {} {:?}",
                    if_name,
                    pack.get_by_debugname(if_name)
                )
            })?;

        let com = components
            .get_mut(&if_id)
            .ok_or_else(|| panic!("Component {if_id} not found"))?;

        com.src.insert("type".to_string(), "layer".to_string());
        com.src.insert("width".to_string(), "512".to_string());
        com.src.insert("height".to_string(), "334".to_string());

        let mut com_name = String::new();
        let mut com_id: i64 = -1;

        for line in lines {
            if line.starts_with('[') {
                com_name = line[1..line.len() - 1].to_string();
                let full_name = format!("{if_name}:{com_name}");
                com_id = registry
                    .interface
                    .get_by_debugname(&full_name)
                    .map(|id| id as i64)
                    .unwrap_or(-1);

                if com_id == -1 || !components.contains_key(&(com_id as u16)) {
                    panic!("Missing component ID {if_name}:{com_name} in interface.order");
                }

                let com_id_u32 = com_id as u16;
                components.get_mut(&com_id_u32).unwrap().root = Some(if_name.clone());
                components
                    .get_mut(&if_id)
                    .unwrap()
                    .children
                    .push(com_id_u32);
                continue;
            }

            if line.trim().is_empty() || line.starts_with("//") || line.starts_with('#') {
                continue;
            }

            let Some(eq) = line.find('=') else {
                continue; // skip lines without = instead of erroring
            };
            let key = &line[..eq];
            let value = &line[eq + 1..];

            if key == "layer" {
                let layer_name = format!("{if_name}:{value}");
                let layer_id = registry
                    .interface
                    .get_by_debugname(&layer_name)
                    .ok_or_else(|| panic!("ERROR: Layer {layer_name} does not exist"))?;

                if com_id >= 0 && components[&layer_id].children.contains(&(com_id as u16)) {
                    panic!("ERROR: Layer {layer_name} already has {com_name} as a child",);
                }

                components
                    .get_mut(&layer_id)
                    .unwrap()
                    .children
                    .push(com_id as u16);
                let pos = components[&if_id]
                    .children
                    .iter()
                    .position(|&c| c == com_id as u16);
                if let Some(pos) = pos {
                    components.get_mut(&if_id).unwrap().children.remove(pos);
                }
            }

            if com_id >= 0 {
                components
                    .get_mut(&(com_id as u16))
                    .unwrap()
                    .src
                    .insert(key.to_string(), value.to_string());
            } else {
                components
                    .get_mut(&if_id)
                    .unwrap()
                    .src
                    .insert(key.to_string(), value.to_string());
            }
        }
    }

    // ---- Pack ----

    let mut client = PackedData::new(pack.max);
    let mut server = PackedData::new(pack.max);

    let mut last_root: Option<String> = None;

    for &id in &interface_order {
        let com = &components[&id];
        let src = &com.src;

        if com.root.is_none() || last_root.as_deref() != com.root.as_deref() {
            client.p2(0xFFFF); // -1
            server.p2(0xFFFF); // -1

            if let Some(root) = &com.root {
                let root_id = registry.interface.get_by_debugname(root).unwrap();
                client.p2(root_id);
                server.p2(root_id);
                last_root = Some(root.clone());
            } else {
                client.p2(id);
                server.p2(id);
                last_root = registry.interface.get_by_id(id).map(|s| s.to_string());
            }
        }

        client.p2(id);
        server.p2(id);

        // server only
        server.pjstr(registry.interface.get_by_id(id).unwrap());
        server.p1(src.get("type").map(|t| t == "overlay").unwrap_or(false) as u8);

        let com_type = IfComponentType::from_config_str(src.get("type").unwrap());
        client.p1(com_type as u8);
        server.p1(com_type as u8);

        let button_type = IfButtonType::from_config_str(get_string(src.get("buttontype")));
        client.p1(button_type as u8);
        server.p1(button_type as u8);

        let clientcode = get_num::<u16>(src.get("clientcode"), 0);
        let width = get_num::<u16>(src.get("width"), 0);
        let height = get_num::<u16>(src.get("height"), 0);
        client.p2(clientcode);
        client.p2(width);
        client.p2(height);
        #[cfg(since_244)]
        client.p1(get_num::<u8>(src.get("trans"), 0));
        server.p2(clientcode);
        server.p2(width);
        server.p2(height);

        if let Some(overlayer) = src.get("overlayer") {
            let layer_name = format!("{}:{}", com.root.as_deref().unwrap(), overlayer);
            let layer_id = registry.interface.get_by_debugname(&layer_name).unwrap();
            client.p2(layer_id + 0x100);
            server.p2(layer_id + 0x100);
        } else {
            client.p1(0);
            server.p1(0);
        }

        // Comparators
        let comparator_count = (1..=5)
            .filter(|j| src.contains_key(&format!("script{j}")))
            .count();

        client.p1(comparator_count as u8);
        server.p1(comparator_count as u8);
        for j in 1..=comparator_count {
            let script_val = src.get(&format!("script{j}")).unwrap();
            let parts: Vec<&str> = script_val.split(',').collect();
            let comparator =
                IfComparator::from_config_str(parts.first().copied().unwrap_or("")) as u8;
            let value = parts
                .get(1)
                .and_then(|v| v.parse::<i16>().ok())
                .unwrap_or(0) as u16;
            client.p1(comparator);
            client.p2(value);
            server.p1(comparator);
            server.p2(value);
        }

        // Scripts
        let script_count = (1..=5)
            .filter(|j| src.contains_key(&format!("script{j}op1")))
            .count();

        client.p1(script_count as u8);
        server.p1(script_count as u8);
        for j in 1..=script_count {
            let op_count = count_script_ops(src, j);

            let first_op = src
                .get(&format!("script{j}op1"))
                .map(|s| s.as_str())
                .unwrap_or("");
            if first_op.is_empty() {
                client.p2(op_count as u16);
                server.p2(op_count as u16);
            } else {
                client.p2((op_count + 1) as u16);
                server.p2((op_count + 1) as u16);
            }

            for k in 1..=op_count {
                let op_key = format!("script{j}op{k}");
                if let Some(op) = src.get(&op_key) {
                    if op.is_empty() {
                        continue;
                    }
                    let parts: Vec<&str> = op.split(',').collect();
                    let describe = || {
                        format!(
                            "interface '{}' {}: '{}'",
                            com.root.as_deref().unwrap_or("<no root>"),
                            op_key,
                            op
                        )
                    };
                    let arg = |i: usize| {
                        parts.get(i).copied().unwrap_or_else(|| {
                            panic!(
                                "Error packing interfaces
{} is missing argument {i}",
                                describe()
                            )
                        })
                    };
                    let op_name = arg(0);
                    client.p2(IfScriptOp::from_config_str(op_name) as u16);
                    server.p2(IfScriptOp::from_config_str(op_name) as u16);

                    match op_name {
                        "stat_level" | "stat_base_level" | "stat_xp" | "stat_xp_remaining" => {
                            client.p2(PlayerStat::from_config_str(arg(1)) as u16);
                            server.p2(PlayerStat::from_config_str(arg(1)) as u16);
                        }
                        "inv_count" | "inv_contains" => {
                            let com_link = registry
                                .interface
                                .get_by_debugname(arg(1))
                                .unwrap_or_else(|| {
                                    panic!(
                                        "Error packing interfaces
{} references unknown interface '{}'",
                                        describe(),
                                        arg(1)
                                    )
                                });
                            let obj_link =
                                registry.obj.get_by_debugname(arg(2)).unwrap_or_else(|| {
                                    panic!(
                                        "Error packing interfaces
{} references unknown obj '{}'",
                                        describe(),
                                        arg(2)
                                    )
                                });
                            client.p2(com_link);
                            client.p2(obj_link);
                            server.p2(com_link);
                            server.p2(obj_link);
                        }
                        "pushvar" => {
                            let varp_link =
                                registry.varp.get_by_debugname(arg(1)).unwrap_or_else(|| {
                                    panic!(
                                        "Error packing interfaces
{} references unknown varp '{}'",
                                        describe(),
                                        arg(1)
                                    )
                                });
                            client.p2(varp_link);
                            server.p2(varp_link);
                        }
                        "testbit" => {
                            let varp_link =
                                registry.varp.get_by_debugname(arg(1)).unwrap_or_else(|| {
                                    panic!(
                                        "Error packing interfaces
{} references unknown varp '{}'",
                                        describe(),
                                        arg(1)
                                    )
                                });
                            let bit = parts
                                .get(2)
                                .and_then(|v| v.parse::<u16>().ok())
                                .unwrap_or(0);
                            client.p2(varp_link);
                            client.p2(bit);
                            server.p2(varp_link);
                            server.p2(bit);
                        }
                        #[cfg(since_254)]
                        "push_varbit" => {
                            let varpbit_link = registry
                                .varpbit
                                .get_by_debugname(arg(1))
                                .unwrap_or_else(|| {
                                    panic!(
                                        "Error packing interfaces
{} references unknown varpbit '{}'",
                                        describe(),
                                        arg(1)
                                    )
                                });
                            client.p2(varpbit_link);
                            server.p2(varpbit_link);
                        }
                        #[cfg(since_254)]
                        "push_constant" => {
                            let value = arg(1).parse::<u16>().unwrap_or_else(|e| {
                                panic!(
                                    "Error packing interfaces
{} has an invalid constant '{}': {e}",
                                    describe(),
                                    arg(1)
                                )
                            });
                            client.p2(value);
                            server.p2(value);
                        }
                        _ => {}
                    }
                }
            }

            if op_count > 0 {
                client.p2(0);
                server.p2(0);
            }
        }

        // Type-specific fields
        if com_type == IfComponentType::Layer {
            client.p2(get_num(src.get("scroll"), 0));
            client.p1(get_bool(src.get("hide")) as u8);
            server.p2(get_num(src.get("scroll"), 0));
            server.p1(get_bool(src.get("hide")) as u8);

            #[cfg(rev = "225")]
            {
                client.p1(com.children.len() as u8);
                server.p1(com.children.len() as u8);
            }
            #[cfg(since_244)]
            {
                client.p2(com.children.len() as u16);
                server.p2(com.children.len() as u16);
            }
            for &child_id in &com.children {
                let child_src = &components[&child_id].src;
                client.p2(child_id);
                let x = get_num::<i16>(child_src.get("x"), 0) as u16;
                let y = get_num::<i16>(child_src.get("y"), 0) as u16;
                client.p2(x);
                client.p2(y);
                server.p2(child_id);
                server.p2(x);
                server.p2(y);
            }
        }

        if com_type == IfComponentType::Inv {
            client.p1(get_bool(src.get("draggable")) as u8);
            client.p1(get_bool(src.get("interactable")) as u8);
            client.p1(get_bool(src.get("usable")) as u8);
            #[cfg(since_245_2)]
            client.p1(get_bool(src.get("swappable")) as u8);
            server.p1(get_bool(src.get("draggable")) as u8);
            server.p1(get_bool(src.get("interactable")) as u8);
            server.p1(get_bool(src.get("usable")) as u8);
            #[cfg(since_245_2)]
            server.p1(get_bool(src.get("swappable")) as u8);

            if let Some(margin) = src.get("margin") {
                let (x, y) = parse_margin(margin, 0, 0);
                client.p1(x);
                client.p1(y);
                server.p1(x);
                server.p1(y);
            } else {
                client.p1(0);
                client.p1(0);
                server.p1(0);
                server.p1(0);
            }

            for j in 1..=20 {
                let slot_key = format!("slot{j}");
                if let Some(slot) = src.get(&slot_key) {
                    client.p1(1);
                    server.p1(1);
                    let (x, y, sprite) = parse_slot_offset(slot);
                    client.p2(x);
                    client.p2(y);
                    client.pjstr(sprite);
                    server.p2(x);
                    server.p2(y);
                    server.pjstr(sprite);
                } else {
                    client.p1(0);
                    server.p1(0);
                }
            }

            for j in 1..=5 {
                client.pjstr(get_string(src.get(&format!("option{j}"))));
                server.pjstr(get_string(src.get(&format!("option{j}"))));
            }
        }

        if com_type == IfComponentType::Rectangle {
            client.p1(get_bool(src.get("fill")) as u8);
            server.p1(get_bool(src.get("fill")) as u8);
        }

        if com_type == IfComponentType::Text {
            client.p1(get_bool(src.get("center")) as u8);
            client.p1(Font::from_config_str(src.get("font").unwrap()) as u8);
            client.p1(get_bool(src.get("shadowed")) as u8);
            client.pjstr(get_string(src.get("text")));
            client.pjstr(get_string(src.get("activetext")));

            server.p1(get_bool(src.get("center")) as u8);
            server.p1(Font::from_config_str(src.get("font").unwrap()) as u8);
            server.p1(get_bool(src.get("shadowed")) as u8);
            server.pjstr(get_string(src.get("text")));
            server.pjstr(get_string(src.get("activetext")));
        }

        if com_type == IfComponentType::Rectangle || com_type == IfComponentType::Text {
            parse_hex(get_colour(src.get("colour")), |v| {
                client.p4(v);
                server.p4(v);
            });
            parse_hex(get_colour(src.get("activecolour")), |v| {
                client.p4(v);
                server.p4(v);
            });
            parse_hex(get_colour(src.get("overcolour")), |v| {
                client.p4(v);
                server.p4(v);
            });
            #[cfg(since_245_2)]
            parse_hex(get_colour(src.get("activeovercolour")), |v| {
                client.p4(v);
                server.p4(v);
            });
        }

        if com_type == IfComponentType::Sprite {
            client.pjstr(get_string(src.get("graphic")));
            client.pjstr(get_string(src.get("activegraphic")));

            server.pjstr(get_string(src.get("graphic")));
            server.pjstr(get_string(src.get("activegraphic")));
        }

        if com_type == IfComponentType::Model {
            if let Some(model) = src.get("model") {
                let model_id = registry.model.get_by_debugname(model).ok_or_else(|| {
                    panic!(
                        "Error packing interfaces\n{} Invalid model: {}",
                        com.root.as_deref().unwrap(),
                        model
                    )
                })?;
                client.p2(model_id + 0x100);
                server.p2(model_id + 0x100);
            } else {
                client.p1(0);
                server.p1(0);
            }

            if let Some(active_model) = src.get("activemodel") {
                let model_id = registry
                    .model
                    .get_by_debugname(active_model)
                    .ok_or_else(|| {
                        panic!(
                            "Error packing interfaces\n{} Invalid activemodel: {}",
                            com.root.as_deref().unwrap(),
                            active_model
                        )
                    })?;
                client.p2(model_id + 0x100);
                server.p2(model_id + 0x100);
            } else {
                client.p1(0);
                server.p1(0);
            }

            if let Some(anim) = src.get("anim") {
                let seq_id = registry.seq.get_by_debugname(anim).ok_or_else(|| {
                    panic!(
                        "Error packing interfaces\n{} Invalid anim: {}",
                        com.root.as_deref().unwrap(),
                        anim
                    )
                })?;
                client.p2(seq_id + 0x100);
                server.p2(seq_id + 0x100);
            } else {
                client.p1(0);
                server.p1(0);
            }

            if let Some(active_anim) = src.get("activeanim") {
                let seq_id = registry.seq.get_by_debugname(active_anim).ok_or_else(|| {
                    panic!(
                        "Error packing interfaces\n{} Invalid activeanim: {}",
                        com.root.as_deref().unwrap(),
                        active_anim
                    )
                })?;
                client.p2(seq_id + 0x100);
                server.p2(seq_id + 0x100);
            } else {
                client.p1(0);
                server.p1(0);
            }

            client.p2(get_num(src.get("zoom"), 0));
            client.p2(get_num(src.get("xan"), 0));
            client.p2(get_num(src.get("yan"), 0));

            server.p2(get_num(src.get("zoom"), 0));
            server.p2(get_num(src.get("xan"), 0));
            server.p2(get_num(src.get("yan"), 0));
        }

        if com_type == IfComponentType::InvText {
            client.p1(get_bool(src.get("center")) as u8);
            client.p1(Font::from_config_str(src.get("font").unwrap()) as u8);
            client.p1(get_bool(src.get("shadowed")) as u8);
            parse_hex(get_colour(src.get("colour")), |v| {
                client.p4(v);
            });

            server.p1(get_bool(src.get("center")) as u8);
            server.p1(Font::from_config_str(src.get("font").unwrap()) as u8);
            server.p1(get_bool(src.get("shadowed")) as u8);
            parse_hex(get_colour(src.get("colour")), |v| {
                server.p4(v);
            });

            let (x, y) = parse_margin(src.get("margin").unwrap(), 0, 0);
            client.p2(x);
            client.p2(y);
            server.p2(x);
            server.p2(y);

            client.p1(get_bool(src.get("interactable")) as u8);
            server.p1(get_bool(src.get("interactable")) as u8);

            for j in 1..=5 {
                client.pjstr(get_string(src.get(&format!("option{j}"))));
                server.pjstr(get_string(src.get(&format!("option{j}"))));
            }
        }

        if button_type == IfButtonType::Target || com_type == IfComponentType::Inv {
            client.pjstr(get_string(src.get("actionverb")));
            client.pjstr(get_string(src.get("action")));

            server.pjstr(get_string(src.get("actionverb")));
            server.pjstr(get_string(src.get("action")));

            let mut flags: u16 = 0;
            if let Some(action_target) = src.get("actiontarget") {
                flags |= parse_action_flags(action_target);
            }
            client.p2(flags);
            server.p2(flags);
        }

        if matches!(
            button_type,
            IfButtonType::Normal
                | IfButtonType::Toggle
                | IfButtonType::Select
                | IfButtonType::Pause
        ) {
            client.pjstr(get_string(src.get("option")));
            server.pjstr(get_string(src.get("option")));
        }
    }

    if verify {
        verify_crc("interface", &client.dat, config_crc::INTERFACE);
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}

#[cfg(before_530)]
fn load_order(path: &Path) -> Result<Vec<u16>> {
    let content = fs::read_to_string(path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| l.trim().parse::<u16>().ok())
        .collect())
}

#[cfg(since_530)]
pub fn pack_archive(
    store: &mut PackedCache,
    content_dir: &Path,
    pack_dir: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
    names: &AssetNames,
) -> Result<()> {
    let aid = archive::INTERFACES;
    let format = archive::archive_format(aid).unwrap();
    #[cfg(all(since_530, before_647))]
    let extensions = ["if1", "if3"];
    #[cfg(since_647)]
    let extensions = ["if3"];
    let if_files = names.files(&content_dir.join("interfaces"), "interface", &extensions)?;
    let gids: Vec<_> = if_files.iter().map(|(gid, _)| *gid).collect();
    let name_hashes = load_name_hashes(pack_dir, aid, &gids);
    let scripts = content_dir.join(
        archive::archive_format(archive::CLIENTSCRIPT)
            .unwrap()
            .content_dir,
    );
    let stage = Stage::new(
        "interfaces",
        vec![
            versions_dep(versions, aid),
            (
                "clientscripts".to_owned(),
                stats_key(&game_files_under(&roots(&scripts))),
            ),
            ("pack".to_owned(), dir_key(pack_dir, &[LOBBY_COMMANDS])),
        ],
    );
    let registry = OnceLock::new();
    let mut prepared = stage.groups(
        names,
        &if_files,
        |path| vec![path.clone()],
        |gid, path, names| {
            let registry = match registry.get() {
                Some(registry) => registry,
                None => {
                    let built = clientscript::registry_with_scripts(&scripts, pack_dir)?;
                    registry.get_or_init(|| built)
                }
            };
            let components = parse_components(
                &fs::read_to_string(path)?,
                gid,
                names,
                registry,
                #[cfg(all(since_530, before_647))]
                path.extension().is_some_and(|ext| ext == "if1"),
            )
            .with_context(|| path.display().to_string())?;
            let entries: Vec<(u32, Vec<u8>)> = components
                .iter()
                .map(|(position, comp)| (*position, encode_component(comp)))
                .collect();
            let refs: Vec<(u32, &[u8])> = entries
                .iter()
                .map(|(position, data)| (*position, data.as_slice()))
                .collect();
            let data = encode_group_entries(&refs);
            let mut group = PreparedGroup::new(aid, gid, &data, versions)?;
            group.fit_files(entries.len());
            Ok(group)
        },
    )?;
    stage.finish(Vec::new());
    for group in &mut prepared {
        group.name_hash = name_hashes.get(&group.id).copied().unwrap_or(-1);
    }

    insert_archive(
        store,
        aid,
        format,
        versions,
        prepared,
        "from .if3 text files",
    )?;

    Ok(())
}

#[cfg(since_530)]
pub fn encode_component(comp: &IfTypeRaw) -> Vec<u8> {
    #[cfg(all(since_530, before_647))]
    if comp.if1 {
        return encode_if1(comp);
    }
    let mut out = PackedData::default();

    out.p1(comp.version);
    let mut type_byte = comp.com_type as u8;
    if comp.com_name.is_some() {
        type_byte |= 0x80;
    }
    out.p1(type_byte);
    if let Some(ref name) = comp.com_name {
        out.pjstr(name);
    }

    out.p2(comp.client_code);
    out.p2(comp.x as u16);
    out.p2(comp.y as u16);
    out.p2(comp.width);
    out.p2(comp.height);
    out.p1(comp.width_mode as u8);
    out.p1(comp.height_mode as u8);
    out.p1(comp.x_mode as u8);
    out.p1(comp.y_mode as u8);

    out.p2(comp.layer.unwrap_or(u16::MAX));
    let var2 = comp.format_version();
    #[cfg(before_742)]
    out.p1(comp.flags);
    #[cfg(since_742)]
    out.p1(if var2 >= 0 && comp.no_click_through {
        comp.flags | NO_CLICK_THROUGH
    } else {
        comp.flags
    });

    if comp.com_type == IfComponentType::Layer {
        out.p2(comp.scroll_width);
        out.p2(comp.scroll_height);
        if var2 < 0 {
            out.p1(comp.no_click_through as u8);
        }
    }

    if comp.com_type == IfComponentType::Sprite {
        out.p4(comp.graphic.map_or(-1, TypeId::int));
        out.p2(comp.angle_2d);
        let sf = comp.tiling as u8 | ((comp.alpha as u8) << 1);
        out.p1(sf);
        out.p1(comp.trans);
        out.p1(comp.outline);
        out.p4(comp.graphic_shadow);
        out.p1(comp.vflip as u8);
        out.p1(comp.hflip as u8);
        #[cfg(since_647)]
        out.p4(comp.colour);
        #[cfg(since_742)]
        if var2 >= 3 {
            out.p1(comp.click_mask as u8);
        }
    }

    if comp.com_type == IfComponentType::Model {
        put_big_id(&mut out, comp.model);
        #[cfg(all(since_530, before_647))]
        {
            out.p2(comp.model_origin_x as u16);
            out.p2(comp.model_origin_y as u16);
            out.p2(comp.xan);
            out.p2(comp.yan);
            out.p2(comp.model_zan);
            out.p2(comp.zoom as u16);
            put_big_id(&mut out, comp.anim);
            out.p1(comp.model_ortho as u8);
            out.p2(comp.model_unknown.0);
            out.p2(comp.model_unknown.1);
            out.p1(comp.model_unknown.2);
        }
        #[cfg(since_647)]
        {
            out.p1(comp.model_flags);
            let position_set = comp.model_flags & 1 != 0;
            let a_bool_19 = comp.model_flags & 2 != 0;
            if position_set {
                out.p2(comp.model_origin_x as u16);
                out.p2(comp.model_origin_y as u16);
                out.p2(comp.xan);
                out.p2(comp.yan);
                out.p2(comp.model_zan);
                out.p2(comp.zoom as u16);
            } else if a_bool_19 {
                out.p2(comp.model_origin_x as u16);
                out.p2(comp.model_origin_y as u16);
                out.p2(comp.model_origin_z as u16);
                out.p2(comp.xan);
                out.p2(comp.yan);
                out.p2(comp.model_zan);
                #[cfg(before_742)]
                out.p1(comp.zoom as u8);
                #[cfg(since_742)]
                out.p2(comp.zoom as u16);
            }
            put_big_id(&mut out, comp.anim);
        }
        if comp.width_mode != 0 {
            out.p2(comp.dynamic_width);
        }
        if comp.height_mode != 0 {
            out.p2(comp.dynamic_height);
        }
    }

    if comp.com_type == IfComponentType::Text {
        put_big_id(&mut out, comp.font);
        #[cfg(since_742)]
        if var2 >= 2 {
            out.p1(comp.font_mono as u8);
        }
        out.pjstr(&comp.text);
        out.p1(comp.line_spacing);
        out.p1(comp.text_halign);
        out.p1(comp.text_valign);
        out.p1(comp.shadowed as u8);
        out.p4(comp.colour);
        #[cfg(since_647)]
        out.p1(comp.trans);
        #[cfg(since_742)]
        if var2 >= 0 {
            out.p1(comp.text_max_lines);
        }
    }

    if comp.com_type == IfComponentType::Rectangle {
        out.p4(comp.colour);
        out.p1(comp.fill as u8);
        out.p1(comp.trans);
    }

    if comp.com_type == IfComponentType::Line {
        out.p1(comp.line_width);
        out.p4(comp.colour);
        out.p1(comp.line_rtl as u8);
    }

    out.p3(comp.active_properties as i32);

    for opkey in &comp.opkeys {
        let rate = if opkey.rate_duration == -1 {
            4095u32
        } else {
            opkey.rate_duration as u32 & 0xFFF
        };
        let header = ((opkey.index + 1) << 4) | ((rate >> 8) as u8 & 0xF);
        let next = (rate & 0xFF) as u8;
        out.p1(header);
        out.p1(next);
        out.p1(opkey.code as u8);
        out.p1(opkey.mask as u8);
    }
    out.p1(0); // opkey terminator

    out.pjstr(&comp.op_base);
    let ops_packed = (comp.ops.len() as u8 & 0xF) | (comp.op_cursor_count << 4);
    out.p1(ops_packed);
    for op in &comp.ops {
        out.pjstr(op);
    }
    if comp.op_cursor_count > 0 && !comp.op_cursors.is_empty() {
        out.p1(comp.op_cursors[0].0);
        out.p2(comp.op_cursors[0].1.g2());
    }
    if comp.op_cursor_count > 1 && comp.op_cursors.len() > 1 {
        out.p1(comp.op_cursors[1].0);
        out.p2(comp.op_cursors[1].1.g2());
    }

    #[cfg(since_647)]
    out.pjstr(&comp.pause_text);
    out.p1(comp.drag_deadzone);
    out.p1(comp.drag_deadtime);
    out.p1(comp.drag_render_behaviour);
    out.pjstr(&comp.target_verb);

    let target_mask = (comp.active_properties >> 11) & 0x7F;
    if target_mask != 0 {
        out.p2(comp.target_object_id.map_or(u16::MAX, TypeId::g2));
        out.p2(comp.valid_target_cursor.map_or(u16::MAX, TypeId::g2));
        out.p2(comp.invalid_target_cursor.map_or(u16::MAX, TypeId::g2));
    }

    #[cfg(since_742)]
    if var2 >= 0 {
        out.p2(comp.mouse_over_cursor.map_or(u16::MAX, TypeId::g2));
        out.p1(comp.int_params.len() as u8);
        for &(key, value) in &comp.int_params {
            out.p3(key as i32);
            out.p4(value);
        }
        out.p1(comp.string_params.len() as u8);
        for (key, value) in &comp.string_params {
            out.p3(*key as i32);
            out.p1(0);
            out.pjstr(value);
        }
    }

    for (hook, values) in comp.hooks.iter().enumerate() {
        if has_hook(comp.version, hook) {
            encode_hook(&mut out, values);
        }
    }

    for list in &comp.transmit_lists {
        encode_transmitlist(&mut out, list);
    }

    out.dat
}

#[cfg(since_530)]
fn put_big_id(out: &mut PackedData, id: TypeId) {
    out.pbigid(id);
}

#[cfg(all(since_530, before_647))]
fn encode_if1(comp: &IfTypeRaw) -> Vec<u8> {
    let mut out = PackedData::default();
    out.p1(comp.com_type as u8);
    out.p1(comp.button_type as u8);
    out.p2(comp.client_code);
    out.p2(comp.x as u16);
    out.p2(comp.y as u16);
    out.p2(comp.width);
    out.p2(comp.height);
    out.p1(comp.trans);
    out.p2(comp.layer.unwrap_or(u16::MAX));
    out.p2(comp.over_layer.unwrap_or(u16::MAX));
    out.p1(comp.comparators.len() as u8);
    for &(comparator, value) in &comp.comparators {
        out.p1(comparator);
        out.p2(value as u16);
    }
    out.p1(comp.scripts.len() as u8);
    for script in &comp.scripts {
        out.p2(script.len() as u16);
        for &word in script {
            out.p2(word);
        }
    }
    match comp.com_type {
        IfComponentType::Layer => {
            out.p2(comp.scroll_height);
            out.p1(comp.flags);
        }
        IfComponentType::Unknown1 => {
            out.p2(comp.unknown_1.0);
            out.p1(comp.unknown_1.1);
        }
        IfComponentType::Inv => {
            out.p1(comp.draggable as u8);
            out.p1(comp.operable as u8);
            out.p1(comp.usable as u8);
            out.p1(comp.swappable as u8);
            out.p1(comp.margin_x as u8);
            out.p1(comp.margin_y as u8);
            for slot in 0..20 {
                match comp.slots.get(slot).copied().flatten() {
                    Some((x, y, graphic)) => {
                        out.p1(1);
                        out.p2(x as u16);
                        out.p2(y as u16);
                        out.p4(graphic);
                    }
                    None => out.p1(0),
                }
            }
            encode_iops(&mut out, comp);
        }
        IfComponentType::Rectangle => out.p1(comp.fill as u8),
        _ => {}
    }
    if matches!(
        comp.com_type,
        IfComponentType::Unknown1 | IfComponentType::Text
    ) {
        out.p1(comp.text_halign);
        out.p1(comp.text_valign);
        out.p1(comp.line_spacing);
        out.p2(comp.font.g2());
        out.p1(comp.shadowed as u8);
    }
    if comp.com_type == IfComponentType::Text {
        out.pjstr(&comp.text);
        out.pjstr(&comp.active_text);
    }
    if matches!(
        comp.com_type,
        IfComponentType::Unknown1 | IfComponentType::Rectangle | IfComponentType::Text
    ) {
        out.p4(comp.colour);
    }
    if matches!(
        comp.com_type,
        IfComponentType::Rectangle | IfComponentType::Text
    ) {
        out.p4(comp.active_colour);
        out.p4(comp.over_colour);
        out.p4(comp.active_over_colour);
    }
    match comp.com_type {
        IfComponentType::Sprite => {
            out.p4(comp.graphic.map_or(-1, TypeId::int));
            out.p4(comp.active_graphic.map_or(-1, TypeId::int));
        }
        IfComponentType::Model => {
            out.p2(comp.model.g2());
            out.p2(comp.active_model.g2());
            out.p2(comp.anim.g2());
            out.p2(comp.active_anim.g2());
            out.p2(comp.zoom as u16);
            out.p2(comp.xan);
            out.p2(comp.yan);
        }
        IfComponentType::InvText => {
            out.p1(comp.text_halign);
            out.p2(comp.font.g2());
            out.p1(comp.shadowed as u8);
            out.p4(comp.colour);
            out.p2(comp.margin_x as u16);
            out.p2(comp.margin_y as u16);
            out.p1(comp.operable as u8);
            encode_iops(&mut out, comp);
        }
        IfComponentType::Unknown8 => out.pjstr(&comp.text),
        _ => {}
    }
    if comp.button_type == IfButtonType::Target || comp.com_type == IfComponentType::Inv {
        out.pjstr(&comp.action_verb);
        out.pjstr(&comp.action);
        out.p2(comp.target_mask);
    }
    if matches!(
        comp.button_type,
        IfButtonType::Normal | IfButtonType::Toggle | IfButtonType::Select | IfButtonType::Pause
    ) {
        out.pjstr(&comp.option);
    }
    out.dat
}

#[cfg(all(since_530, before_647))]
fn encode_iops(out: &mut PackedData, comp: &IfTypeRaw) {
    for op in 0..5 {
        out.pjstr(comp.iop.get(op).map_or("", String::as_str));
    }
}

#[cfg(since_530)]
fn encode_hook(out: &mut PackedData, hook: &Option<Vec<HookArg>>) {
    match hook {
        None => out.p1(0),
        Some(args) => {
            out.p1(args.len() as u8);
            for arg in args {
                match arg {
                    HookArg::Int(v) => {
                        out.p1(0);
                        out.p4(*v);
                    }
                    HookArg::String(s) => {
                        out.p1(1);
                        out.pjstr(s);
                    }
                }
            }
        }
    }
}

#[cfg(since_530)]
fn encode_transmitlist(out: &mut PackedData, list: &Option<Vec<i32>>) {
    match list {
        None => out.p1(0),
        Some(ids) => {
            out.p1(ids.len() as u8);
            for &id in ids {
                out.p4(id);
            }
        }
    }
}

/// Components by their position in the group.
#[cfg(since_530)]
pub fn parse_components(
    text: &str,
    group: u32,
    names: &AssetNames,
    registry: &SymbolRegistry,
    #[cfg(all(since_530, before_647))] if1: bool,
) -> Result<BTreeMap<u32, IfTypeRaw>> {
    let mut components: BTreeMap<u32, IfTypeRaw> = BTreeMap::new();
    let mut current = None;
    for line in text.lines() {
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if let Some(header) = line.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
            let position = names.component_id(group, header)?;
            let comp = IfTypeRaw {
                id: position as u16,
                root_layer: group as i32,
                version: u8::MAX,
                #[cfg(all(since_530, before_647))]
                if1,
                model: TypeId::NULL,
                #[cfg(all(since_530, before_647))]
                active_model: TypeId::NULL,
                anim: TypeId::NULL,
                #[cfg(all(since_530, before_647))]
                active_anim: TypeId::NULL,
                font: TypeId::NULL,
                ..IfTypeRaw::default()
            };
            ensure!(
                components.insert(position, comp).is_none(),
                "Duplicate component {header}"
            );
            current = Some(position);
            continue;
        }
        let comp = current
            .and_then(|position| components.get_mut(&position))
            .with_context(|| format!("Property outside a component: {line}"))?;
        let (key, value) = line
            .split_once('=')
            .with_context(|| format!("Invalid line: {line}"))?;
        parse_property(comp, key, value, group, names, registry)
            .with_context(|| format!("Invalid line: {line}"))?;
    }
    for comp in components.values_mut() {
        if let Some(hook) = (0..HOOK_NAMES.len())
            .find(|&hook| comp.hooks[hook].is_some() && !has_hook(comp.version, hook))
        {
            bail!("{} needs a newer component version", HOOK_NAMES[hook]);
        }
        comp.op_cursor_count = comp.op_cursors.len() as u8;
        #[cfg(all(since_530, before_647))]
        if comp.if1 {
            for script in &mut comp.scripts {
                script.push(0);
            }
        }
        // Without a placement a model is only its id: all of these stay zero.
        #[cfg(since_647)]
        if comp.com_type == IfComponentType::Model
            && comp.model_flags & 2 == 0
            && (comp.model_origin_x != 0
                || comp.model_origin_y != 0
                || comp.xan != 0
                || comp.yan != 0
                || comp.model_zan != 0
                || comp.zoom != 0)
        {
            comp.model_flags |= 1;
        }
    }
    Ok(components)
}

#[cfg(since_530)]
fn parse_property(
    comp: &mut IfTypeRaw,
    key: &str,
    value: &str,
    group: u32,
    names: &AssetNames,
    registry: &SymbolRegistry,
) -> Result<()> {
    let mode =
        |modes: &[&str]| mode_value(modes, value).with_context(|| format!("Unknown mode: {value}"));
    let reference =
        |kind: &str| -> Result<i32> { Ok(names.resolve(kind, value.trim_matches('"'))? as i32) };
    let colour = || parse_colour(value);
    match key {
        "type" => {
            comp.com_type = match value {
                "layer" => IfComponentType::Layer,
                "rectangle" => IfComponentType::Rectangle,
                "text" => {
                    comp.colour = DEFAULT_COLOUR;
                    IfComponentType::Text
                }
                "graphic" => {
                    #[cfg(since_647)]
                    {
                        comp.colour = DEFAULT_COLOUR;
                    }
                    IfComponentType::Sprite
                }
                "model" => {
                    comp.zoom = 100;
                    IfComponentType::Model
                }
                "line" => {
                    comp.line_width = 1;
                    IfComponentType::Line
                }
                #[cfg(all(since_530, before_647))]
                "inv" => IfComponentType::Inv,
                #[cfg(all(since_530, before_647))]
                "unknown1" | "invtext" => {
                    comp.colour = DEFAULT_COLOUR;
                    IfComponentType::from_config_str(value)
                }
                #[cfg(all(since_530, before_647))]
                "unknown8" => IfComponentType::Unknown8,
                _ => bail!("Unknown component type"),
            }
        }
        "name" => comp.com_name = Some(value.into()),
        #[cfg(since_742)]
        "version" => comp.version = value.parse()?,
        "clientcode" => comp.client_code = value.parse()?,
        "x" => comp.x = value.parse()?,
        "y" => comp.y = value.parse()?,
        "width" => comp.width = value.parse()?,
        "height" => comp.height = value.parse()?,
        "widthmode" => comp.width_mode = mode(&SIZE_MODES)? as i8,
        "heightmode" => comp.height_mode = mode(&SIZE_MODES)? as i8,
        "xmode" => comp.x_mode = mode(&X_MODES)? as i8,
        "ymode" => comp.y_mode = mode(&Y_MODES)? as i8,
        "layer" => comp.layer = Some(names.component_id(group, value)? as u16),
        "hide" => comp.flags |= 1,
        "scrollwidth" => comp.scroll_width = value.parse()?,
        "scrollheight" => comp.scroll_height = value.parse()?,
        "noclickthrough" => comp.no_click_through = true,
        "graphic" => {
            comp.graphic = TypeId::from_int(reference("sprites")?).non_null();
        }
        "2dangle" => comp.angle_2d = value.parse()?,
        "tiling" => comp.tiling = true,
        "alpha" => comp.alpha = true,
        "trans" => comp.trans = value.parse()?,
        "outline" => comp.outline = value.parse()?,
        "graphicshadow" => comp.graphic_shadow = value.parse()?,
        "vflip" => comp.vflip = true,
        "hflip" => comp.hflip = true,
        #[cfg(since_742)]
        "clickmask" => comp.click_mask = true,
        #[cfg(since_742)]
        "fontmono" => comp.font_mono = true,
        #[cfg(since_742)]
        "maxlines" => comp.text_max_lines = value.parse()?,
        #[cfg(since_742)]
        "mouseovercursor" => {
            comp.mouse_over_cursor = TypeId::from_int(reference("cursor")?).non_null();
        }
        #[cfg(since_742)]
        "param" => {
            let (param, value) = value.split_once(',').context("Expected param,value")?;
            let param = names.resolve("param", param)? as u32;
            match value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
                Some(text) => comp.string_params.push((param, text.to_owned())),
                None => comp.int_params.push((param, value.parse()?)),
            }
        }
        "colour" => comp.colour = colour()?,
        "fill" => comp.fill = true,
        "model" => comp.model = TypeId::from_int(reference("model")?),
        "modelorigin" => {
            let (x, y) = value.split_once(',').context("Expected x,y")?;
            comp.model_origin_x = x.parse()?;
            comp.model_origin_y = y.parse()?;
        }
        "modelangle" => {
            let mut angles = value.split(',');
            let mut angle =
                || -> Result<u16> { Ok(angles.next().context("Expected x,y,z")?.parse()?) };
            comp.xan = angle()?;
            comp.yan = angle()?;
            comp.model_zan = angle()?;
        }
        "modelzoom" => comp.zoom = value.parse()?,
        #[cfg(since_647)]
        "modeloffset" => {
            comp.model_origin_z = value.parse()?;
            comp.model_flags |= 2;
        }
        "modelanim" => comp.anim = TypeId::from_int(reference("seq")?),
        #[cfg(all(since_530, before_647))]
        "modelorthog" => comp.model_ortho = true,
        #[cfg(since_647)]
        "modelorthog" => comp.model_flags |= 4,
        #[cfg(since_647)]
        "modelnodepth" => comp.model_flags |= 8,
        #[cfg(all(since_530, before_647))]
        "modelunknown" => {
            let mut parts = value.split(',');
            let mut part = || parts.next().context("Expected a,b,c");
            comp.model_unknown = (part()?.parse()?, part()?.parse()?, part()?.parse()?);
        }
        "modelobjwidth" => comp.dynamic_width = value.parse()?,
        "modelobjheight" => comp.dynamic_height = value.parse()?,
        "textfont" => comp.font = TypeId::from_int(reference("fontmetrics")?),
        "text" => comp.text = value.to_owned(),
        "textlineheight" => comp.line_spacing = value.parse()?,
        "textalignh" => comp.text_halign = mode(&TEXT_ALIGN_H)?,
        "textalignv" => comp.text_valign = mode(&TEXT_ALIGN_V)?,
        "textshadow" => comp.shadowed = true,
        "linewid" => comp.line_width = value.parse()?,
        "linedirection" => comp.line_rtl = true,
        "pausebutton" => comp.active_properties |= 1,
        "transmitop" => {
            for op in value.split(',') {
                comp.active_properties |= 1 << op.parse::<u32>()?;
            }
        }
        "targetmask" => {
            for target in value.split(',') {
                let bit = TARGET_MASK
                    .iter()
                    .position(|name| *name == target)
                    .with_context(|| format!("Unknown target: {target}"))?;
                comp.active_properties |= 1 << (11 + bit);
            }
        }
        "dragdepth" => comp.active_properties |= value.parse::<u32>()? << 18,
        "candrop" => comp.active_properties |= 1 << 21,
        "targetable" => comp.active_properties |= 1 << 22,
        "opbase" => comp.op_base = value.to_owned(),
        #[cfg(since_647)]
        "pausetext" => comp.pause_text = value.to_owned(),
        "dragdeadzone" => comp.drag_deadzone = value.parse()?,
        "dragdeadtime" => comp.drag_deadtime = value.parse()?,
        "dragrenderbehaviour" => comp.drag_render_behaviour = value.parse()?,
        "targetverb" => comp.target_verb = value.to_owned(),
        "targetobj" => {
            comp.target_object_id = TypeId::from_int(reference("obj")?).non_null();
        }
        "validtargetcursor" => {
            comp.valid_target_cursor = TypeId::from_int(reference("cursor")?).non_null();
        }
        "invalidtargetcursor" => {
            comp.invalid_target_cursor = TypeId::from_int(reference("cursor")?).non_null();
        }
        _ => {
            #[cfg(all(since_530, before_647))]
            if parse_if1_property(comp, key, value, group, names)? {
                return Ok(());
            }
            if let Some(op) = key.strip_prefix("opkey") {
                let mut parts = value.split(',');
                let code = parts.next().context("Expected code,mask")?.parse()?;
                let mask = parts.next().context("Expected code,mask")?.parse()?;
                let index = op.parse::<u8>()? - 1;
                #[cfg(all(since_530, before_647))]
                ensure!(index < 10, "Opkeys count to 10");
                comp.opkeys.push(OpKey {
                    index,
                    rate_duration: parts.next().map_or(Ok(0), str::parse)?,
                    code,
                    mask,
                });
            } else if let Some(op) = key.strip_prefix("opcursor") {
                comp.op_cursors
                    .push((op.parse()?, TypeId::new(reference("cursor")? as u16)));
            } else if let Some(op) = key
                .strip_prefix("op")
                .and_then(|op| op.parse::<usize>().ok())
            {
                ensure!(op >= 1, "Ops count from 1");
                if comp.ops.len() < op {
                    comp.ops.resize(op, String::new());
                }
                comp.ops[op - 1] = value.to_owned();
            } else if let Some(hook) = HOOK_NAMES.iter().position(|name| *name == key) {
                let list = hook_transmit_list(hook);
                let (values, transmits) =
                    component_hook(value, list.map(|(_, kind)| kind), registry)
                        .map_err(|error| anyhow!(error))?;
                comp.hooks[hook] = Some(values);
                if let Some((list, _)) = list
                    && !transmits.is_empty()
                {
                    comp.transmit_lists[list] = Some(transmits);
                }
            } else {
                bail!("Unknown property");
            }
        }
    }
    Ok(())
}

#[cfg(since_530)]
fn parse_colour(value: &str) -> Result<i32> {
    let digits = value.strip_prefix("0x").context("Expected a hex colour")?;
    Ok(u32::from_str_radix(digits, 16)? as i32)
}

/// Reads a key only if1 components have, and says whether `key` was one.
#[cfg(all(since_530, before_647))]
fn parse_if1_property(
    comp: &mut IfTypeRaw,
    key: &str,
    value: &str,
    group: u32,
    names: &AssetNames,
) -> Result<bool> {
    if !comp.if1 {
        return Ok(false);
    }
    let graphic = || -> Result<Option<TypeId>> {
        let graphic = names.resolve("sprites", value.trim_matches('"'))? as i32;
        Ok(TypeId::from_int(graphic).non_null())
    };
    match key {
        "buttontype" => {
            comp.button_type = (0..=6)
                .filter_map(|button| IfButtonType::try_from(button).ok())
                .find(|button| button.config_str() == value)
                .with_context(|| format!("Unknown button type: {value}"))?;
        }
        "overlayer" => comp.over_layer = Some(names.component_id(group, value)? as u16),
        "unknown1" => {
            let (a, b) = value.split_once(',').context("Expected a,b")?;
            comp.unknown_1 = (a.parse()?, b.parse()?);
        }
        "draggable" => comp.draggable = true,
        "interactable" => comp.operable = true,
        "usable" => comp.usable = true,
        "swappable" => comp.swappable = true,
        "margin" => {
            let (x, y) = value.split_once(',').context("Expected x,y")?;
            comp.margin_x = x.parse()?;
            comp.margin_y = y.parse()?;
        }
        "activetext" => comp.active_text = value.to_owned(),
        "activecolour" => comp.active_colour = parse_colour(value)?,
        "overcolour" => comp.over_colour = parse_colour(value)?,
        "activeovercolour" => comp.active_over_colour = parse_colour(value)?,
        "activegraphic" => comp.active_graphic = graphic()?,
        "activemodel" => {
            comp.active_model = TypeId::from_int(names.resolve("model", value)? as i32);
        }
        "activeanim" => comp.active_anim = TypeId::from_int(names.resolve("seq", value)? as i32),
        "actionverb" => comp.action_verb = value.to_owned(),
        "action" => comp.action = value.to_owned(),
        "actiontarget" => {
            for target in value.split(',') {
                let bit = TARGET_MASK
                    .iter()
                    .position(|name| *name == target)
                    .with_context(|| format!("Unknown target: {target}"))?;
                comp.target_mask |= 1 << bit;
            }
        }
        "option" => comp.option = value.to_owned(),
        _ => {
            if let Some(slot) = key.strip_prefix("slot") {
                let slot: usize = slot.parse()?;
                ensure!((1..=20).contains(&slot), "Slots count from 1 to 20");
                let (sprite, offset) = value.rsplit_once(':').context("Expected graphic:x,y")?;
                let (x, y) = offset.split_once(',').context("Expected graphic:x,y")?;
                let sprite = names.resolve("sprites", sprite.trim_matches('"'))? as i32;
                comp.slots.resize(20, None);
                comp.slots[slot - 1] = Some((x.parse()?, y.parse()?, sprite));
            } else if let Some(op) = key.strip_prefix("option") {
                let op: usize = op.parse()?;
                ensure!((1..=5).contains(&op), "Options count from 1 to 5");
                comp.iop.resize(5, String::new());
                comp.iop[op - 1] = value.to_owned();
            } else if let Some(script) = key.strip_prefix("script") {
                let (script, op) = script
                    .split_once("op")
                    .map_or((script, None), |(script, op)| (script, Some(op)));
                let script: usize = script.parse()?;
                ensure!(script >= 1, "Scripts count from 1");
                if let Some(op) = op {
                    ensure!(op.parse::<usize>()? >= 1, "Script ops count from 1");
                    if comp.scripts.len() < script {
                        comp.scripts.resize(script, Vec::new());
                    }
                    parse_script_op(&mut comp.scripts[script - 1], value, names)?;
                } else {
                    let (comparator, operand) =
                        value.split_once(',').context("Expected comparator,value")?;
                    let comparator = COMPARATORS
                        .iter()
                        .position(|name| *name == comparator)
                        .with_context(|| format!("Unknown comparator: {comparator}"))?;
                    if comp.comparators.len() < script {
                        comp.comparators.resize(script, (0, 0));
                    }
                    comp.comparators[script - 1] =
                        (comparator as u8 + 1, operand.parse::<u16>()? as i16);
                }
            } else {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// One op of an if1 script, as its words: a reference that is null is 65535.
#[cfg(all(since_530, before_647))]
fn parse_script_op(words: &mut Vec<u16>, value: &str, names: &AssetNames) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    let mut parts = value.split(',');
    let name = parts.next().unwrap_or_default();
    let op = SCRIPT_OPS
        .iter()
        .position(|(op, _)| *op == name)
        .with_context(|| format!("Unknown script op: {name}"))?;
    words.push(op as u16 + 1);
    for kind in SCRIPT_OPS[op].1 {
        let operand = parts
            .next()
            .with_context(|| format!("{name} is missing its {kind}"))?;
        match *kind {
            "int" => words.push(operand.parse()?),
            "com" if operand == "null" => words.extend([u16::MAX, u16::MAX]),
            "com" => {
                let (interface, component) = operand
                    .split_once(':')
                    .context("Expected interface:component")?;
                let interface = names.id("interface", interface)?;
                words.push(interface as u16);
                words.push(names.component_id(interface, component)? as u16);
            }
            kind => words.push(names.resolve(kind, operand)? as u16),
        }
    }
    ensure!(parts.next().is_none(), "{name} has too many operands");
    Ok(())
}
