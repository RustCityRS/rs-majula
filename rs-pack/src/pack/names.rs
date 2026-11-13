use crate::archive;
use crate::pack::incremental::{Lookups, Registry, answer};
use crate::pack::pack_registry::PackFile;
#[cfg(since_647)]
use crate::pack::util::placeholder_prefix;
use crate::pack::util::walk;
use anyhow::{Context, Result, bail, ensure};
#[cfg(since_647)]
use rs_io::js5::java_string_hash;
#[cfg(since_647)]
use runec::lexer::Lexer;
use runec::symbol::identifier_form;
#[cfg(since_647)]
use runec::token::Kind;
use std::collections::HashMap;
use std::fs;
#[cfg(since_647)]
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(since_647)]
use tracing::{info, warn};

pub struct AssetNames {
    packs: Arc<HashMap<String, PackFile>>,
    components: Arc<HashMap<u32, HashMap<u32, String>>>,
    lookups: Option<Lookups>,
}

impl runec::clientscript::decompile::Names for AssetNames {
    fn name(&self, kind: &str, id: u32) -> String {
        let name = self.name(kind, id);
        if kind == "interface" {
            identifier_form(&name)
        } else {
            name
        }
    }
    fn component_name(&self, interface: u32, component: u32) -> String {
        identifier_form(&self.component_name(interface, component))
    }
}

/// A unit packed from these names is reused while each name it looked up
/// still resolves as it did.
impl Registry for AssetNames {
    fn recording(&self) -> Self {
        Self {
            packs: self.packs.clone(),
            components: self.components.clone(),
            lookups: Some(Lookups::default()),
        }
    }

    fn recorded(&self) -> Vec<u8> {
        self.lookups.as_ref().map_or_else(Vec::new, Lookups::encode)
    }

    fn unchanged(&self, recorded: &[u8]) -> bool {
        Lookups::decode(recorded).is_some_and(|lookups| {
            lookups.iter().all(|(op, kind, query, answer)| {
                self.current(*op, kind, query).as_ref() == Some(answer)
            })
        })
    }
}

