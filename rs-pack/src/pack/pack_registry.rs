#[cfg(since_647)]
use crate::pack::pack::FileCache;
use crate::pack::packed_data::PackedData;
use anyhow::Result;
#[cfg(since_647)]
use anyhow::bail;
use std::collections::HashMap;
#[cfg(since_647)]
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;
#[cfg(since_647)]
use tracing::info;

#[cfg(before_647)]
pub type PackId = u16;
#[cfg(since_647)]
pub type PackId = u32;

#[derive(Clone)]
pub struct PackFile {
    pub id_to_debugname: HashMap<PackId, String>,
    pub debugname_to_id: HashMap<String, PackId>,
    pub max: PackId,
}

impl PackFile {
    pub fn empty() -> Self {
        Self {
            id_to_debugname: HashMap::new(),
            debugname_to_id: HashMap::new(),
            max: 0,
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let mut id_to_debugname = HashMap::new();
        let mut debugname_to_id = HashMap::new();
        let mut max: PackId = 0;

        if let Ok(text) = fs::read_to_string(path) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let Some((id_str, debugname)) = line.split_once('=') else {
                    continue;
                };
                let Ok(id) = id_str.parse::<PackId>() else {
                    continue;
                };
                let name = debugname.trim().to_string();
                id_to_debugname.insert(id, name.clone());
                debugname_to_id.insert(name, id);
                if id >= max {
                    max = id + 1;
                }
            }
        }

        Ok(Self {
            id_to_debugname,
            debugname_to_id,
            max,
        })
    }

    pub fn get_by_debugname(&self, debugname: &str) -> Option<PackId> {
        self.debugname_to_id.get(debugname).copied()
    }

    pub fn get_by_id(&self, id: PackId) -> Option<&str> {
        self.id_to_debugname.get(&id).map(|s| s.as_str())
    }
}

pub struct PackedFile {
    pub server: PackedData,
    pub client: Option<PackedData>,
}

pub struct PackRegistry {
    pub npc: PackFile,
    pub obj: PackFile,
    pub loc: PackFile,
    pub hunt: PackFile,
    pub param: PackFile,
    pub category: PackFile,
    pub seq: PackFile,
    pub spotanim: PackFile,
    pub varp: PackFile,
    pub varn: PackFile,
    pub inv: PackFile,
    pub idk: PackFile,
    pub r#enum: PackFile,
    pub r#struct: PackFile,
    pub mesanim: PackFile,
    pub vars: PackFile,
    pub model: PackFile,
    pub interface: PackFile,
    pub dbrow: PackFile,
    pub synth: PackFile,
    pub flo: PackFile,
    pub texture: PackFile,
    pub anim: PackFile,
    pub midi: PackFile,
    #[cfg(since_254)]
    pub varbit: PackFile,
    #[cfg(since_647)]
    pub flu: PackFile,
    #[cfg(since_647)]
    pub stat: PackFile,
    #[cfg(since_647)]
    pub component_hashes: HashMap<String, i32>,
    pack_dir: PathBuf,
}

impl PackRegistry {
    pub fn load(pack_dir: &Path) -> Result<Self> {
        let load = |name: &str| -> Result<PackFile> {
            let path = pack_dir.join(format!("{name}.pack"));
            PackFile::load(&path)
        };

        let npc = load("npc")?;
        let obj = load("obj")?;
        let loc = load("loc")?;
        let hunt = load("hunt")?;
        let param = load("param")?;
        let category = load("category")?;
        let seq = load("seq")?;
        let spotanim = load("spotanim")?;
        let varp = load("varp")?;
        let varn = load("varn")?;
        let inv = load("inv")?;
        let idk = load("idk")?;
        let r#enum = load("enum")?;
        let r#struct = load("struct")?;
        let mesanim = load("mesanim")?;
        let vars = load("vars")?;
        let model = load("model")?;
        let interface = load("interface")?;
        #[cfg(since_647)]
        let component_hashes = load_component_hashes(&pack_dir.join("interface.pack"), &interface);
        let dbrow = load("dbrow")?;
        let synth = load("synth")?;
        let flo = load("flo")?;
        let texture = load("texture")?;
        let anim = load("anim")?;
        let midi = load("midi")?;
        #[cfg(since_254)]
        let varbit = load("varbit")?;

        debug!(
            "PackRegistry: npc={} obj={} loc={} hunt={} param={} cat={} seq={} varp={} varn={} model={} interface={} dbrow={} synth={} flo={} texture={} spotanim={} anim={} idk={} midi={}",
            npc.max,
            obj.max,
            loc.max,
            hunt.max,
            param.max,
            category.max,
            seq.max,
            varp.max,
            varn.max,
            model.max,
            interface.max,
            dbrow.max,
            synth.max,
            flo.max,
            texture.max,
            spotanim.max,
            anim.max,
            idk.max,
            midi.max,
        );

        Ok(Self {
            npc,
            obj,
            loc,
            hunt,
            param,
            category,
            seq,
            spotanim,
            varp,
            varn,
            inv,
            idk,
            r#enum,
            r#struct,
            mesanim,
            vars,
            model,
            interface,
            dbrow,
            synth,
            flo,
            texture,
            anim,
            midi,
            #[cfg(since_254)]
            varbit,
            #[cfg(since_647)]
            flu: load("flu")?,
            #[cfg(since_647)]
            stat: load("stat")?,
            #[cfg(since_647)]
            component_hashes,
            pack_dir: pack_dir.to_path_buf(),
        })
    }

