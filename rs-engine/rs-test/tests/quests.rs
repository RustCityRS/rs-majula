//! 289's quest journals on 2011's quest tab.
//!
//! 2011's quest tab (questlist_v4, interface 190) is one list, a row per 2011
//! quest id (clientscript 2193); View, a row's op 1, is IF_BUTTON1 on
//! component 18 with the row as its slot (`p4(com) ip2_alt1(obj) ip2(slot)`),
//! answered by `[if_button1,questlist_v4:com18]`. The server
//! opens that quest's 289 journal on 2011's journal scroll
//! (questjournal_scroll, interface 275: the name on component 2, `ifquestname`,
//! the lines from 16, `qj1`, as the client numbers them), its 289 colour codes
//! as 2011's tags.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

const QUEST_LIST: i32 = 190 << 16 | 18;
const COOKS_ASSISTANT: i32 = 1;
const JOURNAL: u16 = 275;
const JOURNAL_NAME: u32 = 275 << 16 | 2;
const JOURNAL_LINE_1: u32 = 275 << 16 | 16;

/// Each IF_SETTEXT in `packets`: (component hash, text).
fn settexts(packets: &[(u16, Vec<u8>)]) -> Vec<(u32, String)> {
    let opcode = ServerProt::IfSetText as u16;
    packets
        .iter()
        .filter(|(op, _)| *op == opcode)
        .map(|(_, p)| {
            let com = u32::from_be_bytes([p[0], p[1], p[2], p[3]]);
            let end = p[4..]
                .iter()
                .position(|&b| b == 0)
                .map_or(p.len(), |i| 4 + i);
            (com, String::from_utf8_lossy(&p[4..end]).into_owned())
        })
        .collect()
}

#[test]
fn view_on_cooks_assistant_opens_its_2011_journal() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("questor").expect("login");
        engine.cycle();
        client.packets();

        let mut p = ClientPacket::new();
        p.p4(QUEST_LIST).ip2_alt1(-1).ip2(COOKS_ASSISTANT);
        client.send(ClientProt::IfButton1, &p.finish());
        let mut texts = Vec::new();
        for _ in 0..3 {
            engine.cycle();
            texts.extend(settexts(&client.packets()));
        }
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(JOURNAL),
            "the journal scroll is open"
        );
        // In the toplevel's full-size slot (void-634's `wide_screen`), which
        // the journal is laid out for: in the 512x334 main slot its text ran
        // past the scroll.
        let player = engine.harness_player(client.pid).unwrap();
        let wide = if player.toplevel == 548 { 8 } else { 6 };
        assert_eq!(
            player.modal_main_parent,
            (player.toplevel as i32) << 16 | wide
        );
        let text = |com: u32| {
            texts
                .iter()
                .find(|(c, _)| *c == com)
                .map(|(_, t)| t.as_str())
        };
        assert_eq!(
            text(JOURNAL_NAME),
            // 2011's name (its quest struct's), as 289's quest list has it;
            // 289's journal heads it "The Cook's Quest".
            Some("<col=800000>Cook's Assistant"),
            "the journal's name; sent {texts:?}"
        );
        assert_eq!(
            text(JOURNAL_LINE_1),
            Some(
                "<col=000080>I can start this quest by speaking to the <col=800000>Cook <col=000080>in the"
            ),
            "the journal's first line"
        );
    });
}
