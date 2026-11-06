//! A 2011 fight: a player attacks a cow in Lumbridge's field.
//!
//! 289's combat counts whole hitpoints; 2011's counts life points, ten to a
//! hitpoint and granular. 2011's field cow (12362) is void-634's clone of
//! 289's cow (81), and keeps its 8 hitpoints as 80 life points,
//! and an unarmed level-1 player's max hit is 2011's 5 + S/64 (2011 wiki,
//! Maximum melee hit, 2011-05-20; void-634 Damage.kt) with S = 9 * 64
//! (effective strength 9, bonus 0): 14 life points, where 289's whole
//! hitpoints would allow only 10. The case clicks Attack on the cow as the
//! client does (`OPNPC2`: `g1_alt2(ctrl) g2_alt3(npc index)`) and
//! waits for the first hit to land, then checks it took life points and gave
//! 289's 4 xp per hitpoint, which is 0.4 xp (4 tenths) per life point, in
//! Attack.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_pack::ParamValue;
use rs_pack::types::NpcStat;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use std::collections::HashMap;

const COW: u16 = 12362;
const ATTACK: usize = 0;
const HITPOINTS: usize = 3;
/// 2011's xp counter (varp 1801): `~givexp` adds ten to it for each xp it pays.
const XP_COUNTER: u16 = 1801;
const COW_LIFE_POINTS: u16 = 80;
const UNARMED_MAX_HIT: u16 = 14;
/// 2011's field cow covers 2x2 tiles (its cache config's `size`).
const COW_SIZE: u16 = 2;

#[test]
fn a_fight_runs_on_life_points() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("fighter").expect("login");
        engine.cycle();

        // A cow spawned in the field east of Lumbridge castle.
        let (nid, x, z) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .map(|n| (n.npc.uid.nid(), n.npc.uid.id(), n.npc.pathing.coord))
            .find(|(_, id, c)| {
                *id == COW && (3254..3265).contains(&c.x()) && (3256..3297).contains(&c.z())
            })
            .map(|(nid, _, c)| (nid, c.x(), c.z()))
            .expect("a cow in the Lumbridge field");
        let hp = |engine: &rs_engine::Engine| {
            engine.get_npc(nid).unwrap().npc.stats.levels[NpcStat::Hitpoints as usize]
        };
        assert_eq!(
            hp(engine),
            COW_LIFE_POINTS,
            "the cow has 289's 8 hitpoints as life points"
        );

        engine.harness_telejump(client.pid, x - 2, 0, z);
        // 289's single-combat check counts a fight as eight ticks long from
        // `%lastcombat`, 0 for a new player; a reset world's clock starts near
        // 0, so the first eight ticks read as "I'm already under attack!".
        for _ in 0..10 {
            engine.cycle();
        }
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[ATTACK];
        let hp_xp = engine.harness_player(client.pid).unwrap().stats.xp[HITPOINTS];
        let counter = engine
            .harness_player(client.pid)
            .unwrap()
            .vars
            .get(XP_COUNTER)
            .as_int();

        let mut p = ClientPacket::new();
        p.p1_alt2(0).p2_alt3(nid as i32);
        client.send(ClientProt::OpNpc2, &p.finish());

        let mut ticks = 0;
        while hp(engine) == COW_LIFE_POINTS && ticks < 60 {
            engine.cycle();
            ticks += 1;
        }
        let dealt = COW_LIFE_POINTS - hp(engine);
        assert!(
            (1..=UNARMED_MAX_HIT).contains(&dealt),
            "a hit of 1 to {UNARMED_MAX_HIT} life points landed within {ticks} ticks; it took {dealt}"
        );
        let gained = engine.harness_player(client.pid).unwrap().stats.xp[ATTACK] - xp;
        assert_eq!(gained, dealt as i32 * 4, "4 tenths of xp per life point");
        // 2011's xp counter (varp 1801) counts both, ten for each xp `~givexp` pays.
        let player = engine.harness_player(client.pid).unwrap();
        let hp_gained = player.stats.xp[HITPOINTS] - hp_xp;
        assert!(hp_gained > 0, "Constitution xp for the hit");
        assert_eq!(
            player.vars.get(XP_COUNTER).as_int() - counter,
            (gained + hp_gained) * 10,
            "2011's xp counter counts the fight's xp"
        );
    });
}

