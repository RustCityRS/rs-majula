//! Tribal Totem at 647 (`quests/quest_totem`; Kangai Mau in `areas/area_brimhaven`; Horacio, the
//! RPDT employees and Cromperty's teleport in `areas/area_ardougne_east`): 289's quest on cache
//! 786's mansion, where every loc of it moved, and on its combination lock (interface 369). The
//! 2011 wiki (Tribal Totem) gives the reward: 1 quest point, 1,775 Thieving xp and 5 swordfish.

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

const THIEVING: usize = 17;
const BACKPACK: i32 = 149 << 16;
/// View on the quest list's row (questlist_v4:quests) and the journal's first line (qj1).
const QUEST_LIST: i32 = 190 << 16 | 18;
const TRIBAL_TOTEM: i32 = 111;
const JOURNAL_LINE_1: u32 = 275 << 16 | 16;
/// The combination lock (interface 369): its four letters are coms 13-16, each letter's Left
/// and Right coms 17-24 in pairs, Enter com 27.
const LOCK: u16 = 369;
const LOCK_LETTER_A: u32 = (369 << 16) | 13;
const LOCK_LEFT_A: i32 = (369 << 16) | 17;
const LOCK_ENTER: i32 = (369 << 16) | 27;

/// `^totem_started` .. `^totem_complete`.
const STARTED: i32 = 1;
const CRATE_MARKED: i32 = 2;
const CRATE_DELIVERED: i32 = 3;
const TELEPORTED: i32 = 4;
const COMPLETE: i32 = 5;

/// maps/l41_51.jm2 (mapsquare base 2624, 3264).
const DEPOT_HORN_CRATE: (u16, u16) = (2650, 3271);
const DEPOT_BLOCK_CRATE: (u16, u16) = (2650, 3272);
const DEPOT_ARRIVAL: (u16, u16) = (2649, 3272);
const MANSION_ARRIVAL: (u16, u16) = (2642, 3321);
/// The front door is on its tile's north edge: the tile is outside, the hall north of it.
const FRONT_DOOR: (u16, u16) = (2635, 3319);
/// The combination door is on its tile's west edge: the tile is the middle room's.
const LOCK_DOOR: (u16, u16) = (2633, 3322);
const STAIRS: (u16, u16) = (2630, 3324);
const CHEST: (u16, u16) = (2636, 3324);
/// `^totem_stairs_fail_coord`.
const TRAP_FALL: (u16, u16) = (2640, 9719);

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
    set_var(engine, &client, "totemquest", stage);
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

/// `OPLOC2`: `p2_alt1(loc) p1_alt2(ctrl) p2_alt2(z) p2_alt3(x)`, and what it shows.
fn op_loc2(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    (x, z): (u16, u16),
) -> Conversation {
    let loc = id(engine, "loc", loc);
    let mut p = ClientPacket::new();
    p.p2_alt1(loc as i32)
        .p1_alt2(0)
        .p2_alt2(z as i32)
        .p2_alt3(x as i32);
    client.send(ClientProt::OpLoc2, &p.finish());
    converse(engine, client, &[], 12)
}

/// An item used on a loc (`OPLOCT`), and what it shows.
fn use_on_loc(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    loc: &str,
    (x, z): (u16, u16),
) -> Conversation {
    let slot = slot_of(engine, client, obj);
    let mut p = ClientPacket::new();
    p.p2_alt2(id(engine, "obj", obj) as i32)
        .p2_alt2(slot)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt2(x as i32)
        .p1_alt1(0)
        .p2_alt1(id(engine, "loc", loc) as i32);
    client.send(ClientProt::OpLocT, &p.finish());
    converse(engine, client, &[], 12)
}

fn level(engine: &Engine, client: &HarnessClient) -> u8 {
    engine.harness_player(client.pid).unwrap().pathing.coord.y()
}

/// A click on one of the lock's buttons: the texts and the messages of the next ticks.
fn lock_click(
    engine: &mut Engine,
    client: &mut HarnessClient,
    com: i32,
) -> (Vec<(u32, String)>, Vec<String>) {
    let mut p = ClientPacket::new();
    p.p4(com).p2_alt3(-1).p2_alt1(-1);
    client.send(ClientProt::IfButton1, &p.finish());
    let mut texts = Vec::new();
    let mut said = Vec::new();
    for _ in 0..2 {
        engine.cycle();
        let packets = client.packets();
        texts.extend(settexts(&packets));
        said.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, m)| m),
        );
    }
    (texts, said)
}

