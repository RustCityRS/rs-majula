use runec::opcodes::COMMANDS;
use runec::pointer::{PointerType, command_pointers};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[test]
fn compiler_covers_entity_handlers() {
    let ops = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ops");
    let rules = command_pointers();
    let mut missing = BTreeSet::new();
    let mut handlers = 0;
    for entry in fs::read_dir(ops).unwrap() {
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
            let (_, opcode) = COMMANDS
                .iter()
                .find(|(name, _)| name.to_uppercase() == constant)
                .unwrap_or_else(|| panic!("{constant} is not a runec opcode"));
            for (name, _) in COMMANDS.iter().filter(|(_, code)| code == opcode) {
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
                        missing.insert(format!("{command} must require {pointer} ({constant})"));
                    }
                }
            }
        }
    }
    assert!(handlers > 0);
    assert!(
        missing.is_empty(),
        "{}",
        missing.into_iter().collect::<Vec<_>>().join("\n")
    );
}
