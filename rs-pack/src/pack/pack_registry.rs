use crate::pack::incremental::{Hash, Key, Lookups, Registry, answer};
use crate::pack::pack::FileCache;
use crate::pack::pack_dir::replace;
use crate::pack::packed_data::PackedData;
use crate::pack::util::placeholder_prefix;
use anyhow::{Result, bail};
use runec::symbol::identifier_form;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info};

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

    /// A real name that isn't an identifier also answers to the identifier
    /// form scripts write it as, unless two names share that form.
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
    pub varcstr: PackFile,
    #[cfg(since_647)]
    pub varpstr: PackFile,
    #[cfg(since_647)]
    pub varsstr: PackFile,
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
    pub varconbit: PackFile,
    #[cfg(since_647)]
    pub varnbit: PackFile,
    #[cfg(since_647)]
    pub varclan: PackFile,
    #[cfg(since_647)]
    pub itemcode: PackFile,
    #[cfg(since_647)]
    pub config37: PackFile,
    #[cfg(since_647)]
    pub config38: PackFile,
    #[cfg(since_647)]
    pub config39: PackFile,
    #[cfg(since_647)]
    pub config45: PackFile,
    #[cfg(since_647)]
    pub wma: PackFile,
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
            // The pushes carry each field's cfg, which a vec![] cannot.
            #[allow(clippy::vec_init_then_push)]
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
    "varpbit" => varbit,
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
    "varcstr" => varcstr,
    #[cfg(since_647)]
    "varpstr" => varpstr,
    #[cfg(since_647)]
    "varsstr" => varsstr,
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
    #[cfg(since_647)]
    "varconbit" => varconbit,
    #[cfg(since_647)]
    "varnbit" => varnbit,
    #[cfg(since_647)]
    "varclan" => varclan,
    #[cfg(since_647)]
    "itemcode" => itemcode,
    #[cfg(since_647)]
    "config37" => config37,
    #[cfg(since_647)]
    "config38" => config38,
    #[cfg(since_647)]
    "config39" => config39,
    #[cfg(since_647)]
    "config45" => config45,
    #[cfg(since_647)]
    "wma" => wma,
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
        let varbit = load("varpbit")?;

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
            varcstr: load("varcstr")?,
            #[cfg(since_647)]
            varpstr: load("varpstr")?,
            #[cfg(since_647)]
            varsstr: load("varsstr")?,
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
            varconbit: load("varconbit")?,
            #[cfg(since_647)]
            varnbit: load("varnbit")?,
            #[cfg(since_647)]
            varclan: load("varclan")?,
            #[cfg(since_647)]
            itemcode: load("itemcode")?,
            #[cfg(since_647)]
            config37: load("config37")?,
            #[cfg(since_647)]
            config38: load("config38")?,
            #[cfg(since_647)]
            config39: load("config39")?,
            #[cfg(since_647)]
            config45: load("config45")?,
            #[cfg(since_647)]
            wma: load("wma")?,
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

    pub fn check_content(&self, fc: &FileCache) -> Result<()> {
        let mut errors = Vec::new();
        let dbtable = PackFile::load(&self.pack_dir.join("dbtable.pack"))?;
        for &ext in FileCache::TEXT_EXTS {
            let pack = match ext {
                "constant" | "if" => continue,
                "dbtable" => &dbtable,
                "quickchatcat" => self.file("chatcat").unwrap(),
                "quickchatphrase" => self.file("chatphrase").unwrap(),
                _ => self.file(ext).unwrap(),
            };
            for path in fc.collect(ext) {
                let Some(text) = fc.read(path) else {
                    errors.push((
                        path.clone(),
                        1,
                        "could not read config source as UTF-8".to_owned(),
                    ));
                    continue;
                };
                for (line, raw) in text.lines().enumerate() {
                    let text = raw.split("//").next().unwrap_or(raw).trim();
                    let message = if let Some(name) =
                        text.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
                    {
                        let name = name.trim();
                        if pack.debugname_to_id.contains_key(name)
                            || (ext == "varc"
                                && self
                                    .file("varcstr")
                                    .is_some_and(|pack| pack.debugname_to_id.contains_key(name)))
                        {
                            continue;
                        }
                        let id = pack.get_by_debugname(name).or_else(|| {
                            name.strip_prefix(placeholder_prefix(ext))
                                .and_then(|s| s.strip_prefix('_'))
                                .filter(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
                                .and_then(|s| s.parse().ok())
                        });
                        match id.and_then(|id| pack.get_by_id(id).map(|real| (id, real))) {
                            Some((id, real)) => format!(
                                "section [{name}] but {ext}.pack names id {id} '{real}'; \
                                 rename this header to [{real}] (or revert the {ext}.pack rename)"
                            ),
                            None => format!("section [{name}] is not registered in {ext}.pack"),
                        }
                    } else if ext == "npc"
                        && let Some((key, value)) = text.split_once('=')
                    {
                        let (key, value) = (key.trim(), value.trim());
                        let (value, resolves, label) = match key {
                            "multivar" => (
                                value,
                                self.file("varpbit")
                                    .is_some_and(|pack| pack.get_by_debugname(value).is_some())
                                    || self.varp.get_by_debugname(value).is_some(),
                                "varpbit",
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

const AUTO_REGISTER: &[&str] = &[
    "area", "varobj", "hunt", "varn", "vars", "mesanim", "struct", "param", "inv", "enum",
    "dbtable", "dbrow",
];

/// Require pack files, then register new server names when verification is disabled.
pub fn validate_config_packs(fc: &FileCache, pack_dir: &Path, verify: bool) -> Result<()> {
    let mut errors = Vec::new();
    for &ext in FileCache::TEXT_EXTS {
        let kind = match ext {
            "constant" => continue,
            "if" => "interface",
            "quickchatcat" => "chatcat",
            "quickchatphrase" => "chatphrase",
            ext => ext,
        };
        let Some(source) = fc.collect(ext).first() else {
            continue;
        };
        let pack_path = pack_dir.join(format!("{kind}.pack"));
        if !pack_path.is_file() {
            errors.push(format!(
                "{}: missing {} for .{ext} configs",
                source.display(),
                pack_path.display()
            ));
        }
    }
    if !errors.is_empty() {
        bail!("missing config pack files:\n{}", errors.join("\n"));
    }
    for &ext in AUTO_REGISTER {
        let pack_path = pack_dir.join(format!("{ext}.pack"));
        let mut pack = PackFile::load(&pack_path)?;
        let missing: Vec<_> = crawl_config_names(fc, ext)
            .into_iter()
            .filter(|name| pack.get_by_debugname(name).is_none())
            .collect();
        if missing.is_empty() {
            continue;
        }
        if verify {
            for name in missing {
                errors.push(format!("{ext}.pack: section [{name}] is not registered"));
            }
            continue;
        }
        let added = missing.len();
        for name in missing {
            let id = pack.max;
            pack.id_to_debugname.insert(id, name.clone());
            pack.debugname_to_id.insert(name, id);
            pack.max = id + 1;
        }
        save_pack_file(&pack_path, &pack)?;
        info!(
            "  {ext}.pack: registered {added} new names (total: {})",
            pack.max
        );
    }
    if !errors.is_empty() {
        bail!(
            "unregistered config sections (new IDs require --verify false and a server-only type):\n{}",
            errors.join("\n")
        );
    }
    Ok(())
}

/// Crawl all `[name]` sections from config files of a given extension.
/// Matches 2004scape `crawlConfigNames()`.
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
                let name = name.trim();
                if seen.insert(name.to_string()) {
                    names.push(name.to_string());
                }
            }
        }
    }
    names
}

/// Write a pack file back to disk (sorted by ID).
pub(crate) fn save_pack_file(path: &Path, pack: &PackFile) -> Result<()> {
    let mut lines: Vec<(PackId, &str)> = pack
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::pack::{ConfigFiles, check_configs};
    use std::{env, process};

    #[cfg(since_647)]
    #[test]
    fn the_cache_tables_hold_the_unpack_sections_of_every_kind() {
        use crate::pack::config::client_table;
        let dir = env::temp_dir().join(format!("rs-majula-config-paths-{}", process::id()));
        let scripts = dir.join("scripts/feature/configs");
        let unpack = dir.join("scripts/_unpack");
        fs::create_dir_all(&scripts).unwrap();
        fs::create_dir_all(&unpack).unwrap();
        let pack = PackFile::parse("0=first\n1=server\n3=last\n4=after\n");
        let kinds = FileCache::TEXT_EXTS
            .iter()
            .filter(|&&kind| !matches!(kind, "constant" | "if"));
        for &kind in kinds {
            fs::write(unpack.join(format!("all.{kind}")), "[first]\n\n[last]\n").unwrap();
            fs::write(
                scripts.join(format!("anything.{kind}")),
                "[server]\n\n[after]\n",
            )
            .unwrap();
            let fc = FileCache::new(&dir.join("scripts"));
            let table = client_table(&fc, kind, &pack, &HashMap::new()).unwrap();
            let lengths: Vec<_> = PackedData::entries(&table.dat, &table.idx)
                .unwrap()
                .map(|(_, bytes)| bytes.len())
                .collect();
            assert_eq!(lengths, [1, 1, 0, 1], "{kind}");
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unregistered_sections_are_errors_for_every_config_kind() {
        let dir = env::temp_dir().join(format!("rs-majula-unregistered-{}", process::id()));
        let scripts = dir.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        let kinds: Vec<_> = FileCache::TEXT_EXTS
            .iter()
            .copied()
            .filter(|ext| !matches!(*ext, "constant" | "if"))
            .collect();
        for ext in &kinds {
            fs::write(
                scripts.join(format!("custom.{ext}")),
                format!("[missing_{ext}]\n"),
            )
            .unwrap();
        }
        fs::write(
            scripts.join("custom.obj"),
            "[missing_obj]\n[cert_missing_obj]\n",
        )
        .unwrap();
        fs::write(dir.join("obj.pack"), "0=real name\n").unwrap();
        fs::write(scripts.join("alias.obj"), "[real_name]\n").unwrap();
        fs::write(scripts.join("unreadable.npc"), [0xff]).unwrap();
        let error = PackRegistry::load(&dir)
            .unwrap()
            .check_content(&FileCache::new(&scripts))
            .unwrap_err()
            .to_string();
        fs::remove_dir_all(&dir).unwrap();
        for ext in kinds {
            assert!(
                error.contains(&format!("[missing_{ext}] is not registered")),
                "{error}"
            );
        }
        assert!(
            error.contains("[cert_missing_obj] is not registered"),
            "{error}"
        );
        assert!(
            error.contains("section [real_name] but obj.pack names id 0 'real name'"),
            "{error}"
        );
        assert!(
            error.contains("could not read config source as UTF-8"),
            "{error}"
        );
    }

    #[test]
    fn configs_require_pack_files_before_registering_names() {
        let dir = env::temp_dir().join(format!("rs-majula-missing-packs-{}", process::id()));
        let scripts = dir.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        let kinds: Vec<_> = FileCache::TEXT_EXTS
            .iter()
            .copied()
            .filter(|ext| *ext != "constant")
            .collect();
        for ext in &kinds {
            fs::write(scripts.join(format!("all.{ext}")), "[first]\n").unwrap();
        }
        fs::write(scripts.join("all.constant"), "value=1\n").unwrap();
        let files = ConfigFiles::new(&scripts);
        for verify in [false, true] {
            let error = check_configs(&files, &dir, verify).unwrap_err().to_string();
            assert!(error.contains("missing config pack files"), "{error}");
            for ext in &kinds {
                assert!(error.contains(&format!("for .{ext} configs")), "{error}");
            }
            assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        }
        for ext in kinds {
            let kind = match ext {
                "if" => "interface",
                "quickchatcat" => "chatcat",
                "quickchatphrase" => "chatphrase",
                ext => ext,
            };
            fs::write(dir.join(format!("{kind}.pack")), "0=first\n").unwrap();
        }
        check_configs(&files, &dir, false).unwrap();
        fs::remove_file(dir.join("varp.pack")).unwrap();
        let error = check_configs(&files, &dir, false).unwrap_err().to_string();
        assert!(error.contains("for .varp configs"), "{error}");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn new_config_ids_require_verify_false() {
        let dir = env::temp_dir().join(format!("rs-majula-config-ids-{}", process::id()));
        let scripts = dir.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        let kinds: Vec<_> = AUTO_REGISTER
            .iter()
            .copied()
            .filter(|ext| FileCache::TEXT_EXTS.contains(ext))
            .collect();
        for ext in &kinds {
            fs::write(dir.join(format!("{ext}.pack")), "0=existing\n").unwrap();
            fs::write(
                scripts.join(format!("custom.{ext}")),
                "[existing]\n[new_name]\n",
            )
            .unwrap();
        }
        let files = ConfigFiles::new(&scripts);
        let error = check_configs(&files, &dir, true).unwrap_err().to_string();
        for ext in &kinds {
            assert!(
                error.contains(&format!("{ext}.pack: section [new_name]")),
                "{error}"
            );
            assert_eq!(
                fs::read_to_string(dir.join(format!("{ext}.pack"))).unwrap(),
                "0=existing\n"
            );
        }
        check_configs(&files, &dir, false).unwrap();
        check_configs(&files, &dir, true).unwrap();
        for ext in &kinds {
            assert_eq!(
                fs::read_to_string(dir.join(format!("{ext}.pack"))).unwrap(),
                "0=existing\n1=new_name\n"
            );
        }
        fs::write(dir.join("npc.pack"), "").unwrap();
        fs::write(scripts.join("custom.npc"), "[missing_npc]\n").unwrap();
        let error = check_configs(&ConfigFiles::new(&scripts), &dir, false)
            .unwrap_err()
            .to_string();
        fs::remove_dir_all(&dir).unwrap();
        assert!(
            error.contains("section [missing_npc] is not registered in npc.pack"),
            "{error}"
        );
    }
}
