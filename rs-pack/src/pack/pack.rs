use crate::CONFIG_TYPE_CRCS;
#[cfg(since_647)]
use crate::archive;
#[cfg(since_647)]
use crate::cache::js5::PackedCache;
use crate::pack::config::*;
use crate::pack::incremental::{Key, Stage, dir_key};
#[cfg(before_647)]
use crate::pack::interface;
#[cfg(since_647)]
use crate::pack::names::AssetNames;
#[cfg(since_647)]
use crate::pack::other::{map, shader, song};
use crate::pack::pack_registry::{PackRegistry, PackedFile, validate_config_packs};
use crate::pack::util::verify_crc;
#[cfg(since_647)]
use crate::pack::{config, interface, js5, media, model, sound, texture};
use anyhow::{Context, Result, bail};
#[cfg(since_647)]
use rayon::join;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;
use tracing::debug;

/// The registries config packing resolves names through: all but the script
/// ids, which no config names.
fn registry_key(pack_dir: &Path) -> Key {
    dir_key(pack_dir, &["clientscript.pack"])
}

/// The registry a kind reads whole: it packs an entry for every id in it.
fn own_registry(kind: &str) -> Option<&str> {
    match kind {
        "dbtable" | "mes" => None,
        "quickchatcat" => Some("chatcat"),
        "quickchatphrase" => Some("chatphrase"),
        kind => Some(kind),
    }
}

/// Registry files a kind reads from disk itself, which its registry view
/// doesn't see.
fn read_registries(kind: &str) -> &'static [&'static str] {
    match kind {
        "dbrow" | "dbtable" => &["dbtable.pack"],
        "interface" => &["interface.order"],
        "mesanim" => &["mesanim.order"],
        _ => &[],
    }
}

/// The config files a kind packs from: its own, plus the constants and param
/// types every kind can read.
fn config_inputs(kind: &str) -> Vec<&str> {
    let mut exts = match kind {
        "category" => vec![],
        "dbrow" => vec!["dbrow", "dbtable"],
        "obj" => vec!["obj", "varobj"],
        "controller" => vec!["controller", "varcon", "varconbit"],
        "varconbit" => vec!["varconbit", "varcon"],
        "mes" => vec!["mesanim"],
        "varcstr" => vec!["varc"],
        "varpstr" => vec!["varp"],
        "varsstr" => vec!["vars"],
        "interface" => vec!["if"],
        kind => vec![kind],
    };
    exts.extend(["constant", "param"]);
    exts
}

