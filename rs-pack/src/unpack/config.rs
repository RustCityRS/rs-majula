#[cfg(before_647)]
use super::model::load_existing_pack;
use super::report::RecordLeftover;
#[cfg(since_647)]
use super::report::{StepResult, step};
#[cfg(before_647)]
use super::texture::TEXTURE_NAMES;
#[cfg(before_647)]
use crate::PACK_DIR;
#[cfg(since_647)]
use crate::archive;
#[cfg(since_647)]
use crate::cache::r#enum::EnumTypeRaw;
use crate::cache::flo::FloTypeRaw;
#[cfg(since_647)]
use crate::cache::flu::FluType;
use crate::cache::idk::IdkTypeRaw;
#[cfg(since_647)]
use crate::cache::inv::InvTypeRaw;
use crate::cache::loc::LocTypeRaw;
use crate::cache::npc::NpcTypeRaw;
use crate::cache::obj::ObjTypeRaw;
#[cfg(since_647)]
use crate::cache::param::ParamTypeRaw;
use crate::cache::provider::CacheType;
use crate::cache::seq::SeqTypeRaw;
use crate::cache::spotanim::SpotAnimTypeRaw;
#[cfg(since_647)]
use crate::cache::r#struct::StructTypeRaw;
use crate::cache::varp::VarPlayerTypeRaw;
use crate::pack::pack_registry::{PackFile, PackId};
#[cfg(before_647)]
use crate::pack::packed_data::PackedData;
#[cfg(before_647)]
use crate::pack::util::colour::rgb15_to_hsl16;
#[cfg(before_647)]
use crate::types::LocShape;
use order::reorder_by_opcode;
use rs_io::Packet;
#[cfg(before_647)]
use rs_io::jag::JagFile;
#[cfg(since_647)]
use rs_io::js5::Js5Store;
#[cfg(before_647)]
use std::collections::hash_map::Entry;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
#[cfg(since_647)]
use std::fs::File;
#[cfg(since_647)]
use std::io::BufWriter;
use std::io::{self, Write};
use std::mem::replace;
use std::path::Path;
use std::rc::Rc;
#[cfg(before_647)]
use tracing::debug;

#[cfg(since_647)]
pub mod r#enum;
pub mod flo;
#[cfg(since_647)]
pub mod flu;
#[cfg(since_647)]
pub mod generic;
pub mod idk;
#[cfg(since_647)]
pub mod inv;
pub mod loc;
pub mod npc;
pub mod obj;
mod order;
#[cfg(since_647)]
pub mod param;
pub mod seq;
pub mod spotanim;
#[cfg(since_647)]
pub mod r#struct;
#[cfg(since_254)]
pub mod varbit;
pub mod varp;

#[cfg(before_647)]
type ConfigEntries = Vec<(u16, Vec<(String, String)>)>;
pub(crate) type ConfigDecoder =
    fn(&mut Packet, u16, &HashMap<u16, u16>, &mut UnpackedPacks) -> Vec<(String, String)>;

