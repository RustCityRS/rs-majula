//! 289's bank on 2011's: bank_v2_main (762) and bank_v2_side (763).
//!
//! 289's bank opens on 2011's interfaces: the bank (inv 95, 289's) is
//! bank_v2_main's com93 and the backpack beside it bank_v2_side's com0. 2011's
//! client sends an item's ops only for slots the server enables (IF_SETEVENTS)
//! and lays the bank out itself (clientscript 1465). 289's withdraw ops 1-5
//! (1, 5, 10, All, X) are 2011's 1, 2, 3, 6 and 5; 2011 adds Withdraw-<last X>
//! (op4, varp 1249) and All but one (op7), and asks for X with clientscript 108.
//! A booth's Use-quickly opens it; 2011's booths are reached from two sides
//! (their forceapproach 10), which the server now keeps as the client does.
//! A backpack or bank op is IF_BUTTON{n}: `p4(com) p2_alt3(obj) p2_alt1(slot)`;
//! RESUME_P_COUNTDIALOG is `p4(number)`.

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

const BANK: u16 = 762;
const SIDE: u16 = 763;
const BANK_INV: i32 = (BANK as i32) << 16 | 93;
const SIDE_INV: i32 = (SIDE as i32) << 16;
const NOTE_MODE: i32 = (BANK as i32) << 16 | 19;
/// Lumbridge castle's top-floor bank: a 2011 booth (36786, forceapproach 10).
const BOOTH: (u16, u8, u16, u16) = (36786, 2, 3208, 3221);
/// 289's bankcert (2011's note mode) and 2011's last X.
const BANKCERT: u16 = 115;
const LAST_AMOUNT: u16 = 1249;

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<(u16, Vec<u8>)> {
    let mut packets = Vec::new();
    for _ in 0..n {
        engine.cycle();
        packets.extend(client.packets());
    }
    packets
}

fn of(packets: &[(u16, Vec<u8>)], prot: ServerProt) -> impl Iterator<Item = &Vec<u8>> {
    let opcode = prot as u16;
    packets
        .iter()
        .filter(move |(op, _)| *op == opcode)
        .map(|(_, p)| p)
}

