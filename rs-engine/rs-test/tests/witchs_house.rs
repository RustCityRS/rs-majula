//! Witch's House at 647 (`quests/quest_ball`, the boy in `areas/area_taverly`): 289's quest,
//! which 2011 still has. The 2011 wiki (Witch's House, 2011-05-20) gives the reward, 4 quest
//! points and 6,325 Constitution xp, and the gloves the basement's gate takes; its Witch's
//! experiment page (2011-05-16) gives the last form's 550 life points. Cache 786's map has the
//! reworked house: the doors, the gate, the mouse hole and the hedges are where it puts them.

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
use rs_test::talk::{Conversation, converse, settexts, talk_to};
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const ATTACK: usize = 0;
const HITPOINTS: usize = 3;
const BACKPACK: i32 = 149 << 16;
/// View on the quest list's row (questlist_v4:quests) and the journal's first line (qj1).
const QUEST_LIST: i32 = 190 << 16 | 18;
const WITCHS_HOUSE: i32 = 109;
const JOURNAL_LINE_1: u32 = 275 << 16 | 16;
/// The worn inventory's hands slot (`^wearpos_hands`).
const HANDS: u16 = 9;

/// `^ball_started` .. `^ball_complete`.
const STARTED: i32 = 1;
const FOUND_MAGNET: i32 = 2;
const UNLOCKED: i32 = 3;
const READ_DIARY: i32 = 5;
const DEFEATED: i32 = 6;
const COMPLETE: i32 = 7;

/// maps/l45_54.jm2 and, for the basement, maps/l45_154.jm2.
const BOY: (u16, u16) = (2928, 3456);
const DROP_OFF: (u16, u16) = (2929, 3456);
const POT: (u16, u16) = (2900, 3474);
const FRONT_DOOR: (u16, u16) = (2900, 3473);
const BACK_DOOR: (u16, u16) = (2901, 3465);
const MOUSE_HOLE: (u16, u16) = (2903, 3466);
const FOUNTAIN: (u16, u16) = (2909, 3470);
const SHED_DOOR: (u16, u16) = (2934, 3463);
const BALL: (u16, u16) = (2935, 3460);
const GATE_NORTH: (u16, u16) = (2902, 9874);
const GATE_SOUTH: (u16, u16) = (2902, 9873);
const CUPBOARD: (u16, u16) = (2898, 9873);

const FORMS: [&str; 4] = [
    "shapeshifterglob",
    "shapeshifterspider",
    "shapeshifterbear",
    "shapeshifterwolf",
];

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

fn stage(engine: &Engine, client: &HarnessClient) -> i32 {
    let v = id(engine, "varp", "ballquest");
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(v)
        .as_int()
}

