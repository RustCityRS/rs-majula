//! Scorpion Catcher at 647 (`quests/quest_scorpcatcher`, Thormac and the seers in
//! `areas/area_seers`, Peksa in `areas/area_barbarian_village`): 289's quest. The 2011 wiki
//! (Scorpion Catcher, 2011-05-12) gives the sting's 30 life points and the reward, 1 quest point
//! and 6,625 Strength xp; cache 786's interface 332 gives Thormac's seven staves.

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
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const STRENGTH: usize = 2;
const BACKPACK: i32 = 149 << 16;
/// View on the quest list's row (questlist_v4:quests) and the journal's first line (qj1).
const QUEST_LIST: i32 = 190 << 16 | 18;
const SCORPION_CATCHER: i32 = 93;
const JOURNAL_LINE_1: u32 = 275 << 16 | 16;
/// Thormac's screen (interface 332) and its mud battlestaff (com 26).
const THORMAC: u16 = 332;
const THORMAC_MUD: i32 = (332 << 16) | 26;

/// `^scorpcatcher_started` .. `^scorpcatcher_complete`.
const STARTED: i32 = 1;
const FIRST_HINT: i32 = 2;
const SECOND_HINT: i32 = 3;
const COMPLETE: i32 = 6;

/// maps/l44_153.jm2: the false wall in Taverley Dungeon, on its tile's north edge.
const WALL: (u16, u16) = (2875, 9798);

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

/// %life_points: varbit 7198, bits 1-15 of varp 1240.
fn life_points(engine: &Engine, client: &HarnessClient) -> i32 {
    (engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(1240)
        .as_int()
        >> 1)
        & 0x7FFF
}

fn login(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    set_var(engine, &client, "scorpcatcher", stage);
    client
}

fn no_errors(engine: &mut Engine) {
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "no script errors: {errors:?}");
}

fn walkable(x: u16, level: u8, z: u16) -> bool {
    !rsmod::is_flagged(x, z, level, 0x100 | 0x200000)
}

fn coord(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
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

/// Stands the player on a tile beside the npc with no wall between them.
fn stand_by(engine: &mut Engine, client: &mut HarnessClient, npc: &str) -> u16 {
    let (nid, x, z, level) = npcs(engine, npc)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{npc} is spawned"));
    let at = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dz)| {
            (
                x.wrapping_add_signed(dx as i16),
                z.wrapping_add_signed(dz as i16),
                dx,
                dz,
            )
        })
        .find(|&(ax, az, dx, dz)| {
            walkable(ax, level, az)
                && rsmod::can_travel(level, ax, az, -dx, -dz, 1, 0, CollisionType::Normal)
        })
        .map(|(ax, az, ..)| (ax, az))
        .unwrap_or_else(|| panic!("a tile beside {npc}"));
    tele(engine, client, at, level);
    nid
}

/// Talks to the npc of that name, taking `choices`.
fn talk(
    engine: &mut Engine,
    client: &mut HarnessClient,
    npc: &str,
    choices: &[&str],
) -> Conversation {
    let nid = stand_by(engine, client, npc);
    talk_to(client, nid);
    let talk = converse(engine, client, choices, 300);
    assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
    talk
}

/// The rest of a talk that pauses with no chat box open (the script closes it and waits):
/// what follows is added to `talk`.
fn go_on(
    engine: &mut Engine,
    client: &mut HarnessClient,
    talk: &mut Conversation,
    choices: &[&str],
) {
    let more = converse(engine, client, choices, 300);
    assert!(more.unanswered.is_none(), "{:?}", more.unanswered);
    talk.texts.extend(more.texts);
    talk.messages.extend(more.messages);
}

/// `OPLOC1`: `p2_alt1(z) p2_alt1(loc) p1(ctrl) p2_alt2(x)`, and what it shows.
fn op_loc1(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    (x, z): (u16, u16),
) -> Conversation {
    let loc = id(engine, "loc", loc);
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
    converse(engine, client, &[], 12)
}

/// An item used on an npc (`OPNPCT`), and what it shows.
fn use_on_npc(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    nid: u16,
) -> Conversation {
    let slot = slot_of(engine, client, obj);
    let mut p = ClientPacket::new();
    p.p2_alt2(id(engine, "obj", obj) as i32)
        .p2_alt2(slot)
        .p2_alt1(nid as i32)
        .p4_alt3(BACKPACK)
        .p1_alt2(0);
    client.send(ClientProt::OpNpcT, &p.finish());
    converse(engine, client, &[], 12)
}

