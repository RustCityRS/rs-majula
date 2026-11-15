//! 289's dialogue procs (interface_chat/scripts/chat.rs2) on 2011's boxes.
//!
//! Each case says one through a dev cheat (`_test/scripts/cheats/
//! cheat_chat.rs2`) and reads what the client is sent:
//! - `chatnc_cutscene`: a cutscene line in 2011's npc box with no continue,
//!   npcchat_np1-4 (245-248: name com3, lines com4-7), and no wait.
//! - `chatnpc_model`: npcchat1 (241) with a model for its head (com2).
//! - `chatnpcnoturn` and the second-npc `.chatnpc`, `.chatnpcrange`,
//!   `.chatnpcnoturn`: npcchat1 with the right speaker's name.
//! - `p_choice5_header` and `multi5_header`: multi5 (234) under the caller's
//!   header, with the wide swords (com10) shown and the thin (com9) hidden.
//! - `doubleobjbox`, `doubleobjbox2`, `objbox`: 2011's one text component (131
//!   com1, 519 com1), 4 lines 380 wide to a page.
//! - `.mesbox`: message1 (210) opened for the second player.
//!
//! "Click here to continue" and an option are RESUME_PAUSEBUTTON.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::player::StaffModLevel;
use rs_test::names::com;
use rs_test::runner;
use rs_test::server;

const NONE: i32 = 0xFFFF;
/// Lumbridge, east of the castle.
const HOME: (u16, u16) = (3222, 3218);

fn com_id(iface: u16, index: u16) -> i32 {
    (iface as i32) << 16 | index as i32
}

/// Every IF_SETTEXT, `p4(com) jstr(text)`.
fn settexts(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, String)> {
    server::settexts(packets)
}

/// The text last set on `com`.
fn text_of(texts: &[(i32, String)], com: i32) -> Option<&str> {
    texts
        .iter()
        .rev()
        .find(|(c, _)| *c == com)
        .map(|(_, t)| t.as_str())
}

/// Every IF_SETMODEL, `p2_alt3(model) p4(com)`, as (com, model).
fn setmodels(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, u16)> {
    server::models(packets)
        .into_iter()
        .map(|(c, m)| (c, m as u16))
        .collect()
}

/// Every IF_SETANIM, `p4_alt1(com) p2_alt1(seq)`, as (com, seq).
fn setanims(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, u16)> {
    server::anims(packets)
        .into_iter()
        .map(|(c, s)| (c, s as u16))
        .collect()
}

/// Every IF_SETOBJECT, `p4_alt1(com) p4_alt3(scale) p2(obj)`, as (com, obj).
fn setobjects(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, u16)> {
    server::objects(packets)
        .into_iter()
        .map(|(c, o)| (c, o as u16))
        .collect()
}

/// Every IF_SETHIDE, `p4_alt3(com) p1_alt2(hidden)`, as (com, hidden).
fn sethides(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, bool)> {
    server::hides(packets)
}

fn developer(engine: &mut Engine, name: &str, x: u16) -> HarnessClient {
    let mut client = engine.harness_login_returning(name).expect("login");
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .staff_mod_level = StaffModLevel::Maintainer;
    engine.cycle();
    engine.harness_telejump(client.pid, x, 0, HOME.1);
    for _ in 0..12 {
        engine.cycle();
    }
    client.packets();
    client
}

fn cheat(client: &mut HarnessClient, input: &str) {
    client.client_cheat(input);
}

fn resume(client: &mut HarnessClient, com: i32) {
    client.resume_pausebutton(com, NONE as u16);
}

/// Cycles `n` ticks and returns the packets sent meanwhile.
fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<(u16, Vec<u8>)> {
    let mut packets = Vec::new();
    for _ in 0..n {
        engine.cycle();
        packets.extend(client.packets());
    }
    packets
}

fn is_open(engine: &Engine, client: &HarnessClient, iface: u16) -> bool {
    engine
        .harness_player(client.pid)
        .unwrap()
        .is_interface_open(iface)
}

fn messages(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    HarnessClient::game_messages(packets)
        .into_iter()
        .map(|(_, t)| t)
        .collect()
}