/// Each IF_SETEVENTS: `p2_alt2(from) p4_alt3(events) p4_alt3(com) p2_alt3(to)`.
fn events(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, u16, u16, i32)> {
    of(packets, ServerProt::IfSetEvents)
        .map(|p| {
            let from = u16::from_be_bytes([p[0], p[1].wrapping_sub(128)]);
            let ops = i32::from_be_bytes([p[3], p[2], p[5], p[4]]);
            let com = i32::from_be_bytes([p[7], p[6], p[9], p[8]]);
            let to = u16::from_le_bytes([p[10].wrapping_sub(128), p[11]]);
            (com, from, to, ops)
        })
        .collect()
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

fn count(engine: &Engine, pid: u16, inv: &str, obj: u16) -> i32 {
    let id = *engine.cache.invs.debugnames.get(inv).unwrap();
    let inv = engine.harness_player(pid).unwrap().invs.get(&id).unwrap();
    (0..inv.capacity as u16)
        .filter_map(|s| inv.get(s))
        .filter(|i| i.obj == obj)
        .map(|i| i.num as i32)
        .sum()
}

/// The backpack slot holding `obj`.
fn slot_of(engine: &Engine, pid: u16, obj: u16) -> u16 {
    let id = *engine.cache.invs.debugnames.get("inv").unwrap();
    let inv = engine.harness_player(pid).unwrap().invs.get(&id).unwrap();
    (0..inv.capacity as u16)
        .find(|&s| inv.get(s).is_some_and(|i| i.obj == obj))
        .expect("in the backpack")
}

fn button(client: &mut HarnessClient, op: u8, com: i32, obj: u16, slot: u16) {
    let prot = match op {
        1 => ClientProt::IfButton1,
        2 => ClientProt::IfButton2,
        3 => ClientProt::IfButton3,
        4 => ClientProt::IfButton4,
        5 => ClientProt::IfButton5,
        6 => ClientProt::IfButton6,
        7 => ClientProt::IfButton7,
        _ => ClientProt::IfButton10,
    };
    let mut p = ClientPacket::new();
    p.p4(com).p2_alt3(obj as i32).p2_alt1(slot as i32);
    client.send(prot, &p.finish());
}

/// A logged-in player.
fn banker(engine: &mut Engine, name: &str) -> HarnessClient {
    engine.harness_reset();
    let client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    client
}

/// Puts `items` in the open bank (which opening it made), one to a slot.
fn fill_bank(engine: &mut Engine, client: &mut HarnessClient, items: &[(&str, i32)]) {
    let objs: Vec<(u16, i32)> = items.iter().map(|&(n, c)| (obj(engine, n), c)).collect();
    let id = *engine.cache.invs.debugnames.get("bank").unwrap();
    let player = engine.harness_player_mut(client.pid).unwrap();
    let inv = player
        .invs
        .get_mut(&id)
        .expect("the bank, made when it opened");
    for (slot, (o, c)) in objs.into_iter().enumerate() {
        inv.set(slot as u16, o, c as u32);
    }
    ticks(engine, client, 1);
}

/// Uses the booth's `op` from each of its sides in turn until the bank opens.
fn use_booth(engine: &mut Engine, client: &mut HarnessClient, op: u8) -> Vec<(u16, Vec<u8>)> {
    let (loc, y, x, z) = BOOTH;
    let mut packets = Vec::new();
    for (dx, dz) in [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
        engine.harness_telejump(
            client.pid,
            (x as i32 + dx) as u16,
            y,
            (z as i32 + dz) as u16,
        );
        ticks(engine, client, 2);
        let mut p = ClientPacket::new();
        if op == 2 {
            p.p2_alt1(loc as i32)
                .p1_alt2(0)
                .p2_alt2(z as i32)
                .p2_alt3(x as i32);
            client.send(ClientProt::OpLoc2, &p.finish());
        } else {
            p.p2_alt1(z as i32)
                .p2_alt1(loc as i32)
                .p1(0)
                .p2_alt2(x as i32);
            client.send(ClientProt::OpLoc1, &p.finish());
        }
        packets = ticks(engine, client, 4);
        let player = engine.harness_player(client.pid).unwrap();
        if player.is_interface_open(BANK) || player.is_interface_open(241) {
            break;
        }
    }
    packets
}

#[test]
fn a_booth_s_use_quickly_opens_2011s_bank() {
    runner::on_loaded_engine(|engine| {
        let mut client = banker(engine, "booth");
        let packets = use_booth(engine, &mut client, 2);
        fill_bank(engine, &mut client, &[("coins", 1000)]);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(player.is_interface_open(BANK), "2011's bank");
        assert!(player.is_interface_open(SIDE), "and the backpack beside it");
        let sent: Vec<u16> = of(&packets, ServerProt::UpdateInvFull)
            .map(|p| u16::from_be_bytes([p[0], p[1]]))
            .collect();
        assert!(sent.contains(&95), "the bank's inventory; sent {sent:?}");
        let ev = events(&packets);
        assert!(
            ev.iter()
                .any(|&(c, f, t, _)| c == BANK_INV && f == 0 && t == 515),
            "the bank's slots enabled; sent {ev:?}"
        );
        assert!(
            ev.iter()
                .any(|&(c, f, t, _)| c == SIDE_INV && f == 0 && t == 27),
            "the backpack's slots enabled; sent {ev:?}"
        );
        assert!(
            scripts(&packets).contains(&1465),
            "2011's bank layout script"
        );
    });
}

#[test]
fn withdraw_and_deposit_take_289s_amounts_on_2011s_ops() {
    runner::on_loaded_engine(|engine| {
        let coins = obj(engine, "coins");
        let mut client = banker(engine, "teller");
        use_booth(engine, &mut client, 2);
        fill_bank(engine, &mut client, &[("coins", 1000)]);
        let pid = client.pid;
        // Withdraw-1, -5, -10.
        for (op, n) in [(1, 1), (2, 5), (3, 10)] {
            button(&mut client, op, BANK_INV, coins, 0);
            ticks(engine, &mut client, 1);
            assert_eq!(
                count(engine, pid, "inv", coins),
                [0, 1, 6, 16][op as usize],
                "Withdraw-{n}"
            );
        }
        // Deposit-All of the backpack's coins.
        let slot = slot_of(engine, pid, coins);
        button(&mut client, 6, SIDE_INV, coins, slot);
        ticks(engine, &mut client, 1);
        assert_eq!(count(engine, pid, "inv", coins), 0, "Deposit-All");
        assert_eq!(count(engine, pid, "bank", coins), 1000);
        // 2011's Withdraw-All but one, then Withdraw-All.
        button(&mut client, 7, BANK_INV, coins, 0);
        ticks(engine, &mut client, 1);
        assert_eq!(count(engine, pid, "inv", coins), 999, "All but one");
        button(&mut client, 6, BANK_INV, coins, 0);
        ticks(engine, &mut client, 1);
        assert_eq!(count(engine, pid, "inv", coins), 1000, "All");
    });
}

#[test]
fn withdraw_x_asks_and_withdraw_star_takes_the_same_again() {
    runner::on_loaded_engine(|engine| {
        let coins = obj(engine, "coins");
        let mut client = banker(engine, "xteller");
        use_booth(engine, &mut client, 2);
        fill_bank(engine, &mut client, &[("coins", 1000)]);
        button(&mut client, 5, BANK_INV, coins, 0);
        let asked = ticks(engine, &mut client, 1);
        assert!(
            scripts(&asked).contains(&108),
            "2011 asks how many (clientscript 108)"
        );
        let mut p = ClientPacket::new();
        p.p4(7);
        client.send(ClientProt::ResumePCountDialog, &p.finish());
        ticks(engine, &mut client, 2);
        assert_eq!(count(engine, client.pid, "inv", coins), 7, "Withdraw-X: 7");
        let last = engine
            .harness_player(client.pid)
            .unwrap()
            .vars
            .get(LAST_AMOUNT)
            .as_int();
        assert_eq!(last, 7, "the X remembered (varp 1249)");
        button(&mut client, 4, BANK_INV, coins, 0);
        ticks(engine, &mut client, 1);
        assert_eq!(
            count(engine, client.pid, "inv", coins),
            14,
            "Withdraw-7 again"
        );
    });
}

#[test]
fn note_mode_withdraws_notes() {
    runner::on_loaded_engine(|engine| {
        let (logs, cert) = (obj(engine, "logs"), obj(engine, "cert_logs"));
        let mut client = banker(engine, "notes");
        use_booth(engine, &mut client, 2);
        fill_bank(engine, &mut client, &[("logs", 10)]);
        button(&mut client, 1, NOTE_MODE, 0xFFFF, 0xFFFF);
        ticks(engine, &mut client, 1);
        let mode = engine
            .harness_player(client.pid)
            .unwrap()
            .vars
            .get(BANKCERT)
            .as_int();
        assert_eq!(mode, 1, "note mode on (varp 115)");
        button(&mut client, 6, BANK_INV, logs, 0);
        ticks(engine, &mut client, 1);
        assert_eq!(count(engine, client.pid, "inv", cert), 10, "ten noted logs");
        assert_eq!(count(engine, client.pid, "inv", logs), 0);
        // Off again.
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .vars
            .set(BANKCERT, VarValue::Int(1));
        button(&mut client, 1, NOTE_MODE, 0xFFFF, 0xFFFF);
        ticks(engine, &mut client, 1);
        let mode = engine
            .harness_player(client.pid)
            .unwrap()
            .vars
            .get(BANKCERT)
            .as_int();
        assert_eq!(mode, 0, "note mode off");
    });
}

#[test]
fn the_2011_booths_keep_their_two_sided_approach() {
    runner::on_loaded_engine(|engine| {
        let booth = engine.cache.locs.get_by_id(BOOTH.0).expect("booth");
        assert_eq!(
            booth.forceapproach as u8, 10,
            "open north and south, as the cache has it"
        );
    });
}

#[test]
fn a_varrock_booth_s_use_talks_to_the_teller_behind_it() {
    runner::on_loaded_engine(|engine| {
        // Varrock west bank: 2011's booth 782 at 3181,3436 and, behind it at
        // 3180,3436, npc 45, a banker whose ops are memberops; the player in
        // the hall at 3182,3436. Use finds the teller (bank_teller) and talks.
        engine.harness_reset();
        let mut client = engine.harness_login_returning("varrock").expect("login");
        engine.cycle();
        engine.harness_telejump(client.pid, 3182, 0, 3436);
        ticks(engine, &mut client, 3);
        let mut p = ClientPacket::new();
        p.p2_alt1(3436).p2_alt1(782).p1(0).p2_alt2(3181);
        client.send(ClientProt::OpLoc1, &p.finish());
        let packets = ticks(engine, &mut client, 4);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(
            player.is_interface_open(241),
            "the teller answers (npc_chat1); said {:?}",
            HarnessClient::game_messages(&packets)
        );
    });
}

// --- 2011's bank buttons, Equipment Stats screen and tabs -----------------

const SCREEN: u16 = 667;
const WEAR_SIDE: u16 = 670;
const WORN_TAB: u16 = 387;
const DEPOSIT_CARRIED: i32 = (BANK as i32) << 16 | 33;
const DEPOSIT_WORN: i32 = (BANK as i32) << 16 | 35;
const DEPOSIT_BURDEN: i32 = (BANK as i32) << 16 | 37;
const SHOW_STATS: i32 = (BANK as i32) << 16 | 117;
const SHOW_BANK: i32 = (SCREEN as i32) << 16 | 48;
const SCREEN_WORN: i32 = (SCREEN as i32) << 16 | 7;
const SCREEN_SIDE: i32 = (WEAR_SIDE as i32) << 16;
const WORN_SHOW_STATS: i32 = (WORN_TAB as i32) << 16 | 39;
/// The tab buttons: all of the bank, then tabs 1-8 (com62, com60 .. com46).
fn tab_button(tab: i32) -> i32 {
    (BANK as i32) << 16 | (62 - 2 * tab)
}

fn varp(engine: &Engine, client: &HarnessClient, id: u16) -> u32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(id)
        .as_int() as u32
}

