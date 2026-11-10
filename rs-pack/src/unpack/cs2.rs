use crate::CONTENT_DIR;
use crate::archive;
use crate::cache::ScriptVarType;
use crate::cache::param::ParamTypeRaw;
use crate::cache::provider::CacheType;
use crate::pack::names::AssetNames;
use crate::unpack::interface::{decode_interfaces, write_interfaces};
use anyhow::{Context, Result, bail, ensure};
use rs_io::Packet;
use rs_io::js5::Js5Store;
use runec::clientscript;
use runec::clientscript::decode::Script;
use runec::clientscript::decompile::{decompile, infer_signatures};
use runec::lexer::Lexer;
use runec::sources::{Scope, engine_file, script_files, target_config_files};
use runec::symbol::SymbolKind;
use runec::token::Kind;
use runec::types::Type;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

fn script_path(dir: &Path, template: &Path, name: &str, scope: Scope) -> PathBuf {
    let key = clientscript::script_key(name);
    let (_, name) = key[1..key.len() - 1].split_once(',').unwrap();
    let folder = if name
        .strip_prefix("clientscript_")
        .or_else(|| name.strip_prefix("script"))
        .is_some_and(|id| id.parse::<u32>().is_ok())
    {
        dir.join("unpack")
    } else {
        let name = name.trim_start_matches('_');
        let prefix = name.split_once('_').map_or(name, |(prefix, _)| prefix);
        let candidates = [
            prefix.to_owned(),
            format!("skill_{prefix}"),
            format!("interface_{prefix}"),
            format!("{prefix}s"),
        ];
        let matches = candidates
            .iter()
            .filter(|name| template.join(name).is_dir() || dir.join(name).is_dir())
            .collect::<Vec<_>>();
        dir.join(if matches.len() == 1 {
            matches[0].as_str()
        } else {
            prefix
        })
    };
    folder
        .join(if scope == Scope::Global {
            "global"
        } else {
            "client"
        })
        .join(format!("{key}.{}", scope.extension()))
}

fn template_script_paths(dir: &Path, template: &Path) -> Result<HashMap<String, PathBuf>> {
    let mut paths = HashMap::new();
    for path in script_files(template, "cs2")? {
        if path
            .file_name()
            .is_some_and(|name| name == "engine.cs2" || name == "engine.gs2")
        {
            continue;
        }
        let source = clientscript::Source::parse(&fs::read_to_string(&path)?, &path)?;
        let target = dir.join(path.strip_prefix(template)?);
        for script in &source.file.scripts {
            let key = format!("[{},{}]", script.trigger, script.name);
            let path = if source.file.scripts.len() == 1 {
                target.clone()
            } else {
                target.with_file_name(format!(
                    "{key}.{}",
                    path.extension().unwrap().to_string_lossy()
                ))
            };
            paths.insert(key, path);
        }
    }
    Ok(paths)
}