#[test]
fn npc_chats_name_their_speaker_on_2011_s_npc_boxes() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = developer(engine, "chatnpcs", HOME.0);

        // A cutscene's two lines: npcchat_np2, no wait.
        cheat(&mut client, "~chat_cutscene");
        let packets = ticks(engine, &mut client, 2);
        let texts = settexts(&packets);
        assert!(is_open(engine, &client, 246), "npcchat_np2; sent {texts:?}");
        assert_eq!(
            text_of(&texts, com("npcchat_np2:npcchat_np2name")),
            Some("Man")
        );
        assert_eq!(
            text_of(&texts, com("npcchat_np2:npcchat_np2a")),
            Some("Hey, what's that mist coming in through the door!")
        );
        assert_eq!(
            text_of(&texts, com("npcchat_np2:npcchat_np2b")),
            Some("Team, we have a vampire in the room!")
        );
        assert!(
            messages(&packets).contains(&"chat_cutscene done".to_string()),
            "the cutscene line waits for no click"
        );
        // 289's <p,angry> plays on the head; with no expression it held still.
        let head = setanims(&packets);
        assert!(
            head.iter()
                .any(|&(c, seq)| c == com("npcchat_np2:npcchat_np2head_1") && seq != u16::MAX),
            "the man's head plays his expression; anims sent {head:?}"
        );
        // The cutscene's box stays up for the cutscene to close; an open
        // modal keeps the next cheat's p_finduid from the player.
        engine.harness_close_modal(client.pid);
        engine.cycle();

        // A model for a head.
        cheat(&mut client, "~chat_model");
        let packets = ticks(engine, &mut client, 2);
        let texts = settexts(&packets);
        assert!(is_open(engine, &client, 241), "npcchat1; sent {texts:?}");
        assert_eq!(
            text_of(&texts, com("npcchat1:npcchat1name")),
            Some("Parrot")
        );
        assert_eq!(
            text_of(&texts, com("npcchat1:npcchat1a")),
            Some("Pieces of eight!")
        );
        assert!(
            setmodels(&packets).contains(&(com("npcchat1:npcchat1head_1"), 3624)),
            "the head is model 3624; {:?}",
            setmodels(&packets)
        );
        resume(&mut client, com("npcchat1:npcchat1next"));
        let packets = ticks(engine, &mut client, 2);
        assert!(messages(&packets).contains(&"chat_model done".to_string()));

        // The active npc, then the second npc three times.
        cheat(&mut client, "~chat_npcs");
        let mut said = Vec::new();
        for _ in 0..4 {
            let packets = ticks(engine, &mut client, 2);
            let texts = settexts(&packets);
            assert!(is_open(engine, &client, 241), "npcchat1; sent {texts:?}");
            said.push((
                text_of(&texts, com("npcchat1:npcchat1name"))
                    .unwrap_or_default()
                    .to_string(),
                text_of(&texts, com("npcchat1:npcchat1a"))
                    .unwrap_or_default()
                    .to_string(),
            ));
            resume(&mut client, com("npcchat1:npcchat1next"));
        }
        let packets = ticks(engine, &mut client, 2);
        assert_eq!(
            said,
            [
                ("Man", "I keep working."),
                ("Woman", "I answer for him."),
                ("Woman", "From over here."),
                ("Woman", "Still busy."),
            ]
            .map(|(a, b)| (a.to_string(), b.to_string()))
        );
        assert!(messages(&packets).contains(&"chat_npcs done".to_string()));
    });
}

#[test]
fn five_options_under_the_caller_s_header() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = developer(engine, "choice5", HOME.0);

        cheat(&mut client, "~chat_choice5_header");
        let packets = ticks(engine, &mut client, 2);
        let texts = settexts(&packets);
        assert!(is_open(engine, &client, 234), "multi5; sent {texts:?}");
        assert_eq!(
            text_of(&texts, com("multi5:multi5t")),
            Some("Pick a number")
        );
        assert_eq!(text_of(&texts, com("multi5:multi5e")), Some("Five"));
        let hides = sethides(&packets);
        assert!(
            hides.contains(&(com("multi5:multi5swords"), true)),
            "{hides:?}"
        );
        assert!(hides.contains(&(com("multi5:com_10"), false)), "{hides:?}");

        // The third option, com4.
        resume(&mut client, com("multi5:multi5c"));
        let packets = ticks(engine, &mut client, 2);
        assert!(
            messages(&packets).contains(&"picked 3".to_string()),
            "{:?}",
            messages(&packets)
        );
        assert_eq!(
            text_of(&settexts(&packets), com("multi5:multi5t")),
            Some("Pick again")
        );

        // multi5_header jumps to the fifth option's label.
        resume(&mut client, com("multi5:multi5e"));
        let packets = ticks(engine, &mut client, 2);
        assert!(
            messages(&packets).contains(&"label five".to_string()),
            "{:?}",
            messages(&packets)
        );
    });
}