/// A tab's item count: varbits 4885-4892, ten bits each, three to a varp from
/// 1246.
fn tab_size(engine: &Engine, client: &HarnessClient, tab: u16) -> u32 {
    let varp_id = 1246 + (tab - 1) / 3;
    (varp(engine, client, varp_id) >> (((tab - 1) % 3) * 10)) & 0x3FF
}

/// The tab on view (varbit 4893: varp 1248 bits 27-30), 1 for all.
fn open_tab(engine: &Engine, client: &HarnessClient) -> u32 {
    (varp(engine, client, 1248) >> 27) & 0xF
}

/// "Show bank" on the stats screen (varbit 4894: varp 1248 bit 31).
fn from_bank(engine: &Engine, client: &HarnessClient) -> u32 {
    varp(engine, client, 1248) >> 31
}

/// The bank hidden by its client (varbit 8348: varp 638 bit 19).
fn bank_hidden(engine: &Engine, client: &HarnessClient) -> u32 {
    (varp(engine, client, 638) >> 19) & 1
}

/// The bank, slot by slot, up to its first empty slot.
fn bank_objs(engine: &Engine, client: &HarnessClient) -> Vec<u16> {
    let id = *engine.cache.invs.debugnames.get("bank").unwrap();
    let inv = engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&id)
        .unwrap();
    (0..inv.capacity as u16)
        .map_while(|s| inv.get(s).map(|i| i.obj))
        .collect()
}

