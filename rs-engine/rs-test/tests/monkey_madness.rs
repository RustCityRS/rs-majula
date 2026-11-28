//! Monkey Madness at 647 (`quests/quest_mm`): 289's quest on 289's tiles. Its varbits are procs on
//! cache 786's varps (cache 786 has no varbit on them), its chapter screens are interface 225
//! (`mm_message`) and its sliding puzzle is interface 287 (`reinitialisation_puzzle`), whose tiles
//! are components with op 1, Slide.

#![cfg(all(since_647, before_742))]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::names::BACKPACK;
use rs_test::runner;
use rs_test::server;
use rs_test::talk::{Conversation, converse, settexts, talk_to};
use rs_test::world;
use rs_ver::TypeId;
use rs_vm::ops::test::{calls_for, forget_calls, set_recording};
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;
/// The quest list's rows, and Monkey Madness's (`^questlist_mm`).
const QUEST_LIST: i32 = 190 << 16 | 18;
const ROW: i32 = 73;
/// `^grandtree_complete` and `^tree_complete`: Narnode only speaks of the 10th squad after both.
const GRANDTREE_COMPLETE: i32 = 160;
const TREE_COMPLETE: i32 = 9;
/// `^mm_started`.
const MM_STARTED: i32 = 1;
/// `^daero_started_reinit`, in bits 7-10 of `%mm_gnomes` (289's `mm_daero` varbit).
const DAERO_STARTED_REINIT: i32 = 5 << 7;
/// Cache 786's reinitialisation puzzle: interface 287, its bottom right tile column5_row5.
const PUZZLE: u16 = 287;
const LAST_TILE: i32 = 287 << 16 | 29;
/// The client var the puzzle's clientscript (mm_reinit_refresh) picks its inv by.
const MM_REINIT_ACTIVE: u16 = 1032;

/// Cache 786's spawns and locs (289's tiles).
const KING: (u16, u16) = (2466, 3497);
/// bunker_controlpanal at 0_37_154_26_27.
const CONTROL_PANEL: (u16, u16) = (2394, 9883);
/// Ardougne Zoo's monkey pen (in 289's greegree zone 0_40_51_32_1..0_41_51_15_23: on Ape Atoll
/// itself the guards jail a human), and Ardougne, outside every greegree zone.
const ZOO_MONKEYS: (u16, u16) = (2631, 3275);
const ARDOUGNE: (u16, u16) = (2662, 3305);

fn id(engine: &Engine, kind: &str, name: &str) -> TypeId {
    let map = match kind {
        "obj" => &engine.cache.objs.debugnames,
        "npc" => &engine.cache.npcs.debugnames,
        "loc" => &engine.cache.locs.debugnames,
        "varp" => &engine.cache.varps.debugnames,
        "inv" => &engine.cache.invs.debugnames,
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

fn login(engine: &mut Engine, name: &str) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    set_var(engine, &client, "grandtree", GRANDTREE_COMPLETE);
    set_var(engine, &client, "treequest", TREE_COMPLETE);
    seasoned(engine, &client);
    client
}

/// Level 99 in every skill (its xp, in tenths) and full life.
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

fn no_errors(engine: &mut Engine) {
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "no script errors: {errors:?}");
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

/// The nearest walkable tile to `by` (the tile itself first). When `beside`, a tile beside `by`
/// that `by` itself can step to (`world::beside`): a tile walkable by its flags can be cut off
/// from it, as (2483, 3485) is from Daero on (2484, 3486) up the Grand Tree, and an op from there
/// ends in "I can't reach that!".
fn walkable_near((x, z): (u16, u16), level: u8, beside: bool) -> (u16, u16) {
    if beside {
        return world::beside(level, x, z).unwrap_or_else(|| panic!("a tile beside ({x}, {z})"));
    }
    for r in 0i16..=4 {
        for dx in -r..=r {
            for dz in -r..=r {
                if dx.abs().max(dz.abs()) != r {
                    continue;
                }
                let (ax, az) = (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz));
                if walkable(ax, level, az) {
                    return (ax, az);
                }
            }
        }
    }
    panic!("a walkable tile near ({x}, {z})");
}

