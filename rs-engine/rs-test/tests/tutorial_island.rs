//! 289's Tutorial Island on 2011's ids, kept as preserved content.
//!
//! A 2011 player never enters it on their own: the `tutorial_island` cheat
//! does. Each case drives the island as the client does (OPNPC1, OPLOC1,
//! RESUME_PAUSEBUTTON, IF_BUTTON1 on a tab stone) and reads the island's
//! progress varp, `tutorial` (a server-only varp: 2011's 281 is Unstable
//! Foundations').
//!
//! - A returning 2011 player standing on the island (the engine's new-player
//!   spawn) logs in without the island starting.
//! - The cheat puts the player in the RuneScape Guide's house at step 1, with
//!   the instructions (cache 786 interface 372) on the chatbox's foreground
//!   slot (752:7).
//! - The guide's welcome takes step 1 to 4, and the house door 4 to 10.
//! - 2011's tab stones stand in for 289's TUTORIAL_CLICKSIDE: a click on the
//!   step's tab (Inventory, toplevel 548:132) hands out the axe and
//!   tinderbox; a click on another (Stats, 548:130) does not.
//! - A tree on the island gives logs and moves the step on.
//! - The rat pen's gate takes the player through both ways: 2011's leaves
//!   stand on the other side of their wall from 289's.
//! - The run orb is 289's run button for the island's run step.
//! - Off the island, a 2011 player (island varp 0) wields a bronze dagger as
//!   289's players past the island do.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::player::StaffModLevel;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_vm::state::ExecutionState;

/// The chatbox's foreground slot, `chatbox:com7`.
const CHAT_FOREGROUND: i32 = 752 << 16 | 7;
const TUTORIAL_TEXT: u16 = 372;
/// toplevel (548) tab stones: Stats and Inventory.
const STATS_STONE: i32 = 548 << 16 | 130;
const INVENTORY_STONE: i32 = 548 << 16 | 132;
/// The run orb's button, `topstat_run:com1`.
const RUN_ORB: i32 = 750 << 16 | 1;
/// multi2's second option.
const MULTI2_SECOND: i32 = 228 << 16 | 3;
const BACKPACK: i32 = 149 << 16;
/// The island's start, 0_48_48_22_34.
const START: (u16, u16) = (3094, 3106);

// The island's steps (tutorial/configs/tutorial.constant).
const DESIGNED_CHARACTER: i32 = 1;
const INTERACT_WITH_SCENERY: i32 = 4;
const INTERACTED_WITH_DOOR: i32 = 10;
const OPEN_INVENTORY: i32 = 20;
const CUT_TREE: i32 = 30;
const BUILD_FIRE: i32 = 40;
const OPEN_PLAYER_CONTROLS: i32 = 195;
const TOGGLED_ON_RUN: i32 = 200;
const BEFORE_ATTACKING_MELEE: i32 = 430;

static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn varp_id(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .varps
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no varp {name}"))
}

fn tutorial(engine: &Engine, pid: u16) -> i32 {
    let id = varp_id(engine, "tutorial");
    engine.harness_player(pid).unwrap().vars.get(id).as_int()
}

fn set_tutorial(engine: &mut Engine, pid: u16, value: i32) {
    let id = varp_id(engine, "tutorial");
    engine
        .harness_player_mut(pid)
        .unwrap()
        .vars
        .set(id, VarValue::Int(value));
}

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

fn backpack_total(engine: &Engine, pid: u16, name: &str) -> i32 {
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let obj = obj(engine, name);
    engine
        .harness_player(pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap()
        .total(obj) as i32
}

fn coord(engine: &Engine, pid: u16) -> (u16, u16) {
    let c = engine.harness_player(pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

/// A loc's 647 id by its name in `content/647/pack/loc.pack`.
fn loc_id(name: &str) -> u16 {
    let pack = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/647/pack/loc.pack"
    );
    std::fs::read_to_string(pack)
        .expect("loc.pack")
        .lines()
        .find_map(|l| l.split_once('=').filter(|(_, n)| *n == name))
        .and_then(|(id, _)| id.parse().ok())
        .unwrap_or_else(|| panic!("647 has no loc {name}"))
}

fn npc_nid(engine: &Engine, name: &str) -> u16 {
    let id = *engine
        .cache
        .npcs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no npc {name}"));
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == id)
        .map(|n| n.npc.uid.nid())
        .unwrap_or_else(|| panic!("{name} is not spawned"))
}

/// A developer, logged in as a returning 2011 player (character creation
/// done), at `at`, past the clock's first ticks.
fn developer(engine: &mut Engine, name: &str, at: (u16, u16)) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .staff_mod_level = StaffModLevel::Maintainer;
    engine.cycle();
    engine.harness_telejump(client.pid, at.0, 0, at.1);
    for _ in 0..10 {
        engine.cycle();
    }
    client.packets();
    client
}

