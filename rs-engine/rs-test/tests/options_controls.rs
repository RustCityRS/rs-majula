//! 289's options and controls tabs on 2011's tabs.
//!
//! - Options (261): Toggle Number of Mouse Buttons (com6) flips varp 170, and
//!   Toggle Chat Effects (com4) varp 171, as 289's pairs set them; Open Chat
//!   Setup (com5) puts Chat Setup (982) in the tab, whose No split (com41)
//!   and private colours (com49-66) set varp 287 to 0 and 1-18, and whose
//!   Close (com5) puts the options back.
//! - Emotes (464): the Mime's Glass Wall (com29) is locked until varp 313's
//!   bit 0 is set, as 289's controls tab locks it.
//!
//! A tab button is IF_BUTTON1, `p4(com) ip2_alt1(obj) ip2(slot)`.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

const OPTIONS: u16 = 261;
const PMSETUP: u16 = 982;
const EMOTES: u16 = 464;

fn com(iface: u16, index: u16) -> i32 {
    (iface as i32) << 16 | index as i32
}

fn player(engine: &mut Engine, name: &str) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    engine.harness_telejump(client.pid, 3222, 0, 3218);
    for _ in 0..12 {
        engine.cycle();
    }
    client.packets();
    client
}

fn click(engine: &mut Engine, client: &mut HarnessClient, com: i32) -> Vec<(u16, Vec<u8>)> {
    let mut p = ClientPacket::new();
    p.p4(com).ip2_alt1(0xFFFF).ip2(0xFFFF);
    client.send(ClientProt::IfButton1, &p.finish());
    let mut packets = Vec::new();
    for _ in 0..2 {
        engine.cycle();
        packets.extend(client.packets());
    }
    packets
}

fn varp(engine: &Engine, client: &HarnessClient, id: u16) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(id)
        .as_int()
}

fn is_open(engine: &Engine, client: &HarnessClient, iface: u16) -> bool {
    engine
        .harness_player(client.pid)
        .unwrap()
        .is_interface_open(iface)
}

fn settexts(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    let opcode = ServerProt::IfSetText as u16;
    packets
        .iter()
        .filter(|(op, _)| *op == opcode)
        .map(|(_, p)| {
            let end = p[4..]
                .iter()
                .position(|&b| b == 0)
                .map_or(p.len(), |i| 4 + i);
            String::from_utf8_lossy(&p[4..end]).into_owned()
        })
        .collect()
}

#[test]
fn the_options_tab_toggles_289_s_options() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "optionsfan");
        assert!(is_open(engine, &client, OPTIONS), "the options tab");

        let mouse = varp(engine, &client, 170);
        click(engine, &mut client, com(OPTIONS, 6));
        assert_eq!(varp(engine, &client, 170), 1 - mouse, "mouse buttons flip");
        click(engine, &mut client, com(OPTIONS, 6));
        assert_eq!(varp(engine, &client, 170), mouse, "and back");

        let effects = varp(engine, &client, 171);
        click(engine, &mut client, com(OPTIONS, 4));
        assert_eq!(varp(engine, &client, 171), 1 - effects, "chat effects flip");

        click(engine, &mut client, com(OPTIONS, 5));
        assert!(is_open(engine, &client, PMSETUP), "Chat Setup in the tab");
        click(engine, &mut client, com(PMSETUP, 52));
        assert_eq!(varp(engine, &client, 287), 4, "split, in the fourth colour");
        click(engine, &mut client, com(PMSETUP, 41));
        assert_eq!(varp(engine, &client, 287), 0, "no split");
        click(engine, &mut client, com(PMSETUP, 5));
        assert!(
            is_open(engine, &client, OPTIONS),
            "Close puts the options back"
        );
        assert!(!is_open(engine, &client, PMSETUP));
    });
}

#[test]
fn the_mimes_emotes_stay_locked_until_given() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "mimefan");
        assert!(is_open(engine, &client, EMOTES), "the emotes tab");
        let texts = settexts(&click(engine, &mut client, com(EMOTES, 29)));
        assert!(
            texts.iter().any(|t| t == "You can't use that emote yet."),
            "Glass Wall is locked; sent {texts:?}"
        );
        engine.harness_close_modal(client.pid);
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .vars
            .set(313, VarValue::Int(1));
        let texts = settexts(&click(engine, &mut client, com(EMOTES, 29)));
        assert!(
            !texts.iter().any(|t| t == "You can't use that emote yet."),
            "Glass Wall given; sent {texts:?}"
        );
    });
}