/// The journal's text once View is clicked on the quest's row, its colour tags taken off.
fn journal(engine: &mut Engine, client: &mut HarnessClient) -> String {
    let mut p = ClientPacket::new();
    p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(SCORPION_CATCHER);
    client.send(ClientProt::IfButton1, &p.finish());
    let mut texts = Vec::new();
    for _ in 0..3 {
        engine.cycle();
        texts.extend(settexts(&client.packets()));
    }
    engine.harness_close_modal(client.pid);
    let mut plain = String::new();
    for line in 0..40 {
        let text = texts
            .iter()
            .find(|(c, _)| *c == JOURNAL_LINE_1 + line)
            .map_or("", |(_, t)| t.as_str());
        let mut tag = false;
        for ch in text.chars() {
            match ch {
                '<' => tag = true,
                '>' => tag = false,
                _ if !tag => plain.push(ch),
                _ => {}
            }
        }
        plain.push(' ');
    }
    plain
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

fn slot_of(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let bag = engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap();
    (0..28)
        .find(|&slot| bag.has_at(slot, o))
        .unwrap_or_else(|| panic!("{name} is held")) as i32
}

/// Thormac hands over the cage and starts the quest, and gives another to a player who lost it.
#[test]
fn thormac_gives_the_cage_and_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "sc_start", 0);
        let lines = journal(engine, &mut client);
        assert!(
            lines.contains("I can start this quest by speaking to Thormac"),
            "{lines}"
        );

        let t = talk(engine, &mut client, "thormac", &["I'm a little busy."]);
        says(&t, &["Hello I am Thormac the sorcerer."]);
        assert_eq!(var(engine, &client, "scorpcatcher"), 0);

        let mut t = talk(
            engine,
            &mut client,
            "thormac",
            &[
                "What do you need assistance with?",
                "So how would I go about catching them then?",
            ],
        );
        // He hands the cage over, four ticks with no chat box, then goes on.
        go_on(engine, &mut client, &mut t, &["Ok, I will do it then"]);
        says(
            &t,
            &[
                "I've lost my pet scorpions.",
                "Well I have a scorpion cage here",
                "Ok, I will do it then.",
            ],
        );
        tells(&t, "Thormac gives you a cage.");
        assert_eq!(var(engine, &client, "scorpcatcher"), STARTED);
        assert_eq!(held(engine, &client, "scorpioncageempty"), 1, "one cage");

        let t = talk(engine, &mut client, "thormac", &[]);
        says(
            &t,
            &[
                "How goes your quest?",
                "I've not caught all the scorpions yet.",
            ],
        );

        // The cage lost: he has another.
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let cage = id(engine, "obj", "scorpioncageempty");
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .unwrap()
            .delete(cage, 1);
        let t = talk(engine, &mut client, "thormac", &["I've lost my cage."]);
        says(
            &t,
            &[
                "Ok, here's another cage.",
                "You're almost as bad at losing things as me.",
            ],
        );
        assert_eq!(held(engine, &client, "scorpioncageempty"), 1);
        no_errors(engine);
    });
}