fn give(engine: &mut Engine, client: &HarnessClient, inv: &str, slot: u16, obj: u16, n: u32) {
    let id = *engine.cache.invs.debugnames.get(inv).unwrap();
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.invs.get_mut(&id).unwrap().set(slot, obj, n);
}

/// Each IF_SETTEXT on `iface` as `(com, text)`.
fn texts_on(packets: &[(u16, Vec<u8>)], iface: u16) -> Vec<(u16, String)> {
    of(packets, ServerProt::IfSetText)
        .filter_map(|p| {
            let com = u32::from_be_bytes([p[0], p[1], p[2], p[3]]);
            let end = p[4..]
                .iter()
                .position(|&b| b == 0)
                .map_or(p.len(), |i| 4 + i);
            ((com >> 16) as u16 == iface).then(|| {
                (
                    (com & 0xFFFF) as u16,
                    String::from_utf8_lossy(&p[4..end]).into_owned(),
                )
            })
        })
        .collect()
}

/// Each MESSAGE_GAME's text.
fn game_messages(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    of(packets, ServerProt::MessageGame)
        .map(|p| {
            let mut pos = 5;
            let mask = p[pos];
            pos += 1;
            if mask & 1 == 1 {
                pos += p[pos..].iter().position(|&b| b == 0).unwrap() + 1;
            }
            let end = pos
                + p[pos..]
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(p.len() - pos);
            String::from_utf8_lossy(&p[pos..end]).into_owned()
        })
        .collect()
}