#[test]
fn obj_boxes_page_their_text_in_2011_s_one_text_component() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = developer(engine, "objboxes", HOME.0);
        let amulet = engine
            .cache
            .objs
            .get_by_debugname("strung_gold_amulet")
            .unwrap()
            .id;
        let bowl = engine.cache.objs.get_by_debugname("bowl_empty").unwrap().id;

        for (cheat_name, iface, done) in [
            ("~chat_doubleobjbox", 131, "chat_doubleobjbox done"),
            ("~chat_objbox", 519, "chat_objbox done"),
        ] {
            cheat(&mut client, cheat_name);
            let packets = ticks(engine, &mut client, 2);
            let texts = settexts(&packets);
            assert!(is_open(engine, &client, iface), "{iface}; sent {texts:?}");
            let page1 = text_of(&texts, com_id(iface, 1))
                .unwrap_or_default()
                .to_string();
            assert_eq!(
                page1.matches("<br>").count(),
                3,
                "{cheat_name}: four lines on the first page: {page1:?}"
            );
            let continue_com = if iface == 131 { 3 } else { 2 };
            let mut pages = vec![page1];
            let mut finished = false;
            for _ in 0..5 {
                resume(&mut client, com_id(iface, continue_com));
                let packets = ticks(engine, &mut client, 2);
                if messages(&packets).contains(&done.to_string()) {
                    finished = true;
                    break;
                }
                let texts = settexts(&packets);
                assert!(is_open(engine, &client, iface), "{cheat_name}: a next page");
                pages.push(
                    text_of(&texts, com_id(iface, 1))
                        .unwrap_or_default()
                        .to_string(),
                );
            }
            assert!(finished, "{cheat_name} returns; pages {pages:?}");
            assert!(
                pages.len() > 1,
                "{cheat_name}: more than one page: {pages:?}"
            );
            assert!(
                pages
                    .iter()
                    .all(|p| !p.is_empty() && p.matches("<br>").count() < 4),
                "{cheat_name}: at most four lines a page: {pages:?}"
            );
        }

        cheat(&mut client, "~chat_doubleobjbox2");
        let packets = ticks(engine, &mut client, 2);
        let texts = settexts(&packets);
        assert!(
            is_open(engine, &client, 131),
            "doubleobjbox; sent {texts:?}"
        );
        assert_eq!(
            text_of(&texts, com("doubleobjbox:doubleobjbox_text")),
            Some("You can now craft Bowls and Gold Amulets.")
        );
        let objs = setobjects(&packets);
        assert!(
            objs.contains(&(com("doubleobjbox:doubleobjboxmodel1"), amulet.g2())),
            "{objs:?}"
        );
        assert!(
            objs.contains(&(com("doubleobjbox:doubleobjboxmodel2"), bowl.g2())),
            "{objs:?}"
        );
        resume(&mut client, com("doubleobjbox:doubleobjbox_pausebutton"));
        let packets = ticks(engine, &mut client, 2);
        assert!(messages(&packets).contains(&"chat_doubleobjbox2 done".to_string()));
    });
}

#[test]
fn a_message_box_for_the_second_player() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut a = developer(engine, "mesbox_a", HOME.0);
        let mut b = developer(engine, "mesbox_b", HOME.0 + 1);
        cheat(&mut a, "~chat_dotmesbox");
        engine.cycle();
        engine.cycle();
        let texts = settexts(&b.packets());
        a.packets();
        assert!(is_open(engine, &b, 210), "message1 for b; sent {texts:?}");
        assert!(!is_open(engine, &a, 210), "not for a");
        assert_eq!(
            text_of(&texts, com("message1:mes1a")),
            Some("A message for the other player.")
        );
    });
}
