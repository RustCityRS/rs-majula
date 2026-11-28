//! 289's quest journals on 2011's quest tab.
//!
//! 2011's quest tab (questlist_v4, interface 190) is one list, a row per 2011
//! quest id (clientscript 2193); View, a row's op 1, is IF_BUTTON1 on
//! component 18 with the row as its slot,
//! answered by `[if_button1,questlist_v4:com18]`. The server
//! opens that quest's 289 journal on 2011's journal scroll
//! (questjournal_scroll, interface 275: the name on component 2, `ifquestname`,
//! the lines from 16, `qj1`, as the client numbers them), its 289 colour codes
//! as 2011's tags.

#![cfg(all(since_647, before_742))]

use rs_test::names::com;
use rs_test::runner;
use rs_test::server;
use rs_ver::TypeId;

fn quest_list() -> i32 {
    com("questlist_v4:quests")
}

const COOKS_ASSISTANT: i32 = 1;
const JOURNAL: u16 = 275;
fn journal_name() -> u32 {
    com("questjournal_scroll:death_ifquestname") as u32
}

fn journal_line_1() -> u32 {
    com("questjournal_scroll:qj1") as u32
}

/// Each IF_SETTEXT in `packets`: (component hash, text).
fn settexts(packets: &[(u16, Vec<u8>)]) -> Vec<(u32, String)> {
    server::settexts(packets)
        .into_iter()
        .map(|(c, t)| (c as u32, t))
        .collect()
}

#[test]
fn view_on_cooks_assistant_opens_its_2011_journal() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("questor").expect("login");
        engine.cycle();
        client.packets();

        client.if_button(1, quest_list(), COOKS_ASSISTANT as u16, TypeId::NULL);
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
            text(journal_name()),
            // 2011's name (its quest struct's), as 289's quest list has it;
            // 289's journal heads it "The Cook's Quest".
            Some("<col=800000>Cook's Assistant"),
            "the journal's name; sent {texts:?}"
        );
        assert_eq!(
            text(journal_line_1()),
            Some(
                "<col=000080>I can start this quest by speaking to the <col=800000>Cook <col=000080>in the"
            ),
            "the journal's first line"
        );
    });
}

/// View on Doric's Quest (row 3) opens its journal with 2011's not-started text, as the
/// 2010-08-17 recording (YouTube I3zXxHk5zY0, 8-11s) shows it: one colour, "north of", and the
/// line wrapped before "Falador.".
#[test]
fn view_on_dorics_quest_opens_its_2011_journal() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine
            .harness_login_returning("doric_reader")
            .expect("login");
        engine.cycle();
        client.packets();

        client.if_button(1, quest_list(), 3, TypeId::NULL);
        let mut texts = Vec::new();
        for _ in 0..3 {
            engine.cycle();
            texts.extend(settexts(&client.packets()));
        }
        let text = |com: u32| {
            texts
                .iter()
                .find(|(c, _)| *c == com)
                .map(|(_, t)| t.as_str())
        };
        assert_eq!(
            text(journal_name()),
            Some("<col=800000>Doric's Quest"),
            "sent {texts:?}"
        );
        assert_eq!(
            text(journal_line_1()),
            Some("<col=000080>I can start this quest by speaking to Doric who is north of"),
            "sent {texts:?}"
        );
    });
}
