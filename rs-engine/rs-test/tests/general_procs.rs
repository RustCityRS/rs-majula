//! 289's general procs (general/scripts) on 2011's interfaces.
//!
//! - `send_quest_complete2`: 2011's quest scroll (277) with the rewards given
//!   as text on its reward lines (com11-17), below the quest's points (com10).
//! - The lamp: Rub (inventory op 1, IF_BUTTON1 on `inventory:inv`, `p4(com)
//!   p2_alt3(obj) p2_alt1(slot)`) opens 2011's lamp (134); a stat's Select sets
//!   varp 261 to 289's ^xplamp_ value for it; Confirm advances the stat by ten
//!   times its level.
//! - 289's scroll reads: 2011's messagescroll (220), 289's lines one higher.
//!
//! Cheats: `_test/scripts/cheats/cheat_general.rs2` and `cheat_give.rs2`.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::player::StaffModLevel;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

/// Lumbridge, east of the castle.
const HOME: (u16, u16) = (3222, 3218);
/// `inventory:inv`, the backpack component `[login]` binds `inv` to.
const BACKPACK: i32 = 149 << 16;
const LAMP: u16 = 134;
const ATTACK: usize = 0;

fn com(iface: u16, index: u16) -> i32 {
    (iface as i32) << 16 | index as i32
}

/// Every IF_SETTEXT, `p4(com) jstr(text)`.
fn settexts(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, String)> {
    let opcode = ServerProt::IfSetText as u16;
    packets
        .iter()
        .filter(|(op, _)| *op == opcode)
        .map(|(_, p)| {
            let com = i32::from_be_bytes([p[0], p[1], p[2], p[3]]);
            let end = p[4..]
                .iter()
                .position(|&b| b == 0)
                .map_or(p.len(), |i| 4 + i);
            (com, String::from_utf8_lossy(&p[4..end]).into_owned())
        })
        .collect()
}

/// The text last set on `com`.
fn text_of(texts: &[(i32, String)], com: i32) -> Option<&str> {
    texts
        .iter()
        .rev()
        .find(|(c, _)| *c == com)
        .map(|(_, t)| t.as_str())
}

fn developer(engine: &mut Engine, name: &str) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .staff_mod_level = StaffModLevel::Maintainer;
    engine.cycle();
    engine.harness_telejump(client.pid, HOME.0, 0, HOME.1);
    for _ in 0..12 {
        engine.cycle();
    }
    client.packets();
    client
}

fn cheat(client: &mut HarnessClient, input: &str) {
    let mut p = ClientPacket::new();
    p.p1(0).p1(0).pjstr(input);
    client.send(ClientProt::ClientCheat, &p.finish());
}

fn button1(client: &mut HarnessClient, com: i32, slot: i32, obj: i32) {
    let mut p = ClientPacket::new();
    p.p4(com).p2_alt3(obj).p2_alt1(slot);
    client.send(ClientProt::IfButton1, &p.finish());
}

/// Inventory op 4 (Rub) on the backpack's slot 0.
fn rub(client: &mut HarnessClient, obj: u16) {
    let mut p = ClientPacket::new();
    p.p4(BACKPACK).p2_alt3(obj as i32).p2_alt1(0);
    client.send(ClientProt::IfButton4, &p.finish());
}

/// An option of a multi-choice box: RESUME_PAUSEBUTTON, `p2(slot) p4_alt2(com)`.
fn resume(client: &mut HarnessClient, com: i32) {
    let mut p = ClientPacket::new();
    p.p2(0xFFFF).p4_alt2(com);
    client.send(ClientProt::ResumePauseButton, &p.finish());
}

fn coord(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
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

fn messages(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    HarnessClient::game_messages(packets)
        .into_iter()
        .map(|(_, t)| t)
        .collect()
}

fn is_open(engine: &Engine, client: &HarnessClient, iface: u16) -> bool {
    engine
        .harness_player(client.pid)
        .unwrap()
        .is_interface_open(iface)
}

fn varp(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let id = *engine
        .cache
        .varps
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no varp {name}"));
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(id)
        .as_int()
}

fn inv_total(engine: &Engine, client: &HarnessClient, obj: u16) -> u32 {
    let id = *engine.cache.invs.debugnames.get("inv").unwrap();
    let inv = engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&id)
        .expect("inv");
    (0..inv.capacity)
        .filter_map(|slot| inv.get(slot as u16))
        .filter(|i| i.obj == obj)
        .map(|i| i.num)
        .sum()
}