/// A 647 content file, from the repo root.
fn content(path: &str) -> String {
    let path = format!("{}/../../content/647/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).expect(&path)
}

/// A 647 pack registry (`content/647/pack/<kind>.pack`): name to id.
fn pack_ids(kind: &str) -> HashMap<String, i32> {
    content(&format!("pack/{kind}.pack"))
        .lines()
        .filter_map(|l| l.split_once('='))
        .filter_map(|(id, n)| Some((n.to_string(), id.parse().ok()?)))
        .collect()
}

/// The id a 647 pack registry gives a name; 647's param and seq configs carry
/// no debugname.
fn pack_id(kind: &str, name: &str) -> i32 {
    *pack_ids(kind)
        .get(name)
        .unwrap_or_else(|| panic!("{name} is not in {kind}.pack"))
}

/// 2011's field cow wears 2011's cow model and fights with its animations and
/// sounds, not 289's: void-634's `cow.combat.toml` gives attack 5849, defend
/// 5850 and death 5851, which 647's registry names after 289's
/// (`cow_attack`, ...), and sounds 369-371. 289's cow made no sound.
#[test]
fn the_field_cow_fights_with_2011s_animations_and_sounds() {
    runner::on_loaded_engine(|engine| {
        let params = engine
            .cache
            .npcs
            .get_by_id(COW)
            .and_then(|n| n.params.as_deref())
            .expect("the field cow has params");
        for (param, kind, id, name) in [
            ("attack_anim", "seq", 5849, "cow_attack"),
            ("defend_anim", "seq", 5850, "cow_block"),
            ("death_anim", "seq", 5851, "cow_death"),
            ("attack_sound", "synth", 369, "cow_attack"),
            ("defend_sound", "synth", 371, "cow_hit"),
            ("death_sound", "synth", 370, "cow_death"),
        ] {
            let value = params.get(&pack_id("param", param));
            assert!(
                matches!(value, Some(ParamValue::Int(v)) if *v == id),
                "{param} is {value:?}, not {id}"
            );
            assert_eq!(pack_id(kind, name), id, "{kind} {name}");
        }
    });
}

/// How far a tile lies outside a square on each axis (0 inside its span).
fn outside(p: u16, low: u16, size: u16) -> u16 {
    if p < low {
        low - p
    } else {
        p.saturating_sub(low + size - 1)
    }
}

/// A player attacking a cow stands beside all of its 2x2 tiles, never on one:
/// 289's op reach for an npc is the tile edge-adjacent to its whole square
/// (`reached`, shape -2). Coming from the east, a player who treated the cow
/// as one tile would stop on its eastern half.
#[test]
fn the_player_fights_beside_the_cow_not_inside_it() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("beside").expect("login");
        engine.cycle();
        let (nid, x, z) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .map(|n| (n.npc.uid.nid(), n.npc.uid.id(), n.npc.pathing.coord))
            .find(|(_, id, c)| {
                *id == COW && (3254..3262).contains(&c.x()) && (3256..3297).contains(&c.z())
            })
            .map(|(nid, _, c)| (nid, c.x(), c.z()))
            .expect("a cow in the Lumbridge field");
        engine.harness_telejump(client.pid, x + COW_SIZE + 1, 0, z);
        // As in `a_fight_runs_on_life_points`: past `%lastcombat`'s 8 ticks.
        for _ in 0..10 {
            engine.cycle();
        }

        let mut p = ClientPacket::new();
        p.p1_alt2(0).p2_alt3(nid as i32);
        client.send(ClientProt::OpNpc2, &p.finish());

        let hp = |engine: &rs_engine::Engine| {
            engine.get_npc(nid).unwrap().npc.stats.levels[NpcStat::Hitpoints as usize]
        };
        for tick in 0..60 {
            engine.cycle();
            let pc = engine.harness_player(client.pid).unwrap().pathing.coord;
            let nc = engine.get_npc(nid).unwrap().npc.pathing.coord;
            let (dx, dz) = (
                outside(pc.x(), nc.x(), COW_SIZE),
                outside(pc.z(), nc.z(), COW_SIZE),
            );
            assert!(
                dx + dz > 0,
                "tick {tick}: the player at {},{} stands on the cow's 2x2 at {},{}",
                pc.x(),
                pc.z(),
                nc.x(),
                nc.z()
            );
            if hp(engine) < COW_LIFE_POINTS {
                assert_eq!(
                    dx + dz,
                    1,
                    "the hit lands from a tile edge-adjacent to the cow's square, \
                     not {dx},{dz} off it"
                );
                return;
            }
        }
        panic!("no hit landed within 60 ticks");
    });
}

/// The server has every npc's size and combat level, as the client does: the
/// packer writes both to the server's npc config too (Jordan's original).
/// Without them every npc paths and collides as one tile, and a hunt's "not
/// too strong" check reads combat level 0.
#[test]
fn the_server_knows_every_npcs_size_and_combat_level() {
    runner::on_loaded_engine(|engine| {
        let ids = pack_ids("npc");
        let mut checked = 0;
        let mut wrong = Vec::new();
        let unpack = content("scripts/_unpack/all.npc");
        for section in unpack.split("\n[") {
            let section = section.trim_start_matches('[');
            let Some((name, body)) = section.split_once("]\n") else {
                continue;
            };
            let Some(&id) = ids.get(name) else {
                continue;
            };
            // A 2011 config can give a key twice (traiborn's combat level
            // is 107, then hidden); the last one read wins.
            let key = |k: &str| {
                body.lines()
                    .filter_map(|l| l.strip_prefix(k).and_then(|v| v.strip_prefix('=')))
                    .next_back()
            };
            let size: u8 = key("size").map_or(1, |v| v.parse().unwrap());
            let vislevel: Option<u16> =
                key("vislevel").map(|v| if v == "hide" { 0 } else { v.parse().unwrap() });
            let npc = engine.cache.npcs.get_by_id(id as u16).expect(name);
            if npc.size != size || npc.vislevel != vislevel {
                wrong.push(format!(
                    "{name} ({id}): size {} vislevel {:?}, the cache's {size} {vislevel:?}",
                    npc.size, npc.vislevel
                ));
            }
            checked += 1;
        }
        assert!(checked > 10_000, "checked only {checked} npcs");
        assert!(
            wrong.is_empty(),
            "{} npcs differ from the cache:\n  {}",
            wrong.len(),
            wrong[..wrong.len().min(20)].join("\n  ")
        );
    });
}
