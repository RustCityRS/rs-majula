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

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::player::StaffModLevel;
use rs_pack::cache::VarValue;
use rs_pack::types::NpcStat;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::life;
use rs_test::names::BACKPACK;
use rs_test::names::com;
use rs_test::runner;
use rs_ver::TypeId;
use rs_vm::ops::test::{calls_for, forget_calls, set_recording};
use rs_vm::state::ExecutionState;
use std::fs;
use std::sync::Mutex;

/// The chatbox's foreground slot, `chatbox:com7`.
fn chat_foreground() -> i32 {
    com("chattop:mes_layer_large")
}

const TUTORIAL_TEXT: u16 = 372;
/// toplevel (548) tab stones: Stats and Inventory.
/// 530's stats and backpack stones are 1 and 3 (its tabs.constant).
fn stats_stone() -> i32 {
    com(if cfg!(before_647) {
        "toplevel:stone1"
    } else {
        "toplevel:stone2"
    })
}

fn inventory_stone() -> i32 {
    com(if cfg!(before_647) {
        "toplevel:stone3"
    } else {
        "toplevel:stone4"
    })
}

/// The run orb's button, `topstat_run:com1`.
#[cfg(since_647)]
fn run_orb() -> i32 {
    com("topstat_run:run_background")
}

/// multi2's second option.
fn multi2_second() -> i32 {
    com("multi2:multi2b")
}

/// The island's start, 0_48_48_22_34.
const START: (u16, u16) = (3094, 3106);

// The island's steps (tutorial/configs/tutorial.constant).
const DESIGNED_CHARACTER: i32 = 1;
const INTERACT_WITH_SCENERY: i32 = 4;
const INTERACTED_WITH_DOOR: i32 = 10;
const OPEN_INVENTORY: i32 = 20;
const CUT_TREE: i32 = 30;
const BUILD_FIRE: i32 = 40;
#[cfg(since_647)]
const OPEN_PLAYER_CONTROLS: i32 = 195;
#[cfg(since_647)]
const TOGGLED_ON_RUN: i32 = 200;
const BEFORE_ATTACKING_MELEE: i32 = 430;

static TURN: Mutex<()> = Mutex::new(());

fn varp_id(engine: &Engine, name: &str) -> u16 {
    engine
        .cache
        .varps
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no varp {name}"))
        .id
        .g2()
}

fn tutorial(engine: &Engine, pid: u16) -> i32 {
    let id = varp_id(engine, "varplayer_419");
    engine.harness_player(pid).unwrap().vars.get(id).as_int()
}

fn set_tutorial(engine: &mut Engine, pid: u16, value: i32) {
    let id = varp_id(engine, "varplayer_419");
    engine
        .harness_player_mut(pid)
        .unwrap()
        .vars
        .set(id, VarValue::Int(value));
}

fn obj(engine: &Engine, name: &str) -> TypeId {
    engine
        .cache
        .objs
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
        .id
}