impl AssetNames {
    pub fn load(dir: &Path) -> Result<Self> {
        let mut packs = HashMap::new();
        let mut components: HashMap<u32, HashMap<u32, String>> = HashMap::new();
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "pack") {
                let name = path
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .context("Invalid pack name")?;
                let text = fs::read_to_string(&path).unwrap_or_default();
                if name == "interface" {
                    for line in text.lines() {
                        let Some((id, name)) = line.split_once('=') else {
                            continue;
                        };
                        let Some((group, file)) = id.split_once(':') else {
                            continue;
                        };
                        components
                            .entry(group.parse()?)
                            .or_default()
                            .insert(file.parse()?, name.to_owned());
                    }
                }
                packs.insert(name.to_owned(), PackFile::parse(&text));
            }
        }
        Ok(Self {
            packs: Arc::new(packs),
            components: Arc::new(components),
            lookups: None,
        })
    }

    /// What a recorded lookup gives now.
    fn current(&self, op: u8, kind: &str, query: &str) -> Option<String> {
        Some(match op {
            b'i' => answer(self.lookup(kind, query).ok()),
            b'n' => self.find_name(kind, query.parse().ok()?),
            b'C' => self.find_component_name(kind.parse().ok()?, query.parse().ok()?),
            b'c' => answer(self.find_component_id(kind.parse().ok()?, query).ok()),
            _ => return None,
        })
    }

    pub fn name(&self, kind: &str, id: u32) -> String {
        let name = self.find_name(kind, id);
        if let Some(lookups) = &self.lookups {
            lookups.record(b'n', kind, &id.to_string(), || name.clone());
        }
        name
    }

    fn find_name(&self, kind: &str, id: u32) -> String {
        self.packs
            .get(kind)
            .and_then(|p| p.get_by_id(id))
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{}_{id}", asset_prefix(kind)))
    }

    pub fn id(&self, kind: &str, name: &str) -> Result<u32> {
        let id = self.lookup(kind, name);
        if let Some(lookups) = &self.lookups {
            lookups.record(b'i', kind, name, || answer(id.as_ref().ok()));
        }
        id
    }

    /// [`id`](Self::id) without recording: also how [`files`](Self::files)
    /// finds a stage's files, which is not what it packs from them.
    fn lookup(&self, kind: &str, name: &str) -> Result<u32> {
        if let Some(id) = self.packs.get(kind).and_then(|p| p.get_by_debugname(name)) {
            return Ok(id);
        }
        let number = name
            .strip_prefix(&format!("{kind}_"))
            .or_else(|| name.strip_prefix(&format!("{}_", asset_prefix(kind))))
            .unwrap_or(name);
        if let Ok(id) = number.parse() {
            return Ok(id);
        }
        bail!("Unknown {kind}: {name}")
    }

    pub fn reference(&self, kind: &str, id: i64) -> String {
        if id < 0 {
            "null".to_owned()
        } else {
            self.name(kind, id as u32)
        }
    }

    pub fn resolve(&self, kind: &str, value: &str) -> Result<i64> {
        if value == "null" || value == "-1" {
            Ok(-1)
        } else {
            Ok(self.id(kind, value)? as i64)
        }
    }

    pub fn files(
        &self,
        dir: &Path,
        kind: &str,
        extensions: &[&str],
    ) -> Result<Vec<(u32, PathBuf)>> {
        let mut files = HashMap::new();
        for path in walk(dir) {
            if !path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|e| extensions.contains(&e))
            {
                continue;
            }
            let relative = path.strip_prefix(dir)?;
            let full = relative
                .to_str()
                .context("Invalid asset filename")?
                .replace('\\', "/");
            let name = relative.with_extension("");
            let name = name
                .to_str()
                .context("Invalid asset filename")?
                .replace('\\', "/");
            let id = self
                .lookup(kind, &full)
                .or_else(|_| self.lookup(kind, &name))
                .with_context(|| path.display().to_string())?;
            ensure!(
                files.insert(id, path).is_none(),
                "Duplicate {kind} asset {id}"
            );
        }
        let mut files: Vec<_> = files.into_iter().collect();
        files.sort_by_key(|(id, _)| *id);
        Ok(files)
    }

    pub fn component_name(&self, group: u32, file: u32) -> String {
        let name = self.find_component_name(group, file);
        if let Some(lookups) = &self.lookups {
            lookups.record(b'C', &group.to_string(), &file.to_string(), || name.clone());
        }
        name
    }

    fn find_component_name(&self, group: u32, file: u32) -> String {
        self.components
            .get(&group)
            .and_then(|v| v.get(&file))
            .cloned()
            .unwrap_or_else(|| {
                #[cfg(before_647)]
                {
                    format!("com{file}")
                }
                #[cfg(since_647)]
                {
                    format!("com_{file}")
                }
            })
    }

    pub fn component_id(&self, group: u32, name: &str) -> Result<u32> {
        let id = self.find_component_id(group, name);
        if let Some(lookups) = &self.lookups {
            lookups.record(b'c', &group.to_string(), name, || answer(id.as_ref().ok()));
        }
        id
    }

    fn find_component_id(&self, group: u32, name: &str) -> Result<u32> {
        let components = self.components.get(&group);
        if let Some((&id, _)) = components
            .and_then(|v| v.iter().find(|(_, value)| *value == name))
            .or_else(|| {
                components.and_then(|v| v.iter().find(|(_, value)| identifier_form(value) == name))
            })
        {
            return Ok(id);
        }
        name.strip_prefix("com_")
            .or_else(|| name.strip_prefix("com"))
            .unwrap_or(name)
            .parse()
            .with_context(|| format!("Unknown component: {name}"))
    }
}

/// What an asset of a table goes by without a name of its own. Textures keep
/// their table's name: their files carry it.
#[cfg(since_647)]
fn asset_prefix(kind: &str) -> &str {
    if kind == "texture" {
        kind
    } else {
        placeholder_prefix(kind)
    }
}

#[cfg(before_647)]
fn asset_prefix(kind: &str) -> &str {
    kind
}

pub fn archive_name(id: u8) -> &'static str {
    match id {
        archive::ANIMS => "anim",
        archive::BASES => "base",
        archive::INTERFACES => "interface",
        archive::SYNTH => "synth",
        archive::MAPS => "map",
        archive::SONGS => "songs",
        archive::MODELS => "model",
        archive::SPRITES => "sprites",
        archive::TEXTURES => "texture",
        archive::BINARY => "binary",
        archive::JINGLES => "jingle",
        archive::CLIENTSCRIPT => "clientscript",
        archive::FONTMETRICS => "fontmetrics",
        archive::VORBIS => "vorbis",
        archive::INSTRUMENTS => "instrument",
        archive::WORLDMAP => "worldmap",
        archive::MATERIALS => "material",
        archive::DEFAULTS => "defaults",
        archive::DLLS => "dlls",
        archive::SHADERS => "shaders",
        archive::LOADING_SPRITES | archive::LOADING_SPRITES_RAW => "loading_sprites",
        archive::LOADING_SCREENS => "loading_backgrounds",
        archive::WORLDMAP_LOCS => "worldmap_loc",
        _ => "asset",
    }
}