fn pack_cached_assets(
    registry: &PackRegistry,
    files: &ConfigFiles,
    verify: bool,
) -> anyhow::Result<HashMap<String, PackedFile>> {
    let loaded_constants = OnceLock::new();
    let constants = || {
        loaded_constants.get_or_init(|| {
            let constants = load_constants_from_cache(files.read());
            debug!("  Loaded {} constants", constants.len());
            constants
        })
    };
    let loaded_param_types = OnceLock::new();
    let param_types = || -> anyhow::Result<&HashMap<String, String>> {
        if let Some(types) = loaded_param_types.get() {
            return Ok(types);
        }
        let types = param::load_param_types(files.read(), constants())?;
        Ok(loaded_param_types.get_or_init(|| types))
    };
    let stage = Stage::new("configs", vec![("verify".to_owned(), Key::from(verify))]);
    let kinds = [
        "param",
        "dbtable",
        "dbrow",
        "enum",
        "flo",
        "inv",
        "mesanim",
        "struct",
        "seq",
        "loc",
        "npc",
        "obj",
        "varp",
        "hunt",
        "varn",
        "vars",
        "spotanim",
        "idk",
        "category",
        #[cfg(before_647)]
        "interface",
        #[cfg(since_254)]
        "varpbit",
        #[cfg(all(since_274, before_647))]
        "mes",
        #[cfg(since_647)]
        "flu",
        #[cfg(since_647)]
        "bas",
        #[cfg(since_647)]
        "cursor",
        #[cfg(since_647)]
        "hitmark",
        #[cfg(since_647)]
        "billboard",
        #[cfg(since_647)]
        "particleeffector",
        #[cfg(since_647)]
        "particleemitter",
        #[cfg(since_647)]
        "light",
        #[cfg(since_647)]
        "mel",
        #[cfg(since_647)]
        "msi",
        #[cfg(since_647)]
        "quest",
        #[cfg(since_647)]
        "quickchatcat",
        #[cfg(since_647)]
        "quickchatphrase",
        #[cfg(since_647)]
        "varc",
        #[cfg(since_647)]
        "varcstr",
        #[cfg(since_647)]
        "varpstr",
        #[cfg(since_647)]
        "varsstr",
        #[cfg(since_647)]
        "varclansetting",
        #[cfg(since_647)]
        "area",
        #[cfg(since_647)]
        "varobj",
        #[cfg(since_647)]
        "varcon",
        #[cfg(since_647)]
        "varconbit",
        #[cfg(since_647)]
        "controller",
    ];
    let results: HashMap<String, PackedFile> = kinds
        .par_iter()
        .enumerate()
        .map(|(id, &kind)| {
            let start = Instant::now();
            let mut sources = files.with_ext(&config_inputs(kind));
            sources.extend(
                read_registries(kind)
                    .iter()
                    .map(|name| registry.pack_dir().join(name)),
            );
            let file = stage.unit_with(registry, id as u32, &sources, |registry| {
                if let Some(own) = own_registry(kind) {
                    registry.depend_on(own);
                }
                let fc = files.read();
                let constants = constants();
                let param_types = param_types()?;
                Ok(match kind {
                    "param" => param::pack_params(fc, registry, constants)?,
                    "dbtable" => dbtable::pack_dbtables(fc, registry, constants)?,
                    "dbrow" => dbrow::pack_dbrows(fc, registry, constants)?,
                    "enum" => r#enum::pack_enums(fc, registry, constants)?,
                    "flo" => flo::pack_flos(fc, registry, constants)?,
                    "inv" => inv::pack_invs(fc, registry, constants)?,
                    "mesanim" => mesanim::pack_mesanims(fc, registry, constants)?,
                    "struct" => r#struct::pack_structs(fc, registry, constants, param_types)?,
                    "seq" => seq::pack_seqs(fc, registry, constants)?,
                    "loc" => loc::pack_locs(fc, registry, constants, param_types)?,
                    "npc" => npc::pack_npcs(fc, registry, constants, param_types)?,
                    "obj" => obj::pack_objs(fc, registry, constants, param_types)?,
                    "varp" => varp::pack_varps(fc, registry, constants)?,
                    "hunt" => hunt::pack_hunts(fc, registry, constants)?,
                    "varn" => vars::pack_vars(fc, &registry.varn, kind, constants)?,
                    "vars" => vars::pack_vars(fc, &registry.vars, kind, constants)?,
                    "spotanim" => spotanim::pack_spotanims(fc, registry, constants)?,
                    "idk" => idk::pack_idks(fc, registry, constants)?,
                    "category" => PackedFile {
                        server: category::pack_categories(registry)?,
                        client: None,
                    },
                    #[cfg(before_647)]
                    "interface" => interface::pack_interfaces(files.dir(), registry, verify)?,
                    #[cfg(since_254)]
                    "varpbit" => varbit::pack_varbits(fc, registry, constants)?,
                    #[cfg(all(since_274, before_647))]
                    "mes" => mes::pack_mes(fc, constants)?,
                    #[cfg(since_647)]
                    "flu" => flu::pack_flus(fc, registry, constants)?,
                    #[cfg(since_647)]
                    "bas" => bas::pack_bases(fc, registry, constants)?,
                    #[cfg(since_647)]
                    "cursor" => cursor::pack_cursors(fc, registry, constants)?,
                    #[cfg(since_647)]
                    "hitmark" => hitmark::pack_hitmarks(fc, registry, constants)?,
                    #[cfg(since_647)]
                    "billboard" => billboard::pack_billboards(fc, registry, constants)?,
                    #[cfg(since_647)]
                    "particleeffector" => {
                        particleeffector::pack_particleeffectors(fc, registry, constants)?
                    }
                    #[cfg(since_647)]
                    "particleemitter" => {
                        particleemitter::pack_particleemitters(fc, registry, constants)?
                    }
                    #[cfg(since_647)]
                    "light" => light::pack_lights(fc, registry, constants)?,
                    #[cfg(since_647)]
                    "mel" => mel::pack_mels(fc, registry, constants, param_types)?,
                    #[cfg(since_647)]
                    "msi" => msi::pack_msis(fc, registry, constants)?,
                    #[cfg(since_647)]
                    "quest" => quest::pack_quests(fc, registry, constants, param_types)?,
                    #[cfg(since_647)]
                    "quickchatcat" => quickchatcat::pack_quickchatcats(fc, registry, constants)?,
                    #[cfg(since_647)]
                    "quickchatphrase" => {
                        quickchatphrase::pack_quickchatphrases(fc, registry, constants)?
                    }
                    #[cfg(since_647)]
                    "varc" => varc::pack_varcs(fc, &registry.varc, constants)?,
                    #[cfg(since_647)]
                    "varcstr" => varc::pack_varc_strings(fc, &registry.varcstr, constants)?,
                    #[cfg(since_647)]
                    "varpstr" => vars::pack_string_vars(fc, &registry.varpstr, "varp", constants)?,
                    #[cfg(since_647)]
                    "varsstr" => vars::pack_string_vars(fc, &registry.varsstr, "vars", constants)?,
                    #[cfg(since_647)]
                    "varclansetting" => varclansetting::pack_varclansettings(
                        fc,
                        &registry.varclansetting,
                        constants,
                    )?,
                    #[cfg(since_647)]
                    "area" => area::pack_areas(fc, registry, constants)?,
                    #[cfg(since_647)]
                    "varobj" => varobj::pack_varobjs(fc, registry, constants)?,
                    #[cfg(since_647)]
                    "varcon" => PackedFile {
                        server: varcon::pack_varcons(fc, registry, constants)?,
                        client: None,
                    },
                    #[cfg(since_647)]
                    "varconbit" => PackedFile {
                        server: varconbit::pack_varconbits(fc, registry, constants)?,
                        client: None,
                    },
                    #[cfg(since_647)]
                    "controller" => PackedFile {
                        server: controller::pack_controllers(fc, registry, constants)?,
                        client: None,
                    },
                    _ => unreachable!(),
                })
            })?;
            debug!(
                "Packed {kind} configs in {:.2}s",
                start.elapsed().as_secs_f64()
            );
            Ok((kind.to_owned(), file))
        })
        .collect::<anyhow::Result<_>>()?;
    stage.finish(Vec::new());
    if verify {
        for &(name, expected) in CONFIG_TYPE_CRCS {
            let client = results[name]
                .client
                .as_ref()
                .expect("Missing client config");
            verify_crc(name, &client.dat, expected);
        }
    }
    Ok(results)
}

