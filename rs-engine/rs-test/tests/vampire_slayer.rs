//! Vampire Slayer at 647 (`quests/quest_vampire`, Morgan, Dr Harlow): 289's
//! quest as 2011 has it. Dr Harlow gives a stake and a stake hammer for a
//! beer, the Count rises from 2011's coffin and is staked with the stake
//! hammer, and the quest ends at Morgan, taking every stake.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_pack::types::NpcStat;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{converse, explore, talk_to};
use rsmod::rsmod::collision::collision_strategy::CollisionType;

/// Every engine in this binary shares rs-pathfinder's process-static
/// collision map: the cases take turns.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 2011's coffin in Draynor Manor's basement (`l48_152`: 0 8 57, 3x3).
const COFFIN: (u16, u16, u16) = (158, 3080, 9785);
const ATTACK: usize = 0;

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("obj.pack names {name}"))
}

fn varp(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .varps
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("varp.pack names {name}"))
}

fn npc_type(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .npcs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("npc.pack names {name}"))
}

/// A spawn of npc `name`: its nid, and a tile beside it that one step from
/// the npc reaches.
fn beside(engine: &Engine, name: &str) -> (u16, u16, u8, u16) {
    let id = npc_type(engine, name);
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

fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) {
    for _ in 0..n {
        engine.cycle();
        client.packets();
    }
}

/// Logs in at (x, level, z) with %vampire at `stage`, %vampire_count_slain at
/// `slain` and `items` in the backpack.
fn login(
    engine: &mut Engine,
    (x, level, z): (u16, u8, u16),
    stage: i32,
    slain: i32,
    items: &[(&str, i32)],
) -> HarnessClient {
    let (quest, count) = (varp(engine, "vampire"), varp(engine, "vampire_count_slain"));
    let backpack = *engine.cache.invs.debugnames.get("inv").unwrap();
    let items: Vec<(u16, i32)> = items.iter().map(|&(n, c)| (obj(engine, n), c)).collect();
    engine.harness_reset();
    let mut client = engine
        .harness_login_returning("vampire_slayer")
        .expect("login");
    for _ in 0..10 {
        engine.cycle();
    }
    engine.harness_telejump(client.pid, x, level, z);
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.vars.set(quest, VarValue::Int(stage));
    player.vars.set(count, VarValue::Int(slain));
    let pack = player.invs.get_mut(&backpack).unwrap();
    for (slot, &(o, n)) in items.iter().enumerate() {
        pack.set(slot as u16, o, n as u32);
    }
    ticks(engine, &mut client, 3);
    client
}

fn visit(
    engine: &mut Engine,
    npc: &str,
    stage: i32,
    slain: i32,
    items: &[(&str, i32)],
) -> HarnessClient {
    let (nid, x, y, z) = beside(engine, npc);
    let mut client = login(engine, (x, y, z), stage, slain, items);
    talk_to(&mut client, nid);
    client
}

fn var(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let id = varp(engine, name);
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(id)
        .as_int()
}

fn total(engine: &Engine, client: &HarnessClient, inv: &str, name: &str) -> i32 {
    let inv = *engine.cache.invs.debugnames.get(inv).unwrap();
    let o = obj(engine, name);
    engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .map_or(0, |i| i.total(o) as i32)
}

fn no_script_errors(engine: &mut Engine, what: &str) {
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "{what}: {errors:?}");
}

