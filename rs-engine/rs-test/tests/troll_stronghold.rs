//! Troll Stronghold at 647 (`quests/quest_troll`): 289's quest on 2011's redrawn Trollheim. Cache
//! 786's map puts the rocks, the arena's gates and the stronghold's entrances on other tiles and
//! makes the passes caves; the reward is the 2011 wiki's two mysterious lamps (since 7 May 2008)
//! in place of 289's law talisman.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{Conversation, converse, settexts, talk_to};
use rs_ver::TypeId;
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;
/// The worn slot of boots.
const FEET: u16 = 10;

/// `^troll_started` .. `^troll_complete`, and Death Plateau's `^death_complete`.
const STARTED: i32 = 10;
const DEFEATED_DAD: i32 = 20;
const ENTERED_PRISON: i32 = 30;
const FREED_GODRIC: i32 = 40;
const COMPLETE: i32 = 50;
const DEATH_COMPLETE: i32 = 80;
/// `%troll_varbit`'s bits.
const OPENED_BACK_EXIT: i32 = 1 << 1;
const ENTERED_STRONGHOLD: i32 = 1 << 2;

/// Cache 786's map.
const CAVE: &str = "trollheim_mountain_cave_entrance_4x4_1a";
const BOOTS_ROCKS: (u16, u16) = (2846, 3620);
const SOUTH_OF_THE_ROCKS: (u16, u16) = (2848, 3619);
const NORTH_OF_THE_ROCKS: (u16, u16) = (2848, 3621);
const PASS_CAVE_SOUTH: (u16, u16) = (2910, 3637);
const PASS_CAVE_NORTH: (u16, u16) = (2920, 3654);
const IN_THE_PASS_SOUTH: (u16, u16) = (2907, 10019);
const IN_THE_PASS_NORTH: (u16, u16) = (2907, 10035);
const OUT_OF_THE_PASS_SOUTH: (u16, u16) = (2911, 3636);
const OUT_OF_THE_PASS_NORTH: (u16, u16) = (2921, 3658);
const PASS_EXITS: [(u16, u16); 2] = [(2906, 10017), (2906, 10036)];
const EADGARS_CAVE: (u16, u16) = (2885, 3673);
const IN_EADGARS_CAVE: (u16, u16) = (2893, 10074);
const EADGARS_CAVE_EXIT: (u16, u16) = (2892, 10072);
const OUT_OF_EADGARS_CAVE: (u16, u16) = (2893, 3671);
const STRONGHOLD_CAVE: (u16, u16) = (2847, 3688);
const IN_THE_STRONGHOLD: (u16, u16) = (2837, 10090);
const TOP_EXIT: (u16, u16) = (2838, 10089);
const OUT_OF_THE_STRONGHOLD: (u16, u16) = (2848, 3687);
const SECRET_EXIT: (u16, u16) = (2823, 10048);
const OUT_OF_THE_SECRET_DOOR: (u16, u16) = (2830, 3637);
const SECRET_ENTRANCE: (u16, u16) = (2829, 3638);
const IN_THE_SECRET_DOOR: (u16, u16) = (2823, 10050);
const WEST_GATE: (u16, u16) = (2912, 3609);
const NORTH_EAST_GATE: (u16, u16) = (2927, 3619);
const PRISON_DOOR: (u16, u16) = (2848, 10107);
const GODRICS_CELL: (u16, u16) = (2832, 10078);

fn var(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let v = id(engine, "varp", name);
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(v.g2())
        .as_int()
}

fn set_var(engine: &mut Engine, client: &HarnessClient, name: &str, value: i32) {
    let v = id(engine, "varp", name);
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(v.g2(), VarValue::Int(value));
}

fn login(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    set_var(engine, &client, "troll_quest", stage);
    client
}

fn no_errors(engine: &mut Engine) {
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "no script errors: {errors:?}");
}

fn walkable(x: u16, level: u8, z: u16) -> bool {
    !rsmod::is_flagged(x, z, level, 0x100 | 0x200000)
}

fn tele(engine: &mut Engine, client: &mut HarnessClient, (x, z): (u16, u16), level: u8) {
    assert!(
        walkable(x, level, z),
        "({x}, {z}) on level {level} is walkable"
    );
    engine.harness_telejump(client.pid, x, level, z);
    settle(engine, client, 3);
}

