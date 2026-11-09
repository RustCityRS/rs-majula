use crate::pack::incremental::{Hash, Key, Lookups, Registry, answer};
#[cfg(since_647)]
use crate::pack::pack::FileCache;
#[cfg(since_647)]
use crate::pack::pack_dir::replace;
use crate::pack::packed_data::PackedData;
#[cfg(since_647)]
use crate::pack::util::placeholder_prefix;
use anyhow::Result;
#[cfg(since_647)]
use anyhow::bail;
#[cfg(since_647)]
use runec::symbol::identifier_form;
use std::collections::HashMap;
#[cfg(since_647)]
use std::collections::HashSet;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;
#[cfg(since_647)]
use tracing::info;

#[cfg(before_647)]
pub type PackId = u16;
#[cfg(since_647)]
pub type PackId = u32;

/// A registry's names. Copies share them, and a pack cache view of a
/// [`PackRegistry`] records the lookups made through it.
#[derive(Clone)]
pub struct PackFile {
    entries: Arc<PackEntries>,
    lookups: Option<(Arc<str>, Arc<Lookups>)>,
}

#[derive(Clone)]
pub struct PackEntries {
    pub id_to_debugname: HashMap<PackId, String>,
    pub debugname_to_id: HashMap<String, PackId>,
    #[cfg(since_647)]
    identifiers: HashMap<String, PackId>,
    pub max: PackId,
    key: Key,
}

impl Deref for PackFile {
    type Target = PackEntries;

    fn deref(&self) -> &PackEntries {
        &self.entries
    }
}

impl DerefMut for PackFile {
    fn deref_mut(&mut self) -> &mut PackEntries {
        Arc::make_mut(&mut self.entries)
    }
}

impl PackFile {
    pub fn empty() -> Self {
        Self::parse("")
    }

    pub fn load(path: &Path) -> Result<Self> {
        Ok(Self::parse(&fs::read_to_string(path).unwrap_or_default()))
    }

    pub fn parse(text: &str) -> Self {
        let mut id_to_debugname = HashMap::new();
        let mut debugname_to_id = HashMap::new();
        let mut max: PackId = 0;

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

        Self {
            entries: Arc::new(PackEntries {
                #[cfg(since_647)]
                identifiers: identifier_aliases(&debugname_to_id),
                id_to_debugname,
                debugname_to_id,
                max,
                key: Hash::new("registry").bytes(text.as_bytes()).key(),
            }),
            lookups: None,
        }
    }

    pub fn get_by_debugname(&self, debugname: &str) -> Option<PackId> {
        let id = self.find(debugname);
        if let Some((kind, lookups)) = &self.lookups {
            lookups.record(b'i', kind, debugname, || answer(id));
        }
        id
    }

    #[cfg(before_647)]
    fn find(&self, debugname: &str) -> Option<PackId> {
        self.debugname_to_id.get(debugname).copied()
    }

    /// A real name that isn't an identifier also answers to the identifier
    /// form scripts write it as, unless two names share that form.
    #[cfg(since_647)]
    fn find(&self, debugname: &str) -> Option<PackId> {
        self.debugname_to_id
            .get(debugname)
            .or_else(|| self.identifiers.get(debugname))
            .copied()
    }

    pub fn get_by_id(&self, id: PackId) -> Option<&str> {
        let name = self.id_to_debugname.get(&id).map(|s| s.as_str());
        if let Some((kind, lookups)) = &self.lookups {
            lookups.record(b'n', kind, &id.to_string(), || answer(name));
        }
        name
    }
}

#[cfg(since_647)]
fn identifier_aliases(names: &HashMap<String, PackId>) -> HashMap<String, PackId> {
    let mut aliases: HashMap<String, Option<PackId>> = HashMap::new();
    for (name, &id) in names {
        let form = identifier_form(name);
        if form != *name && !names.contains_key(&form) {
            aliases
                .entry(form)
                .and_modify(|shared| *shared = None)
                .or_insert(Some(id));
        }
    }
    aliases
        .into_iter()
        .filter_map(|(form, id)| Some((form, id?)))
        .collect()
}

pub struct PackedFile {
    pub server: PackedData,
    pub client: Option<PackedData>,
}