#[test]
fn quest_complete2_lists_the_rewards_on_2011_s_scroll() {
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "questdone");
        cheat(&mut client, "~quest_complete2");
        let packets = ticks(engine, &mut client, 2);
        let texts = settexts(&packets);
        assert!(is_open(engine, &client, 277), "the scroll; sent {texts:?}");
        assert_eq!(
            text_of(&texts, com(277, 4)),
            Some("'In Search of the Myreque' completed.")
        );
        assert_eq!(text_of(&texts, com(277, 10)), Some("2 Quest Points"));
        assert_eq!(text_of(&texts, com(277, 11)), Some("Attack & Defence XP"));
        assert_eq!(
            text_of(&texts, com(277, 12)),
            Some("Strength & Hitpoint XP")
        );
        assert_eq!(text_of(&texts, com(277, 13)), Some("Crafting XP"));
        assert_eq!(
            text_of(&texts, com(277, 14)),
            Some("Quick Route to Mort'ton")
        );
        assert_eq!(text_of(&texts, com(277, 15)), Some(""));
    });
}

#[test]
fn the_lamp_advances_the_stat_picked_on_2011_s_lamp() {
    runner::on_loaded_engine(|engine| {
        let lamp = *engine.cache.objs.debugnames.get("lamp").unwrap();
        let mut client = developer(engine, "lamprub");
        cheat(&mut client, "~give lamp 1");
        ticks(engine, &mut client, 2);
        assert_eq!(inv_total(engine, &client, lamp), 1, "the lamp given");

        button1(&mut client, BACKPACK, 0, lamp as i32);
        ticks(engine, &mut client, 2);
        assert!(is_open(engine, &client, LAMP), "Rub opens 2011's lamp");
        assert_eq!(varp(engine, &client, "xplamp_stat"), 0);

        // Herblore wants Druidic Ritual first, as 289's.
        button1(&mut client, com(LAMP, 38), 0xFFFF, 0xFFFF);
        let said = messages(&ticks(engine, &mut client, 1));
        assert!(
            said.contains(
                &"You need to complete the Druidic Ritual quest to choose that option.".to_string()
            ),
            "{said:?}"
        );
        assert_eq!(varp(engine, &client, "xplamp_stat"), 0);

        // Each button its stat's value: the box clientscript 790 lights.
        for (button, value) in [(54, 25), (52, 22), (50, 23), (41, 19), (47, 18), (30, 1)] {
            button1(&mut client, com(LAMP, button), 0xFFFF, 0xFFFF);
            ticks(engine, &mut client, 1);
            assert_eq!(
                varp(engine, &client, "xplamp_stat"),
                value,
                "button {button}"
            );
        }

        // Confirm: Attack, at level 1, takes 10 xp (the engine's tenths).
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[ATTACK];
        button1(&mut client, com(LAMP, 2), 0xFFFF, 0xFFFF);
        let packets = ticks(engine, &mut client, 2);
        let texts = settexts(&packets);
        assert_eq!(
            engine.harness_player(client.pid).unwrap().stats.xp[ATTACK] - xp,
            100
        );
        assert_eq!(inv_total(engine, &client, lamp), 0, "the lamp is used up");
        assert_eq!(varp(engine, &client, "xplamp_stat"), 0);
        assert!(!is_open(engine, &client, LAMP), "the lamp closes");
        assert!(
            texts
                .iter()
                .any(|(_, t)| t == "You have been awarded 10 Attack experience!"),
            "{texts:?}"
        );
    });
}