fn backpack_total(engine: &Engine, pid: u16, name: &str) -> i32 {
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
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

/// A loc's id by its name in the rev's `pack/loc.pack`.
fn loc_id(name: &str) -> u16 {
    let pack = &format!(
        "{}/../../{}/pack/loc.pack",
        env!("CARGO_MANIFEST_DIR"),
        rs_pack::CONTENT_DIR
    );
    fs::read_to_string(pack)
        .expect("loc.pack")
        .lines()
        .find_map(|l| l.split_once('=').filter(|(_, n)| *n == name))
        .and_then(|(id, _)| id.parse().ok())
        .unwrap_or_else(|| panic!("647 has no loc {name}"))
}

fn npc_nid(engine: &Engine, name: &str) -> u16 {
    let id = engine
        .cache
        .npcs
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no npc {name}"))
        .id;
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
    client.client_cheat(input);
}

fn button1(client: &mut HarnessClient, com: i32) {
    client.if_button(1, com, 0xFFFF, TypeId::NULL);
}

fn op_npc1(client: &mut HarnessClient, nid: u16) {
    client.op_npc(1, nid);
}

fn op_loc1(client: &mut HarnessClient, loc: u16, x: u16, z: u16) {
    client.op_loc(1, x, z, TypeId::from_int(loc as i32));
}

fn resume(client: &mut HarnessClient, com: i32) {
    client.resume_pausebutton(com, 0xFFFF);
}

/// The "Click here to continue" of the dialogue open in the chatbox: 2011's client names the
/// button it continues from, and the server checks it is that interface's pause button.
fn open_chat(engine: &Engine, pid: u16) -> i32 {
    let chat = engine
        .harness_player(pid)
        .unwrap()
        .modal_chat
        .expect("a dialogue is open") as i32;
    (0..64)
        .map(|com| chat << 16 | com)
        .find(|&com| {
            engine
                .cache
                .interfaces
                .get_by_id(com)
                .is_some_and(|c| c.active_properties & 1 != 0)
        })
        .unwrap_or_else(|| panic!("interface {chat} has no pause button"))
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
        .subs
        .open
        .get(&chat_foreground())
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
            engine.harness_player(client.pid).unwrap().subs.open
        );

        // The guide: a dev world first asks to skip (289's map_live check);
        // no, then his five lines.
        let guide = npc_nid(engine, "newbie_basics_instructor");
        op_npc1(&mut client, guide);
        assert!(until_paused(engine, &mut client, 10), "the guide speaks");
        let chat = open_chat(engine, client.pid);
        resume(&mut client, chat);
        assert!(until_paused(engine, &mut client, 3), "skip the tutorial?");
        resume(&mut client, multi2_second());
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

        button1(&mut client, stats_stone());
        ticks(engine, &mut client, 2);
        assert_eq!(
            tutorial(engine, client.pid),
            OPEN_INVENTORY,
            "another tab leaves the step"
        );
        assert_eq!(backpack_total(engine, client.pid, "tinderbox"), 0);

        button1(&mut client, inventory_stone());
        ticks(engine, &mut client, 2);
        assert_eq!(tutorial(engine, client.pid), CUT_TREE);
        assert_eq!(backpack_total(engine, client.pid, "bronze_axe"), 1);
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
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let hatchet = obj(engine, "bronze_axe");
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

#[cfg(since_647)]
#[test]
fn the_run_orb_is_the_island_s_run_button() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "islandrunner", START);
        set_tutorial(engine, client.pid, OPEN_PLAYER_CONTROLS);
        button1(&mut client, run_orb());
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
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let dagger = obj(engine, "bronze_dagger");
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .unwrap()
            .set(0, dagger, 1);

        client.if_button(2, BACKPACK, 0, dagger);
        ticks(engine, &mut client, 2);
        assert_eq!(
            backpack_total(engine, client.pid, "bronze_dagger"),
            0,
            "wielded"
        );
    });
}

