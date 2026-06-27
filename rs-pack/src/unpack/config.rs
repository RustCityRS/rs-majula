use std::collections::hash_map::Entry;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use super::report::RecordLeftover;
use crate::pack::util::colour::rgb15_to_hsl16;
use crate::types::LocShape;
use rs_io::Packet;
use rs_io::jag::JagFile;
use tracing::debug;

type ConfigEntries = Vec<(u16, Vec<(String, String)>)>;
type ConfigDecoder = fn(&[u8], &[u8], &HashMap<u16, u16>, &mut UnpackedPacks) -> ConfigEntries;

#[derive(Clone, Debug)]
pub enum ModelCategory {
    Npc,
    Obj,
    Loc,
    Spotanim,
    IdkMan,
    IdkWoman,
}

pub struct UnpackedPacks {
    pub model_names: HashMap<u16, String>,
    pub model_categories: HashMap<u16, ModelCategory>,
    pub seq_ids: BTreeSet<u16>,
    pub anim_ids: BTreeSet<u16>,
    pub obj_ids: BTreeSet<u16>,
    pub texture_ids: BTreeSet<u16>,
    pub category_ids: BTreeSet<u16>,
    pub cert_objs: HashMap<u16, u16>,
    pub cert_template_id: Option<u16>,
    pub flo_names: HashMap<u16, String>,
    pub leftovers: Vec<RecordLeftover>,
    pub dat_trailing: Vec<(String, usize)>,
}

#[allow(clippy::new_without_default)]
impl UnpackedPacks {
    pub fn new() -> Self {
        Self {
            model_names: HashMap::new(),
            model_categories: HashMap::new(),
            seq_ids: BTreeSet::new(),
            anim_ids: BTreeSet::new(),
            obj_ids: BTreeSet::new(),
            texture_ids: BTreeSet::new(),
            category_ids: BTreeSet::new(),
            cert_objs: HashMap::new(),
            cert_template_id: None,
            flo_names: HashMap::new(),
            leftovers: Vec::new(),
            dat_trailing: Vec::new(),
        }
    }

    fn name_model(&mut self, id: u16, name: String, category: ModelCategory) -> String {
        if let Entry::Vacant(e) = self.model_names.entry(id) {
            e.insert(name);
            self.model_categories.insert(id, category);
        }
        self.model_names.get(&id).unwrap().clone()
    }

    pub fn write_pack_files(&self, pack_dir: &Path) -> anyhow::Result<()> {
        write_model_pack(pack_dir, &self.model_names)?;
        write_pack_file(pack_dir, "anim", &self.anim_ids, "anim")?;
        write_pack_file(pack_dir, "texture", &self.texture_ids, "texture")?;
        write_pack_file(pack_dir, "category", &self.category_ids, "category")?;
        Ok(())
    }
}

fn write_model_pack(pack_dir: &Path, names: &HashMap<u16, String>) -> anyhow::Result<()> {
    if names.is_empty() {
        return Ok(());
    }
    let max_id = names.keys().copied().max().unwrap_or(0);
    let mut lines = Vec::new();
    for id in 0..=max_id {
        let name = names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("model_{id}"));
        lines.push(format!("{id}={name}"));
    }
    std::fs::write(pack_dir.join("model.pack"), lines.join("\n") + "\n")?;
    Ok(())
}

fn write_pack_file(
    pack_dir: &Path,
    filename: &str,
    ids: &BTreeSet<u16>,
    prefix: &str,
) -> anyhow::Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let max_id = ids.iter().copied().max().unwrap_or(0);
    let mut lines = Vec::new();
    for id in 0..=max_id {
        lines.push(format!("{id}={prefix}_{id}"));
    }
    std::fs::write(
        pack_dir.join(format!("{filename}.pack")),
        lines.join("\n") + "\n",
    )?;
    Ok(())
}

fn entry_name(config_type: &str, id: u16, packs: &UnpackedPacks) -> String {
    if config_type == "flo"
        && let Some(name) = packs.flo_names.get(&id)
    {
        return name.clone();
    }
    format!("{config_type}_{id}")
}

pub fn write_config_pack_file(
    pack_dir: &Path,
    config_type: &str,
    count: u16,
    packs: &UnpackedPacks,
) -> anyhow::Result<()> {
    let mut lines = Vec::new();
    for id in 0..count {
        if config_type == "obj" {
            if let Some(&linked_id) = packs.cert_objs.get(&id) {
                lines.push(format!("{id}=cert_{}", entry_name("obj", linked_id, packs)));
                continue;
            }
            if packs.cert_template_id == Some(id) {
                lines.push(format!("{id}=template_for_cert"));
                continue;
            }
        }
        lines.push(format!("{id}={}", entry_name(config_type, id, packs)));
    }
    std::fs::write(
        pack_dir.join(format!("{config_type}.pack")),
        lines.join("\n") + "\n",
    )?;
    Ok(())
}

