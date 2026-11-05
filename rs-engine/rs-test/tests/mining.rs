//! Mining from 289, at a 2011 rock.
//!
//! 289's rocks are the 2005 ones; Rimmington's copper rock 9708 is 2011's
//! (void-634 `copper_rocks_rimmington_1`), depleting to 9723 (void-634's
//! `depleted_rocks_rimmington_1`). The case clicks Mine on it as the client
//! does (OPLOC1, `ip2(z) ip2(loc) p1(ctrl) p2_alt1(x)`) with a bronze pickaxe
//! in the backpack, swings until the rock gives, and then clicks the depleted
//! rock.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const MINING: usize = 14;
/// 2011's xp counter (varp 1801): `~givexp` adds ten to it for each xp it pays.
const XP_COUNTER: u16 = 1801;
const ROCK: (u16, u16, u16) = (9708, 2977, 3247);
const DEPLETED: u16 = 9723;

#[test]
fn a_2011_copper_rock_gives_ore_and_depletes() {
    mine(ROCK, DEPLETED, "copper_ore", 1, 175, "copper");
}

/// 2011's reworked Lumbridge Swamp rocks, the only mines with them: the
/// south-east mine's copper (3027, shape 64231, bare 3281) and the
/// south-west mine's coal (3233, shape 64233, bare 3227). 289's rocks never
/// stood there, so nothing answered them ("Nothing interesting happens").
#[test]
fn the_lumbridge_swamp_south_east_mine_gives_copper() {
    mine((3027, 3230, 3147), 3281, "copper_ore", 1, 175, "copper");
}

#[test]
fn the_lumbridge_swamp_south_west_mine_gives_coal() {
    mine((3233, 3144, 3148), 3227, "coal", 30, 500, "coal");
}

/// Mines `rock` at level `level` with a bronze pickaxe until it gives one
/// `ore`, then clicks the bare rock it leaves.
fn mine(rock: (u16, u16, u16), depleted: u16, ore: &str, level: u16, exp: i32, said_ore: &str) {
    // The cases share one loaded engine and each starts with a reset, which
    // would log the others' miners out: one at a time.
    static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let ore_name = ore.to_string();
    let said_ore = said_ore.to_string();
    runner::on_loaded_engine(move |engine| {
        let pickaxe = *engine.cache.objs.debugnames.get("bronze_pickaxe").unwrap();
        let ore = *engine.cache.objs.debugnames.get(ore_name.as_str()).unwrap();
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();

        engine.harness_reset();
        let mut client = engine
            .harness_login_returning(&format!("miner{}", rock.0))
            .expect("login");
        engine.cycle();
        // Beside the rock, on a tile with no loc or blocked floor: the swamp
        // mines' rocks stand in clusters at the water's edge.
        let (sx, sz) = [(0, 1), (1, 0), (0, -1), (-1, 0)]
            .into_iter()
            .map(|(dx, dz)| {
                (
                    rock.1.wrapping_add_signed(dx),
                    rock.2.wrapping_add_signed(dz),
                )
            })
            .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
            .expect("a walkable tile beside the rock");
        engine.harness_telejump(client.pid, sx, 0, sz);
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&inv).unwrap().set(0, pickaxe, 1);
        player.stats.levels[MINING] = level;
        player.stats.base_levels[MINING] = level;
        let xp = player.stats.xp[MINING];
        let counter = player.vars.get(XP_COUNTER).as_int();
        engine.cycle();
        client.packets();

        click(&mut client, rock);
        let mut said = Vec::new();
        for tick in 0..200 {
            // The client's keep-alive, as the 2011 client sends it: a rock
            // that takes over 100 ticks to give would log a silent miner out
            // (TIMEOUT_NO_RESPONSE, phases/logout.rs).
            if tick % 20 == 19 {
                client.send(ClientProt::NoTimeout, &[]);
            }
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, t)| t),
            );
            let player = engine.harness_player(client.pid).unwrap_or_else(|| {
                panic!("the miner at {} is gone; the game said {said:?}", rock.0)
            });
            if player.invs.get(&inv).unwrap().total(ore) > 0 {
                break;
            }
        }
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(
            player.invs.get(&inv).unwrap().total(ore),
            1,
            "one {ore_name} from {}; the game said {said:?}",
            rock.0
        );
        assert_eq!(player.stats.xp[MINING] - xp, exp, "{ore_name}'s rock_exp");
        assert_eq!(
            player.vars.get(XP_COUNTER).as_int() - counter,
            exp * 10,
            "2011's xp counter counts the {ore_name}"
        );
        assert_eq!(
            said.first().map(String::as_str),
            Some("You swing your pick at the rock.")
        );
        assert_eq!(
            said.last().map(String::as_str),
            Some(format!("You manage to mine some {said_ore}.").as_str())
        );

        // The rock is now bare and holds nothing.
        click(&mut client, (depleted, rock.1, rock.2));
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

fn click(client: &mut HarnessClient, (loc, x, z): (u16, u16, u16)) {
    let mut p = ClientPacket::new();
    p.ip2(z as i32).ip2(loc as i32).p1(0).p2_alt1(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}
