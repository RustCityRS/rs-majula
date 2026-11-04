//! Smelting from 289, at 2011's furnaces.
//!
//! Lumbridge's furnace (loc 45310 at 3226,3256, from the 2009 rebuild) smelts
//! on op2, as 289's furnaces do (OPLOC2: `ip2(loc) p1_alt1(ctrl) p2_alt1(z)
//! ip2_alt1(x)`). 2011 asks which bar in its make-X dialogue (skillmulti2010,
//! 905, with its number panel 916), where 289 opened its own smelting
//! interface: the amount first (IF_BUTTON1 `p4(com) ip2_alt1(obj)
//! ip2(slot)`), then the bar, a pause button (RESUME_PAUSEBUTTON `p2(slot)
//! p4_alt2(com)`). A bronze bar takes a copper and a tin ore and pays 6.2
//! xp, which 2011's xp counter (varp 1801, in tenths) counts.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
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
        p.ip2(loc as i32)
            .p1_alt1(0)
            .p2_alt1(z as i32)
            .ip2_alt1(x as i32);
        client.send(ClientProt::OpLoc2, &p.finish());
        let said = ticks(engine, &mut client, 6);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(
            player.is_interface_open(905),
            "2011's make-X asks which bar; said {said:?}"
        );

        let mut p = ClientPacket::new();
        p.p4(ONE).ip2_alt1(NONE).ip2(NONE);
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