/// An npc, %vampire, %vampire_count_slain and the backpack.
type Case = (&'static str, i32, i32, &'static [(&'static str, i32)]);

/// Morgan and Dr Harlow at each step say something down every branch, and
/// fail no script.
#[test]
fn morgan_and_harlow_answer_every_branch() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let cases: &[Case] = &[
            ("morgan", 0, 0, &[]),
            ("morgan", 1, 0, &[]),
            ("morgan", 2, 0, &[]),
            ("morgan", 3, 0, &[]),
            ("dr_harlow", 0, 0, &[]),
            ("dr_harlow", 1, 0, &[]),
            ("dr_harlow", 2, 0, &[]),
            ("dr_harlow", 2, 0, &[("beer", 1)]),
            ("dr_harlow", 2, 0, &[("beer", 1), ("stake", 1)]),
            ("dr_harlow", 3, 0, &[]),
        ];
        for &(npc, stage, slain, items) in cases {
            let _ = engine.script_errors.take();
            let branches = explore(engine, |engine| visit(engine, npc, stage, slain, items), 24);
            no_script_errors(engine, &format!("{npc} at {stage}"));
            assert!(!branches.is_empty(), "{npc}");
            for branch in &branches {
                assert!(
                    !branch.conversation.texts.is_empty(),
                    "{npc} at {stage}, path {:?}: nothing said ({:?})",
                    branch.path,
                    branch.conversation.messages
                );
            }
        }
    });
}

/// Morgan sends the player to the Blue Moon Inn, where Dr Harlow takes a beer
/// for a stake and a stake hammer.
#[test]
fn harlow_gives_a_stake_and_a_stake_hammer_for_a_beer() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let _ = engine.script_errors.take();
        let mut client = visit(engine, "morgan", 0, 0, &[]);
        let talk = converse(engine, &mut client, &["Ok, I'm up for an adventure."], 300);
        assert!(talk.said("Blue Moon Inn"), "{:?}", talk.texts);
        assert_eq!(
            var(engine, &client, "vampire") & 3,
            1,
            "the quest is started"
        );

        let mut client = visit(engine, "dr_harlow", 1, 0, &[]);
        let talk = converse(engine, &mut client, &["Morgan needs your help"], 300);
        assert_eq!(talk.unanswered, None, "{:?}", talk.texts);
        assert_eq!(var(engine, &client, "vampire") & 3, 2);

        let mut client = visit(engine, "dr_harlow", 2, 0, &[("beer", 1)]);
        let talk = converse(engine, &mut client, &[], 300);
        no_script_errors(engine, "Harlow");
        assert!(
            talk.said("Dr Harlow hands you a stake hammer."),
            "{:?}",
            talk.texts
        );
        assert_eq!(total(engine, &client, "inv", "beer"), 0);
        assert_eq!(total(engine, &client, "inv", "stake"), 1);
        assert_eq!(total(engine, &client, "inv", "stake_hammer"), 1);
    });
}