/// A drag from the bank's slot `from` onto `to_com` (a tab button: no slot) or
/// onto bank slot `to_slot`.
fn bank_drag(client: &mut HarnessClient, from: u16, from_obj: u16, to_com: i32, to_slot: i32) {
    let mut p = ClientPacket::new();
    p.p4(BANK_INV)
        .p2_alt3(from as i32)
        .p2_alt3(to_slot)
        .p4_alt2(to_com)
        .p2_alt1(from_obj as i32)
        .p2(-1);
    client.send(ClientProt::IfButtonD, &p.finish());
}

/// A player at the booth with its bank open.
fn at_the_bank(engine: &mut Engine, name: &str) -> HarnessClient {
    let mut client = banker(engine, name);
    use_booth(engine, &mut client, 2);
    assert!(
        engine
            .harness_player(client.pid)
            .unwrap()
            .is_interface_open(BANK)
    );
    client
}

#[test]
fn deposit_carried_and_worn_empty_them_into_the_bank() {
    runner::on_loaded_engine(|engine| {
        let mut client = at_the_bank(engine, "depositor");
        let (logs, sword) = (obj(engine, "logs"), obj(engine, "bronze_sword"));
        give(engine, &client, "inv", 0, logs, 1);
        give(engine, &client, "inv", 5, logs, 1);
        give(engine, &client, "worn", 3, sword, 1);
        ticks(engine, &mut client, 1);

        button(&mut client, 1, DEPOSIT_CARRIED, 0xFFFF, 0xFFFF);
        ticks(engine, &mut client, 2);
        assert_eq!(
            count(engine, client.pid, "inv", logs),
            0,
            "the backpack emptied"
        );
        assert_eq!(count(engine, client.pid, "bank", logs), 2);

        button(&mut client, 1, DEPOSIT_WORN, 0xFFFF, 0xFFFF);
        ticks(engine, &mut client, 2);
        assert_eq!(count(engine, client.pid, "worn", sword), 0, "nothing worn");
        assert_eq!(count(engine, client.pid, "bank", sword), 1);

        // Both empty now, and no familiar: each says so.
        button(&mut client, 1, DEPOSIT_CARRIED, 0xFFFF, 0xFFFF);
        button(&mut client, 1, DEPOSIT_WORN, 0xFFFF, 0xFFFF);
        button(&mut client, 1, DEPOSIT_BURDEN, 0xFFFF, 0xFFFF);
        let said = game_messages(&ticks(engine, &mut client, 2));
        for line in [
            "You have no items in your inventory to deposit.",
            "You have no equipped items to deposit.",
            "You do not have a beast of burden.",
        ] {
            assert!(said.iter().any(|m| m == line), "{line:?} in {said:?}");
        }
    });
}