    pub fn pack_dir(&self) -> &Path {
        &self.pack_dir
    }

    #[cfg(since_647)]
    pub fn check_content(&self, fc: &FileCache) -> Result<()> {
        let mut errors = Vec::new();
        let mut unregistered = 0;
        for (ext, pack) in [
            ("npc", &self.npc),
            ("obj", &self.obj),
            ("loc", &self.loc),
            ("varp", &self.varp),
            ("varbit", &self.varbit),
            ("seq", &self.seq),
            ("spotanim", &self.spotanim),
            ("inv", &self.inv),
            ("enum", &self.r#enum),
            ("struct", &self.r#struct),
            ("idk", &self.idk),
            ("flo", &self.flo),
            ("flu", &self.flu),
            ("hunt", &self.hunt),
            ("param", &self.param),
            ("mesanim", &self.mesanim),
        ] {
            for path in fc.collect(ext) {
                let Some(text) = fc.read(path) else {
                    continue;
                };
                for (line, raw) in text.lines().enumerate() {
                    let text = raw.split("//").next().unwrap_or(raw).trim();
                    let message = if let Some(name) =
                        text.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
                    {
                        let name = name.trim();
                        if pack.get_by_debugname(name).is_some() {
                            continue;
                        }
                        let id = name
                            .strip_prefix(ext)
                            .and_then(|s| s.strip_prefix('_'))
                            .filter(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
                            .and_then(|s| s.parse().ok());
                        let Some(id) = id else {
                            unregistered += 1;
                            continue;
                        };
                        match pack.get_by_id(id) {
                            Some(real) => format!(
                                "section [{name}] but {ext}.pack names id {id} '{real}'; \
                                 rename this header to [{real}] (or revert the {ext}.pack rename)"
                            ),
                            None => format!("section [{name}] but {ext}.pack has no id {id}"),
                        }
                    } else if ext == "npc"
                        && let Some((key, value)) = text.split_once('=')
                    {
                        let (key, value) = (key.trim(), value.trim());
                        let (target, label) = match key {
                            "multivar" => (&self.varbit, "varbit"),
                            "multivarp" => (&self.varp, "varp"),
                            "multidefault" => (&self.npc, "npc"),
                            _ if key.strip_prefix("multi").is_some_and(|s| {
                                !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())
                            }) =>
                            {
                                (&self.npc, "npc")
                            }
                            _ => continue,
                        };
                        if value == "null"
                            || value.parse::<i64>().is_ok()
                            || target.get_by_debugname(value).is_some()
                        {
                            continue;
                        }
                        format!(
                            "reference {key}={value} does not resolve to any {label} in \
                             {label}.pack; renamed in {label}.pack but not here?"
                        )
                    } else {
                        continue;
                    };
                    errors.push((path.clone(), line + 1, message));
                }
            }
        }
        if !errors.is_empty() {
            errors.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
            let errors = errors
                .iter()
                .map(|(path, line, message)| {
                    format!("[error] {}:{line}: {message}", path.display())
                })
                .collect::<Vec<_>>()
                .join("\n");
            bail!(
                "pack registries and content are out of sync:\n{errors}\n\
                 Rename the matching content sections and references together with their pack entries."
            );
        }
        if unregistered > 0 {
            info!("pack_all: {unregistered} unregistered content section(s) won't be packed");
        }
        Ok(())
    }
}

#[cfg(since_647)]
fn load_component_hashes(path: &Path, interfaces: &PackFile) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    let Ok(text) = fs::read_to_string(path) else {
        return map;
    };
    for line in text.lines() {
        let line = line.trim();
        let Some((id_str, comp_name)) = line.split_once('=') else {
            continue;
        };
        let Some((iface_str, comp_str)) = id_str.split_once(':') else {
            continue; // interface line, not a component
        };
        let (Ok(iface_id), Ok(comp_idx)) = (iface_str.parse::<u32>(), comp_str.parse::<u32>())
        else {
            continue;
        };
        let Some(iface_name) = interfaces.get_by_id(iface_id) else {
            continue;
        };
        let hash = ((iface_id << 16) | comp_idx) as i32;
        map.insert(format!("{iface_name}:{}", comp_name.trim()), hash);
    }
    map
}

#[cfg(since_647)]
const CONFIG_PACKS: &[(&str, bool)] = &[
    ("hunt", false),
    ("varn", false),
    ("vars", false),
    ("mesanim", false),
    ("struct", false),
    ("param", false),
    ("inv", false),
    ("enum", false),
    ("dbtable", false),
    ("dbrow", false),
    ("npc", true),
    ("obj", true),
    ("loc", true),
    ("seq", true),
    ("varp", true),
    ("spotanim", true),
    ("idk", true),
    ("flo", true),
];

/// Register server config names and validate client IDs against their pack files.
#[cfg(since_647)]
pub fn validate_config_packs(fc: &FileCache, pack_dir: &Path) -> Result<()> {
    for &(ext, transmitted) in CONFIG_PACKS {
        let pack_path = pack_dir.join(format!("{ext}.pack"));
        let mut pack = PackFile::load(&pack_path)?;

        // Crawl all [name] sections from content files
        let config_names = crawl_config_names(fc, ext);

        // Auto-register only for non-transmitted (server-only) types.
        // Transmitted-type ids are bound to the cache and must come from
        // `rs-pack discover symbols/npc`, never from validate.
        if !transmitted {
            let mut added = 0u32;
            for name in &config_names {
                if pack.get_by_debugname(name).is_none() && !name.starts_with("cert_") {
                    let new_id = pack.max;
                    pack.id_to_debugname.insert(new_id, name.to_string());
                    pack.debugname_to_id.insert(name.to_string(), new_id);
                    pack.max = new_id + 1;
                    added += 1;
                }
            }
            if added > 0 {
                save_pack_file(&pack_path, &pack)?;
                info!(
                    "  {ext}.pack: registered {added} new names (total: {})",
                    pack.max
                );
            }
        }

        // Non-transmitted names are registered above.
        if transmitted {
            let mut missing: Vec<&str> = Vec::new();
            for name in &config_names {
                if pack.get_by_debugname(name).is_none() && !name.starts_with("cert_") {
                    missing.push(name);
                }
            }
            if !missing.is_empty() {
                let list = missing
                    .iter()
                    .take(10)
                    .map(|n| format!("  [{n}]"))
                    .collect::<Vec<_>>()
                    .join("\n");
                tracing::warn!(
                    "{ext}.pack: {} config name(s) missing from pack — entries will be skipped at pack time:\n{list}",
                    missing.len()
                );
            }
        }
    }
    Ok(())
}

/// Crawl all `[name]` sections from config files of a given extension.
/// Matches 2004scape `crawlConfigNames()`.
#[cfg(since_647)]
fn crawl_config_names(fc: &FileCache, ext: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    for path in fc.collect(ext) {
        let Some(text) = fc.read(path) else {
            continue;
        };
        for line in text.lines() {
            let line = line.split("//").next().unwrap_or("").trim();
            if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                if seen.insert(name.to_string()) {
                    names.push(name.to_string());
                }
            }
        }
    }
    names
}

/// Write a pack file back to disk (sorted by ID).
#[cfg(since_647)]
fn save_pack_file(path: &Path, pack: &PackFile) -> Result<()> {
    let mut lines: Vec<(u32, &str)> = pack
        .id_to_debugname
        .iter()
        .map(|(&id, name)| (id, name.as_str()))
        .collect();
    lines.sort_by_key(|(id, _)| *id);
    let content: String = lines
        .iter()
        .map(|(id, name)| format!("{id}={name}\n"))
        .collect();
    fs::write(path, content)?;
    Ok(())
}