fn cheat(client: &mut HarnessClient, input: &str) {
    let mut p = ClientPacket::new();
    p.p1(0).p1(0).pjstr(input);
    client.send(ClientProt::ClientCheat, &p.finish());
}

fn button1(client: &mut HarnessClient, com: i32) {
    let mut p = ClientPacket::new();
    p.p4(com).p2_alt3(0xFFFF).p2_alt1(0xFFFF);
    client.send(ClientProt::IfButton1, &p.finish());
}

fn op_npc1(client: &mut HarnessClient, nid: u16) {
    let mut p = ClientPacket::new();
    p.p1_alt3(0).p2(nid as i32);
    client.send(ClientProt::OpNpc1, &p.finish());
}

fn op_loc1(client: &mut HarnessClient, loc: u16, x: u16, z: u16) {
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

fn resume(client: &mut HarnessClient, com: i32) {
    let mut p = ClientPacket::new();
    p.p2(0xFFFF).p4_alt2(com);
    client.send(ClientProt::ResumePauseButton, &p.finish());
}

/// A component of the dialogue open in the chatbox: 2011's client names the
/// button it continues from, and the server checks its interface is open.
fn open_chat(engine: &Engine, pid: u16) -> i32 {
    let chat = engine
        .harness_player(pid)
        .unwrap()
        .modal_chat
        .expect("a dialogue is open");
    (chat as i32) << 16
}

fn paused(engine: &Engine, pid: u16) -> bool {
    engine
        .harness_player(pid)
        .unwrap()
        .state
        .active_script
        .as_ref()
        .is_some_and(|s| s.execution == ExecutionState::PauseButton)
}

/// Cycles until the player's script waits on a dialogue, at most `n` ticks.
fn until_paused(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> bool {
    for _ in 0..n {
        engine.cycle();
        client.packets();
        if paused(engine, client.pid) {
            return true;
        }
    }
    false
}

/// Cycles `n` ticks; the game messages said meanwhile.
fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..n {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    said
}

fn instructions_open(engine: &Engine, pid: u16) -> bool {
    engine
        .harness_player(pid)
        .unwrap()
        .open_subs
        .get(&CHAT_FOREGROUND)
        .is_some_and(|&(sub, _)| sub == TUTORIAL_TEXT)
}

#[test]
fn a_2011_player_on_the_island_logs_in_without_it_starting() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        // The returning player logs in where the engine puts new players,
        // the island's start.
        let client = engine
            .harness_login_returning("islandvisitor")
            .expect("login");
        for _ in 0..5 {
            engine.cycle();
        }
        assert_eq!(coord(engine, client.pid), START, "on the island");
        assert_eq!(tutorial(engine, client.pid), 0, "the island not entered");
        assert!(
            !instructions_open(engine, client.pid),
            "no island instructions"
        );
    });
}

#[test]
fn the_cheat_enters_the_island_and_the_guide_and_door_move_it_on() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "islander", (3222, 3218));
        cheat(&mut client, "~tutorial_island");
        ticks(engine, &mut client, 3);
        assert_eq!(coord(engine, client.pid), START, "at the island's start");
        assert_eq!(tutorial(engine, client.pid), DESIGNED_CHARACTER);
        assert!(
            instructions_open(engine, client.pid),
            "372 on chatbox:com7; subs {:?}",
            engine.harness_player(client.pid).unwrap().open_subs
        );

        // The guide: a dev world first asks to skip (289's map_live check);
        // no, then his five lines.
        let guide = npc_nid(engine, "runescape_guide");
        op_npc1(&mut client, guide);
        assert!(until_paused(engine, &mut client, 10), "the guide speaks");
        let chat = open_chat(engine, client.pid);
        resume(&mut client, chat);
        assert!(until_paused(engine, &mut client, 3), "skip the tutorial?");
        resume(&mut client, MULTI2_SECOND);
        let mut lines = 0;
        while until_paused(engine, &mut client, 3) {
            lines += 1;
            assert!(lines <= 10, "the guide talks on");
            let chat = open_chat(engine, client.pid);
            resume(&mut client, chat);
        }
        assert!(lines >= 5, "the guide's five lines; {lines} pages");
        assert_eq!(tutorial(engine, client.pid), INTERACT_WITH_SCENERY);

        // The house's door, from inside (0_48_48_26_35, its west edge).
        let door = loc_id("newbie_door1");
        engine.harness_telejump(client.pid, 3097, 0, 3107);
        ticks(engine, &mut client, 1);
        op_loc1(&mut client, door, 3098, 3107);
        ticks(engine, &mut client, 6);
        assert_eq!(tutorial(engine, client.pid), INTERACTED_WITH_DOOR);
    });
}

