//! Fletching from 289, through 2011's make-X dialogue.
//!
//! 289 asks what to cut from a log with `skill_multi3` (Make 1/5/10/X under
//! each item). 2011 asks in skillmulti2010 (905) with its number panel
//! skillmulti2010_numbers (916): the amount first, from 1/5/10/All/+1/-1
//! (IF_BUTTON1: `p4(com) ip2_alt1(obj) ip2(slot)`), then the item, a pause
//! button (RESUME_PAUSEBUTTON: `p2(slot) p4_alt2(com)`). The case uses a
//! knife on logs (IF_BUTTONT), picks 1 and then arrow shafts, and cuts one
//! log of three; then picks 5, which the two left cannot fill, and cuts both.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const FLETCHING: usize = 9;
/// 2011's xp counter, which counts ten times what `~givexp` pays.
const XP_COUNTER: u16 = 1801;

// The cases share one loaded engine and each starts with a reset, which would
// log the others' fletchers out: one at a time.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// `inventory:inv`, the backpack component `[login]` binds `inv` to.
const BACKPACK: i32 = 149 << 16;
const MAKE: i32 = 905 << 16;
const NUMBERS: i32 = 916 << 16;
/// skillmulti2010_numbers' 1 and 5, and skillmulti2010's first item.
const ONE: i32 = NUMBERS | 5;
const FIVE: i32 = NUMBERS | 6;
const FIRST_ITEM: i32 = MAKE | 14;
const NONE: i32 = 0xFFFF;

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

fn knife_on_logs(client: &mut HarnessClient, knife: u16, logs: u16, logs_slot: u16) {
    let mut p = ClientPacket::new();
    p.p4(BACKPACK)
        .ip2(logs_slot as i32)
        .ip2(0)
        .p2_alt1(knife as i32)
        .ip4(BACKPACK)
        .p2(logs as i32);
    client.send(ClientProt::IfButtonT, &p.finish());
}

fn button(client: &mut HarnessClient, com: i32) {
    let mut p = ClientPacket::new();
    p.p4(com).ip2_alt1(NONE).ip2(NONE);
    client.send(ClientProt::IfButton1, &p.finish());
}

fn resume(client: &mut HarnessClient, com: i32) {
    let mut p = ClientPacket::new();
    p.p2(NONE).p4_alt2(com);
    client.send(ClientProt::ResumePauseButton, &p.finish());
}

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

#[test]
fn cutting_logs_through_2011s_make_x() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let knife = obj(engine, "knife");
        let logs = obj(engine, "logs");
        let shafts = obj(engine, "arrow_shaft");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();

        engine.harness_reset();
        let mut client = engine.harness_login_returning("fletcher").expect("login");
        engine.cycle();
        let player = engine.harness_player_mut(client.pid).unwrap();
        let backpack = player.invs.get_mut(&inv).unwrap();
        backpack.set(0, knife, 1);
        backpack.set(1, logs, 1);
        backpack.set(2, logs, 1);
        backpack.set(3, logs, 1);
        let xp = player.stats.xp[FLETCHING];
        let counted = player.vars.get(XP_COUNTER).as_int();
        engine.cycle();
        client.packets();

        // The knife on a log asks in 2011's dialogue.
        knife_on_logs(&mut client, knife, logs, 1);
        ticks(engine, &mut client, 2);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(player.is_interface_open(905), "skillmulti2010 is open");
        assert!(player.is_interface_open(916), "its number panel is open");

        // 1, then arrow shafts: one log becomes 15 shafts.
        button(&mut client, ONE);
        let said = ticks(engine, &mut client, 1);
        assert!(said.is_empty(), "the one button said {said:?}");
        resume(&mut client, FIRST_ITEM);
        let said = ticks(engine, &mut client, 8);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(!player.is_interface_open(905), "the dialogue closed");
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(shafts), 15, "the game said {said:?}");
        assert_eq!(backpack.total(logs), 2);
        assert_eq!(said, ["You carefully cut the wood into 15 arrow shafts."]);
        assert_eq!(player.stats.xp[FLETCHING] - xp, 75);
        assert_eq!(
            player.vars.get(XP_COUNTER).as_int() - counted,
            750,
            "the xp counter counts the cut"
        );

        // 5, with two logs left: both are cut, and then the logs run out.
        knife_on_logs(&mut client, knife, logs, 2);
        ticks(engine, &mut client, 2);
        button(&mut client, FIVE);
        let said = ticks(engine, &mut client, 1);
        assert!(said.is_empty(), "the five button said {said:?}");
        resume(&mut client, FIRST_ITEM);
        let said = ticks(engine, &mut client, 16);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(shafts), 45, "the game said {said:?}");
        assert_eq!(backpack.total(logs), 0);
        assert_eq!(
            said,
            [
                "You carefully cut the wood into 15 arrow shafts.",
                "You carefully cut the wood into 15 arrow shafts.",
                "You have run out of logs.",
            ]
        );
    });
}

/// 289's ogre arrow shafts from achey logs (a stub of the same label in
/// `_migration_closure` duplicated it, which the compiler warned of): at
/// Fletching 5, Big Chompy Bird Hunting begun, a knife on the logs cuts 2 to
/// 6 shafts at 1.8 xp each, and 2011's xp counter counts them.
#[test]
fn a_knife_cuts_achey_logs_into_ogre_arrow_shafts() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let knife = obj(engine, "knife");
        let logs = obj(engine, "achey_tree_logs");
        let shafts = obj(engine, "ogre_arrow_shaft");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let chompybird = *engine.cache.varps.debugnames.get("chompybird").unwrap();

        engine.harness_reset();
        let mut client = engine
            .harness_login_returning("ogrefletcher")
            .expect("login");
        engine.cycle();
        let player = engine.harness_player_mut(client.pid).unwrap();
        let backpack = player.invs.get_mut(&inv).unwrap();
        backpack.set(0, knife, 1);
        backpack.set(1, logs, 1);
        player.stats.levels[FLETCHING] = 5;
        player.stats.base_levels[FLETCHING] = 5;
        player
            .vars
            .set(chompybird, rs_pack::cache::VarValue::Int(1));
        let xp = player.stats.xp[FLETCHING];
        let counted = player.vars.get(XP_COUNTER).as_int();
        engine.cycle();
        client.packets();

        knife_on_logs(&mut client, knife, logs, 1);
        let said = ticks(engine, &mut client, 3);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        let cut = backpack.total(shafts);
        assert!(
            (2..=6).contains(&cut),
            "2 to 6 shafts; the game said {said:?}"
        );
        assert_eq!(backpack.total(logs), 0);
        assert_eq!(
            said,
            [format!(
                "You carefully cut the wood into {cut} arrow shafts."
            )]
        );
        let paid = player.stats.xp[FLETCHING] - xp;
        assert_eq!(paid, 18 * cut as i32);
        assert_eq!(player.vars.get(XP_COUNTER).as_int() - counted, paid * 10);
    });
}
