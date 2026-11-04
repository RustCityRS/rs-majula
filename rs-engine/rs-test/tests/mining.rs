//! Mining from 289, at a 2011 rock.
//!
//! 289's rocks are the 2005 ones; Rimmington's copper rock 9708 is 2011's
//! (void-634 `copper_rocks_rimmington_1`), depleting to 9723. The case clicks
//! Mine on it as the client does (OPLOC1, `ip2(z) ip2(loc) p1(ctrl)
//! p2_alt1(x)`) with a bronze pickaxe in the backpack, swings until the rock
//! gives, and then clicks the depleted rock.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const MINING: usize = 14;
const ROCK: (u16, u16, u16) = (9708, 2977, 3247);
const DEPLETED: u16 = 9723;

#[test]
fn a_2011_copper_rock_gives_ore_and_depletes() {
    runner::on_loaded_engine(|engine| {
        let pickaxe = *engine.cache.objs.debugnames.get("bronze_pickaxe").unwrap();
        let ore = *engine.cache.objs.debugnames.get("copper_ore").unwrap();
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();

        engine.harness_reset();
        let mut client = engine.harness_login_returning("miner").expect("login");
        engine.cycle();
        engine.harness_telejump(client.pid, ROCK.1, 0, ROCK.2 + 2);
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&inv).unwrap().set(0, pickaxe, 1);
        let xp = player.stats.xp[MINING];
        engine.cycle();
        client.packets();

        click(&mut client, ROCK.0);
        let mut said = Vec::new();
        for _ in 0..100 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, t)| t),
            );
            let player = engine.harness_player(client.pid).unwrap();
            if player.invs.get(&inv).unwrap().total(ore) > 0 {
                break;
            }
        }
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(
            player.invs.get(&inv).unwrap().total(ore),
            1,
            "one copper ore; the game said {said:?}"
        );
        assert_eq!(player.stats.xp[MINING] - xp, 175, "copper's rock_exp");
        assert_eq!(
            said.first().map(String::as_str),
            Some("You swing your pick at the rock.")
        );
        assert_eq!(
            said.last().map(String::as_str),
            Some("You manage to mine some copper.")
        );

        // The rock is now 9723, which holds nothing.
        click(&mut client, DEPLETED);
        let mut answer = None;
        for _ in 0..10 {
            engine.cycle();
            if let Some((_, t)) = HarnessClient::game_messages(&client.packets()).pop() {
                answer = Some(t);
                break;
            }
        }
        assert_eq!(
            answer.as_deref(),
            Some("There is no ore currently available in this rock.")
        );
    });
}

fn click(client: &mut HarnessClient, loc: u16) {
    let mut p = ClientPacket::new();
    p.ip2(ROCK.2 as i32)
        .ip2(loc as i32)
        .p1(0)
        .p2_alt1(ROCK.1 as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}
