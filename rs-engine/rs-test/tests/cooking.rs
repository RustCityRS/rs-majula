//! Cooking from 289, with 2011's amount.
//!
//! 289 cooks one raw item per use. 2011 asks how many first, in the make-X
//! dialogue with the verb Cook ("How many would you like to cook?"), when
//! more than one is held, and cooks that many, one every four ticks
//! (void-634's Cooking). The case uses three raw shrimps on Lumbridge
//! castle's cook-o-matic (loc 114) as the client does (OPLOCT), picks All
//! (skillmulti2010_numbers com8, IF_BUTTON1) and the shrimps (RESUME_PAUSEBUTTON
//! on skillmulti2010 com14), and sees all three cooked or burnt. The
//! player has done Cook's Assistant, which the castle's range asks for.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const COOKING: usize = 7;

/// `%cookquest` and 289's `^cook_complete`.
const COOKQUEST: u16 = 29;
const COOK_COMPLETE: i32 = 2;

/// `inventory:inv`, the backpack component `[login]` binds `inv` to.
const BACKPACK: i32 = 149 << 16;
const ALL: i32 = 916 << 16 | 8;
const FIRST_ITEM: i32 = 905 << 16 | 14;
const NONE: i32 = 0xFFFF;

/// Lumbridge castle's cook-o-matic (289's `cooking_range`), 1x2.
const RANGE: (u16, u16, u16) = (114, 3212, 3215);

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
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
fn three_shrimps_on_a_range_in_one_go() {
    runner::on_loaded_engine(|engine| {
        let raw = obj(engine, "raw_shrimps");
        let cooked = obj(engine, "shrimps");
        let burnt = obj(engine, "burnt_fish");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();

        engine.harness_reset();
        let mut client = engine.harness_login_returning("cook").expect("login");
        engine.cycle();
        engine.harness_telejump(client.pid, 3211, 0, 3214);
        let player = engine.harness_player_mut(client.pid).unwrap();
        // The castle's range is the Cook's: 289 lets a player use it only
        // after Cook's Assistant.
        player.vars.set(COOKQUEST, VarValue::Int(COOK_COMPLETE));
        let backpack = player.invs.get_mut(&inv).unwrap();
        for slot in 0..3 {
            backpack.set(slot, raw, 1);
        }
        let xp = player.stats.xp[COOKING];
        engine.cycle();
        client.packets();

        let (loc, x, z) = RANGE;
        let mut p = ClientPacket::new();
        p.p2_alt1(raw as i32)
            .p2_alt1(0)
            .p2(z as i32)
            .p4_alt2(BACKPACK)
            .p2_alt1(x as i32)
            .p1_alt3(0)
            .ip2(loc as i32);
        client.send(ClientProt::OpLocT, &p.finish());
        let said = ticks(engine, &mut client, 6);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(
            player.is_interface_open(905),
            "2011 asks how many; the game said {said:?}"
        );

        let mut p = ClientPacket::new();
        p.p4(ALL).ip2_alt1(NONE).ip2(NONE);
        client.send(ClientProt::IfButton1, &p.finish());
        let said = ticks(engine, &mut client, 1);
        assert!(said.is_empty(), "All said {said:?}");
        let mut p = ClientPacket::new();
        p.p2(NONE).p4_alt2(FIRST_ITEM);
        client.send(ClientProt::ResumePauseButton, &p.finish());

        let said = ticks(engine, &mut client, 20);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(
            backpack.total(raw),
            0,
            "all three cooked; the game said {said:?}"
        );
        let (done, lost) = (backpack.total(cooked), backpack.total(burnt));
        assert_eq!(done + lost, 3, "the game said {said:?}");
        assert_eq!(
            said.iter().filter(|m| m.contains("on the stove")).count(),
            3,
            "one cook message a shrimp; the game said {said:?}"
        );
        assert_eq!(
            said.iter()
                .filter(|m| *m == "The shrimps are now nicely cooked.")
                .count() as u32,
            done
        );
        assert_eq!(player.stats.xp[COOKING] - xp, 300 * done as i32);
    });
}