/// The journal's text once View is clicked on the quest's row, its colour tags taken off.
fn journal(engine: &mut Engine, client: &mut HarnessClient) -> String {
    let mut p = ClientPacket::new();
    p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(TRIBAL_TOTEM);
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

/// Kangai Mau stands in Brimhaven's restaurant, where 289 has him and the 2011 wiki says, and
/// starts the quest.
#[test]
fn kangai_mau_asks_for_the_totem() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tt_start", 0);
        let spawned = npcs(engine, "kangai_mau");
        assert_eq!(spawned.len(), 1, "Kangai Mau is spawned once");
        let lines = journal(engine, &mut client);
        assert!(
            lines.contains("I can start this quest by speaking to Kangai Mau"),
            "{lines}"
        );

        let t = talk(
            engine,
            &mut client,
            "kangai_mau",
            &[
                "I'm in search of adventure!",
                "Why does he have it?",
                "Ok, I will get it back.",
            ],
        );
        says(
            &t,
            &[
                "Hello. I Kangai Mau of the Rantuki tribe.",
                "Adventure is something I may be able to give.",
                "We need it back.",
                "Best of luck with that adventurer.",
            ],
        );
        assert_eq!(var(engine, &client, "totemquest"), STARTED);

        let t = talk(engine, &mut client, "kangai_mau", &[]);
        says(
            &t,
            &[
                "Have you got our totem back?",
                "No, it's not that easy.",
                "Bah, you no good.",
            ],
        );
        no_errors(engine);
    });
}

/// The gardener gives the lock away, the depot's crate gives up its label, the label goes on
/// the crate with the block in it, and an employee sends it on.
#[test]
fn the_label_sends_the_block_to_the_mansion() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tt_label", STARTED);
        let t = talk(
            engine,
            &mut client,
            "horacio",
            &[
                "So... who are you?",
                "So... do you garden round the back too?",
            ],
        );
        says(
            &t,
            &[
                "My name is Horacio Dobson.",
                "It's rather easy, it's his middle name.",
                "Hum. I probably shouldn't have said that. Forget I mentioned it.",
            ],
        );

        tele(engine, &mut client, DEPOT_ARRIVAL, 0);
        let t = op_loc2(engine, &mut client, "horncrate", DEPOT_HORN_CRATE);
        says(
            &t,
            &["There is a label on this crate. It says; To Lord Handelmort"],
        );
        tells(&t, "This crate is securely fastened shut.");
        assert_eq!(held(engine, &client, "tribal_totem_label"), 1);
        let t = op_loc2(engine, &mut client, "horncrate", DEPOT_HORN_CRATE);
        says(
            &t,
            &["You can see the gluey outline from where you peeled the address label off."],
        );
        assert_eq!(held(engine, &client, "tribal_totem_label"), 1);

        let t = op_loc2(engine, &mut client, "teleportcrate", DEPOT_BLOCK_CRATE);
        says(&t, &["Senior Patents Clerk, Chamber of Invention"]);
        assert!(
            !t.said("To Lord Handelmort, Handelmort Mansion, Ardougne."),
            "not relabelled yet: {:?}",
            t.texts
        );

        let t = use_on_loc(
            engine,
            &mut client,
            "tribal_totem_label",
            "teleportcrate",
            DEPOT_BLOCK_CRATE,
        );
        says(
            &t,
            &[
                "You carefully place the delivery label over the existing label",
                "Now I just need someone to deliver it for me.",
            ],
        );
        assert_eq!(var(engine, &client, "totemquest"), CRATE_MARKED);
        assert_eq!(held(engine, &client, "tribal_totem_label"), 0);
        let t = op_loc2(engine, &mut client, "teleportcrate", DEPOT_BLOCK_CRATE);
        says(&t, &["To Lord Handelmort, Handelmort Mansion, Ardougne."]);

        let t = talk(
            engine,
            &mut client,
            "rpdt_employee",
            &["So, when are you going to deliver this crate?"],
        );
        says(
            &t,
            &["Welcome to RPDT!", "Well... I guess we could do it now..."],
        );
        assert_eq!(var(engine, &client, "totemquest"), CRATE_DELIVERED);
        no_errors(engine);
    });
}

