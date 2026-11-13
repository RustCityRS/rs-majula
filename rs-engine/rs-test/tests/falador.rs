//! 289's Falador at 647 (`areas/area_falador`): every way through its
//! shopkeepers' and Sir Vyvin's Talk-to says something and fails no script, and
//! the Mining Guild's door in the Dwarven Mine wants Mining 60.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{explore, talk_to};
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::sync::Mutex;

/// Every engine in this binary shares rs-pathfinder's process-static
/// collision map: the cases take turns.
static TURN: Mutex<()> = Mutex::new(());

const MINING: usize = 14;
/// The Mining Guild's door (2112) in the Dwarven Mine, as 289 and 2011 place it.
const GUILD_DOOR: (u16, u16, u16) = (2112, 3046, 9756);

/// A spawn of npc `name`: its nid, and a tile beside it that one step from
/// the npc reaches (npcs wander).
fn beside(engine: &Engine, name: &str) -> (u16, u16, u8, u16) {
    let id = *engine
        .cache
        .npcs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("npc.pack names {name}"));
    let (nid, nx, y, nz) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == id)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.y(), c.z())
        })
        .unwrap_or_else(|| panic!("{name} is spawned"));
    let (sx, sz) = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find(|&(dx, dz)| {
            rsmod::can_travel(y, nx, nz, dx, dz, 1, 0, CollisionType::Normal)
                && !rsmod::is_flagged(
                    nx.wrapping_add_signed(dx as i16),
                    nz.wrapping_add_signed(dz as i16),
                    y,
                    0x100 | 0x200000,
                )
        })
        .map(|(dx, dz)| {
            (
                nx.wrapping_add_signed(dx as i16),
                nz.wrapping_add_signed(dz as i16),
            )
        })
        .unwrap_or_else(|| panic!("a tile beside {name} at {nx},{nz}"));
    (nid, sx, y, sz)
}

#[test]
fn falador_s_npcs_answer_every_branch() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        for npc in [
            "cassie",
            "drogo",
            "flynn",
            "herquin",
            "nurmof",
            "sir_vyvin",
            "wayne",
            "wyson",
        ] {
            let _ = engine.script_errors.take();
            let branches = explore(
                engine,
                |engine| {
                    engine.harness_reset();
                    let mut client: HarnessClient = engine
                        .harness_login_returning("falador_visitor")
                        .expect("login");
                    for _ in 0..10 {
                        engine.cycle();
                    }
                    let (nid, x, y, z) = beside(engine, npc);
                    engine.harness_telejump(client.pid, x, y, z);
                    for _ in 0..3 {
                        engine.cycle();
                    }
                    client.packets();
                    talk_to(&mut client, nid);
                    client
                },
                24,
            );
            let errors: Vec<String> = engine
                .script_errors
                .take()
                .into_iter()
                .map(|e| format!("{e:?}"))
                .collect();
            assert!(errors.is_empty(), "{npc}: {errors:?}");
            assert!(!branches.is_empty(), "{npc}");
            for branch in &branches {
                assert!(
                    !branch.conversation.texts.is_empty(),
                    "{npc}, path {:?}: nothing said ({:?})",
                    branch.path,
                    branch.conversation.messages
                );
            }
        }
    });
}

/// `OPLOC1`: `p2_alt1(z) p2_alt1(loc) p1(ctrl) p2_alt2(x)`.
fn op_loc1(client: &mut HarnessClient, (loc, x, z): (u16, u16, u16)) {
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

/// 289's door: below Mining 60 the guild's dwarf turns the player away; at 60
/// it opens (to 289's loc_1532, which 2011's config lacks: mining_guild.loc).
#[test]
fn the_mining_guild_door_wants_mining_60() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // 289's open door, loc_1532, somewhere about the doorway.
        let door_open = |engine: &Engine| {
            (GUILD_DOOR.1 - 1..=GUILD_DOOR.1 + 1).any(|x| {
                (GUILD_DOOR.2 - 1..=GUILD_DOOR.2 + 1).any(|z| {
                    engine
                        .zones
                        .zone(x, 0, z)
                        .is_some_and(|zone| zone.get_loc(x, z, 1532).is_some())
                })
            })
        };
        for (level, opens) in [(1, false), (60, true)] {
            engine.harness_reset();
            let mut client = engine
                .harness_login_returning(&format!("guild_miner_{level}"))
                .expect("login");
            engine.cycle();
            let player = engine.harness_player_mut(client.pid).unwrap();
            player.stats.levels[MINING] = level;
            player.stats.base_levels[MINING] = level;
            engine.harness_telejump(client.pid, GUILD_DOOR.1 + 1, 0, GUILD_DOOR.2);
            for _ in 0..3 {
                engine.cycle();
            }
            client.packets();
            let _ = engine.script_errors.take();
            op_loc1(&mut client, GUILD_DOOR);
            let mut opened = false;
            for _ in 0..10 {
                engine.cycle();
                client.packets();
                opened |= door_open(engine);
            }
            assert_eq!(opened, opens, "Mining {level}");
            let errors = engine.script_errors.take();
            assert!(errors.is_empty(), "{errors:?}");
        }
    });
}