/// The Cooking Guild's door (`chefdoor`, 0_49_53_7_51). 2011 moved it a
/// square south of 289's, onto the street's side of its wall: the guild is
/// north (its Head chef, range and tables), so the door's own square is
/// outside. OPLOC1 is `ip2(z) ip2(loc) p1(ctrl) p2_alt1(x)`.
const CHEFDOOR: (u16, u16) = (3143, 3443);
/// npc_chat3, the Head chef's three lines, and its "Click here to continue".
const NPC_CHAT_3: u16 = 243;
const NPC_CHAT_3_CONTINUE: i32 = 243 << 16 | 7;

/// `chefdoor` in `loc.pack`.
const CHEFDOOR_LOC: i32 = 2712;

fn open_chefdoor(client: &mut HarnessClient) {
    let mut p = ClientPacket::new();
    p.ip2(CHEFDOOR.1 as i32)
        .ip2(CHEFDOOR_LOC)
        .p1(0)
        .p2_alt1(CHEFDOOR.0 as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

/// A cook below 32 at `(x, z)`, the welcome sent.
fn guild_guest(engine: &mut Engine, name: &str, x: u16, z: u16) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    engine.harness_telejump(client.pid, x, 0, z);
    engine.cycle();
    client.packets();
    rs_vm::ops::test::forget_calls();
    client
}

/// From the street below level 32, 289's refusal: the Head chef speaks
/// (289's `~chatnpc_specific`, an npc the player has not clicked) and his
/// line waits for "Click here to continue", then closes on it. The cook
/// stays outside.
/// The guild's door is one wall in rs-pathfinder's collision map, a process
/// static every engine in this test binary shares: one door test's open door
/// moved the other's walk to it, and its refusal never came. They take turns.
static CHEFDOOR_TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn the_guild_door_turns_away_a_cook_below_32() {
    let _turn = CHEFDOOR_TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = guild_guest(engine, "guildguest", 3143, 3441);
        open_chefdoor(&mut client);
        let mut open = false;
        for _ in 0..10 {
            engine.cycle();
            client.packets();
            if engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(NPC_CHAT_3)
            {
                open = true;
                break;
            }
        }
        assert!(open, "the Head chef's line");
        assert_eq!(
            rs_vm::ops::test::calls_for("mes"),
            ["You need a cooking level of 32 to enter the Chef's Guild."]
        );
        engine.cycle();
        let player = engine.harness_player(client.pid).unwrap();
        assert!(player.state.active_script.is_some(), "waiting for continue");
        assert!(player.pathing.coord.z() < 3444, "still outside");

        let mut p = ClientPacket::new();
        p.p2(0xFFFF).p4_alt2(NPC_CHAT_3_CONTINUE);
        client.send(ClientProt::ResumePauseButton, &p.finish());
        engine.cycle();
        engine.cycle();
        let player = engine.harness_player(client.pid).unwrap();
        assert!(!player.is_interface_open(NPC_CHAT_3), "closed on continue");
        assert!(player.state.active_script.is_none(), "and done");
    });
}

/// Leaving needs nothing: from inside, the door opens for anyone and they
/// step out onto the street.
#[test]
fn the_guild_door_lets_anyone_out() {
    let _turn = CHEFDOOR_TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = guild_guest(engine, "guildleaver", 3143, 3445);
        open_chefdoor(&mut client);
        for _ in 0..10 {
            engine.cycle();
            client.packets();
        }
        assert!(rs_vm::ops::test::calls_for("mes").is_empty(), "no refusal");
        let player = engine.harness_player(client.pid).unwrap();
        assert!(!player.is_interface_open(NPC_CHAT_3), "no Head chef");
        assert!(player.pathing.coord.z() <= CHEFDOOR.1, "out on the street");
    });
}
