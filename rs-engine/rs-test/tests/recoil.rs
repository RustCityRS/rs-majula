//! A ring of recoil at 647: 2011 pays recoil in whole hitpoints, ten life points each, with the
//! deflect hitmark. Its own binary, so its strong cow leaves no mark on combat_fight's field.

#![cfg(since_647)]

use rs_pack::types::NpcStat;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::life;
use rs_test::runner;
use rs_ver::TypeId;
use rs_vm::ops::test::{calls_for, forget_calls, set_recording};

const COW: u16 = 12362;
const COW_LIFE_POINTS: u16 = 80;
const HITPOINTS: usize = 3;
/// A ring of recoil pays back each hit as 2011 does: in the 2011-04-21 recording (YouTube
/// LJsjL-Lu9cQ, 147.9-154.1s) the Count's hits of 20, 2, 21 and 23 each send 10 back onto him,
/// in the same tick, with 2011's deflect icon (hitmark 4; the 2011 wiki's Hitsplat page lists
/// the Ring of Recoil under "Deflected Attacks"). That is 289's recoil, a tenth of the hit plus
/// one hitpoint, counted in hitpoints and paid in life points: 10 for any hit under 100.
#[test]
fn a_ring_of_recoil_sends_back_2011s_ten() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("recoiler").expect("login");
        engine.cycle();
        let (nid, x, z) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .find(|n| {
                let c = n.npc.pathing.coord;
                n.npc.uid.id() == TypeId::new(COW)
                    // The player stands 2 west: the field's west fence is on x = 3252.
                    && (3256..3265).contains(&c.x())
                    && (3256..3297).contains(&c.z())
                    && n.npc.stats.levels[NpcStat::Hitpoints as usize] == COW_LIFE_POINTS
            })
            .map(|n| {
                (
                    n.npc.uid.nid(),
                    n.npc.pathing.coord.x(),
                    n.npc.pathing.coord.z(),
                )
            })
            .expect("an unhurt cow in the Lumbridge field");
        let worn = engine.cache.invs.get_by_debugname("worn").unwrap().id;
        let ring = engine
            .cache
            .objs
            .get_by_debugname("ring_of_recoil")
            .unwrap()
            .id;
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&worn).unwrap().set(12, ring, 1);
        // Enough life to take a strong cow's bites.
        player.stats.levels[HITPOINTS] = 99;
        player.stats.base_levels[HITPOINTS] = 99;
        life::set(player, 99 * life::SCALE);
        engine.harness_telejump(client.pid, x - 2, 0, z);
        for _ in 0..10 {
            engine.cycle();
        }
        // A cow that bites hard enough for hits over 10, where 289's +1 hitpoint and 2011's differ.
        let cow = &mut engine.get_npc_mut(nid).unwrap().npc.stats.levels;
        cow[NpcStat::Hitpoints as usize] = 3000;
        cow[NpcStat::Attack as usize] = 99;
        cow[NpcStat::Strength as usize] = 99;
        set_recording(true);
        forget_calls();
        client.op_npc(2, nid);
        let mut on_cow = Vec::new();
        // Ticks where a recoil landed with no bite on the player in that same tick.
        // A bite's own damage call, not the life total: regen can refill a bite of 1 that tick.
        let mut late = Vec::new();
        for tick in 0..80 {
            engine.cycle();
            client.packets();
            if tick % 20 == 0 {
                client.send(ClientProt::NoTimeout, &[]);
            }
            let bit = calls_for("damage")
                .iter()
                .any(|a| a.split(',').nth(1).and_then(|v| v.parse::<i32>().ok()) > Some(0));
            let calls = calls_for("npc_damage");
            if calls.iter().any(|a| a.starts_with("4,")) && !bit {
                late.push(tick);
            }
            on_cow.extend(calls);
            forget_calls();
        }
        set_recording(false);
        // The 2011-04-21 recording's 10s show 0.07-0.2s after each of the Count's hits: the
        // same tick (0.6s).
        assert!(
            late.is_empty(),
            "recoils a tick after their bite, at ticks {late:?}"
        );
        // "type,amount" for each hit on the cow: its recoils carry the deflect hitmark (4).
        let recoils: Vec<i32> = on_cow
            .iter()
            .filter(|a| a.starts_with("4,"))
            .filter_map(|a| a.split(',').nth(1)?.parse().ok())
            .collect();
        assert!(!recoils.is_empty(), "the cow's bites recoil: {on_cow:?}");
        // Whole hitpoints, paid as life points: a multiple of ten, 10 for a bite under 100.
        assert!(
            recoils.iter().all(|r| r % 10 == 0) && recoils.contains(&10),
            "2011's recoil is a whole hitpoint per started hundred: {recoils:?}"
        );
    });
}