fn tele(engine: &mut Engine, client: &mut HarnessClient, at: (u16, u16), beside: bool) {
    let (x, z) = walkable_near(at, 0, beside);
    engine.harness_telejump(client.pid, x, 0, z);
    settle(engine, client, 3);
}

fn give(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str) {
    let o = id(engine, "obj", name);
    let inv = id(engine, "inv", "inv");
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
    let inv = id(engine, "inv", "inv");
    engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap()
        .total(o) as i32
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

/// What the chat and the boxes are told over `ticks`.
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

/// King Narnode, after The Grand Tree and Tree Gnome Village, tells of the missing 10th squad and
/// starts the quest with the royal seal; the journal then reads the seal's stage back (289's
/// `mm_narnode` varbit, a proc pair on `%mm_gnomes` here).
#[test]
fn king_narnode_starts_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mmstart");
        let want = id(engine, "npc", "grandtree_narnode");
        let (nid, nx, nz) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .filter(|n| n.npc.uid.id() == want)
            .map(|n| {
                let c = n.npc.pathing.coord;
                (n.npc.uid.nid(), c.x(), c.z())
            })
            .min_by_key(|n| n.1.abs_diff(KING.0) + n.2.abs_diff(KING.1))
            .expect("King Narnode is spawned");
        tele(engine, &mut client, (nx, nz), true);
        talk_to(&mut client, nid);
        let t = converse(engine, &mut client, &["Yes"], 400);
        says(
            &t,
            &[
                "King, you look worried. Is anything the matter?",
                "Start Monkey Madness?",
            ],
        );
        assert_eq!(var(engine, &client, "mm_main"), MM_STARTED);
        assert_eq!(held(engine, &client, "mm_gnome_royal_seal"), 1);

        let mut p = ClientPacket::new();
        p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(ROW);
        client.send(ClientProt::IfButton1, &p.finish());
        let t = watch(engine, &mut client, 3);
        assert!(
            t.texts
                .iter()
                .any(|l| l.contains("Narnode has given me a copy of the Royal Seal")),
            "289's journal at the seal's stage: {:?}",
            t.texts
        );
        no_errors(engine);
    });
}

/// A greegree works only in its zones (Ape Atoll, Ardougne Zoo's monkeys): there the player
/// becomes its monkey (cache 786's mm_transmogrification npc), elsewhere nothing happens.
#[test]
fn a_greegree_turns_the_player_into_a_monkey_in_its_zones() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mmgreegree");
        let greegree = id(engine, "obj", "mm_monkey_greegree_for_small_ninja_monkey");
        let monkey = id(engine, "npc", "mm_transmogrification_small_ninja_monkey");

        tele(engine, &mut client, ARDOUGNE, false);
        give(
            engine,
            &client,
            0,
            "mm_monkey_greegree_for_small_ninja_monkey",
        );
        client.if_button(2, BACKPACK, 0, greegree);
        let t = watch(engine, &mut client, 3);
        assert!(
            t.messages
                .iter()
                .any(|m| m == "You attempt to use the Monkey Greegree but nothing happens."),
            "{:?}",
            t.messages
        );
        assert_eq!(engine.harness_player(client.pid).unwrap().transmog, None);

        // The zoo's brown bears share the zone and wander up to the player: a bear's swing makes
        // the player strike back, which as a monkey opens "You cannot attack as a monkey.", and
        // with that box open the greegree's timer cannot take it off.
        world::calm(engine, ZOO_MONKEYS.0, ZOO_MONKEYS.1, 12);
        tele(engine, &mut client, ZOO_MONKEYS, false);
        // Out of combat: the clock is back at 0 after the harness reset, below a saved %lastcombat.
        set_var(engine, &client, "lastcombat", -1);
        client.if_button(2, BACKPACK, 0, greegree);
        settle(engine, &mut client, 3);
        assert_eq!(
            held(engine, &client, "mm_monkey_greegree_for_small_ninja_monkey"),
            0
        );
        assert_eq!(
            engine.harness_player(client.pid).unwrap().transmog,
            Some(monkey.g2()),
            "a small ninja monkey"
        );

        // Out of the zone, the greegree's timer takes it off.
        tele(engine, &mut client, ARDOUGNE, false);
        settle(engine, &mut client, 3);
        assert_eq!(engine.harness_player(client.pid).unwrap().transmog, None);
        no_errors(engine);
    });
}