/// The coffin raises the Count; beaten, he is staked with the stake hammer
/// (without it he regenerates), and Morgan then ends the quest: 4825 Attack
/// xp, and the stakes go from the backpack and the bank.
#[test]
fn the_count_is_staked_and_morgan_ends_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let _ = engine.script_errors.take();
        let count = npc_type(engine, "count_draynor");
        let find = |engine: &Engine| {
            engine
                .npc_list
                .npcs
                .iter()
                .flatten()
                .find(|n| n.npc.uid.id() == count)
                .map(|n| n.npc.uid.nid())
        };
        let fight = |engine: &mut Engine, client: &mut HarnessClient| -> Vec<String> {
            // Through the setstat cheat, so [changestat] works the max hit
            // out again (a level set directly leaves %com_maxhit as it was).
            for stat in ["attack", "strength", "defence"] {
                let mut p = ClientPacket::new();
                p.p1(0).p1(0).pjstr(&format!("setstat {stat} 99"));
                client.send(ClientProt::ClientCheat, &p.finish());
                ticks(engine, client, 1);
            }
            let player = engine.harness_player_mut(client.pid).unwrap();
            player.stats.levels[3] = 99;
            player.stats.base_levels[3] = 99;
            player.stats.xp[3] = 130_344_310;
            let lp = player.vars.get(1240).as_int();
            player.vars.set(1240, VarValue::Int((lp & 1) | 990 << 1));
            ticks(engine, client, 12);
            let mut p = ClientPacket::new();
            p.p2_alt1(COFFIN.2 as i32)
                .p2_alt1(COFFIN.0 as i32)
                .p1(0)
                .p2_alt2(COFFIN.1 as i32);
            client.send(ClientProt::OpLoc1, &p.finish());
            let mut said = Vec::new();
            let mut risen = false;
            let mut risen_at = 0;
            // The Count's last life points and tile, and the tick he went.
            let mut last: Option<(usize, i32, u16, u16)> = None;
            // Every 10 ticks: the player's life points and tile, the Count's life points.
            let mut trace: Vec<(usize, i32, u16, u16, Option<i32>, i32)> = Vec::new();
            for tick in 0..400 {
                engine.cycle();
                let Some(player) = engine.harness_player(client.pid) else {
                    let errors = engine.script_errors.take();
                    panic!(
                        "the player is gone at tick {tick}, the Count last {last:?}, trace {trace:?}: {said:?} {errors:?}"
                    );
                };
                if tick % 10 == 0 {
                    let c = player.pathing.coord;
                    trace.push((
                        tick,
                        (player.vars.get(1240).as_int() >> 1) & 0x7FFF,
                        c.x(),
                        c.z(),
                        last.map(|l| l.1),
                        player.stats.xp[ATTACK],
                    ));
                }
                said.extend(
                    HarnessClient::game_messages(&client.packets())
                        .into_iter()
                        .map(|(_, m)| m),
                );
                // A harness client sends nothing on its own: without this
                // the engine drops it after 100 silent ticks.
                if tick % 20 == 0 {
                    client.send(ClientProt::NoTimeout, &[]);
                }
                match find(engine) {
                    Some(nid) => {
                        let n = engine.get_npc(nid).unwrap();
                        let c = n.npc.pathing.coord;
                        last = Some((
                            tick,
                            n.npc.stats.levels[NpcStat::Hitpoints as usize] as i32,
                            c.x(),
                            c.z(),
                        ));
                        if !risen {
                            risen = true;
                            risen_at = tick;
                            // A hit or two from beaten: the fight is not what
                            // is under test, and a harness player's max hit
                            // stays its login one (14 life points).
                            engine.get_npc_mut(nid).unwrap().npc.stats.levels
                                [NpcStat::Hitpoints as usize] = 10;
                        }
                        // Attack once the client has seen him, and again
                        // every 30 ticks in case a swing was refused.
                        if tick >= risen_at + 3 && (tick - risen_at - 3) % 30 == 0 {
                            let mut p = ClientPacket::new();
                            p.p1_alt2(0).p2_alt3(nid as i32);
                            client.send(ClientProt::OpNpc2, &p.finish());
                        }
                    }
                    None if risen => break,
                    None => {}
                }
                if said.iter().any(|m| m.contains("regenerate")) {
                    break;
                }
            }
            assert!(risen, "the Count rises: {said:?}");
            let at = engine.harness_player(client.pid).unwrap().pathing.coord;
            said.push(format!(
                "(the Count last at tick, life points, x, z: {last:?}; trace {trace:?}; the player at {},{})",
                at.x(),
                at.z()
            ));
            said
        };

        // No stake hammer: he regenerates.
        let below = (COFFIN.1 + 1, 0, COFFIN.2 - 1);
        // Garlic: without it each miss heals him 10 life points (2011 wiki),
        // more than a level-1 player's hits take off.
        let mut client = login(engine, below, 2, 0, &[("stake", 1), ("garlic", 1)]);
        let said = fight(engine, &mut client);
        assert!(
            said.iter().any(|m| m.contains("unable to push the stake")),
            "{said:?}"
        );
        assert_eq!(var(engine, &client, "vampire_count_slain"), 0);
        no_script_errors(engine, "no hammer");

        // With it, he is staked.
        let mut client = login(
            engine,
            below,
            2,
            0,
            &[("stake", 1), ("stake_hammer", 1), ("garlic", 1)],
        );
        let said = fight(engine, &mut client);
        no_script_errors(engine, "the fight");
        assert!(
            said.iter()
                .any(|m| m == "You hammer the stake into the vampire's chest!"),
            "{said:?}"
        );
        assert_eq!(var(engine, &client, "vampire_count_slain"), 1);
        assert_eq!(
            var(engine, &client, "vampire") & 3,
            2,
            "not over until Morgan hears"
        );

        // Morgan ends it.
        let bank = *engine.cache.invs.debugnames.get("bank").unwrap();
        let stake = obj(engine, "stake");
        let mut client = visit(engine, "morgan", 2, 1, &[("stake", 1)]);
        // A new player's bank comes when it is first opened: one made here,
        // as the engine makes it, holds two stakes.
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .entry(bank)
            .or_insert_with(|| rs_inv::Inventory::with_stack_mode(516, rs_inv::StackMode::Always))
            .set(0, stake, 2);
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[ATTACK];
        let talk = converse(engine, &mut client, &[], 300);
        no_script_errors(engine, "Morgan");
        assert!(
            talk.said("I have slain the foul creature!"),
            "{:?}",
            talk.texts
        );
        assert_eq!(
            var(engine, &client, "vampire") & 3,
            3,
            "the quest is complete"
        );
        assert_eq!(var(engine, &client, "vampire_count_slain"), 0);
        assert_eq!(
            engine.harness_player(client.pid).unwrap().stats.xp[ATTACK] - xp,
            48_250
        );
        assert_eq!(total(engine, &client, "inv", "stake"), 0);
        assert_eq!(
            total(engine, &client, "bank", "stake"),
            0,
            "the banked stakes go too"
        );
    });
}