/// 289's tutorial rat (tutorial/scripts/npcs/tut_giant_rat.rs2) bites with
/// 2011's melee mark, as every npc melee hit paints (hitmark.rs2), not the
/// plain mark a bare damage gets. Fought until a bite lands, every bite it
/// paints is a melee hit, its crit or a miss.
#[test]
fn the_tutorial_rat_bites_with_the_melee_mark() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "ratbitten", (3258, 3270));
        cheat(&mut client, "npcadd newbiegiantrat");
        engine.cycle();
        // Off the rat's 2x2 tiles, and a few ticks for the client to see it.
        engine.harness_telejump(client.pid, 3255, 0, 3270);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();
        let rat = rat_beside(engine);
        // Alive the whole fight: the bites are under test, not the kill.
        engine.get_npc_mut(rat).unwrap().npc.stats.levels[NpcStat::Hitpoints as usize] = 3000;
        set_recording(true);
        forget_calls();
        client.op_npc(2, rat);
        let mut bites: Vec<(i32, i32)> = Vec::new();
        let mut said = Vec::new();
        // 530's rat hits for 0 or 1 hitpoint, so a swing lands damage about 3 times in 10.
        let landed = |(mark, amount): &(i32, i32)| (*mark == 0 || *mark == 10) && *amount > 0;
        for tick in 0..300 {
            engine.cycle();
            for (_, m) in HarnessClient::game_messages(&client.packets()) {
                said.push(m);
            }
            if tick % 20 == 0 {
                client.send(ClientProt::NoTimeout, &[]);
            }
            for call in calls_for("damage") {
                let parts: Vec<i32> = call.split(',').map(|v| v.parse().unwrap()).collect();
                bites.push((parts[0], parts[1]));
            }
            forget_calls();
            if bites.iter().any(landed) {
                break;
            }
        }
        set_recording(false);
        assert!(
            bites.iter().all(|(mark, _)| [0, 8, 10].contains(mark)),
            "melee, its crit or a miss: {bites:?}"
        );
        assert!(
            bites.iter().any(landed),
            "a bite lands: {bites:?}; said {said:?}; rat hp {:?}",
            engine
                .get_npc(rat)
                .map(|n| n.npc.stats.levels[NpcStat::Hitpoints as usize])
        );
    });
}

/// 289's tutorial rat spares the player's last hitpoint (10 life points from
/// 647). There, a bite it paints is a miss; it still swings.
#[test]
fn the_tutorial_rat_spares_the_last_hitpoint() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "ratspared", (3258, 3270));
        cheat(&mut client, "npcadd newbiegiantrat");
        life::set(engine.harness_player_mut(client.pid).unwrap(), life::SCALE);
        engine.cycle();
        engine.harness_telejump(client.pid, 3255, 0, 3270);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();
        let pid = client.pid;
        assert_eq!(life::get(engine, pid), life::SCALE);
        let rat = rat_beside(engine);
        // Alive the whole fight: the bites are under test, not the kill.
        engine.get_npc_mut(rat).unwrap().npc.stats.levels[NpcStat::Hitpoints as usize] = 3000;
        set_recording(true);
        forget_calls();
        client.op_npc(2, rat);
        let mut bites = Vec::new();
        for tick in 0..40 {
            // Held on the last hitpoint: the 10-tick regen would lift the player off it, and a bite
            // could then kill.
            life::set(engine.harness_player_mut(pid).unwrap(), life::SCALE);
            let before = life::get(engine, pid);
            engine.cycle();
            client.packets();
            if tick % 20 == 0 {
                client.send(ClientProt::NoTimeout, &[]);
            }
            for call in calls_for("damage") {
                let parts: Vec<i32> = call.split(',').map(|v| v.parse().unwrap()).collect();
                bites.push((tick, before, parts[0], parts[1]));
            }
            forget_calls();
        }
        set_recording(false);
        assert!(
            bites
                .iter()
                .all(|(_, before, _, amount)| *before > life::SCALE || *amount == 0),
            "nothing bitten off the last hitpoint: {bites:?}"
        );
        assert!(
            bites.iter().any(|(_, _, mark, _)| *mark == 8),
            "the rat swings: {bites:?}"
        );
        assert!(life::get(engine, pid) > 0);
    });
}

/// The rat `npcadd` put down in the cow field (3258,3270), not one of the
/// island's own, which are far off.
fn rat_beside(engine: &Engine) -> u16 {
    let id = engine
        .cache
        .npcs
        .get_by_debugname("newbiegiantrat")
        .unwrap()
        .id;
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == id)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x().abs_diff(3258) + c.z().abs_diff(3270))
        })
        // It wanders from the tile it was put down on before it is looked for.
        .filter(|&(_, d)| d <= 10)
        .min_by_key(|&(_, d)| d)
        .map(|(nid, _)| nid)
        .expect("the rat npcadd spawned")
}