/// The control panel opens cache 786's puzzle screen (interface 287) on the real puzzle, and its
/// tile's Slide moves it: the last tile into place finishes the puzzle.
#[test]
fn the_reinitialisation_puzzle_slides_on_2011_s_screen() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mmpuzzle");
        set_var(engine, &client, "mm_main", 5);
        set_var(engine, &client, "mm_gnomes", DAERO_STARTED_REINIT);
        tele(engine, &mut client, CONTROL_PANEL, true);

        let panel = id(engine, "loc", "bunker_controlpanal");
        let mut p = ClientPacket::new();
        p.p2_alt1(CONTROL_PANEL.1 as i32)
            .p2_alt1(panel.int())
            .p1(0)
            .p2_alt2(CONTROL_PANEL.0 as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        let mut sent = Vec::new();
        for _ in 0..8 {
            engine.cycle();
            sent.extend(client.packets());
        }
        assert_eq!(
            engine.harness_player(client.pid).unwrap().modal_main,
            Some(PUZZLE),
            "the puzzle screen is open"
        );
        assert!(
            server::varcs(&sent).contains(&(MM_REINIT_ACTIVE, 1)),
            "the screen draws the real puzzle: {:?}",
            server::varcs(&sent)
        );

        // One move from done: tiles 1-23 in place, the gap at slot 23, tile 24 in slot 24.
        let inv = id(engine, "inv", "reinitialisation_inv");
        let tiles: Vec<TypeId> = (1..=24)
            .map(|n| id(engine, "obj", &format!("reinitialisation_{n:02}")))
            .collect();
        {
            let puzzle = engine
                .harness_player_mut(client.pid)
                .unwrap()
                .invs
                .get_mut(&inv)
                .expect("the puzzle's inv");
            puzzle.clear();
            for (slot, tile) in tiles.iter().enumerate().take(23) {
                puzzle.set(slot as u16, *tile, 1);
            }
            puzzle.set(24, tiles[23], 1);
        }
        let mut p = ClientPacket::new();
        p.p4(LAST_TILE).p2_alt3(-1).p2_alt1(-1);
        client.send(ClientProt::IfButton1, &p.finish());
        let t = watch(engine, &mut client, 3);
        assert!(
            t.messages
                .iter()
                .any(|m| m.starts_with("As you slide the final piece into place")),
            "{:?}",
            t.messages
        );
        // 289's mm_hangar_puzzle_complete, bit 4 of %mm_flags.
        assert_eq!(var(engine, &client, "mm_flags") >> 4 & 1, 1);
        no_errors(engine);
    });
}

fn life_points(engine: &Engine, client: &HarnessClient) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(LIFE)
        .as_int()
        >> 1
}

fn set_life_points(engine: &mut Engine, client: &HarnessClient, points: i32) {
    let player = engine.harness_player_mut(client.pid).unwrap();
    let bit0 = player.vars.get(LIFE).as_int() & 1;
    player.vars.set(LIFE, VarValue::Int((points << 1) | bit0));
}

/// The Ape Atoll food heals the 2011 wiki's life points (Monkey bar 90, not 289's 50), and the
/// banana stew leaves its bowl.
#[test]
fn the_ape_atoll_food_heals_2011_s_life_points() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mmfood");
        for (food, heal) in [
            ("mm_monkey_nuts", 40),
            ("mm_monkey_bar", 90),
            ("mm_banana_stew", 110),
        ] {
            set_life_points(engine, &client, 500);
            give(engine, &client, 0, food);
            let obj = id(engine, "obj", food);
            client.if_button(1, BACKPACK, 0, obj);
            let t = watch(engine, &mut client, 4);
            // Natural regeneration may add a life point over the ticks.
            let healed = life_points(engine, &client) - 500;
            assert!(
                (heal..=heal + 1).contains(&healed),
                "{food} heals {heal}: {healed}"
            );
            assert_eq!(
                held(engine, &client, food),
                0,
                "{food} is eaten: {:?}",
                t.messages
            );
        }
        assert_eq!(held(engine, &client, "bowl_empty"), 1, "the stew's bowl");
        no_errors(engine);
    });
}

