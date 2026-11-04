//! Agility from 289, with 2011's stances.
//!
//! 289 sets each base animation by seq while a player balances or climbs;
//! 2011's appearance carries one render anim (BAS), which pairs a ready and a
//! walk seq, so 289's log-balance seqs (763, 762) are 2011's BAS 155
//! (`void-634`'s `rope_balance`). The case walks the Gnome Stronghold course's
//! log (loc 2295) as the client clicks it (OPLOC1: `ip2(z) ip2(loc) p1(ctrl)
//! p2_alt1(x)`): the player crosses seven tiles south in the balance stance,
//! takes 7.5 xp, and stands as before on the far side.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const AGILITY: usize = 16;

/// 289's `log_balance`, the course's first obstacle, at 2474,3435.
const LOG: (u16, u16, u16) = (2295, 2474, 3435);
const START: (u16, u16) = (2474, 3436);
const END: (u16, u16) = (2474, 3429);
const BALANCE_BAS: u16 = 155;
const HUMAN_BAS: u16 = 1426;

#[test]
fn the_gnome_course_log_in_2011s_balance_stance() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("acrobat").expect("login");
        engine.cycle();
        engine.harness_telejump(client.pid, START.0, 0, START.1);
        engine.cycle();
        engine.cycle();
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[AGILITY];
        client.packets();

        let (loc, x, z) = LOG;
        let mut p = ClientPacket::new();
        p.ip2(z as i32).ip2(loc as i32).p1(0).p2_alt1(x as i32);
        client.send(ClientProt::OpLoc1, &p.finish());

        let mut said = Vec::new();
        let mut stances = Vec::new();
        for _ in 0..20 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, t)| t),
            );
            let player = engine.harness_player(client.pid).unwrap();
            stances.push(player.info.readyanim);
            let c = player.pathing.coord;
            if (c.x(), c.z()) == END && said.len() >= 2 {
                break;
            }
        }
        let player = engine.harness_player(client.pid).unwrap();
        let c = player.pathing.coord;
        assert_eq!((c.x(), c.z()), END, "the game said {said:?}");
        assert_eq!(
            said,
            [
                "You walk carefully across the slippery log...",
                "...You make it safely to the other side.",
            ]
        );
        assert_eq!(player.stats.xp[AGILITY] - xp, 75);
        assert!(
            stances.contains(&Some(BALANCE_BAS)),
            "balanced in BAS {BALANCE_BAS}; stances {stances:?}"
        );
        assert_eq!(player.info.readyanim, Some(HUMAN_BAS), "stands as before");
    });
}