#[test]
fn a_289_scroll_reads_on_2011_s_message_scroll() {
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "scrollread");
        cheat(&mut client, "~scroll_hazelmere");
        let packets = ticks(engine, &mut client, 2);
        let texts = settexts(&packets);
        assert!(
            is_open(engine, &client, 220),
            "messagescroll; sent {texts:?}"
        );
        // 289's com_6-8 are 2011's com5-7.
        assert_eq!(
            text_of(&texts, com(220, 5)),
            Some("Es lemanto meso pro eis prit ta Cinqo mond.")
        );
        assert_eq!(
            text_of(&texts, com(220, 7)),
            Some("et Daconia arpos eto meriz ta priw!")
        );
        assert_eq!(text_of(&texts, com(220, 15)), Some(""));
    });
}

/// Rubs `name` (given to slot 0), picks the multi box's option `com`, and
/// returns where the player lands, the obj then in slot 0 and what was said.
/// One player rubs each: a staff player removed by a harness reset underflows
/// the player list's isolated count (its level was raised after login).
fn rub_and_go(
    engine: &mut Engine,
    client: &mut HarnessClient,
    name: &str,
    multi: u16,
    option: u16,
) -> ((u16, u16), Option<u16>, Vec<String>) {
    let obj = *engine.cache.objs.debugnames.get(name).unwrap();
    engine.harness_telejump(client.pid, HOME.0, 0, HOME.1);
    cheat(client, "~clearinv inv");
    ticks(engine, client, 2);
    cheat(client, &format!("~give {name} 1"));
    ticks(engine, client, 2);
    rub(client, obj);
    let mut said = messages(&ticks(engine, client, 3));
    assert!(
        is_open(engine, client, multi),
        "{name}: the options; said {said:?}"
    );
    resume(client, com(multi, option));
    said.extend(messages(&ticks(engine, client, 8)));
    let id = *engine.cache.invs.debugnames.get("inv").unwrap();
    let left = engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&id)
        .unwrap()
        .get(0)
        .map(|i| i.obj);
    (coord(engine, client), left, said)
}

fn near(at: (u16, u16), x: (u16, u16), z: (u16, u16)) -> bool {
    (x.0.saturating_sub(3)..=x.1 + 3).contains(&at.0)
        && (z.0.saturating_sub(3)..=z.1 + 3).contains(&at.1)
}

// 2011's places (2011 wiki): the ring's Mobilising Armies (multi5, option 3 =
// com4), the necklace's Barbarian Outpost (multi3, option 2 = com3), the
// glory's Edgeville (multi5, option 1 = com2), each in void-634's area.
#[test]
fn teleport_jewellery_goes_to_2011_s_places() {
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "jeweller");
        let seven = *engine
            .cache
            .objs
            .debugnames
            .get("ring_of_duelling__7")
            .unwrap();
        let (at, left, said) = rub_and_go(engine, &mut client, "ring_of_duelling__8", 234, 4);
        assert!(
            near(at, (2412, 2415), (2847, 2849)),
            "Mobilising Armies: at {at:?}; said {said:?}"
        );
        assert_eq!(left, Some(seven));
        assert!(
            said.contains(&"Your Ring of Dueling has 7 uses left.".to_string()),
            "{said:?}"
        );

        let seven = *engine
            .cache
            .objs
            .debugnames
            .get("games_necklace__7")
            .unwrap();
        let (at, left, said) = rub_and_go(engine, &mut client, "games_necklace__8", 230, 3);
        assert!(
            near(at, (2519, 2521), (3572, 3574)),
            "Barbarian Outpost: at {at:?}; said {said:?}"
        );
        assert_eq!(left, Some(seven));

        let three = *engine
            .cache
            .objs
            .debugnames
            .get("amulet_of_glory__3")
            .unwrap();
        let (at, left, said) = rub_and_go(engine, &mut client, "amulet_of_glory__4", 234, 2);
        assert!(
            near(at, (3087, 3089), (3496, 3498)),
            "Edgeville: at {at:?}; said {said:?}"
        );
        assert_eq!(left, Some(three));
        assert!(
            said.contains(&"Your amulet has three charges left.".to_string()),
            "{said:?}"
        );
    });
}

