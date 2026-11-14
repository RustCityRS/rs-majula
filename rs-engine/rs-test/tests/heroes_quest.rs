//! Heroes' Quest at 647 (`quests/quest_hero`; Achietties and the guild's door in
//! `areas/areas_heroes_guild`; Alfonse in `areas/area_brimhaven`; Gerrant, Straven and Katrine in
//! their areas): 289's quest. The rewards are the 2011 wiki's as 289's (Heroes' Quest, 2011-05-12:
//! 1 quest point and twelve skills' experience); the feather's burn is in 2011's life points.

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
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const BACKPACK: i32 = 149 << 16;
/// Worn slots.
const HEAD: u16 = 0;
const BODY: u16 = 4;
const LEGS: u16 = 7;
const HANDS: u16 = 9;
const ATTACK: usize = 0;
const HERBLORE: usize = 15;

/// `^hero_started` .. `^hero_complete`.
const STARTED: i32 = 1;
const PHOENIX_SPOKEN: i32 = 2;
const PHOENIX_ALFONSE: i32 = 3;
const PHOENIX_CHARLIE: i32 = 4;
const PHOENIX_ARMBAND: i32 = 6;
const BLACKARM_SPOKEN: i32 = 7;
const BLACKARM_HQ: i32 = 8;
const BLACKARM_PAPERS: i32 = 9;
const BLACKARM_MANSION: i32 = 10;
const BLACKARM_PAPERS_GIVEN: i32 = 11;
const BLACKARM_LOOTED: i32 = 12;
const COMPLETE: i32 = 15;

/// Cache 786's map: the guild's west door, the Black Arm hideout's door, the mansion's door and
/// the candlesticks' chest in Brimhaven.
const GUILD_DOOR: (u16, u16) = (2902, 3510);
const HIDEOUT_DOOR: (u16, u16) = (2811, 3170);
const MANSION_DOOR: (u16, u16) = (2774, 3187);
const CANDLE_CHEST: (u16, u16) = (2766, 3199);

fn var(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let v = id(engine, "varp", name);
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(v)
        .as_int()
}

fn set_var(engine: &mut Engine, client: &HarnessClient, name: &str, value: i32) {
    let v = id(engine, "varp", name);
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(v, VarValue::Int(value));
}