pub fn unpack_cs2_scripts(cache_dir: &Path, output_dir: &Path, pack_dir: &Path) -> Result<u32> {
    super::prepare_pack(pack_dir)?;
    let mut store = Js5Store::open(cache_dir)?;
    let names = AssetNames::load(pack_dir)?;
    let scripts_dir = output_dir.join("scripts");
    let configs = scripts_dir.join("configs");
    // The standalone clientscript importer also needs the declarations that
    // grant scripts access to cache configs. The full importer writes these first.
    if !configs.join("_unpack").is_dir() {
        fs::create_dir_all(configs.join("_unpack"))?;
        let mut results = Vec::new();
        super::unpack_configs(&mut results, cache_dir, pack_dir, &configs);
        super::report::write_report(&results, &output_dir.join("unpack_report.txt"))?;
    }
    let template = pack_dir.with_file_name("scripts");
    let (template, template_engine) = if let Some(engine) = engine_file(&template, Scope::Client)? {
        (template, engine)
    } else {
        let template = Path::new(CONTENT_DIR).join("scripts");
        let engine = engine_file(&template, Scope::Client)?
            .with_context(|| format!("Missing engine.cs2 under {}", template.display()))?;
        (template, engine)
    };
    fs::create_dir_all(&scripts_dir)?;
    let engine = if let Some(engine) = engine_file(&scripts_dir, Scope::Client)? {
        engine
    } else {
        let target = scripts_dir.join(template_engine.strip_prefix(&template)?);
        fs::create_dir_all(target.parent().unwrap())?;
        fs::copy(&template_engine, &target).with_context(|| {
            format!(
                "Copying {} to {}",
                template_engine.display(),
                target.display()
            )
        })?;
        target
    };
    for source in target_config_files(&template, Scope::Client)? {
        if source.extension().is_none_or(|ext| ext != "constant") {
            continue;
        }
        let target = scripts_dir.join(source.strip_prefix(&template)?);
        if !target.exists() {
            fs::create_dir_all(target.parent().unwrap())?;
            fs::copy(&source, &target)
                .with_context(|| format!("Copying {} to {}", source.display(), target.display()))?;
        }
    }
    let template_paths = template_script_paths(&scripts_dir, &template)?;
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
    let mut registry = clientscript::registry(&scripts_dir, pack_dir)?;
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
    let commands = clientscript::load_commands(&pack_dir.join("cs2_command.pack"), &engine)?;
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
        let path = template_paths
            .get(&clientscript::script_key(&name))
            .cloned()
            .unwrap_or_else(|| script_path(&scripts_dir, &template, &name, Scope::Client));
        if text.contains("__") {
            let tokens = Lexer::new(&text, &path).tokenize()?;
            if let Some(pair) = tokens.windows(2).find(|pair| {
                pair[0].kind == Kind::Identifier
                    && pair[0].value.starts_with("__")
                    && pair[1].kind == Kind::LParen
            }) {
                bail!(
                    "Clientscript {} still requires helper {} at line {}; cannot export ordinary source",
                    info.id,
                    pair[0].value,
                    pair[0].line
                );
            }
        }
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
            && matches!(category.as_str(), "varc" | "varcstr")
        {
            variables.insert((category.as_str(), *var_id), name);
        }
    }
    let table = fs::read_to_string(configs.join("_unpack/all.varc"))?;
    let path = configs.join("client/all.varc");
    let mut text = fs::read_to_string(&path).unwrap_or_default();
    let existing: HashSet<String> = table
        .lines()
        .chain(text.lines())
        .filter_map(|line| line.trim().strip_prefix('[')?.strip_suffix(']'))
        .map(str::to_owned)
        .collect();
    for ((category, id), name) in variables {
        if !existing.contains(name) {
            let props = if category == "varcstr" {
                "type=string\n"
            } else {
                ""
            };
            text.push_str(&format!("// {id}\n[{name}]\n{props}\n"));
        }
    }
    if !text.is_empty() {
        if !path.exists() {
            text.insert_str(0, "// @scope(client)\n");
        }
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, text)?;
    }
    registry.load_visibility(&scripts_dir);
    let mut aliases = Vec::new();
    let ids = clientscript::script_ids(pack_dir)?;
    for source in clientscript::load_sources(&scripts_dir)? {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, process};

    #[test]
    fn unpack_preserves_existing_feature_paths_and_uses_matching_server_folders() {
        struct Fixture(PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let path = self.0.canonicalize().unwrap();
                let temp = env::temp_dir().canonicalize().unwrap();
                assert!(path != temp && path.starts_with(temp));
                fs::remove_dir_all(path).unwrap();
            }
        }
        let fixture = Fixture(env::temp_dir().join(format!("rs-unpack-layout-{}", process::id())));
        let template = fixture.0.join("template");
        let output = fixture.0.join("output");
        for (path, text) in [
            (
                "interface_bank/client/custom.cs2",
                "[proc,bank_a]() return;",
            ),
            (
                "skill_combat/global/bundle.gs2",
                "[proc,combat_a]() return;\n[proc,combat_b]() return;",
            ),
        ] {
            let path = template.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        fs::create_dir_all(template.join("tasks")).unwrap();
        let paths = template_script_paths(&output, &template).unwrap();
        assert_eq!(
            paths["[proc,bank_a]"],
            output.join("interface_bank/client/custom.cs2")
        );
        for name in ["combat_a", "combat_b"] {
            assert_eq!(
                paths[&format!("[proc,{name}]")],
                output.join(format!("skill_combat/global/[proc,{name}].gs2"))
            );
        }
        for (name, folder) in [
            ("bank_new", "interface_bank"),
            ("combat_new", "skill_combat"),
            ("task_new", "tasks"),
            ("fresh_new", "fresh"),
            ("clientscript_42", "unpack"),
        ] {
            assert_eq!(
                script_path(&output, &template, name, Scope::Client),
                output.join(format!("{folder}/client/[proc,{name}].cs2")),
            );
        }
        fs::create_dir_all(template.join("combat")).unwrap();
        assert_eq!(
            script_path(&output, &template, "combat_new", Scope::Client),
            output.join("combat/client/[proc,combat_new].cs2"),
        );
    }
}
