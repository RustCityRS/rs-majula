use std::collections::HashMap;
#[cfg(rev = "225")]
use std::fmt::Write as _;
use std::path::Path;

use super::config::ModelCategory;
#[cfg(rev = "225")]
use crate::types::BoneType;
use rs_io::Packet;
use rs_io::jag::{JagCompression, JagFile};
use tracing::debug;

#[cfg(rev = "225")]
const JAG_ENTRY_NAMES: [&str; 21] = [
    "base_label.dat",
    "ob_point1.dat",
    "ob_point2.dat",
    "ob_point3.dat",
    "ob_point4.dat",
    "ob_point5.dat",
    "ob_head.dat",
    "base_head.dat",
    "frame_head.dat",
    "frame_tran1.dat",
    "frame_tran2.dat",
    "ob_vertex1.dat",
    "ob_vertex2.dat",
    "frame_del.dat",
    "base_type.dat",
    "ob_face1.dat",
    "ob_face2.dat",
    "ob_face3.dat",
    "ob_face4.dat",
    "ob_face5.dat",
    "ob_axis.dat",
];

#[cfg(rev = "225")]
pub(crate) fn known_hashes() -> Vec<i32> {
    JAG_ENTRY_NAMES.iter().map(|n| JagFile::hash(n)).collect()
}

#[cfg(rev = "225")]
pub fn unpack_models(
    jag: &JagFile,
    output_dir: &Path,
    pack_dir: &Path,
    model_categories: &HashMap<u16, ModelCategory>,
) -> anyhow::Result<()> {
    let models_dir = output_dir.join("models");
    std::fs::create_dir_all(&models_dir)?;

    let raw_dir = models_dir.join("_raw");
    std::fs::create_dir_all(&raw_dir)?;

    let mut jag_order = Vec::new();
    for i in 0..jag.file_count {
        let hash = jag.file_hash(i);
        for name in JAG_ENTRY_NAMES {
            if JagFile::hash(name) == hash {
                jag_order.push(name.to_string());
                break;
            }
        }
    }
    for name in &jag_order {
        if let Some(data) = jag.read(name) {
            std::fs::write(raw_dir.join(name), &data.data)?;
        }
    }
    std::fs::write(raw_dir.join("_jag_order.txt"), jag_order.join("\n") + "\n")?;

    let read_stream =
        |name: &str| -> Vec<u8> { jag.read(name).map(|p| p.data).unwrap_or_default() };

    let base_head = read_stream("base_head.dat");
    let base_type = read_stream("base_type.dat");
    let base_label = read_stream("base_label.dat");
    let frame_head = read_stream("frame_head.dat");
    let frame_tran1 = read_stream("frame_tran1.dat");
    let frame_tran2 = read_stream("frame_tran2.dat");
    let frame_del = read_stream("frame_del.dat");
    let ob_head = read_stream("ob_head.dat");
    let ob_point1 = read_stream("ob_point1.dat");
    let ob_point2 = read_stream("ob_point2.dat");
    let ob_point3 = read_stream("ob_point3.dat");
    let ob_point4 = read_stream("ob_point4.dat");
    let ob_point5 = read_stream("ob_point5.dat");
    let ob_vertex1 = read_stream("ob_vertex1.dat");
    let ob_vertex2 = read_stream("ob_vertex2.dat");
    let ob_face1 = read_stream("ob_face1.dat");
    let ob_face2 = read_stream("ob_face2.dat");
    let ob_face3 = read_stream("ob_face3.dat");
    let ob_face4 = read_stream("ob_face4.dat");
    let ob_face5 = read_stream("ob_face5.dat");
    let ob_axis = read_stream("ob_axis.dat");

    let existing_model_names = load_existing_pack(pack_dir, "model");

    let base_out = models_dir.join("_unpack").join("base");
    std::fs::create_dir_all(&base_out)?;
    let base_count = extract_bases(&base_head, &base_type, &base_label, &base_out, pack_dir)?;
    debug!("  Extracted {} bases", base_count);

    let frame_out = models_dir.join("_unpack").join("frame");
    std::fs::create_dir_all(&frame_out)?;
    let frame_count = extract_frames(
        &frame_head,
        &frame_tran1,
        &frame_tran2,
        &frame_del,
        &frame_out,
        pack_dir,
    )?;
    debug!("  Extracted {} frames", frame_count);

    let ob2_count = extract_ob2(
        &ob_head,
        &ob_point1,
        &ob_point2,
        &ob_point3,
        &ob_point4,
        &ob_point5,
        &ob_vertex1,
        &ob_vertex2,
        &ob_face1,
        &ob_face2,
        &ob_face3,
        &ob_face4,
        &ob_face5,
        &ob_axis,
        &models_dir,
        pack_dir,
        &existing_model_names,
        model_categories,
    )?;
    debug!("  Extracted {} ob2 models", ob2_count);

    debug!("Unpacked models JAG");
    Ok(())
}