#[test]
fn the_banks_equipment_button_shows_the_stats_screen_and_show_bank_returns() {
    runner::on_loaded_engine(|engine| {
        let mut client = at_the_bank(engine, "statsbank");
        let sword = obj(engine, "bronze_sword");
        give(engine, &client, "worn", 3, sword, 1);
        ticks(engine, &mut client, 1);

        button(&mut client, 1, SHOW_STATS, 0xFFFF, 0xFFFF);
        let packets = ticks(engine, &mut client, 2);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(player.is_interface_open(SCREEN), "the stats screen");
        assert!(
            player.is_interface_open(WEAR_SIDE),
            "the backpack beside it"
        );
        assert!(!player.is_interface_open(BANK), "the bank gone");
        assert_eq!(from_bank(engine, &client), 1, "Show bank shows");
        assert_eq!(bank_hidden(engine, &client), 1);
        // 289's lines for a bronze sword, at 2011's components and labels.
        let lines = texts_on(&packets, SCREEN);
        for (com, text) in [
            (30, "Stab: +4"),
            (31, "Slash: +3"),
            (32, "Crush: -2"),
            (34, "Ranged: +0"),
            (36, "Slash: +2"),
            (37, "Crush: +1"),
            (44, "Strength: +5"),
            (46, "Prayer: +0"),
        ] {
            assert!(
                lines.iter().any(|(c, t)| *c == com && t == text),
                "667:{com} {text:?} in {lines:?}"
            );
        }

        button(&mut client, 1, SHOW_BANK, 0xFFFF, 0xFFFF);
        ticks(engine, &mut client, 2);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(player.is_interface_open(BANK), "the bank again");
        assert!(!player.is_interface_open(SCREEN));
        assert_eq!(bank_hidden(engine, &client), 0, "and shown");
    });
}

#[test]
fn the_worn_tabs_button_shows_the_screen_without_show_bank() {
    runner::on_loaded_engine(|engine| {
        let mut client = banker(engine, "statstab");
        button(&mut client, 1, WORN_SHOW_STATS, 0xFFFF, 0xFFFF);
        let packets = ticks(engine, &mut client, 2);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(SCREEN)
        );
        // The screen's root starts hidden: proc 2319 shows it, which no bank is
        // open to run here. Content runs it through
        // [clientscript,bank_v2_showequipscreen] (2317 in clientscript.pack),
        // whose whole body calls the proc. It must reach the client after the
        // screen's open, or the open loads the root hidden again.
        let open = ServerProt::IfOpensub as u16;
        let last_open = packets.iter().rposition(|(op, _)| *op == open);
        let reveal = packets.iter().position(|(op, p)| {
            *op == ServerProt::RunClientScript as u16 && scripts(&[(*op, p.clone())]) == [2317]
        });
        assert!(
            matches!((last_open, reveal), (Some(o), Some(r)) if r > o),
            "open at {last_open:?}, 2317 at {reveal:?}"
        );
        assert_eq!(from_bank(engine, &client), 0);
        // Show bank does nothing from here.
        button(&mut client, 1, SHOW_BANK, 0xFFFF, 0xFFFF);
        ticks(engine, &mut client, 2);
        assert!(
            !engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(BANK)
        );
    });
}

