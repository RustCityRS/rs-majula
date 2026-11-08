//! Dragon Slayer at 647 (`quests/quest_dragon` and its npcs): 289's quest with 2011's lines from
//! the 2011-05-04 recordings (YouTube UYp0v2Pc1yI and mYqjoMQCOHI, TimeRunescape; transcripts in
//! runescape/target/refs/dragon). Stage one: the Guildmaster, Oziach, the Duke's shield, and the
//! quest's end at Oziach with Elvarg's head.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{Conversation, converse, talk_to};

static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

const BACKPACK: i32 = 149 << 16;

fn id(engine: &Engine, kind: &str, name: &str) -> u16 {
    let map = match kind {
        "obj" => &engine.cache.objs.debugnames,
        "npc" => &engine.cache.npcs.debugnames,
        "varp" => &engine.cache.varps.debugnames,
        _ => unreachable!(),
    };
    *map.get(name).unwrap_or_else(|| panic!("no {kind} {name}"))
}

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

fn give(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str, count: u32) {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap()
        .set(slot, o, count);
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

fn tele(engine: &mut Engine, client: &mut HarnessClient, x: u16, level: u8, z: u16) {
    engine.harness_telejump(client.pid, x, level, z);
    settle(engine, client, 3);
}

fn says(t: &Conversation, lines: &[&str]) {
    for line in lines {
        assert!(
            t.said(line),
            "2011 shows {line:?}; {:?} (chat {:?})",
            t.texts,
            t.messages
        );
    }
}

fn login(engine: &mut Engine, name: &str) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    client
}

/// The nid of the nearest npc of that name to (x, z), and a walkable tile beside it on `level`.
fn npc_near_on(engine: &Engine, name: &str, x: u16, z: u16, level: u8) -> (u16, u16, u16) {
    let want = id(engine, "npc", name);
    let (nid, nx, nz) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == want)
        .map(|n| {
            (
                n.npc.uid.nid(),
                n.npc.pathing.coord.x(),
                n.npc.pathing.coord.z(),
            )
        })
        .min_by_key(|&(_, nx, nz)| nx.abs_diff(x) as u32 + nz.abs_diff(z) as u32)
        .unwrap_or_else(|| panic!("{name} is spawned"));
    let (sx, sz) = [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dz)| (nx.wrapping_add_signed(dx), nz.wrapping_add_signed(dz)))
        .find(|&(x, z)| !rsmod::is_flagged(x, z, level, 0x100 | 0x200000))
        .unwrap_or_else(|| panic!("a walkable tile beside {name}"));
    (nid, sx, sz)
}

fn talk_on(
    engine: &mut Engine,
    client: &mut HarnessClient,
    name: &str,
    (x, z, level): (u16, u16, u8),
    choices: &[&str],
) -> Conversation {
    let (nid, sx, sz) = npc_near_on(engine, name, x, z, level);
    tele(engine, client, sx, level, sz);
    talk_to(client, nid);
    converse(engine, client, choices, 300)
}

const GUILDMASTER: (u16, u16, u8) = (3191, 3362, 0);
const OZIACH: (u16, u16, u8) = (3068, 3516, 0);
const DUKE: (u16, u16, u8) = (3212, 3220, 1);

/// The Guildmaster's "Can I have a quest?" and Oziach's quest, in 2011's words (2011-05-04
/// recording UYp0v2Pc1yI, 29.2-90.0s).
#[test]
fn the_guildmaster_and_oziach_start_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ds_start");
        let t = talk_on(
            engine,
            &mut client,
            "guildmaster",
            GUILDMASTER,
            &["Can I have a quest?"],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        says(
            &t,
            &[
                "Greetings!",
                "Aha!",
                "Yes! A mighty and perilous quest fit only for the most powerful champions! And, at the end of it, you will earn the right to wear the legendary rune platebody!",
                "So, what is this quest?",
                "Oziach lives in a hut, by the cliffs to the west of Edgeville. He can be a little...odd...sometimes, though.",
            ],
        );
        assert_eq!(var(engine, &client, "dragonquest"), 1);

        let t = talk_on(
            engine,
            &mut client,
            "oziach",
            OZIACH,
            &[
                "Can you sell me a rune platebody?",
                "The Guildmaster of the Champions' Guild told me.",
                "I thought you were going to give me a quest.",
                "A dragon, that sounds like fun.",
            ],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        says(
            &t,
            &[
                "Good day to you.",
                "So, how does thee know I 'ave some?",
                "Yes, I suppose he would, wouldn't he? He's always sending you fancy-pants 'heroes' up to bother me. Telling me I'll give them a quest or sommat like that.",
                "*Sigh*",
                "Slay the dragon of Crandor!",
                "A dragon, that sounds like fun!",
                "Hah, yes, you are a typical reckless adventurer, aren't you? Now go kill the dragon and get out of my face.",
                "But how can I defeat the dragon?",
            ],
        );
        assert_eq!(var(engine, &client, "dragonquest"), 2);
    });
}