pub fn unpack_config(
    jag: &JagFile,
    output_dir: &Path,
    pack_dir: &Path,
) -> anyhow::Result<UnpackedPacks> {
    std::fs::create_dir_all(pack_dir)?;
    let reverse_hsl = build_reverse_hsl_table();
    let mut packs = UnpackedPacks::new();

    let types: &[(&str, ConfigDecoder)] = &[
        ("idk", decode_idk_entries),
        ("obj", decode_obj_entries),
        ("npc", decode_npc_entries),
        ("spotanim", decode_spotanim_entries),
        ("flo", decode_flo_entries),
        ("seq", decode_seq_entries),
        ("loc", decode_loc_entries),
        ("varp", decode_varp_entries),
        #[cfg(since_254)]
        ("varbit", decode_varbit_entries),
    ];

    for (name, decoder) in types {
        let dat_name = format!("{name}.dat");
        let idx_name = format!("{name}.idx");

        let Some(dat) = jag.read(&dat_name) else {
            continue;
        };
        let Some(idx) = jag.read(&idx_name) else {
            continue;
        };

        let entries = decoder(&dat.data, &idx.data, &reverse_hsl, &mut packs);

        let trailing = config_dat_trailing(&dat.data, &idx.data);
        if trailing > 0 {
            packs.dat_trailing.push((name.to_string(), trailing));
        }

        let count = {
            let mut idx_buf = Packet::from(idx.data.clone());
            idx_buf.g2()
        };
        write_config_pack_file(pack_dir, name, count, &packs)?;

        write_config_text_file(output_dir, name, count, &entries, &packs)?;
        debug!("  Unpacked {} {name} entries", entries.len());
    }

    packs.write_pack_files(pack_dir)?;
    Ok(packs)
}

fn write_config_text_file(
    output_dir: &Path,
    config_type: &str,
    count: u16,
    entries: &[(u16, Vec<(String, String)>)],
    packs: &UnpackedPacks,
) -> anyhow::Result<()> {
    let entry_map: HashMap<u16, &Vec<(String, String)>> =
        entries.iter().map(|(id, props)| (*id, props)).collect();

    let mut lines = Vec::new();
    for id in 0..count {
        if config_type == "obj"
            && (packs.cert_objs.contains_key(&id) || packs.cert_template_id == Some(id))
        {
            continue;
        }
        lines.push(format!("[{}]", entry_name(config_type, id, packs)));
        if let Some(props) = entry_map.get(&id) {
            for (key, value) in *props {
                lines.push(format!("{key}={value}"));
            }
        }
        lines.push(String::new());
    }

    std::fs::write(
        output_dir.join(format!("all.{config_type}")),
        lines.join("\n") + "\n",
    )?;
    Ok(())
}

fn build_reverse_hsl_table() -> HashMap<u16, u16> {
    let mut table = HashMap::new();
    for rgb15 in 0..32768u16 {
        let hsl16 = rgb15_to_hsl16(rgb15);
        table.entry(hsl16).or_insert(rgb15);
    }
    table
}

fn read_entries(dat: &[u8], idx: &[u8]) -> Vec<(u16, Vec<u8>)> {
    let mut dat_buf = Packet::from(dat.to_vec());
    let mut idx_buf = Packet::from(idx.to_vec());

    let count = dat_buf.g2() as usize;
    let _idx_count = idx_buf.g2();

    let mut lengths = Vec::with_capacity(count);
    for _ in 0..count {
        lengths.push(idx_buf.g2() as usize);
    }

    let mut entries = Vec::with_capacity(count);
    for (id, &len) in lengths.iter().enumerate() {
        if len == 0 || dat_buf.pos + len > dat_buf.data.len() {
            entries.push((id as u16, Vec::new()));
            continue;
        }
        let data = dat_buf.data[dat_buf.pos..dat_buf.pos + len].to_vec();
        dat_buf.pos += len;
        entries.push((id as u16, data));
    }

    entries
}

fn config_dat_trailing(dat: &[u8], idx: &[u8]) -> usize {
    if dat.len() < 2 || idx.len() < 2 {
        return 0;
    }
    let count = u16::from_be_bytes([dat[0], dat[1]]) as usize;
    let mut consumed = 2;
    for k in 0..count {
        let off = 2 + k * 2;
        if off + 2 > idx.len() {
            break;
        }
        consumed += u16::from_be_bytes([idx[off], idx[off + 1]]) as usize;
    }
    dat.len().saturating_sub(consumed)
}