#[derive(Clone)]
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
    pub bas: PackFile,
    #[cfg(since_647)]
    pub cursor: PackFile,
    #[cfg(since_647)]
    pub hitmark: PackFile,
    #[cfg(since_647)]
    pub billboard: PackFile,
    #[cfg(since_647)]
    pub fontmetrics: PackFile,
    #[cfg(since_647)]
    pub particleeffector: PackFile,
    #[cfg(since_647)]
    pub particleemitter: PackFile,
    #[cfg(since_647)]
    pub light: PackFile,
    #[cfg(since_647)]
    pub mel: PackFile,
    #[cfg(since_647)]
    pub msi: PackFile,
    #[cfg(since_647)]
    pub quest: PackFile,
    #[cfg(since_647)]
    pub chatcat: PackFile,
    #[cfg(since_647)]
    pub chatphrase: PackFile,
    #[cfg(since_647)]
    pub sprites: PackFile,
    #[cfg(since_647)]
    pub varc: PackFile,
    #[cfg(since_647)]
    pub varcs: PackFile,
    #[cfg(since_647)]
    pub varclansetting: PackFile,
    #[cfg(since_647)]
    pub stat: PackFile,
    #[cfg(since_647)]
    pub area: PackFile,
    #[cfg(since_647)]
    pub varobj: PackFile,
    #[cfg(since_647)]
    pub controller: PackFile,
    #[cfg(since_647)]
    pub varcon: PackFile,
    #[cfg(since_647)]
    pub component_hashes: Arc<HashMap<String, i32>>,
    pack_dir: PathBuf,
    lookups: Option<Arc<Lookups>>,
}

/// A config kind packed from a registry view is reused while every name it
/// looked up still resolves as it did.
impl Registry for PackRegistry {
    fn recording(&self) -> Self {
        let lookups = Arc::new(Lookups::default());
        let mut view = self.clone();
        for (kind, file) in view.files_mut() {
            file.lookups = Some((Arc::from(kind), lookups.clone()));
        }
        view.lookups = Some(lookups);
        view
    }

    fn recorded(&self) -> Vec<u8> {
        self.lookups
            .as_deref()
            .map_or_else(Vec::new, Lookups::encode)
    }

    fn unchanged(&self, recorded: &[u8]) -> bool {
        Lookups::decode(recorded).is_some_and(|lookups| {
            lookups.iter().all(|(op, kind, query, answer)| {
                self.current(*op, kind, query).as_ref() == Some(answer)
            })
        })
    }
}