/// After Oziach: Crandor's story the first time, Melzar's piece with the maze key, then "About my
/// quest..." and the breath (UYp0v2Pc1yI 108.4-142.0s, 245.0-273.0s).
#[test]
fn the_guildmaster_tells_of_crandor_and_the_map() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ds_guildmaster");
        set_var(engine, &client, "dragonquest", 2);
        let t = talk_on(
            engine,
            &mut client,
            "guildmaster",
            GUILDMASTER,
            &[
                "I talked to Oziach...",
                "How can I find the route to Crandor?",
                "Where is Melzar's map piece?",
                "Okay, I'll get going!",
            ],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        says(
            &t,
            &[
                "I talked to Oziach and he gave me a quest.",
                "Goodness, he hasn't given you an easy job, has he?",
                "Out of all the people of Crandor there were only three survivors: a trio of wizards who used magic to escape. Their names were: Thalzar, Lozar and Melzar.",
                "If you're serious about taking on Elvarg, first you'll need to get to Crandor. The island is surrounded by dangerous reefs, so you'll need a ship capable of getting through them and a map to show you the way.",
                "Only one map exists that shows the route through the reefs of Crandor. That map was split into three pieces by Melzar, Thalzar and Lozar, the wizards who escaped from the dragon. Each of them took one piece.",
                "Adventurers sometimes go in there to prove themselves, so I can give you this key to Melzar's Maze.",
                "The Guildmaster hands you a key.",
                "Where is Thalzar's map piece?",
            ],
        );
        assert_eq!(held(engine, &client, "maze_key"), 1);

        let t = talk_on(
            engine,
            &mut client,
            "guildmaster",
            GUILDMASTER,
            &[
                "About my quest to kill the dragon...",
                "How can I protect myself from the dragon's breath?",
                "Okay, I'll get going!",
            ],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        assert!(
            !t.said("I talked to Oziach and he gave me a quest."),
            "told once"
        );
        says(
            &t,
            &[
                "That part shouldn't be too difficult, actually. I believe the Duke of Lumbridge has a special shield in his armoury that is enchanted against dragon's breath.",
            ],
        );
        assert_eq!(var(engine, &client, "dragon_shield"), 1);
    });
}

/// Duke Horacio's 2011 shield (UYp0v2Pc1yI 197.4-226.6s), offered once the Guildmaster has told
/// of it.
#[test]
fn the_duke_gives_the_shield() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ds_duke");
        set_var(engine, &client, "dragonquest", 2);
        set_var(engine, &client, "dragon_shield", 1);
        let t = talk_on(
            engine,
            &mut client,
            "duke_horacio",
            DUKE,
            &[
                "I seek a shield that will protect me from dragonbreath.",
                "Elvarg, the dragon of Crandor island!",
                "Yes",
                "So, are you going to give me the shield or not?",
            ],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        says(
            &t,
            &[
                "A knight going on a dragon quest, hmm? What dragon do you intend to slay?",
                "Well, are you sure?",
                "Well, you're a braver man than I!",
                "Back in my father's day, Crandor was an important city-state. Politically, it was as important as Falador or Varrock and its ships traded with every port.",
                "The Duke hands you a heavy orange shield.",
                "If you kill it, for Saradomin's sake make sure it's really dead!",
            ],
        );
        assert_eq!(held(engine, &client, "anti_dragon_shield"), 1);
        assert!(
            t.messages
                .iter()
                .any(|m| m.contains("the Task 'Always Be Prepared'")),
            "the shield completes task 107: {:?}",
            t.messages
        );
    });
}

/// Elvarg's head to Oziach ends the quest (mYqjoMQCOHI 353.6-362.6s): 2011's lines, the head
/// kept, 18,650 Strength and Defence xp, and the quest complete.
#[test]
fn oziach_takes_elvargs_head() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ds_head");
        set_var(engine, &client, "dragonquest", 9);
        give(engine, &client, 0, "elvarg_s_head", 1);
        let t = talk_on(engine, &mut client, "oziach", OZIACH, &[]);
        says(
            &t,
            &[
                "Good day to you.",
                "Have ye slayed that dragon yet?",
                "Yes, I have!",
                "I have its head here.",
                "You actually did it?",
                "Yer a true hero, and I'd be happy to sell ye rune platebodies.",
            ],
        );
        settle(engine, &mut client, 4);
        assert_eq!(var(engine, &client, "dragonquest"), 10, "complete");
        assert_eq!(held(engine, &client, "elvarg_s_head"), 0, "Oziach keeps it");
    });
}