fn reverse_recol_pair(bs: u16, bd: u16, reverse_hsl: &HashMap<u16, u16>) -> (u16, u16) {
    if bs < 100 && bd < 100 {
        return (bs, bd);
    }
    let rs = reverse_hsl.get(&bs).copied().unwrap_or(bs);
    let rd = reverse_hsl.get(&bd).copied().unwrap_or(bd);
    (rs, rd)
}

fn model_ref(id: u16, suffix: &str, category: ModelCategory, packs: &mut UnpackedPacks) -> String {
    let name = if suffix.is_empty() {
        format!("model_{id}")
    } else {
        format!("model_{id}_{suffix}")
    };
    packs.name_model(id, name, category)
}

fn seq_ref(id: u16, packs: &mut UnpackedPacks) -> String {
    packs.seq_ids.insert(id);
    format!("seq_{id}")
}

fn obj_ref(id: u16, packs: &mut UnpackedPacks) -> String {
    packs.obj_ids.insert(id);
    format!("obj_{id}")
}

fn anim_ref(id: u16, packs: &mut UnpackedPacks) -> String {
    packs.anim_ids.insert(id);
    format!("anim_{id}")
}

fn texture_ref(id: u16, packs: &mut UnpackedPacks) -> String {
    packs.texture_ids.insert(id);
    super::texture::TEXTURE_NAMES
        .get(id as usize)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("texture_{id}"))
}

fn decode_flo_entries(
    dat: &[u8],
    idx: &[u8],
    _reverse_hsl: &HashMap<u16, u16>,
    packs: &mut UnpackedPacks,
) -> Vec<(u16, Vec<(String, String)>)> {
    let raw = read_entries(dat, idx);
    let mut results = Vec::new();

    for (id, data) in raw {
        if data.is_empty() {
            continue;
        }
        let mut buf = Packet::from(data);
        let mut props = Vec::new();

        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => {
                    let colour = buf.g3();
                    props.push(("colour".into(), format!("0x{colour:06X}")));
                }
                2 => {
                    let tex_id = buf.g1() as u16;
                    props.push(("texture".into(), texture_ref(tex_id, packs)));
                }
                3 => props.push(("overlay".into(), "yes".into())),
                5 => props.push(("occlude".into(), "no".into())),
                6 => {
                    let name = buf.gjstr(10);
                    packs.flo_names.insert(id, name);
                }
                _ => panic!("Unrecognized flo config code: {code}"),
            }
        }

        if buf.remaining() > 0 {
            packs.leftovers.push(RecordLeftover {
                config_type: "flo",
                id,
                bytes: buf.remaining() as usize,
            });
        }

        if !props.is_empty() {
            results.push((id, props));
        }
    }
    results
}

fn decode_varp_entries(
    dat: &[u8],
    idx: &[u8],
    _reverse_hsl: &HashMap<u16, u16>,
    packs: &mut UnpackedPacks,
) -> Vec<(u16, Vec<(String, String)>)> {
    let raw = read_entries(dat, idx);
    let mut results = Vec::new();

    for (id, data) in raw {
        if data.is_empty() {
            continue;
        }
        let mut buf = Packet::from(data);
        let mut props = Vec::new();

        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                5 => props.push(("clientcode".into(), buf.g2().to_string())),
                _ => panic!("Unrecognized varp config code: {code}"),
            }
        }
        if buf.remaining() > 0 {
            packs.leftovers.push(RecordLeftover {
                config_type: "varp",
                id,
                bytes: buf.remaining() as usize,
            });
        }
        results.push((id, props));
    }
    results
}