/// 2011's Equipment Stats screen totals the worn armour's soak percents
/// (player/scripts/damage.rs2 `~worn_absorb`): full dragon is the 2011 wiki's
/// 30% absorb ranged (Damage soaking), 14% melee and nothing against magic.
#[test]
fn the_stats_screen_totals_the_worn_absorb() {
    runner::on_loaded_engine(|engine| {
        let mut client = banker(engine, "statsabsorb");
        for (slot, name) in [
            (0, "dragon_full_helm"),
            (4, "dragon_platebody"),
            (7, "dragon_platelegs"),
            (5, "dragon_sq_shield"),
        ] {
            let id = obj(engine, name);
            give(engine, &client, "worn", slot, id, 1);
        }
        ticks(engine, &mut client, 1);
        button(&mut client, 1, WORN_SHOW_STATS, 0xFFFF, 0xFFFF);
        let packets = ticks(engine, &mut client, 2);
        let lines = texts_on(&packets, SCREEN);
        for (com, text) in [
            (41, "Absorb Melee: +14%"),
            (42, "Absorb Magic: +0%"),
            (43, "Absorb Ranged: +30%"),
        ] {
            assert!(
                lines.iter().any(|(c, t)| *c == com && t == text),
                "667:{com} {text:?} in {lines:?}"
            );
        }
    });
}

#[test]
fn remove_and_equip_on_the_stats_screen_keep_it_open() {
    runner::on_loaded_engine(|engine| {
        let mut client = banker(engine, "statsequip");
        let sword = obj(engine, "bronze_sword");
        give(engine, &client, "worn", 3, sword, 1);
        ticks(engine, &mut client, 1);
        button(&mut client, 1, WORN_SHOW_STATS, 0xFFFF, 0xFFFF);
        ticks(engine, &mut client, 2);

        button(&mut client, 1, SCREEN_WORN, sword, 3);
        let packets = ticks(engine, &mut client, 2);
        assert_eq!(count(engine, client.pid, "worn", sword), 0, "removed");
        assert_eq!(count(engine, client.pid, "inv", sword), 1);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(SCREEN)
        );
        assert!(
            texts_on(&packets, SCREEN)
                .iter()
                .any(|(c, t)| *c == 30 && t == "Stab: +0"),
            "the bonuses follow"
        );

        let slot = slot_of(engine, client.pid, sword);
        button(&mut client, 1, SCREEN_SIDE, sword, slot);
        let packets = ticks(engine, &mut client, 2);
        assert_eq!(
            count(engine, client.pid, "worn", sword),
            1,
            "equipped again"
        );
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(SCREEN)
        );
        assert!(
            texts_on(&packets, SCREEN)
                .iter()
                .any(|(c, t)| *c == 30 && t == "Stab: +4")
        );
    });
}

#[test]
fn a_drag_onto_a_tab_button_makes_a_tab_and_taking_it_all_out_ends_it() {
    runner::on_loaded_engine(|engine| {
        let mut client = at_the_bank(engine, "tabmaker");
        fill_bank(
            engine,
            &mut client,
            &[("logs", 5), ("coins", 100), ("bronze_bar", 3)],
        );
        let (logs, coins, bar) = (
            obj(engine, "logs"),
            obj(engine, "coins"),
            obj(engine, "bronze_bar"),
        );

        // The bar onto the new-tab button (tab 1): it leads the bank in tab 1.
        bank_drag(&mut client, 2, bar, tab_button(1), -1);
        ticks(engine, &mut client, 1);
        assert_eq!(bank_objs(engine, &client), vec![bar, logs, coins]);
        assert_eq!(tab_size(engine, &client, 1), 1);

        // Coins onto tab 2's button, past the first empty one: tab 2.
        bank_drag(&mut client, 2, coins, tab_button(5), -1);
        ticks(engine, &mut client, 1);
        assert_eq!(bank_objs(engine, &client), vec![bar, coins, logs]);
        assert_eq!(
            (tab_size(engine, &client, 1), tab_size(engine, &client, 2)),
            (1, 1)
        );

        // Viewing tab 2, all of the bar out: tab 1 goes, tab 2 becomes it,
        // and the view follows its items.
        button(&mut client, 1, tab_button(2), 0xFFFF, 0xFFFF);
        ticks(engine, &mut client, 1);
        assert_eq!(open_tab(engine, &client), 3);
        button(&mut client, 6, BANK_INV, bar, 0);
        ticks(engine, &mut client, 1);
        assert_eq!(bank_objs(engine, &client), vec![coins, logs], "no gap left");
        assert_eq!(
            (tab_size(engine, &client, 1), tab_size(engine, &client, 2)),
            (1, 0)
        );
        assert_eq!(open_tab(engine, &client), 2, "still on the coins");
    });
}

