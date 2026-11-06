#[cfg(before_647)]
use super::config::ModelCategory;
#[cfg(all(since_244, before_647))]
use crate::PACK_DIR;
#[cfg(since_244)]
use crate::archive;
#[cfg(before_647)]
use crate::archive::JagArchive;
#[cfg(since_647)]
use crate::cache::seq_frame::{AnimFrame, SkeletonBase};
#[cfg(since_647)]
use crate::pack::pack_registry::{PackFile, PackRegistry};
#[cfg(rev = "225")]
use crate::types::BoneType;
#[cfg(since_647)]
use anyhow::{Context, Result};
#[cfg(before_647)]
use rs_io::Packet;
#[cfg(before_647)]
use rs_io::jag::JagFile;
#[cfg(since_244)]
use rs_io::js5::Js5Store;
#[cfg(since_647)]
use rs_io::js5::compression::{self, Container};
#[cfg(since_647)]
use serde_json::json;
#[cfg(before_647)]
use std::collections::HashMap;
#[cfg(all(since_244, before_647))]
use std::collections::HashSet;
#[cfg(rev = "225")]
use std::fmt::Write as _;
use std::fs;
#[cfg(since_647)]
use std::fs::File;
#[cfg(since_647)]
use std::io::Write;
use std::path::Path;
#[cfg(before_647)]
use tracing::debug;
#[cfg(since_647)]
use tracing::info;

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
    fs::create_dir_all(&models_dir)?;

    let raw_dir = models_dir.join("_raw");
    fs::create_dir_all(&raw_dir)?;

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
            fs::write(raw_dir.join(name), &data.data)?;
        }
    }
    fs::write(raw_dir.join("_jag_order.txt"), jag_order.join("\n") + "\n")?;

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
    fs::create_dir_all(&base_out)?;
    let base_count = extract_bases(&base_head, &base_type, &base_label, &base_out, pack_dir)?;
    debug!("  Extracted {} bases", base_count);

    let frame_out = models_dir.join("_unpack").join("frame");
    fs::create_dir_all(&frame_out)?;
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

#[cfg(before_647)]
pub fn pack_models_from_raw(raw_dir: &Path) -> Vec<u8> {
    let order: Vec<String> = fs::read_to_string(raw_dir.join("_jag_order.txt"))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    let mut jag = JagFile::new();
    for name in &order {
        if let Ok(data) = fs::read(raw_dir.join(name)) {
            jag.write(name, data);
        }
    }
    jag.build(JagArchive::Models.compression())
}

#[cfg(before_647)]
pub(crate) fn load_existing_pack(pack_dir: &Path, name: &str) -> HashMap<u16, String> {
    let mut map = HashMap::new();
    if let Ok(text) = fs::read_to_string(pack_dir.join(format!("{name}.pack"))) {
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

        fs::write(out_dir.join(format!("{name}.base")), &text)?;
    }

    fs::write(pack_dir.join("base.order"), order_lines.join("\n") + "\n")?;
    let max_base_id = order_lines
        .iter()
        .filter_map(|s| s.parse::<u16>().ok())
        .max()
        .unwrap_or(0);
    let mut base_pack_lines: Vec<String> = Vec::new();
    for id in 0..=max_base_id {
        base_pack_lines.push(format!("{id}=base_{id}"));
    }
    fs::write(
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

        fs::write(out_dir.join(format!("{name}.frame")), &text)?;
    }

    fs::write(pack_dir.join("anim.order"), order_lines.join("\n") + "\n")?;
    let max_anim_id = order_lines
        .iter()
        .filter_map(|s| s.parse::<u16>().ok())
        .max()
        .unwrap_or(0);
    let mut anim_pack_lines: Vec<String> = Vec::new();
    for id in 0..=max_anim_id {
        anim_pack_lines.push(format!("{id}=anim_{id}"));
    }
    fs::write(
        pack_dir.join("anim.pack"),
        anim_pack_lines.join("\n") + "\n",
    )?;
    Ok(count)
}

#[cfg(before_647)]
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
        fs::create_dir_all(&out_dir)?;
        fs::write(out_dir.join(format!("{name}.ob2")), &out)?;
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
    fs::write(pack_dir.join("model.order"), order_lines.join("\n") + "\n")?;
    fs::write(pack_dir.join("model.pack"), full_pack.join("\n") + "\n")?;
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