/// The spawned `name` nearest `near` (any, without it): its nid, tile and level.
fn spawned(engine: &Engine, name: &str, near: Option<(u16, u16)>) -> Option<(u16, u16, u16, u8)> {
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
        .min_by_key(|n| near.map_or(0, |(x, z)| n.1.abs_diff(x) + n.2.abs_diff(z)))
}

/// Beside the spawned `name` nearest `near`, on its level; its nid.
fn stand_beside(
    engine: &mut Engine,
    client: &mut HarnessClient,
    name: &str,
    near: (u16, u16),
) -> u16 {
    let (nid, nx, nz, level) =
        spawned(engine, name, Some(near)).unwrap_or_else(|| panic!("{name} is spawned"));
    let (x, z) = walkable_near((nx, nz), level, true);
    engine.harness_telejump(client.pid, x, level, z);
    settle(engine, client, 3);
    nid
}

/// `^mm_completed_ch3`: the sigil is given, the demon not yet fought.
const MM_COMPLETED_CH3: i32 = 5;

/// The 10th squad's sigil takes the player to the final battle, where the jungle demon spawns and
/// attacks: it casts from 3 tiles or more, and casts or swings from nearer. Defence and Magic are
/// 1, so its spells land, and life is put back each tick. Gives each hit painted on the player
/// over the fight's first ticks (`damage`'s hitmark and amount), once no script has failed.
fn fight_the_jungle_demon(engine: &mut Engine, name: &str) -> Vec<(i32, i32)> {
    let mut client = login(engine, name);
    {
        let player = engine.harness_player_mut(client.pid).unwrap();
        for skill in [1, 6] {
            player.stats.levels[skill] = 1;
            player.stats.base_levels[skill] = 1;
        }
    }
    tele(engine, &mut client, ARDOUGNE, false);
    set_var(engine, &client, "mm_main", MM_COMPLETED_CH3);
    set_var(engine, &client, "lastcombat", -1);
    give(engine, &client, 0, "mm_sigil");
    let sigil = id(engine, "obj", "mm_sigil");
    set_recording(true);
    forget_calls();
    client.if_button(2, BACKPACK, 0, sigil);
    // The sigil's timer, the teleport and the cutscene, to Garkor's "the final battle begins!": the
    // demon spawns as his words close.
    let t = converse(engine, &mut client, &[], 200);
    says(&t, &["Ready yourself, human: the final battle begins!"]);

    let mut messages = t.messages;
    for _ in 0..80 {
        set_life_points(engine, &client, 990);
        engine.cycle();
        messages.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, m)| m),
        );
    }
    let hits = calls_for("damage")
        .iter()
        .map(|hit| {
            let mut n = hit.split(',').map(|n| n.parse::<i32>().unwrap());
            (n.next().unwrap(), n.next().unwrap())
        })
        .collect();
    set_recording(false);
    assert!(
        spawned(engine, "mm_demon", None).is_some(),
        "the jungle demon is there"
    );
    // A script that fails tells its player so (the harness keeps no error log).
    let errors: Vec<&String> = messages
        .iter()
        .filter(|m| m.starts_with("script error"))
        .collect();
    assert!(errors.is_empty(), "the demon's scripts run: {errors:?}");
    no_errors(engine);
    hits
}

/// `^hitmark_magic`, and with `^hitmark_crit` (a hit above nine tenths of the most).
const MAGIC_HITS: [i32; 2] = [2, 12];

/// The spells' hits among a fight's: their amounts.
fn spell_hits(hits: &[(i32, i32)]) -> Vec<i32> {
    hits.iter()
        .filter(|(hitmark, _)| MAGIC_HITS.contains(hitmark))
        .map(|&(_, amount)| amount)
        .collect()
}

