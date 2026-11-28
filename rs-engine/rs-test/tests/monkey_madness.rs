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
use rs_protocol::network::game::server_prot::ServerProt;
use rs_protocol::network::game::zone_prot::ZoneProt;
use rs_test::names::BACKPACK;
use rs_test::runner;
use rs_test::server;
use rs_test::talk::{Conversation, converse, settexts, talk_to};
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

/// The nearest walkable tile to `by` (the tile itself first), not `by` when `beside`.
fn walkable_near((x, z): (u16, u16), level: u8, beside: bool) -> (u16, u16) {
    for r in 0i16..=4 {
        for dx in -r..=r {
            for dz in -r..=r {
                if dx.abs().max(dz.abs()) != r || (beside && r == 0) {
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

/// The spotanims of the projectiles in `packets`. MAP_PROJANIM rides in the tick's
/// UPDATE_ZONE_PARTIAL_ENCLOSED: its spotanim is the u16 after the coord, dx, dz and target.
fn projectiles(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    packets
        .iter()
        .filter(|(op, _)| *op == ServerProt::UpdateZonePartialEnclosed as u16)
        .flat_map(|(_, body)| {
            (0..body.len())
                .filter(|&i| body[i] == ZoneProt::MapProjAnim as u8 && i + 8 <= body.len())
                .map(|i| u16::from_be_bytes([body[i + 6], body[i + 7]]))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// 289's wave spells' travel spotanims, which cache 786 keeps: fire 156, wind 159, water 162 and
/// earth 165.
const WAVES: [u16; 4] = [156, 159, 162, 165];
/// `^mm_completed_ch3`: the sigil is given, the demon not yet fought.
const MM_COMPLETED_CH3: i32 = 5;

/// What the jungle demon's fight did over its first ticks: the projectiles launched at the player,
/// the spotanims shown on the player (`spotanim_pl`'s id, height and delay) and each hit painted on
/// the player (`damage`'s hitmark and amount).
struct DemonFight {
    launched: Vec<u16>,
    shown: Vec<String>,
    hits: Vec<(i32, i32)>,
}

/// The 10th squad's sigil takes the player to the final battle, where the jungle demon spawns and
/// attacks: it casts from 3 tiles or more, and casts or swings from nearer. Defence and Magic are
/// 1, so its spells land, and life is put back each tick.
fn fight_the_jungle_demon(engine: &mut Engine, name: &str) -> DemonFight {
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

    let mut launched = projectiles(&t.packets);
    let mut messages = t.messages;
    for _ in 0..80 {
        set_life_points(engine, &client, 990);
        engine.cycle();
        let packets = client.packets();
        launched.extend(projectiles(&packets));
        messages.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, m)| m),
        );
    }
    let fight = DemonFight {
        launched,
        shown: calls_for("spotanim_pl"),
        hits: calls_for("damage")
            .iter()
            .map(|hit| {
                let mut n = hit.split(',').map(|n| n.parse::<i32>().unwrap());
                (n.next().unwrap(), n.next().unwrap())
            })
            .collect(),
    };
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
    fight
}

/// The jungle demon's wave spells fly: each cast launches 289's travel spotanim, which cache 786
/// keeps, at the player, and the spell that lands shows its impact (at the wave rows' height, 124)
/// and hits. 647's magic_spell_table rows for the wave spells name spotanims 2011 does not have,
/// so the cast read -1 and failed on its projectile ("projanim_pl error: Spotanim not found: -1",
/// every cast of a fight in the client).
#[test]
fn the_jungle_demon_s_spells_fly_and_hit() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let fight = fight_the_jungle_demon(engine, "mmdemon");
        assert!(
            fight.launched.iter().any(|s| WAVES.contains(s)),
            "a wave spell's projectile: {:?}",
            fight.launched
        );
        assert!(
            fight.shown.iter().any(|s| {
                WAVES
                    .iter()
                    .any(|travel| s.starts_with(&format!("{},124,", travel + 1)))
            }),
            "a wave spell's impact: {:?}",
            fight.shown
        );
        assert!(!fight.hits.is_empty(), "a spell lands");
    });
}