fn login(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    set_var(engine, &client, "heroquest", stage);
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

fn id(engine: &Engine, kind: &str, name: &str) -> u16 {
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
            .p2_alt1(loc as i32)
            .p1(0)
            .p2_alt2(x as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
    } else {
        p.p2_alt1(loc as i32)
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

fn slot_of(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let player = engine.harness_player(client.pid).unwrap();
    let inv = player.invs.get(&inv).unwrap();
    (0..28)
        .find(|&s| inv.has_at(s, o))
        .unwrap_or_else(|| panic!("{name} is held")) as i32
}

/// The quests and quest points Achietties asks for, with the player in `gang` ("blackarmgang" or
/// "phoenixgang").
fn worthy(engine: &mut Engine, client: &HarnessClient, gang: &str) {
    set_var(engine, client, "qp", 55);
    set_var(engine, client, "zanaris", 6);
    set_var(engine, client, "dragonquest", 10);
    set_var(engine, client, "arthur", 7);
    set_var(
        engine,
        client,
        gang,
        if gang == "blackarmgang" { 4 } else { 10 },
    );
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

fn xp(engine: &Engine, client: &HarnessClient, skill: usize) -> i32 {
    engine.harness_player(client.pid).unwrap().stats.xp[skill]
}

/// Stands the player on a walkable tile within two of a loc's tile, nearest first.
fn stand_by(engine: &mut Engine, client: &mut HarnessClient, (x, z): (u16, u16)) {
    let mut tiles: Vec<(i16, i16)> = (-2i16..=2)
        .flat_map(|dx| (-2i16..=2).map(move |dz| (dx, dz)))
        .filter(|&(dx, dz)| (dx, dz) != (0, 0))
        .collect();
    tiles.sort_by_key(|&(dx, dz)| dx.abs() + dz.abs());
    let at = tiles
        .into_iter()
        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
        .find(|&(ax, az)| walkable(ax, 0, az))
        .unwrap_or_else(|| panic!("a tile by ({x}, {z})"));
    tele(engine, client, at, 0);
}

/// Uses the backpack obj `used` on the backpack obj `target` (`IF_BUTTONT`).
fn use_on_obj(engine: &mut Engine, client: &mut HarnessClient, used: &str, target: &str) {
    let (us, ts) = (
        slot_of(engine, client, used),
        slot_of(engine, client, target),
    );
    let (uo, to) = (id(engine, "obj", used), id(engine, "obj", target));
    let mut p = ClientPacket::new();
    p.p4(BACKPACK)
        .p2_alt1(ts)
        .p2_alt1(us)
        .p2_alt2(uo as i32)
        .p4_alt1(BACKPACK)
        .p2(to as i32);
    client.send(ClientProt::IfButtonT, &p.finish());
    settle(engine, client, 3);
}

/// Achietties takes only a hero: 55 quest points, the four quests and a gang; then she names
/// the three things to bring.
#[test]
fn achietties_takes_only_a_hero() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hq_start", 0);
        let t = talk_near(
            engine,
            &mut client,
            "achietties",
            &["I'm a hero, may I apply to join?"],
        );
        says(
            &t,
            &[
                "Greetings. Welcome to the Heroes' Guild.",
                "You're a hero? I've never heard of YOU.",
            ],
        );
        assert_eq!(var(engine, &client, "heroquest"), 0);
        // The door is hers until then.
        tele(engine, &mut client, (GUILD_DOOR.0 - 1, GUILD_DOOR.1), 0);
        op_loc(
            engine,
            &mut client,
            1,
            "herodoor_l",
            GUILD_DOOR,
            &["Good for the foremost heroes of the land."],
        );
        settle(engine, &mut client, 3);
        assert!(
            coord(engine, &client).0 < GUILD_DOOR.0,
            "the guild's door stays shut"
        );

        worthy(engine, &client, "blackarmgang");
        let t = talk_near(
            engine,
            &mut client,
            "achietties",
            &[
                "I'm a hero, may I apply to join?",
                "I'll start looking for all those things then.",
            ],
        );
        says(
            &t,
            &["An Entranan Firebird feather,", "Good luck with that."],
        );
        assert_eq!(var(engine, &client, "heroquest"), STARTED);
        no_errors(engine);
    });
}

/// The firebird's feather burns a bare hand and is taken in ice gloves; blamish oil makes the
/// rod for lava eels; Gerrant has the slime for it.
#[test]
fn the_feather_and_the_oily_rod() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hq_feather", STARTED);
        {
            let player = engine.harness_player_mut(client.pid).unwrap();
            for skill in [0, 1, 2, 3] {
                player.stats.levels[skill] = 99;
                player.stats.base_levels[skill] = 99;
            }
            let bit0 = player.vars.get(1240).as_int() & 1;
            player.vars.set(1240, VarValue::Int((990 << 1) | bit0));
        }
        // The Entrana firebird falls and leaves its feather.
        let (nid, ..) = npcs(engine, "fire_bird")
            .into_iter()
            .next()
            .expect("the firebird is spawned");
        let mut fell = None;
        for tick in 0..200 {
            let Some(n) = engine.get_npc_mut(nid) else {
                break;
            };
            let c = n.npc.pathing.coord;
            let stats = &mut n.npc.stats.levels;
            let gone = stats[rs_pack::types::NpcStat::Hitpoints as usize] == 0;
            stats[rs_pack::types::NpcStat::Hitpoints as usize] =
                stats[rs_pack::types::NpcStat::Hitpoints as usize].min(1);
            stats[rs_pack::types::NpcStat::Defence as usize] = 1;
            if gone {
                fell = Some((c.x(), c.z()));
            }
            if tick % 6 == 0 {
                let (px, pz) = coord(engine, &client);
                if px.abs_diff(c.x()) > 1 || pz.abs_diff(c.z()) > 1 {
                    if let Some(at) = [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
                        .into_iter()
                        .map(|(dx, dz)| {
                            (c.x().wrapping_add_signed(dx), c.z().wrapping_add_signed(dz))
                        })
                        .find(|&(ax, az)| walkable(ax, 0, az))
                    {
                        engine.harness_telejump(client.pid, at.0, 0, at.1);
                    }
                }
                let mut p = ClientPacket::new();
                p.p1_alt2(0).p2_alt3(nid as i32);
                client.send(ClientProt::OpNpc2, &p.finish());
            }
            engine.cycle();
            client.packets();
            if fell.is_some() {
                break;
            }
        }
        let at = fell.expect("the firebird is killed");
        settle(engine, &mut client, 8);
        let take = |engine: &mut Engine, client: &mut HarnessClient| {
            let mut p = ClientPacket::new();
            p.p2_alt1(id(engine, "obj", "hot_feather") as i32)
                .p2_alt2(at.1 as i32)
                .p1(0)
                .p2(at.0 as i32);
            client.send(ClientProt::OpObj3, &p.finish());
            shown(engine, client, &[])
        };
        let life = |engine: &Engine, client: &HarnessClient| {
            engine
                .harness_player(client.pid)
                .unwrap()
                .vars
                .get(1240)
                .as_int()
                >> 1
        };
        let before = life(engine, &client);
        let t = take(engine, &mut client);
        tells(
            &t,
            "It is too hot to take. You need something cold to pick it up with.",
        );
        assert_eq!(held(engine, &client, "hot_feather"), 0);
        // 289's an-eighth-and-one of the hitpoints, in life points: (99 / 8 + 1) x 10, less
        // whatever a tick of healing gave back meanwhile.
        let burn = before - life(engine, &client);
        assert!((120..=130).contains(&burn), "the burn: {burn}");

        wear(engine, &client, HANDS, "ice_gloves");
        take(engine, &mut client);
        settle(engine, &mut client, 3);
        assert_eq!(
            held(engine, &client, "hot_feather"),
            1,
            "taken in ice gloves"
        );

        give(engine, &client, 5, "blamish_oil");
        give(engine, &client, 6, "fishing_rod");
        use_on_obj(engine, &mut client, "blamish_oil", "fishing_rod");
        assert_eq!(held(engine, &client, "oily_fishing_rod"), 1);
        assert_eq!(held(engine, &client, "vial_empty"), 1);
        assert_eq!(held(engine, &client, "blamish_oil"), 0);

        // Gerrant's third option is back with the quest.
        let t = talk_near(
            engine,
            &mut client,
            "gerrant_npc",
            &["I want to find out how to catch a lava eel."],
        );
        says(&t, &["I'm sure I already gave you what you need for this"]);
        no_errors(engine);
    });
}