/// The jungle demon's wave spells cast and hit while 647's spell table has no graphics for them.
/// Its magic_spell_table rows for the wave spells name spotanims 2011 does not have, so each reads
/// as null; the cast failed on its projectile ("projanim_pl error: Spotanim not found: -1", every
/// cast of a fight in the client) and nothing hit. A null travel or impact is now left unshown
/// and the spell still rolls and hits. The graphics wait on the table's verified 2011 ids, which
/// the demon reads as 289 does; this case does not show a resolved row's being launched (the
/// harness has no row to give it).
#[test]
fn the_jungle_demon_s_spells_cast_and_hit() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let hits = fight_the_jungle_demon(engine, "mmdemon");
        assert!(!spell_hits(&hits).is_empty(), "a spell lands: {hits:?}");
    });
}

/// The demon's spells hit in life points, with 2011's magic hitmark: 289's most, 31 hitpoints, is
/// 310 life points (the 2011 wiki's Jungle demon page, 2011-05-22: "max hit = 320 (approx)"), so
/// over a fight's casts one takes more than 289's whole 31.
#[test]
fn the_jungle_demon_s_spells_hit_in_life_points() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let hits = fight_the_jungle_demon(engine, "mmdemonhit");
        let hardest = spell_hits(&hits).into_iter().max().unwrap_or(0);
        assert!(
            (32..=310).contains(&hardest),
            "a spell takes up to 310 life points: {hits:?}"
        );
    });
}

/// `^mm_reported_narnode`: the king has heard the report and owes the reward.
const MM_REPORTED_NARNODE: i32 = 7;
/// `^mm_complete`: rewarded, Daero's training still to do.
const MM_COMPLETE: i32 = 9;
/// Cache 786's quest complete scroll.
const QUEST_SCROLL: u16 = 277;

/// King Narnode hands over the reward, the scroll opens, and once it is closed he sends the
/// player to Daero's training (289's mm_complete_dialogue, queued behind the scroll).
#[test]
fn king_narnode_speaks_of_the_training_after_the_reward() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mmreward");
        set_var(engine, &client, "mm_main", MM_REPORTED_NARNODE);
        let (nid, nx, nz, _) = spawned(engine, "grandtree_narnode", Some(KING)).expect("the king");
        tele(engine, &mut client, (nx, nz), true);
        talk_to(&mut client, nid);
        let t = converse(engine, &mut client, &[], 400);
        says(
            &t,
            &["King Narnode hands you a huge stack  of gold coins and several diamonds! "],
        );
        assert_eq!(var(engine, &client, "mm_main"), MM_COMPLETE);
        assert_eq!(held(engine, &client, "diamond"), 3);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(QUEST_SCROLL),
            "the reward scroll is open"
        );
        assert!(
            !t.said("So you're officially a member of the 10th squad then?"),
            "nothing is said under the scroll"
        );

        // The scroll's X.
        client.send(ClientProt::CloseModal, &[]);
        let t = converse(engine, &mut client, &[], 60);
        says(
            &t,
            &[
                "So you're officially a member of the 10th squad then?",
                "The High Tree Guardian Daero is in charge of the training programme. You should know where to find him by now.",
            ],
        );
        no_errors(engine);
    });
}

/// Daero's spawn up the Grand Tree (1_38_54_52_30), and `^narnode_received_orders` in bits 0-3 of
/// `%mm_gnomes` (289's `mm_narnode` varbit): before the orders Daero has no time to talk.
const DAERO: (u16, u16) = (2484, 3486);
const NARNODE_RECEIVED_ORDERS: i32 = 7;