fn decode_idk_entries(
    dat: &[u8],
    idx: &[u8],
    reverse_hsl: &HashMap<u16, u16>,
    packs: &mut UnpackedPacks,
) -> Vec<(u16, Vec<(String, String)>)> {
    let raw = read_entries(dat, idx);
    let mut results = Vec::new();

    let type_names = [
        "man_hair",
        "man_jaw",
        "man_torso",
        "man_arms",
        "man_hands",
        "man_legs",
        "man_feet",
        "woman_hair",
        "woman_jaw",
        "woman_torso",
        "woman_arms",
        "woman_hands",
        "woman_legs",
        "woman_feet",
    ];

    for (id, data) in raw {
        if data.is_empty() {
            continue;
        }

        let is_woman = {
            let mut scan = Packet::from(data.clone());
            let mut found_type = 0u8;
            while scan.remaining() > 0 {
                let code: u8 = scan.g1();
                match code {
                    0 => break,
                    1 => {
                        found_type = scan.g1();
                        break;
                    }
                    2 => {
                        let c = scan.g1() as usize;
                        for _ in 0..c {
                            scan.g2();
                        }
                    }
                    3 => {}
                    40..=69 => {
                        scan.g2();
                    }
                    _ => panic!("Unrecognized idk config code: {code}"),
                }
            }
            found_type >= 7
        };
        let cat = if is_woman {
            ModelCategory::IdkWoman
        } else {
            ModelCategory::IdkMan
        };

        let mut buf = Packet::from(data);
        let mut props = Vec::new();

        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => {
                    let id = buf.g1() as usize;
                    props.push((
                        "type".into(),
                        type_names.get(id).unwrap_or(&"???").to_string(),
                    ));
                }
                2 => {
                    let count = buf.g1() as usize;
                    for i in 0..count {
                        props.push((
                            format!("model{}", i + 1),
                            model_ref(buf.g2(), "idk", cat.clone(), packs),
                        ));
                    }
                }
                3 => props.push(("disable".into(), "yes".into())),
                40..=49 => {
                    let i = code - 39;
                    let hsl = buf.g2();
                    let rgb = reverse_hsl.get(&hsl).copied().unwrap_or(hsl);
                    props.push((format!("recol{i}s"), rgb.to_string()));
                }
                50..=59 => {
                    let i = code - 49;
                    let hsl = buf.g2();
                    let rgb = reverse_hsl.get(&hsl).copied().unwrap_or(hsl);
                    props.push((format!("recol{i}d"), rgb.to_string()));
                }
                60..=69 => {
                    let i = code - 59;
                    props.push((
                        format!("head{i}"),
                        model_ref(buf.g2(), "idk_head", cat.clone(), packs),
                    ));
                }
                _ => panic!("Unrecognized idk config code: {code}"),
            }
        }
        if buf.remaining() > 0 {
            packs.leftovers.push(RecordLeftover {
                config_type: "idk",
                id,
                bytes: buf.remaining() as usize,
            });
        }
        if !props.is_empty() {
            results.push((id, props));
        }
    }
    results
}

fn decode_spotanim_entries(
    dat: &[u8],
    idx: &[u8],
    reverse_hsl: &HashMap<u16, u16>,
    packs: &mut UnpackedPacks,
) -> Vec<(u16, Vec<(String, String)>)> {
    let raw = read_entries(dat, idx);
    let mut results = Vec::new();

    for (id, data) in raw {
        if data.is_empty() {
            continue;
        }
        let mut buf = Packet::from(data);
        let mut props = Vec::new();

        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => props.push((
                    "model".into(),
                    model_ref(buf.g2(), "spotanim", ModelCategory::Spotanim, packs),
                )),
                2 => props.push(("anim".into(), seq_ref(buf.g2(), packs))),
                3 => props.push(("hasalpha".into(), "yes".into())),
                4 => props.push(("resizeh".into(), buf.g2().to_string())),
                5 => props.push(("resizev".into(), buf.g2().to_string())),
                6 => props.push(("angle".into(), buf.g2().to_string())),
                7 => props.push(("ambient".into(), buf.g1().to_string())),
                8 => props.push(("contrast".into(), buf.g1().to_string())),
                40..=49 => {
                    let i = code - 39;
                    let hsl = buf.g2();
                    let rgb = reverse_hsl.get(&hsl).copied().unwrap_or(hsl);
                    props.push((format!("recol{i}s"), rgb.to_string()));
                }
                50..=59 => {
                    let i = code - 49;
                    let hsl = buf.g2();
                    let rgb = reverse_hsl.get(&hsl).copied().unwrap_or(hsl);
                    props.push((format!("recol{i}d"), rgb.to_string()));
                }
                _ => panic!("Unrecognized spotanim config code: {code}"),
            }
        }
        if buf.remaining() > 0 {
            packs.leftovers.push(RecordLeftover {
                config_type: "spotanim",
                id,
                bytes: buf.remaining() as usize,
            });
        }
        if !props.is_empty() {
            results.push((id, props));
        }
    }
    results
}

