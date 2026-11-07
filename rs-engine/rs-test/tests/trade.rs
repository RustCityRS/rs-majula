//! 289's trade on 2011's: trademain (335), tradeside (336), tradeconfirm (334).
//!
//! Two players: "Trade with" (op4) on the other sends 289's request, which
//! 2011's client takes as message type 100 from the requester; the other's
//! "Trade with" back opens both screens. 2011's client draws the offers and
//! the backpack as grids the server makes once the screens are open
//! (clientscripts 150, own, and 695, the other's) and gets the other's offer
//! as inv 90 with the "other" flag. Both accepting twice swaps the offers.
//! OPPLAYER4 is `p2_alt2(pid) p1(ctrl)`; an IF_BUTTON{n} is
//! `p4(com) p2_alt3(obj) p2_alt1(slot)`.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

const MAIN: u16 = 335;
const SIDE: u16 = 336;
const CONFIRM: u16 = 334;
const OFFER: i32 = (MAIN as i32) << 16 | 31;
const OTHER_OFFER: i32 = (MAIN as i32) << 16 | 34;
const BACKPACK: i32 = (SIDE as i32) << 16;
const ACCEPT: i32 = (MAIN as i32) << 16 | 16;
const DECLINE: i32 = (MAIN as i32) << 16 | 18;
const CONFIRM_ACCEPT: i32 = (CONFIRM as i32) << 16 | 21;
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

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
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

fn of(packets: &[(u16, Vec<u8>)], prot: ServerProt) -> impl Iterator<Item = &Vec<u8>> {
    let opcode = prot as u16;
    packets
        .iter()
        .filter(move |(op, _)| *op == opcode)
        .map(|(_, p)| p)
}

/// The clientscripts RUNCLIENTSCRIPT ran (`... p4 script_id` last).
fn scripts(packets: &[(u16, Vec<u8>)]) -> Vec<i32> {
    of(packets, ServerProt::RunClientScript)
        .map(|p| {
            let n = p.len();
            i32::from_be_bytes([p[n - 4], p[n - 3], p[n - 2], p[n - 1]])
        })
        .collect()
}

/// Each UPDATE_INV_FULL as `(inv, other, [(obj, count)] of its filled slots)`:
/// `g2 inv; g1 flags; g2 size;` per slot `g1 count (255: ig4); g2_alt2 obj+1`.
fn invs(packets: &[(u16, Vec<u8>)]) -> Vec<Inv> {
    of(packets, ServerProt::UpdateInvFull)
        .map(|p| {
            let inv = u16::from_be_bytes([p[0], p[1]]);
            let other = p[2] & 1 == 1;
            let size = u16::from_be_bytes([p[3], p[4]]);
            let mut pos = 5;
            let mut slots = Vec::new();
            for _ in 0..size {
                let mut count = p[pos] as i32;
                pos += 1;
                if count == 255 {
                    count = i32::from_le_bytes([p[pos], p[pos + 1], p[pos + 2], p[pos + 3]]);
                    pos += 4;
                }
                let id = ((p[pos] as u16) << 8 | p[pos + 1].wrapping_sub(128) as u16) as i32 - 1;
                pos += 2;
                if id >= 0 {
                    slots.push((id as u16, count));
                }
            }
            (inv, other, slots)
        })
        .collect()
}

/// Each varc set, `(varc, value)`: VARC_SMALL `p1_alt2 value, p2_alt1 varc`;
/// VARC_LARGE `p4_alt1 value, p2_alt3 varc`.
fn varcs(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, i32)> {
    let mut out: Vec<(u16, i32)> = of(packets, ServerProt::VarcSmall)
        .map(|p| {
            (
                u16::from_le_bytes([p[1], p[2]]),
                0u8.wrapping_sub(p[0]) as i32,
            )
        })
        .collect();
    out.extend(of(packets, ServerProt::VarcLarge).map(|p| {
        (
            u16::from_le_bytes([p[4].wrapping_sub(128), p[5]]),
            i32::from_le_bytes([p[0], p[1], p[2], p[3]]),
        )
    }));
    out
}

