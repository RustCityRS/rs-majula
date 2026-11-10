//! 2011's re-issued npcs answer their 289 base's triggers.
//!
//! `npc_clones_2011.npc` (since 9217853428 folded into `globalscript/_unpack/all.npc`) gives 362 of 2011's npcs the server config of a 289
//! npc, named in each section's comment ("289's [cow]"). Config alone left 127
//! of them without some of their base's own triggers, so they fell to 647's
//! defaults: a fire giant dropped only its bones, a chaos druid never cast, a
//! sheep never baa'd. Each clone now runs its base's script for every trigger
//! the base answers, unless the base's script does not fit it, and those are
//! listed here with the reason.

#![cfg(since_647)]

use rs_test::runner;
use rs_vm::trigger::ServerTriggerType;
use std::collections::{BTreeSet, HashMap};
use std::fs;

/// The npc triggers a 289 base answers by its own name or category.
#[rustfmt::skip]
const TRIGGERS: &[(ServerTriggerType, &str)] = {
    use ServerTriggerType::*;
    &[
        (OpNpc1, "opnpc1"), (OpNpc2, "opnpc2"), (OpNpc3, "opnpc3"), (OpNpc4, "opnpc4"),
        (OpNpc5, "opnpc5"), (OpNpcU, "opnpcu"), (ApNpc1, "apnpc1"), (ApNpc2, "apnpc2"),
        (ApNpc3, "apnpc3"), (ApNpc4, "apnpc4"), (ApNpc5, "apnpc5"), (ApNpcU, "apnpcu"),
        (AiApPlayer1, "ai_applayer1"), (AiApPlayer2, "ai_applayer2"),
        (AiOpPlayer1, "ai_opplayer1"), (AiOpPlayer2, "ai_opplayer2"),
        (AiQueue1, "ai_queue1"), (AiQueue2, "ai_queue2"), (AiQueue3, "ai_queue3"),
        (AiQueue4, "ai_queue4"), (AiQueue5, "ai_queue5"), (AiQueue6, "ai_queue6"),
        (AiQueue7, "ai_queue7"), (AiQueue8, "ai_queue8"), (AiQueue9, "ai_queue9"),
        (AiQueue10, "ai_queue10"), (AiTimer, "ai_timer"), (AiSpawn, "ai_spawn"),
        (AiDespawn, "ai_despawn"),
    ]
};