pub type ConfigProperties = Vec<(String, String)>;
pub type ConfigSections = HashMap<String, ConfigProperties>;

pub struct FileCache {
    by_ext: HashMap<String, Vec<PathBuf>>,
    contents: HashMap<PathBuf, String>,
}

impl FileCache {
    pub(crate) const TEXT_EXTS: &[&str] = &[
        "param",
        "obj",
        "npc",
        "loc",
        "seq",
        "flo",
        #[cfg(since_647)]
        "flu",
        "inv",
        "enum",
        "hunt",
        "idk",
        "mesanim",
        "spotanim",
        "varp",
        "varn",
        "vars",
        "struct",
        "dbtable",
        "dbrow",
        "constant",
        "if",
        #[cfg(since_254)]
        "varpbit",
        #[cfg(since_647)]
        "bas",
        #[cfg(since_647)]
        "cursor",
        #[cfg(since_647)]
        "hitmark",
        #[cfg(since_647)]
        "billboard",
        #[cfg(since_647)]
        "particleeffector",
        #[cfg(since_647)]
        "particleemitter",
        #[cfg(since_647)]
        "light",
        #[cfg(since_647)]
        "mel",
        #[cfg(since_647)]
        "msi",
        #[cfg(since_647)]
        "quest",
        #[cfg(since_647)]
        "quickchatcat",
        #[cfg(since_647)]
        "quickchatphrase",
        #[cfg(since_647)]
        "varc",
        #[cfg(since_647)]
        "varclansetting",
        #[cfg(since_647)]
        "area",
        #[cfg(since_647)]
        "varobj",
        #[cfg(since_647)]
        "controller",
        #[cfg(since_647)]
        "varcon",
        #[cfg(since_647)]
        "varconbit",
        #[cfg(since_647)]
        "varnbit",
        #[cfg(since_647)]
        "varclan",
        #[cfg(since_647)]
        "itemcode",
        #[cfg(since_647)]
        "config37",
        #[cfg(since_647)]
        "config38",
        #[cfg(since_647)]
        "config39",
        #[cfg(since_647)]
        "config45",
        #[cfg(since_647)]
        "wma",
    ];

