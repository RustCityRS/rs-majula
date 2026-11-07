//! Smelting and anvil smithing from 289, at 2011's furnaces and anvils.
//!
//! Lumbridge's furnace (loc 45310 at 3226,3256; no source found for when it
//! came) smelts on op2, as 289's furnaces do (OPLOC2: `p2_alt1(loc) p1_alt2(ctrl)
//! p2_alt2(z) p2_alt3(x)`). The port asks which bar in the make-X dialogue
//! (skillmulti2010, 905, with its number panel 916), as void-634's Furnace.kt
//! does (no 2011 source found; cache 786 still has a 289-style smelting
//! interface, 941), where 289 opened its own smelting interface: the amount
//! first (IF_BUTTON1 `p4(com) p2_alt3(obj)
//! p2_alt1(slot)`), then the bar, a pause button (RESUME_PAUSEBUTTON `p2(slot)
//! p4_alt2(com)`). A bronze bar takes a copper and a tin ore and pays 6.2
//! xp, which 2011's xp counter (varp 1801, in tenths) counts.
//!
//! A bar used on Lumbridge's anvil (loc 12692 at 3229,3254, OPLOCT
//! `p2_alt2(obj) p2_alt2(slot) p2(z) p4_alt2(com) p2_alt2(x) p1_alt1(ctrl)
//! p2_alt1(loc)`) opens 2011's anvil interface (smithing_new, 300) with each of
//! 289's products in its slot (IF_SETOBJECT `p4_alt1(com) p4_alt3(count)
//! p2(obj)`); a slot's Make 1 or Make All (IF_BUTTON1) smiths from that bar
//! at 289's rates: a bronze dagger is a bar and 12.5 xp.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

const SMITHING: usize = 13;
const XP_COUNTER: u16 = 1801;
const FURNACE: (u16, u16, u16) = (45310, 3226, 3256);
const MAKE: i32 = 905 << 16;
const NUMBERS: i32 = 916 << 16;
const ONE: i32 = NUMBERS | 5;
/// skillmulti2010's first item: the bronze bar.
const BRONZE: i32 = MAKE | 14;
const NONE: i32 = 0xFFFF;

/// Lumbridge's anvil, 2011's (loc 12692).
const ANVIL: (u16, u16, u16) = (12692, 3229, 3254);
/// `inventory:inv`, the backpack component `[login]` binds `inv` to.
const BACKPACK: i32 = 149 << 16;
const ANVIL_IF: i32 = 300 << 16;
/// The dagger's slot (its item component), its Make All and its Make 1.
const DAGGER: i32 = ANVIL_IF | 18;
const DAGGER_ALL: i32 = ANVIL_IF | 21;
const DAGGER_ONE: i32 = ANVIL_IF | 24;
/// The bronze wire's and the claws' slots (their layers).
const WIRE: i32 = ANVIL_IF | 81;
const CLAWS: i32 = ANVIL_IF | 209;

/// Every engine in this binary shares rs-pathfinder's process-static
/// collision map: the cases take turns.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
fn lumbridge_s_furnace_smelts_a_bronze_bar() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let copper = obj(engine, "copper_ore");
        let tin = obj(engine, "tin_ore");
        let bronze = obj(engine, "bronze_bar");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();

        engine.harness_reset();
        let mut client = engine.harness_login_returning("smelter").expect("login");
        engine.cycle();
        let (sx, sz) = [(0, -1), (-1, 0), (0, 2), (2, 0), (-1, 1), (1, -1)]
            .into_iter()
            .map(|(dx, dz)| {
                (
                    FURNACE.1.wrapping_add_signed(dx),
                    FURNACE.2.wrapping_add_signed(dz),
                )
            })
            .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
            .expect("a walkable tile beside the furnace");
        engine.harness_telejump(client.pid, sx, 0, sz);
        let player = engine.harness_player_mut(client.pid).unwrap();
        let backpack = player.invs.get_mut(&inv).unwrap();
        backpack.set(0, copper, 1);
        backpack.set(1, tin, 1);
        let xp = player.stats.xp[SMITHING];
        let counted = player.vars.get(XP_COUNTER).as_int();
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();

        let (loc, x, z) = FURNACE;
        let mut p = ClientPacket::new();
        p.p2_alt1(loc as i32)
            .p1_alt2(0)
            .p2_alt2(z as i32)
            .p2_alt3(x as i32);
        client.send(ClientProt::OpLoc2, &p.finish());
        let said = ticks(engine, &mut client, 6);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(
            player.is_interface_open(905),
            "2011's make-X asks which bar; said {said:?}"
        );

        let mut p = ClientPacket::new();
        p.p4(ONE).p2_alt3(NONE).p2_alt1(NONE);
        client.send(ClientProt::IfButton1, &p.finish());
        ticks(engine, &mut client, 1);
        let mut p = ClientPacket::new();
        p.p2(NONE).p4_alt2(BRONZE);
        client.send(ClientProt::ResumePauseButton, &p.finish());
        let said = ticks(engine, &mut client, 10);

        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(bronze), 1, "the game said {said:?}");
        assert_eq!(backpack.total(copper) + backpack.total(tin), 0);
        assert_eq!(player.stats.xp[SMITHING] - xp, 62, "6.2 xp");
        assert_eq!(player.vars.get(XP_COUNTER).as_int() - counted, 620);
        assert_eq!(
            said,
            [
                "You smelt the copper and tin together in the furnace.",
                "You retrieve a bar of bronze."
            ]
        );
    });
}

/// What the server sent, packet by packet: `(opcode, payload)`.
type Packets = Vec<(u16, Vec<u8>)>;

