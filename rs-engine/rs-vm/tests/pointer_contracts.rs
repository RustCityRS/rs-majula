use runec::pointer::{PointerType, command_pointers};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[test]
fn compiler_covers_entity_handlers_in_every_command_pack() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let constants = fs::read_to_string(root.join("../rs-pack/src/cache/script.rs")).unwrap();
    let opcodes: BTreeMap<_, _> = constants
        .lines()
        .filter_map(|line| line.strip_prefix("pub const "))
        .filter_map(|line| line.split_once(": u16 = "))
        .filter_map(|(name, value)| {
            value
                .split(';')
                .next()?
                .parse::<u16>()
                .ok()
                .map(|value| (name, value))
        })
        .collect();
    let packs: Vec<_> = ["225", "244", "245.2", "254", "274", "289", "647"]
        .into_iter()
        .map(|rev| {
            let text = fs::read_to_string(root.join(format!("../content/{rev}/pack/command.pack")))
                .unwrap();
            let commands: BTreeMap<_, _> = text
                .lines()
                .filter_map(|line| line.split_once('='))
                .map(|(opcode, name)| (opcode.parse::<u16>().unwrap(), name.to_owned()))
                .collect();
            (rev, commands)
        })
        .collect();
    let rules = command_pointers();
    let mut missing = BTreeSet::new();
    let mut handlers = 0;
    for entry in fs::read_dir(root.join("rs-vm/src/ops")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        for line in source.lines() {
            let Some((handler, rest)) = line.trim().split_once("!(m, ") else {
                continue;
            };
            let pointers = match handler.strip_suffix("_mut").unwrap_or(handler) {
                "active_player" => [PointerType::ActivePlayer, PointerType::ActivePlayer2],
                "protected_active_player" => {
                    [PointerType::PActivePlayer, PointerType::PActivePlayer2]
                }
                "active_npc" => [PointerType::ActiveNpc, PointerType::ActiveNpc2],
                "active_loc" => [PointerType::ActiveLoc, PointerType::ActiveLoc2],
                "active_obj" => [PointerType::ActiveObj, PointerType::ActiveObj2],
                _ => continue,
            };
            handlers += 1;
            let constant = rest.split_whitespace().next().unwrap();
            let opcode = opcodes[constant];
            let mut registered = false;
            for (rev, commands) in &packs {
                let Some(name) = commands.get(&opcode) else {
                    continue;
                };
                registered = true;
                for (dot, pointer) in ["", "."].into_iter().zip(pointers) {
                    let command = format!("{dot}{name}");
                    let covered = rules.get(&command).is_some_and(|holder| {
                        holder.required.contains(pointer)
                            || match pointer {
                                PointerType::ActivePlayer => {
                                    holder.required.contains(PointerType::PActivePlayer)
                                }
                                PointerType::ActivePlayer2 => {
                                    holder.required.contains(PointerType::PActivePlayer2)
                                }
                                _ => false,
                            }
                    });
                    if !covered {
                        missing.insert(format!(
                            "{rev}: {command} must require {pointer} ({constant})"
                        ));
                    }
                }
            }
            assert!(registered, "{constant} has no command-pack entry");
        }
    }
    assert!(handlers > 0);
    assert!(
        missing.is_empty(),
        "{}",
        missing.into_iter().collect::<Vec<_>>().join("\n")
    );
}