/// A 647 pack registry's id for `name` (`content/647/pack/<kind>.pack`).
fn pack_id(kind: &str, name: &str) -> i32 {
    let path = format!(
        "{}/../../content/647/pack/{kind}.pack",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path)
        .expect(&path)
        .lines()
        .find_map(|l| {
            let (id, n) = l.split_once('=')?;
            (n == name).then(|| id.parse().unwrap())
        })
        .unwrap_or_else(|| panic!("{kind}.pack names {name}"))
}

/// The frame sets a seq's frames come from, from 647's unpacked cache
/// (`all.seq`: `frameN=<delay>:<frame>:<frame set>`).
fn frame_sets(seq: &str) -> std::collections::BTreeSet<String> {
    let path = format!(
        "{}/../../content/647/scripts/runescript/_unpack/all.seq",
        env!("CARGO_MANIFEST_DIR")
    );
    let all = std::fs::read_to_string(&path).expect(&path);
    let head = format!(
        "[{seq}]
"
    );
    let start = all
        .find(&head)
        .unwrap_or_else(|| panic!("all.seq has {seq}"))
        + head.len();
    let body = &all[start
        ..all[start..]
            .find(
                "
[",
            )
            .map_or(all.len(), |e| start + e)];
    body.lines()
        .filter_map(|l| l.strip_prefix("frame"))
        .filter_map(|l| l.split_once('='))
        .filter_map(|(_, v)| v.rsplit(':').next().map(str::to_string))
        .collect()
}

/// The Count's attack, block and death play on his own skeleton, the frame
/// set his 2011 stand (bas 1669: seq 3330) uses, not a human one (289's punch
/// and block, the default death, vamp_claw), which twists his model.
#[test]
fn the_count_fights_on_his_own_skeleton() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let count = npc_type(engine, "count_draynor");
        let params = engine
            .cache
            .npcs
            .get_by_id(count)
            .and_then(|n| n.params.as_deref())
            .expect("the Count has params");
        let own = frame_sets("seq_3330");
        assert_eq!(own.len(), 1, "his stand is one frame set: {own:?}");
        assert_ne!(own, frame_sets("human_unarmedpunch"));
        for (param, seq) in [
            ("attack_anim", 3331),
            ("defend_anim", 3332),
            ("death_anim", 3112),
        ] {
            let value = params.get(&pack_id("param", param));
            assert!(
                matches!(value, Some(rs_pack::ParamValue::Int(v)) if *v == seq),
                "{param} is {value:?}, not {seq}"
            );
            assert_eq!(frame_sets(&format!("seq_{seq}")), own, "{param} {seq}");
        }
    });
}

