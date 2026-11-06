use crate::CONFIG_TYPE_CRCS;
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
#[cfg(since_647)]
use crate::startup::{cache_build_inputs, file_mtime, is_up_to_date, max_of, write_stamp};
#[cfg(since_647)]
use crate::{archive, config_crc};
#[cfg(since_647)]
use anyhow::Result;
#[cfg(since_647)]
use rs_io::jag::{JagCompression, JagFile};
#[cfg(since_647)]
use rs_io::js5::Js5Store;
#[cfg(since_647)]
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;
#[cfg(since_647)]
use tracing::{info, warn};

pub fn pack_assets(
    registry: &PackRegistry,
    source_dir: &Path,
    verify: bool,
) -> anyhow::Result<HashMap<String, PackedFile>> {
    let fc = FileCache::new(source_dir);

    let constants = load_constants_from_cache(&fc);
    debug!("  Loaded {} constants", constants.len());

    let param_types = param::load_param_types(&fc, &constants);
    let mut results = HashMap::new();

    debug!("Packing param configs...");
    results.insert(
        "param".to_string(),
        param::pack_params(&fc, registry, &constants)?,
    );
    debug!("Packing dbtable configs...");
    results.insert(
        "dbtable".to_string(),
        dbtable::pack_dbtables(&fc, registry, &constants)?,
    );
    debug!("Packing dbrow configs...");
    results.insert(
        "dbrow".to_string(),
        dbrow::pack_dbrows(&fc, registry, &constants)?,
    );
    debug!("Packing enum configs...");
    results.insert(
        "enum".to_string(),
        r#enum::pack_enums(&fc, registry, &constants)?,
    );
    debug!("Packing flo configs...");
    results.insert(
        "flo".to_string(),
        flo::pack_flos(&fc, registry, &constants)?,
    );
    debug!("Packing inv configs...");
    results.insert(
        "inv".to_string(),
        inv::pack_invs(&fc, registry, &constants)?,
    );
    debug!("Packing mesanim configs...");
    results.insert(
        "mesanim".to_string(),
        mesanim::pack_mesanims(&fc, registry, &constants)?,
    );
    debug!("Packing struct configs...");
    results.insert(
        "struct".to_string(),
        r#struct::pack_structs(&fc, registry, &constants, &param_types)?,
    );
    debug!("Packing seq configs...");
    results.insert(
        "seq".to_string(),
        seq::pack_seqs(&fc, registry, &constants)?,
    );
    debug!("Packing loc configs...");
    results.insert(
        "loc".to_string(),
        loc::pack_locs(&fc, registry, &constants, &param_types)?,
    );
    debug!("Packing npc configs...");
    results.insert(
        "npc".to_string(),
        npc::pack_npcs(&fc, registry, &constants, &param_types)?,
    );
    debug!("Packing obj configs...");
    results.insert(
        "obj".to_string(),
        obj::pack_objs(&fc, registry, &constants, &param_types)?,
    );
    debug!("Packing varp configs...");
    results.insert(
        "varp".to_string(),
        varp::pack_varps(&fc, registry, &constants)?,
    );
    debug!("Packing hunt configs...");
    results.insert(
        "hunt".to_string(),
        hunt::pack_hunts(&fc, registry, &constants)?,
    );
    debug!("Packing varn configs...");
    results.insert(
        "varn".to_string(),
        vars::pack_vars(&fc, &registry.varn, "varn", &constants)?,
    );
    debug!("Packing vars configs...");
    results.insert(
        "vars".to_string(),
        vars::pack_vars(&fc, &registry.vars, "vars", &constants)?,
    );
    #[cfg(before_647)]
    {
        debug!("Packing interface configs...");
        results.insert(
            "interface".to_string(),
            interface::pack_interfaces(source_dir, registry, verify)?,
        );
    }
    debug!("Packing spotanim configs...");
    results.insert(
        "spotanim".to_string(),
        spotanim::pack_spotanims(&fc, registry, &constants)?,
    );
    debug!("Packing idk configs...");
    results.insert(
        "idk".to_string(),
        idk::pack_idks(&fc, registry, &constants)?,
    );
    debug!("Packing categories...");
    results.insert(
        "category".to_string(),
        PackedFile {
            server: category::pack_categories(registry)?,
            client: None,
        },
    );
    #[cfg(since_254)]
    {
        debug!("Packing varbit configs...");
        results.insert(
            "varbit".to_string(),
            varbit::pack_varbits(&fc, registry, &constants)?,
        );
    }
    #[cfg(all(since_274, before_647))]
    {
        debug!("Packing mes configs...");
        results.insert("mes".to_string(), mes::pack_mes(&fc, &constants)?);
    }
    #[cfg(since_647)]
    results.insert(
        "flu".to_string(),
        flu::pack_flus(&fc, registry, &constants)?,
    );
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
    ];
    pub fn new(source_dir: &Path) -> Self {
        let mut files = Vec::new();
        collect_all_recursive(source_dir, &mut files);
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
    pub fn client_configs(&self, ext: &str, constants: &HashMap<String, String>) -> ConfigSections {
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
    ) -> Option<PackId> {
        self.client_configs(ext, constants)
            .keys()
            .filter_map(|name| pack.get_by_debugname(name))
            .max()
            .map(|id| id + 1)
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
    collect_all_recursive(dir, &mut files);
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
    let mut sorted: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    sorted.sort();
    for path in sorted {
        if path.is_dir() {
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
) -> ConfigSections {
    let contents: Vec<_> = files.iter().map(fs::read_to_string).collect();
    parse_sections(
        files
            .iter()
            .zip(&contents)
            .filter_map(|(path, text)| text.as_deref().ok().map(|text| (path.as_path(), text))),
        constants,
    )
}

pub fn parse_config_sections_cached(
    fc: &FileCache,
    ext: &str,
    constants: &HashMap<String, String>,
) -> ConfigSections {
    let configs = parse_sections(
        fc.collect(ext)
            .iter()
            .filter_map(|path| fc.read(path).map(|text| (path.as_path(), text))),
        constants,
    );
    debug!(
        "  {ext}: {} files, {} configs",
        fc.collect(ext).len(),
        configs.len()
    );
    configs
}

pub(super) fn parse_sections<'a>(
    files: impl IntoIterator<Item = (&'a Path, &'a str)>,
    constants: &HashMap<String, String>,
) -> ConfigSections {
    let mut layers: [ConfigSections; 2] = Default::default();
    #[cfg(since_647)]
    let mut sources: [HashMap<String, PathBuf>; 2] = Default::default();

    for (_path, text) in files {
        #[cfg(before_647)]
        let layer = 0;
        #[cfg(since_647)]
        let layer = usize::from(!is_unpacked(_path));
        let mut current_name: Option<String> = None;
        #[cfg(since_647)]
        let mut seen = HashSet::new();

        #[cfg_attr(before_647, allow(clippy::unused_enumerate_index))]
        for (_line_no, raw_line) in text.lines().enumerate() {
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
                #[cfg(since_647)]
                {
                    if !seen.insert(name.to_string()) {
                        warn!(
                            "{}:{}: Duplicate config [{}] in same file",
                            _path.display(),
                            _line_no + 1,
                            name
                        );
                    }
                    if let Some(previous) = sources[layer].get(name) {
                        assert_eq!(
                            previous,
                            _path,
                            "Two authored files cannot define one section [{name}]: {} and {}:{}",
                            previous.display(),
                            _path.display(),
                            _line_no + 1
                        );
                    }
                    sources[layer].insert(name.to_string(), _path.to_path_buf());
                    layers[layer].insert(name.to_string(), Vec::new());
                }
                #[cfg(before_647)]
                layers[layer].entry(name.to_string()).or_default();
                current_name = Some(name.to_string());
                continue;
            }
            if let Some((key, value)) = line.split_once('=')
                && let Some(name) = &current_name
            {
                layers[layer].get_mut(name).unwrap().push((
                    key.trim().to_string(),
                    substitute_constants(value, constants),
                ));
            }
        }
    }
    let [mut configs, authored] = layers;
    for (name, overlay) in authored {
        let props = match configs.remove(&name) {
            Some(base) => overlay_section(base, overlay),
            None => overlay,
        };
        configs.insert(name, props);
    }
    configs
}

pub fn is_unpacked(path: &Path) -> bool {
    path.components().any(|part| part.as_os_str() == "_unpack")
}

pub fn overlay_section(
    base: Vec<(String, String)>,
    overlay: Vec<(String, String)>,
) -> Vec<(String, String)> {
    let named: HashSet<&str> = overlay.iter().map(|(k, _)| k.as_str()).collect();
    if base.iter().all(|(k, _)| named.contains(k.as_str())) {
        return overlay;
    }
    let mut out = Vec::with_capacity(base.len() + overlay.len());
    let mut placed: HashSet<String> = HashSet::new();
    for (key, value) in &base {
        if !named.contains(key.as_str()) {
            out.push((key.clone(), value.clone()));
        } else if placed.insert(key.clone()) {
            out.extend(overlay.iter().filter(|(k, _)| k == key).cloned());
        }
    }
    out.extend(overlay.iter().filter(|(k, _)| !placed.contains(k)).cloned());
    out
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
pub fn pack_all(
    source_dir: &Path,
    pack_dir: &Path,
    server_dir: &Path,
    client_dir: &Path,
    verify: bool,
) -> Result<()> {
    fs::create_dir_all(server_dir)?;
    fs::create_dir_all(client_dir)?;

    let mut config = JagFile::new();

    // Validate pack files: auto-register names from content files.
    validate_config_packs(source_dir, pack_dir)?;

    let registry = PackRegistry::load(pack_dir)?;
    let assets = pack_assets(&registry, source_dir, verify)?;
    for &(name, expected) in config_crc::GENERIC {
        let path = source_dir.join("_unpack").join(format!("all.{name}"));
        if !path.exists() && !verify {
            continue;
        }
        let client = generic::pack_generic_config(source_dir, name)?;
        if verify {
            verify_crc(name, &client.dat, expected);
        }
        config.write(&format!("{name}.dat"), client.dat);
        config.write(&format!("{name}.idx"), client.idx);
    }
    for (name, asset) in assets.into_iter().collect::<BTreeMap<_, _>>() {
        asset.server.save(server_dir, &name)?;
        if let Some(client) = asset.client {
            config.write(&format!("{name}.dat"), client.dat);
            config.write(&format!("{name}.idx"), client.idx);
        }
    }

    fs::write(
        client_dir.join("config"),
        config.build(JagCompression::PerFile),
    )?;

    info!("Pack complete.");
    Ok(())
}

#[cfg(since_647)]
pub fn pack_cache(
    source_cache: &Path,
    content_dir: &Path,
    output_dir: &Path,
    client_data_dir: &Path,
    force_rebuild: bool,
    prebuilt_anims: Option<BTreeMap<(u32, u32), Vec<u8>>>,
) -> Result<()> {
    let config_path = client_data_dir.join("config");
    let master_stamp = output_dir.join(".stamp_cache_built");
    let inputs = max_of(&[cache_build_inputs(content_dir), file_mtime(&config_path)]);
    if !force_rebuild
        && output_dir.join("main_file_cache.dat2").exists()
        && is_up_to_date(&master_stamp, inputs)
    {
        info!("JS5 cache up to date, skipping build");
        return Ok(());
    }

    let versions = archive::load_versions_dir(&content_dir.join("versions"))?;
    let keys_dst = output_dir.join("keys.json");
    let keys_src = content_dir.join("keys.json");
    if keys_src.exists() {
        fs::copy(keys_src, &keys_dst)?;
    } else if !keys_dst.exists() {
        warn!("No keys.json found for map encryption");
    }

    let mut store = Js5Store::create(output_dir, archive::ARCHIVE_IDS)?;
    let pack_dir = content_dir.join("pack");
    model::pack_archive(
        &mut store,
        content_dir,
        output_dir,
        &pack_dir,
        &versions,
        force_rebuild,
        prebuilt_anims,
    )?;
    config::pack_archives(
        &mut store,
        content_dir,
        output_dir,
        &config_path,
        &versions,
        force_rebuild,
    )?;
    interface::pack_archive(
        &mut store,
        content_dir,
        output_dir,
        &pack_dir,
        &versions,
        force_rebuild,
    )?;
    js5::pack_binary_archives(
        &mut store,
        content_dir,
        output_dir,
        &pack_dir,
        &versions,
        force_rebuild,
    )?;
    sound::pack_archive(
        &mut store,
        content_dir,
        output_dir,
        &versions,
        force_rebuild,
    )?;
    song::pack_archive(
        &mut store,
        content_dir,
        output_dir,
        &pack_dir,
        &versions,
        force_rebuild,
    )?;
    texture::pack_archive(
        &mut store,
        content_dir,
        output_dir,
        &versions,
        force_rebuild,
    )?;
    media::pack_archive(
        &mut store,
        content_dir,
        output_dir,
        &pack_dir,
        &versions,
        force_rebuild,
    )?;
    map::pack_archive(
        &mut store,
        content_dir,
        output_dir,
        &pack_dir,
        &versions,
        force_rebuild,
    )?;
    drop(store);
    js5::verify_cache(source_cache, output_dir)?;
    write_stamp(&master_stamp)?;
    info!("Cache built at {}", output_dir.display());
    Ok(())
}