/// The seers find the first scorpion in their looking glass, and the other two once it is
/// caught.
#[test]
fn the_seers_look_for_the_scorpions() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "sc_seer", 0);
        let t = talk(engine, &mut client, "seer", &["Many greetings."]);
        says(
            &t,
            &["Remember, whenever you set out to do something, something else must be done first."],
        );

        set_var(engine, &client, "scorpcatcher", STARTED);
        give(engine, &client, 0, "scorpioncageempty");
        let mut t = talk(
            engine,
            &mut client,
            "seer",
            &["I need to locate some scorpions."],
        );
        // The looking glass: nine ticks with no chat box, then what the seer saw.
        go_on(engine, &mut client, &mut t, &[]);
        says(
            &t,
            &[
                "Let me look into my looking glass.",
                "I can see two coffins there as well.",
                "It's gone into some sort of secret room.",
            ],
        );
        for line in [
            "The seer produces a small mirror",
            "The seer gazes into the mirror",
            "The seer smoothes his hair with his hand",
        ] {
            tells(&t, line);
        }
        assert_eq!(var(engine, &client, "scorpcatcher"), FIRST_HINT);

        // With the first scorpion caged, the seer has the other two.
        give(engine, &client, 0, "scorpioncagea");
        let t = talk(engine, &mut client, "seer", &[]);
        says(
            &t,
            &[
                "Hi, I've retrieved the scorpion from near the spiders.",
                "Some kind of merchant there seems to have picked it up.",
                "There seems to be some sort of brown clothing lying on a table.",
            ],
        );
        assert_eq!(var(engine, &client, "scorpcatcher"), SECOND_HINT);

        // Peksa owns up to the second, once the seer has named him.
        let t = talk(
            engine,
            &mut client,
            "peksa",
            &["I've heard you have a small scorpion in your possession."],
        );
        says(
            &t,
            &[
                "Now how could you know about that, I wonder?",
                "I gave it as a present to my brother Ivor when I visited our outpost northwest of Camelot.",
            ],
        );
        set_var(engine, &client, "scorpcatcher", FIRST_HINT);
        let t = talk(engine, &mut client, "peksa", &["No, I'll pass on that."]);
        assert!(
            !t.texts
                .iter()
                .any(|t| t.contains("I've heard you have a small scorpion")),
            "not offered before the seer names him: {:?}",
            t.texts
        );
        no_errors(engine);
    });
}

/// The false wall in Taverley Dungeon opens once the seer has seen the room behind it, and
/// the scorpion there stings a hand and is caught in the cage.
#[test]
fn the_false_wall_and_the_first_scorpion() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // The wall is on its tile's north edge: the passage is north of it, the room south.
        let passage = (WALL.0, WALL.1 + 1);
        let mut client = login(engine, "sc_wall", STARTED);
        tele(engine, &mut client, passage, 0);
        let t = op_loc1(engine, &mut client, "scorpionwall", WALL);
        tells(&t, "Looks like a wall...");
        assert_eq!(coord(engine, &client), passage);

        set_var(engine, &client, "scorpcatcher", FIRST_HINT);
        let t = op_loc1(engine, &mut client, "scorpionwall", WALL);
        tells(&t, "You've found a secret door");
        assert_eq!(coord(engine, &client), WALL, "through the wall");
        settle(engine, &mut client, 6);

        // Picked up by hand, it stings for 30 life points.
        let scorpion = stand_by(engine, &mut client, "questscorpiona");
        let before = life_points(engine, &client);
        talk_to(&mut client, scorpion);
        let mut said = Vec::new();
        let mut sting = 0;
        let mut was = before;
        for _ in 0..6 {
            engine.cycle();
            said.extend(HarnessClient::game_messages(&client.packets()));
            let now = life_points(engine, &client);
            sting = sting.max(was - now);
            was = now;
        }
        assert!(
            said.iter().any(|(_, m)| m == "The scorpion stings you!"),
            "{said:?}"
        );
        assert_eq!(sting, 30, "30 life points (of {before})");

        // A cage that holds it already does not take it again; the empty one does.
        give(engine, &client, 0, "scorpioncagea");
        let scorpion = stand_by(engine, &mut client, "questscorpiona");
        let t = use_on_npc(engine, &mut client, "scorpioncagea", scorpion);
        tells(&t, "You've already caught this scorpion");
        give(engine, &client, 0, "scorpioncageempty");
        let scorpion = stand_by(engine, &mut client, "questscorpiona");
        let t = use_on_npc(engine, &mut client, "scorpioncageempty", scorpion);
        tells(&t, "You catch a scorpion!");
        assert_eq!(held(engine, &client, "scorpioncageempty"), 0);
        assert_eq!(held(engine, &client, "scorpioncagea"), 1);

        // Back out through the wall.
        tele(engine, &mut client, WALL, 0);
        op_loc1(engine, &mut client, "scorpionwall", WALL);
        assert_eq!(coord(engine, &client), passage, "out again");
        settle(engine, &mut client, 6);
        no_errors(engine);
    });
}

