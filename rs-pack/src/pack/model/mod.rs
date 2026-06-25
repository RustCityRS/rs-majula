use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rs_io::Packet;
use rs_io::jag::{JagCompression, JagFile};
use tracing::{debug, warn};

use crate::pack::pack_registry::{PackFile, PackRegistry};
use crate::types::BoneType;

fn walk(path: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let Ok(entries) = std::fs::read_dir(path) else {
        return results;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            results.extend(walk(&path));
        } else {
            results.push(path);
        }
    }
    results
}

fn load_order(path: &Path) -> Vec<u16> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|l| l.trim().parse::<u16>().ok())
        .collect()
}

pub fn pack_models(registry: &PackRegistry, content_dir: &Path, pack_dir: &Path) -> Vec<u8> {
    let models_dir = content_dir.join("models");
    if !models_dir.exists() {
        return Vec::new();
    }

    let model_order = load_order(&pack_dir.join("model.order"));
    let anim_order = load_order(&pack_dir.join("anim.order"));
    let base_order = load_order(&pack_dir.join("base.order"));

    let model_pack = &registry.model;
    let anim_pack =
        PackFile::load(&pack_dir.join("anim.pack")).unwrap_or_else(|_| PackFile::empty());
    let base_pack =
        PackFile::load(&pack_dir.join("base.pack")).unwrap_or_else(|_| PackFile::empty());

    let files = walk(&models_dir);
    let mut file_data: HashMap<String, Vec<u8>> = HashMap::with_capacity(files.len());
    for f in &files {
        if let Some(name) = f.file_name().and_then(|n| n.to_str())
            && let Ok(data) = std::fs::read(f)
        {
            file_data.insert(name.to_string(), data);
        }
    }

    // Base streams
    let mut base_head: Vec<u8> = Vec::new();
    let mut base_type: Vec<u8> = Vec::new();
    let mut base_label: Vec<u8> = Vec::new();

    {
        let count = base_order.len() as u16;
        base_head.push((count >> 8) as u8);
        base_head.push(count as u8);

        let highest = base_order.iter().copied().max().unwrap_or(0);
        base_head.push((highest >> 8) as u8);
        base_head.push(highest as u8);

        for &id in &base_order {
            let Some(name) = base_pack.get_by_id(id) else {
                warn!("missing base pack entry {id}");
                continue;
            };

            let Some(raw) = file_data.get(&format!("{name}.base")) else {
                warn!("missing base file {id} {name}");
                continue;
            };

            let text = String::from_utf8_lossy(raw);
            let props = parse_text_props(&text);

            let mut bone_count = 0u8;
            while props.contains_key(&format!("bone{bone_count}")) {
                bone_count += 1;
            }
            let max_bone = props
                .keys()
                .filter_map(|k| k.strip_prefix("bone")?.parse::<u8>().ok())
                .max();
            if let Some(max) = max_bone {
                if max >= bone_count {
                    panic!(
                        "base {name} has gap in bones: found bone{max} but bone{bone_count} is missing"
                    );
                }
            }

            base_head.push((id >> 8) as u8);
            base_head.push(id as u8);
            base_head.push(bone_count);

            for i in 0..bone_count {
                let value = props.get(&format!("bone{i}")).unwrap();
                let (bone_type, labels) = parse_bone_entry(value);
                base_type.push(bone_type);
                base_label.push(labels.len() as u8);
                base_label.extend_from_slice(&labels);
            }
        }
    }

    // Frame streams
    let mut frame_head: Vec<u8> = Vec::new();
    let mut frame_tran1: Vec<u8> = Vec::new();
    let mut frame_tran2: Vec<u8> = Vec::new();
    let mut frame_del: Vec<u8> = Vec::new();

    {
        let count = anim_order.len() as u16;
        frame_head.push((count >> 8) as u8);
        frame_head.push(count as u8);

        let highest = anim_order.iter().copied().max().unwrap_or(0);
        frame_head.push((highest >> 8) as u8);
        frame_head.push(highest as u8);

        for &id in &anim_order {
            let Some(name) = anim_pack.get_by_id(id) else {
                warn!("missing anim pack entry {id}");
                continue;
            };

            let Some(raw) = file_data.get(&format!("{name}.frame")) else {
                warn!("missing frame file {id} {name}");
                continue;
            };

            let text = String::from_utf8_lossy(raw);
            let props = parse_text_props(&text);

            let delay: u8 = props.get("delay").and_then(|v| v.parse().ok()).unwrap_or(0);
            let base_name = props
                .get("base")
                .unwrap_or_else(|| panic!("missing 'base' property in frame {name}"))
                .as_str();
            let base_id = base_pack
                .get_by_debugname(base_name)
                .unwrap_or_else(|| panic!("unknown base reference '{base_name}' in frame {name}"));

            let mut bone_count = 0u8;
            while props.contains_key(&format!("bone{bone_count}")) {
                bone_count += 1;
            }
            let max_bone = props
                .keys()
                .filter_map(|k| k.strip_prefix("bone")?.parse::<u8>().ok())
                .max();
            if let Some(max) = max_bone {
                if max >= bone_count {
                    panic!(
                        "frame {name} has gap in bones: found bone{max} but bone{bone_count} is missing"
                    );
                }
            }

            let mut head_buf = Packet::new(5 + bone_count as usize);
            head_buf.p2(id);
            head_buf.p2(base_id);
            head_buf.p1(bone_count);
            frame_head.extend_from_slice(&head_buf.data[..head_buf.pos]);

            frame_del.push(delay);

            let mut tran2_buf = Packet::new(bone_count as usize * 6);
            for i in 0..bone_count {
                let value = props.get(&format!("bone{i}")).unwrap();
                let (flags, deltas) = parse_frame_bone(value);
                frame_tran1.push(flags);
                for &d in &deltas {
                    tran2_buf.psmart1or2(d);
                }
            }
            frame_tran2.extend_from_slice(&tran2_buf.data[..tran2_buf.pos]);
        }
    }

    // Model (ob2) streams
    let mut ob_head: Vec<u8> = Vec::new();
    let mut ob_face1: Vec<u8> = Vec::new();
    let mut ob_face2: Vec<u8> = Vec::new();
    let mut ob_face3: Vec<u8> = Vec::new();
    let mut ob_face4: Vec<u8> = Vec::new();
    let mut ob_face5: Vec<u8> = Vec::new();
    let mut ob_point1: Vec<u8> = Vec::new();
    let mut ob_point2: Vec<u8> = Vec::new();
    let mut ob_point3: Vec<u8> = Vec::new();
    let mut ob_point4: Vec<u8> = Vec::new();
    let mut ob_point5: Vec<u8> = Vec::new();
    let mut ob_vertex1: Vec<u8> = Vec::new();
    let mut ob_vertex2: Vec<u8> = Vec::new();
    let mut ob_axis: Vec<u8> = Vec::new();

    {
        let count = model_order.len() as u16;
        ob_head.push((count >> 8) as u8);
        ob_head.push(count as u8);

        for &id in &model_order {
            let Some(name) = model_pack.get_by_id(id) else {
                warn!("missing model pack entry {id}");
                continue;
            };

            let Some(raw) = file_data.get(&format!("{name}.ob2")) else {
                warn!("missing ob2 file {id} {name}");
                continue;
            };
            if raw.len() < 18 {
                continue;
            }

            let t = raw.len() - 18;
            let vertex_count = ((raw[t] as usize) << 8) | raw[t + 1] as usize;
            let face_count = ((raw[t + 2] as usize) << 8) | raw[t + 3] as usize;
            let textured_face_count = raw[t + 4] as usize;
            let has_info = raw[t + 5];
            let has_priorities = raw[t + 6];
            let has_alpha = raw[t + 7];
            let has_face_labels = raw[t + 8];
            let has_vertex_labels = raw[t + 9];
            let vertex_x_length = ((raw[t + 10] as usize) << 8) | raw[t + 11] as usize;
            let vertex_y_length = ((raw[t + 12] as usize) << 8) | raw[t + 13] as usize;
            let vertex_z_length = ((raw[t + 14] as usize) << 8) | raw[t + 15] as usize;
            let face_vertex_length = ((raw[t + 16] as usize) << 8) | raw[t + 17] as usize;

            ob_head.push((id >> 8) as u8);
            ob_head.push(id as u8);
            ob_head.push((vertex_count >> 8) as u8);
            ob_head.push(vertex_count as u8);
            ob_head.push((face_count >> 8) as u8);
            ob_head.push(face_count as u8);
            ob_head.push(textured_face_count as u8);
            ob_head.push(has_info);
            ob_head.push(has_priorities);
            ob_head.push(has_alpha);
            ob_head.push(has_face_labels);
            ob_head.push(has_vertex_labels);

            let mut pos = 0;
            ob_point1.extend_from_slice(&raw[pos..pos + vertex_count]);
            pos += vertex_count;
            ob_vertex2.extend_from_slice(&raw[pos..pos + face_count]);
            pos += face_count;

            if has_priorities == 255 {
                ob_face3.extend_from_slice(&raw[pos..pos + face_count]);
                pos += face_count;
            }
            if has_face_labels == 1 {
                ob_face5.extend_from_slice(&raw[pos..pos + face_count]);
                pos += face_count;
            }
            if has_info == 1 {
                ob_face2.extend_from_slice(&raw[pos..pos + face_count]);
                pos += face_count;
            }
            if has_vertex_labels == 1 {
                ob_point5.extend_from_slice(&raw[pos..pos + vertex_count]);
                pos += vertex_count;
            }
            if has_alpha == 1 {
                ob_face4.extend_from_slice(&raw[pos..pos + face_count]);
                pos += face_count;
            }

            ob_vertex1.extend_from_slice(&raw[pos..pos + face_vertex_length]);
            pos += face_vertex_length;
            ob_face1.extend_from_slice(&raw[pos..pos + face_count * 2]);
            pos += face_count * 2;
            ob_axis.extend_from_slice(&raw[pos..pos + textured_face_count * 6]);
            pos += textured_face_count * 6;
            ob_point2.extend_from_slice(&raw[pos..pos + vertex_x_length]);
            pos += vertex_x_length;
            ob_point3.extend_from_slice(&raw[pos..pos + vertex_y_length]);
            pos += vertex_y_length;
            ob_point4.extend_from_slice(&raw[pos..pos + vertex_z_length]);
        }
    }

    // Assemble Jag in the specific order the client expects
    let mut jag = JagFile::new();
    let entries: [(&str, Vec<u8>); 21] = [
        ("base_label.dat", base_label),
        ("ob_point1.dat", ob_point1),
        ("ob_point2.dat", ob_point2),
        ("ob_point3.dat", ob_point3),
        ("ob_point4.dat", ob_point4),
        ("ob_point5.dat", ob_point5),
        ("ob_head.dat", ob_head),
        ("base_head.dat", base_head),
        ("frame_head.dat", frame_head),
        ("frame_tran1.dat", frame_tran1),
        ("frame_tran2.dat", frame_tran2),
        ("ob_vertex1.dat", ob_vertex1),
        ("ob_vertex2.dat", ob_vertex2),
        ("frame_del.dat", frame_del),
        ("base_type.dat", base_type),
        ("ob_face1.dat", ob_face1),
        ("ob_face2.dat", ob_face2),
        ("ob_face3.dat", ob_face3),
        ("ob_face4.dat", ob_face4),
        ("ob_face5.dat", ob_face5),
        ("ob_axis.dat", ob_axis),
    ];

    for (name, data) in entries {
        jag.write(name, data);
    }

    debug!(
        "Packed {} models, {} anims, {} bases into models Jag",
        model_order.len(),
        anim_order.len(),
        base_order.len()
    );
    jag.build(JagCompression::PerFile)
}