pub fn pack_models_from_raw(raw_dir: &Path) -> Vec<u8> {
    let order: Vec<String> = std::fs::read_to_string(raw_dir.join("_jag_order.txt"))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    let mut jag = JagFile::new();
    for name in &order {
        if let Ok(data) = std::fs::read(raw_dir.join(name)) {
            jag.write(name, data);
        }
    }
    jag.build(JagCompression::PerFile)
}

pub(crate) fn load_existing_pack(pack_dir: &Path, name: &str) -> HashMap<u16, String> {
    let mut map = HashMap::new();
    if let Ok(text) = std::fs::read_to_string(pack_dir.join(format!("{name}.pack"))) {
        for line in text.lines() {
            if let Some((id_str, n)) = line.split_once('=')
                && let Ok(id) = id_str.parse::<u16>()
            {
                map.insert(id, n.to_string());
            }
        }
    }
    map
}

#[cfg(rev = "225")]
fn take(buf: &mut Packet, n: usize) -> Vec<u8> {
    let end = (buf.pos + n).min(buf.data.len());
    let result = buf.data[buf.pos..end].to_vec();
    buf.pos = end;
    result
}

#[cfg(rev = "225")]
fn axis_flags_name(flags: u8) -> &'static str {
    match flags & 0x7 {
        1 => "x",
        2 => "y",
        3 => "xy",
        4 => "z",
        5 => "xz",
        6 => "yz",
        7 => "xyz",
        _ => "none",
    }
}

#[cfg(rev = "225")]
fn extract_bases(
    head: &[u8],
    type_data: &[u8],
    label_data: &[u8],
    out_dir: &Path,
    pack_dir: &Path,
) -> anyhow::Result<usize> {
    if head.len() < 4 {
        return Ok(0);
    }
    let mut h = Packet::from(head.to_vec());
    let mut t = Packet::from(type_data.to_vec());
    let mut l = Packet::from(label_data.to_vec());

    let count = h.g2() as usize;
    let _highest = h.g2();

    let mut order_lines = Vec::new();

    for _ in 0..count {
        let id = h.g2();
        let type_length = h.g1() as usize;

        let name = format!("base_{id}");
        order_lines.push(id.to_string());

        let t_bytes = take(&mut t, type_length);

        let mut labels: Vec<Vec<u8>> = Vec::with_capacity(type_length);
        for _ in 0..type_length {
            let group_count = l.g1() as usize;
            let mut group = Vec::with_capacity(group_count);
            for _ in 0..group_count {
                group.push(l.g1());
            }
            labels.push(group);
        }

        let mut text = String::new();
        writeln!(text, "[{name}]").unwrap();

        for (i, (bone_type, group)) in t_bytes.iter().zip(labels.iter()).enumerate() {
            let type_name = BoneType::try_from(*bone_type)
                .expect("unknown bone type")
                .config_str();
            let vals: Vec<String> = group.iter().map(|b| b.to_string()).collect();
            if vals.is_empty() {
                writeln!(text, "bone{}={}", i, type_name).unwrap();
            } else {
                writeln!(text, "bone{}={},{}", i, type_name, vals.join(",")).unwrap();
            }
        }

        std::fs::write(out_dir.join(format!("{name}.base")), &text)?;
    }

    std::fs::write(pack_dir.join("base.order"), order_lines.join("\n") + "\n")?;
    let max_base_id = order_lines
        .iter()
        .filter_map(|s| s.parse::<u16>().ok())
        .max()
        .unwrap_or(0);
    let mut base_pack_lines: Vec<String> = Vec::new();
    for id in 0..=max_base_id {
        base_pack_lines.push(format!("{id}=base_{id}"));
    }
    std::fs::write(
        pack_dir.join("base.pack"),
        base_pack_lines.join("\n") + "\n",
    )?;
    Ok(count)
}