#[test]
fn a_new_item_goes_into_the_tab_on_view_and_collapse_returns_it() {
    runner::on_loaded_engine(|engine| {
        let mut client = at_the_bank(engine, "tabdeposit");
        fill_bank(engine, &mut client, &[("logs", 5), ("coins", 100)]);
        let (logs, coins, bar) = (
            obj(engine, "logs"),
            obj(engine, "coins"),
            obj(engine, "bronze_bar"),
        );
        bank_drag(&mut client, 1, coins, tab_button(1), -1);
        ticks(engine, &mut client, 1);
        button(&mut client, 1, tab_button(1), 0xFFFF, 0xFFFF);
        give(engine, &client, "inv", 0, bar, 1);
        ticks(engine, &mut client, 1);

        // Deposit-1 from the backpack beside the bank.
        button(&mut client, 1, SIDE_INV, bar, 0);
        ticks(engine, &mut client, 1);
        assert_eq!(bank_objs(engine, &client), vec![coins, bar, logs]);
        assert_eq!(tab_size(engine, &client, 1), 2);

        // Collapse tab 1: its items after the rest, in their order.
        button(&mut client, 2, tab_button(1), 0xFFFF, 0xFFFF);
        ticks(engine, &mut client, 1);
        assert_eq!(bank_objs(engine, &client), vec![logs, coins, bar]);
        assert_eq!(tab_size(engine, &client, 1), 0);
        assert_eq!(open_tab(engine, &client), 1, "back to all of it");
    });
}

#[test]
fn inserting_into_another_tab_moves_the_item_to_it() {
    runner::on_loaded_engine(|engine| {
        let mut client = at_the_bank(engine, "tabinsert");
        fill_bank(
            engine,
            &mut client,
            &[("logs", 5), ("coins", 100), ("bronze_bar", 3)],
        );
        let (logs, coins, bar) = (
            obj(engine, "logs"),
            obj(engine, "coins"),
            obj(engine, "bronze_bar"),
        );
        bank_drag(&mut client, 2, bar, tab_button(1), -1);
        ticks(engine, &mut client, 1);
        // Insert mode (com15), then logs from the rest onto the bar.
        button(&mut client, 1, (BANK as i32) << 16 | 15, 0xFFFF, 0xFFFF);
        ticks(engine, &mut client, 1);
        bank_drag(&mut client, 1, logs, BANK_INV, 0);
        ticks(engine, &mut client, 1);
        assert_eq!(bank_objs(engine, &client), vec![logs, bar, coins]);
        assert_eq!(tab_size(engine, &client, 1), 2, "the logs joined tab 1");
    });
}

/// Withdraw-All but one of the only one of an item withdraws nothing and says
/// so: "You only have one of this item, so withdrawing all but one does
/// nothing." (2011 wiki, Bank, 2011-05-21).
#[test]
fn all_but_one_of_one_says_it_does_nothing() {
    runner::on_loaded_engine(|engine| {
        let gloves = obj(engine, "leather_gloves");
        let mut client = banker(engine, "lonelyitem");
        use_booth(engine, &mut client, 2);
        fill_bank(engine, &mut client, &[("leather_gloves", 1)]);
        let pid = client.pid;
        button(&mut client, 7, BANK_INV, gloves, 0);
        let said: Vec<String> = HarnessClient::game_messages(&ticks(engine, &mut client, 2))
            .into_iter()
            .map(|(_, m)| m)
            .collect();
        assert_eq!(count(engine, pid, "inv", gloves), 0, "nothing withdrawn");
        assert_eq!(count(engine, pid, "bank", gloves), 1, "still banked");
        assert!(
            said.contains(
                &"You only have one of this item, so withdrawing all but one does nothing."
                    .to_string()
            ),
            "{said:?}"
        );
    });
}