#[cfg(all(since_244, before_647))]
pub fn unpack_models(
    cache: &mut Js5Store,
    output_dir: &Path,
    pack_dir: &Path,
    model_categories: &HashMap<u16, ModelCategory>,
) -> anyhow::Result<()> {
    let models_dir = output_dir.join("models");
    fs::create_dir_all(&models_dir)?;

    let existing_names = load_existing_pack(Path::new(PACK_DIR), "model");
    let count = cache.group_count(archive::MODELS).unwrap_or(0) as usize;

    let mut pack_lines = Vec::with_capacity(count);
    let mut written = 0;
    for id in 0..count {
        let id16 = id as u16;
        let name = existing_names
            .get(&id16)
            .cloned()
            .unwrap_or_else(|| format!("model_{id}"));
        pack_lines.push(format!("{id}={name}"));

        let Some(data) = cache
            .read_group_raw_decompressed(archive::MODELS, id as u32)
            .ok()
            .flatten()
            .filter(|d| !d.is_empty())
        else {
            continue;
        };

        let out_dir = models_dir.join(model_subdir(model_categories.get(&id16)));
        fs::create_dir_all(&out_dir)?;
        fs::write(out_dir.join(format!("{name}.ob2")), &data)?;
        written += 1;
    }

    let order: Vec<String> = (0..count).map(|id| id.to_string()).collect();
    fs::write(pack_dir.join("model.order"), order.join("\n") + "\n")?;
    fs::write(pack_dir.join("model.pack"), pack_lines.join("\n") + "\n")?;

    debug!("Unpacked {written} models");
    Ok(())
}

#[cfg(all(since_244, before_647))]
pub fn ob2_textures(data: &[u8]) -> Vec<u16> {
    if data.len() < 18 {
        return Vec::new();
    }
    let mut h = Packet::from(data[data.len() - 18..].to_vec());
    let vertex_count = h.g2() as usize;
    let face_count = h.g2() as usize;
    let textured_face_count = h.g1() as usize;
    let has_info = h.g1();
    let priority = h.g1();
    let has_alpha = h.g1();
    let has_face_labels = h.g1();
    let has_vertex_labels = h.g1();
    h.pos += 2;
    h.pos += 2;
    h.pos += 2;
    let face_orient_len = h.g2() as usize;
    if has_info != 1 || textured_face_count == 0 {
        return Vec::new();
    }
    let mut pos = vertex_count + face_count;
    if priority == 255 {
        pos += face_count;
    }
    if has_face_labels == 1 {
        pos += face_count;
    }
    let face_infos_off = pos;
    pos += face_count;
    if has_vertex_labels == 1 {
        pos += vertex_count;
    }
    if has_alpha == 1 {
        pos += face_count;
    }
    pos += face_orient_len;
    let face_colours_off = pos;
    if face_colours_off + face_count * 2 > data.len() {
        return Vec::new();
    }
    let mut textures = Vec::new();
    for f in 0..face_count {
        if data[face_infos_off + f] & 0x3 > 1 {
            textures.push(u16::from_be_bytes([
                data[face_colours_off + f * 2],
                data[face_colours_off + f * 2 + 1],
            ]));
        }
    }
    textures
}

#[cfg(all(since_244, before_647))]
pub fn build_model_textures(cache: &mut Js5Store) -> HashMap<u16, HashSet<u16>> {
    let mut map = HashMap::new();
    for id in 0..cache.group_count(archive::MODELS).unwrap_or(0) as usize {
        let Some(data) = cache
            .read_group_raw_decompressed(archive::MODELS, id as u32)
            .ok()
            .flatten()
            .filter(|d| !d.is_empty())
        else {
            continue;
        };
        let textures = ob2_textures(&data);
        if !textures.is_empty() {
            map.insert(id as u16, textures.into_iter().collect());
        }
    }
    map
}