fn set_stage(engine: &mut Engine, client: &HarnessClient, value: i32) {
    let v = id(engine, "varp", "ballquest");
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(v, VarValue::Int(value));
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

/// Levels of 99 in the melee skills and 990 life points, for the shed.
fn fighter(engine: &mut Engine, client: &HarnessClient) {
    let player = engine.harness_player_mut(client.pid).unwrap();
    for skill in [0, 1, 2, 3] {
        player.stats.levels[skill] = 99;
        player.stats.base_levels[skill] = 99;
        player.stats.xp[skill] = 130_344_310;
    }
    let bit0 = player.vars.get(1240).as_int() & 1;
    player.vars.set(1240, VarValue::Int((990 << 1) | bit0));
}

fn settle(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) {
    for _ in 0..ticks {
        engine.cycle();
    }
    client.packets();
}

fn walkable(x: u16, level: u8, z: u16) -> bool {
    !rsmod::is_flagged(x, z, level, 0x100 | 0x200000)
}

fn coord(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

fn tele(engine: &mut Engine, client: &mut HarnessClient, (x, z): (u16, u16)) {
    assert!(walkable(x, 0, z), "({x}, {z}) is walkable");
    engine.harness_telejump(client.pid, x, 0, z);
    settle(engine, client, 3);
}

fn login(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    set_stage(engine, &client, stage);
    client
}

/// Leaves the garden and lets a curse the witch began run out, so her timer is going again
/// for the next case.
fn leave(engine: &mut Engine, client: &mut HarnessClient) {
    tele(engine, client, (BOY.0, BOY.1 + 2));
    settle(engine, client, 8);
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "no script errors: {errors:?}");
}

/// Every spawned npc of that name: its nid and tile.
fn npcs(engine: &Engine, name: &str) -> Vec<(u16, u16, u16)> {
    let want = id(engine, "npc", name);
    engine
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
        .collect()
}

/// Talks to the boy, taking `choices`.
fn talk(engine: &mut Engine, client: &mut HarnessClient, choices: &[&str]) -> Conversation {
    let (nid, x, z) = npcs(engine, "boy")
        .into_iter()
        .next()
        .expect("the boy is spawned");
    let at = [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
        .find(|&(x, z)| walkable(x, 0, z))
        .expect("a walkable tile beside the boy");
    tele(engine, client, at);
    talk_to(client, nid);
    let talk = converse(engine, client, choices, 300);
    assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
    talk
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

/// Take on the ball in the shed (`OPOBJ3`), and what it shows.
fn take_ball(engine: &mut Engine, client: &mut HarnessClient) -> Conversation {
    let mut p = ClientPacket::new();
    p.p2_alt1(id(engine, "obj", "ball") as i32)
        .p2_alt2(BALL.1 as i32)
        .p1(0)
        .p2(BALL.0 as i32);
    client.send(ClientProt::OpObj3, &p.finish());
    converse(engine, client, &[], 12)
}

/// The journal's lines once View is clicked on the quest's row, their colour tags taken off.
fn journal(engine: &mut Engine, client: &mut HarnessClient) -> Vec<String> {
    let mut p = ClientPacket::new();
    p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(WITCHS_HOUSE);
    client.send(ClientProt::IfButton1, &p.finish());
    let mut texts = Vec::new();
    for _ in 0..3 {
        engine.cycle();
        texts.extend(settexts(&client.packets()));
    }
    engine.harness_close_modal(client.pid);
    (0..10)
        .map(|line| {
            let text = texts
                .iter()
                .find(|(c, _)| *c == JOURNAL_LINE_1 + line)
                .map_or("", |(_, t)| t.as_str());
            let mut plain = String::new();
            let mut tag = false;
            for ch in text.chars() {
                match ch {
                    '<' => tag = true,
                    '>' => tag = false,
                    _ if !tag => plain.push(ch),
                    _ => {}
                }
            }
            plain
        })
        .collect()
}

/// The boy and the tile the witch's curse sends a player to stand on open ground at 2011, and
/// he starts the quest with 289's lines.
#[test]
fn the_boy_starts_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wh_boy", 0);
        assert!(walkable(BOY.0, 0, BOY.1), "the boy's tile");
        assert!(walkable(DROP_OFF.0, 0, DROP_OFF.1), "the curse's tile");
        assert!(
            npcs(engine, "boy")
                .iter()
                .any(|&(_, x, z)| x.abs_diff(BOY.0) <= 5 && z.abs_diff(BOY.1) <= 5),
            "the boy stands by the garden"
        );

        let lines = journal(engine, &mut client);
        assert_eq!(
            lines[..3],
            [
                "I can start this quest by speaking to the little boy",
                "standing by the long garden just north of taverley",
                "I must be able to defeat a level 53 enemy",
            ]
        );

        let t = talk(
            engine,
            &mut client,
            &["Well if you're not going to answer, I'll go."],
        );
        says(
            &t,
            &[
                "Hello young man.",
                "The boy sobs.",
                "The boy sniffs slightly.",
            ],
        );
        let t = talk(
            engine,
            &mut client,
            &["What's the matter?", "Get it back yourself."],
        );
        says(
            &t,
            &[
                "I've kicked my ball over that hedge, into that garden!",
                "The old lady who lives there is scary...",
                "She's locked the ball in her wooden shed!",
                "Can you get my ball back for me please?",
                "You're a meany!",
                "The boy starts crying again.",
            ],
        );
        assert_eq!(stage(engine, &client), 0, "not started");

        let t = talk(
            engine,
            &mut client,
            &["What's the matter?", "Ok, I'll see what I can do."],
        );
        says(&t, &["Ok, I'll see what I can do.", "Thanks mister!"]);
        assert_eq!(stage(engine, &client), STARTED);

        let lines = journal(engine, &mut client);
        assert_eq!(
            lines[..3],
            [
                "A small boy has kicked his ball over the fence into the",
                "nearby garden, and I have agreed to retrieve it for him.",
                "I should find a way into the garden where the ball is",
            ]
        );

        let t = talk(engine, &mut client, &[]);
        says(
            &t,
            &[
                "Have you got my ball back yet?",
                "Not yet.",
                "Well, it's in the shed in that garden.",
            ],
        );
        leave(engine, &mut client);
    });
}