fn parse_text_props(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.to_string(), value.to_string());
        }
    }
    map
}

fn axis_flags_id(name: &str) -> u8 {
    match name {
        "none" => 0,
        "x" => 1,
        "y" => 2,
        "xy" => 3,
        "z" => 4,
        "xz" => 5,
        "yz" => 6,
        "xyz" => 7,
        _ => panic!("unknown axis flags: {name}"),
    }
}

fn parse_frame_bone(s: &str) -> (u8, Vec<i32>) {
    if s == "none" {
        return (0, Vec::new());
    }
    let mut parts = s.splitn(2, ',');
    let axes = parts.next().unwrap().trim();
    let flags = axis_flags_id(axes);
    let deltas = parts.next().map(parse_csv_i32).unwrap_or_default();
    (flags, deltas)
}

fn parse_bone_entry(s: &str) -> (u8, Vec<u8>) {
    let mut parts = s.splitn(2, ',');
    let type_name = parts.next().unwrap().trim();
    let bone_type = BoneType::from_config_str(type_name) as u8;
    let labels = parts.next().map(parse_csv_u8).unwrap_or_default();
    (bone_type, labels)
}

fn parse_csv_u8(s: &str) -> Vec<u8> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(',')
        .filter_map(|v| v.trim().parse::<u8>().ok())
        .collect()
}

fn parse_csv_i32(s: &str) -> Vec<i32> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(',')
        .filter_map(|v| v.trim().parse::<i32>().ok())
        .collect()
}
