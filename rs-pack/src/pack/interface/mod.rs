#[cfg(since_647)]
use crate::archive::{self, ArchiveVersions};
#[cfg(since_647)]
use crate::cache::r#if::{HookArg, IfTypeRaw, OpKey};
#[cfg(since_647)]
use crate::cache::js5::PackedCache;
#[cfg(before_647)]
use crate::config_crc;
#[cfg(since_647)]
use crate::pack::js5::{PreparedGroup, insert_archive, load_name_hashes};
#[cfg(before_647)]
use crate::pack::pack::collect_if_files;
#[cfg(since_647)]
use crate::pack::pack_registry::PackFile;
#[cfg(before_647)]
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
#[cfg(before_647)]
use crate::pack::util::{parse_hex, verify_crc};
use crate::types::IfComponentType;
#[cfg(before_647)]
use crate::types::{Font, IfButtonType, IfComparator, IfScriptOp, PlayerStat};
use anyhow::Result;
#[cfg(since_647)]
use rayon::prelude::*;
#[cfg(since_647)]
use rs_io::js5::encode_group_entries;
#[cfg(since_647)]
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
#[cfg(since_647)]
use std::path::PathBuf;
#[cfg(before_647)]
use std::str::FromStr;
#[cfg(before_647)]
use tracing::debug;

#[cfg(before_647)]
fn get_colour(val: Option<&String>) -> &str {
    val.map(|s| s.as_str()).unwrap_or("0")
}

#[cfg(before_647)]
fn get_string(val: Option<&String>) -> &str {
    val.map(|s| s.as_str()).unwrap_or("")
}

#[cfg(before_647)]
fn get_bool(val: Option<&String>) -> bool {
    val.map(|v| v == "yes").unwrap_or(false)
}

#[cfg(before_647)]
fn get_num<T: FromStr>(val: Option<&String>, default: T) -> T {
    val.and_then(|v| v.parse::<T>().ok()).unwrap_or(default)
}

#[cfg(before_647)]
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

#[cfg(before_647)]
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

#[cfg(before_647)]
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

#[cfg(before_647)]
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

#[cfg(before_647)]
struct Component {
    root: Option<String>,
    children: Vec<u16>,
    src: HashMap<String, String>,
}