pub(crate) fn config_decoders() -> Vec<(&'static str, ConfigDecoder)> {
    vec![
        ("idk", |buf, id, hsl, packs| {
            decode_record::<IdkTypeRaw>(buf, id, hsl, packs, idk::emit_idk_props)
        }),
        ("obj", |buf, id, hsl, packs| {
            decode_record::<ObjTypeRaw>(buf, id, hsl, packs, obj::emit_obj_props)
        }),
        ("npc", |buf, id, hsl, packs| {
            decode_record::<NpcTypeRaw>(buf, id, hsl, packs, npc::emit_npc_props)
        }),
        ("spotanim", |buf, id, hsl, packs| {
            decode_record::<SpotAnimTypeRaw>(buf, id, hsl, packs, spotanim::emit_spotanim_props)
        }),
        ("flo", |buf, id, hsl, packs| {
            decode_record::<FloTypeRaw>(buf, id, hsl, packs, flo::emit_flo_props)
        }),
        ("seq", |buf, id, hsl, packs| {
            decode_record::<SeqTypeRaw>(buf, id, hsl, packs, seq::emit_seq_props)
        }),
        ("loc", |buf, id, hsl, packs| {
            decode_record::<LocTypeRaw>(buf, id, hsl, packs, loc::emit_loc_props)
        }),
        ("varp", |buf, id, hsl, packs| {
            decode_record::<VarPlayerTypeRaw>(buf, id, hsl, packs, varp::emit_varp_props)
        }),
        #[cfg(since_254)]
        ("varbit", varbit::decode_varbit),
        #[cfg(since_647)]
        ("param", |buf, id, hsl, packs| {
            decode_record::<ParamTypeRaw>(buf, id, hsl, packs, param::emit_param_props)
        }),
        #[cfg(since_647)]
        ("enum", |buf, id, hsl, packs| {
            decode_record::<EnumTypeRaw>(buf, id, hsl, packs, r#enum::emit_enum_props)
        }),
        #[cfg(since_647)]
        ("inv", |buf, id, hsl, packs| {
            decode_record::<InvTypeRaw>(buf, id, hsl, packs, inv::emit_inv_props)
        }),
        #[cfg(since_647)]
        ("struct", |buf, id, hsl, packs| {
            decode_record::<StructTypeRaw>(buf, id, hsl, packs, r#struct::emit_struct_props)
        }),
        #[cfg(since_647)]
        ("flu", |buf, id, hsl, packs| {
            decode_record::<FluType>(buf, id, hsl, packs, flu::emit_flu_props)
        }),
        #[cfg(all(since_274, before_647))]
        ("mesanim", |buf, _, _, _| {
            decode_empty_record("mesanim", buf)
        }),
        #[cfg(all(since_274, before_647))]
        ("mes", |buf, _, _, _| decode_empty_record("mes", buf)),
        #[cfg(all(since_274, before_647))]
        ("param", |buf, _, _, _| decode_empty_record("param", buf)),
        #[cfg(all(since_274, before_647))]
        ("hunt", |buf, _, _, _| decode_empty_record("hunt", buf)),
    ]
}

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
    pub existing_model_names: HashMap<u16, String>,
    pub existing_config_names: HashMap<String, PackFile>,
    pub anim_ids: BTreeSet<u16>,
    pub texture_ids: BTreeSet<u16>,
    pub category_ids: BTreeSet<u16>,
    pub cert_objs: HashMap<u16, u16>,
    pub cert_template_id: Option<u16>,
    pub flo_names: HashMap<u16, String>,
    #[cfg(since_647)]
    pub transmit_varps: HashSet<u32>,
    pub model_textures: Rc<HashMap<u16, HashSet<u16>>>,
    pub leftovers: Vec<RecordLeftover>,
    pub dat_trailing: Vec<(String, usize)>,
}

#[allow(clippy::new_without_default)]
impl UnpackedPacks {
    pub fn new() -> Self {
        Self {
            model_names: HashMap::new(),
            model_categories: HashMap::new(),
            existing_model_names: HashMap::new(),
            existing_config_names: HashMap::new(),
            anim_ids: BTreeSet::new(),
            texture_ids: BTreeSet::new(),
            category_ids: BTreeSet::new(),
            cert_objs: HashMap::new(),
            cert_template_id: None,
            flo_names: HashMap::new(),
            #[cfg(since_647)]
            transmit_varps: HashSet::new(),
            model_textures: Rc::new(HashMap::new()),
            leftovers: Vec::new(),
            dat_trailing: Vec::new(),
        }
    }

    pub fn load_existing_names(&mut self, pack_dir: &Path) {
        #[cfg(before_647)]
        {
            self.existing_model_names = load_existing_pack(pack_dir, "model");
        }
        self.existing_config_names = load_committed_config_names(pack_dir);
    }

    #[cfg(since_647)]
    pub fn config_id(&self, name: &str, value: &str) -> Option<u32> {
        self.existing_config_names
            .get(name)?
            .get_by_debugname(value)
    }

    pub fn config_name(&self, name: &str, id: u32) -> Option<&str> {
        #[cfg(before_647)]
        if name == "model"
            && let Some(value) = self.model_names.get(&(id as u16))
        {
            return Some(value);
        }
        self.existing_config_names
            .get(name)?
            .get_by_id(id as PackId)
    }

    #[cfg(before_647)]
    fn name_model(&mut self, id: u16, name: String, category: ModelCategory) -> String {
        if let Entry::Vacant(e) = self.model_names.entry(id) {
            let name = self.existing_model_names.get(&id).cloned().unwrap_or(name);
            e.insert(name);
            self.model_categories.insert(id, category);
        }
        self.model_names.get(&id).unwrap().clone()
    }

    #[cfg(before_647)]
    pub fn write_pack_files(&self, pack_dir: &Path) -> anyhow::Result<()> {
        write_model_pack(pack_dir, &self.model_names)?;
        write_pack_file(pack_dir, "anim", &self.anim_ids, "anim")?;
        write_pack_file(pack_dir, "texture", &self.texture_ids, "texture")?;
        write_pack_file(pack_dir, "category", &self.category_ids, "category")?;
        Ok(())
    }
}

#[cfg(before_647)]
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
    fs::write(pack_dir.join("model.pack"), lines.join("\n") + "\n")?;
    Ok(())
}