#[cfg(all(since_244, before_647))]
pub fn unpack_anims(
    cache: &mut Js5Store,
    output_dir: &Path,
    pack_dir: &Path,
) -> anyhow::Result<()> {
    let anim_dir = output_dir.join("models").join("anim");
    fs::create_dir_all(&anim_dir)?;

    let count = cache.group_count(archive::ANIMS).unwrap_or(0) as usize;

    // Seed with the frames config already named (those referenced by seqs), then
    // add every frame declared in an anim set head.
    let mut frame_names = load_existing_pack(pack_dir, "anim");
    let mut written = 0;

    for set_id in 0..count {
        let Some(set) = cache
            .read_group_raw_decompressed(archive::ANIMS, set_id as u32)
            .ok()
            .flatten()
            .filter(|d| !d.is_empty())
        else {
            continue;
        };
        fs::write(anim_dir.join(format!("anim_{set_id}.anim")), &set)?;
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
    fs::write(pack_dir.join("anim.pack"), anim_pack.join("\n") + "\n")?;

    let base_pack: Vec<String> = (0..count).map(|id| format!("{id}=base_{id}")).collect();
    fs::write(pack_dir.join("base.pack"), base_pack.join("\n") + "\n")?;

    debug!("Unpacked {written} anim sets");
    Ok(())
}

#[cfg(all(since_244, before_647))]
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

#[cfg(since_647)]
const CATEGORY_SUFFIXES: &[(&str, &str)] = &[
    ("_npc_head", "npc_head"),
    ("_obj_wear", "obj_wear"),
    ("_obj_head", "obj_head"),
    ("_idk_head", "idk_head"),
    ("_spotanim", "spotanim"),
    ("_npc", "npc"),
    ("_obj", "obj"),
    ("_loc", "loc"),
    ("_idk", "idk"),
];

#[cfg(since_647)]
fn categorize(name: &str) -> &'static str {
    for &(suffix, category) in CATEGORY_SUFFIXES {
        if name.ends_with(suffix) {
            return category;
        }
    }
    "uncategorized"
}

#[cfg(since_647)]
pub fn unpack_models(
    cache_dir: &Path,
    registry: &PackRegistry,
    output_dir: &Path,
) -> Result<UnpackModelStats> {
    let mut store = Js5Store::open(cache_dir)?;
    let group_count = store.group_count(archive::MODELS).unwrap_or(0);

    // Create category directories
    let categories = [
        "loc",
        "npc",
        "npc_head",
        "obj",
        "obj_wear",
        "obj_head",
        "spotanim",
        "idk",
        "idk_head",
        "uncategorized",
    ];
    for cat in &categories {
        fs::create_dir_all(output_dir.join(cat))?;
    }

    let mut groups = Vec::with_capacity(group_count as usize);
    let mut stats = UnpackModelStats::default();

    for gid in 0..group_count {
        let Some(raw) = store.read_raw_group_bytes(archive::MODELS, gid)? else {
            continue;
        };
        if raw.len() < 5 {
            continue;
        }

        let container = Container::parse(&raw)?;
        let decompressed = match compression::decompress(&raw) {
            Ok(d) => d,
            Err(_) => {
                stats.failed += 1;
                continue;
            }
        };

        let name = registry
            .model
            .get_by_id(gid)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("model_{}", gid));

        let category = categorize(&name);
        let file_path = output_dir.join(category).join(format!("{name}.ob2"));
        fs::write(&file_path, &decompressed)?;

        groups.push(json!({
            "group_id": gid,
            "name": name,
            "category": category,
            "compression": container.compression as u8,
            "version": container.version.unwrap_or(0),
            "decompressed_len": decompressed.len() as u32,
        }));

        match category {
            "loc" => stats.loc += 1,
            "npc" => stats.npc += 1,
            "npc_head" => stats.npc_head += 1,
            "obj" => stats.obj += 1,
            "obj_wear" => stats.obj_wear += 1,
            "obj_head" => stats.obj_head += 1,
            "spotanim" => stats.spotanim += 1,
            "idk" => stats.idk += 1,
            "idk_head" => stats.idk_head += 1,
            _ => stats.uncategorized += 1,
        }

        if (stats.total() % 10000) == 0 && stats.total() > 0 {
            info!("  {} models unpacked...", stats.total());
        }
    }

    let manifest = json!({ "archive_id": archive::MODELS, "groups": groups });
    let manifest_path = output_dir.join("manifest.json");
    let json = serde_json::to_string_pretty(&manifest).context("serialize manifest")?;
    fs::write(&manifest_path, json)?;

    Ok(stats)
}