/// Clones left on 647's default for a trigger their base answers, and why
/// (the sources are in the scripts' comments).
#[rustfmt::skip]
const NOT_THE_BASE_S: &[(&str, &str, &str)] = &[
    // 289's rat drops no bones; the 2011 wiki's Crypt rat always drops bones.
    ("barrows_rat", "ai_queue3", "289's rat table does not fit"),
    // Lambs are non-interactive at 2011 and cannot be sheared.
    ("li_l_lamb", "ai_timer", "no source has lambs baa"),
    ("li_l_lamb", "opnpc1", "a lamb has no Shear op"),
    ("li_l_lamb", "opnpcu", "a lamb cannot be sheared"),
    ("lamb", "ai_timer", "no source has lambs baa"),
    ("lamb", "opnpc1", "a lamb has no Shear op"),
    ("lamb", "opnpcu", "a lamb cannot be sheared"),
    // Two penguins dressed as a sheep: they get away when sheared
    // (shear_sheep.rs2), but do not baa.
    ("sheep_shearer_the_thing", "ai_timer", "waits for the Cold War port"),
    // Shorn sheep have no Shear op (their use is the shorn sheep's).
    ("sheep_1762", "opnpc1", "shorn: no Shear op"),
    ("sheep_1764", "opnpc1", "shorn: no Shear op"),
    ("sheep_5148", "opnpc1", "shorn: no Shear op"),
    ("sheep_5149", "opnpc1", "shorn: no Shear op"),
    ("sheep_5150", "opnpc1", "shorn: no Shear op"),
    ("sheep_5151", "opnpc1", "shorn: no Shear op"),
    ("sheep_5152", "opnpc1", "shorn: no Shear op"),
    ("sheep_5153", "opnpc1", "shorn: no Shear op"),
    ("sheep_5154", "opnpc1", "shorn: no Shear op"),
    ("sheep_5155", "opnpc1", "shorn: no Shear op"),
    ("sheep_8877", "opnpc1", "shorn: no Shear op"),
    // The Dorgesh-Kaan station's dwarves share the guild dwarf's name only.
    ("slice_dwarf1", "opnpc1", "not a Mining Guild dwarf"),
    ("slice_dwarf2", "opnpc1", "not a Mining Guild dwarf"),
    ("slice_dwarf3", "opnpc1", "not a Mining Guild dwarf"),
    ("slice_dwarf4", "opnpc1", "not a Mining Guild dwarf"),
    ("slice_dwarf5", "opnpc1", "not a Mining Guild dwarf"),
    ("slice_dwarf6", "opnpc1", "not a Mining Guild dwarf"),
    // Fight as 289's man, but cache 786 gives them no Pickpocket.
    ("thief2", "opnpc3", "no Pickpocket op"),
    ("thief_2696", "opnpc3", "no Pickpocket op"),
    ("mugger_2697", "opnpc3", "no Pickpocket op"),
    ("surok_king", "opnpc3", "no Pickpocket op"),
    ("qip_soa_thief_male01", "opnpc3", "no Pickpocket op"),
    ("qip_soa_thief_male02", "opnpc3", "no Pickpocket op"),
    ("qip_soa_thief_female01", "opnpc3", "no Pickpocket op"),
    ("qip_soa_thief_female02", "opnpc3", "no Pickpocket op"),
    ("rc_zmi_runerunner", "opnpc3", "no Pickpocket op"),
    ("rc_zmi_runerunner2", "opnpc3", "no Pickpocket op"),
    ("icemt_monk", "opnpc3", "no Pickpocket op"),
    ("luc2_lucien_spy", "opnpc3", "no Pickpocket op"),
    ("luc2_spy2", "opnpc3", "no Pickpocket op"),
];

/// A 647 pack registry (`content/647/pack/<kind>.pack`): name to id.
fn pack(kind: &str) -> HashMap<String, i32> {
    let path = format!(
        "{}/../../content/647/pack/{kind}.pack",
        env!("CARGO_MANIFEST_DIR")
    );
    fs::read_to_string(&path)
        .expect(&path)
        .lines()
        .filter_map(|l| l.split_once('='))
        .filter_map(|(id, n)| Some((n.to_string(), id.parse().ok()?)))
        .collect()
}

/// Each clone (a section whose comment names "289's [base]") and the 289 npc
/// that comment names. The clones' sections were `npc_clones_2011.npc` until
/// 9217853428 folded every authored npc section into `globalscript/_unpack/all.npc`,
/// where the comment follows the cache's lines.
fn clones() -> Vec<(String, String)> {
    let path = format!(
        "{}/../../content/647/scripts/globalscript/_unpack/all.npc",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = fs::read_to_string(&path).expect(&path);
    let mut out = Vec::new();
    let mut section: Option<String> = None;
    for line in text.lines() {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = Some(name.to_string());
        } else if line.starts_with("//")
            && let Some(at) = line.find("289's [")
            && let Some(clone) = section.take()
        {
            let rest = &line[at + "289's [".len()..];
            let base = &rest[..rest.find(']').expect("a closed base name")];
            out.push((clone, base.to_string()));
        }
    }
    out
}