    pub fn new(source_dir: &Path) -> Self {
        Self::from_files(source_files(source_dir))
    }

    fn from_files(files: Vec<PathBuf>) -> Self {
        let mut by_ext: HashMap<String, Vec<PathBuf>> = HashMap::new();
        let mut contents = HashMap::new();

        for path in files {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if Self::TEXT_EXTS.contains(&ext) {
                    if let Ok(text) = fs::read_to_string(&path) {
                        contents.insert(path.clone(), text);
                    }
                }
                by_ext.entry(ext.to_string()).or_default().push(path);
            }
        }
        Self { by_ext, contents }
    }

    pub fn collect(&self, ext: &str) -> &[PathBuf] {
        self.by_ext.get(ext).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn read(&self, path: &Path) -> Option<&str> {
        self.contents.get(path).map(|s| s.as_str())
    }
}

/// The config sources under a directory: listed up front, read only when a
/// config has to be packed.
pub struct ConfigFiles {
    dir: PathBuf,
    files: Vec<PathBuf>,
    cache: OnceLock<FileCache>,
}

impl ConfigFiles {
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            files: source_files(dir),
            cache: OnceLock::new(),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn read(&self) -> &FileCache {
        self.cache
            .get_or_init(|| FileCache::from_files(self.files.clone()))
    }

    pub fn with_ext(&self, exts: &[&str]) -> Vec<PathBuf> {
        self.files
            .iter()
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| exts.contains(&ext))
            })
            .cloned()
            .collect()
    }
}

/// Every file under a config source directory's roots, in order.
fn source_files(source_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in runec::sources::roots(source_dir) {
        collect_all_recursive(&root, &mut files);
    }
    files.sort();
    files
}

pub fn collect_config_files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in runec::sources::roots(dir) {
        collect_all_recursive(&root, &mut files);
    }
    files.sort();
    files
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(ext))
        .collect()
}

fn collect_all_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            collect_all_recursive(&path, out);
        } else {
            out.push(path);
        }
    }
}

pub fn collect_if_files(dir: &Path) -> Vec<(String, Vec<String>)> {
    let mut results = Vec::new();
    let files = collect_config_files(dir, "if");
    for path in files {
        let if_name = path.file_stem().unwrap().to_string_lossy().to_string();
        let content = fs::read_to_string(&path).unwrap_or_default();
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        results.push((if_name, lines));
    }
    results
}

pub fn load_constants(source_dir: &Path) -> HashMap<String, String> {
    let mut constants = HashMap::new();
    let files = collect_config_files(source_dir, "constant");
    for path in &files {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        parse_constants_text(&text, &mut constants);
    }
    constants
}

pub fn load_constants_from_cache(fc: &FileCache) -> HashMap<String, String> {
    let mut constants = HashMap::new();
    for path in fc.collect("constant") {
        if let Some(text) = fc.read(path) {
            parse_constants_text(text, &mut constants);
        }
    }
    constants
}

fn parse_constants_text(text: &str, constants: &mut HashMap<String, String>) {
    for line in text.lines() {
        let line = line.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some((name, value)) = line.split_once('=') {
            let name = name.trim().strip_prefix('^').unwrap_or(name.trim());
            constants.insert(name.to_string(), value.trim().to_string());
        }
    }
}