/// Each IF_SETOBJECT as `(com, obj)`.
fn objects(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, u16)> {
    packets
        .iter()
        .filter(|(op, _)| *op == ServerProt::IfSetObject as u16)
        .map(|(_, p)| {
            (
                i32::from_le_bytes([p[0], p[1], p[2], p[3]]),
                u16::from_be_bytes([p[8], p[9]]),
            )
        })
        .collect()
}

/// Each IF_SETHIDE as `(com, hidden)`: `p4_alt3 com, p1_alt2 hidden`.
fn hides(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, bool)> {
    packets
        .iter()
        .filter(|(op, _)| *op == ServerProt::IfSetHide as u16)
        .map(|(_, p)| {
            let com = i32::from_be_bytes([p[1], p[0], p[3], p[2]]);
            (com, 0u8.wrapping_sub(p[4]) == 1)
        })
        .collect()
}

/// Uses the first bronze bar in the backpack on Lumbridge's anvil (OPLOCT).
fn bar_on_anvil(engine: &Engine, client: &mut HarnessClient) {
    let bronze = obj(engine, "bronze_bar");
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let player = engine.harness_player(client.pid).unwrap();
    let backpack = player.invs.get(&inv).unwrap();
    let slot = (0..28)
        .find(|&s| backpack.get(s).is_some_and(|o| o.obj == bronze))
        .expect("a bronze bar");
    let (loc, x, z) = ANVIL;
    let mut p = ClientPacket::new();
    p.p2_alt2(bronze as i32)
        .p2_alt2(slot as i32)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt2(x as i32)
        .p1_alt1(0)
        .p2_alt1(loc as i32);
    client.send(ClientProt::OpLocT, &p.finish());
}

/// Logs a smith in beside Lumbridge's anvil with a hammer and `bars` bronze
/// bars, uses a bar on the anvil and returns what the server sent.
fn at_the_anvil(
    engine: &mut Engine,
    name: &str,
    bars: u16,
) -> (HarnessClient, Packets, Vec<String>) {
    let hammer = obj(engine, "hammer");
    let bronze = obj(engine, "bronze_bar");
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();

    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    let (sx, sz) = [(0, -1), (-1, 0), (0, 1), (1, 0), (-1, -1), (1, 1)]
        .into_iter()
        .map(|(dx, dz)| {
            (
                ANVIL.1.wrapping_add_signed(dx),
                ANVIL.2.wrapping_add_signed(dz),
            )
        })
        .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
        .expect("a walkable tile beside the anvil");
    engine.harness_telejump(client.pid, sx, 0, sz);
    let player = engine.harness_player_mut(client.pid).unwrap();
    let backpack = player.invs.get_mut(&inv).unwrap();
    backpack.set(0, hammer, 1);
    for slot in 0..bars {
        backpack.set(slot + 1, bronze, 1);
    }
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();

    bar_on_anvil(engine, &mut client);
    let mut sent = Vec::new();
    let mut said = Vec::new();
    for _ in 0..6 {
        engine.cycle();
        let packets = client.packets();
        said.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, t)| t),
        );
        sent.extend(packets);
    }
    (client, sent, said)
}

#[test]
fn a_bronze_bar_on_lumbridge_s_anvil_opens_2011_s_slots() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let dagger = obj(engine, "bronze_dagger");
        let (client, sent, said) = at_the_anvil(engine, "anvilslots", 1);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(
            player.is_interface_open(300),
            "2011's anvil interface; the game said {said:?}"
        );
        assert!(
            objects(&sent).contains(&(DAGGER, dagger)),
            "the bronze dagger in its slot; sent {:?}",
            objects(&sent)
        );
        let hidden = hides(&sent);
        assert!(hidden.contains(&(WIRE, false)), "bronze shows its wire");
        assert!(
            hidden.contains(&(CLAWS, true)),
            "no claws before Death Plateau"
        );
    });
}

#[test]
fn make_1_smiths_a_bronze_dagger_and_make_all_the_rest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let bronze = obj(engine, "bronze_bar");
        let dagger = obj(engine, "bronze_dagger");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let (mut client, _, said) = at_the_anvil(engine, "anvilsmith", 3);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(player.is_interface_open(300), "the game said {said:?}");
        let xp = player.stats.xp[SMITHING];
        let counted = player.vars.get(XP_COUNTER).as_int();

        let mut p = ClientPacket::new();
        p.p4(DAGGER_ONE).p2_alt3(NONE).p2_alt1(NONE);
        client.send(ClientProt::IfButton1, &p.finish());
        let said = ticks(engine, &mut client, 6);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(dagger), 1, "the game said {said:?}");
        assert_eq!(backpack.total(bronze), 2);
        assert_eq!(player.stats.xp[SMITHING] - xp, 125, "12.5 xp");
        assert_eq!(player.vars.get(XP_COUNTER).as_int() - counted, 1250);
        assert_eq!(said, ["You hammer the bronze and make a dagger."]);
        assert!(!player.is_interface_open(300), "the interface closes");

        // Again, Make All: the two bars left make two, and that is all.
        bar_on_anvil(engine, &mut client);
        for _ in 0..10 {
            ticks(engine, &mut client, 1);
            if engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(300)
            {
                break;
            }
        }
        let mut p = ClientPacket::new();
        p.p4(DAGGER_ALL).p2_alt3(NONE).p2_alt1(NONE);
        client.send(ClientProt::IfButton1, &p.finish());
        let said = ticks(engine, &mut client, 12);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(dagger), 3, "the game said {said:?}");
        assert_eq!(backpack.total(bronze), 0);
        assert_eq!(player.stats.xp[SMITHING] - xp, 375);
        assert_eq!(
            said,
            [
                "You hammer the bronze and make a dagger.",
                "You hammer the bronze and make a dagger."
            ],
            "no word of running out"
        );
    });
}