/// The 647 client's script triggers. A trigger script's cache name hash is
/// `trigger | subject << 10` (`GLXTexture3D.method3079`), not a string hash.
#[cfg(since_647)]
const CLIENT_TRIGGERS: [(&str, i32); 10] = [
    ("opworldmapelement1", 10),
    ("opworldmapelement2", 11),
    ("opworldmapelement3", 12),
    ("opworldmapelement4", 13),
    ("opworldmapelement5", 14),
    ("worldmapelementmouseover", 15),
    ("worldmapelementmouseleave", 16),
    ("worldmapelementmouserepeat", 17),
    ("video_end", 18),
    ("jcoins_updated", 19),
];

/// Fails the pack when a registry name for an asset whose name hash comes from
/// `<kind>.hashes` or `interfaces.fileids` is neither the real name that hash
/// was made from nor the placeholder for its id. A real interface or component
/// name that isn't an identifier must lex as one in its identifier form, used
/// by no other name in its scope: that form is how scripts write it.
#[cfg(since_647)]
pub fn verify_names(content_dir: &Path, pack_dir: &Path, strict: bool) -> Result<()> {
    let (checked, problems) = name_problems(content_dir, pack_dir)?;
    if problems.is_empty() {
        info!("Name gate: {checked} hashed names match the cache");
        return Ok(());
    }
    let list = problems
        .iter()
        .take(20)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n  ");
    if strict {
        bail!(
            "Name gate: {} problem(s) in {checked} hashed names:\n  {list}",
            problems.len()
        );
    }
    warn!(
        "Name gate: {} problem(s) in {checked} hashed names (verification disabled):\n  {list}",
        problems.len()
    );
    Ok(())
}

#[cfg(since_647)]
fn name_problems(content_dir: &Path, pack_dir: &Path) -> Result<(usize, Vec<String>)> {
    let names = AssetNames::load(pack_dir)?;
    let mut kinds = Vec::new();
    for entry in fs::read_dir(pack_dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "hashes")
            && let Some(kind) = path.file_stem().and_then(|s| s.to_str())
            && names.packs.contains_key(kind)
        {
            kinds.push(kind.to_owned());
        }
    }
    kinds.sort();
    let mut checked = 0;
    let mut problems = Vec::new();
    for kind in &kinds {
        let mut hashed = Vec::new();
        for (key, hash) in read_pairs(&pack_dir.join(format!("{kind}.hashes")))? {
            let id: u32 = key
                .parse()
                .with_context(|| format!("{kind}.hashes: {key}"))?;
            if let Some(name) = names.packs[kind].get_by_id(id) {
                hashed.push((
                    kind.as_str(),
                    key,
                    id,
                    name.to_owned(),
                    hash.parse::<i32>()?,
                ));
            }
        }
        if kind == "interface" {
            let fileids = fs::read_to_string(content_dir.join("versions/interfaces.fileids"))?;
            for line in fileids.lines() {
                let Some((group, files)) = line.split_once('=') else {
                    continue;
                };
                let group: u32 = group.parse()?;
                for (file, hash) in files.split(',').filter_map(|e| e.split_once(':')) {
                    let file: u32 = file.parse()?;
                    if let Some(name) = names.components.get(&group).and_then(|c| c.get(&file)) {
                        hashed.push((
                            "component",
                            format!("{group}:{file}"),
                            file,
                            name.clone(),
                            hash.parse()?,
                        ));
                    }
                }
            }
        }
        for (kind, key, id, name, hash) in hashed {
            if hash == -1 {
                continue;
            }
            checked += 1;
            if java_string_hash(&name) != hash
                && trigger_hash(&names, &name) != Some(hash)
                && !is_placeholder(kind, &name, id)
            {
                problems.push(format!(
                    "{kind} {key}: {name:?} doesn't match the cache hash"
                ));
            }
        }
    }
    if let Some(interfaces) = names.packs.get("interface") {
        let entries = interfaces
            .id_to_debugname
            .iter()
            .map(|(id, name)| (id.to_string(), name.as_str()));
        identifier_problems("interface", entries.collect(), &mut problems);
    }
    for (group, components) in names.components.iter() {
        let entries = components
            .iter()
            .map(|(file, name)| (format!("{group}:{file}"), name.as_str()));
        identifier_problems("component", entries.collect(), &mut problems);
    }
    problems.sort();
    Ok((checked, problems))
}