/// Every spawned npc of that name: its nid, tile and level.
fn npcs(engine: &Engine, name: &str) -> Vec<(u16, u16, u16, u8)> {
    let want = id(engine, "npc", name);
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == want)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.z(), c.y())
        })
        .collect()
}

/// Talks to an npc that wanders or stands in a cell: from each walkable tile near it in turn,
/// until one reaches it.
fn talk_near(
    engine: &mut Engine,
    client: &mut HarnessClient,
    npc: &str,
    choices: &[&str],
) -> Conversation {
    let mut last = Conversation::default();
    for attempt in 0..12usize {
        let (nid, x, z, level) = npcs(engine, npc)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("{npc} is spawned"));
        let tiles: Vec<(u16, u16)> = (1i16..=2)
            .flat_map(|r| {
                [
                    (r, 0),
                    (-r, 0),
                    (0, r),
                    (0, -r),
                    (r, r),
                    (-r, -r),
                    (r, -r),
                    (-r, r),
                ]
            })
            .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
            .filter(|&(ax, az)| walkable(ax, level, az))
            .collect();
        assert!(!tiles.is_empty(), "a tile near {npc}");
        let at = tiles[attempt % tiles.len()];
        engine.harness_telejump(client.pid, at.0, level, at.1);
        settle(engine, client, 2);
        talk_to(client, nid);
        // A short look first: a talk that never starts must not idle the client out. The
        // first box's lines are kept for what the talk said.
        let mut first: Vec<String> = Vec::new();
        let mut chat: Vec<String> = Vec::new();
        for _ in 0..10 {
            engine.cycle();
            let packets = client.packets();
            first.extend(settexts(&packets).into_iter().map(|(_, t)| t));
            chat.extend(
                HarnessClient::game_messages(&packets)
                    .into_iter()
                    .map(|(_, m)| m),
            );
            if !first.is_empty() {
                break;
            }
        }
        last = Conversation::default();
        last.messages = chat;
        if !first.is_empty() {
            let more = converse(engine, client, choices, 300);
            assert!(more.unanswered.is_none(), "{:?}", more.unanswered);
            first.extend(more.texts);
            last.texts = first;
            last.messages.extend(more.messages);
            return last;
        }
    }
    panic!("{npc} was never reached: {:?}", last.messages);
}

fn id(engine: &Engine, kind: &str, name: &str) -> TypeId {
    let map = match kind {
        "obj" => &engine.cache.objs.debugnames,
        "npc" => &engine.cache.npcs.debugnames,
        "loc" => &engine.cache.locs.debugnames,
        "varp" => &engine.cache.varps.debugnames,
        _ => unreachable!(),
    };
    *map.get(name).unwrap_or_else(|| panic!("no {kind} {name}"))
}

fn give(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str) {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap()
        .set(slot, o, 1);
}

fn held(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap()
        .total(o) as i32
}

fn settle(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) {
    for _ in 0..ticks {
        engine.cycle();
    }
    client.packets();
}

fn says(t: &Conversation, lines: &[&str]) {
    for line in lines {
        assert!(
            t.said(line),
            "289 has {line:?}; {:?} (chat {:?})",
            t.texts,
            t.messages
        );
    }
}

fn tells(t: &Conversation, line: &str) {
    assert!(
        t.messages.iter().any(|m| m == line),
        "289 has {line:?}; chat {:?} (texts {:?})",
        t.messages,
        t.texts
    );
}