pub fn parse_config_sections(
    files: &[PathBuf],
    constants: &HashMap<String, String>,
) -> Result<ConfigSections> {
    let contents = files
        .iter()
        .map(|path| fs::read_to_string(path).with_context(|| path.display().to_string()))
        .collect::<Result<Vec<_>>>()?;
    parse_sections(
        files
            .iter()
            .zip(&contents)
            .map(|(path, text)| (path.as_path(), text.as_str())),
        constants,
    )
}

pub fn parse_config_sections_cached(
    fc: &FileCache,
    ext: &str,
    constants: &HashMap<String, String>,
) -> Result<ConfigSections> {
    let configs = parse_sections(
        fc.collect(ext)
            .iter()
            .filter_map(|path| fc.read(path).map(|text| (path.as_path(), text))),
        constants,
    )?;
    debug!(
        "  {ext}: {} files, {} configs",
        fc.collect(ext).len(),
        configs.len()
    );
    Ok(configs)
}

pub(super) fn parse_sections<'a>(
    files: impl IntoIterator<Item = (&'a Path, &'a str)>,
    constants: &HashMap<String, String>,
) -> Result<ConfigSections> {
    let mut configs = ConfigSections::new();
    let mut sources = HashMap::new();

    for (path, text) in files {
        let mut current_name: Option<String> = None;

        for (line_no, raw_line) in text.lines().enumerate() {
            #[cfg(before_647)]
            let line = raw_line.split("//").next().unwrap_or("").trim();
            #[cfg(since_647)]
            let line = {
                let line = raw_line.trim_start().trim_end_matches('\r');
                find_comment_start(line).map_or(line, |index| &line[..index])
            };
            let header = line.trim_end();
            if header.is_empty() {
                continue;
            }
            if let Some(name) = header.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                if let Some((previous, previous_line)) = sources.insert(name, (path, line_no + 1)) {
                    bail!(
                        "{}:{}: Duplicate config [{name}]; first defined at {}:{previous_line}",
                        path.display(),
                        line_no + 1,
                        previous.display()
                    );
                }
                configs.insert(name.to_string(), Vec::new());
                current_name = Some(name.to_string());
                continue;
            }
            if let Some((key, value)) = line.split_once('=')
                && let Some(name) = &current_name
            {
                configs.get_mut(name).unwrap().push((
                    key.trim().to_string(),
                    substitute_constants(value, constants),
                ));
            }
        }
    }
    Ok(configs)
}