fn decode_seq_entries(
    dat: &[u8],
    idx: &[u8],
    _reverse_hsl: &HashMap<u16, u16>,
    packs: &mut UnpackedPacks,
) -> Vec<(u16, Vec<(String, String)>)> {
    let raw = read_entries(dat, idx);
    let mut results = Vec::new();

    for (id, data) in raw {
        if data.is_empty() {
            continue;
        }
        let mut buf = Packet::from(data);
        let mut props = Vec::new();

        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => {
                    let count = buf.g1() as usize;
                    for i in 0..count {
                        let frame = buf.g2();
                        let iframe = buf.g2();
                        let delay = buf.g2();
                        props.push((format!("frame{i}"), anim_ref(frame, packs)));
                        if iframe != 0xFFFF {
                            props.push((format!("iframe{i}"), anim_ref(iframe, packs)));
                        }
                        if delay != 0 {
                            props.push((format!("delay{i}"), delay.to_string()));
                        }
                    }
                }
                2 => props.push(("loops".into(), buf.g2().to_string())),
                3 => {
                    let count = buf.g1() as usize;
                    let labels: Vec<String> =
                        (0..count).map(|_| format!("label_{}", buf.g1())).collect();
                    props.push(("walkmerge".into(), labels.join(",")));
                }
                4 => props.push(("stretches".into(), "yes".into())),
                5 => props.push(("priority".into(), buf.g1().to_string())),
                6 => {
                    let v = buf.g2();
                    if v == 0 {
                        props.push(("replaceheldleft".into(), "hide".into()));
                    } else {
                        props.push(("replaceheldleft".into(), obj_ref(v - 512, packs)));
                    }
                }
                7 => {
                    let v = buf.g2();
                    if v == 0 {
                        props.push(("replaceheldright".into(), "hide".into()));
                    } else {
                        props.push(("replaceheldright".into(), obj_ref(v - 512, packs)));
                    }
                }
                8 => props.push(("maxloops".into(), buf.g1().to_string())),
                #[cfg(since_244)]
                9 => {
                    let v = buf.g1();
                    let s = match v {
                        0 => "delaymove".to_string(),
                        1 => "delayanim".to_string(),
                        2 => "merge".to_string(),
                        _ => panic!("Unrecognized seq config preanim_move value: {v}"),
                    };
                    props.push(("preanim_move".into(), s));
                }
                #[cfg(since_244)]
                10 => {
                    let v = buf.g1();
                    let s = match v {
                        0 => "delaymove".to_string(),
                        1 => "abortanim".to_string(),
                        2 => "merge".to_string(),
                        _ => panic!("Unrecognized seq config postanim_move value: {v}"),
                    };
                    props.push(("postanim_move".into(), s));
                }
                #[cfg(since_244)]
                11 => {
                    let v = buf.g1();
                    let s = match v {
                        1 => "reset".to_string(),
                        2 => "reset_loop".to_string(),
                        _ => panic!("Unrecognized seq config duplicatebehavior value: {v}"),
                    };
                    // TODO: this has to be in british spelling
                    props.push(("duplicatebehavior".into(), s));
                }
                _ => panic!("Unrecognized seq config code: {code}"),
            }
        }
        if buf.remaining() > 0 {
            packs.leftovers.push(RecordLeftover {
                config_type: "seq",
                id,
                bytes: buf.remaining() as usize,
            });
        }
        if !props.is_empty() {
            results.push((id, props));
        }
    }
    results
}