fn coord(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

/// `OPLOC1` (`p2_alt1(z) p2_alt1(loc) p1(ctrl) p2_alt2(x)`) or `OPLOC2` (`p2_alt1(loc)
/// p1_alt2(ctrl) p2_alt2(z) p2_alt3(x)`) on a loc, taking `choices`.
fn op_loc(
    engine: &mut Engine,
    client: &mut HarnessClient,
    op: u8,
    loc: &str,
    (x, z): (u16, u16),
    choices: &[&str],
) -> Conversation {
    let loc = id(engine, "loc", loc);
    let mut p = ClientPacket::new();
    if op == 1 {
        p.p2_alt1(z as i32)
            .p2_alt1(loc.int())
            .p1(0)
            .p2_alt2(x as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
    } else {
        p.p2_alt1(loc.int())
            .p1_alt2(0)
            .p2_alt2(z as i32)
            .p2_alt3(x as i32);
        client.send(ClientProt::OpLoc2, &p.finish());
    }
    shown(engine, client, choices)
}

/// What an op shows: a short look first (an op that only writes to the chat must not idle the
/// client out), then the boxes, taking `choices`.
fn shown(engine: &mut Engine, client: &mut HarnessClient, choices: &[&str]) -> Conversation {
    let mut first: Vec<String> = Vec::new();
    let mut chat: Vec<String> = Vec::new();
    for _ in 0..12 {
        engine.cycle();
        let packets = client.packets();
        first.extend(settexts(&packets).into_iter().map(|(_, t)| t));
        chat.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, m)| m),
        );
        if !first.is_empty() {
            break;
        }
    }
    let mut t = Conversation {
        messages: chat,
        ..Default::default()
    };
    if !first.is_empty() {
        let more = converse(engine, client, choices, 400);
        assert!(more.unanswered.is_none(), "{:?}", more.unanswered);
        first.extend(more.texts);
        t.messages.extend(more.messages);
    }
    t.texts = first;
    t
}

/// What the chat is told over `ticks` (an op that opens no box).
fn watch(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) -> Conversation {
    let mut t = Conversation::default();
    for _ in 0..ticks {
        engine.cycle();
        let packets = client.packets();
        t.texts
            .extend(settexts(&packets).into_iter().map(|(_, text)| text));
        t.messages.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, m)| m),
        );
    }
    t
}

/// `OPLOC1` on a loc, and what the chat is told over `ticks`.
fn search(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    (x, z): (u16, u16),
    ticks: usize,
) -> Conversation {
    let loc = id(engine, "loc", loc);
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc.int())
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
    watch(engine, client, ticks)
}

/// Level 99 in every skill (its xp, in tenths) and full life: no level-up box follows a reward,
/// and a fall is survived.
fn seasoned(engine: &mut Engine, client: &HarnessClient) {
    let player = engine.harness_player_mut(client.pid).unwrap();
    for skill in 0..player.stats.levels.len() {
        player.stats.levels[skill] = 99;
        player.stats.base_levels[skill] = 99;
        player.stats.xp[skill] = 130_344_310;
    }
    let bit0 = player.vars.get(LIFE).as_int() & 1;
    player.vars.set(LIFE, VarValue::Int((990 << 1) | bit0));
}

fn stage(engine: &Engine, client: &HarnessClient) -> i32 {
    var(engine, client, "troll_quest")
}

fn wear(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str) {
    let o = id(engine, "obj", name);
    let worn = *engine.cache.invs.debugnames.get("worn").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&worn)
        .unwrap()
        .set(slot, o, 1);
}

/// A plain `OPLOC1` from each tile around a loc of up to 4x4 in turn, until `done` holds.
fn plain_until(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    tile: (u16, u16),
    level: u8,
    done: impl Fn(&Engine, &HarnessClient, &Conversation) -> bool,
) -> Conversation {
    let mut last = Conversation::default();
    for dx in -1i16..=4 {
        for dz in -1i16..=4 {
            let at = (
                tile.0.wrapping_add_signed(dx),
                tile.1.wrapping_add_signed(dz),
            );
            if !walkable(at.0, level, at.1) {
                continue;
            }
            tele(engine, client, at, level);
            last = search(engine, client, loc, tile, 4);
            if done(engine, client, &last) {
                return last;
            }
        }
    }
    panic!(
        "{loc} at {tile:?}: {:?} at {:?}",
        last.messages,
        coord(engine, client)
    );
}

/// With Death Plateau done, Denulth tells of Godric's capture and the quest is taken.
#[test]
fn denulth_asks_for_godric_back() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ts_start", 0);
        set_var(engine, &client, "death_equiproom", DEATH_COMPLETE);
        let t = talk_near(
            engine,
            &mut client,
            "denulth",
            &[
                "How goes your fight with the trolls?",
                "Is there anything I can do to help?",
                "I'll get Godric back!",
            ],
        );
        says(&t, &["God speed friend!"]);
        assert_eq!(stage(engine, &client), STARTED);
        no_errors(engine);
    });
}