/// The second and third scorpions, where cache 786 has them, fill the cage.
#[test]
fn the_other_two_scorpions_fill_the_cage() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "sc_cage", SECOND_HINT);
        give(engine, &client, 0, "scorpioncagea");
        let b = stand_by(engine, &mut client, "questscorpionb");
        let t = use_on_npc(engine, &mut client, "scorpioncagea", b);
        tells(&t, "You catch a scorpion!");
        assert_eq!(held(engine, &client, "scorpioncageab"), 1);
        assert_eq!(held(engine, &client, "scorpioncagea"), 0);

        let c = stand_by(engine, &mut client, "questscorpionc");
        let t = use_on_npc(engine, &mut client, "scorpioncageab", c);
        tells(&t, "You catch a scorpion!");
        assert_eq!(held(engine, &client, "scorpioncagefull"), 1);
        assert_eq!(held(engine, &client, "scorpioncageab"), 0);
        no_errors(engine);
    });
}

/// Thormac takes his scorpions back: 1 quest point and 6,625 Strength xp (2011 wiki, Scorpion
/// Catcher, 2011-05-12). After that he enchants a battlestaff for 40,000 coins, the mud
/// battlestaff among cache 786's seven.
#[test]
fn thormac_rewards_the_full_cage_and_enchants_staves() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "sc_reward", SECOND_HINT);
        give(engine, &client, 0, "scorpioncagefull");
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[STRENGTH];
        let qp = var(engine, &client, "qp");
        let mut t = talk(engine, &mut client, "thormac", &[]);
        for _ in 0..10 {
            engine.cycle();
            let packets = client.packets();
            t.texts
                .extend(settexts(&packets).into_iter().map(|(_, t)| t));
        }
        says(
            &t,
            &[
                "I have retrieved all your scorpions.",
                "Aha, my little scorpions home at last!",
                "You have completed the Scorpion Catcher Quest!",
                "6,625 Strength XP",
            ],
        );
        assert_eq!(var(engine, &client, "scorpcatcher"), COMPLETE);
        assert_eq!(held(engine, &client, "scorpioncagefull"), 0);
        let gained = engine.harness_player(client.pid).unwrap().stats.xp[STRENGTH] - xp;
        assert_eq!(gained, 66_250, "6,625 Strength xp (tenths)");
        assert_eq!(var(engine, &client, "qp") - qp, 1, "1 quest point");
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);
        let lines = journal(engine, &mut client);
        assert!(lines.contains("QUEST COMPLETE!"), "{lines}");

        // His screen, and a staff with neither the staff nor the coins.
        let t = talk(
            engine,
            &mut client,
            "thormac",
            &["You said you'd enchant my battlestaff for me."],
        );
        says(&t, &["Yes, it'll cost you 40,000 coins"]);
        settle(engine, &mut client, 2);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(THORMAC),
            "Thormac's screen is open"
        );
        let click = |engine: &mut Engine, client: &mut HarnessClient| {
            let mut p = ClientPacket::new();
            p.p4(THORMAC_MUD).p2_alt3(-1).p2_alt1(-1);
            client.send(ClientProt::IfButton1, &p.finish());
            let mut said = Vec::new();
            let mut texts = Vec::new();
            for _ in 0..4 {
                engine.cycle();
                let packets = client.packets();
                said.extend(
                    HarnessClient::game_messages(&packets)
                        .into_iter()
                        .map(|(_, m)| m),
                );
                texts.extend(settexts(&packets).into_iter().map(|(_, t)| t));
            }
            (said, texts)
        };
        let (said, _) = click(engine, &mut client);
        assert!(
            said.iter()
                .any(|m| m == "You don't have a mud battlestaff."),
            "{said:?}"
        );

        give(engine, &client, 1, "mud_battlestaff");
        let (_, texts) = click(engine, &mut client);
        assert!(
            texts
                .iter()
                .any(|t| t.contains("You need 40,000 coins to pay Thormac.")),
            "{texts:?}"
        );
        assert_eq!(held(engine, &client, "mud_battlestaff"), 1);

        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let coins = id(engine, "obj", "coins");
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .unwrap()
            .set(2, coins, 40_000);
        engine.harness_close_modal(client.pid);
        talk(
            engine,
            &mut client,
            "thormac",
            &["You said you'd enchant my battlestaff for me."],
        );
        settle(engine, &mut client, 2);
        let (said, _) = click(engine, &mut client);
        assert!(
            said.iter()
                .any(|m| m == "Thormac enchants your Mud battlestaff."),
            "{said:?}"
        );
        assert_eq!(held(engine, &client, "mystic_mud_staff"), 1);
        assert_eq!(held(engine, &client, "mud_battlestaff"), 0);
        assert_eq!(held(engine, &client, "coins"), 0);
        no_errors(engine);
    });
}