/// Cromperty's teleport lands beside the block's crate: in the depot until the crate is
/// delivered, in the mansion after. Cache 786 has both crates off 289's tiles.
#[test]
fn cromperty_teleports_to_the_blocks_crate() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let ask = [
            "So what have you invented?",
            "Can I be teleported please?",
            "Yes, that sounds good. Teleport me!",
        ];
        let mut client = login(engine, "tt_tele", STARTED);
        let t = talk(engine, &mut client, "ardounge_wizard", &ask);
        says(&t, &["My name is Cromperty.", "Okey dokey! Ready?"]);
        settle(engine, &mut client, 8);
        assert_eq!(
            coord(engine, &client),
            DEPOT_ARRIVAL,
            "beside the depot's crate"
        );
        assert_eq!(var(engine, &client, "totemquest"), STARTED);

        set_var(engine, &client, "totemquest", CRATE_DELIVERED);
        talk(engine, &mut client, "ardounge_wizard", &ask);
        settle(engine, &mut client, 8);
        assert_eq!(
            coord(engine, &client),
            MANSION_ARRIVAL,
            "beside the mansion's crate"
        );
        assert_eq!(var(engine, &client, "totemquest"), TELEPORTED);

        // After the quest he finds no signal.
        set_var(engine, &client, "totemquest", COMPLETE);
        let t = talk(engine, &mut client, "ardounge_wizard", &ask);
        says(
            &t,
            &["Hmm.... that's odd... I can't seem to get a signal..."],
        );
        no_errors(engine);
    });
}

/// The mansion: the combination lock takes K, U, R, T; the stairs drop a player who has not
/// found their trap into the sewers; the chest upstairs holds the totem; the front door lets
/// out and not in.
#[test]
fn the_mansions_lock_stairs_and_chest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tt_mansion", TELEPORTED);
        assert!(walkable(TRAP_FALL.0, 0, TRAP_FALL.1), "the trap's landing");

        // The lock: Enter with the letters at A, A, A, A is wrong.
        tele(engine, &mut client, LOCK_DOOR, 0);
        op_loc1(engine, &mut client, "combodoor", LOCK_DOOR);
        settle(engine, &mut client, 2);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(LOCK),
            "the lock's screen"
        );
        assert_eq!(coord(engine, &client), LOCK_DOOR, "the door stays shut");
        let (_, said) = lock_click(engine, &mut client, LOCK_ENTER);
        assert!(
            said.iter().any(|m| m == "This combination is incorrect."),
            "{said:?}"
        );

        // K, U, R, T: 10 and 17 to the right, 6 and 7 to the left (U and T from A backwards).
        op_loc1(engine, &mut client, "combodoor", LOCK_DOOR);
        settle(engine, &mut client, 2);
        let mut shown = [String::new(), String::new(), String::new(), String::new()];
        for (letter, clicks, right) in [(0, 10, true), (1, 6, false), (2, 17, true), (3, 7, false)]
        {
            let com = LOCK_LEFT_A + 2 * letter + i32::from(right);
            for _ in 0..clicks {
                let (texts, _) = lock_click(engine, &mut client, com);
                for (c, t) in texts {
                    if c == LOCK_LETTER_A + letter as u32 {
                        shown[letter as usize] = t;
                    }
                }
            }
        }
        assert_eq!(shown, ["K", "U", "R", "T"], "the letters the screen shows");
        let (_, said) = lock_click(engine, &mut client, LOCK_ENTER);
        assert!(
            said.iter().any(|m| m == "The combination seems correct!"),
            "{said:?}"
        );
        settle(engine, &mut client, 2);
        op_loc1(engine, &mut client, "combodoor", LOCK_DOOR);
        assert_eq!(
            coord(engine, &client),
            (LOCK_DOOR.0 - 1, LOCK_DOOR.1),
            "through the combination door"
        );
        settle(engine, &mut client, 6);

        // The stairs, their trap not found: a fall into the sewers.
        let before = life_points(engine, &client);
        let mut p = ClientPacket::new();
        p.p2_alt1(STAIRS.1 as i32)
            .p2_alt1(id(engine, "loc", "totemtrapstairs") as i32)
            .p1(0)
            .p2_alt2(STAIRS.0 as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        let mut said = Vec::new();
        for _ in 0..16 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, m)| m),
            );
        }
        for line in [
            "As you climb the stairs you hear a click...",
            "You have fallen through a trap!",
        ] {
            assert!(said.iter().any(|m| m == line), "{line:?}: {said:?}");
        }
        assert_eq!(coord(engine, &client), TRAP_FALL, "in the sewers");
        let lost = before - life_points(engine, &client);
        let fall = (before / 50 + 1) * 10;
        assert!(
            lost == fall || lost == fall - 1,
            "289's fifth of the hitpoints and one, in life points: {lost} of {before}"
        );

        // Investigated without the Thieving for it, the stairs show nothing.
        let foot = (STAIRS.0 + 2, STAIRS.1);
        tele(engine, &mut client, foot, 0);
        let t = op_loc2(engine, &mut client, "totemtrapstairs", STAIRS);
        tells(&t, "You don't find anything interesting");
        {
            let player = engine.harness_player_mut(client.pid).unwrap();
            player.stats.levels[THIEVING] = 21;
            player.stats.base_levels[THIEVING] = 21;
        }
        let t = op_loc2(engine, &mut client, "totemtrapstairs", STAIRS);
        says(&t, &["Your trained senses as a thief enable you"]);

        let mut p = ClientPacket::new();
        p.p2_alt1(STAIRS.1 as i32)
            .p2_alt1(id(engine, "loc", "totemtrapstairs") as i32)
            .p1(0)
            .p2_alt2(STAIRS.0 as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        let mut said = Vec::new();
        for _ in 0..8 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, m)| m),
            );
        }
        assert!(
            said.iter().any(|m| m == "You climb up the stairs."),
            "{said:?}"
        );
        assert_eq!(level(engine, &client), 1, "upstairs");
        let (x, z) = coord(engine, &client);
        assert!(walkable(x, 1, z), "on a floor tile upstairs: ({x}, {z})");

        // The chest, from its west: opened, searched, and empty to a player who has the totem.
        tele(engine, &mut client, (CHEST.0 - 1, CHEST.1), 1);
        let t = op_loc1(engine, &mut client, "totemshutchest", CHEST);
        tells(&t, "You open the chest.");
        let t = op_loc1(engine, &mut client, "totemopenchest", CHEST);
        says(&t, &["Inside the chest you find the tribal totem."]);
        assert_eq!(held(engine, &client, "tribal_totem"), 1);
        op_loc1(engine, &mut client, "totemshutchest", CHEST);
        let t = op_loc1(engine, &mut client, "totemopenchest", CHEST);
        tells(&t, "The chest is empty.");
        assert_eq!(held(engine, &client, "tribal_totem"), 1);

        // The front door: shut to the street, open from the hall.
        tele(engine, &mut client, FRONT_DOOR, 0);
        let t = op_loc1(engine, &mut client, "tribaltotemdoor", FRONT_DOOR);
        tells(&t, "This door is securely locked.");
        assert_eq!(coord(engine, &client), FRONT_DOOR);
        let hall = (FRONT_DOOR.0, FRONT_DOOR.1 + 1);
        tele(engine, &mut client, hall, 0);
        op_loc1(engine, &mut client, "tribaltotemdoor", FRONT_DOOR);
        assert_eq!(coord(engine, &client), FRONT_DOOR, "let out");
        settle(engine, &mut client, 6);
        no_errors(engine);
    });
}