/// The rocks north of the secret path want the quest started and, from their south side, climbing
/// boots: either of 2011's two pairs. Crossed, the player stands beside the rock's middle tile.
#[test]
fn the_rocks_want_the_quest_and_the_boots() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ts_rocks", 0);
        seasoned(engine, &client);
        tele(engine, &mut client, SOUTH_OF_THE_ROCKS, 0);
        let t = search(engine, &mut client, "troll_climbingrocks", BOOTS_ROCKS, 4);
        tells(
            &t,
            "That looks dangerous. I'll need a good reason before I venture that way.",
        );

        set_var(engine, &client, "troll_quest", STARTED);
        let t = search(engine, &mut client, "troll_climbingrocks", BOOTS_ROCKS, 4);
        tells(&t, "You'll need some climbing boots to go that way.");
        assert_eq!(coord(engine, &client), SOUTH_OF_THE_ROCKS);

        wear(engine, &client, FEET, "death_climbingboots_quest");
        let mut crossed = false;
        for _ in 0..12 {
            tele(engine, &mut client, SOUTH_OF_THE_ROCKS, 0);
            let t = search(engine, &mut client, "troll_climbingrocks", BOOTS_ROCKS, 8);
            tells(&t, "You climb onto the rock...");
            if t.messages
                .iter()
                .any(|m| m == "...and step down the other side.")
            {
                crossed = true;
                break;
            }
            tells(&t, "...but you slip back down.");
        }
        assert!(crossed, "the rocks are crossed within a dozen tries");
        assert_eq!(coord(engine, &client), NORTH_OF_THE_ROCKS);

        // Back south needs no boots check (289 asks for them from the south side only).
        let mut back = false;
        for _ in 0..12 {
            tele(engine, &mut client, NORTH_OF_THE_ROCKS, 0);
            let t = search(engine, &mut client, "troll_climbingrocks", BOOTS_ROCKS, 8);
            if t.messages
                .iter()
                .any(|m| m == "...and step down the other side.")
            {
                back = true;
                break;
            }
        }
        assert!(back);
        assert_eq!(coord(engine, &client), SOUTH_OF_THE_ROCKS);
        no_errors(engine);
    });
}

/// 2011's caves: the pass under the mountain both ways, Mad Eadgar's cave, and the stronghold's
/// door, each to 289's tile inside and back out before the cave.
#[test]
fn the_caves_lead_where_289s_passes_and_doors_did() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ts_caves", STARTED);
        seasoned(engine, &client);
        plain_until(engine, &mut client, CAVE, PASS_CAVE_SOUTH, 0, |e, c, _| {
            coord(e, c) == IN_THE_PASS_SOUTH
        });
        plain_until(engine, &mut client, CAVE, PASS_CAVE_NORTH, 0, |e, c, _| {
            coord(e, c) == IN_THE_PASS_NORTH
        });
        let mut outs = Vec::new();
        for exit in PASS_EXITS {
            plain_until(
                engine,
                &mut client,
                "troll_pass_exit",
                exit,
                0,
                |e, c, _| coord(e, c).1 < 4000,
            );
            outs.push(coord(engine, &client));
        }
        outs.sort();
        assert_eq!(
            outs,
            vec![OUT_OF_THE_PASS_SOUTH, OUT_OF_THE_PASS_NORTH],
            "one exit to each cave"
        );

        plain_until(engine, &mut client, CAVE, EADGARS_CAVE, 0, |e, c, _| {
            coord(e, c) == IN_EADGARS_CAVE
        });
        plain_until(
            engine,
            &mut client,
            "di2_cave_wall_exit_light_grey",
            EADGARS_CAVE_EXIT,
            0,
            |e, c, _| coord(e, c) == OUT_OF_EADGARS_CAVE,
        );

        assert_eq!(var(engine, &client, "troll_varbit") & ENTERED_STRONGHOLD, 0);
        plain_until(engine, &mut client, CAVE, STRONGHOLD_CAVE, 0, |e, c, _| {
            coord(e, c) == IN_THE_STRONGHOLD
        });
        assert_eq!(
            var(engine, &client, "troll_varbit") & ENTERED_STRONGHOLD,
            ENTERED_STRONGHOLD
        );
        plain_until(
            engine,
            &mut client,
            "troll_stronghold_top_exit_mid",
            TOP_EXIT,
            2,
            |e, c, _| coord(e, c) == OUT_OF_THE_STRONGHOLD,
        );
        no_errors(engine);
    });
}