fn decode_loc_entries(
    dat: &[u8],
    idx: &[u8],
    _reverse_hsl: &HashMap<u16, u16>,
    packs: &mut UnpackedPacks,
) -> Vec<(u16, Vec<(String, String)>)> {
    let raw = read_entries(dat, idx);
    let mut results = Vec::new();

    for (id, data) in raw {
        if data.is_empty() {
            continue;
        }
        let mut buf = Packet::from(data);
        let mut props = Vec::new();

        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => {
                    let count = buf.g1() as usize;
                    let mut pairs: Vec<(u16, u8)> = Vec::new();
                    for _ in 0..count {
                        pairs.push((buf.g2(), buf.g1()));
                    }
                    let base = format!("model_loc_{id}");
                    for &(mid, shape) in &pairs {
                        let suffix = LocShape::try_from(shape)
                            .expect("unknown loc shape")
                            .suffix();
                        packs.name_model(mid, format!("{base}{suffix}"), ModelCategory::Loc);
                    }
                    props.push(("model".into(), base));
                }
                2 => props.push(("name".into(), buf.gjstr(10))),
                3 => props.push(("desc".into(), buf.gjstr(10))),
                5 => {
                    let count = buf.g1() as usize;
                    let mut pairs: Vec<(u16, u8)> = Vec::new();
                    for _ in 0..count {
                        pairs.push((buf.g2(), LocShape::CentrepieceStraight as u8));
                    }
                    let base = format!("model_loc_{id}");
                    for &(mid, shape) in &pairs {
                        let suffix = LocShape::try_from(shape)
                            .expect("unknown loc shape")
                            .suffix();
                        packs.name_model(mid, format!("{base}{suffix}"), ModelCategory::Loc);
                    }
                    props.push(("model".into(), base));
                }
                14 => props.push(("width".into(), buf.g1().to_string())),
                15 => props.push(("length".into(), buf.g1().to_string())),
                17 => props.push(("blockwalk".into(), "no".into())),
                18 => props.push(("blockrange".into(), "no".into())),
                19 => {
                    let v = buf.g1();
                    props.push(("active".into(), if v == 1 { "yes" } else { "no" }.into()));
                }
                21 => props.push(("hillskew".into(), "yes".into())),
                22 => props.push(("sharelight".into(), "yes".into())),
                23 => props.push(("occlude".into(), "yes".into())),
                24 => props.push(("anim".into(), seq_ref(buf.g2(), packs))),
                25 => props.push(("hasalpha".into(), "yes".into())),
                28 => props.push(("wallwidth".into(), buf.g1().to_string())),
                29 => props.push(("ambient".into(), buf.g1s().to_string())),
                30 => props.push(("op1".into(), buf.gjstr(10))),
                31 => props.push(("op2".into(), buf.gjstr(10))),
                32 => props.push(("op3".into(), buf.gjstr(10))),
                33 => props.push(("op4".into(), buf.gjstr(10))),
                34 => props.push(("op5".into(), buf.gjstr(10))),
                39 => props.push(("contrast".into(), buf.g1s().to_string())),
                40 => {
                    let count = buf.g1() as usize;
                    for i in 0..count {
                        let s = buf.g2();
                        let d = buf.g2();
                        props.push((format!("recol{}s", i + 1), s.to_string()));
                        props.push((format!("recol{}d", i + 1), d.to_string()));
                    }
                }
                60 => props.push(("mapfunction".into(), buf.g2().to_string())),
                62 => props.push(("mirror".into(), "yes".into())),
                64 => props.push(("shadow".into(), "no".into())),
                65 => props.push(("resizex".into(), buf.g2().to_string())),
                66 => props.push(("resizey".into(), buf.g2().to_string())),
                67 => props.push(("resizez".into(), buf.g2().to_string())),
                68 => props.push(("mapscene".into(), buf.g2().to_string())),
                69 => {
                    let flags = buf.g1();
                    let dir = match flags {
                        0b1110 => "north",
                        0b1101 => "east",
                        0b1011 => "south",
                        0b0111 => "west",
                        _ => panic!("Unrecognized loc config forceapproach flags: {flags}"),
                    };
                    props.push(("forceapproach".into(), dir.into()));
                }
                70 => props.push(("offsetx".into(), buf.g2().to_string())),
                71 => props.push(("offsety".into(), buf.g2().to_string())),
                72 => props.push(("offsetz".into(), buf.g2().to_string())),
                73 => props.push(("forcedecor".into(), "yes".into())),
                #[cfg(since_245_2)]
                74 => props.push(("breakroutefinding".into(), "yes".into())),
                #[cfg(since_254)]
                75 => {
                    let v = if buf.g1() == 1 { "yes" } else { "no" };
                    props.push(("raiseobject".into(), v.into()));
                }
                _ => panic!("Unrecognized loc config code: {code}"),
            }
        }
        if buf.remaining() > 0 {
            packs.leftovers.push(RecordLeftover {
                config_type: "loc",
                id,
                bytes: buf.remaining() as usize,
            });
        }
        if !props.is_empty() {
            results.push((id, props));
        }
    }
    results
}

fn decode_npc_entries(
    dat: &[u8],
    idx: &[u8],
    reverse_hsl: &HashMap<u16, u16>,
    packs: &mut UnpackedPacks,
) -> Vec<(u16, Vec<(String, String)>)> {
    let raw = read_entries(dat, idx);
    let mut results = Vec::new();

    for (id, data) in raw {
        if data.is_empty() {
            continue;
        }
        let mut buf = Packet::from(data);
        let mut props = Vec::new();

        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => {
                    let count = buf.g1() as usize;
                    for i in 0..count {
                        props.push((
                            format!("model{}", i + 1),
                            model_ref(buf.g2(), "npc", ModelCategory::Npc, packs),
                        ));
                    }
                }
                2 => props.push(("name".into(), buf.gjstr(10))),
                3 => props.push(("desc".into(), buf.gjstr(10))),
                12 => props.push(("size".into(), buf.g1().to_string())),
                13 => props.push(("readyanim".into(), seq_ref(buf.g2(), packs))),
                14 => props.push(("walkanim".into(), seq_ref(buf.g2(), packs))),
                16 => props.push(("hasalpha".into(), "yes".into())),
                17 => {
                    let a = seq_ref(buf.g2(), packs);
                    let b = seq_ref(buf.g2(), packs);
                    let c = seq_ref(buf.g2(), packs);
                    let d = seq_ref(buf.g2(), packs);
                    props.push(("walkanim".into(), format!("{a},{b},{c},{d}")));
                }
                30 => props.push(("op1".into(), buf.gjstr(10))),
                31 => props.push(("op2".into(), buf.gjstr(10))),
                32 => props.push(("op3".into(), buf.gjstr(10))),
                33 => props.push(("op4".into(), buf.gjstr(10))),
                34 => props.push(("op5".into(), buf.gjstr(10))),
                40 => {
                    let count = buf.g1() as usize;
                    for i in 0..count {
                        let s = buf.g2();
                        let d = buf.g2();
                        let (rs, rd) = reverse_recol_pair(s, d, reverse_hsl);
                        props.push((format!("recol{}s", i + 1), rs.to_string()));
                        props.push((format!("recol{}d", i + 1), rd.to_string()));
                    }
                }
                60 => {
                    let count = buf.g1() as usize;
                    for i in 0..count {
                        props.push((
                            format!("head{}", i + 1),
                            model_ref(buf.g2(), "npc_head", ModelCategory::Npc, packs),
                        ));
                    }
                }
                90 => props.push(("resizex".into(), buf.g2().to_string())),
                91 => props.push(("resizey".into(), buf.g2().to_string())),
                92 => props.push(("resizez".into(), buf.g2().to_string())),
                93 => props.push(("minimap".into(), "no".into())),
                95 => {
                    let v = buf.g2();
                    props.push((
                        "vislevel".into(),
                        if v == 0 { "hide".into() } else { v.to_string() },
                    ));
                }
                97 => props.push(("resizeh".into(), buf.g2().to_string())),
                98 => props.push(("resizev".into(), buf.g2().to_string())),
                #[cfg(since_244)]
                99 => props.push(("alwaysontop".into(), "yes".into())),
                #[cfg(since_244)]
                100 => props.push(("ambient".into(), buf.g1s().to_string())),
                #[cfg(since_244)]
                101 => props.push(("contrast".into(), buf.g1s().to_string())),
                #[cfg(since_244)]
                102 => props.push(("headicon".into(), buf.g2().to_string())),
                _ => panic!("Unrecognized npc config code: {code}"),
            }
        }
        if buf.remaining() > 0 {
            packs.leftovers.push(RecordLeftover {
                config_type: "npc",
                id,
                bytes: buf.remaining() as usize,
            });
        }
        if !props.is_empty() {
            results.push((id, props));
        }
    }
    results
}

