use crate::CONTENT_DIR;
use crate::archive;
use crate::cache::ScriptVarType;
use crate::cache::param::ParamTypeRaw;
use crate::cache::provider::CacheType;
use crate::pack::names::AssetNames;
use crate::pack::pack::collect_config_files;
use crate::unpack::interface::{decode_interfaces, write_interfaces};
use anyhow::{Context, Result, ensure};
use rs_io::Packet;
use rs_io::js5::Js5Store;
use runec::clientscript;
use runec::clientscript::decode::Script;
use runec::clientscript::decompile::{decompile, infer_signatures};
use runec::symbol::SymbolKind;
use runec::types::Type;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

fn script_path(dir: &Path, name: &str) -> PathBuf {
    let key = clientscript::script_key(name);
    let (trigger, name) = key[1..key.len() - 1].split_once(',').unwrap();
    let folder = if name
        .strip_prefix("clientscript_")
        .or_else(|| name.strip_prefix("script"))
        .is_some_and(|id| id.parse::<u32>().is_ok())
    {
        dir.join("unpack")
    } else {
        let name = name.trim_start_matches('_');
        dir.join(name.split_once('_').map_or(name, |(prefix, _)| prefix))
            .join(if trigger == "clientscript" {
                "script"
            } else {
                "proc"
            })
    };
    folder.join(format!("{key}.cs2"))
}