/// The secret door opens from outside only once it has been opened from inside.
#[test]
fn the_secret_door_is_learnt_from_inside() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ts_secret", STARTED);
        seasoned(engine, &client);
        let t = plain_until(
            engine,
            &mut client,
            "troll_stronghold_entrance",
            SECRET_ENTRANCE,
            0,
            |_, _, t| {
                t.messages
                    .iter()
                    .any(|m| m == "You don't know how to open the secret door.")
            },
        );
        tells(&t, "You don't know how to open the secret door.");

        plain_until(
            engine,
            &mut client,
            "troll_stronghold_exit",
            SECRET_EXIT,
            0,
            |e, c, _| coord(e, c) == OUT_OF_THE_SECRET_DOOR,
        );
        assert_eq!(
            var(engine, &client, "troll_varbit") & OPENED_BACK_EXIT,
            OPENED_BACK_EXIT
        );
        let t = plain_until(
            engine,
            &mut client,
            "troll_stronghold_entrance",
            SECRET_ENTRANCE,
            0,
            |e, c, _| coord(e, c) == IN_THE_SECRET_DOOR,
        );
        tells(&t, "You open the secret door.");
        no_errors(engine);
    });
}

/// 2011's arena: the west gate lets the player in and Dad is there (added by the gate when he is
/// not) to say his line; the
/// north-east gate stays shut until he is beaten.
#[test]
fn dad_guards_the_arenas_far_gate() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ts_arena", STARTED);
        seasoned(engine, &client);
        tele(engine, &mut client, WEST_GATE, 0);
        let t = op_loc(
            engine,
            &mut client,
            1,
            "trollheim_trollgate_left",
            WEST_GATE,
            &[],
        );
        assert!(
            t.said("No human pass through arena without defeating Dad!"),
            "{:?} {:?}",
            t.texts,
            t.messages
        );
        assert!(
            coord(engine, &client).0 > WEST_GATE.0,
            "inside: {:?}",
            coord(engine, &client)
        );
        let dad = npcs(engine, "troll_champion");
        assert_eq!(dad.len(), 1, "Dad is in the arena");
        assert!(
            walkable(dad[0].1, 0, dad[0].2),
            "on open ground: {:?}",
            dad[0]
        );

        // The far gate, from inside (its south side).
        let inside = (NORTH_EAST_GATE.0, NORTH_EAST_GATE.1 - 1);
        tele(engine, &mut client, inside, 0);
        let t = op_loc(
            engine,
            &mut client,
            1,
            "trollheim_trollgate_left",
            NORTH_EAST_GATE,
            &[],
        );
        assert!(
            t.said("No human pass through arena without defeating Dad!"),
            "{:?}",
            t.texts
        );
        assert!(coord(engine, &client).1 < NORTH_EAST_GATE.1, "still inside");

        set_var(engine, &client, "troll_quest", DEFEATED_DAD);
        tele(engine, &mut client, inside, 0);
        search(
            engine,
            &mut client,
            "trollheim_trollgate_left",
            NORTH_EAST_GATE,
            4,
        );
        assert!(
            coord(engine, &client).1 >= NORTH_EAST_GATE.1,
            "through the far gate: {:?}",
            coord(engine, &client)
        );
        no_errors(engine);
    });
}