/// The key is under the pot, and the front door takes it once the quest is on. Cache 786 has
/// the door on the tile outside the wall (289's stood inside it): the tile west of it is the
/// street, the tile east the hall.
#[test]
fn the_pot_hides_the_key_to_the_front_door() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let outside = FRONT_DOOR;
        let inside = (FRONT_DOOR.0 + 1, FRONT_DOOR.1);

        let mut client = login(engine, "wh_door", 0);
        tele(engine, &mut client, outside);
        let t = op_loc1(engine, &mut client, "witchpot", POT);
        says(&t, &["You find a key hidden under the flower pot."]);
        assert_eq!(held(engine, &client, "witches_doorkey"), 1);
        let t = op_loc1(engine, &mut client, "witchpot", POT);
        tells(&t, "You don't find anything interesting.");
        assert_eq!(held(engine, &client, "witches_doorkey"), 1);

        // Not on the quest: the key does not let a player in.
        let t = op_loc1(engine, &mut client, "witchhousedoor", FRONT_DOOR);
        says(&t, &["It would be rude to break into this house."]);
        assert_eq!(coord(engine, &client), outside);

        set_stage(engine, &client, STARTED);
        op_loc1(engine, &mut client, "witchhousedoor", FRONT_DOOR);
        assert_eq!(coord(engine, &client), inside, "let in with the key");
        settle(engine, &mut client, 6);
        let t = use_on_loc(
            engine,
            &mut client,
            "witches_doorkey",
            "witchhousedoor",
            FRONT_DOOR,
        );
        assert_eq!(coord(engine, &client), outside, "{:?}", t.messages);
        settle(engine, &mut client, 6);
        use_on_loc(
            engine,
            &mut client,
            "witches_doorkey",
            "witchhousedoor",
            FRONT_DOOR,
        );
        assert_eq!(coord(engine, &client), inside, "the key used on the door");
        leave(engine, &mut client);

        // Without the key the door lets a player out, not in.
        let mut client = login(engine, "wh_door_keyless", STARTED);
        tele(engine, &mut client, inside);
        op_loc1(engine, &mut client, "witchhousedoor", FRONT_DOOR);
        assert_eq!(coord(engine, &client), outside, "let out without the key");
        settle(engine, &mut client, 6);
        let t = op_loc1(engine, &mut client, "witchhousedoor", FRONT_DOOR);
        tells(&t, "The door is locked.");
        assert_eq!(coord(engine, &client), outside);
        leave(engine, &mut client);
    });
}

