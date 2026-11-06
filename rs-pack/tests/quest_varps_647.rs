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
//! (`content/647/clientscript/_unpack/2193.bin`), and fails if a listed
//! varp is not there with that value, or if 647's registry does not give the
//! name that id.
//!
//! ```text
//! cargo test -p rs-pack --test quest_varps_647
//! ```

#![cfg(since_647)]

use rs_pack::cs2::decompile;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// varp id -> the completion values the client's quest table gives it.
fn client_quest_table(root: &Path) -> BTreeMap<u32, Vec<i32>> {
    // 786's own script, as the repo carries it unpacked: CI has no
    // reference cache in expected/ since the unpacked cache is in git.
    let script = fs::read(root.join("content/647/clientscript/_unpack/2193.bin"))
        .expect("clientscript 2193 in content/647/clientscript/_unpack");
    let source = decompile(&script, 2193).expect("decompile 2193");

    // Each case is `~script_2157(%varp63, 6)`, or -- decompiled without
    // the callee's signature, as here -- `~script_2157;` followed by
    // `return(%varp63, 6);`, the arguments left on the stack.
    let mut table: BTreeMap<u32, Vec<i32>> = BTreeMap::new();
    let mut pending: Option<&str> = None;
    for line in source.lines().map(str::trim) {
        let (script, args) = if let Some(call) = line.strip_prefix("~script_") {
            match call.split_once('(') {
                Some((script, args)) => (script, args),
                None => {
                    pending = Some(call.trim_end_matches(';'));
                    continue;
                }
            }
        } else if let (Some(script), Some(args)) = (pending.take(), line.strip_prefix("return(")) {
            (script, args)
        } else {
            continue;
        };
        let args: Vec<&str> = args
            .trim_end_matches([')', ';'])
            .split(',')
            .map(str::trim)
            .collect();
        // 2157(var, complete); 2158(var, start, complete);
        // 2159(var, complete, var, complete).
        let pairs: Vec<(&str, &str)> = match script {
            "2157" => vec![(args[0], args[1])],
            "2158" => vec![(args[0], args[2])],
            "2159" => vec![(args[0], args[1]), (args[2], args[3])],
            _ => continue,
        };
        for (var, complete) in pairs {
            if let (Some(id), Ok(complete)) = (var.strip_prefix("%varp"), complete.parse()) {
                if let Ok(id) = id.parse() {
                    table.entry(id).or_default().push(complete);
                }
            }
        }
    }
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
