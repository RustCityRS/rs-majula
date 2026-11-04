//! 289's npc scripts (`npc/scripts`) on 2011's npcs.
//!
//! 2011 re-issued Barbarian Village's barbarians as 17 named npcs, 3246-3263,
//! men and women alike; it has no "barbarian woman". 289 gave its two
//! barbarians, the barbarian and the barbarian woman, its battle cry
//! (`[label,barbarian_default_attack]`), and they are 289's whole barbarian
//! category, so 2011's barbarians of that category cry it too.

#![cfg(since_647)]

use rs_test::runner;
use rs_vm::trigger::ServerTriggerType;
use std::collections::HashMap;

/// A 647 pack registry (`content/647/pack/<kind>.pack`): name to id.
fn pack_id(kind: &str, name: &str) -> i32 {
    let path = format!(
        "{}/../../content/647/pack/{kind}.pack",
        env!("CARGO_MANIFEST_DIR")
    );
    let ids: HashMap<String, i32> = std::fs::read_to_string(&path)
        .expect(&path)
        .lines()
        .filter_map(|l| l.split_once('='))
        .filter_map(|(id, n)| Some((n.to_string(), id.parse().ok()?)))
        .collect();
    *ids.get(name)
        .unwrap_or_else(|| panic!("{name} is not in {kind}.pack"))
}

/// The script an npc runs when it attacks a player (`ai_opplayer2`): its own,
/// else its category's.
fn attack_script(engine: &rs_engine::Engine, npc: u16) -> Option<i32> {
    let category = engine
        .cache
        .npcs
        .get_by_id(npc)
        .and_then(|t| t.category)
        .map(i32::from);
    engine
        .script_by_key(
            ServerTriggerType::AiOpPlayer2,
            Some(i32::from(npc)),
            category,
        )
        .map(|s| s.id)
}

/// Alberich (3246, 2011's `barbarian`) runs 289's barbarian trigger; Fasolt,
/// Minarch and Brunnhilde, 2011's barbarians of 289's category, its
/// category's. Both jump to 289's battle cry.
#[test]
fn barbarian_village_s_barbarians_attack_with_289_s_battle_cry() {
    runner::on_loaded_engine(|engine| {
        assert_eq!(
            attack_script(engine, 3246),
            Some(pack_id("script", "[ai_opplayer2,barbarian]"))
        );
        let category = pack_id("script", "[ai_opplayer2,_barbarian]");
        for npc in [3248, 3253, 3260] {
            assert_eq!(attack_script(engine, npc), Some(category), "{npc}");
        }
    });
}
