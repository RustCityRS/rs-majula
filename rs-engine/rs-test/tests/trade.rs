//! 289's trade on 2011's: trademain (335), tradeside (336), tradeconfirm (334).
//!
//! Two players: "Trade with" (op4) on the other sends 289's request, which
//! 2011's client takes as message type 100 from the requester; the other's
//! "Trade with" back opens both screens. 2011's client draws the offers and
//! the backpack as grids the server makes once the screens are open
//! (clientscripts 150, own, and 695, the other's) and gets the other's offer
//! as inv 90 with the "other" flag. Both accepting twice swaps the offers.
//! An IF_BUTTON{n} is `p4(com) p2_alt3(obj) p2_alt1(slot)`.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_test::names::com;
use rs_test::runner;
use rs_test::server;
#[cfg(since_647)]
use rs_util::base37::to_screen_name;
use rs_ver::TypeId;

const MAIN: u16 = 335;
#[cfg(since_647)]
const SIDE: u16 = 336;
const CONFIRM: u16 = 334;
/// 289's tradeoffer, 2011's inv 90.
const TRADEOFFER: u16 = 90;
/// "Trade modified": the player's own offer, the other's.
const OFFER_MODIFIED: u16 = 1042;
const OTHER_OFFER_MODIFIED: u16 = 1043;
/// Lumbridge, outside the castle.
const TILE: (u16, u16) = (3222, 3218);

/// What a client was sent: `(opcode, payload)`.
type Packets = Vec<(u16, Vec<u8>)>;
/// An UPDATE_INV_FULL: `(inv, other, [(obj, count)] of its filled slots)`.
type Inv = (u16, bool, Vec<(u16, i32)>);

fn obj(engine: &Engine, name: &str) -> TypeId {
    engine
        .cache
        .objs
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
        .id
}

/// Runs `n` ticks, gathering what each client was sent.
fn ticks(
    engine: &mut Engine,
    a: &mut HarnessClient,
    b: &mut HarnessClient,
    n: usize,
) -> (Packets, Packets) {
    let (mut pa, mut pb) = (Vec::new(), Vec::new());
    for _ in 0..n {
        engine.cycle();
        pa.extend(a.packets());
        pb.extend(b.packets());
    }
    (pa, pb)
}

/// The clientscripts RUNCLIENTSCRIPT ran (`... p4 script_id` last).
fn scripts(packets: &[(u16, Vec<u8>)]) -> Vec<i32> {
    server::scripts(packets)
        .into_iter()
        .map(|(s, _)| s)
        .collect()
}

/// Each UPDATE_INV_FULL as `(inv, other, [(obj, count)] of its filled slots)`.
fn invs(packets: &[(u16, Vec<u8>)]) -> Vec<Inv> {
    server::invs(packets)
        .into_iter()
        .map(|i| {
            let slots = i.slots.into_iter().flatten();
            (
                i.inv as u16,
                i.other,
                slots.map(|(o, c)| (o as u16, c)).collect(),
            )
        })
        .collect()
}

/// Each varc set, `(varc, value)`.
#[cfg(since_647)]
fn varcs(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, i32)> {
    server::varcs(packets)
}

/// Each MESSAGE_GAME as `(type, name, text)`.
fn messages(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, Option<String>, String)> {
    server::messages(packets)
}

fn count(engine: &Engine, pid: u16, inv: u16, obj: u16) -> i32 {
    let inv = engine
        .harness_player(pid)
        .unwrap()
        .invs
        .get(&TypeId::new(inv))
        .unwrap();
    (0..inv.capacity as u16)
        .filter_map(|s| inv.get(s))
        .filter(|i| i.obj == TypeId::new(obj))
        .map(|i| i.num as i32)
        .sum()
}