#[cfg(before_647)]
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
                            let varbit_link =
                                registry.varbit.get_by_debugname(arg(1)).unwrap_or_else(|| {
                                    panic!(
                                        "Error packing interfaces
{} references unknown varbit '{}'",
                                        describe(),
                                        arg(1)
                                    )
                                });
                            client.p2(varbit_link);
                            server.p2(varbit_link);
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

        if com_type == IfComponentType::Rect {
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

        if com_type == IfComponentType::Rect || com_type == IfComponentType::Text {
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

        if com_type == IfComponentType::Graphic {
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

#[cfg(before_647)]
fn load_order(path: &Path) -> Result<Vec<u16>> {
    let content = fs::read_to_string(path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| l.trim().parse::<u16>().ok())
        .collect())
}

#[cfg(since_647)]
pub fn pack_archive(
    store: &mut PackedCache,
    content_dir: &Path,
    pack_dir: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
) -> Result<()> {
    let aid = archive::INTERFACES;
    let format = archive::archive_format(aid).unwrap();
    let if_dir = content_dir.join("interfaces").join("_unpack");

    let iface_pack =
        PackFile::load(&pack_dir.join("interface.pack")).unwrap_or_else(|_| PackFile::empty());

    let comp_name_maps: HashMap<u32, HashMap<String, u32>> = {
        let mut maps: HashMap<u32, HashMap<String, u32>> = HashMap::new();
        let pack_path = pack_dir.join("interface.pack");
        if let Ok(text) = fs::read_to_string(&pack_path) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let Some((id_str, name)) = line.split_once('=') else {
                    continue;
                };
                let Some((gid_str, cid_str)) = id_str.split_once(':') else {
                    continue;
                };
                let Ok(gid) = gid_str.parse::<u32>() else {
                    continue;
                };
                let Ok(cid) = cid_str.parse::<u32>() else {
                    continue;
                };
                maps.entry(gid)
                    .or_default()
                    .insert(name.trim().to_string(), cid);
            }
        }
        maps
    };

    let mut if_files: Vec<(u32, PathBuf)> = Vec::new();
    if if_dir.is_dir() {
        for entry in fs::read_dir(&if_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("if") {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if let Ok(gid) = stem.parse::<u32>() {
                if_files.push((gid, path));
            } else if let Some(gid) = iface_pack.get_by_debugname(stem) {
                if_files.push((gid, path));
            }
        }
    }
    if_files.sort_by_key(|(gid, _)| *gid);
    let gids: Vec<_> = if_files.iter().map(|(gid, _)| *gid).collect();
    let name_hashes = load_name_hashes(pack_dir, aid, &gids);

    let prepared: BTreeMap<u32, PreparedGroup> = if_files
        .par_iter()
        .map(|(gid, path)| {
            let text = fs::read_to_string(path).unwrap();
            let names = comp_name_maps.get(gid);
            let components = parse_interface_text(&text, names);
            let mut file_entries: Vec<(u32, Vec<u8>)> = components
                .iter()
                .map(|(id, comp)| (*id, encode_component(comp)))
                .collect();
            file_entries.sort_by_key(|(id, _)| *id);
            let refs: Vec<(u32, &[u8])> = file_entries
                .iter()
                .map(|(id, data)| (*id, data.as_slice()))
                .collect();
            let data = encode_group_entries(&refs);
            let mut group = PreparedGroup::new(aid, *gid, &data, versions)?;
            group.name_hash = name_hashes.get(gid).copied().unwrap_or(-1);
            Ok((*gid, group))
        })
        .collect::<Result<_>>()?;

    insert_archive(
        store,
        aid,
        format,
        versions,
        prepared.into_values().collect(),
        "from .if text files",
    )?;

    Ok(())
}

#[cfg(since_647)]
pub fn encode_component(comp: &IfTypeRaw) -> Vec<u8> {
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

    out.p2(comp.parent_id);
    out.p1(comp.flags);

    let var2: i32 = if comp.version == 255 {
        -1
    } else {
        comp.version as i32
    };

    if comp.com_type == IfComponentType::Layer {
        out.p2(comp.scroll_width);
        out.p2(comp.scroll);
        if var2 < 0 {
            out.p1(comp.no_click_through as u8);
        }
    }

    if comp.com_type == IfComponentType::Graphic {
        out.p4(comp.graphic);
        out.p2(comp.angle_2d);
        let sf = comp.tiling as u8 | ((comp.alpha as u8) << 1);
        out.p1(sf);
        out.p1(comp.trans);
        out.p1(comp.outline);
        out.p4(comp.graphic_shadow);
        out.p1(comp.vflip as u8);
        out.p1(comp.hflip as u8);
        out.p4(comp.colour);
    }

    if comp.com_type == IfComponentType::Model {
        out.p2(comp.model as u16);
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
            out.p2(comp.sprite_y_offset as u16);
            out.p2(comp.xan);
            out.p2(comp.yan);
            out.p2(comp.model_zan);
            out.p1(comp.zoom as u8);
        }
        out.p2(comp.anim as u16);
        if comp.width_mode != 0 {
            out.p2(comp.dynamic_width);
        }
        if comp.height_mode != 0 {
            out.p2(comp.dynamic_height);
        }
    }

    if comp.com_type == IfComponentType::Text {
        out.p2(comp.font as u16);
        out.pjstr(&comp.text);
        out.p1(comp.text_style);
        out.p1(comp.text_halign);
        out.p1(comp.text_valign);
        out.p1(comp.shadowed as u8);
        out.p4(comp.colour);
        out.p1(comp.trans);
    }

    if comp.com_type == IfComponentType::Rect {
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
        out.p2(comp.op_cursors[0].1);
    }
    if comp.op_cursor_count > 1 && comp.op_cursors.len() > 1 {
        out.p1(comp.op_cursors[1].0);
        out.p2(comp.op_cursors[1].1);
    }

    out.pjstr(&comp.pause_text);
    out.p1(comp.drag_deadzone);
    out.p1(comp.drag_deadtime);
    out.p1(comp.drag_renderbehavior);
    out.pjstr(&comp.target_verb);

    let target_mask = (comp.active_properties >> 11) & 0x7F;
    if target_mask != 0 {
        out.p2(comp.target_object_id as u16);
        out.p2(comp.target_enter_cursor as u16);
        out.p2(comp.target_leave_cursor as u16);
    }

    for hook in &comp.hooks {
        encode_hook(&mut out, hook);
    }

    for list in &comp.transmit_lists {
        encode_transmitlist(&mut out, list);
    }

    out.dat
}

#[cfg(since_647)]
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
                    HookArg::Str(s) => {
                        out.p1(1);
                        out.pjstr(s);
                    }
                }
            }
        }
    }
}

#[cfg(since_647)]
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

