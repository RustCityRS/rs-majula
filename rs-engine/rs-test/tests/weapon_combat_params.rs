//! Weapons 2011 renamed keep 289's combat params.
//!
//! 647's first port carried 289's combat params (category, bonuses, attack and
//! defend anims, sounds) over to cache 786's objs by display name. Where 2011
//! renamed an obj -- "Bronze axe" is "Bronze hatchet", "Staff of guthix" is
//! "Guthix staff", "Prod" is "Cattleprod" -- the params were left out, so the
//! player punched with it (`slashattack_anim`'s default, `human_unarmedpunch`),
//! blocked unarmed (`human_unarmed_def`) and had no attack bonuses.

#![cfg(since_647)]

use rs_pack::ParamValue;
use rs_pack::types::NpcStat;
use rs_test::names::BACKPACK;
use rs_test::runner;
use rs_ver::TypeId;
use rs_vm::ops::test::{calls_for, forget_calls, set_recording};

/// The objs the first port missed: in both 289 and 647 under one debugname,
/// with 289 combat params and none at 647.
const RENAMED: &[&str] = &[
    "silver_sickle",
    "saradomin_staff",
    "guthix_staff",
    "zamorak_staff",
    "iron_axe",
    "bronze_axe",
    "steel_axe",
    "mithril_axe",
    "adamant_axe",
    "rune_axe",
    "black_axe",
    "iron_sickle",
    "castlewars_saradomin_banner",
    "castlewars_zamorak_banner",
    "castlewars_sword",
    "trollromance_toboggon_waxed",
    "trollromance_rare_flower",
    "machette",
    "ibanstaff",
    "scythe",
    "zqdeadbeads",
    "holy_water",
    "deathdagger",
    "deathdaggerdone",
    "silver_sickle_blessed",
    "unfinished_saradominbook",
    "saradominbook_complete",
    "unfinished_zamorakbook",
    "zamorakbook_complete",
    "unfinished_guthixbook",
    "guthixbook_complete",
    "elemental_shield",
    "dagger_wolfbane",
    "faladian_sword",
    "dramen_staff",
    "phoenix_crossbow",
    "viking_sword",
    "viking_shield",
    "excalibur",
    "cattleprod",
    "macro_pickaxehandle",
    "macro_broken_bronze_pickaxe",
    "macro_broken_iron_pickaxe",
    "macro_broken_steel_pickaxe",
    "macro_broken_mithril_pickaxe",
    "macro_broken_adamant_pickaxe",
    "macro_broken_rune_pickaxe",
    "macro_hatchethandle",
    "macro_broken_bronze_hatchet",
    "macro_broken_iron_hatchet",
    "macro_broken_steel_hatchet",
    "macro_broken_black_hatchet",
    "macro_broken_mithril_hatchet",
    "macro_broken_adamant_hatchet",
    "macro_broken_rune_hatchet",
];

const COW: u16 = 12362;
/// The worn inventory's weapon slot.
const RIGHTHAND: u16 = 3;

fn param_id(engine: &rs_engine::Engine, name: &str) -> TypeId {
    engine
        .cache
        .params
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no param {name}"))
        .id
}

#[test]
fn every_renamed_weapon_has_a_category_and_a_defend_anim() {
    runner::on_loaded_engine(|engine| {
        let defend = param_id(engine, "defend_anim");
        let mut missing = Vec::new();
        for name in RENAMED {
            let obj = engine
                .cache
                .objs
                .get_by_debugname(name)
                .unwrap_or_else(|| panic!("no obj {name}"));
            let has_defend = obj
                .params
                .as_deref()
                .is_some_and(|p| p.get(&defend).is_some());
            let has_attack = ["stabattack", "slashattack", "crushattack", "attackrate"]
                .iter()
                .any(|p| {
                    let id = param_id(engine, p);
                    obj.params
                        .as_deref()
                        .is_some_and(|ps| ps.get(&id).is_some())
                });
            if !has_defend && !has_attack {
                missing.push(*name);
            }
        }
        assert!(
            missing.is_empty(),
            "renamed objs without combat params: {missing:?}"
        );
    });
}

#[test]
fn the_bronze_hatchet_hacks_and_blocks_as_an_axe() {
    runner::on_loaded_engine(|engine| {
        let obj = engine.cache.objs.get_by_debugname("bronze_axe").unwrap();
        let params = obj.params.as_deref().expect("the hatchet has params");
        for (param, seq) in [
            ("slashattack_anim", "human_axe_hack"),
            ("crushattack_anim", "human_blunt_pound"),
            ("defend_anim", "human_axe_block"),
        ] {
            let want = engine.cache.seqs.get_by_debugname(seq).expect(seq).id;
            let got = params.get(&param_id(engine, param));
            assert!(
                matches!(got, Some(ParamValue::Int(v)) if *v == want.index() as i32),
                "{param} is {got:?}, not {seq}"
            );
        }
    });
}

/// A player wielding a bronze hatchet attacks a field cow and plays the axe's
/// hack, never the unarmed punch.
#[test]
fn a_player_with_a_hatchet_attacks_with_it() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let _ = engine.script_errors.take();
        let mut client = engine.harness_login_returning("hatchet").expect("login");
        engine.cycle();
        // Wielded as the client does (Wield, `IF_BUTTON2` on backpack slot 0): the wield
        // trigger is what sets the player's attack and defend anims (`%com_attackanim`).
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let worn = engine.cache.invs.get_by_debugname("worn").unwrap().id;
        let hatchet = engine.cache.objs.get_by_debugname("bronze_axe").unwrap().id;
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .expect("backpack")
            .set(0, hatchet, 1);
        client.if_button(2, BACKPACK, 0, hatchet);
        engine.cycle();
        assert!(
            engine
                .harness_player(client.pid)
                .and_then(|p| p.invs.get(&worn))
                .is_some_and(|w| w.has_at(RIGHTHAND, hatchet)),
            "the hatchet is wielded"
        );
        let (nid, x, z) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .map(|n| (n.npc.uid.nid(), n.npc.uid.id(), n.npc.pathing.coord))
            .find(|(_, id, c)| {
                *id == TypeId::new(COW)
                    && (3256..3265).contains(&c.x())
                    && (3256..3297).contains(&c.z())
            })
            .map(|(nid, _, c)| (nid, c.x(), c.z()))
            .expect("a cow in the Lumbridge field");
        engine.harness_telejump(client.pid, x - 2, 0, z);
        for _ in 0..10 {
            engine.cycle();
        }
        // Alive for the whole fight.
        engine.get_npc_mut(nid).unwrap().npc.stats.levels[NpcStat::Hitpoints as usize] = 3000;
        set_recording(true);
        forget_calls();
        client.op_npc(2, nid);
        let mut played = Vec::new();
        for _ in 0..20 {
            engine.cycle();
            client.packets();
            played.extend(calls_for("anim"));
            forget_calls();
        }
        set_recording(false);
        // A recorded `anim` call names its seq by debugname or by id.
        let is = |s: &String, name: &str| {
            let id = engine
                .cache
                .seqs
                .get_by_debugname(name)
                .unwrap()
                .id
                .index()
                .to_string();
            s.split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|part| part == name || part == id)
        };
        let (hack, punch) = ("human_axe_hack", "human_unarmedpunch");
        assert!(
            played.iter().any(|s| is(s, hack)),
            "the axe hack plays: {played:?}"
        );
        assert!(
            !played.iter().any(|s| is(s, punch)),
            "no unarmed punch: {played:?}"
        );
        assert!(engine.script_errors.take().is_empty(), "no script errors");
    });
}