/// Each MESSAGE_GAME as `(type, name, text)`: `gsmart type; g4; g1 mask;`
/// a name when mask bit 0 is set, then the text.
fn messages(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, Option<String>, String)> {
    of(packets, ServerProt::MessageGame)
        .map(|p| {
            let kind = p[0] as i32;
            let mut pos = 5;
            let mask = p[pos];
            pos += 1;
            let mut string = || {
                let end = pos + p[pos..].iter().position(|&b| b == 0).unwrap();
                let s = String::from_utf8_lossy(&p[pos..end]).into_owned();
                pos = end + 1;
                s
            };
            let name = (mask & 1 == 1).then(&mut string);
            (kind, name, string())
        })
        .collect()
}

fn count(engine: &Engine, pid: u16, inv: u16, obj: u16) -> i32 {
    let inv = engine.harness_player(pid).unwrap().invs.get(&inv).unwrap();
    (0..inv.capacity as u16)
        .filter_map(|s| inv.get(s))
        .filter(|i| i.obj == obj)
        .map(|i| i.num as i32)
        .sum()
}

fn give(engine: &mut Engine, pid: u16, slot: u16, obj: u16, number: u32) {
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine
        .harness_player_mut(pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap()
        .set(slot, obj, number);
}

fn varp(engine: &Engine, pid: u16, varp: u16) -> i32 {
    engine.harness_player(pid).unwrap().vars.get(varp).as_int()
}

/// The name `displayname` gives a player logged in as `username` who has
/// claimed no display name: the login name as 289 shows it, as the client
/// draws them.
fn display_name(username: &str) -> String {
    rs_util::base37::to_screen_name(username)
}

fn trade_with(client: &mut HarnessClient, other: u16) {
    let mut p = ClientPacket::new();
    p.p2_alt2(other as i32).p1(0);
    client.send(ClientProt::OpPlayer4, &p.finish());
}

fn button(client: &mut HarnessClient, op: u8, com: i32, obj: u16, slot: u16) {
    let prot = match op {
        1 => ClientProt::IfButton1,
        2 => ClientProt::IfButton2,
        3 => ClientProt::IfButton3,
        4 => ClientProt::IfButton4,
        5 => ClientProt::IfButton5,
        9 => ClientProt::IfButton9,
        _ => ClientProt::IfButton10,
    };
    let mut p = ClientPacket::new();
    p.p4(com).p2_alt3(obj as i32).p2_alt1(slot as i32);
    client.send(prot, &p.finish());
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
        let names: Vec<String> = of(&pb, ServerProt::VarcStrSmall)
            .map(|p| String::from_utf8_lossy(&p[..p.len() - 3]).into_owned())
            .collect();
        assert!(
            names.contains(&display_name("trader_a")),
            "the trader's name (varcstr 203); sent {names:?}"
        );
    });
}