#[cfg(since_647)]
pub fn parse_interface_text(
    text: &str,
    comp_names: Option<&HashMap<String, u32>>,
) -> Vec<(u32, IfTypeRaw)> {
    const HOOK_NAMES: [&str; 20] = [
        "onload",
        "onmouseover",
        "onmouseleave",
        "ontargetleave",
        "ontargetenter",
        "onvartransmit",
        "oninvtransmit",
        "onstattransmit",
        "ontimer",
        "onop",
        "hook10",
        "hook11",
        "hook12",
        "hook13",
        "hook14",
        "hook15",
        "hook16",
        "hook17",
        "hook18",
        "hook19",
    ];

    let mut components = Vec::new();
    let mut current: Option<(u32, IfTypeRaw)> = None;

    for line in text.lines() {
        let comp_id = if let Some(header) = line.strip_prefix("[com_") {
            let idx_str = header.strip_suffix(']').unwrap_or(header);
            idx_str.parse::<u32>().ok()
        } else if let Some(names) = comp_names {
            line.strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .and_then(|name| names.get(name).copied())
        } else {
            None
        };

        if let Some(idx) = comp_id {
            if let Some(prev) = current.take() {
                components.push(prev);
            }
            let comp = IfTypeRaw {
                version: 255,
                parent_id: u16::MAX,
                target_object_id: -1,
                target_enter_cursor: -1,
                target_leave_cursor: -1,
                ..IfTypeRaw::default()
            };
            current = Some((idx, comp));
            continue;
        }

        let Some((_, ref mut comp)) = current else {
            continue;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();

        match key {
            "type" => {
                comp.com_type = val
                    .trim()
                    .parse::<u8>()
                    .ok()
                    .map(|id| IfComponentType::try_from(id).unwrap())
                    .unwrap_or_else(|| IfComponentType::from_config_str(val.trim()));
            }
            "name" => comp.com_name = Some(val.into()),
            "client_code" => comp.client_code = val.trim().parse().unwrap_or(0),
            "x" => comp.x = val.trim().parse().unwrap_or(0),
            "y" => comp.y = val.trim().parse().unwrap_or(0),
            "width" => comp.width = val.trim().parse().unwrap_or(0),
            "height" => comp.height = val.trim().parse().unwrap_or(0),
            "width_mode" => comp.width_mode = val.trim().parse().unwrap_or(0),
            "height_mode" => comp.height_mode = val.trim().parse().unwrap_or(0),
            "x_mode" => comp.x_mode = val.trim().parse().unwrap_or(0),
            "y_mode" => comp.y_mode = val.trim().parse().unwrap_or(0),
            "parent_id" => comp.parent_id = val.trim().parse().unwrap_or(65535),
            "flags" => comp.flags = val.trim().parse().unwrap_or(0),
            "scroll_width" => comp.scroll_width = val.trim().parse().unwrap_or(0),
            "scroll_height" => comp.scroll = val.trim().parse().unwrap_or(0),
            "no_click_through" => comp.no_click_through = val.trim() == "yes",
            "graphic" => comp.graphic = val.trim().parse().unwrap_or(0),
            "angle_2d" => comp.angle_2d = val.trim().parse().unwrap_or(0),
            "tiling" => comp.tiling = val.trim() == "yes",
            "alpha" => comp.alpha = val.trim() == "yes",
            "trans" => comp.trans = val.trim().parse().unwrap_or(0),
            "outline" => comp.outline = val.trim().parse().unwrap_or(0),
            "graphic_shadow" => comp.graphic_shadow = val.trim().parse().unwrap_or(0),
            "vflip" => comp.vflip = val.trim() == "yes",
            "hflip" => comp.hflip = val.trim() == "yes",
            "model" => comp.model = val.trim().parse().unwrap_or(-1),
            "model_flags" => comp.model_flags = val.trim().parse().unwrap_or(0),
            "model_origin_x" => comp.model_origin_x = val.trim().parse().unwrap_or(0),
            "model_origin_y" => comp.model_origin_y = val.trim().parse().unwrap_or(0),
            "model_xan" => comp.xan = val.trim().parse().unwrap_or(0),
            "model_yan" => comp.yan = val.trim().parse().unwrap_or(0),
            "model_zan" => comp.model_zan = val.trim().parse().unwrap_or(0),
            "model_zoom" => comp.zoom = val.trim().parse().unwrap_or(0),
            "sprite_y_offset" => comp.sprite_y_offset = val.trim().parse().unwrap_or(0),
            "animation" => comp.anim = val.trim().parse().unwrap_or(0),
            "dynamic_width" => comp.dynamic_width = val.trim().parse().unwrap_or(0),
            "dynamic_height" => comp.dynamic_height = val.trim().parse().unwrap_or(0),
            "text_font" => comp.font = val.trim().parse().unwrap_or(-1),
            "text" => comp.text = val.to_string(),
            "text_style" => comp.text_style = val.trim().parse().unwrap_or(0),
            "text_halign" => comp.text_halign = val.trim().parse().unwrap_or(0),
            "text_valign" => comp.text_valign = val.trim().parse().unwrap_or(0),
            "text_shadow" => comp.shadowed = val.trim() == "yes",
            "color" => comp.colour = val.trim().parse().unwrap_or(0),
            "fill" => comp.fill = val.trim() == "yes",
            "line_width" => comp.line_width = val.trim().parse().unwrap_or(0),
            "line_rtl" => comp.line_rtl = val.trim() == "yes",
            "active_properties" => comp.active_properties = val.trim().parse().unwrap_or(0),
            "opkey" => {
                let parts: Vec<&str> = val.split(',').collect();
                if parts.len() == 4 {
                    comp.opkeys.push(OpKey {
                        index: parts[0].trim().parse().unwrap_or(0),
                        rate_duration: parts[1].trim().parse().unwrap_or(0),
                        code: parts[2].trim().parse().unwrap_or(0),
                        mask: parts[3].trim().parse().unwrap_or(0),
                    });
                }
            }
            "op_base" => comp.op_base = val.to_string(),
            "op_cursor_count" => comp.op_cursor_count = val.trim().parse().unwrap_or(0),
            "op_cursor" => {
                let parts: Vec<&str> = val.split(',').collect();
                if parts.len() == 2 {
                    comp.op_cursors.push((
                        parts[0].trim().parse().unwrap_or(0),
                        parts[1].trim().parse().unwrap_or(0),
                    ));
                }
            }
            "pause_text" => comp.pause_text = val.to_string(),
            "drag_deadzone" => comp.drag_deadzone = val.trim().parse().unwrap_or(0),
            "drag_deadtime" => comp.drag_deadtime = val.trim().parse().unwrap_or(0),
            "drag_renderbehavior" => comp.drag_renderbehavior = val.trim().parse().unwrap_or(0),
            "target_verb" => comp.target_verb = val.to_string(),
            "target_object_id" => comp.target_object_id = val.trim().parse().unwrap_or(-1),
            "target_enter_cursor" => comp.target_enter_cursor = val.trim().parse().unwrap_or(-1),
            "target_leave_cursor" => comp.target_leave_cursor = val.trim().parse().unwrap_or(-1),
            "vartransmit_list" => {
                comp.transmit_lists[0] = Some(
                    val.split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect(),
                );
            }
            "invtransmit_list" => {
                comp.transmit_lists[1] = Some(
                    val.split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect(),
                );
            }
            "stattransmit_list" => {
                comp.transmit_lists[2] = Some(
                    val.split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect(),
                );
            }
            "varctransmit_list" => {
                comp.transmit_lists[3] = Some(
                    val.split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect(),
                );
            }
            "varcstrtransmit_list" => {
                comp.transmit_lists[4] = Some(
                    val.split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect(),
                );
            }
            k => {
                if let Some(n_str) = k.strip_prefix("op") {
                    if !n_str.is_empty() && n_str.chars().all(|c| c.is_ascii_digit()) {
                        if let Ok(n) = n_str.parse::<usize>() {
                            if n >= 1 {
                                let idx = n - 1;
                                while comp.ops.len() <= idx {
                                    comp.ops.push(String::new());
                                }
                                comp.ops[idx] = val.to_string();
                            }
                        }
                    }
                } else if let Some(hi) = HOOK_NAMES.iter().position(|&name| name == k) {
                    comp.hooks[hi] = parse_hook_text(val);
                }
            }
        }
    }

    if let Some(prev) = current {
        components.push(prev);
    }

    components
}

#[cfg(since_647)]
fn parse_hook_text(val: &str) -> Option<Vec<HookArg>> {
    let val = val.trim();
    if val.is_empty() {
        return None;
    }
    let mut args = Vec::new();
    let mut chars = val.chars().peekable();

    while chars.peek().is_some() {
        while chars.peek() == Some(&',') {
            chars.next();
        }
        match chars.peek() {
            None => break,
            Some(&'i') => {
                chars.next();
                let num: String = chars.by_ref().take_while(|c| *c != ',').collect();
                if let Ok(v) = num.parse::<i32>() {
                    args.push(HookArg::Int(v));
                }
            }
            Some(&'s') => {
                chars.next();
                if chars.peek() == Some(&'"') {
                    chars.next();
                    let mut s = String::new();
                    loop {
                        match chars.next() {
                            None | Some('"') => break,
                            Some('\\') => {
                                if let Some(c) = chars.next() {
                                    s.push(c);
                                }
                            }
                            Some(c) => s.push(c),
                        }
                    }
                    args.push(HookArg::Str(s));
                } else {
                    let s: String = chars.by_ref().collect();
                    args.push(HookArg::Str(s));
                    break;
                }
            }
            Some(_) => {
                chars.next();
            }
        }
    }

    if args.is_empty() { None } else { Some(args) }
}