/// Every IF_SETHIDE, `p4_alt3(com) p1_alt2(hidden)`, as (com, hidden).
fn sethides(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, bool)> {
    let opcode = ServerProt::IfSetHide as u16;
    packets
        .iter()
        .filter(|(op, _)| *op == opcode)
        .map(|(_, p)| {
            // p4_alt3 writes `>>16, >>24, v, >>8`; p1_alt2 writes `-v`.
            let com = i32::from_be_bytes([p[1], p[0], p[3], p[2]]);
            (com, p[4] != 0)
        })
        .collect()
}

fn hidden(hides: &[(i32, bool)], com: i32) -> Option<bool> {
    hides.iter().rev().find(|(c, _)| *c == com).map(|(_, h)| *h)
}

// 289's book on 2011's (959): its arrows are pause buttons, Back com28 and
// Next com29, and the book waits on its page for one.
#[test]
fn a_289_book_turns_its_pages_on_2011_s_book() {
    runner::on_loaded_engine(|engine| {
        const BOOK: u16 = 959;
        let mut client = developer(engine, "bookread");
        cheat(&mut client, "~book_test");
        let packets = ticks(engine, &mut client, 2);
        let (texts, hides) = (settexts(&packets), sethides(&packets));
        assert!(is_open(engine, &client, BOOK), "the book; sent {texts:?}");
        assert_eq!(text_of(&texts, com(BOOK, 5)), Some("Test Book"));
        assert_eq!(text_of(&texts, com(BOOK, 30)), Some("First page."));
        assert_eq!(text_of(&texts, com(BOOK, 52)), Some("1"));
        assert_eq!(
            hidden(&hides, com(BOOK, 28)),
            Some(true),
            "no Back on the first page"
        );
        assert_eq!(hidden(&hides, com(BOOK, 29)), Some(false), "Next");
        assert_eq!(hidden(&hides, com(BOOK, 7)), Some(true), "no obj by line 2");

        resume(&mut client, com(BOOK, 29));
        let packets = ticks(engine, &mut client, 2);
        let (texts, hides) = (settexts(&packets), sethides(&packets));
        assert!(is_open(engine, &client, BOOK));
        assert_eq!(text_of(&texts, com(BOOK, 30)), Some("Second page."));
        assert_eq!(text_of(&texts, com(BOOK, 31)), Some("With a dagger."));
        assert_eq!(text_of(&texts, com(BOOK, 52)), Some("3"));
        assert_eq!(
            hidden(&hides, com(BOOK, 7)),
            Some(false),
            "the dagger by line 2"
        );
        assert_eq!(
            hidden(&hides, com(BOOK, 29)),
            Some(true),
            "no Next on the last page"
        );

        resume(&mut client, com(BOOK, 28));
        let texts = settexts(&ticks(engine, &mut client, 2));
        assert_eq!(text_of(&texts, com(BOOK, 30)), Some("First page."));
    });
}

// 289's wine of zamorak at the Chaos Temple (2930, 3515): Take burns the
// thief, drains the combat stats and says "STOP STEALING MY WINE! GAH!".
#[test]
fn taking_the_temple_s_wine_burns_the_thief() {
    runner::on_loaded_engine(|engine| {
        let wine = *engine.cache.objs.debugnames.get("wine_of_zamorak").unwrap();
        let mut client = developer(engine, "winethief");
        engine.harness_add_obj(2930, 0, 3515, wine, 1);
        // West of it: east of it, past the wall, is a 2-wide loc (loc_15573).
        engine.harness_telejump(client.pid, 2929, 0, 3515);
        ticks(engine, &mut client, 2);
        let life = life_points(engine, &client);
        let mut p = ClientPacket::new();
        p.p2_alt1(wine as i32).p2_alt2(3515).p1(0).p2(2930);
        client.send(ClientProt::OpObj3, &p.finish());
        let said = messages(&ticks(engine, &mut client, 6));
        assert!(
            said.contains(&"STOP STEALING MY WINE! GAH!".to_string()),
            "{said:?}"
        );
        assert!(life_points(engine, &client) < life, "burnt");
    });
}

/// The player's life points: varp 1240, bits 1-15 (varbit life_points).
fn life_points(engine: &Engine, client: &HarnessClient) -> i32 {
    (engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(1240)
        .as_int()
        >> 1)
        & 0x7FFF
}
