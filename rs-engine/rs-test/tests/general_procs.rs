//! 289's general procs (general/scripts) on 2011's interfaces.
//!
//! - `send_quest_complete2`: 2011's quest scroll (277) with the rewards given
//!   as text on its reward lines (com11-17), below the quest's points (com10).
//! - The lamp: Rub (inventory op 1, IF_BUTTON1 on `inventory:inv`) opens 2011's
//!   lamp (134); a stat's Select sets varp 261 to 289's ^xplamp_ value for it;
//!   Confirm advances the stat by ten times its level.
//! - 289's scroll reads: 2011's messagescroll (220), 289's lines one higher.
//!
//! Cheats: `_test/scripts/cheats/cheat_general.rs2` and `cheat_give.rs2`.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::player::StaffModLevel;
#[cfg(since_647)]
use rs_test::life;
use rs_test::names::BACKPACK;
use rs_test::names::com;
use rs_test::runner;
use rs_test::server;
use rs_ver::TypeId;

/// Lumbridge, east of the castle.
const HOME: (u16, u16) = (3222, 3218);
#[cfg(before_742)]
#[cfg(since_647)]
const LAMP: u16 = 134;
#[cfg(before_742)]
#[cfg(since_647)]
const ATTACK: usize = 0;

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
    client.client_cheat(input);
}

#[cfg(before_742)]
#[cfg(since_647)]
fn button1(client: &mut HarnessClient, com: i32, slot: i32, obj: i32) {
    client.if_button(1, com, slot as u16, TypeId::from_int(obj));
}

/// An obj's iop4 (Rub) on the backpack's slot 0: the client puts iop4 on the slot's op 7.
fn rub(client: &mut HarnessClient, obj: u16) {
    client.if_button(7, BACKPACK, 0, TypeId::from_int(obj as i32));
}

/// An option of a multi-choice box: RESUME_PAUSEBUTTON.
fn resume(client: &mut HarnessClient, com: i32) {
    client.resume_pausebutton(com, 0xFFFF);
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

#[cfg(before_742)]
#[cfg(since_647)]
fn varp(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let id = engine
        .cache
        .varps
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no varp {name}"))
        .id;
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(id.g2())
        .as_int()
}

#[cfg(before_742)]
#[cfg(since_647)]
fn inv_total(engine: &Engine, client: &HarnessClient, obj: u16) -> u32 {
    let id = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    let inv = engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&id)
        .expect("inv");
    (0..inv.capacity)
        .filter_map(|slot| inv.get(slot as u16))
        .filter(|i| i.obj == TypeId::new(obj))
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
            text_of(&texts, com("questscroll_ver3:quest_title_ver3")),
            Some("'In Search of the Myreque' completed.")
        );
        assert_eq!(
            text_of(&texts, com("questscroll_ver3:quest_reward1_ver3")),
            Some("2 Quest Points")
        );
        assert_eq!(
            text_of(&texts, com("questscroll_ver3:quest_reward2_ver3")),
            Some("Attack & Defence XP")
        );
        assert_eq!(
            text_of(&texts, com("questscroll_ver3:quest_reward3_ver3")),
            Some("Strength & Hitpoint XP")
        );
        assert_eq!(
            text_of(&texts, com("questscroll_ver3:quest_reward4_ver3")),
            Some("Crafting XP")
        );
        assert_eq!(
            text_of(&texts, com("questscroll_ver3:quest_reward5_ver3")),
            Some("Quick Route to Mort'ton")
        );
        assert_eq!(
            text_of(&texts, com("questscroll_ver3:quest_reward6_ver3")),
            Some("")
        );
    });
}