fn give(engine: &mut Engine, pid: u16, slot: u16, obj: u16, number: u32) {
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    engine
        .harness_player_mut(pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap()
        .set(slot, TypeId::new(obj), number);
}

fn varp(engine: &Engine, pid: u16, varp: u16) -> i32 {
    engine.harness_player(pid).unwrap().vars.get(varp).as_int()
}

/// The name `displayname` gives a player logged in as `username` who has
/// claimed no display name: the login name as 289 shows it, as the client
/// draws them.
#[cfg(since_647)]
fn display_name(username: &str) -> String {
    to_screen_name(username)
}

fn trade_with(client: &mut HarnessClient, other: u16) {
    client.op_player(4, other);
}

fn button(client: &mut HarnessClient, op: u8, com: i32, obj: u16, slot: u16) {
    let op = if matches!(op, 1..=5 | 9) { op } else { 10 };
    client.if_button(op, com, slot, TypeId::new(obj));
}

/// Two players side by side.
fn traders(engine: &mut Engine) -> (HarnessClient, HarnessClient) {
    engine.harness_reset();
    let a = engine.harness_login_returning("trader_a").expect("login");
    let b = engine.harness_login_returning("trader_b").expect("login");
    let (x, z) = TILE;
    engine.harness_telejump(a.pid, x, 0, z);
    engine.harness_telejump(b.pid, x + 1, 0, z);
    (a, b)
}

/// A asks, B answers: both screens open.
fn open_trade(
    engine: &mut Engine,
    a: &mut HarnessClient,
    b: &mut HarnessClient,
) -> (Packets, Packets) {
    ticks(engine, a, b, 2);
    trade_with(a, b.pid);
    ticks(engine, a, b, 3);
    trade_with(b, a.pid);
    ticks(engine, a, b, 3)
}

/// The request names the requester as the other's client draws them: by the
/// display name they claimed (2011's client finds the player to trade back
/// by that name), not their login's.
#[cfg(since_647)]
#[test]
fn trade_with_sends_2011s_request() {
    runner::on_loaded_engine(|engine| {
        let (mut a, mut b) = traders(engine);
        engine.harness_player_mut(a.pid).unwrap().display_name = Some("Tradesman".into());
        ticks(engine, &mut a, &mut b, 2);
        trade_with(&mut a, b.pid);
        let (pa, pb) = ticks(engine, &mut a, &mut b, 3);
        let name = "Tradesman".to_string();
        assert!(
            messages(&pa)
                .iter()
                .any(|m| m.2 == "Sending trade offer..."),
            "the requester's chat; sent {:?}",
            messages(&pa)
        );
        assert!(
            messages(&pb).contains(&(100, Some(name), "wishes to trade with you.".into())),
            "a type-100 request from the requester; sent {:?}",
            messages(&pb)
        );
        assert!(
            !engine
                .harness_player(b.pid)
                .unwrap()
                .is_interface_open(MAIN)
        );
    });
}

#[cfg(since_647)]
#[test]
fn trading_back_opens_both_screens_with_their_grids() {
    runner::on_loaded_engine(|engine| {
        let (mut a, mut b) = traders(engine);
        let (pa, pb) = open_trade(engine, &mut a, &mut b);
        for (pid, packets, who) in [(a.pid, &pa, "requester"), (b.pid, &pb, "accepter")] {
            let player = engine.harness_player(pid).unwrap();
            assert!(player.is_interface_open(MAIN), "{who}: 2011's trade screen");
            assert!(
                player.is_interface_open(SIDE),
                "{who}: the backpack beside it"
            );
            let ran = scripts(packets);
            assert!(ran.contains(&150), "{who}: own grids (150); ran {ran:?}");
            assert!(
                ran.contains(&695),
                "{who}: the other's grid (695); ran {ran:?}"
            );
            let sent = invs(packets);
            assert!(
                sent.iter().any(|(i, o, _)| *i == TRADEOFFER && !o),
                "{who}: own offer, inv 90; sent {sent:?}"
            );
            assert!(
                sent.iter().any(|(i, o, _)| *i == TRADEOFFER && *o),
                "{who}: the other's offer, inv 90 with the other flag; sent {sent:?}"
            );
        }
        let names: Vec<String> = server::varcstrs(&pb).into_iter().map(|(_, v)| v).collect();
        assert!(
            names.contains(&display_name("trader_a")),
            "the trader's name (varcstr 203); sent {names:?}"
        );
    });
}

#[cfg(since_647)]
#[test]
fn an_offer_reaches_the_other_as_their_other_inv() {
    runner::on_loaded_engine(|engine| {
        let coins = obj(engine, "coins");
        let (mut a, mut b) = traders(engine);
        give(engine, a.pid, 0, coins.g2(), 1000);
        open_trade(engine, &mut a, &mut b);
        // Offer-5, then Offer-All.
        button(&mut a, 2, com("tradeside:side_layer"), coins.g2(), 0);
        ticks(engine, &mut a, &mut b, 1);
        assert_eq!(count(engine, a.pid, TRADEOFFER, coins.g2()), 5, "Offer-5");
        button(&mut a, 4, com("tradeside:side_layer"), coins.g2(), 0);
        let (pa, pb) = ticks(engine, &mut a, &mut b, 1);
        assert_eq!(
            count(engine, a.pid, TRADEOFFER, coins.g2()),
            1000,
            "Offer-All"
        );
        assert!(
            invs(&pb).contains(&(TRADEOFFER, true, vec![(coins.g2(), 1000)])),
            "the other sees it as the other's offer; sent {:?}",
            invs(&pb)
        );
        let cost = engine.cache.objs.get_by_id(coins).unwrap().cost;
        let worth = 1000 * cost;
        assert!(
            varcs(&pa).contains(&(729, worth)),
            "own offer's worth; sent {:?}",
            varcs(&pa)
        );
        assert!(
            varcs(&pb).contains(&(697, worth)),
            "the other's offer's worth; sent {:?}",
            varcs(&pb)
        );
    });
}

#[test]
fn both_accepting_twice_swaps_the_offers() {
    runner::on_loaded_engine(|engine| {
        let coins = obj(engine, "coins");
        let axe = obj(engine, "bronze_axe");
        let (mut a, mut b) = traders(engine);
        give(engine, a.pid, 0, coins.g2(), 1000);
        give(engine, b.pid, 0, axe.g2(), 1);
        open_trade(engine, &mut a, &mut b);
        button(&mut a, 4, com("tradeside:side_layer"), coins.g2(), 0);
        button(&mut b, 1, com("tradeside:side_layer"), axe.g2(), 0);
        ticks(engine, &mut a, &mut b, 1);
        button(&mut a, 1, com("trademain:accept_button"), 0xFFFF, 0xFFFF);
        ticks(engine, &mut a, &mut b, 1);
        assert!(
            !engine
                .harness_player(a.pid)
                .unwrap()
                .is_interface_open(CONFIRM)
        );
        button(&mut b, 1, com("trademain:accept_button"), 0xFFFF, 0xFFFF);
        let (pa, _) = ticks(engine, &mut a, &mut b, 1);
        for pid in [a.pid, b.pid] {
            let player = engine.harness_player(pid).unwrap();
            assert!(player.is_interface_open(CONFIRM), "2011's confirm screen");
            assert!(!player.is_interface_open(MAIN));
        }
        let sent = invs(&pa);
        assert!(
            sent.contains(&(TRADEOFFER, false, vec![(coins.g2(), 1000)]))
                && sent.contains(&(TRADEOFFER, true, vec![(axe.g2(), 1)])),
            "both offers again, which 2011's confirm screen lists itself; sent {sent:?}"
        );
        button(&mut a, 1, com("tradeconfirm:trade2accept"), 0xFFFF, 0xFFFF);
        button(&mut b, 1, com("tradeconfirm:trade2accept"), 0xFFFF, 0xFFFF);
        let (pa, pb) = ticks(engine, &mut a, &mut b, 1);
        assert_eq!(
            count(engine, b.pid, 93, coins.g2()),
            1000,
            "the coins went across"
        );
        assert_eq!(count(engine, a.pid, 93, axe.g2()), 1, "and the axe back");
        assert_eq!(count(engine, a.pid, 93, coins.g2()), 0);
        assert_eq!(count(engine, b.pid, 93, axe.g2()), 0);
        assert!(messages(&pa).iter().any(|m| m.2 == "Accepted trade."));
        assert!(messages(&pb).iter().any(|m| m.2 == "Accepted trade."));
        assert!(
            !engine
                .harness_player(a.pid)
                .unwrap()
                .is_interface_open(CONFIRM)
        );
    });
}

#[test]
fn declining_gives_the_offer_back() {
    runner::on_loaded_engine(|engine| {
        let coins = obj(engine, "coins");
        let (mut a, mut b) = traders(engine);
        give(engine, a.pid, 0, coins.g2(), 1000);
        open_trade(engine, &mut a, &mut b);
        button(&mut a, 4, com("tradeside:side_layer"), coins.g2(), 0);
        ticks(engine, &mut a, &mut b, 1);
        button(&mut a, 1, com("trademain:decline_button"), 0xFFFF, 0xFFFF);
        let (_, pb) = ticks(engine, &mut a, &mut b, 1);
        assert_eq!(
            count(engine, a.pid, 93, coins.g2()),
            1000,
            "back in the backpack"
        );
        assert_eq!(count(engine, a.pid, TRADEOFFER, coins.g2()), 0);
        for pid in [a.pid, b.pid] {
            assert!(!engine.harness_player(pid).unwrap().is_interface_open(MAIN));
        }
        assert!(
            messages(&pb)
                .iter()
                .any(|m| m.2 == "Other player declined trade."),
            "sent {:?}",
            messages(&pb)
        );
    });
}

#[test]
fn a_removal_marks_the_trade_modified() {
    runner::on_loaded_engine(|engine| {
        let coins = obj(engine, "coins");
        let (mut a, mut b) = traders(engine);
        give(engine, a.pid, 0, coins.g2(), 1000);
        open_trade(engine, &mut a, &mut b);
        assert_eq!(varp(engine, a.pid, OFFER_MODIFIED), 0);
        button(&mut a, 4, com("tradeside:side_layer"), coins.g2(), 0);
        ticks(engine, &mut a, &mut b, 1);
        button(
            &mut a,
            3,
            com("trademain:your_offer_layer_flashy"),
            coins.g2(),
            0,
        );
        let (_, pb) = ticks(engine, &mut a, &mut b, 1);
        assert_eq!(
            count(engine, a.pid, TRADEOFFER, coins.g2()),
            990,
            "Remove-10"
        );
        assert_eq!(varp(engine, a.pid, OFFER_MODIFIED), 1, "own offer modified");
        assert_eq!(
            varp(engine, b.pid, OTHER_OFFER_MODIFIED),
            1,
            "the other's, for the other"
        );
        assert!(
            scripts(&pb).contains(&143),
            "the slot flashes on the other's screen"
        );
        // Examine on the other's offer (op9).
        button(
            &mut b,
            9,
            com("trademain:other_offer_layer_flashy"),
            coins.g2(),
            0,
        );
        let (_, pb) = ticks(engine, &mut a, &mut b, 1);
        let desc = engine.cache.objs.get_by_id(coins).unwrap().desc.clone();
        assert!(
            messages(&pb)
                .iter()
                .any(|m| Some(m.2.as_str()) == desc.as_deref()),
            "its examine; sent {:?}",
            messages(&pb)
        );
    });
}

/// Each IF_SETHIDE as `(com, hidden)`: `p4_alt3 com, p1_alt2 hidden`.
fn hides(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, bool)> {
    server::hides(packets)
}

/// The confirm screen says "Trade modified" (com55) to the one whose trader
/// took an item back, and not to the trader.
#[test]
fn the_confirm_screen_says_the_trade_was_modified() {
    runner::on_loaded_engine(|engine| {
        let coins = obj(engine, "coins");
        let (mut a, mut b) = traders(engine);
        give(engine, a.pid, 0, coins.g2(), 1000);
        open_trade(engine, &mut a, &mut b);
        button(&mut a, 4, com("tradeside:side_layer"), coins.g2(), 0);
        ticks(engine, &mut a, &mut b, 1);
        button(
            &mut a,
            1,
            com("trademain:your_offer_layer_flashy"),
            coins.g2(),
            0,
        );
        ticks(engine, &mut a, &mut b, 1);
        button(&mut a, 1, com("trademain:accept_button"), 0xFFFF, 0xFFFF);
        button(&mut b, 1, com("trademain:accept_button"), 0xFFFF, 0xFFFF);
        let (pa, pb) = ticks(engine, &mut a, &mut b, 2);
        assert!(
            engine
                .harness_player(b.pid)
                .unwrap()
                .is_interface_open(CONFIRM)
        );
        let modified = com("tradeconfirm:trade_removed_warning_right");
        assert!(
            hides(&pb).contains(&(modified, false)),
            "shown to the other; sent {:?}",
            hides(&pb)
        );
        assert!(
            !hides(&pa).contains(&(modified, false)),
            "not to the one who took it back"
        );
    });
}

/// "Trade with" on a player with a screen open says 2011's line (2011 wiki,
/// Trading, 2011-05-05: "The other player is busy at the moment"), not 289's
/// "<name> is busy at the moment.".
#[test]
fn trade_with_a_busy_player_says_2011s_line() {
    runner::on_loaded_engine(|engine| {
        let (mut a, mut b) = traders(engine);
        ticks(engine, &mut a, &mut b, 2);
        engine.harness_open_main(b.pid, MAIN);
        ticks(engine, &mut a, &mut b, 1);
        trade_with(&mut a, b.pid);
        let (pa, _) = ticks(engine, &mut a, &mut b, 3);
        let said: Vec<String> = messages(&pa).into_iter().map(|m| m.2).collect();
        assert!(
            said.contains(&"The other player is busy at the moment".to_string()),
            "the requester's chat: {said:?}"
        );
    });
}