/// Kangai Mau takes the totem: 1 quest point, 1,775 Thieving xp and 5 swordfish (2011 wiki,
/// Tribal Totem).
#[test]
fn kangai_mau_rewards_the_totem() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tt_reward", TELEPORTED);
        give(engine, &client, 0, "tribal_totem");
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[THIEVING];
        let qp = var(engine, &client, "qp");
        let mut t = talk(engine, &mut client, "kangai_mau", &[]);
        for _ in 0..10 {
            engine.cycle();
            let packets = client.packets();
            t.texts
                .extend(settexts(&packets).into_iter().map(|(_, t)| t));
        }
        says(
            &t,
            &[
                "Yes I have.",
                "You hand over the Tribal Totem.",
                "You have completed the Tribal Totem Quest!",
                "1,775 Thieving XP",
                "5 Swordfish",
            ],
        );
        assert_eq!(var(engine, &client, "totemquest"), COMPLETE);
        assert_eq!(held(engine, &client, "tribal_totem"), 0);
        assert_eq!(held(engine, &client, "swordfish"), 5);
        let gained = engine.harness_player(client.pid).unwrap().stats.xp[THIEVING] - xp;
        assert_eq!(gained, 17_750, "1,775 Thieving xp (tenths)");
        assert_eq!(var(engine, &client, "qp") - qp, 1, "1 quest point");
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);
        let lines = journal(engine, &mut client);
        assert!(lines.contains("QUEST COMPLETE!"), "{lines}");
        let t = talk(engine, &mut client, "kangai_mau", &[]);
        says(&t, &["Many greetings esteemed thief."]);
        no_errors(engine);
    });
}
