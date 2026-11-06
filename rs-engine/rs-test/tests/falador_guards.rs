//! Falador's 2011 guards fight as 289's guards, with void-634's animations.
//!
//! 2011 re-issued 289's Falador guards under new ids. void-634 records 3229-
//! 3233 as clones of 3228, which takes 289's guard as the one of its name and
//! level; the clones had no config of their own, so they fought with the
//! param default, an unarmed punch, and no 289 stats (user-reported). 3229 is
//! void-634's crossbow guard (`guard_xbow`), 289's `guard2`: it fights at
//! range with void-634's crossbow animation (4230) and a bolt.
//! `OPNPC2`: `g1_alt2(ctrl) g2_alt3(npc index)`.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::ParamValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
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

fn param(engine: &Engine, npc: u16, name: &str) -> Option<i32> {
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

/// Each of Falador's 2011 sword guards fights as 289's guard: its life points
/// and void-634's `guard` animations (stab 386, defend 424, death 836).
#[test]
fn falador_s_sword_guards_fight_as_289_s_guard() {
    runner::on_loaded_engine(|engine| {
        for npc in [3228, 3230, 3231, 3232, 3233, 3241] {
            let t = engine.cache.npcs.get_by_id(npc).expect("a guard");
            assert_eq!(t.hitpoints, 220, "{npc}: 289's 22 hitpoints as life points");
            assert_eq!(param(engine, npc, "attack_anim"), Some(386), "{npc} attack");
            assert_eq!(param(engine, npc, "defend_anim"), Some(424), "{npc} defend");
            assert_eq!(param(engine, npc, "death_anim"), Some(836), "{npc} death");
        }
    });
}

/// void-634's crossbow guard (3229) is 289's guard2: ranged, void-634's
/// crossbow animation, the bolt.
#[test]
fn falador_s_crossbow_guard_is_289_s_guard2() {
    runner::on_loaded_engine(|engine| {
        let t = engine
            .cache
            .npcs
            .get_by_id(3229)
            .expect("the crossbow guard");
        assert_eq!(t.hitpoints, 220);
        assert_eq!(param(engine, 3229, "attack_anim"), Some(4230));
        assert_eq!(param(engine, 3229, "proj_travel"), Some(27));
    });
}

/// In a fight the crossbow guard shoots back: the player takes its bolts. It
/// launches none (289's spotanim 28 is another effect at 2011), and a null
/// `spotanim_npc` ended 289's ranged attack in a script error before it shot.
/// (Its animation is a per-tick info mask the end of the tick clears; the
/// param above is what it plays.)
#[test]
fn the_crossbow_guard_shoots_back() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine
            .harness_login_returning("guardfighter")
            .expect("login");
        let (nid, x, z) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .find(|n| n.npc.base_type == 3229)
            .map(|n| {
                (
                    n.npc.uid.nid(),
                    n.npc.pathing.coord.x(),
                    n.npc.pathing.coord.z(),
                )
            })
            .expect("a crossbow guard");
        engine.harness_telejump(client.pid, x, 0, z + 3);
        // Past 289's "under attack" window (`%lastcombat` + 8 ticks).
        for _ in 0..12 {
            engine.cycle();
        }
        let mut p = ClientPacket::new();
        p.p1_alt2(0).p2_alt3(nid as i32);
        client.send(ClientProt::OpNpc2, &p.finish());
        // A player's life points: varp 1240, bits 1-15 (varbit life_points).
        let pid = client.pid;
        let life_points = |engine: &Engine| {
            (engine.harness_player(pid).unwrap().vars.get(1240).as_int() >> 1) & 0x7FFF
        };
        let full = life_points(engine);
        let mut hit = false;
        for _ in 0..60 {
            engine.cycle();
            let said = HarnessClient::game_messages(&client.packets());
            assert!(
                !said.iter().any(|(_, m)| m.starts_with("script error")),
                "no script error: {said:?}"
            );
            if life_points(engine) < full {
                hit = true;
                break;
            }
        }
        assert!(hit, "the guard's bolts took life points within 60 ticks");
        client.packets();
    });
}