pub fn unpack_cs2_scripts(cache_dir: &Path, output_dir: &Path, pack_dir: &Path) -> Result<u32> {
    super::prepare_pack(pack_dir)?;
    let mut store = Js5Store::open(cache_dir)?;
    let names = AssetNames::load(pack_dir)?;
    let dir = output_dir.join("scripts/clientscript");
    let global = output_dir.join("scripts/globalscript");
    // The standalone clientscript importer also needs the declarations that
    // grant scripts access to cache configs. The full importer writes these first.
    if !global.join("_unpack").is_dir() {
        fs::create_dir_all(global.join("_unpack"))?;
        let mut results = Vec::new();
        super::unpack_configs(&mut results, cache_dir, pack_dir, &global);
        super::report::write_report(&results, &output_dir.join("unpack_report.txt"))?;
    }
    let template = pack_dir.with_file_name("scripts");
    let template = if template.join("clientscript/engine.cs2").is_file() {
        template
    } else {
        Path::new(CONTENT_DIR).join("scripts")
    };
    fs::create_dir_all(&dir)?;
    for name in ["engine.cs2", "cs2.constant"] {
        let target = dir.join(name);
        if !target.exists() {
            let source = template.join("clientscript").join(name);
            fs::copy(&source, &target)
                .with_context(|| format!("Copying {} to {}", source.display(), target.display()))?;
        }
    }
    let shared_template = template.join("globalscript");
    let mut shared_paths = HashMap::new();
    if shared_template.is_dir() {
        for source in clientscript::load_sources(&shared_template)? {
            for script in &source.file.scripts {
                let key = format!("[{},{}]", script.trigger, script.name);
                let path = if source.file.scripts.len() == 1 {
                    global.join(source.path.strip_prefix(&shared_template)?)
                } else {
                    script_path(&global, &key).with_extension("gs2")
                };
                shared_paths.insert(key, path);
            }
        }
        for source in collect_config_files(&shared_template, "constant") {
            let target = global.join(source.strip_prefix(&shared_template)?);
            if !target.exists() {
                fs::create_dir_all(target.parent().unwrap())?;
                fs::copy(source, target)?;
            }
        }
    }
    let groups = store
        .reference_table(archive::CLIENTSCRIPT)
        .context("Missing clientscript table")?
        .groups
        .clone();
    let mut original = HashMap::new();
    let mut scripts = HashMap::new();
    let format = clientscript::format::Format::load(&pack_dir.join("cs2_command.pack"))?;
    for info in &groups {
        let data = store
            .read_group_raw_decompressed(archive::CLIENTSCRIPT, info.id)?
            .context("Missing clientscript")?;
        scripts.insert(info.id, Script::decode(&data, &format)?);
        original.insert(info.id, data);
    }
    let mut params = HashMap::new();
    let mut registry = clientscript::registry(&dir, pack_dir)?;
    if let Some(group) = store.read_group(archive::CONFIG, archive::CONFIG_GROUP_PARAM)? {
        for entry in &group.entries {
            let mut param = ParamTypeRaw::new(entry.id as u16);
            param.decode(&mut Packet::from(entry.data().to_vec()));
            params.insert(entry.id as i32, param.var_type == ScriptVarType::String);
            registry.register_param_type(
                entry.id as i32,
                Type::from_char(param.var_type.to_char()).unwrap_or(Type::Int),
            );
        }
    }
    let commands =
        clientscript::load_commands(&pack_dir.join("cs2_command.pack"), &dir.join("engine.cs2"))?;
    let interfaces = decode_interfaces(&mut store)?;
    let hooks: Vec<_> = interfaces
        .iter()
        .flat_map(|interface| &interface.components)
        .flat_map(|component| component.hooks.iter().flatten().cloned())
        .collect();
    let signatures = infer_signatures(&scripts, &hooks, &names, &params, &commands, &mut registry)?;
    let mut sources = Vec::new();
    let mut texts = Vec::new();
    let mut paths = HashMap::new();
    for info in &groups {
        let text = decompile(
            &scripts[&info.id],
            info.id,
            &names,
            &signatures,
            &params,
            &commands,
            &registry,
        )
        .with_context(|| format!("Decompiling clientscript {}", info.id))?;
        let text = format!("// {}\n{text}", info.id);
        let name = names.name("clientscript", info.id);
        let path = shared_paths
            .get(&clientscript::script_key(&name))
            .cloned()
            .unwrap_or_else(|| script_path(&dir, &name));
        sources.push(
            clientscript::Source::parse(&text, &path)
                .with_context(|| format!("Parsing {}", path.display()))?,
        );
        paths.insert(info.id, path.clone());
        texts.push((path, text));
    }
    let ids = clientscript::script_ids(pack_dir)?;
    for source in &sources {
        clientscript::register(&source.file, &ids, &mut registry)?;
    }
    // Variable declarations make client-only ownership explicit, including
    // unnamed variables recovered from bytecode rather than the pack registry.
    let mut variables = BTreeMap::new();
    for (name, symbol) in registry.game_variables() {
        if let SymbolKind::GameVar {
            category, var_id, ..
        } = &symbol.kind
            && matches!(category.as_str(), "varc" | "varcs")
        {
            variables.insert((category.as_str(), *var_id), name);
        }
    }
    let table = fs::read_to_string(global.join("_unpack/all.varc"))?;
    let path = dir.join("configs/all.varc");
    let mut text = fs::read_to_string(&path).unwrap_or_default();
    let existing: HashSet<String> = table
        .lines()
        .chain(text.lines())
        .filter_map(|line| line.trim().strip_prefix('[')?.strip_suffix(']'))
        .map(str::to_owned)
        .collect();
    for ((category, id), name) in variables {
        if !existing.contains(name) {
            let props = if category == "varcs" {
                "type=string\n"
            } else {
                ""
            };
            text.push_str(&format!("// {id}\n[{name}]\n{props}\n"));
        }
    }
    if !text.is_empty() {
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, text)?;
    }
    registry.load_visibility(&dir);
    let mut aliases = Vec::new();
    let ids = clientscript::script_ids(pack_dir)?;
    for source in clientscript::load_sources(&dir)? {
        for script in &source.file.scripts {
            let key = format!("[{},{}]", script.trigger, script.name);
            let id = ids
                .get(&key)
                .copied()
                .map(|id| id as u32)
                .or_else(|| script.name.strip_prefix("clientscript_")?.parse().ok());
            let Some(path) = id.and_then(|id| paths.get(&id)) else {
                continue;
            };
            if source.path == *path {
                continue;
            }
            ensure!(
                source.file.scripts.len() == 1
                    && source
                        .compile(&registry, &format)?
                        .into_iter()
                        .all(|(id, bytes)| original.get(&(id as u32)) == Some(&bytes)),
                "Cannot replace authored script {} with generated source",
                source.path.display()
            );
            aliases.push(source.path.clone());
        }
    }
    write_interfaces(&interfaces, output_dir, &signatures, &names, &registry)?;
    let compiled = clientscript::compile_registered_sources(&sources, pack_dir, registry)?;
    let mut changed: Vec<_> = original
        .iter()
        .filter(|(id, bytes)| compiled.get(id) != Some(bytes))
        .map(|(id, _)| *id)
        .collect();
    changed.sort_unstable();
    ensure!(
        changed.is_empty(),
        "Clientscripts {changed:?} source round trip changed bytes"
    );
    for (path, text) in texts {
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, text)?;
    }
    for path in aliases {
        fs::remove_file(path)?;
    }
    Ok(groups.len() as u32)
}

pub fn scan_cs2_varp_refs(store: &mut Js5Store) -> Result<(HashSet<u32>, HashSet<u32>)> {
    let format =
        clientscript::format::Format::load(&Path::new(CONTENT_DIR).join("pack/cs2_command.pack"))?;
    let mut varps = HashSet::new();
    let mut varbits = HashSet::new();
    let groups = store
        .reference_table(archive::CLIENTSCRIPT)
        .context("Missing clientscript table")?
        .groups
        .clone();
    for info in groups {
        let data = store
            .read_group_raw_decompressed(archive::CLIENTSCRIPT, info.id)?
            .context("Missing clientscript")?;
        let script = Script::decode(&data, &format)?;
        for (&opcode, &operand) in script.opcodes.iter().zip(&script.int_operands) {
            match opcode {
                1 | 2 => {
                    varps.insert(operand as u32);
                }
                25 | 27 => {
                    varbits.insert(operand as u32);
                }
                _ => {}
            }
        }
    }
    Ok((varps, varbits))
}