#[cfg(before_647)]
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
    fs::write(
        pack_dir.join(format!("{filename}.pack")),
        lines.join("\n") + "\n",
    )?;
    Ok(())
}

pub(crate) fn load_committed_config_names(pack_dir: &Path) -> HashMap<String, PackFile> {
    let mut names = HashMap::new();
    if let Ok(files) = fs::read_dir(pack_dir) {
        for entry in files.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "pack") {
                let name = path.file_stem().unwrap().to_string_lossy().into_owned();
                if let Ok(pack) = PackFile::load(&path) {
                    names.insert(name, pack);
                }
            }
        }
    }
    names
}

fn entry_name(config_type: &str, id: u16, packs: &UnpackedPacks) -> String {
    if config_type == "flo"
        && let Some(name) = packs.flo_names.get(&id)
    {
        return name.clone();
    }
    if let Some(name) = packs
        .existing_config_names
        .get(config_type)
        .and_then(|names| names.get_by_id(id as PackId))
    {
        return name.to_string();
    }
    format!("{config_type}_{id}")
}

#[cfg(before_647)]
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
    fs::write(
        pack_dir.join(format!("{config_type}.pack")),
        lines.join("\n") + "\n",
    )?;
    Ok(())
}

#[cfg(before_647)]
pub fn unpack_config(
    jag: &JagFile,
    output_dir: &Path,
    pack_dir: &Path,
    model_textures: Rc<HashMap<u16, HashSet<u16>>>,
) -> anyhow::Result<UnpackedPacks> {
    fs::create_dir_all(pack_dir)?;
    let reverse_hsl = build_reverse_hsl_table();
    let mut packs = UnpackedPacks::new();
    packs.load_existing_names(Path::new(PACK_DIR));
    packs.model_textures = model_textures;

    let types = config_decoders();

    for (name, decoder) in &types {
        let dat_name = format!("{name}.dat");
        let idx_name = format!("{name}.idx");

        let Some(dat) = jag.read(&dat_name) else {
            continue;
        };
        let Some(idx) = jag.read(&idx_name) else {
            continue;
        };

        let entries = decode_records(
            name,
            read_entries(&dat.data, &idx.data),
            &reverse_hsl,
            &mut packs,
            *decoder,
        );

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

#[cfg(before_647)]
fn write_config_text_file(
    output_dir: &Path,
    config_type: &str,
    count: u16,
    entries: &[(u16, Vec<(String, String)>)],
    packs: &UnpackedPacks,
) -> anyhow::Result<()> {
    let entry_map: HashMap<u16, &Vec<(String, String)>> =
        entries.iter().map(|(id, props)| (*id, props)).collect();

    let mut output = Vec::new();
    for id in 0..count {
        if config_type == "obj"
            && (packs.cert_objs.contains_key(&id) || packs.cert_template_id == Some(id))
        {
            continue;
        }
        write_config_section(
            &mut output,
            &entry_name(config_type, id, packs),
            entry_map.get(&id).map_or(&[], |props| props.as_slice()),
        )?;
    }

    fs::write(
        output_dir.join(format!("all.{config_type}")),
        if output.is_empty() {
            b"\n".as_slice()
        } else {
            &output
        },
    )?;
    Ok(())
}

#[cfg(before_647)]
pub(crate) fn build_reverse_hsl_table() -> HashMap<u16, u16> {
    let mut table = HashMap::new();
    for rgb15 in 0..32768u16 {
        let hsl16 = rgb15_to_hsl16(rgb15);
        table.entry(hsl16).or_insert(rgb15);
    }
    table
}

#[cfg(before_647)]
pub(crate) fn read_entries(dat: &[u8], idx: &[u8]) -> Vec<(u16, Vec<u8>)> {
    PackedData::entries(dat, idx)
        .expect("Invalid config data/index")
        .map(|(id, data)| (id, data.to_vec()))
        .collect()
}

#[cfg(before_647)]
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

#[cfg(before_647)]
fn model_ref(id: u16, suffix: &str, category: ModelCategory, packs: &mut UnpackedPacks) -> String {
    let name = if suffix.is_empty() {
        format!("model_{id}")
    } else {
        format!("model_{id}_{suffix}")
    };
    packs.name_model(id, name, category)
}

#[cfg(before_647)]
fn seq_ref(id: u16, packs: &UnpackedPacks) -> String {
    entry_name("seq", id, packs)
}

#[cfg(before_647)]
fn obj_ref(id: u16, packs: &UnpackedPacks) -> String {
    entry_name("obj", id, packs)
}

#[cfg(since_289)]
#[cfg(before_647)]
fn loc_ref(id: u16, packs: &UnpackedPacks) -> String {
    entry_name("loc", id, packs)
}

#[cfg(since_289)]
#[cfg(before_647)]
fn varbit_ref(id: u16, packs: &UnpackedPacks) -> String {
    entry_name("varbit", id, packs)
}

#[cfg(before_647)]
fn obj_ref_to_id(value: &str, packs: &UnpackedPacks) -> Option<u16> {
    if let Some(id) = value
        .strip_prefix("obj_")
        .and_then(|s| s.parse::<u16>().ok())
    {
        return Some(id);
    }
    packs.existing_config_names.get("obj").and_then(|names| {
        names
            .id_to_debugname
            .iter()
            .find(|(_, name)| name.as_str() == value)
            .map(|(id, _)| *id)
    })
}

#[cfg(before_647)]
fn anim_ref(id: u16, packs: &mut UnpackedPacks) -> String {
    packs.anim_ids.insert(id);
    format!("anim_{id}")
}

#[cfg(before_647)]
fn texture_ref(id: u16, packs: &mut UnpackedPacks) -> String {
    packs.texture_ids.insert(id);
    TEXTURE_NAMES
        .get(id as usize)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("texture_{id}"))
}

fn decode_record<T: CacheType>(
    buf: &mut Packet,
    id: u16,
    hsl: &HashMap<u16, u16>,
    packs: &mut UnpackedPacks,
    emit: impl Fn(&T, &mut UnpackedPacks, &HashMap<u16, u16>) -> Vec<(String, String)>,
) -> Vec<(String, String)> {
    let mut def = T::new(id);
    def.decode(buf);
    emit(&def, packs, hsl)
}

fn decode_entry(
    name: &'static str,
    id: u16,
    data: Vec<u8>,
    reverse_hsl: &HashMap<u16, u16>,
    packs: &mut UnpackedPacks,
    decoder: ConfigDecoder,
) -> Vec<(String, String)> {
    let mut buf = Packet::from(data);
    let props = decoder(&mut buf, id, reverse_hsl, packs);
    if buf.remaining() > 0 {
        packs.leftovers.push(RecordLeftover {
            config_type: name,
            id,
            bytes: buf.remaining() as usize,
        });
    }
    #[cfg(before_647)]
    if name == "obj"
        && props.len() == 2
        && props
            .iter()
            .all(|(key, _)| key == "certlink" || key == "certtemplate")
    {
        for (key, value) in &props {
            if let Some(value) = obj_ref_to_id(value, packs) {
                if key == "certlink" {
                    packs.cert_objs.insert(id, value);
                } else {
                    packs.cert_template_id = Some(value);
                }
            }
        }
    }
    props
}

#[cfg(before_647)]
pub(crate) fn decode_records(
    name: &'static str,
    entries: Vec<(u16, Vec<u8>)>,
    hsl: &HashMap<u16, u16>,
    packs: &mut UnpackedPacks,
    decoder: ConfigDecoder,
) -> ConfigEntries {
    entries
        .into_iter()
        .filter(|(_, data)| !data.is_empty())
        .map(|(id, data)| (id, decode_entry(name, id, data, hsl, packs, decoder)))
        .collect()
}

#[cfg(all(since_274, before_647))]
fn decode_empty_record(name: &str, buf: &mut Packet) -> Vec<(String, String)> {
    let code = buf.g1();
    assert_eq!(code, 0, "Unrecognized {name} config code: {code}");
    Vec::new()
}

fn write_config_section(
    out: &mut impl Write,
    section: &str,
    props: &[(String, String)],
) -> io::Result<()> {
    writeln!(out, "[{section}]")?;
    for (key, value) in props {
        writeln!(out, "{key}={value}")?;
    }
    writeln!(out)
}

#[cfg(since_647)]
pub(super) fn unpack_config(
    cache_dir: &Path,
    output_dir: &Path,
    pack_dir: &Path,
    results: &mut Vec<StepResult>,
) -> anyhow::Result<()> {
    let mut store = Js5Store::open(cache_dir)?;
    let mut packs = UnpackedPacks::new();
    packs.load_existing_names(pack_dir);
    packs.transmit_varps = varp::transmit_varps(&mut store)?;
    let hsl = HashMap::new();
    for (name, decoder) in config_decoders() {
        step(results, name, || {
            let (archive_id, stride) = match name {
                "loc" => (archive::CONFIG_LOC, 256),
                "enum" => (archive::CONFIG_ENUM, 256),
                "npc" => (archive::CONFIG_NPC, 128),
                "obj" => (archive::CONFIG_OBJ, 256),
                "seq" => (archive::CONFIG_SEQ, 128),
                "spotanim" => (archive::CONFIG_SPOTANIM, 256),
                "varbit" => (archive::CONFIG_VARBIT, 1024),
                _ => (archive::CONFIG, 0),
            };
            let groups = if stride == 0 {
                let group = archive::CONFIG_GROUPS
                    .iter()
                    .find(|(kind, _)| *kind == name)
                    .unwrap()
                    .1;
                group..group + 1
            } else {
                0..store.group_count(archive_id).unwrap_or(0)
            };
            let mut seen = HashSet::new();
            let mut file = BufWriter::new(File::create(output_dir.join(format!("all.{name}")))?);
            let mut count = 0;
            for group_id in groups {
                let Some(mut group) = store.read_group(archive_id, group_id)? else {
                    continue;
                };
                group.entries.sort_by_key(|entry| entry.id);
                for entry in group.entries {
                    if entry.data().is_empty() {
                        continue;
                    }
                    let id = (group_id * stride + entry.id) as u16;
                    let props =
                        decode_entry(name, id, entry.data().to_vec(), &hsl, &mut packs, decoder);
                    let mut section = entry_name(name, id, &packs);
                    if !seen.insert(section.clone()) {
                        section = format!("{section}__{id}");
                        seen.insert(section.clone());
                    }
                    write_config_section(&mut file, &section, &props)?;
                    count += 1;
                }
            }
            file.flush()?;
            Ok(format!("{count} configs"))
        });
    }
    Ok(())
}
#[cfg(before_647)]
fn rename_loc_model(name: &str, shape: u8) -> &str {
    let base = name.strip_suffix("_ld").unwrap_or(name);
    LocShape::try_from(shape)
        .ok()
        .and_then(|s| base.strip_suffix(s.suffix()))
        .unwrap_or(base)
}

#[cfg(before_647)]
fn push_loc_model_refs(
    loc_id: u16,
    pairs: &[(u16, u8)],
    packs: &mut UnpackedPacks,
    props: &mut Vec<(String, String)>,
) {
    let mut bases: Vec<String> = Vec::new();
    for &(mid, shape) in pairs {
        let suffix = LocShape::try_from(shape)
            .expect("unknown loc shape")
            .suffix();
        let name = packs.name_model(mid, format!("loc_{loc_id}{suffix}"), ModelCategory::Loc);
        let base = rename_loc_model(&name, shape).to_string();
        if !bases.contains(&base) {
            bases.push(base);
        }
    }
    for base in bases {
        props.push(("model".into(), base));
    }
}

#[cfg(before_647)]
fn models_have_texture(
    model_ids: &[u16],
    texture: u16,
    model_textures: &HashMap<u16, HashSet<u16>>,
) -> bool {
    model_ids
        .iter()
        .any(|m| model_textures.get(m).is_some_and(|t| t.contains(&texture)))
}

fn recol_value(value: u16, other: u16, _reverse_hsl: &HashMap<u16, u16>) -> u16 {
    #[cfg(before_647)]
    if value >= 100 || other >= 100 {
        return _reverse_hsl.get(&value).copied().unwrap_or(value);
    }
    #[cfg(since_647)]
    let _ = other;
    value
}

fn has_repeated_opcodes(order: &[u8]) -> bool {
    let mut seen = [false; 256];
    order
        .iter()
        .any(|&code| replace(&mut seen[code as usize], true))
}

fn emit_repeated<T: CacheType>(
    id: u16,
    records: &[(u8, Vec<u8>)],
    packs: &mut UnpackedPacks,
    reverse_hsl: &HashMap<u16, u16>,
    emit: impl Fn(&T, &mut UnpackedPacks, &HashMap<u16, u16>) -> Vec<(String, String)>,
    defer_recolours: bool,
    clear: impl Fn(&mut T),
) -> Vec<(String, String)> {
    let mut props = Vec::new();
    let mut value = T::new(id);
    for (code, raw) in records
        .iter()
        .filter(|(code, _)| !defer_recolours || *code != 40)
        .chain(
            records
                .iter()
                .filter(|(code, _)| defer_recolours && *code == 40),
        )
    {
        let mut data = Vec::with_capacity(raw.len() + 2);
        data.push(*code);
        data.extend_from_slice(raw);
        data.push(0);
        clear(&mut value);
        value.decode(&mut Packet::from(data));
        let emitted = emit(&value, packs, reverse_hsl);
        props.extend(emitted);
    }
    props
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_ver::STRING_TERMINATOR;

    fn unpack(
        name: &'static str,
        data: Vec<u8>,
        packs: &mut UnpackedPacks,
        hsl: &HashMap<u16, u16>,
    ) -> Vec<(String, String)> {
        let decoder = config_decoders()
            .into_iter()
            .find(|(kind, _)| *kind == name)
            .unwrap()
            .1;
        decode_entry(name, 7, data, hsl, packs, decoder)
    }

    #[test]
    fn repeated_values_and_trailing_bytes_survive_unpacking() {
        let mut packs = UnpackedPacks::new();
        let props = unpack(
            "npc",
            vec![
                2,
                b'a',
                STRING_TERMINATOR,
                2,
                b'b',
                STRING_TERMINATOR,
                0,
                0xaa,
            ],
            &mut packs,
            &HashMap::new(),
        );
        assert_eq!(
            props,
            vec![("name".into(), "a".into()), ("name".into(), "b".into())]
        );
        assert_eq!(packs.leftovers.len(), 1);
        assert_eq!((packs.leftovers[0].id, packs.leftovers[0].bytes), (7, 1));
    }

    #[cfg(before_647)]
    #[test]
    fn repeated_loc_fields_keep_texture_detection() {
        let mut packs = UnpackedPacks::new();
        packs.model_textures = Rc::new(HashMap::from([(5, HashSet::from([1]))]));
        let hsl = HashMap::from([(1, 1000), (2, 2000)]);
        let props = unpack(
            "loc",
            vec![
                40,
                1,
                0,
                1,
                0,
                2,
                1,
                1,
                0,
                5,
                10,
                2,
                b'a',
                STRING_TERMINATOR,
                2,
                b'b',
                STRING_TERMINATOR,
                0,
            ],
            &mut packs,
            &hsl,
        );
        assert_eq!(props.iter().filter(|(key, _)| key == "name").count(), 2);
        assert_eq!(props[props.len() - 2].0, "retex1s");
        assert_eq!(props[props.len() - 1].0, "retex1d");
    }

    #[test]
    fn count_opcode_100_is_the_first_obj_variant() {
        let props = unpack(
            "obj",
            vec![100, 0, 2, 0, 5, 0],
            &mut UnpackedPacks::new(),
            &HashMap::new(),
        );
        assert_eq!(props, vec![("count1".into(), "obj_2,5".into())]);
    }

    #[cfg(all(since_254, before_647))]
    #[test]
    fn varbit_zero_values_and_legacy_debugname_are_preserved() {
        let props = unpack(
            "varbit",
            vec![1, 0, 0, 0, 0, 10, b'x', STRING_TERMINATOR, 0],
            &mut UnpackedPacks::new(),
            &HashMap::new(),
        );
        assert_eq!(
            props,
            vec![
                ("basevar".into(), "varp_0".into()),
                ("startbit".into(), "0".into()),
                ("endbit".into(), "0".into()),
                ("debugname".into(), "x".into())
            ]
        );
    }

    #[cfg(since_647)]
    #[test]
    fn overlapping_loc_opcodes_keep_their_original_values() {
        let props = unpack(
            "loc",
            vec![27, 22, 17, 22, 0],
            &mut UnpackedPacks::new(),
            &HashMap::new(),
        );
        assert_eq!(
            props,
            vec![
                ("interact_type1".into(), "yes".into()),
                ("sharelight".into(), "yes".into()),
                ("blockwalk".into(), "no".into()),
                ("sharelight".into(), "yes".into()),
            ]
        );
    }

    #[cfg(since_647)]
    #[test]
    fn params_keep_repeated_keys_without_reemitting_stale_values() {
        let props = unpack(
            "npc",
            vec![
                249,
                2,
                0,
                0,
                0,
                1,
                0,
                0,
                0,
                7,
                0,
                0,
                0,
                1,
                0,
                0,
                0,
                8,
                249,
                0,
                249,
                1,
                1,
                0,
                0,
                2,
                b'a',
                STRING_TERMINATOR,
                0,
            ],
            &mut UnpackedPacks::new(),
            &HashMap::new(),
        );
        assert_eq!(
            props,
            vec![
                ("paramint".into(), "1,7".into()),
                ("paramint".into(), "1,8".into()),
                ("paramstr".into(), "2,a".into()),
            ]
        );
    }
}
