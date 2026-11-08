use crate::CONFIG_TYPE_CRCS;
#[cfg(since_647)]
use crate::archive;
#[cfg(since_647)]
use crate::cache::js5::PackedCache;
use crate::pack::config::*;
#[cfg(before_647)]
use crate::pack::interface;
#[cfg(since_647)]
use crate::pack::other::{map, song};
#[cfg(since_647)]
use crate::pack::pack_registry::{PackFile, PackId, validate_config_packs};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
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
use std::time::Instant;
use tracing::debug;

pub fn pack_assets(
    registry: &PackRegistry,
    source_dir: &Path,
    verify: bool,
) -> anyhow::Result<HashMap<String, PackedFile>> {
    let fc = FileCache::new(source_dir);
    pack_cached_assets(registry, source_dir, &fc, verify)
}

fn pack_cached_assets(
    registry: &PackRegistry,
    _source_dir: &Path,
    fc: &FileCache,
    verify: bool,
) -> anyhow::Result<HashMap<String, PackedFile>> {
    let constants = load_constants_from_cache(fc);
    debug!("  Loaded {} constants", constants.len());

    let param_types = param::load_param_types(fc, &constants)?;
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
        "varbit",
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
        "varcs",
        #[cfg(since_647)]
        "varclansetting",
        #[cfg(since_647)]
        "area",
    ];
    let results: HashMap<String, PackedFile> = kinds
        .par_iter()
        .map(|&kind| {
            let start = Instant::now();
            let file = match kind {
                "param" => param::pack_params(fc, registry, &constants)?,
                "dbtable" => dbtable::pack_dbtables(fc, registry, &constants)?,
                "dbrow" => dbrow::pack_dbrows(fc, registry, &constants)?,
                "enum" => r#enum::pack_enums(fc, registry, &constants)?,
                "flo" => flo::pack_flos(fc, registry, &constants)?,
                "inv" => inv::pack_invs(fc, registry, &constants)?,
                "mesanim" => mesanim::pack_mesanims(fc, registry, &constants)?,
                "struct" => r#struct::pack_structs(fc, registry, &constants, &param_types)?,
                "seq" => seq::pack_seqs(fc, registry, &constants)?,
                "loc" => loc::pack_locs(fc, registry, &constants, &param_types)?,
                "npc" => npc::pack_npcs(fc, registry, &constants, &param_types)?,
                "obj" => obj::pack_objs(fc, registry, &constants, &param_types)?,
                "varp" => varp::pack_varps(fc, registry, &constants)?,
                "hunt" => hunt::pack_hunts(fc, registry, &constants)?,
                "varn" => vars::pack_vars(fc, &registry.varn, kind, &constants)?,
                "vars" => vars::pack_vars(fc, &registry.vars, kind, &constants)?,
                "spotanim" => spotanim::pack_spotanims(fc, registry, &constants)?,
                "idk" => idk::pack_idks(fc, registry, &constants)?,
                "category" => PackedFile {
                    server: category::pack_categories(registry)?,
                    client: None,
                },
                #[cfg(before_647)]
                "interface" => interface::pack_interfaces(_source_dir, registry, verify)?,
                #[cfg(since_254)]
                "varbit" => varbit::pack_varbits(fc, registry, &constants)?,
                #[cfg(all(since_274, before_647))]
                "mes" => mes::pack_mes(fc, &constants)?,
                #[cfg(since_647)]
                "flu" => flu::pack_flus(fc, registry, &constants)?,
                #[cfg(since_647)]
                "bas" => bas::pack_bases(fc, registry, &constants)?,
                #[cfg(since_647)]
                "cursor" => cursor::pack_cursors(fc, registry, &constants)?,
                #[cfg(since_647)]
                "hitmark" => hitmark::pack_hitmarks(fc, registry, &constants)?,
                #[cfg(since_647)]
                "billboard" => billboard::pack_billboards(fc, registry, &constants)?,
                #[cfg(since_647)]
                "particleeffector" => {
                    particleeffector::pack_particleeffectors(fc, registry, &constants)?
                }
                #[cfg(since_647)]
                "particleemitter" => {
                    particleemitter::pack_particleemitters(fc, registry, &constants)?
                }
                #[cfg(since_647)]
                "light" => light::pack_lights(fc, registry, &constants)?,
                #[cfg(since_647)]
                "mel" => mel::pack_mels(fc, registry, &constants, &param_types)?,
                #[cfg(since_647)]
                "msi" => msi::pack_msis(fc, registry, &constants)?,
                #[cfg(since_647)]
                "quest" => quest::pack_quests(fc, registry, &constants, &param_types)?,
                #[cfg(since_647)]
                "quickchatcat" => quickchatcat::pack_quickchatcats(fc, registry, &constants)?,
                #[cfg(since_647)]
                "quickchatphrase" => {
                    quickchatphrase::pack_quickchatphrases(fc, registry, &constants)?
                }
                #[cfg(since_647)]
                "varc" => varc::pack_varcs(fc, &registry.varc, &constants)?,
                #[cfg(since_647)]
                "varcs" => varc::pack_varc_strings(fc, &registry.varcs, &constants)?,
                #[cfg(since_647)]
                "varclansetting" => {
                    varclansetting::pack_varclansettings(fc, &registry.varclansetting, &constants)?
                }
                #[cfg(since_647)]
                "area" => area::pack_areas(fc, registry, &constants)?,
                _ => unreachable!(),
            };
            debug!(
                "Packed {kind} configs in {:.2}s",
                start.elapsed().as_secs_f64()
            );
            Ok((kind.to_owned(), file))
        })
        .collect::<anyhow::Result<_>>()?;
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
        "varbit",
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
    ];
    pub fn new(source_dir: &Path) -> Self {
        let mut files = Vec::new();
        for root in runec::sources::roots(source_dir) {
            collect_all_recursive(&root, &mut files);
        }
        files.sort();

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

    #[cfg(since_647)]
    pub fn client_configs(
        &self,
        ext: &str,
        constants: &HashMap<String, String>,
    ) -> Result<ConfigSections> {
        parse_sections(
            self.collect(ext)
                .iter()
                .filter(|path| is_unpacked(path))
                .filter_map(|path| self.read(path).map(|text| (path.as_path(), text))),
            constants,
        )
    }

    #[cfg(since_647)]
    pub fn client_count(
        &self,
        ext: &str,
        pack: &PackFile,
        constants: &HashMap<String, String>,
    ) -> Result<Option<PackId>> {
        Ok(self
            .client_configs(ext, constants)?
            .keys()
            .filter_map(|name| pack.get_by_debugname(name))
            .max()
            .map(|id| id + 1))
    }

    pub fn collect(&self, ext: &str) -> &[PathBuf] {
        self.by_ext.get(ext).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn read(&self, path: &Path) -> Option<&str> {
        self.contents.get(path).map(|s| s.as_str())
    }
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

pub fn is_unpacked(path: &Path) -> bool {
    path.components().any(|part| part.as_os_str() == "_unpack")
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

#[cfg(since_647)]
pub fn pack_configs(
    source_dir: &Path,
    pack_dir: &Path,
    verify: bool,
) -> Result<HashMap<String, PackedFile>> {
    let registry = PackRegistry::load(pack_dir)?;
    let fc = FileCache::new(source_dir);
    registry.check_content(&fc)?;
    validate_config_packs(&fc, pack_dir)?;
    let registry = PackRegistry::load(pack_dir)?;
    let mut assets = pack_cached_assets(&registry, source_dir, &fc, verify)?;
    let empty = archive::CONFIG_EMPTY_TABLES
        .par_iter()
        .map(|&(name, _, _, expected)| {
            if fc.collect(name).is_empty() && !verify {
                return Ok(None);
            }
            let client = pack_empty_table(source_dir, name)?;
            if verify {
                verify_crc(name, &client.dat, expected);
            }
            Ok(Some((
                name.to_owned(),
                PackedFile {
                    server: Default::default(),
                    client: Some(client),
                },
            )))
        })
        .collect::<Result<Vec<_>>>()?;
    assets.extend(empty.into_iter().flatten());
    Ok(assets)
}

#[cfg(since_647)]
pub fn pack_cache(
    content_dir: &Path,
    pack_dir: &Path,
    verify: bool,
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
            let assets = pack_configs(&content_dir.join("scripts"), pack_dir, verify)?;
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
                            model::pack_archive(&mut cache, content_dir, pack_dir, &versions)?
                        }
                        "interfaces" => {
                            interface::pack_archive(&mut cache, content_dir, pack_dir, &versions)?
                        }
                        "binary" => {
                            js5::pack_asset_archives(&mut cache, content_dir, pack_dir, &versions)?
                        }
                        "synth" => {
                            sound::pack_archive(&mut cache, content_dir, pack_dir, &versions)?
                        }
                        "shaders" => crate::pack::other::shader::pack_archive(
                            &mut cache,
                            content_dir,
                            pack_dir,
                            &versions,
                        )?,
                        "songs" => {
                            song::pack_archive(&mut cache, content_dir, pack_dir, &versions)?
                        }
                        "textures" => {
                            texture::pack_archive(&mut cache, content_dir, pack_dir, &versions)?
                        }
                        "sprites" => {
                            media::pack_archive(&mut cache, content_dir, pack_dir, &versions)?
                        }
                        "maps" => map::pack_archive(&mut cache, content_dir, pack_dir, &versions)?,
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