#[cfg(before_742)]
#[cfg(since_647)]
#[test]
fn the_lamp_advances_the_stat_picked_on_2011_s_lamp() {
    runner::on_loaded_engine(|engine| {
        let lamp = engine
            .cache
            .objs
            .get_by_debugname("macro_genilamp")
            .unwrap()
            .id;
        let mut client = developer(engine, "lamprub");
        cheat(&mut client, "~give macro_genilamp 1");
        ticks(engine, &mut client, 2);
        assert_eq!(inv_total(engine, &client, lamp.g2()), 1, "the lamp given");

        button1(&mut client, BACKPACK, 0, lamp.int());
        ticks(engine, &mut client, 2);
        assert!(is_open(engine, &client, LAMP), "Rub opens 2011's lamp");
        assert_eq!(varp(engine, &client, "if1"), 0);

        // Herblore wants Druidic Ritual first, as 289's.
        button1(&mut client, com("interface_134:herblore"), 0xFFFF, 0xFFFF);
        let said = messages(&ticks(engine, &mut client, 1));
        assert!(
            said.contains(
                &"You need to complete the Druidic Ritual quest to choose that option.".to_string()
            ),
            "{said:?}"
        );
        assert_eq!(varp(engine, &client, "if1"), 0);

        // Each button its stat's value: the box clientscript 790 lights.
        for (button, value) in [(54, 25), (52, 22), (50, 23), (41, 19), (47, 18), (30, 1)] {
            button1(&mut client, com_id(LAMP, button), 0xFFFF, 0xFFFF);
            ticks(engine, &mut client, 1);
            assert_eq!(varp(engine, &client, "if1"), value, "button {button}");
        }

        // Confirm: Attack, at level 1, takes 10 xp (the engine's tenths).
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[ATTACK];
        button1(&mut client, com("interface_134:confirm"), 0xFFFF, 0xFFFF);
        let packets = ticks(engine, &mut client, 2);
        let texts = settexts(&packets);
        assert_eq!(
            engine.harness_player(client.pid).unwrap().stats.xp[ATTACK] - xp,
            100
        );
        assert_eq!(
            inv_total(engine, &client, lamp.g2()),
            0,
            "the lamp is used up"
        );
        assert_eq!(varp(engine, &client, "if1"), 0);
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
            text_of(&texts, com("messagescroll:messcroll5")),
            Some("Es lemanto meso pro eis prit ta Cinqo mond.")
        );
        assert_eq!(
            text_of(&texts, com("messagescroll:messcroll7")),
            Some("et Daconia arpos eto meriz ta priw!")
        );
        assert_eq!(text_of(&texts, com("messagescroll:messcroll15")), Some(""));
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
) -> ((u16, u16), Option<TypeId>, Vec<String>) {
    let obj = engine.cache.objs.get_by_debugname(name).unwrap().id;
    engine.harness_telejump(client.pid, HOME.0, 0, HOME.1);
    cheat(client, "~clearinv inv");
    ticks(engine, client, 2);
    cheat(client, &format!("~give {name} 1"));
    ticks(engine, client, 2);
    rub(client, obj.g2());
    let mut said = messages(&ticks(engine, client, 3));
    assert!(
        is_open(engine, client, multi),
        "{name}: the options; said {said:?}"
    );
    resume(client, com_id(multi, option));
    said.extend(messages(&ticks(engine, client, 8)));
    let id = engine.cache.invs.get_by_debugname("inv").unwrap().id;
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
        let seven = engine
            .cache
            .objs
            .get_by_debugname("ring_of_dueling_7")
            .unwrap()
            .id;
        // 530's map has no Mobilising Armies: its ring offers the Fist of Guthix third (multi4).
        #[cfg(before_647)]
        let (at, left, said) = rub_and_go(engine, &mut client, "ring_of_dueling_8", 232, 4);
        #[cfg(before_647)]
        assert!(
            near(at, (1692, 1696), (5599, 5603)),
            "Fist of Guthix: at {at:?}; said {said:?}"
        );
        #[cfg(since_647)]
        let (at, left, said) = rub_and_go(engine, &mut client, "ring_of_dueling_8", 234, 4);
        #[cfg(since_647)]
        assert!(
            near(at, (2412, 2415), (2847, 2849)),
            "Mobilising Armies: at {at:?}; said {said:?}"
        );
        assert_eq!(left, Some(seven));
        assert!(
            said.contains(&"Your Ring of Dueling has 7 uses left.".to_string()),
            "{said:?}"
        );

        let seven = engine
            .cache
            .objs
            .get_by_debugname("necklace_of_minigames_7")
            .unwrap()
            .id;
        let (at, left, said) = rub_and_go(engine, &mut client, "necklace_of_minigames_8", 230, 3);
        assert!(
            near(at, (2519, 2521), (3572, 3574)),
            "Barbarian Outpost: at {at:?}; said {said:?}"
        );
        assert_eq!(left, Some(seven));

        let three = engine
            .cache
            .objs
            .get_by_debugname("amulet_of_glory_3")
            .unwrap()
            .id;
        let (at, left, said) = rub_and_go(engine, &mut client, "amulet_of_glory_4", 234, 2);
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
#[cfg(since_647)]
fn sethides(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, bool)> {
    server::hides(packets)
}

#[cfg(since_647)]
fn hidden(hides: &[(i32, bool)], com: i32) -> Option<bool> {
    hides.iter().rev().find(|(c, _)| *c == com).map(|(_, h)| *h)
}

// 289's book on 2011's (959): its arrows are pause buttons, Back com28 and
// Next com29, and the book waits on its page for one.
#[cfg(since_647)]
#[test]
fn a_289_book_turns_its_pages_on_2011_s_book() {
    runner::on_loaded_engine(|engine| {
        const BOOK: u16 = 959;
        let mut client = developer(engine, "bookread");
        cheat(&mut client, "~book_test");
        let packets = ticks(engine, &mut client, 2);
        let (texts, hides) = (settexts(&packets), sethides(&packets));
        assert!(is_open(engine, &client, BOOK), "the book; sent {texts:?}");
        assert_eq!(
            text_of(&texts, com("book_small:book_title")),
            Some("Test Book")
        );
        assert_eq!(
            text_of(&texts, com("book_small:line1a")),
            Some("First page.")
        );
        assert_eq!(text_of(&texts, com("book_small:pagenum_left")), Some("1"));
        assert_eq!(
            hidden(&hides, com("book_small:left_arrow")),
            Some(true),
            "no Back on the first page"
        );
        assert_eq!(
            hidden(&hides, com("book_small:right_arrow")),
            Some(false),
            "Next"
        );
        assert_eq!(
            hidden(&hides, com("book_small:pic2")),
            Some(true),
            "no obj by line 2"
        );

        resume(&mut client, com("book_small:right_arrow"));
        let packets = ticks(engine, &mut client, 2);
        let (texts, hides) = (settexts(&packets), sethides(&packets));
        assert!(is_open(engine, &client, BOOK));
        assert_eq!(
            text_of(&texts, com("book_small:line1a")),
            Some("Second page.")
        );
        assert_eq!(
            text_of(&texts, com("book_small:line1b")),
            Some("With a dagger.")
        );
        assert_eq!(text_of(&texts, com("book_small:pagenum_left")), Some("3"));
        assert_eq!(
            hidden(&hides, com("book_small:pic2")),
            Some(false),
            "the dagger by line 2"
        );
        assert_eq!(
            hidden(&hides, com("book_small:right_arrow")),
            Some(true),
            "no Next on the last page"
        );

        resume(&mut client, com("book_small:left_arrow"));
        let texts = settexts(&ticks(engine, &mut client, 2));
        assert_eq!(
            text_of(&texts, com("book_small:line1a")),
            Some("First page.")
        );
    });
}

// 289's wine of zamorak at the Chaos Temple (2930, 3515): Take burns the
// thief, drains the combat stats and says "STOP STEALING MY WINE! GAH!".
#[test]
fn taking_the_temple_s_wine_burns_the_thief() {
    runner::on_loaded_engine(|engine| {
        let wine = engine
            .cache
            .objs
            .get_by_debugname("wine_of_zamorak")
            .unwrap()
            .id;
        let mut client = developer(engine, "winethief");
        engine.harness_add_obj(2930, 0, 3515, wine, 1);
        // West of it: east of it, past the wall, is a 2-wide loc (loc_15573).
        engine.harness_telejump(client.pid, 2929, 0, 3515);
        ticks(engine, &mut client, 2);
        // 530 has hitpoints, not 2011's life points.
        #[cfg(since_647)]
        let life = life::get(engine, client.pid);
        client.op_obj(3, 2930, 3515, wine);
        let said = messages(&ticks(engine, &mut client, 6));
        assert!(
            said.contains(&"STOP STEALING MY WINE! GAH!".to_string()),
            "{said:?}"
        );
        #[cfg(since_647)]
        assert!(life::get(engine, client.pid) < life, "burnt");
    });
}