/// The registries by the name their `.pack` file goes by.
macro_rules! registry_files {
    ($($(#[$cfg:meta])* $kind:literal => $field:ident,)*) => {
        impl PackRegistry {
            pub(crate) fn file(&self, kind: &str) -> Option<&PackFile> {
                match kind {
                    $($(#[$cfg])* $kind => Some(&self.$field),)*
                    _ => None,
                }
            }

            /// Every registry, named: a field left out of the list doesn't
            /// compile, so no registry's lookups go unrecorded.
            fn files_mut(&mut self) -> Vec<(&'static str, &mut PackFile)> {
                let PackRegistry {
                    $($(#[$cfg])* $field,)*
                    #[cfg(since_647)]
                    component_hashes: _,
                    pack_dir: _,
                    lookups: _,
                } = self;
                let mut files = Vec::new();
                $($(#[$cfg])* files.push(($kind, $field));)*
                files
            }
        }
    };
}

registry_files! {
    "npc" => npc,
    "obj" => obj,
    "loc" => loc,
    "hunt" => hunt,
    "param" => param,
    "category" => category,
    "seq" => seq,
    "spotanim" => spotanim,
    "varp" => varp,
    "varn" => varn,
    "inv" => inv,
    "idk" => idk,
    "enum" => r#enum,
    "struct" => r#struct,
    "mesanim" => mesanim,
    "vars" => vars,
    "model" => model,
    "interface" => interface,
    "dbrow" => dbrow,
    "synth" => synth,
    "flo" => flo,
    "texture" => texture,
    "anim" => anim,
    "midi" => midi,
    #[cfg(since_254)]
    "varbit" => varbit,
    #[cfg(since_647)]
    "flu" => flu,
    #[cfg(since_647)]
    "bas" => bas,
    #[cfg(since_647)]
    "cursor" => cursor,
    #[cfg(since_647)]
    "hitmark" => hitmark,
    #[cfg(since_647)]
    "billboard" => billboard,
    #[cfg(since_647)]
    "fontmetrics" => fontmetrics,
    #[cfg(since_647)]
    "particleeffector" => particleeffector,
    #[cfg(since_647)]
    "particleemitter" => particleemitter,
    #[cfg(since_647)]
    "light" => light,
    #[cfg(since_647)]
    "mel" => mel,
    #[cfg(since_647)]
    "msi" => msi,
    #[cfg(since_647)]
    "quest" => quest,
    #[cfg(since_647)]
    "chatcat" => chatcat,
    #[cfg(since_647)]
    "chatphrase" => chatphrase,
    #[cfg(since_647)]
    "sprites" => sprites,
    #[cfg(since_647)]
    "varc" => varc,
    #[cfg(since_647)]
    "varcs" => varcs,
    #[cfg(since_647)]
    "varclansetting" => varclansetting,
    #[cfg(since_647)]
    "stat" => stat,
    #[cfg(since_647)]
    "area" => area,
    #[cfg(since_647)]
    "varobj" => varobj,
    #[cfg(since_647)]
    "controller" => controller,
    #[cfg(since_647)]
    "varcon" => varcon,
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
            bas: load("bas")?,
            #[cfg(since_647)]
            cursor: load("cursor")?,
            #[cfg(since_647)]
            hitmark: load("hitmark")?,
            #[cfg(since_647)]
            billboard: load("billboard")?,
            #[cfg(since_647)]
            fontmetrics: load("fontmetrics")?,
            #[cfg(since_647)]
            particleeffector: load("particleeffector")?,
            #[cfg(since_647)]
            particleemitter: load("particleemitter")?,
            #[cfg(since_647)]
            light: load("light")?,
            #[cfg(since_647)]
            mel: load("mel")?,
            #[cfg(since_647)]
            msi: load("msi")?,
            #[cfg(since_647)]
            quest: load("quest")?,
            #[cfg(since_647)]
            chatcat: load("chatcat")?,
            #[cfg(since_647)]
            chatphrase: load("chatphrase")?,
            #[cfg(since_647)]
            sprites: load("sprites")?,
            #[cfg(since_647)]
            varc: load("varc")?,
            #[cfg(since_647)]
            varcs: load("varcs")?,
            #[cfg(since_647)]
            varclansetting: load("varclansetting")?,
            #[cfg(since_647)]
            stat: load("stat")?,
            #[cfg(since_647)]
            area: load("area")?,
            #[cfg(since_647)]
            varobj: load("varobj")?,
            #[cfg(since_647)]
            controller: load("controller")?,
            #[cfg(since_647)]
            varcon: load("varcon")?,
            #[cfg(since_647)]
            component_hashes: Arc::new(component_hashes),
            pack_dir: pack_dir.to_path_buf(),
            lookups: None,
        })
    }

    pub fn pack_dir(&self) -> &Path {
        &self.pack_dir
    }

    /// A component's hash by its `interface:component` name.
    #[cfg(since_647)]
    pub fn component_hash(&self, name: &str) -> Option<i32> {
        let hash = self.component_hashes.get(name).copied();
        if let Some(lookups) = &self.lookups {
            lookups.record(b'c', "component", name, || answer(hash));
        }
        hash
    }

    /// Records that a unit reads all of registry `kind`, as a config kind
    /// does its own: any change to it repacks the unit.
    pub fn depend_on(&self, kind: &str) {
        if let Some(lookups) = &self.lookups {
            let key = self.file(kind).map(|file| file.key);
            lookups.record(b'w', kind, "", || answer(key));
        }
    }

    /// What a recorded lookup gives now.
    fn current(&self, op: u8, kind: &str, query: &str) -> Option<String> {
        Some(match op {
            b'i' => answer(self.file(kind)?.find(query)),
            b'n' => answer(self.file(kind)?.id_to_debugname.get(&query.parse().ok()?)),
            #[cfg(since_647)]
            b'c' => answer(self.component_hashes.get(query)),
            b'w' => answer(self.file(kind).map(|file| file.key)),
            _ => return None,
        })
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
            ("vars", &self.vars),
            ("varn", &self.varn),
            ("bas", &self.bas),
            ("cursor", &self.cursor),
            ("hitmark", &self.hitmark),
            ("billboard", &self.billboard),
            ("particleeffector", &self.particleeffector),
            ("particleemitter", &self.particleemitter),
            ("light", &self.light),
            ("mel", &self.mel),
            ("msi", &self.msi),
            ("quest", &self.quest),
            ("quickchatcat", &self.chatcat),
            ("quickchatphrase", &self.chatphrase),
            ("area", &self.area),
            ("varobj", &self.varobj),
            ("controller", &self.controller),
            ("varcon", &self.varcon),
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
                            .strip_prefix(placeholder_prefix(ext))
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
                        let (value, resolves, label) = match key {
                            "multivar" => (
                                value,
                                self.varbit.get_by_debugname(value).is_some()
                                    || self.varp.get_by_debugname(value).is_some(),
                                "varbit",
                            ),
                            "multinpc" => {
                                let target = value.rsplit(',').next().unwrap_or(value).trim();
                                (target, self.npc.get_by_debugname(target).is_some(), "npc")
                            }
                            _ => continue,
                        };
                        if value == "null" || value.parse::<i64>().is_ok() || resolves {
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
        map.insert(
            format!(
                "{}:{}",
                identifier_form(iface_name),
                identifier_form(comp_name)
            ),
            hash,
        );
    }
    map
}

#[cfg(since_647)]
const CONFIG_PACKS: &[(&str, bool)] = &[
    ("area", false),
    ("varobj", false),
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
    replace(path, content.as_bytes())?;
    Ok(())
}