/// The basement: the gate shocks bare hands for a tenth of the player's hitpoints and one
/// more (in 2011's life points), opens to gloved ones, and the cupboard holds the magnet.
#[test]
fn the_basements_gate_shocks_bare_hands_and_the_cupboard_holds_the_magnet() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wh_basement", STARTED);
        // The ladder's side of the gate is east of it.
        let east = (GATE_NORTH.0 + 1, GATE_NORTH.1);
        tele(engine, &mut client, east);
        fighter(engine, &client);
        let before = life_points(engine, &client);
        let gate = id(engine, "loc", "shockgatel");
        let mut p = ClientPacket::new();
        p.p2_alt1(GATE_NORTH.1 as i32)
            .p2_alt1(gate as i32)
            .p1(0)
            .p2_alt2(GATE_NORTH.0 as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        // The shock is the one tick's drop: a regeneration tick may fall among these.
        let mut texts = Vec::new();
        let mut shock = 0;
        let mut was = before;
        for tick in 0..8 {
            // The queued shock waits for the message box to be closed.
            if tick == 3 {
                engine.harness_close_modal(client.pid);
            }
            engine.cycle();
            texts.extend(settexts(&client.packets()).into_iter().map(|(_, t)| t));
            let now = life_points(engine, &client);
            shock = shock.max(was - now);
            was = now;
        }
        assert!(
            texts
                .iter()
                .any(|t| t == "As your bare hands touch the gate you feel a shock."),
            "{texts:?}"
        );
        assert_eq!(
            shock,
            (before / 100 + 1) * 10,
            "289's hitpoints / 10 + 1, in life points (of {before})"
        );
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);
        assert_eq!(coord(engine, &client), east, "the gate stays shut");

        // Each pair of gloves the 2011 wiki lists opens it; 289 has the leather ones alone.
        let worn = *engine.cache.invs.debugnames.get("worn").unwrap();
        for (i, gloves) in [
            "leather_gloves",
            "sops_gloves",
            "pest_void_knight_gloves",
            "secret_ghost_gloves",
            "barbassault_penance_gloves",
            "barbassault_penance_gloves_worn",
        ]
        .into_iter()
        .enumerate()
        {
            let o = id(engine, "obj", gloves);
            engine
                .harness_player_mut(client.pid)
                .unwrap()
                .invs
                .get_mut(&worn)
                .unwrap()
                .set(HANDS, o, 1);
            // The north leaf and the south leaf in turn.
            let (name, gate) = if i % 2 == 0 {
                ("shockgatel", GATE_NORTH)
            } else {
                ("shockgater", GATE_SOUTH)
            };
            tele(engine, &mut client, (gate.0 + 1, gate.1));
            op_loc1(engine, &mut client, name, gate);
            assert_eq!(coord(engine, &client), gate, "{gloves} open the gate");
            settle(engine, &mut client, 6);
            op_loc1(engine, &mut client, name, gate);
            assert_eq!(
                coord(engine, &client),
                (gate.0 + 1, gate.1),
                "and back through it"
            );
            settle(engine, &mut client, 6);
        }

        // The cupboard is opened from its east (forceapproach).
        tele(engine, &mut client, (CUPBOARD.0 + 1, CUPBOARD.1));
        op_loc1(engine, &mut client, "magnetcbshut", CUPBOARD);
        let t = op_loc1(engine, &mut client, "magnetcbopen", CUPBOARD);
        says(&t, &["You find a magnet in the cupboard."]);
        assert_eq!(held(engine, &client, "magnet"), 1);
        assert_eq!(stage(engine, &client), FOUND_MAGNET);
        let t = op_loc1(engine, &mut client, "magnetcbopen", CUPBOARD);
        tells(&t, "You don't find anything interesting.");
        assert_eq!(held(engine, &client, "magnet"), 1);

        // Shut, it is searched no more until opened again.
        op_loc2(engine, &mut client, "magnetcbopen", CUPBOARD);
        let t = op_loc1(engine, &mut client, "magnetcbopen", CUPBOARD);
        assert!(t.messages.is_empty(), "shut: {:?}", t.messages);
        op_loc1(engine, &mut client, "magnetcbshut", CUPBOARD);
        let t = op_loc1(engine, &mut client, "magnetcbopen", CUPBOARD);
        tells(&t, "You don't find anything interesting.");
        leave(engine, &mut client);
    });
}