/// The Black Arm Gang's way to the candlesticks: the hideout's password, Hartigen's papers,
/// the mansion in black armour, Grip's key to pass on, and the chest.
#[test]
fn the_black_arm_gang_takes_the_candlesticks() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hq_blackarm", BLACKARM_SPOKEN);
        worthy(engine, &client, "blackarmgang");
        stand_by(engine, &mut client, HIDEOUT_DOOR);
        let t = op_loc(
            engine,
            &mut client,
            1,
            "grubordoor",
            HIDEOUT_DOOR,
            &["Four leaved clover."],
        );
        says(&t, &["You hear the door being unbarred from inside."]);
        assert_eq!(var(engine, &client, "heroquest"), BLACKARM_HQ);

        let t = talk_near(
            engine,
            &mut client,
            "trobert",
            &[
                "So can you help me get Scarface Pete's candlesticks?",
                "I volunteer to undertake that mission!",
            ],
        );
        says(&t, &["I volunteer to undertake that mission!"]);
        assert_eq!(var(engine, &client, "heroquest"), BLACKARM_PAPERS);
        assert_eq!(held(engine, &client, "id_papers"), 1);

        // Garv lets Hartigen in only in black armour.
        stand_by(engine, &mut client, MANSION_DOOR);
        let t = op_loc(engine, &mut client, 1, "garvdoor", MANSION_DOOR, &[]);
        says(
            &t,
            &["Hartigen the Black Knight? I don't think so. He doesn't dress like that."],
        );
        assert_eq!(var(engine, &client, "heroquest"), BLACKARM_PAPERS);
        wear(engine, &client, HEAD, "black_full_helm");
        wear(engine, &client, BODY, "black_platebody");
        wear(engine, &client, LEGS, "black_platelegs");
        let t = op_loc(engine, &mut client, 1, "garvdoor", MANSION_DOOR, &[]);
        says(
            &t,
            &["You'd better come in then. Grip will want to talk to you."],
        );
        assert_eq!(var(engine, &client, "heroquest"), BLACKARM_MANSION);

        let t = talk_near(
            engine,
            &mut client,
            "grip",
            &["So what do my duties involve?", "Anything I can do now?"],
        );
        says(
            &t,
            &[
                "You hand the ID papers over to Grip.",
                "Grip hands you a key.",
            ],
        );
        assert_eq!(var(engine, &client, "heroquest"), BLACKARM_PAPERS_GIVEN);
        assert_eq!(held(engine, &client, "misc_key"), 1);
        assert_eq!(held(engine, &client, "id_papers"), 0);

        // The chest in the treasure room: two candlesticks, one for the partner.
        stand_by(engine, &mut client, CANDLE_CHEST);
        op_loc(engine, &mut client, 1, "shutcandlechest", CANDLE_CHEST, &[]);
        settle(engine, &mut client, 2);
        let t = op_loc(engine, &mut client, 1, "opencandlechest", CANDLE_CHEST, &[]);
        says(&t, &["You find two candlesticks in the chest."]);
        assert_eq!(held(engine, &client, "petecandlestick"), 2);
        assert_eq!(var(engine, &client, "heroquest"), BLACKARM_LOOTED);
        no_errors(engine);
    });
}