/// Every (clone, trigger) where the base runs a script of its own (or its
/// category's) and the clone runs 647's default.
#[test]
fn every_2011_clone_answers_its_289_base_s_triggers() {
    runner::on_loaded_engine(|engine| {
        let npcs = pack("npc");
        let clones = clones();
        assert!(clones.len() > 300, "{} clones read", clones.len());
        let script_of = |trigger: ServerTriggerType, npc: i32| {
            let category = engine
                .cache
                .npcs
                .get_by_id(npc as u16)
                .and_then(|t| t.category)
                .map(i32::from);
            engine
                .script_by_key(trigger, Some(npc), category)
                .map(|s| s.id)
        };
        let mut gaps = BTreeSet::new();
        for (clone, base) in &clones {
            let (Some(&c), Some(&b)) = (npcs.get(clone), npcs.get(base)) else {
                panic!("{clone} or {base} is not in npc.pack");
            };
            for &(trigger, name) in TRIGGERS {
                let default = engine.script_by_key(trigger, None, None).map(|s| s.id);
                if script_of(trigger, b) != default && script_of(trigger, c) == default {
                    gaps.insert((clone.clone(), name.to_string()));
                }
            }
        }
        let expected: BTreeSet<(String, String)> = NOT_THE_BASE_S
            .iter()
            .map(|&(clone, trigger, _)| (clone.to_string(), trigger.to_string()))
            .collect();
        let missing: Vec<_> = gaps.difference(&expected).collect();
        let stale: Vec<_> = expected.difference(&gaps).collect();
        assert!(
            missing.is_empty() && stale.is_empty(),
            "{} clones lack their base's trigger: {missing:?}\n\
             {} listed exceptions answer it now: {stale:?}",
            missing.len(),
            stale.len()
        );
    });
}

/// The clones jump to the very label their base does: a 2011 fire giant to
/// 289's fire giant's table, a Black Guard to the dwarves', a Mining Guild
/// dwarf to the guild dwarf's talk.
#[test]
fn clones_jump_to_their_base_s_label() {
    runner::on_loaded_engine(|engine| {
        let npcs = pack("npc");
        let scripts = pack("script");
        #[rustfmt::skip]
        let cases = [
            (ServerTriggerType::AiQueue3, "firegiant", "firegiant2", "[label,fire_giant_drop_table]"),
            (ServerTriggerType::AiQueue3, "dwarf_mountain", "fai_black_guard_01", "[label,dwarf_drop_table]"),
            (ServerTriggerType::AiQueue3, "skeleton_armed", "barrows_skeleton_unarmed", "[label,skeleton_armed]"),
            (ServerTriggerType::AiQueue3, "zombie_unarmed", "sos_fam_zombie_unarmed2", "[label,unarmed_zombie]"),
            (ServerTriggerType::AiQueue3, "hobgoblin", "hobgoblin_2688", "[label,hobgoblin_drop_table]"),
            (ServerTriggerType::AiOpPlayer2, "chaos_druid", "chaos_druid_2", "[label,chaos_druid_ai_opplayer2]"),
            (ServerTriggerType::AiApPlayer2, "highwayman", "highwayman_7443", "[label,highwayman_ai_applayer2]"),
            (ServerTriggerType::AiQueue2, "imp", "godwars_ancient_imp", "[label,imp_ai_queue2]"),
            (ServerTriggerType::AiTimer, "sheepunsheered", "sheep_5153", "[label,sheep_baa]"),
            (ServerTriggerType::OpNpc1, "sheepunsheered", "sheep_5162", "[label,shear_sheep_op1]"),
            (ServerTriggerType::OpNpcU, "cow", "cow_12362", "[label,cow_use]"),
            (ServerTriggerType::OpNpc1, "guilddwarf", "dwarf_3294", "[label,guilddwarf_talk]"),
            (ServerTriggerType::OpNpc3, "man", "rimmington_hengel", "[label,attempt_pick_pocket]"),
        ];
        for (trigger, base, clone, label) in cases {
            let label = scripts[label];
            for npc in [base, clone] {
                let script = engine
                    .script_by_key(trigger, Some(npcs[npc]), None)
                    .unwrap_or_else(|| panic!("{npc} has no {trigger:?}"));
                assert!(
                    script.int_operands.contains(&label),
                    "{npc}'s {trigger:?} does not jump to {label}"
                );
            }
        }
    });
}