#[cfg(since_647)]
fn identifier_problems(kind: &str, mut entries: Vec<(String, &str)>, problems: &mut Vec<String>) {
    entries.sort();
    let mut forms = HashMap::new();
    for (key, name) in entries {
        let form = identifier_form(name);
        if !lexes_as_identifier(&form) {
            problems.push(format!(
                "{kind} {key}: {name:?} has no identifier form ({form}); use its placeholder"
            ));
        }
        if let Some(other) = forms.insert(form.clone(), key.clone()) {
            problems.push(format!(
                "{kind} {key}: {name:?} is written {form}, as {kind} {other} is"
            ));
        }
    }
}

/// runec's parser also takes a trigger keyword (`login`, `logout`) as a name.
#[cfg(since_647)]
fn lexes_as_identifier(form: &str) -> bool {
    let path = PathBuf::new();
    Lexer::new(form, &path)
        .tokenize()
        .is_ok_and(|tokens| match tokens.as_slice() {
            [token, end] => {
                matches!(token.kind, Kind::Identifier | Kind::Trigger)
                    && token.value == form
                    && matches!(end.kind, Kind::EndOfFile)
            }
            _ => false,
        })
}

#[cfg(since_647)]
fn read_pairs(path: &Path) -> Result<HashMap<String, String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(HashMap::new()),
        Err(e) => Err(e).with_context(|| path.display().to_string()),
    }
}

#[cfg(since_647)]
fn is_placeholder(kind: &str, name: &str, id: u32) -> bool {
    match kind {
        "clientscript" => {
            name == format!("[proc,script{id}]") || name == format!("[clientscript,script{id}]")
        }
        "component" => name == format!("com_{id}"),
        "fontmetrics" => name == format!("fontmetrics_{id}") || name == format!("graphic_{id}"),
        _ => name == format!("{}_{id}", asset_prefix(kind)) || name == format!("{kind}_{id}"),
    }
}

#[cfg(since_647)]
fn trigger_hash(names: &AssetNames, name: &str) -> Option<i32> {
    let (trigger, subject) = name.strip_prefix('[')?.strip_suffix(']')?.split_once(',')?;
    let (_, id) = CLIENT_TRIGGERS.iter().find(|(t, _)| *t == trigger)?;
    let subject = if subject == "_" {
        0xFFFF
    } else if let Some(category) = subject.strip_prefix('_') {
        names.id("category", category).ok()? as i32 + 65536
    } else {
        names.id("mel", subject).ok()? as i32
    };
    Some(id | subject << 10)
}

#[cfg(all(test, since_647))]
mod tests {
    use super::*;
    use std::process;

    #[test]
    fn names_must_be_real_or_placeholders() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/name-gate-test")
            .join(process::id().to_string());
        let pack = dir.join("pack");
        fs::create_dir_all(dir.join("versions")).unwrap();
        fs::create_dir_all(&pack).unwrap();
        fs::write(
            pack.join("clientscript.pack"),
            "0=[clientscript,good]\n1=[clientscript,script1]\n2=[proc,guess]\n3=[opworldmapelement1,mapelement_2]\n4=bare\n",
        )
        .unwrap();
        fs::write(
            pack.join("clientscript.hashes"),
            format!(
                "0={}\n1=7\n2=9\n3={}\n4={}\n",
                java_string_hash("[clientscript,good]"),
                10 | 2 << 10,
                java_string_hash("bare")
            ),
        )
        .unwrap();
        fs::write(
            pack.join("interface.pack"),
            "0=cove lid\n0:0=a history\n0:1=com_1\n0:2=meter1-\n0:3=meter1_\n0:4=2\n0:5=logout\n",
        )
        .unwrap();
        fs::write(
            pack.join("interface.hashes"),
            format!("0={}\n", java_string_hash("cove lid")),
        )
        .unwrap();
        fs::write(
            dir.join("versions/interfaces.fileids"),
            format!(
                "0=0:{},1:77,2:{},3:{},4:{},5:{}\n",
                java_string_hash("a history"),
                java_string_hash("meter1-"),
                java_string_hash("meter1_"),
                java_string_hash("2"),
                java_string_hash("logout")
            ),
        )
        .unwrap();
        let names = AssetNames::load(&pack).unwrap();
        assert_eq!(names.id("interface", "cove_lid").unwrap(), 0);
        assert_eq!(names.component_id(0, "a_history").unwrap(), 0);

        let (checked, problems) = name_problems(&dir, &pack).unwrap();
        assert_eq!(checked, 12);
        assert_eq!(
            problems,
            [
                "clientscript 2: \"[proc,guess]\" doesn't match the cache hash",
                "component 0:3: \"meter1_\" is written meter1_, as component 0:2 is",
                "component 0:4: \"2\" has no identifier form (2); use its placeholder",
            ]
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