/// Dad gives in when a blow lands with him under a sixth of his life (289: under 20 of his 120
/// hitpoints; cache 786's Dad has 1,200 life points, so under 200). He is not killed: he stands
/// there at full life again, and the quest moves on.
#[test]
fn dad_gives_in_before_he_dies() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ts_dad", STARTED);
        seasoned(engine, &client);
        tele(engine, &mut client, WEST_GATE, 0);
        op_loc(
            engine,
            &mut client,
            1,
            "trollheim_trollgate_left",
            WEST_GATE,
            &[],
        );
        let (nid, ..) = npcs(engine, "troll_champion")
            .into_iter()
            .next()
            .expect("Dad is added");
        let hp = rs_pack::types::NpcStat::Hitpoints as usize;
        for tick in 0..300 {
            if stage(engine, &client) == DEFEATED_DAD {
                break;
            }
            let Some(n) = engine.get_npc_mut(nid) else {
                break;
            };
            let c = n.npc.pathing.coord;
            let stats = &mut n.npc.stats.levels;
            // Held between 289's 20 and 2011's 200: a blow that lands should end it, and none
            // can take him under 20 first.
            stats[hp] = 150;
            stats[rs_pack::types::NpcStat::Defence as usize] = 1;
            if tick % 6 == 0 {
                let at = (c.x().wrapping_sub(1), c.z());
                if walkable(at.0, 0, at.1) {
                    engine.harness_telejump(client.pid, at.0, 0, at.1);
                }
                let mut p = ClientPacket::new();
                p.p1_alt2(0).p2_alt3(nid as i32);
                client.send(ClientProt::OpNpc2, &p.finish());
            }
            engine.cycle();
            client.packets();
        }
        assert_eq!(stage(engine, &client), DEFEATED_DAD, "Dad is beaten");
        let n = engine
            .get_npc_mut(nid)
            .expect("Dad is still there: he gave in, he did not die");
        assert_eq!(n.npc.stats.levels[hp], 1200, "and is at full life again");
        let _ = engine.script_errors.take();
    });
}

/// The prison's door wants the generals' key, and Godric's cell its guard's.
#[test]
fn the_keys_open_the_prison_and_godrics_cell() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ts_prison", DEFEATED_DAD);
        seasoned(engine, &client);
        let t = plain_until(
            engine,
            &mut client,
            "troll_stronghold_prison_door_closed",
            PRISON_DOOR,
            1,
            |_, _, t| t.messages.iter().any(|m| m == "You need a key."),
        );
        tells(&t, "You need a key.");
        give(engine, &client, 0, "troll_key_prison");
        let t = plain_until(
            engine,
            &mut client,
            "troll_stronghold_prison_door_closed",
            PRISON_DOOR,
            1,
            |e, c, _| stage(e, c) == ENTERED_PRISON,
        );
        tells(&t, "You unlock the prison door.");
        assert_eq!(held(engine, &client, "troll_key_prison"), 0);

        let t = plain_until(
            engine,
            &mut client,
            "troll_celldoor_godric",
            GODRICS_CELL,
            0,
            |_, _, t| {
                t.messages
                    .iter()
                    .any(|m| m == "You need a key to unlock the door.")
            },
        );
        tells(&t, "You need a key to unlock the door.");
        give(engine, &client, 0, "troll_key_godric");
        let mut freed = false;
        for dx in -1i16..=1 {
            for dz in -1i16..=1 {
                let at = (
                    GODRICS_CELL.0.wrapping_add_signed(dx),
                    GODRICS_CELL.1.wrapping_add_signed(dz),
                );
                if !walkable(at.0, 0, at.1) || stage(engine, &client) == FREED_GODRIC {
                    continue;
                }
                tele(engine, &mut client, at, 0);
                let t = op_loc(
                    engine,
                    &mut client,
                    1,
                    "troll_celldoor_godric",
                    GODRICS_CELL,
                    &[],
                );
                if t.said("Thank you, my friend.") {
                    freed = true;
                }
            }
        }
        assert!(freed, "Godric thanks the player");
        assert_eq!(stage(engine, &client), FREED_GODRIC);
        assert_eq!(held(engine, &client, "troll_key_godric"), 0);
        no_errors(engine);
    });
}

/// Dunstan's reward is 2011's: two mysterious lamps and a quest point (the 2011 wiki's Troll
/// Stronghold and Mysterious lamp pages), not 289's law talisman.
#[test]
fn dunstan_gives_two_mysterious_lamps() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ts_done", FREED_GODRIC);
        seasoned(engine, &client);
        set_var(engine, &client, "death_equiproom", DEATH_COMPLETE);
        let t = talk_near(engine, &mut client, "death_smithy", &[]);
        says(&t, &["Has Godric returned home?"]);
        settle(engine, &mut client, 6);
        assert_eq!(stage(engine, &client), COMPLETE);
        assert_eq!(held(engine, &client, "law_reward_lamp"), 2);
        assert_eq!(held(engine, &client, "law_talisman"), 0);
        no_errors(engine);
    });
}
