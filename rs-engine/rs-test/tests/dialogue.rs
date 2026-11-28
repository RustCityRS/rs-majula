//! 289's multi-choice dialogues answer 2011's option clicks.
//!
//! 289's `~p_choice2..5` open a multi box, name each option a resume button
//! (`if_addresumebutton`) and pause; the option clicked comes back as
//! `last_com`. 2011's client sends that click as RESUME_PAUSEBUTTON. The case
//! talks to Lumbridge's Cook as the client does: Talk-to (OPNPC1), "Click here
//! to continue" on his line (npc_chat1, 241, component 5), then the second of
//! his four options (multi4, 232: the options are components 2-5), and the
//! player says it.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_test::names::com;
use rs_test::runner;
use rs_test::server;

const NPC_CHAT_1: u16 = 241;
fn npc_continue() -> i32 {
    com("npcchat1:npcchat1next")
}

const MULTI4: u16 = 232;
fn second_option() -> i32 {
    com("multi4:multi4b")
}

const NONE: i32 = 0xFFFF;

/// Every IF_SETTEXT's text in `packets`.
fn settexts(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    server::settexts(packets)
        .into_iter()
        .map(|(_, t)| t)
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
    client.resume_pausebutton(com, NONE as u16);
}

#[test]
fn the_cook_s_choice_answers_the_option_clicked() {
    runner::on_loaded_engine(|engine| {
        let cook = engine
            .cache
            .npcs
            .get_by_debugname("cook")
            .expect("npc cook")
            .id;
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

        client.op_npc(1, nid);
        let texts = until_open(engine, &mut client, NPC_CHAT_1, 10);
        assert!(
            texts.iter().any(|t| t == "What am I to do?"),
            "the cook speaks; sent {texts:?}"
        );

        resume(&mut client, npc_continue());
        let texts = until_open(engine, &mut client, MULTI4, 3);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(MULTI4),
            "his four options; sent {texts:?}"
        );

        resume(&mut client, second_option());
        let mut texts = Vec::new();
        for _ in 0..3 {
            engine.cycle();
            texts.extend(settexts(&client.packets()));
        }
        // void-634 Cook's second option (after Cook's Assistant's rework); 530 keeps 289's.
        #[cfg(all(since_530, before_647))]
        let second = "Well you could give me all your money!";
        #[cfg(since_647)]
        let second = "Can you make me a cake?";
        assert!(
            texts.iter().any(|t| t == second),
            "the player says the second option; sent {texts:?}"
        );
    });
}

/// Every IF_SETTEXT in `packets` on a chat line: npc_chat1-4 (241-244) or
/// chat1-4 (64-67), components 4-7.
fn chat_lines(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    server::settexts(packets)
        .into_iter()
        .filter_map(|(com, text)| {
            let (iface, index) = (com >> 16, com & 0xFFFF);
            let chat = (241..=244).contains(&iface) || (64..=67).contains(&iface);
            (chat && (4..=7).contains(&index)).then_some(text)
        })
        .collect()
}

/// 2011 wrapped chat lines at 380 in q8_full (font 497), past their 350-wide
/// components. The Cook's "What's wrong?" answer ends its lines where the
/// 2011-05-20 recording (YouTube PnoZkxqcszs, 82.1s and 83.4s) does.
#[test]
fn the_cook_s_long_answer_wraps_as_2011_s_does() {
    runner::on_loaded_engine(|engine| {
        let cook = engine
            .cache
            .npcs
            .get_by_debugname("cook")
            .expect("npc cook")
            .id;
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

        client.op_npc(1, nid);
        until_open(engine, &mut client, NPC_CHAT_1, 10);
        resume(&mut client, npc_continue());
        until_open(engine, &mut client, MULTI4, 3);
        resume(&mut client, com("multi4:multi4a"));

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
        for line in &lines {
            assert!(
                font.string_width(line) <= 380,
                "{line:?} is {} wide in q8_full",
                font.string_width(line)
            );
        }
        // 530 keeps 289's answer, which no 2009 recording shows wrapped.
        #[cfg(since_647)]
        let recorded = [
            "Oh dear, oh dear, oh dear, I'm in a terrible, terrible",
            "mess! It's the Duke's birthday today, and I should be",
            "making him a lovely, big birthday cake using special",
            "ingredients...",
            "...but I've forgotten to get the ingredients. I'll never get",
            "them in time now. He'll sack me! What ever will I do? I",
            "have four children and a goat to look after. Would you",
            "help me? Please?",
        ];
        #[cfg(since_647)]
        {
            let start = lines
                .iter()
                .position(|l| l == recorded[0])
                .unwrap_or_else(|| panic!("2011's first line; sent {lines:?}"));
            assert_eq!(
                &lines[start..start + recorded.len()],
                &recorded[..],
                "2011's line ends; sent {lines:?}"
            );
        }
    });
}