/// Cheese at the mouse hole, used on it or dropped by it, brings the mouse out; the magnet on
/// the mouse unlocks the back door, which then lets the player into the garden.
#[test]
fn cheese_brings_the_mouse_and_the_magnet_unlocks_the_back_door() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wh_mouse", FOUND_MAGNET);
        give(engine, &client, 0, "cheese");
        give(engine, &client, 1, "cheese");
        give(engine, &client, 2, "magnet");
        let porch = (BACK_DOOR.0, BACK_DOOR.1 + 1);
        tele(engine, &mut client, porch);

        let t = op_loc1(engine, &mut client, "witchbackdoor", BACK_DOOR);
        says(
            &t,
            &[
                "This door is locked.",
                "Strange... I can't see any kind of lock or handle to open this door...",
            ],
        );
        assert_eq!(coord(engine, &client), porch);

        let t = use_on_loc(engine, &mut client, "cheese", "witchmousehole", MOUSE_HOLE);
        says(&t, &["A mouse runs out of the hole."]);
        assert_eq!(held(engine, &client, "cheese"), 1);
        let mice = npcs(engine, "witchrat");
        assert_eq!(mice.len(), 1, "one mouse: {mice:?}");
        assert_eq!((mice[0].1, mice[0].2), MOUSE_HOLE, "at its hole");

        // A magnet from before the quest's cupboard search does not do.
        set_stage(engine, &client, STARTED);
        let t = use_on_npc(engine, &mut client, "magnet", mice[0].0);
        says(
            &t,
            &["This doesn't seem to be the right magnet to open this door..."],
        );
        assert_eq!(stage(engine, &client), STARTED);
        set_stage(engine, &client, FOUND_MAGNET);

        // Dropped by the hole, the cheese brings a new mouse for the old.
        let mut p = ClientPacket::new();
        p.p4(BACKPACK)
            .p2_alt3(id(engine, "obj", "cheese") as i32)
            .p2_alt1(1);
        client.send(ClientProt::IfButton8, &p.finish());
        let t = converse(engine, &mut client, &[], 12);
        says(&t, &["A mouse runs out of a hole."]);
        assert_eq!(held(engine, &client, "cheese"), 0);
        let mice = npcs(engine, "witchrat");
        assert_eq!(mice.len(), 1, "one mouse: {mice:?}");

        let t = use_on_npc(engine, &mut client, "magnet", mice[0].0);
        says(
            &t,
            &[
                "You attach the magnet to the mouse's harness.",
                "There is a strange whirring noise from above the door frame.",
            ],
        );
        assert_eq!(stage(engine, &client), UNLOCKED);
        assert_eq!(held(engine, &client, "magnet"), 0);
        assert!(npcs(engine, "witchrat").is_empty(), "the mouse runs back");

        tele(engine, &mut client, porch);
        op_loc1(engine, &mut client, "witchbackdoor", BACK_DOOR);
        assert_eq!(coord(engine, &client), BACK_DOOR, "out into the garden");
        leave(engine, &mut client);
    });
}