/// The client's own opcodes (`ServerProt.java`) for the camera packets.
const CAM_LOOKAT: u16 = 26;
const CAM_RESET: u16 = 67;
const CAM_MOVETO: u16 = 94;

/// A CAM_MOVETO's height, variable speed and constant speed, as its encoder
/// writes them: `p1_alt1 x, p1_alt2 z, p1_alt1 var, p2 height, p1_alt1 const`.
fn moveto(payload: &[u8]) -> (u16, u8, u8) {
    (
        u16::from_be_bytes([payload[3], payload[4]]),
        payload[2].wrapping_sub(128),
        payload[5].wrapping_sub(128),
    )
}

/// A CAM_LOOKAT's height, variable speed and constant speed: `p1_alt2 x,
/// p2_alt3 height, p1_alt3 z, p1_alt2 const, p1_alt3 var`.
fn lookat(payload: &[u8]) -> (u16, u8, u8) {
    (
        u16::from_le_bytes([payload[1].wrapping_sub(128), payload[2]]),
        128u8.wrapping_sub(payload[5]),
        0u8.wrapping_sub(payload[4]),
    )
}

/// Opening the coffin plays 2011's cutscene, timed to the 2010-10-15
/// recording (YouTube TLKwCRHHM6M), whose fades last 2011's default fade:
/// the track unlocked at once, the player's push on the lid at tick 6, the
/// stake failing at tick 10 as he is thrown back a tile, the camera's
/// three fitted moves, the coffin left open and empty as the Count rises on
/// 3081,9777 at tick 21; he stays there, the player's camera comes back when
/// the player goes to attack him, and the coffin closes once he has gone.
#[test]
fn the_coffin_plays_2011s_cutscene() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let _ = engine.script_errors.take();
        let count = npc_type(engine, "count_draynor");
        let mut client = login(
            engine,
            (3079, 0, 9786),
            2,
            0,
            &[("stake", 1), ("garlic", 1)],
        );
        client.packets();
        let mut p = ClientPacket::new();
        p.p2_alt1(COFFIN.2 as i32)
            .p2_alt1(COFFIN.0 as i32)
            .p1(0)
            .p2_alt2(COFFIN.1 as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        let mut said: Vec<(usize, String)> = Vec::new();
        let mut moves: Vec<(usize, (u16, u8, u8))> = Vec::new();
        let mut looks: Vec<(usize, (u16, u8, u8))> = Vec::new();
        let mut risen: Option<(usize, u16, u16, u16)> = None;
        // The seqs the player plays, by tick.
        let mut played: Vec<(usize, String)> = Vec::new();
        rs_vm::ops::test::set_recording(true);
        rs_vm::ops::test::forget_calls();
        for tick in 0..32 {
            engine.cycle();
            for seq in rs_vm::ops::test::calls_for("anim") {
                played.push((tick, seq));
            }
            rs_vm::ops::test::forget_calls();
            let packets = client.packets();
            for (_, m) in HarnessClient::game_messages(&packets) {
                said.push((tick, m));
            }
            for (op, payload) in &packets {
                match *op {
                    CAM_MOVETO => moves.push((tick, moveto(payload))),
                    CAM_LOOKAT => looks.push((tick, lookat(payload))),
                    _ => {}
                }
            }
            if risen.is_none()
                && let Some(n) = engine
                    .npc_list
                    .npcs
                    .iter()
                    .flatten()
                    .find(|n| n.npc.uid.id() == count)
            {
                let c = n.npc.pathing.coord;
                risen = Some((tick, n.npc.uid.nid(), c.x(), c.z()));
            }
            if tick % 20 == 0 {
                client.send(ClientProt::NoTimeout, &[]);
            }
        }
        rs_vm::ops::test::set_recording(false);
        no_script_errors(engine, "the cutscene");
        let at = |text: &str| {
            said.iter()
                .find(|(_, m)| m.contains(text))
                .map(|&(t, _)| t)
                .unwrap_or_else(|| panic!("{text}: {said:?}"))
        };
        let open = at("You have unlocked a new music track: I'm Counting on You.");
        assert!(open <= 1, "the track unlocks as the coffin opens: {said:?}");
        assert_eq!(
            at("You stab the vampire with the stake, but it does not go deep enough.") - open,
            10,
            "{said:?}"
        );
        // May 2011's player: the push on the lid (2991), the stake (2992),
        // thrown back (3108), lying (3109), up again as the Count turns
        // (3110); frame set 2897's.
        let seq = |n: &str| pack_id("seq", n).to_string();
        let played: Vec<_> = played.iter().map(|(t, s)| (t - open, s.clone())).collect();
        assert_eq!(
            played,
            [
                (6, seq("seq_2991")),
                (10, seq("seq_2992")),
                (11, seq("seq_3108")),
                (13, seq("seq_3109")),
                (25, seq("seq_3110"))
            ],
            "the player's seqs (tick, seq)"
        );
        let at = engine.harness_player(client.pid).unwrap().pathing.coord;
        assert_eq!((at.x(), at.z()), (3078, 9786), "thrown a tile west");
        let moves: Vec<_> = moves.iter().map(|&(t, m)| (t - open, m)).collect();
        assert_eq!(
            moves,
            [(3, (750, 100, 0)), (13, (840, 90, 0)), (16, (520, 16, 0))],
            "the camera's moves (tick, (height, var speed, const speed))"
        );
        let looks: Vec<_> = looks.iter().map(|&(t, l)| (t - open, l)).collect();
        assert_eq!(looks, [(3, (300, 100, 0)), (13, (350, 90, 1))], "its look");
        let (tick, nid, x, z) = risen.expect("the Count rises");
        assert_eq!((tick - open, x, z), (21, 3081, 9777));
        assert!(
            loc_at(engine, "coffin_163", COFFIN.1, COFFIN.2),
            "the coffin lies open and empty"
        );
        // He stands where he rose.
        ticks(engine, &mut client, 10);
        let c = engine.get_npc(nid).expect("he stays").npc.pathing.coord;
        assert_eq!((c.x(), c.z()), (3081, 9777));
        // The player goes for him: the camera is the player's again.
        let mut p = ClientPacket::new();
        p.p1_alt2(0).p2_alt3(nid as i32);
        client.send(ClientProt::OpNpc2, &p.finish());
        let mut reset = false;
        for _ in 0..3 {
            engine.cycle();
            reset |= client.packets().iter().any(|(op, _)| *op == CAM_RESET);
        }
        assert!(reset, "CAM_RESET once the player attacks");
        no_script_errors(engine, "the attack");
        // Away from him, the player leaves him to his ai_timer: he goes, the
        // player has the camera back, and the coffin closes for another try.
        engine.harness_telejump(client.pid, 3222, 0, 3218);
        let mut reset = false;
        for _ in 0..5 {
            engine.cycle();
            reset |= client.packets().iter().any(|(op, _)| *op == CAM_RESET);
        }
        assert!(reset, "CAM_RESET as he goes");
        assert!(
            engine.get_npc(nid).is_none(),
            "he goes once the player is away"
        );
        assert!(
            loc_at(engine, "coffin", COFFIN.1, COFFIN.2),
            "the coffin closes once he has gone"
        );
    });
}

/// Whether loc `name` stands at `x`,`z` on level 0.
fn loc_at(engine: &Engine, name: &str, x: u16, z: u16) -> bool {
    let id = pack_id("loc", name) as u16;
    engine
        .zones
        .zone(x, 0, z)
        .and_then(|zone| zone.get_loc(x, z, id))
        .is_some()
}
