//! 289's multi-choice dialogues answer 2011's option clicks.
//!
//! 289's `~p_choice2..5` open a multi box, name each option a resume button
//! (`if_addresumebutton`) and pause; the option clicked comes back as
//! `last_com`. 2011's client sends that click as RESUME_PAUSEBUTTON
//! (`p2(slot) p4_alt2(com)`). The case talks to Lumbridge's Cook as the client
//! does: Talk-to (OPNPC1, `p1_alt2(ctrl) p2(npc)`), "Click here to continue"
//! on his line (npc_chat1, 241, component 5), then the second of his four
//! options (multi4, 232: the options are components 2-5), and the player says
//! it.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

const NPC_CHAT_1: u16 = 241;
const NPC_CONTINUE: i32 = 241 << 16 | 5;
const MULTI4: u16 = 232;
const SECOND_OPTION: i32 = 232 << 16 | 3;
const NONE: i32 = 0xFFFF;

/// Every IF_SETTEXT's text in `packets`.
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

/// Cycles until `iface` is open, at most `n` ticks, and returns the texts set.
fn until_open(
    engine: &mut Engine,
    client: &mut HarnessClient,
    iface: u16,
    n: usize,
) -> Vec<String> {
    let mut texts = Vec::new();
    for _ in 0..n {
        engine.cycle();
        texts.extend(settexts(&client.packets()));
        if engine
            .harness_player(client.pid)
            .unwrap()
            .is_interface_open(iface)
        {
            break;
        }
    }
    texts
}

fn resume(client: &mut HarnessClient, com: i32) {
    let mut p = ClientPacket::new();
    p.p2(NONE).p4_alt2(com);
    client.send(ClientProt::ResumePauseButton, &p.finish());
}

#[test]
fn the_cook_s_choice_answers_the_option_clicked() {
    runner::on_loaded_engine(|engine| {
        let cook = *engine.cache.npcs.debugnames.get("cook").expect("npc cook");
        let (nid, x, z) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .map(|n| (n.npc.uid.nid(), n.npc.uid.id(), n.npc.pathing.coord))
            .find(|(_, id, _)| *id == cook)
            .map(|(nid, _, c)| (nid, c.x(), c.z()))
            .expect("Lumbridge's cook");

        engine.harness_reset();
        let mut client = engine.harness_login_returning("chatter").expect("login");
        engine.cycle();
        engine.harness_telejump(client.pid, x + 1, 0, z);
        engine.cycle();
        client.packets();

        let mut p = ClientPacket::new();
        p.p1_alt2(0).p2(nid as i32);
        client.send(ClientProt::OpNpc1, &p.finish());
        let texts = until_open(engine, &mut client, NPC_CHAT_1, 10);
        assert!(
            texts.iter().any(|t| t == "What am I to do?"),
            "the cook speaks; sent {texts:?}"
        );

        resume(&mut client, NPC_CONTINUE);
        let texts = until_open(engine, &mut client, MULTI4, 3);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(MULTI4),
            "his four options; sent {texts:?}"
        );

        resume(&mut client, SECOND_OPTION);
        let mut texts = Vec::new();
        for _ in 0..3 {
            engine.cycle();
            texts.extend(settexts(&client.packets()));
        }
        // void-634 Cook's second option (after Cook's Assistant's rework).
        assert!(
            texts.iter().any(|t| t == "Can you make me a cake?"),
            "the player says the second option; sent {texts:?}"
        );
    });
}

/// Every IF_SETTEXT in `packets` on a chat line: npc_chat1-4 (241-244) or
/// chat1-4 (64-67), components 4-7.
fn chat_lines(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    let opcode = ServerProt::IfSetText as u16;
    packets
        .iter()
        .filter(|(op, _)| *op == opcode)
        .filter_map(|(_, p)| {
            let com = u32::from_be_bytes([p[0], p[1], p[2], p[3]]);
            let (iface, index) = (com >> 16, com & 0xFFFF);
            let chat = (241..=244).contains(&iface) || (64..=67).contains(&iface);
            (chat && (4..=7).contains(&index)).then(|| {
                let end = p[4..]
                    .iter()
                    .position(|&b| b == 0)
                    .map_or(p.len(), |i| 4 + i);
                String::from_utf8_lossy(&p[4..end]).into_owned()
            })
        })
        .collect()
}

/// 2011's chat lines are 350 wide in q8_full (font 497); 289 wrapped at 380
/// in p12_full, and its long lines ran past the box. The Cook's "What's
/// wrong?" answer is his longest.
#[test]
fn the_cook_s_long_answer_fits_2011_s_chat_box() {
    runner::on_loaded_engine(|engine| {
        let cook = *engine.cache.npcs.debugnames.get("cook").expect("npc cook");
        let (nid, x, z) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .map(|n| (n.npc.uid.nid(), n.npc.uid.id(), n.npc.pathing.coord))
            .find(|(_, id, _)| *id == cook)
            .map(|(nid, _, c)| (nid, c.x(), c.z()))
            .expect("Lumbridge's cook");

        engine.harness_reset();
        let mut client = engine.harness_login_returning("longtalk").expect("login");
        engine.cycle();
        engine.harness_telejump(client.pid, x + 1, 0, z);
        engine.cycle();
        client.packets();

        let mut p = ClientPacket::new();
        p.p1_alt2(0).p2(nid as i32);
        client.send(ClientProt::OpNpc1, &p.finish());
        until_open(engine, &mut client, NPC_CHAT_1, 10);
        resume(&mut client, NPC_CONTINUE);
        until_open(engine, &mut client, MULTI4, 3);
        resume(&mut client, 232 << 16 | 2);

        // The player's "What's wrong?", then the Cook's answer, a page at a
        // time: continue whatever chat box is open.
        let mut lines = Vec::new();
        for _ in 0..12 {
            for _ in 0..2 {
                engine.cycle();
                lines.extend(chat_lines(&client.packets()));
            }
            let open = [64u16, 65, 66, 67, 241, 242, 243, 244]
                .into_iter()
                .find(|&i| {
                    engine
                        .harness_player(client.pid)
                        .unwrap()
                        .is_interface_open(i)
                });
            let Some(iface) = open else { break };
            let lines_in = if iface >= 241 {
                iface - 240
            } else {
                iface - 63
            };
            resume(&mut client, (iface as i32) << 16 | (4 + lines_in as i32));
        }
        let font = engine.cache.fonts.get_by_id(497).expect("q8_full");
        assert!(lines.len() > 3, "the Cook's answer; sent {lines:?}");
        for line in &lines {
            assert!(
                font.string_width(line) <= 350,
                "{line:?} is {} wide in q8_full",
                font.string_width(line)
            );
        }
    });
}