fn oploc1(client: &mut HarnessClient, loc: (u16, u16, u16)) {
    let (id, x, z) = loc;
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(id as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

fn use_on_loc(client: &mut HarnessClient, obj: u16, slot: u16, loc: (u16, u16, u16)) {
    let (id, x, z) = loc;
    let mut p = ClientPacket::new();
    p.p2_alt2(obj as i32)
        .p2_alt2(slot as i32)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt2(x as i32)
        .p1_alt1(0)
        .p2_alt1(id as i32);
    client.send(ClientProt::OpLocT, &p.finish());
}

/// The backpack slot holding `name`.
fn slot_of(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let pack = engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap();
    (0..28u16)
        .find(|&s| pack.get(s).map(|i| i.obj) == Some(o))
        .unwrap_or_else(|| panic!("{name} in the backpack")) as i32
}

/// Uses the backpack's `used` on its `target` (IF_BUTTONT) and answers what follows.
fn use_on_obj(
    engine: &mut Engine,
    client: &mut HarnessClient,
    used: &str,
    target: &str,
    choices: &[&str],
) -> Conversation {
    let (us, ts) = (
        slot_of(engine, client, used),
        slot_of(engine, client, target),
    );
    let mut p = ClientPacket::new();
    p.p4(BACKPACK)
        .p2_alt1(ts)
        .p2_alt1(us)
        .p2_alt2(id(engine, "obj", used) as i32)
        .p4_alt1(BACKPACK)
        .p2(id(engine, "obj", target) as i32);
    client.send(ClientProt::IfButtonT, &p.finish());
    converse(engine, client, choices, 300)
}

const ORACLE: (u16, u16, u8) = (3013, 3501, 0);
/// Thalzar's door (2011's magic_door_25115, the l47_153 map's 42,47) and the chest behind it.
const MAGIC_DOOR: (u16, u16, u16) = (25115, 3050, 9839);
const ORACLE_CHEST: (u16, u16, u16) = (2587, 3057, 9841);

/// The Oracle's riddle, the four items used on Thalzar's door one by one, "The door opens...", and
/// the chest's inscription and piece (2011-05-04 recording UYp0v2Pc1yI, 647.6-714.8s).
#[test]
fn the_oracle_thalzars_door_and_chest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ds_oracle");
        set_var(engine, &client, "dragonquest", 2);
        set_var(engine, &client, "dragon_oracle", 1);
        let t = talk_on(
            engine,
            &mut client,
            "oracle",
            ORACLE,
            &["I seek a piece of the map to the island of Crandor."],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        says(
            &t,
            &[
                "The map's behind a door below,",
                "Last, a bowl that's not seen heat.",
            ],
        );
        assert_eq!(var(engine, &client, "dragon_oracle"), 2);

        tele(engine, &mut client, 3049, 0, 9840);
        let items = [
            ("silk", "You put the silk into the opening in the door."),
            (
                "bowl__unfired",
                "You put the unfired bowl into the opening in the door.",
            ),
            (
                "wizard_s_mind_bomb",
                "You pour the Wizard's Mind Bomb into the opening in the door.",
            ),
            (
                "lobster_pot",
                "You put the small crustacean cage into the opening in the door.",
            ),
        ];
        for (slot, (name, _)) in items.iter().enumerate() {
            give(engine, &client, slot as u16, name, 1);
        }
        for (slot, (name, line)) in items.iter().enumerate() {
            use_on_loc(
                &mut client,
                id(engine, "obj", name),
                slot as u16,
                MAGIC_DOOR,
            );
            let said = messages(engine, &mut client, 6);
            assert!(said.iter().any(|m| m == line), "2011's {line:?}: {said:?}");
            assert_eq!(held(engine, &client, name), 0, "{name} goes into the door");
        }
        oploc1(&mut client, MAGIC_DOOR);
        let said = messages(engine, &mut client, 8);
        assert!(said.iter().any(|m| m == "The door opens..."), "{said:?}");
        let at = engine.harness_player(client.pid).unwrap().pathing.coord;
        assert_eq!(at.x(), 3051, "through the door to the chest's side");
        assert_eq!(var(engine, &client, "dragon_oracle"), 3);

        oploc1(&mut client, ORACLE_CHEST);
        let t = converse(engine, &mut client, &[], 300);
        says(
            &t,
            &[
                "As you open the chest, you notice an inscription on the lid:",
                "Here I rest the map to my beloved home. To whoever finds it, I beg of you, let it be.",
                "There is no shame in backing out now.",
                "You find a map piece in the chest.",
            ],
        );
        assert_eq!(held(engine, &client, "mappart3"), 1);
    });
}

/// The three pieces joined: 2011's item box and line, with no full stop (UYp0v2Pc1yI 720.2s).
#[test]
fn the_three_pieces_make_the_map() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ds_map");
        set_var(engine, &client, "dragonquest", 2);
        give(engine, &client, 0, "map_part", 1);
        give(engine, &client, 1, "mappart2", 1);
        give(engine, &client, 2, "mappart3", 1);
        let t = use_on_obj(engine, &mut client, "map_part", "mappart2", &[]);
        assert!(
            t.said("You put the three pieces together and assemble a map that shows the route through the reefs to Crandor"),
            "{:?}",
            t.texts
        );
        assert_eq!(held(engine, &client, "crandor_map"), 1);
        for piece in ["map_part", "mappart2", "mappart3"] {
            assert_eq!(held(engine, &client, piece), 0, "{piece} joined");
        }
    });
}

fn messages(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) -> Vec<String> {
    let mut out = Vec::new();
    for _ in 0..ticks {
        engine.cycle();
        out.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    out
}