#[test]
fn an_offer_reaches_the_other_as_their_other_inv() {
    runner::on_loaded_engine(|engine| {
        let coins = obj(engine, "coins");
        let (mut a, mut b) = traders(engine);
        give(engine, a.pid, 0, coins, 1000);
        open_trade(engine, &mut a, &mut b);
        // Offer-5, then Offer-All.
        button(&mut a, 2, BACKPACK, coins, 0);
        ticks(engine, &mut a, &mut b, 1);
        assert_eq!(count(engine, a.pid, TRADEOFFER, coins), 5, "Offer-5");
        button(&mut a, 4, BACKPACK, coins, 0);
        let (pa, pb) = ticks(engine, &mut a, &mut b, 1);
        assert_eq!(count(engine, a.pid, TRADEOFFER, coins), 1000, "Offer-All");
        assert!(
            invs(&pb).contains(&(TRADEOFFER, true, vec![(coins, 1000)])),
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
        let axe = obj(engine, "bronze_hatchet");
        let (mut a, mut b) = traders(engine);
        give(engine, a.pid, 0, coins, 1000);
        give(engine, b.pid, 0, axe, 1);
        open_trade(engine, &mut a, &mut b);
        button(&mut a, 4, BACKPACK, coins, 0);
        button(&mut b, 1, BACKPACK, axe, 0);
        ticks(engine, &mut a, &mut b, 1);
        button(&mut a, 1, ACCEPT, 0xFFFF, 0xFFFF);
        ticks(engine, &mut a, &mut b, 1);
        assert!(
            !engine
                .harness_player(a.pid)
                .unwrap()
                .is_interface_open(CONFIRM)
        );
        button(&mut b, 1, ACCEPT, 0xFFFF, 0xFFFF);
        let (pa, _) = ticks(engine, &mut a, &mut b, 1);
        for pid in [a.pid, b.pid] {
            let player = engine.harness_player(pid).unwrap();
            assert!(player.is_interface_open(CONFIRM), "2011's confirm screen");
            assert!(!player.is_interface_open(MAIN));
        }
        let sent = invs(&pa);
        assert!(
            sent.contains(&(TRADEOFFER, false, vec![(coins, 1000)]))
                && sent.contains(&(TRADEOFFER, true, vec![(axe, 1)])),
            "both offers again, which 2011's confirm screen lists itself; sent {sent:?}"
        );
        button(&mut a, 1, CONFIRM_ACCEPT, 0xFFFF, 0xFFFF);
        button(&mut b, 1, CONFIRM_ACCEPT, 0xFFFF, 0xFFFF);
        let (pa, pb) = ticks(engine, &mut a, &mut b, 1);
        assert_eq!(
            count(engine, b.pid, 93, coins),
            1000,
            "the coins went across"
        );
        assert_eq!(count(engine, a.pid, 93, axe), 1, "and the axe back");
        assert_eq!(count(engine, a.pid, 93, coins), 0);
        assert_eq!(count(engine, b.pid, 93, axe), 0);
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
        give(engine, a.pid, 0, coins, 1000);
        open_trade(engine, &mut a, &mut b);
        button(&mut a, 4, BACKPACK, coins, 0);
        ticks(engine, &mut a, &mut b, 1);
        button(&mut a, 1, DECLINE, 0xFFFF, 0xFFFF);
        let (_, pb) = ticks(engine, &mut a, &mut b, 1);
        assert_eq!(
            count(engine, a.pid, 93, coins),
            1000,
            "back in the backpack"
        );
        assert_eq!(count(engine, a.pid, TRADEOFFER, coins), 0);
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
        give(engine, a.pid, 0, coins, 1000);
        open_trade(engine, &mut a, &mut b);
        assert_eq!(varp(engine, a.pid, OFFER_MODIFIED), 0);
        button(&mut a, 4, BACKPACK, coins, 0);
        ticks(engine, &mut a, &mut b, 1);
        button(&mut a, 3, OFFER, coins, 0);
        let (_, pb) = ticks(engine, &mut a, &mut b, 1);
        assert_eq!(count(engine, a.pid, TRADEOFFER, coins), 990, "Remove-10");
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
        button(&mut b, 9, OTHER_OFFER, coins, 0);
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
    of(packets, ServerProt::IfSetHide)
        .map(|p| {
            let com = i32::from_be_bytes([p[1], p[0], p[3], p[2]]);
            (com, 0u8.wrapping_sub(p[4]) == 1)
        })
        .collect()
}

/// The confirm screen says "Trade modified" (com55) to the one whose trader
/// took an item back, and not to the trader.
#[test]
fn the_confirm_screen_says_the_trade_was_modified() {
    runner::on_loaded_engine(|engine| {
        let coins = obj(engine, "coins");
        let (mut a, mut b) = traders(engine);
        give(engine, a.pid, 0, coins, 1000);
        open_trade(engine, &mut a, &mut b);
        button(&mut a, 4, BACKPACK, coins, 0);
        ticks(engine, &mut a, &mut b, 1);
        button(&mut a, 1, OFFER, coins, 0);
        ticks(engine, &mut a, &mut b, 1);
        button(&mut a, 1, ACCEPT, 0xFFFF, 0xFFFF);
        button(&mut b, 1, ACCEPT, 0xFFFF, 0xFFFF);
        let (pa, pb) = ticks(engine, &mut a, &mut b, 2);
        assert!(
            engine
                .harness_player(b.pid)
                .unwrap()
                .is_interface_open(CONFIRM)
        );
        let modified = (CONFIRM as i32) << 16 | 55;
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