#[cfg(rev = "225")]
fn extract_frames(
    head: &[u8],
    tran1: &[u8],
    tran2: &[u8],
    del: &[u8],
    out_dir: &Path,
    pack_dir: &Path,
) -> anyhow::Result<usize> {
    if head.len() < 4 {
        return Ok(0);
    }
    let mut h = Packet::from(head.to_vec());
    let mut t1 = Packet::from(tran1.to_vec());
    let mut t2 = Packet::from(tran2.to_vec());
    let mut d = Packet::from(del.to_vec());

    let count = h.g2() as usize;
    let _highest = h.g2();

    let mut order_lines = Vec::new();

    for _ in 0..count {
        let id = h.g2();
        let delay = d.g1();
        let base_id = h.g2();
        let group_count = h.g1() as usize;

        let name = format!("anim_{id}");
        order_lines.push(id.to_string());

        let mut text = String::new();
        writeln!(text, "[{name}]").unwrap();
        writeln!(text, "delay={delay}").unwrap();
        writeln!(text, "base=base_{base_id}").unwrap();

        for i in 0..group_count {
            let flags = t1.g1();
            if flags == 0 {
                writeln!(text, "bone{i}=none").unwrap();
                continue;
            }
            let axes = axis_flags_name(flags);
            let mut vals = vec![axes.to_string()];
            if flags & 0x1 != 0 {
                vals.push(t2.gsmart1or2().to_string());
            }
            if flags & 0x2 != 0 {
                vals.push(t2.gsmart1or2().to_string());
            }
            if flags & 0x4 != 0 {
                vals.push(t2.gsmart1or2().to_string());
            }
            writeln!(text, "bone{}={}", i, vals.join(",")).unwrap();
        }

        std::fs::write(out_dir.join(format!("{name}.frame")), &text)?;
    }

    std::fs::write(pack_dir.join("anim.order"), order_lines.join("\n") + "\n")?;
    let max_anim_id = order_lines
        .iter()
        .filter_map(|s| s.parse::<u16>().ok())
        .max()
        .unwrap_or(0);
    let mut anim_pack_lines: Vec<String> = Vec::new();
    for id in 0..=max_anim_id {
        anim_pack_lines.push(format!("{id}=anim_{id}"));
    }
    std::fs::write(
        pack_dir.join("anim.pack"),
        anim_pack_lines.join("\n") + "\n",
    )?;
    Ok(count)
}

fn model_subdir(category: Option<&ModelCategory>) -> &'static str {
    match category {
        Some(ModelCategory::Npc) => "npc",
        Some(ModelCategory::Obj) => "obj",
        Some(ModelCategory::Loc) => "loc",
        Some(ModelCategory::Spotanim) => "spotanim",
        Some(ModelCategory::IdkMan) => "human/man",
        Some(ModelCategory::IdkWoman) => "human/woman",
        None => "_unpack",
    }
}