#[test]
fn the_step_s_tab_stone_hands_out_the_axe_and_tinderbox() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // By the survival expert, at the step that flashes the backpack.
        let mut client = developer(engine, "tabclicker", (3102, 3094));
        set_tutorial(engine, client.pid, OPEN_INVENTORY);

        button1(&mut client, STATS_STONE);
        ticks(engine, &mut client, 2);
        assert_eq!(
            tutorial(engine, client.pid),
            OPEN_INVENTORY,
            "another tab leaves the step"
        );
        assert_eq!(backpack_total(engine, client.pid, "tinderbox"), 0);

        button1(&mut client, INVENTORY_STONE);
        ticks(engine, &mut client, 2);
        assert_eq!(tutorial(engine, client.pid), CUT_TREE);
        assert_eq!(backpack_total(engine, client.pid, "bronze_hatchet"), 1);
        assert_eq!(backpack_total(engine, client.pid, "tinderbox"), 1);
    });
}

#[test]
fn an_island_tree_gives_logs_and_the_fire_step() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // newbietree at 0_48_48_27_23 (2x2), from its south side.
        let (x, z) = (3099, 3095);
        let tree = loc_id("newbietree");
        let stand = (3099, 3094);
        assert!(
            !rsmod::is_flagged(stand.0, stand.1, 0, 0x100 | 0x200000),
            "a walkable tile by the tree"
        );
        let mut client = developer(engine, "islandchopper", stand);
        set_tutorial(engine, client.pid, CUT_TREE);
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let hatchet = obj(engine, "bronze_hatchet");
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .unwrap()
            .set(0, hatchet, 1);

        op_loc1(&mut client, tree, x, z);
        for _ in 0..40 {
            ticks(engine, &mut client, 1);
            if backpack_total(engine, client.pid, "newbielogs") > 0 {
                break;
            }
        }
        assert_eq!(backpack_total(engine, client.pid, "newbielogs"), 1);
        assert_eq!(tutorial(engine, client.pid), BUILD_FIRE);
    });
}

#[test]
fn the_rat_pen_gate_takes_the_player_through_both_ways() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // 2011's gate: newbiedoor5_r at 0_48_148_38_46, angle 2, the wall
        // between x 3110 (the pen) and 3111.
        let leaf = loc_id("newbiedoor5_r");
        let (gx, gz) = (3110, 9518);
        let mut client = developer(engine, "ratpen", (3111, gz));
        set_tutorial(engine, client.pid, BEFORE_ATTACKING_MELEE);

        op_loc1(&mut client, leaf, gx, gz);
        ticks(engine, &mut client, 4);
        assert_eq!(coord(engine, client.pid), (3110, gz), "into the pen");

        // The gate is open three ticks; it shuts before the way back.
        ticks(engine, &mut client, 4);
        op_loc1(&mut client, leaf, gx, gz);
        ticks(engine, &mut client, 4);
        assert_eq!(coord(engine, client.pid), (3111, gz), "out of the pen");
    });
}

#[test]
fn the_run_orb_is_the_island_s_run_button() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "islandrunner", START);
        set_tutorial(engine, client.pid, OPEN_PLAYER_CONTROLS);
        button1(&mut client, RUN_ORB);
        ticks(engine, &mut client, 2);
        assert_eq!(tutorial(engine, client.pid), TOGGLED_ON_RUN);
    });
}

#[test]
fn a_2011_player_wields_a_bronze_dagger_off_the_island() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "daggerwielder", (3222, 3218));
        assert_eq!(tutorial(engine, client.pid), 0);
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let dagger = obj(engine, "bronze_dagger");
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .unwrap()
            .set(0, dagger, 1);

        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(dagger as i32).p2_alt1(0);
        client.send(ClientProt::IfButton2, &p.finish());
        ticks(engine, &mut client, 2);
        assert_eq!(
            backpack_total(engine, client.pid, "bronze_dagger"),
            0,
            "wielded"
        );
    });
}