/// Daero's training names 2011's skill: Constitution (cache 786's stat names, enum 680), where
/// 289 has Hitpoints.
#[test]
fn daero_s_training_gives_constitution_xp() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        for (choice, lines) in [
            (
                "Focus on increasing strength and stamina...",
                [
                    "You gain 35,000 Strength XP.",
                    "You gain 35,000 Constitution XP.",
                    "You gain 20,000 Attack XP.",
                    "You gain 20,000 Defence XP.",
                ],
            ),
            (
                "Focus on improving attack and defence techniques...",
                [
                    "You gain 20,000 Strength XP.",
                    "You gain 20,000 Constitution XP.",
                    "You gain 35,000 Attack XP.",
                    "You gain 35,000 Defence XP.",
                ],
            ),
        ] {
            let mut client = login(engine, "mmtraining");
            set_var(engine, &client, "mm_main", MM_COMPLETE);
            set_var(engine, &client, "mm_gnomes", NARNODE_RECEIVED_ORDERS);
            let nid = stand_beside(engine, &mut client, "daero", DAERO);
            talk_to(&mut client, nid);
            let mut t = converse(engine, &mut client, &[choice], 400);
            t.messages.extend(watch(engine, &mut client, 40).messages);
            let gains: Vec<&String> = t
                .messages
                .iter()
                .filter(|m| m.starts_with("You gain"))
                .collect();
            assert_eq!(
                gains,
                lines.iter().collect::<Vec<_>>(),
                "after {choice:?}: {:?}",
                t.messages
            );
            no_errors(engine);
        }
    });
}

/// Cache 786's quest journal, interface 275.
const JOURNAL: u32 = 275;

/// A finished journal line is struck from its first word to its last: every line of the journal
/// opens with its own tag (a colour, or the strike), and no strike starts part way along a line.
/// 289 marks each line of a struck paragraph where its own journal wrapped (q8_full, 400 wide);
/// 2011's wraps elsewhere (p12_full, 404 wide), which left the wrapped ends of lines unstruck.
#[test]
fn the_journal_strikes_whole_lines() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mmjournal");
        // The seal's stage (`^narnode_given_seal`), as the client showed it; then each stage of
        // the quest with every gnome's and monkey's own stage (289's varbits, by their bits on
        // `%mm_gnomes`, `%mm_monkeys` and `%mm_misc`) at each value from its start to its end.
        let each = |fields: &[(u32, u32)], stage: i32| {
            fields.iter().fold(0, |var, &(low, high)| {
                var | stage.min((1 << (high - low + 1)) - 1) << low
            })
        };
        let gnomes = [
            (0, 3),
            (4, 6),
            (7, 10),
            (11, 12),
            (13, 15),
            (16, 19),
            (20, 23),
            (24, 25),
            (26, 27),
        ];
        let monkeys = [(0, 1), (2, 4), (5, 8), (9, 10)];
        let misc = [(0, 1), (2, 9), (12, 16)];
        let mut stages: Vec<(i32, i32, i32, i32, i32)> = vec![(MM_STARTED, 3, 0, 0, 0)];
        for main in MM_STARTED..=10 {
            // 289's stages run to 10 (quest_mm.constant); 15 is every field at its last.
            for stage in (0..=10).chain([15]) {
                stages.push((
                    main,
                    each(&gnomes, stage),
                    each(&monkeys, stage),
                    if stage == 0 { 0 } else { 0x3FFF },
                    each(&misc, stage),
                ));
            }
        }
        for (main, gnomes, monkeys, flags, misc) in stages {
            set_var(engine, &client, "mm_main", main);
            set_var(engine, &client, "mm_gnomes", gnomes);
            set_var(engine, &client, "mm_monkeys", monkeys);
            set_var(engine, &client, "mm_flags", flags);
            set_var(engine, &client, "mm_misc", misc);
            let mut p = ClientPacket::new();
            p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(ROW);
            client.send(ClientProt::IfButton1, &p.finish());
            let mut lines = Vec::new();
            for _ in 0..3 {
                engine.cycle();
                lines.extend(
                    settexts(&client.packets())
                        .into_iter()
                        .filter(|(com, text)| com >> 16 == JOURNAL && !text.is_empty())
                        .map(|(_, text)| text),
                );
            }
            assert!(lines.len() >= 2, "the journal at stage {main}: {lines:?}");
            for line in &lines {
                assert!(
                    line.starts_with('<') && !line[1..].contains("<str>"),
                    "stage {main} ({gnomes:#x}): {line:?} is struck in part"
                );
            }
            client.send(ClientProt::CloseModal, &[]);
            settle(engine, &mut client, 2);
        }
        no_errors(engine);
    });
}