#[cfg(rev = "225")]
fn extract_ob2(
    head: &[u8],
    point1: &[u8],
    point2: &[u8],
    point3: &[u8],
    point4: &[u8],
    point5: &[u8],
    vertex1: &[u8],
    vertex2: &[u8],
    face1: &[u8],
    face2: &[u8],
    face3: &[u8],
    face4: &[u8],
    face5: &[u8],
    axis: &[u8],
    models_dir: &Path,
    pack_dir: &Path,
    existing_names: &HashMap<u16, String>,
    categories: &HashMap<u16, ModelCategory>,
) -> anyhow::Result<usize> {
    if head.len() < 2 {
        return Ok(0);
    }
    let mut oh = Packet::from(head.to_vec());
    let mut op1 = Packet::from(point1.to_vec());
    let mut op2 = Packet::from(point2.to_vec());
    let mut op3 = Packet::from(point3.to_vec());
    let mut op4 = Packet::from(point4.to_vec());
    let mut op5 = Packet::from(point5.to_vec());
    let mut ov1 = Packet::from(vertex1.to_vec());
    let mut ov2 = Packet::from(vertex2.to_vec());
    let mut of1 = Packet::from(face1.to_vec());
    let mut of2 = Packet::from(face2.to_vec());
    let mut of3 = Packet::from(face3.to_vec());
    let mut of4 = Packet::from(face4.to_vec());
    let mut of5 = Packet::from(face5.to_vec());
    let mut oa = Packet::from(axis.to_vec());

    let model_count = oh.g2() as usize;
    let mut order_lines = Vec::new();
    let mut pack_lines = Vec::new();

    for _ in 0..model_count {
        if oh.remaining() < 12 {
            break;
        }
        let id = oh.g2();
        let vertex_count = oh.g2() as usize;
        let face_count = oh.g2() as usize;
        let textured_face_count = oh.g1() as usize;
        let has_info = oh.g1();
        let has_priorities = oh.g1();
        let has_alpha = oh.g1();
        let has_face_labels = oh.g1();
        let has_vertex_labels = oh.g1();

        let name = existing_names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("model_{id}"));
        order_lines.push(id.to_string());
        pack_lines.push(format!("{id}={name}"));

        let p1d = take(&mut op1, vertex_count);
        let v2d = take(&mut ov2, face_count);
        let f3d = if has_priorities == 255 {
            take(&mut of3, face_count)
        } else {
            vec![]
        };
        let f5d = if has_face_labels == 1 {
            take(&mut of5, face_count)
        } else {
            vec![]
        };
        let f2d = if has_info == 1 {
            take(&mut of2, face_count)
        } else {
            vec![]
        };
        let p5d = if has_vertex_labels == 1 {
            take(&mut op5, vertex_count)
        } else {
            vec![]
        };
        let f4d = if has_alpha == 1 {
            take(&mut of4, face_count)
        } else {
            vec![]
        };
        let v1_len = compute_face_vertex_len(face_count, &v2d);
        let v1d = take(&mut ov1, v1_len);
        let f1d = take(&mut of1, face_count * 2);
        let axd = take(&mut oa, textured_face_count * 6);
        let vx_len = compute_vertex_delta_len(vertex_count, &p1d, 0);
        let vy_len = compute_vertex_delta_len(vertex_count, &p1d, 1);
        let vz_len = compute_vertex_delta_len(vertex_count, &p1d, 2);
        let vxd = take(&mut op2, vx_len);
        let vyd = take(&mut op3, vy_len);
        let vzd = take(&mut op4, vz_len);

        let mut out = Vec::new();
        out.extend_from_slice(&p1d);
        out.extend_from_slice(&v2d);
        out.extend_from_slice(&f3d);
        out.extend_from_slice(&f5d);
        out.extend_from_slice(&f2d);
        out.extend_from_slice(&p5d);
        out.extend_from_slice(&f4d);
        out.extend_from_slice(&v1d);
        out.extend_from_slice(&f1d);
        out.extend_from_slice(&axd);
        out.extend_from_slice(&vxd);
        out.extend_from_slice(&vyd);
        out.extend_from_slice(&vzd);
        out.extend_from_slice(&(vertex_count as u16).to_be_bytes());
        out.extend_from_slice(&(face_count as u16).to_be_bytes());
        out.push(textured_face_count as u8);
        out.push(has_info);
        out.push(has_priorities);
        out.push(has_alpha);
        out.push(has_face_labels);
        out.push(has_vertex_labels);
        out.extend_from_slice(&(vx_len as u16).to_be_bytes());
        out.extend_from_slice(&(vy_len as u16).to_be_bytes());
        out.extend_from_slice(&(vz_len as u16).to_be_bytes());
        out.extend_from_slice(&(v1d.len() as u16).to_be_bytes());

        let out_dir = models_dir.join(model_subdir(categories.get(&id)));
        std::fs::create_dir_all(&out_dir)?;
        std::fs::write(out_dir.join(format!("{name}.ob2")), &out)?;
    }

    let max_id = order_lines
        .iter()
        .filter_map(|s| s.parse::<u16>().ok())
        .max()
        .unwrap_or(0);
    let mut full_pack = Vec::new();
    for id in 0..=max_id {
        let name = existing_names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("model_{id}"));
        full_pack.push(format!("{id}={name}"));
    }
    std::fs::write(pack_dir.join("model.order"), order_lines.join("\n") + "\n")?;
    std::fs::write(pack_dir.join("model.pack"), full_pack.join("\n") + "\n")?;
    Ok(model_count)
}