fn decode_obj_entries(
    dat: &[u8],
    idx: &[u8],
    reverse_hsl: &HashMap<u16, u16>,
    packs: &mut UnpackedPacks,
) -> Vec<(u16, Vec<(String, String)>)> {
    let raw = read_entries(dat, idx);
    let mut results = Vec::new();

    for (id, data) in raw {
        if data.is_empty() {
            continue;
        }
        let mut buf = Packet::from(data);
        let mut props = Vec::new();

        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => props.push((
                    "model".into(),
                    model_ref(buf.g2(), "obj", ModelCategory::Obj, packs),
                )),
                2 => props.push(("name".into(), buf.gjstr(10))),
                3 => props.push(("desc".into(), buf.gjstr(10))),
                4 => props.push(("2dzoom".into(), buf.g2().to_string())),
                5 => props.push(("2dxan".into(), buf.g2().to_string())),
                6 => props.push(("2dyan".into(), buf.g2().to_string())),
                7 => props.push(("2dxof".into(), (buf.g2() as i16).to_string())),
                8 => props.push(("2dyof".into(), (buf.g2() as i16).to_string())),
                9 => props.push(("code9".into(), "yes".into())),
                10 => props.push(("code10".into(), seq_ref(buf.g2(), packs))),
                11 => props.push(("stackable".into(), "yes".into())),
                12 => props.push(("cost".into(), buf.g4s().to_string())),
                16 => props.push(("members".into(), "yes".into())),
                23 => {
                    let m = model_ref(buf.g2(), "obj_wear", ModelCategory::Obj, packs);
                    let idx = buf.g1();
                    props.push(("manwear".into(), format!("{m},{idx}")));
                }
                24 => props.push((
                    "manwear2".into(),
                    model_ref(buf.g2(), "obj_wear", ModelCategory::Obj, packs),
                )),
                25 => {
                    let m = model_ref(buf.g2(), "obj_wear", ModelCategory::Obj, packs);
                    let idx = buf.g1();
                    props.push(("womanwear".into(), format!("{m},{idx}")));
                }
                26 => props.push((
                    "womanwear2".into(),
                    model_ref(buf.g2(), "obj_wear", ModelCategory::Obj, packs),
                )),
                30 => props.push(("op1".into(), buf.gjstr(10))),
                31 => props.push(("op2".into(), buf.gjstr(10))),
                32 => props.push(("op3".into(), buf.gjstr(10))),
                33 => props.push(("op4".into(), buf.gjstr(10))),
                34 => props.push(("op5".into(), buf.gjstr(10))),
                35 => props.push(("iop1".into(), buf.gjstr(10))),
                36 => props.push(("iop2".into(), buf.gjstr(10))),
                37 => props.push(("iop3".into(), buf.gjstr(10))),
                38 => props.push(("iop4".into(), buf.gjstr(10))),
                39 => props.push(("iop5".into(), buf.gjstr(10))),
                40 => {
                    let count = buf.g1() as usize;
                    for i in 0..count {
                        let s = buf.g2();
                        let d = buf.g2();
                        let (rs, rd) = reverse_recol_pair(s, d, reverse_hsl);
                        props.push((format!("recol{}s", i + 1), rs.to_string()));
                        props.push((format!("recol{}d", i + 1), rd.to_string()));
                    }
                }
                78 => props.push((
                    "manwear3".into(),
                    model_ref(buf.g2(), "obj_wear", ModelCategory::Obj, packs),
                )),
                79 => props.push((
                    "womanwear3".into(),
                    model_ref(buf.g2(), "obj_wear", ModelCategory::Obj, packs),
                )),
                90 => props.push((
                    "manhead".into(),
                    model_ref(buf.g2(), "obj_wear", ModelCategory::Obj, packs),
                )),
                91 => props.push((
                    "womanhead".into(),
                    model_ref(buf.g2(), "obj_wear", ModelCategory::Obj, packs),
                )),
                92 => props.push((
                    "manhead2".into(),
                    model_ref(buf.g2(), "obj_wear", ModelCategory::Obj, packs),
                )),
                93 => props.push((
                    "womanhead2".into(),
                    model_ref(buf.g2(), "obj_wear", ModelCategory::Obj, packs),
                )),
                95 => props.push(("2dzan".into(), buf.g2().to_string())),
                97 => props.push(("certlink".into(), obj_ref(buf.g2(), packs))),
                98 => props.push(("certtemplate".into(), obj_ref(buf.g2(), packs))),
                #[cfg(rev = "225")]
                99..=108 => {
                    let i = code - 99;
                    let oid = obj_ref(buf.g2(), packs);
                    let count = buf.g2();
                    props.push((format!("count{}", i + 1), format!("{oid},{count}")));
                }
                #[cfg(since_244)]
                100..=109 => {
                    let i = code - 100;
                    let oid = obj_ref(buf.g2(), packs);
                    let count = buf.g2();
                    props.push((format!("count{}", i + 1), format!("{oid},{count}")));
                }
                #[cfg(since_244)]
                110 => props.push(("resizex".into(), buf.g2().to_string())),
                #[cfg(since_244)]
                111 => props.push(("resizey".into(), buf.g2().to_string())),
                #[cfg(since_244)]
                112 => props.push(("resizez".into(), buf.g2().to_string())),
                #[cfg(since_244)]
                113 => props.push(("ambient".into(), buf.g1s().to_string())),
                #[cfg(since_244)]
                114 => props.push(("contrast".into(), buf.g1s().to_string())),
                _ => panic!("Unrecognized obj config code: {code}"),
            }
        }
        if buf.remaining() > 0 {
            packs.leftovers.push(RecordLeftover {
                config_type: "obj",
                id,
                bytes: buf.remaining() as usize,
            });
        }
        let is_cert = props.len() == 2
            && props
                .iter()
                .all(|(k, _)| k == "certlink" || k == "certtemplate");
        if is_cert {
            if let Some((_, link_name)) = props.iter().find(|(k, _)| k == "certlink") {
                if let Some(linked_id) = link_name
                    .strip_prefix("obj_")
                    .and_then(|s| s.parse::<u16>().ok())
                {
                    packs.cert_objs.insert(id, linked_id);
                }
            }
            if let Some((_, tmpl_name)) = props.iter().find(|(k, _)| k == "certtemplate") {
                if let Some(tmpl_id) = tmpl_name
                    .strip_prefix("obj_")
                    .and_then(|s| s.parse::<u16>().ok())
                {
                    packs.cert_template_id = Some(tmpl_id);
                }
            }
        }
        if !props.is_empty() {
            results.push((id, props));
        }
    }
    results
}

#[cfg(since_254)]
fn decode_varbit_entries(
    dat: &[u8],
    idx: &[u8],
    _reverse_hsl: &HashMap<u16, u16>,
    packs: &mut UnpackedPacks,
) -> Vec<(u16, Vec<(String, String)>)> {
    let raw = read_entries(dat, idx);
    let mut results = Vec::new();

    for (id, data) in raw {
        if data.is_empty() {
            continue;
        }
        let mut buf = Packet::from(data);
        let mut props = Vec::new();

        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => {
                    props.push(("basevar".into(), buf.g2().to_string()));
                    props.push(("startbit".into(), buf.g1().to_string()));
                    props.push(("endbit".into(), buf.g1().to_string()));
                }
                10 => props.push(("debugname".into(), buf.gjstr(10))),
                _ => panic!("Unrecognized varbit config code: {code}"),
            }
        }
        if buf.remaining() > 0 {
            packs.leftovers.push(RecordLeftover {
                config_type: "varbit",
                id,
                bytes: buf.remaining() as usize,
            });
        }
        results.push((id, props));
    }
    results
}
