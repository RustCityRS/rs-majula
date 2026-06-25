use std::collections::HashMap;
use std::path::Path;

use crate::config_crc;
use crate::pack::pack::collect_if_files;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_hex;
use crate::types::{Font, IfButtonType, IfComparator, IfComponentType, IfScriptOp, PlayerStat};
use anyhow::Result;
use rs_io::crc;
use tracing::debug;

fn name_to_type(name: &str) -> IfComponentType {
    IfComponentType::from_config_str(name)
}

fn name_to_button_type(name: &str) -> IfButtonType {
    IfButtonType::from_config_str(name)
}

fn name_to_comparator(name: &str) -> IfComparator {
    IfComparator::from_config_str(name)
}

fn name_to_script(name: &str) -> IfScriptOp {
    IfScriptOp::from_config_str(name)
}

fn name_to_stat(name: &str) -> PlayerStat {
    PlayerStat::from_config_str(name)
}

fn name_to_font(name: &str) -> Font {
    Font::from_config_str(name)
}

fn get_colour(val: Option<&String>) -> &str {
    val.map(|s| s.as_str()).unwrap_or("0")
}

fn get_string(val: Option<&String>) -> &str {
    val.map(|s| s.as_str()).unwrap_or("")
}

fn get_bool(val: Option<&String>) -> bool {
    val.map(|v| v == "yes").unwrap_or(false)
}

fn get_num<T: std::str::FromStr>(val: Option<&String>, default: T) -> T {
    val.and_then(|v| v.parse::<T>().ok()).unwrap_or(default)
}

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
                _ => {}
            }
        }
    }
    count
}

fn parse_margin<T: std::str::FromStr>(margin: &str, defaultx: T, defaulty: T) -> (T, T) {
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

struct Component {
    root: Option<String>,
    children: Vec<u16>,
    src: HashMap<String, String>,
}

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
        server.pbool(src.get("type").map(|t| t == "overlay").unwrap_or(false));

        let com_type = name_to_type(src.get("type").unwrap());
        client.p1(com_type as u8);
        server.p1(com_type as u8);

        let button_type = name_to_button_type(get_string(src.get("buttontype")));
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
            let comparator = name_to_comparator(parts.first().copied().unwrap_or("")) as u8;
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
                    let op_name = parts.first().copied().unwrap();
                    client.p2(name_to_script(op_name) as u16);
                    server.p2(name_to_script(op_name) as u16);

                    match op_name {
                        "stat_level" | "stat_base_level" | "stat_xp" | "stat_xp_remaining" => {
                            client.p2(name_to_stat(parts.get(1).copied().unwrap()) as u16);
                            server.p2(name_to_stat(parts.get(1).copied().unwrap()) as u16);
                        }
                        "inv_count" | "inv_contains" => {
                            let com_link = registry
                                .interface
                                .get_by_debugname(parts.get(1).copied().unwrap())
                                .unwrap();
                            let obj_link = registry
                                .obj
                                .get_by_debugname(parts.get(2).copied().unwrap())
                                .unwrap();
                            client.p2(com_link);
                            client.p2(obj_link);
                            server.p2(com_link);
                            server.p2(obj_link);
                        }
                        "pushvar" => {
                            let varp_link = registry
                                .varp
                                .get_by_debugname(parts.get(1).copied().unwrap())
                                .unwrap();
                            client.p2(varp_link);
                            server.p2(varp_link);
                        }
                        "testbit" => {
                            let varp_link = registry
                                .varp
                                .get_by_debugname(parts.get(1).copied().unwrap())
                                .unwrap();
                            let bit = parts
                                .get(2)
                                .and_then(|v| v.parse::<u16>().ok())
                                .unwrap_or(0);
                            client.p2(varp_link);
                            client.p2(bit);
                            server.p2(varp_link);
                            server.p2(bit);
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
            client.pbool(get_bool(src.get("hide")));
            server.p2(get_num(src.get("scroll"), 0));
            server.pbool(get_bool(src.get("hide")));

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
                server.p2(x);
                server.p2(y);
            }
        }

        if com_type == IfComponentType::Inv {
            client.pbool(get_bool(src.get("draggable")));
            client.pbool(get_bool(src.get("interactable")));
            client.pbool(get_bool(src.get("usable")));
            server.pbool(get_bool(src.get("draggable")));
            server.pbool(get_bool(src.get("interactable")));
            server.pbool(get_bool(src.get("usable")));

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
                    client.pbool(true);
                    server.pbool(true);
                    let (x, y, sprite) = parse_slot_offset(slot);
                    client.p2(x);
                    client.p2(y);
                    client.pjstr(sprite);
                    server.p2(x);
                    server.p2(y);
                    server.pjstr(sprite);
                } else {
                    client.pbool(false);
                    server.pbool(false);
                }
            }

            for j in 1..=5 {
                client.pjstr(get_string(src.get(&format!("option{j}"))));
                server.pjstr(get_string(src.get(&format!("option{j}"))));
            }
        }

        if com_type == IfComponentType::Rect {
            client.pbool(get_bool(src.get("fill")));
            server.pbool(get_bool(src.get("fill")));
        }

        if com_type == IfComponentType::Text {
            client.pbool(get_bool(src.get("center")));
            client.p1(name_to_font(src.get("font").unwrap()) as u8);
            client.pbool(get_bool(src.get("shadowed")));
            client.pjstr(get_string(src.get("text")));
            client.pjstr(get_string(src.get("activetext")));

            server.pbool(get_bool(src.get("center")));
            server.p1(name_to_font(src.get("font").unwrap()) as u8);
            server.pbool(get_bool(src.get("shadowed")));
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
            client.pbool(get_bool(src.get("center")));
            client.p1(name_to_font(src.get("font").unwrap()) as u8);
            client.pbool(get_bool(src.get("shadowed")));
            parse_hex(get_colour(src.get("colour")), |v| {
                client.p4(v);
            });

            server.pbool(get_bool(src.get("center")));
            server.p1(name_to_font(src.get("font").unwrap()) as u8);
            server.pbool(get_bool(src.get("shadowed")));
            parse_hex(get_colour(src.get("colour")), |v| {
                server.p4(v);
            });

            let (x, y) = parse_margin(src.get("margin").unwrap(), 0, 0);
            client.p2(x);
            client.p2(y);
            server.p2(x);
            server.p2(y);

            client.pbool(get_bool(src.get("interactable")));
            server.pbool(get_bool(src.get("interactable")));

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
        let crc = crc::getcrc(&client.dat, 0, client.dat.len());
        let expected = config_crc::INTERFACE;
        if crc != expected {
            panic!("CRC mismatch ['interface']: Got: {crc}, Expected: {expected}");
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}

fn load_order(path: &Path) -> Result<Vec<u16>> {
    let content = std::fs::read_to_string(path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| l.trim().parse::<u16>().ok())
        .collect())
}