#[cfg(rev = "225")]
fn compute_face_vertex_len(face_count: usize, v2: &[u8]) -> usize {
    let mut len = 0;
    for &v in v2.iter().take(face_count.min(v2.len())) {
        match v {
            1 | 2 => len += 4,
            _ => len += 6,
        }
    }
    len
}

#[cfg(rev = "225")]
fn compute_vertex_delta_len(vertex_count: usize, p1: &[u8], axis: u8) -> usize {
    let mut len = 0;
    for &p in p1.iter().take(vertex_count.min(p1.len())) {
        if p & (1 << axis) != 0 {
            len += 2;
        } else if p & (1 << (axis + 3)) != 0 {
            len += 1;
        }
    }
    len
}

#[cfg(since_244)]
pub fn unpack_models(
    cache: &rs_io::js5::Js5Store,
    output_dir: &Path,
    pack_dir: &Path,
    model_categories: &HashMap<u16, ModelCategory>,
) -> anyhow::Result<()> {
    let models_dir = output_dir.join("models");
    std::fs::create_dir_all(&models_dir)?;

    let existing_names = load_existing_pack(pack_dir, "model");
    let count = cache.count(1);

    let mut pack_lines = Vec::with_capacity(count);
    let mut written = 0;
    for id in 0..count {
        let id16 = id as u16;
        let name = existing_names
            .get(&id16)
            .cloned()
            .unwrap_or_else(|| format!("model_{id}"));
        pack_lines.push(format!("{id}={name}"));

        let Some(data) = cache.read(1, id, true).filter(|d| !d.is_empty()) else {
            continue;
        };

        let out_dir = models_dir.join(model_subdir(model_categories.get(&id16)));
        std::fs::create_dir_all(&out_dir)?;
        std::fs::write(out_dir.join(format!("{name}.ob2")), &data)?;
        written += 1;
    }

    let order: Vec<String> = (0..count).map(|id| id.to_string()).collect();
    std::fs::write(pack_dir.join("model.order"), order.join("\n") + "\n")?;
    std::fs::write(pack_dir.join("model.pack"), pack_lines.join("\n") + "\n")?;

    debug!("Unpacked {written} models");
    Ok(())
}

#[cfg(since_244)]
pub fn unpack_anims(
    cache: &rs_io::js5::Js5Store,
    output_dir: &Path,
    pack_dir: &Path,
) -> anyhow::Result<()> {
    let anim_dir = output_dir.join("models").join("anim");
    std::fs::create_dir_all(&anim_dir)?;

    let count = cache.count(2);

    // Seed with the frames config already named (those referenced by seqs), then
    // add every frame declared in an anim set head.
    let mut frame_names = load_existing_pack(pack_dir, "anim");
    let mut written = 0;

    for set_id in 0..count {
        let Some(set) = cache.read(2, set_id, true).filter(|d| !d.is_empty()) else {
            continue;
        };
        std::fs::write(anim_dir.join(format!("anim_{set_id}.anim")), &set)?;
        written += 1;

        for frame_id in anim_set_frame_ids(&set) {
            frame_names
                .entry(frame_id)
                .or_insert_with(|| format!("anim_{frame_id}"));
        }
    }

    let max_frame = frame_names.keys().copied().max().unwrap_or(0);
    let mut anim_pack = Vec::with_capacity(max_frame as usize + 1);
    for id in 0..=max_frame {
        let name = frame_names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("anim_{id}"));
        anim_pack.push(format!("{id}={name}"));
    }
    std::fs::write(pack_dir.join("anim.pack"), anim_pack.join("\n") + "\n")?;

    let base_pack: Vec<String> = (0..count).map(|id| format!("{id}=base_{id}")).collect();
    std::fs::write(pack_dir.join("base.pack"), base_pack.join("\n") + "\n")?;

    debug!("Unpacked {written} anim sets");
    Ok(())
}

#[cfg(since_244)]
fn anim_set_frame_ids(set: &[u8]) -> Vec<u16> {
    if set.len() < 2 {
        return Vec::new();
    }
    let mut head = Packet::from(set.to_vec());
    let frame_count = head.g2() as usize;
    let mut ids = Vec::with_capacity(frame_count);
    for _ in 0..frame_count {
        if head.pos + 3 > set.len() {
            break;
        }
        let frame_id = head.g2();
        let _label_count = head.g1();
        ids.push(frame_id);
    }
    ids
}