/// The fountain gives the shed's key, the key lets a player into the shed where the
/// experiment waits, and the ball is not taken while it lives.
#[test]
fn the_fountain_hides_the_shed_key_and_the_experiment_guards_the_ball() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wh_shed", UNLOCKED);
        fighter(engine, &client);
        tele(engine, &mut client, (FOUNTAIN.0 - 1, FOUNTAIN.1));
        let t = op_loc2(engine, &mut client, "witchfountain", FOUNTAIN);
        says(
            &t,
            &[
                "You search for the secret compartment mentioned in the diary. Inside it you find a small key. You take the key.",
            ],
        );
        assert_eq!(held(engine, &client, "witches_shedkey"), 1);
        let t = op_loc2(engine, &mut client, "witchfountain", FOUNTAIN);
        tells(&t, "There is nothing in the fountain.");
        assert_eq!(held(engine, &client, "witches_shedkey"), 1);

        let outside = (SHED_DOOR.0 - 1, SHED_DOOR.1);
        tele(engine, &mut client, outside);
        let t = op_loc1(engine, &mut client, "witchsheddoor", SHED_DOOR);
        tells(&t, "The shed door is locked.");
        assert_eq!(coord(engine, &client), outside);

        use_on_loc(
            engine,
            &mut client,
            "witches_shedkey",
            "witchsheddoor",
            SHED_DOOR,
        );
        assert_eq!(coord(engine, &client), SHED_DOOR, "into the shed");
        let globs = npcs(engine, "shapeshifterglob");
        assert_eq!(globs.len(), 1, "the experiment: {globs:?}");
        assert_eq!((globs[0].1, globs[0].2), (2935, 3462), "289's tile");

        let t = take_ball(engine, &mut client);
        tells(
            &t,
            "The shapeshifter glares at you. You feel slightly weakened.",
        );
        assert_eq!(held(engine, &client, "ball"), 0, "the ball stays");
        let level = engine.harness_player(client.pid).unwrap().stats.levels[ATTACK];
        assert!(level < 99, "attack is drained: {level}");

        // Leaving and unlocking the door again swaps the experiment for a fresh one.
        settle(engine, &mut client, 4);
        engine.harness_telejump(client.pid, outside.0, 0, outside.1);
        settle(engine, &mut client, 3);
        use_on_loc(
            engine,
            &mut client,
            "witches_shedkey",
            "witchsheddoor",
            SHED_DOOR,
        );
        let all: usize = FORMS.iter().map(|f| npcs(engine, f).len()).sum();
        assert_eq!(all, 1, "one experiment");
        leave(engine, &mut client);
    });
}

/// The experiment dies four times, a form stronger each time, and only then is the ball the
/// player's to take.
#[test]
fn the_experiment_takes_four_forms_before_the_ball_is_free() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wh_fight", UNLOCKED);
        fighter(engine, &client);
        give(engine, &client, 0, "witches_shedkey");
        tele(engine, &mut client, (SHED_DOOR.0 - 1, SHED_DOOR.1));
        use_on_loc(
            engine,
            &mut client,
            "witches_shedkey",
            "witchsheddoor",
            SHED_DOOR,
        );
        assert_eq!(coord(engine, &client), SHED_DOOR, "into the shed");

        let last = "You finally kill the shapeshifter once and for all.";
        let mut said: Vec<String> = Vec::new();
        let mut seen = Vec::new();
        let mut target = None;
        for tick in 0..400 {
            let alive = FORMS
                .iter()
                .find_map(|f| npcs(engine, f).first().map(|&(nid, ..)| (*f, nid)));
            if let Some((form, nid)) = alive {
                if seen.last() != Some(&form) {
                    seen.push(form);
                }
                // 1 life point, and defence 1 so a swing lands: the test is each death's
                // trigger, not the fight's dice.
                let stats = &mut engine.get_npc_mut(nid).unwrap().npc.stats.levels;
                stats[NpcStat::Hitpoints as usize] = 1;
                stats[NpcStat::Defence as usize] = 1;
                if target != Some(nid) || tick % 8 == 0 {
                    target = Some(nid);
                    let mut p = ClientPacket::new();
                    p.p1_alt2(0).p2_alt3(nid as i32);
                    client.send(ClientProt::OpNpc2, &p.finish());
                }
            }
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, t)| t),
            );
            if said.iter().any(|m| m == last) {
                break;
            }
        }
        assert_eq!(seen, FORMS, "each form in turn");
        let lines: Vec<&str> = said
            .iter()
            .map(String::as_str)
            .filter(|m| m.contains("shapeshifter"))
            .collect();
        assert_eq!(
            lines,
            [
                "The shapeshifters' body begins to deform!",
                "The shapeshifter turns into a spider!",
                "The shapeshifters' body begins to twist!",
                "The shapeshifter turns into a bear!",
                "The shapeshifters' body pulses!",
                "The shapeshifter turns into a wolf!",
                last,
            ]
        );
        assert_eq!(stage(engine, &client), DEFEATED);
        settle(engine, &mut client, 3);

        take_ball(engine, &mut client);
        assert_eq!(held(engine, &client, "ball"), 1, "the boy's ball");

        // The shed's door now opens without the key.
        tele(engine, &mut client, SHED_DOOR);
        op_loc1(engine, &mut client, "witchsheddoor", SHED_DOOR);
        assert_eq!(
            coord(engine, &client),
            (SHED_DOOR.0 - 1, SHED_DOOR.1),
            "out of the shed"
        );
        leave(engine, &mut client);
    });
}