#[cfg(since_647)]
fn find_comment_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'/' {
            if i == 0 || bytes[i - 1].is_ascii_whitespace() {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn substitute_constants(value: &str, constants: &HashMap<String, String>) -> String {
    if !value.contains('^') {
        return value.to_string();
    }
    let mut result = value.to_string();
    while let Some(start) = result.find('^') {
        let rest = &result[start + 1..];
        let end = rest
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        let name = &rest[..end];
        if let Some(replacement) = constants.get(name) {
            result = format!("{}{}{}", &result[..start], replacement, &rest[end..]);
        } else {
            break;
        }
    }
    result
}

pub fn pack_configs(
    source_dir: &Path,
    pack_dir: &Path,
    verify: bool,
) -> Result<HashMap<String, PackedFile>> {
    let files = ConfigFiles::new(source_dir);
    check_configs(&files, pack_dir, verify)?;
    pack_config_files(&files, pack_dir, verify)
}

/// Checks config names against the existing registries.
pub fn check_configs(files: &ConfigFiles, pack_dir: &Path, verify: bool) -> Result<()> {
    let check = Stage::new(
        "config names",
        vec![
            ("pack".to_owned(), registry_key(pack_dir)),
            ("verify".to_owned(), Key::from(verify)),
        ],
    );
    check.unit(0, &files.with_ext(FileCache::TEXT_EXTS), || {
        validate_config_packs(files.read(), pack_dir)?;
        PackRegistry::load(pack_dir)?.check_content(files.read())?;
        Ok(Vec::new())
    })?;
    check.finish(vec![("pack".to_owned(), registry_key(pack_dir))]);
    Ok(())
}

/// [`pack_configs`] once [`check_configs`] has run.
pub fn pack_config_files(
    files: &ConfigFiles,
    pack_dir: &Path,
    verify: bool,
) -> Result<HashMap<String, PackedFile>> {
    let registry = PackRegistry::load(pack_dir)?;
    let assets = pack_cached_assets(&registry, files, verify)?;
    #[cfg(since_647)]
    let assets = {
        let mut assets = assets;
        let empty = archive::CONFIG_EMPTY_TABLES
            .par_iter()
            .map(|&(name, _, _, expected)| {
                if files.with_ext(&[name]).is_empty() && !verify {
                    return Ok(None);
                }
                let pack = registry
                    .file(name)
                    .with_context(|| format!("No pack registry for .{name} configs"))?;
                let client = pack_empty_table(files.dir(), name, pack)?;
                if verify {
                    verify_crc(name, &client.dat, expected);
                }
                Ok(Some((name.to_owned(), client)))
            })
            .collect::<Result<Vec<_>>>()?;
        for (name, client) in empty.into_iter().flatten() {
            assets
                .entry(name)
                .or_insert_with(|| PackedFile {
                    server: Default::default(),
                    client: None,
                })
                .client = Some(client);
        }
        assets
    };
    Ok(assets)
}

#[cfg(since_647)]
pub fn pack_cache(
    content_dir: &Path,
    pack_dir: &Path,
    verify: bool,
    files: &ConfigFiles,
    names: &AssetNames,
) -> Result<(HashMap<String, PackedFile>, PackedCache)> {
    let versions = archive::load_versions_dir(&content_dir.join("versions"))?;
    let stages = [
        "animations",
        "shaders",
        "interfaces",
        "binary",
        "synth",
        "songs",
        "textures",
        "sprites",
        "maps",
    ];
    let (configs, archives) = join(
        || -> Result<_> {
            let start = Instant::now();
            let assets = pack_config_files(files, pack_dir, verify)?;
            let mut cache = PackedCache::default();
            config::pack_archives(&mut cache, &assets, &versions)?;
            debug!("Packed configs in {:.2}s", start.elapsed().as_secs_f64());
            Ok((assets, cache))
        },
        || -> Result<_> {
            stages
                .par_iter()
                .map(|&stage| {
                    let start = Instant::now();
                    let mut cache = PackedCache::default();
                    match stage {
                        "animations" => {
                            model::pack_archive(&mut cache, content_dir, &versions, names)?
                        }
                        "interfaces" => interface::pack_archive(
                            &mut cache,
                            content_dir,
                            pack_dir,
                            &versions,
                            names,
                        )?,
                        "binary" => js5::pack_asset_archives(
                            &mut cache,
                            content_dir,
                            pack_dir,
                            &versions,
                            names,
                            verify,
                        )?,
                        "synth" => sound::pack_archive(&mut cache, content_dir, &versions, names)?,
                        "shaders" => shader::pack_archive(
                            &mut cache,
                            content_dir,
                            pack_dir,
                            &versions,
                            names,
                        )?,
                        "songs" => {
                            song::pack_archive(&mut cache, content_dir, pack_dir, &versions, names)?
                        }
                        "textures" => {
                            texture::pack_archive(&mut cache, content_dir, &versions, names)?
                        }
                        "sprites" => media::pack_archive(
                            &mut cache,
                            content_dir,
                            pack_dir,
                            &versions,
                            names,
                        )?,
                        "maps" => {
                            map::pack_archive(&mut cache, content_dir, pack_dir, &versions, names)?
                        }
                        _ => unreachable!(),
                    }
                    debug!("Packed {stage} in {:.2}s", start.elapsed().as_secs_f64());
                    Ok(cache)
                })
                .try_reduce(PackedCache::default, |mut a, b| {
                    a.extend(b);
                    Ok(a)
                })
        },
    );
    let (assets, mut cache) = configs?;
    cache.extend(archives?);
    Ok((assets, cache))
}