#[cfg(since_647)]
#[derive(Debug, Default)]
pub struct UnpackModelStats {
    pub loc: u32,
    pub npc: u32,
    pub npc_head: u32,
    pub obj: u32,
    pub obj_wear: u32,
    pub obj_head: u32,
    pub spotanim: u32,
    pub idk: u32,
    pub idk_head: u32,
    pub uncategorized: u32,
    pub failed: u32,
}

#[cfg(since_647)]
impl UnpackModelStats {
    pub fn total(&self) -> u32 {
        self.loc
            + self.npc
            + self.npc_head
            + self.obj
            + self.obj_wear
            + self.obj_head
            + self.spotanim
            + self.idk
            + self.idk_head
            + self.uncategorized
    }
}

#[cfg(since_647)]
fn load_or_create_pack(pack_dir: &Path, name: &str) -> PackFile {
    let path = pack_dir.join(format!("{}.pack", name));
    if path.exists() {
        PackFile::load(&path).unwrap_or_else(|_| PackFile::empty())
    } else {
        PackFile::empty()
    }
}

#[cfg(since_647)]
fn name_or_fallback(pack: &PackFile, id: u32, prefix: &str) -> String {
    pack.get_by_id(id)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}_{}", prefix, id))
}

#[cfg(since_647)]
pub fn unpack_bases(store: &mut Js5Store, pack_dir: &Path, output_dir: &Path) -> Result<u32> {
    let group_count = store.group_count(archive::BASES).unwrap_or(0);
    if group_count == 0 {
        return Ok(0);
    }

    let pack = load_or_create_pack(pack_dir, "base");
    fs::create_dir_all(output_dir)?;
    let mut file = File::create(output_dir.join("all.base"))?;
    writeln!(file, "# name bone_count [type:parent:label:groups ...]")?;
    let mut total = 0u32;

    for gid in 0..group_count {
        let Some(group) = store.read_group(archive::BASES, gid)? else {
            continue;
        };
        for entry in &group.entries {
            if entry.data().is_empty() {
                continue;
            }
            let base = SkeletonBase::decode(entry.data());
            let name = name_or_fallback(&pack, gid, "base");
            let mut line = format!("{} {}", name, base.bone_count);
            for i in 0..base.bone_count as usize {
                let p = if base.parent_flags[i] { 1 } else { 0 };
                let grps: Vec<String> =
                    base.label_groups[i].iter().map(|g| g.to_string()).collect();
                let grp_str = if grps.is_empty() {
                    "-".to_string()
                } else {
                    grps.join(";")
                };
                line.push_str(&format!(
                    " {}:{}:{}:{}",
                    base.transform_types[i], p, base.labels[i], grp_str
                ));
            }
            writeln!(file, "{}", line)?;
            total += 1;
        }
    }

    info!("  {} skeleton bases unpacked", total);
    Ok(total)
}

#[cfg(since_647)]
pub fn unpack_anims(store: &mut Js5Store, pack_dir: &Path, output_dir: &Path) -> Result<u32> {
    let group_count = store.group_count(archive::ANIMS).unwrap_or(0);
    if group_count == 0 {
        return Ok(0);
    }

    let pack = load_or_create_pack(pack_dir, "anim");
    let anim_dir = output_dir.join("_unpack").join("animations");
    fs::create_dir_all(&anim_dir)?;
    let mut total = 0u32;

    for gid in 0..group_count {
        let Some(group) = store.read_group(archive::ANIMS, gid)? else {
            continue;
        };
        let anim_name = name_or_fallback(&pack, gid, "anim");
        let file_path = anim_dir.join(format!("{anim_name}.anim"));
        let mut file = File::create(&file_path)?;

        for entry in &group.entries {
            if entry.data().is_empty() {
                continue;
            }
            let frame = AnimFrame::decode(entry.data());
            writeln!(file, "[frame_{}]", entry.id)?;
            writeln!(file, "type={}", frame.type_marker)?;
            writeln!(file, "base={}", frame.base_id)?;
            writeln!(file, "bone_count={}", frame.bone_count)?;
            for bone in &frame.bones {
                writeln!(
                    file,
                    "bone={}:{}:{}:{}:{}",
                    bone.bone_index, bone.mask, bone.dx, bone.dy, bone.dz
                )?;
            }
            writeln!(file)?;
            total += 1;
        }
    }

    info!(
        "  {} animation frames unpacked to {}",
        total,
        anim_dir.display()
    );
    Ok(total)
}