/// The four forms' stats: 289's levels, its hitpoints as life points, and the 2011 wiki's 550
/// for the last.
#[test]
fn the_experiments_forms_have_2011s_life_points() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        for (form, life, attack) in [
            ("shapeshifterglob", 210, 18),
            ("shapeshifterspider", 310, 28),
            ("shapeshifterbear", 410, 38),
            ("shapeshifterwolf", 550, 48),
        ] {
            let t = engine
                .cache
                .npcs
                .get_by_id(id(engine, "npc", form))
                .expect(form);
            assert_eq!(t.hitpoints, life, "{form}'s life points");
            assert_eq!(t.attack, attack, "{form}'s attack");
            assert!(t.category.is_some(), "{form} is a witches_experiment");
        }
    });
}

/// The witch curses a player she sees in her garden back to the boy, taking the shed's key
/// and the ball and locking the back door again; a hedge between them hides the player. Cache
/// 786's map ends its hedge runs in hedge_term, which hides as the hedge does.
#[test]
fn the_witch_catches_a_player_in_the_open_and_not_behind_a_hedge() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // The hedges the witch's script looks for are in the zones (maps/l45_54.jm2).
        for (name, x, z) in [
            ("hedge", 2916, 3461),
            ("hedge_term", 2915, 3461),
            ("hedgecorner", 2902, 3461),
        ] {
            let loc = id(engine, "loc", name);
            assert!(
                engine
                    .zones
                    .zone(x, 0, z)
                    .and_then(|zone| zone.get_loc(x, z, loc))
                    .is_some(),
                "{name} at ({x}, {z})"
            );
        }

        let mut client = login(engine, "wh_witch", UNLOCKED);
        give(engine, &client, 0, "witches_shedkey");
        give(engine, &client, 1, "ball");

        // South of a hedge_term on her patrol's line: she passes both ways and sees nothing.
        let hidden = (2915, 3460);
        tele(engine, &mut client, hidden);
        let mut nearest = u16::MAX;
        for tick in 0..80 {
            if tick % 20 == 0 {
                client.send(ClientProt::NoTimeout, &[]);
            }
            engine.cycle();
            client.packets();
            let (_, x, _) = npcs(engine, "nora_t_hagg")[0];
            nearest = nearest.min(x.abs_diff(hidden.0));
            assert_eq!(coord(engine, &client), hidden, "hidden behind the hedge");
        }
        assert!(nearest <= 1, "she walked past: {nearest}");
        assert_eq!(stage(engine, &client), UNLOCKED);

        // Three tiles west there is no hedge between them.
        let open = (2912, 3460);
        tele(engine, &mut client, open);
        for tick in 0..80 {
            if tick % 20 == 0 {
                client.send(ClientProt::NoTimeout, &[]);
            }
            engine.cycle();
            client.packets();
            if coord(engine, &client) == DROP_OFF {
                break;
            }
        }
        assert_eq!(coord(engine, &client), DROP_OFF, "cursed back to the boy");
        assert_eq!(stage(engine, &client), STARTED, "the back door locks again");
        assert_eq!(held(engine, &client, "witches_shedkey"), 0);
        assert_eq!(held(engine, &client, "ball"), 0);
        settle(engine, &mut client, 3);

        // Having read the diary after unlocking the door, the stage is kept (289).
        set_stage(engine, &client, READ_DIARY);
        tele(engine, &mut client, open);
        for tick in 0..80 {
            if tick % 20 == 0 {
                client.send(ClientProt::NoTimeout, &[]);
            }
            engine.cycle();
            client.packets();
            if coord(engine, &client) == DROP_OFF {
                break;
            }
        }
        assert_eq!(coord(engine, &client), DROP_OFF, "cursed again");
        assert_eq!(stage(engine, &client), READ_DIARY);
        leave(engine, &mut client);
    });
}

