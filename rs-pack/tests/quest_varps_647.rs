//! 289's quest varps are the ones the 647 client reads for those quests.
//!
//! The 647 client decides each quest's state in clientscript 2193, a switch
//! on the quest id that hands one var and its completion value to
//! script_2157/2158. 289 content tracks a quest in a varp and finishes it at a
//! `^<quest>_complete` value. Where the two agree -- same varp id, same
//! completion value -- 647's registry names the varp as 289 does, so 289's
//! quest content ports unchanged and the client's quest list follows it.
//!
//! `quest_varps_647.tsv` lists each such varp. This decompiles the client's
//! own table from 786's clientscript 2193, unpacked into the repo
//! (`content/647/scripts/clientscript/**/*.cs2`), and fails if a listed
//! varp is not there with that value, or if 647's registry does not give the
//! name that id.
//!
//! ```text
//! cargo test -p rs-pack --test quest_varps_647
//! ```

#![cfg(since_647)]

use rs_pack::pack::names::AssetNames;
use runec::clientscript;
use runec::parser::{Expr, Statement};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn client_quest_table(root: &Path) -> BTreeMap<u32, Vec<i32>> {
    let names = AssetNames::load(&root.join("content/647/pack")).unwrap();
    let key = clientscript::script_key(&names.name("clientscript", 2193));
    let source = clientscript::load_sources(&root.join("content/647/scripts/clientscript"))
        .unwrap()
        .into_iter()
        .find(|source| {
            source
                .file
                .scripts
                .iter()
                .any(|script| format!("[{},{}]", script.trigger, script.name) == key)
        })
        .expect("Missing client quest table");
    let mut table = BTreeMap::new();
    fn visit(body: &[Statement], names: &AssetNames, table: &mut BTreeMap<u32, Vec<i32>>) {
        for statement in body {
            match statement {
                Statement::Return { values, .. } => {
                    for value in values {
                        let Expr::ProcCall { name, args, .. } = value else {
                            continue;
                        };
                        let pairs: &[(usize, usize)] = match name.as_str() {
                            "clientscript_2157" => &[(0, 1)],
                            "clientscript_2158" => &[(0, 2)],
                            "clientscript_2159" => &[(0, 1), (2, 3)],
                            _ => continue,
                        };
                        for &(var, value) in pairs {
                            if let (Expr::GameVar(name, _), Expr::IntLiteral(value)) =
                                (&args[var], &args[value])
                                && let Ok(id) = names.id("varp", name)
                            {
                                table.entry(id).or_default().push(*value);
                            }
                        }
                    }
                }
                Statement::If {
                    body,
                    else_if,
                    else_body,
                    ..
                } => {
                    visit(body, names, table);
                    for (_, body, _) in else_if {
                        visit(body, names, table);
                    }
                    if let Some(body) = else_body {
                        visit(body, names, table);
                    }
                }
                Statement::Switch { cases, default, .. } => {
                    for case in cases {
                        visit(&case.body, names, table);
                    }
                    if let Some(body) = default {
                        visit(body, names, table);
                    }
                }
                _ => {}
            }
        }
    }
    visit(&source.file.scripts[0].body, &names, &mut table);
    table
}

#[test]
fn quest_varps_named_from_289_are_the_clients() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let table = client_quest_table(&root);
    assert!(
        table.len() > 50,
        "the client's quest table parsed to only {} varps; has the decompiler's output changed?",
        table.len()
    );

    let pack = fs::read_to_string(root.join("content/647/pack/varp.pack")).unwrap();
    let names: BTreeMap<u32, &str> = pack
        .lines()
        .filter_map(|l| {
            let (id, name) = l.split_once('=')?;
            Some((id.parse().ok()?, name))
        })
        .collect();

    let mut wrong = Vec::new();
    for line in include_str!("quest_varps_647.tsv")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let f: Vec<&str> = line.split('\t').collect();
        let (name, id, complete, quest) = (f[0], f[1].parse::<u32>().unwrap(), f[2], f[4]);
        let complete: i32 = complete.parse().unwrap();
        if names.get(&id) != Some(&name) {
            wrong.push(format!("{name}: 647's varp {id} is `{:?}`", names.get(&id)));
        }
        match table.get(&id) {
            Some(values) if values.contains(&complete) => {}
            Some(values) => wrong.push(format!(
                "{name} ({quest}): the client completes varp {id} at {values:?}, not {complete}"
            )),
            None => wrong.push(format!(
                "{name} ({quest}): the client's quest table does not read varp {id}"
            )),
        }
    }
    assert!(
        wrong.is_empty(),
        "quest varps out of step:\n  {}",
        wrong.join("\n  ")
    );
}
