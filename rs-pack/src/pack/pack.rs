use crate::pack::config::*;
use crate::pack::interface;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use std::collections::HashMap;
use std::path::Path;
use tracing::debug;

pub fn pack_assets(
    registry: &PackRegistry,
    source_dir: &Path,
    verify: bool,
) -> anyhow::Result<HashMap<String, PackedFile>> {
    let fc = FileCache::new(source_dir);

    let constants = load_constants_from_cache(&fc);
    debug!("  Loaded {} constants", constants.len());

    let param_types = param::load_param_types(&fc);
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
        flo::pack_flos(&fc, registry, &constants, verify)?,
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
        seq::pack_seqs(&fc, registry, &constants, verify)?,
    );
    debug!("Packing loc configs...");
    results.insert(
        "loc".to_string(),
        loc::pack_locs(&fc, registry, &constants, &param_types, verify)?,
    );
    debug!("Packing npc configs...");
    results.insert(
        "npc".to_string(),
        npc::pack_npcs(&fc, registry, &constants, &param_types, verify)?,
    );
    debug!("Packing obj configs...");
    results.insert(
        "obj".to_string(),
        obj::pack_objs(&fc, registry, &constants, &param_types, verify)?,
    );
    debug!("Packing varp configs...");
    results.insert(
        "varp".to_string(),
        varp::pack_varps(&fc, registry, &constants, verify)?,
    );
    debug!("Packing hunt configs...");
    results.insert(
        "hunt".to_string(),
        hunt::pack_hunts(&fc, registry, &constants)?,
    );
    debug!("Packing varn configs...");
    results.insert(
        "varn".to_string(),
        varn::pack_varns(&fc, registry, &constants)?,
    );
    debug!("Packing vars configs...");
    results.insert(
        "vars".to_string(),
        vars::pack_vars_configs(&fc, registry, &constants)?,
    );
    debug!("Packing interface configs...");
    results.insert(
        "interface".to_string(),
        interface::pack_interfaces(source_dir, registry, verify)?,
    );
    debug!("Packing spotanim configs...");
    results.insert(
        "spotanim".to_string(),
        spotanim::pack_spotanims(&fc, registry, &constants, verify)?,
    );
    debug!("Packing idk configs...");
    results.insert(
        "idk".to_string(),
        idk::pack_idks(&fc, registry, &constants, verify)?,
    );
    debug!("Packing categories...");
    results.insert("category".to_string(), category::pack_categories(registry)?);
    #[cfg(since_254)]
    debug!("Packing varbit configs...");
    #[cfg(since_254)]
    results.insert(
        "varbit".to_string(),
        varbit::pack_varbits(&fc, registry, &constants, verify)?,
    );
    Ok(results)
}

pub struct FileCache {
    by_ext: HashMap<String, Vec<std::path::PathBuf>>,
    contents: HashMap<std::path::PathBuf, String>,
}

impl FileCache {
    const TEXT_EXTS: &[&str] = &[
        "param",
        "obj",
        "npc",
        "loc",
        "seq",
        "flo",
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

        let mut by_ext: HashMap<String, Vec<std::path::PathBuf>> = HashMap::new();
        let mut contents = HashMap::new();

        for path in files {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if Self::TEXT_EXTS.contains(&ext) {
                    if let Ok(text) = std::fs::read_to_string(&path) {
                        contents.insert(path.clone(), text);
                    }
                }
                by_ext.entry(ext.to_string()).or_default().push(path);
            }
        }
        Self { by_ext, contents }
    }

    pub fn collect(&self, ext: &str) -> &[std::path::PathBuf] {
        self.by_ext.get(ext).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn read(&self, path: &Path) -> Option<&str> {
        self.contents.get(path).map(|s| s.as_str())
    }
}

pub fn collect_config_files(dir: &Path, ext: &str) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    collect_all_recursive(dir, &mut files);
    files.sort();
    files
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(ext))
        .collect()
}

fn collect_all_recursive(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut sorted: Vec<std::path::PathBuf> = entries.flatten().map(|e| e.path()).collect();
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
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        results.push((if_name, lines));
    }
    results
}

pub fn load_constants(source_dir: &Path) -> HashMap<String, String> {
    let mut constants = HashMap::new();
    let files = collect_config_files(source_dir, "constant");
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        parse_constants_text(&text, &mut constants);
    }
    constants
}

fn load_constants_from_cache(fc: &FileCache) -> HashMap<String, String> {
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
    files: &[std::path::PathBuf],
    constants: &HashMap<String, String>,
) -> HashMap<String, Vec<(String, String)>> {
    let mut configs: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for path in files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        parse_config_text(&text, constants, &mut configs);
    }

    configs
}

pub fn parse_config_sections_cached(
    fc: &FileCache,
    ext: &str,
    constants: &HashMap<String, String>,
) -> HashMap<String, Vec<(String, String)>> {
    let mut configs: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for path in fc.collect(ext) {
        if let Some(text) = fc.read(path) {
            parse_config_text(text, constants, &mut configs);
        }
    }

    configs
}

fn parse_config_text(
    text: &str,
    constants: &HashMap<String, String>,
    configs: &mut HashMap<String, Vec<(String, String)>>,
) {
    let mut current_name: Option<String> = None;

    for raw_line in text.lines() {
        let line = raw_line.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            current_name = Some(name.to_string());
            configs.entry(name.to_string()).or_default();
            continue;
        }

        if let Some((key, raw_value)) = line.split_once('=')
            && let Some(ref name) = current_name
        {
            let value = substitute_constants(raw_value, constants);
            configs
                .get_mut(name)
                .unwrap()
                .push((key.trim().to_string(), value));
        }
    }
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