/// The Phoenix Gang's way in: Alfonse's gherkins and Charlie's panel.
#[test]
fn the_phoenix_gang_finds_the_way_in() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hq_phoenix", PHOENIX_SPOKEN);
        worthy(engine, &client, "phoenixgang");
        let t = talk_near(
            engine,
            &mut client,
            "alfonse_the_waiter",
            &["Do you sell Gherkins?"],
        );
        says(&t, &["Alfonse winks at you."]);
        assert_eq!(var(engine, &client, "heroquest"), PHOENIX_ALFONSE);

        let t = talk_near(
            engine,
            &mut client,
            "charlie_the_cook",
            &[
                "I'm looking for a gherkin...",
                "I want to steal Scarface Pete's candlesticks.",
            ],
        );
        says(&t, &["Mind if I check it out for myself?"]);
        assert_eq!(var(engine, &client, "heroquest"), PHOENIX_CHARLIE);
        no_errors(engine);
    });
}

/// The three things brought, Achietties opens the guild: 1 quest point and the twelve skills'
/// experience (2011 wiki, as 289).
#[test]
fn the_three_things_open_the_guild() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hq_done", PHOENIX_ARMBAND);
        worthy(engine, &client, "phoenixgang");
        give(engine, &client, 0, "hot_feather");
        give(engine, &client, 1, "lava_eel");
        give(engine, &client, 2, "master_thief_armband");
        // Level 99 in every skill (its xp, in tenths): the twelve rewards raise no level, so no
        // level-up box stands between the reward and the door.
        {
            let player = engine.harness_player_mut(client.pid).unwrap();
            for skill in 0..player.stats.levels.len() {
                player.stats.levels[skill] = 99;
                player.stats.base_levels[skill] = 99;
                player.stats.xp[skill] = 130_344_310;
            }
        }
        let (attack, herblore) = (xp(engine, &client, ATTACK), xp(engine, &client, HERBLORE));
        let t = talk_near(engine, &mut client, "achietties", &[]);
        says(
            &t,
            &[
                "I have all the required items.",
                "Congratulations! You have completed the Hero's Guild",
            ],
        );
        settle(engine, &mut client, 3);
        assert_eq!(var(engine, &client, "heroquest"), COMPLETE);
        assert_eq!(
            xp(engine, &client, ATTACK) - attack,
            30_750,
            "3,075 Attack xp"
        );
        assert_eq!(
            xp(engine, &client, HERBLORE) - herblore,
            13_250,
            "1,325 Herblore xp"
        );
        for obj in ["hot_feather", "lava_eel", "master_thief_armband"] {
            assert_eq!(held(engine, &client, obj), 0, "{obj} is handed over");
        }
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);

        tele(engine, &mut client, (GUILD_DOOR.0 - 1, GUILD_DOOR.1), 0);
        let door = id(engine, "loc", "herodoor_l");
        let mut p = ClientPacket::new();
        p.p2_alt1(GUILD_DOOR.1 as i32)
            .p2_alt1(door as i32)
            .p1(0)
            .p2_alt2(GUILD_DOOR.0 as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        let mut said = Vec::new();
        for _ in 0..6 {
            engine.cycle();
            said.extend(HarnessClient::game_messages(&client.packets()));
        }
        assert!(
            coord(engine, &client).0 >= GUILD_DOOR.0,
            "through the guild's door: {:?} {said:?}",
            coord(engine, &client)
        );
        no_errors(engine);
    });
}
