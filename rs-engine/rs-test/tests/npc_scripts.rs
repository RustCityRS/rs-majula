//! 289's npc scripts (`npc/scripts`) on 2011's npcs.
//!
//! 2011 re-issued Barbarian Village's barbarians as 17 named npcs, 3246-3263,
//! men and women alike; it has no "barbarian woman". 289 gave its two
//! barbarians, the barbarian and the barbarian woman, its battle cry
//! (`[label,barbarian_default_attack]`), and they are 289's whole barbarian
//! category, so 2011's barbarians of that category cry it too.

#![cfg(since_647)]

use rs_pack::ParamValue;
use rs_test::runner;
use rs_vm::trigger::ServerTriggerType;
use std::collections::HashMap;
use std::fs;

/// A 647 pack registry (`content/647/pack/<kind>.pack`): name to id.
fn pack_id(kind: &str, name: &str) -> i32 {
    let path = format!(
        "{}/../../content/647/pack/{kind}.pack",
        env!("CARGO_MANIFEST_DIR")
    );
    let ids: HashMap<String, i32> = fs::read_to_string(&path)
        .expect(&path)
        .lines()
        .filter_map(|l| l.split_once('='))
        .filter_map(|(id, n)| Some((n.to_string(), id.parse().ok()?)))
        .collect();
    *ids.get(name)
        .unwrap_or_else(|| panic!("{name} is not in {kind}.pack"))
}

/// The script an npc runs for `trigger`: its own, else its category's.
fn npc_script(engine: &rs_engine::Engine, trigger: ServerTriggerType, npc: u16) -> Option<i32> {
    let category = engine
        .cache
        .npcs
        .get_by_id(npc)
        .and_then(|t| t.category)
        .map(i32::from);
    engine
        .script_by_key(trigger, Some(i32::from(npc)), category)
        .map(|s| s.id)
}

/// An npc's int param, by its `param.pack` name.
fn param(engine: &rs_engine::Engine, npc: u16, name: &str) -> Option<i32> {
    let params = engine
        .cache
        .npcs
        .get_by_id(npc)
        .and_then(|n| n.params.as_deref())?;
    match params.get(&pack_id("param", name)) {
        Some(ParamValue::Int(v)) => Some(*v),
        _ => None,
    }
}

/// The script an npc runs when it attacks a player (`ai_opplayer2`): its own,
/// else its category's.
fn attack_script(engine: &rs_engine::Engine, npc: u16) -> Option<i32> {
    npc_script(engine, ServerTriggerType::AiOpPlayer2, npc)
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

/// Each of Barbarian Village's 17 barbarians at 2011 (3246-3263, no 3254) is of
/// 289's barbarian category, with void-634's life points for its level and its
/// weapon's attack animation: the axes 2067, the warhammers 401, the swords
/// 390, the spears 428, Aitan's bow 426. 13 had no config at all before.
#[test]
fn every_2011_barbarian_is_289_s_barbarian_with_its_weapon() {
    runner::on_loaded_engine(|engine| {
        let barbarian = pack_id("category", "barbarian");
        #[rustfmt::skip]
        let rows: [(u16, u16, i32); 17] = [
            (3246, 240, 2067), (3247, 180, 390), (3248, 240, 2067), (3249, 240, 401),
            (3250, 180, 2067), (3251, 180, 390), (3252, 180, 2067), (3253, 240, 401),
            (3255, 180, 401), (3256, 180, 390), (3257, 180, 2067), (3258, 180, 401),
            (3259, 240, 426), (3260, 240, 428), (3261, 200, 428), (3262, 180, 428),
            (3263, 200, 428),
        ];
        for (npc, life_points, anim) in rows {
            let t = engine.cache.npcs.get_by_id(npc).expect("a barbarian");
            assert_eq!(
                t.category.map(i32::from),
                Some(barbarian),
                "{npc}'s category"
            );
            assert_eq!(t.hitpoints, life_points, "{npc}'s life points");
            assert_eq!(
                param(engine, npc, "attack_anim"),
                Some(anim),
                "{npc}'s weapon"
            );
        }
    });
}

/// Aitan (3259) shoots: his attack is his own trigger, not the battle cry's.
/// Every barbarian talks with 289's barbarian lines (the category's op1).
#[test]
fn aitan_shoots_and_every_barbarian_talks() {
    runner::on_loaded_engine(|engine| {
        assert_eq!(
            npc_script(engine, ServerTriggerType::AiOpPlayer2, 3259),
            Some(pack_id("script", "[ai_opplayer2,barbarian_3259]"))
        );
        let talk = pack_id("script", "[opnpc1,_barbarian]");
        for npc in [3246, 3247, 3259, 3263] {
            assert_eq!(
                npc_script(engine, ServerTriggerType::OpNpc1, npc),
                Some(talk),
                "{npc} talks"
            );
        }
    });
}

/// 289's monks of Zamorak zap a quarter of the time (`monk_of_zamorak.rs2`):
/// each attacks through one label with its zap max in life points, 289's
/// chaosmonk1 (188) up to 80 and chaosmonk2 (189) up to 60 (the 2011 wiki's
/// level-22 and level-17 figures), chaosmonk3 (190) 289's 130, and 2011's
/// Defender of Varrock monk (8178, level 17) 60.
#[test]
fn monks_of_zamorak_zap_with_their_level_s_max() {
    runner::on_loaded_engine(|engine| {
        let label = pack_id("script", "[label,monk_of_zamorak_attack]");
        for (npc, max) in [(188u16, 80), (189, 60), (190, 130), (8178, 60)] {
            let script = engine
                .script_by_key(ServerTriggerType::AiOpPlayer2, Some(i32::from(npc)), None)
                .unwrap_or_else(|| panic!("{npc} has no ai_opplayer2"));
            assert!(
                script.int_operands.contains(&label),
                "{npc} does not attack through the monks' label"
            );
            assert!(
                script.int_operands.contains(&max),
                "{npc}'s zap max is not {max}"
            );
        }
    });
}