/// Read opens the diary on 2011's book, and reading it once the back door is unlocked moves
/// the quest on a stage (289).
#[test]
fn the_diary_opens_as_a_book() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wh_diary", FOUND_MAGNET);
        give(engine, &client, 0, "witches_diary");
        let read = |engine: &mut Engine, client: &mut HarnessClient| {
            let mut p = ClientPacket::new();
            p.p4(BACKPACK)
                .p2_alt3(id(engine, "obj", "witches_diary") as i32)
                .p2_alt1(0);
            client.send(ClientProt::IfButton1, &p.finish());
            let mut texts = Vec::new();
            for _ in 0..3 {
                engine.cycle();
                texts.extend(settexts(&client.packets()).into_iter().map(|(_, t)| t));
            }
            engine.harness_close_modal(client.pid);
            settle(engine, client, 2);
            texts
        };
        let texts = read(engine, &mut client);
        for line in ["Witches' Diary", "2nd of Pentember", "3rd of Pentember"] {
            assert!(
                texts.iter().any(|t| t.contains(line)),
                "{line:?}: {texts:?}"
            );
        }
        assert_eq!(stage(engine, &client), FOUND_MAGNET);

        set_stage(engine, &client, UNLOCKED);
        read(engine, &mut client);
        assert_eq!(stage(engine, &client), READ_DIARY);
        leave(engine, &mut client);
    });
}

/// The boy takes his ball: 4 quest points and 6,325 Constitution xp (2011 wiki, Witch's
/// House, 2011-05-20).
#[test]
fn the_boy_takes_his_ball_back() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wh_reward", DEFEATED);
        let lines = journal(engine, &mut client);
        assert_eq!(
            lines[2],
            "Now the shapeshifter is dead I should return the boy's ball"
        );
        give(engine, &client, 0, "ball");
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[HITPOINTS];
        let qp = {
            let v = id(engine, "varp", "qp");
            engine
                .harness_player(client.pid)
                .unwrap()
                .vars
                .get(v)
                .as_int()
        };
        let mut t = talk(engine, &mut client, &[]);
        for _ in 0..10 {
            engine.cycle();
            let packets = client.packets();
            t.texts
                .extend(settexts(&packets).into_iter().map(|(_, t)| t));
        }
        says(
            &t,
            &[
                "Hi, I have got your ball back. It was MUCH harder than I thought it would be.",
                "You give the ball back.",
                "Thank you so much!",
                "You have completed the Witch's House Quest!",
                "6,325 Constitution XP",
            ],
        );
        assert_eq!(held(engine, &client, "ball"), 0);
        assert_eq!(stage(engine, &client), COMPLETE);
        let gained = engine.harness_player(client.pid).unwrap().stats.xp[HITPOINTS] - xp;
        assert_eq!(gained, 63_250, "6,325 Constitution xp (tenths)");
        let v = id(engine, "varp", "qp");
        let now = engine
            .harness_player(client.pid)
            .unwrap()
            .vars
            .get(v)
            .as_int();
        assert_eq!(now - qp, 4, "4 quest points");

        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);
        let lines = journal(engine, &mut client);
        assert!(lines.iter().any(|l| l == "QUEST COMPLETE!"), "{lines:?}");
        let t = talk(engine, &mut client, &[]);
        says(&t, &["Thank you for getting my ball back!"]);
        leave(engine, &mut client);
    });
}
