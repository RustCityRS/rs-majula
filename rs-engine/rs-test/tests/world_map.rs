//! 2011's world map: interface 755 (worldmap), opened over the whole screen.
//!
//! The globe beside the minimap (548:179 worldmap_icon fixed, 746:179
//! worldmap_button resizable) swaps the gameframe for 755 with IF_OPENTOP.
//! The client draws everything on it (zoom, panning, the key, search); the
//! server tells it where the player is through two varcs it reads when the
//! map loads: 622, the map's centre (clientscript 1369 worldmap_init), and
//! 674, "You are here" (clientscript 2045 worldmap_overlay). The close
//! button (755:44) puts the player's frame back. "World map view cannot be
//! accessed during combat" (2011 wiki, World map, rev 2011-05-10; Jagex
//! Knowledge Base, Controls - World Map View, snapshot 2011-05-20).
//!
//! An IF_BUTTON{n} is `p4(com) p2_alt3(obj) p2_alt1(slot)`. IF_OPENTOP is
//! `p2_alt1(interface) p1_alt1(mode)`; CLIENT_SETVARC_LARGE is
//! `p4_alt1(value) p2_alt3(id)`.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_pack::cache::varp::VarPlayerScope;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;
use rs_vm::ops::test::{calls_for, forget_calls, set_recording};

const FIXED: u16 = 548;
const RESIZABLE: u16 = 746;
const WORLDMAP: u16 = 755;
const BACKPACK: u16 = 149;
/// The globe: toplevel:worldmap_icon, toplevel_fullscreen:worldmap_button.
const GLOBE_FIXED: i32 = (FIXED as i32) << 16 | 179;
const GLOBE_RESIZABLE: i32 = (RESIZABLE as i32) << 16 | 179;
/// worldmap:close, worldmap:sort_button.
const CLOSE: i32 = (WORLDMAP as i32) << 16 | 44;
const SORT: i32 = (WORLDMAP as i32) << 16 | 54;
/// worldmap:com_49, the key: its option lines are dynamic slots 0-19.
const KEY_LIST: i32 = (WORLDMAP as i32) << 16 | 49;
/// The key's order (varbit 5367, bits 0-1) and options (6174-6177, bits 2-5).
const KEY_VARP: u16 = 463;
/// The map's centre and "You are here".
const VARC_CENTRE: u16 = 622;
const VARC_YOU_ARE_HERE: u16 = 674;
/// %lastcombat.
const LASTCOMBAT: u16 = 38;
/// Lumbridge, outside the castle.
const TILE: (u16, u16) = (3222, 3218);
const BUSY: &str = "Please finish what you are doing before opening the world map.";

type Packets = Vec<(u16, Vec<u8>)>;

fn packed(level: i32, x: u16, z: u16) -> i32 {
    (level << 28) | ((x as i32) << 14) | z as i32
}

fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Packets {
    let mut packets = Vec::new();
    for _ in 0..n {
        engine.cycle();
        packets.extend(client.packets());
    }
    packets
}

fn button(client: &mut HarnessClient, op: u8, com: i32, slot: u16) {
    let prot = match op {
        1 => ClientProt::IfButton1,
        2 => ClientProt::IfButton2,
        _ => ClientProt::IfButton10,
    };
    let mut p = ClientPacket::new();
    p.p4(com).p2_alt3(0xFFFF).p2_alt1(slot as i32);
    client.send(prot, &p.finish());
}

/// Each IF_OPENTOP's interface, in the order sent.
fn tops(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    top_modes(packets).into_iter().map(|(top, _)| top).collect()
}

/// Each IF_OPENTOP as `(interface, mode)`.
fn top_modes(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, u8)> {
    packets
        .iter()
        .filter(|(op, _)| *op == ServerProt::IfOpentop as u16)
        .map(|(_, p)| (u16::from_le_bytes([p[0], p[1]]), p[2].wrapping_sub(128)))
        .collect()
}

/// Each CLIENT_SETVARC_LARGE as `(id, value)`.
fn varcs(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, i32)> {
    packets
        .iter()
        .filter(|(op, _)| *op == ServerProt::VarcLarge as u16)
        .map(|(_, p)| {
            (
                u16::from_le_bytes([p[4].wrapping_sub(128), p[5]]),
                i32::from_le_bytes([p[0], p[1], p[2], p[3]]),
            )
        })
        .collect()
}

/// Where a packet first appears among `packets`.
fn first(packets: &[(u16, Vec<u8>)], prot: ServerProt) -> Option<usize> {
    let opcode = prot as u16;
    packets.iter().position(|(op, _)| *op == opcode)
}

fn messages(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    HarnessClient::game_messages(packets)
        .into_iter()
        .map(|(_, t)| t)
        .collect()
}

/// A returning player at Lumbridge, in `mode` (1 fixed, 2 resizable).
fn player(engine: &mut Engine, name: &str, mode: u8) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine
        .harness_login_returning_at(name, TILE.0, TILE.1)
        .expect("login");
    engine.cycle();
    if mode != 1 {
        let mut p = ClientPacket::new();
        p.p1(mode as i32).p2(1024).p2(768).p1(0);
        client.send(ClientProt::WindowStatus, &p.finish());
    }
    // A new player's %lastcombat is 0: on a fresh engine's first eight ticks
    // that reads as a fight that has just ended.
    let settle = 2 + 10usize.saturating_sub(engine.clock as usize);
    ticks(engine, &mut client, settle);
    client
}

fn toplevel(engine: &Engine, pid: u16) -> u16 {
    engine.harness_player(pid).unwrap().toplevel
}

/// The globe, fixed: the map's centre and "You are here" are the player's
/// tile, and both arrive before the IF_OPENTOP that loads the map, since
/// worldmap_init reads varc 622 as 755 loads.
#[test]
fn globe_opens_the_map_on_the_players_tile() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "wm_open", 1);
        assert_eq!(toplevel(engine, client.pid), FIXED);
        button(&mut client, 1, GLOBE_FIXED, 0xFFFF);
        let packets = ticks(engine, &mut client, 1);
        let here = packed(0, TILE.0, TILE.1);
        let sent = varcs(&packets);
        assert!(
            sent.contains(&(VARC_CENTRE, here)),
            "varc 622 = the player's tile; sent {sent:?}, said {:?}",
            messages(&packets)
        );
        assert!(
            sent.contains(&(VARC_YOU_ARE_HERE, here)),
            "varc 674 = the player's tile; sent {sent:?}"
        );
        assert_eq!(tops(&packets), vec![WORLDMAP], "the map opens on top");
        assert!(
            first(&packets, ServerProt::VarcLarge) < first(&packets, ServerProt::IfOpentop),
            "the varcs reach the client before the map loads"
        );
        assert_eq!(toplevel(engine, client.pid), WORLDMAP);
        assert!(
            !engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(BACKPACK),
            "the map replaces the whole screen"
        );
    });
}

#[test]
fn resizable_globe_opens_the_map() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "wm_resizable", 2);
        assert_eq!(toplevel(engine, client.pid), RESIZABLE);
        button(&mut client, 1, GLOBE_RESIZABLE, 0xFFFF);
        let packets = ticks(engine, &mut client, 1);
        let here = packed(0, TILE.0, TILE.1);
        assert!(varcs(&packets).contains(&(VARC_CENTRE, here)));
        assert!(varcs(&packets).contains(&(VARC_YOU_ARE_HERE, here)));
        assert_eq!(tops(&packets), vec![WORLDMAP]);
    });
}

/// The close button puts back the frame the player plays in. The client
/// keeps every sub-interface across IF_OPENTOP, so nothing is re-sent. Mode 2
/// has the client rebuild the scene behind "Loading - please wait.", as 2011
/// shows after the map (YouTube 0PgitXjVAiU, 2011-02-09, 09:24).
#[test]
fn close_returns_to_the_players_frame() {
    runner::on_loaded_engine(|engine| {
        for (name, mode, frame, globe) in [
            ("wm_close_fixed", 1, FIXED, GLOBE_FIXED),
            ("wm_close_resizable", 2, RESIZABLE, GLOBE_RESIZABLE),
        ] {
            let mut client = player(engine, name, mode);
            button(&mut client, 1, globe, 0xFFFF);
            ticks(engine, &mut client, 1);
            assert_eq!(toplevel(engine, client.pid), WORLDMAP, "{name}: open");
            button(&mut client, 1, CLOSE, 0xFFFF);
            let packets = ticks(engine, &mut client, 1);
            assert_eq!(
                top_modes(&packets),
                vec![(frame, 2)],
                "{name}: back to {frame}, rebuilding the scene"
            );
            assert_eq!(
                first(&packets, ServerProt::IfOpensub),
                None,
                "{name}: no sub re-sent"
            );
            assert!(
                engine
                    .harness_player(client.pid)
                    .unwrap()
                    .is_interface_open(BACKPACK),
                "{name}: the backpack is back"
            );
        }
    });
}

/// "you will not be able to load the world map while you are in combat"
/// (Jagex Knowledge Base, Controls - World Map View, snapshot 2011-05-20).
/// The refusal's text is not sourced: the magic carpet's is used (2011 wiki,
/// Magic carpet, rev 2011-04-17).
#[test]
fn the_map_will_not_open_in_combat() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "wm_combat", 1);
        let clock = engine.clock as i32;
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .vars
            .set(LASTCOMBAT, VarValue::Int(clock));
        button(&mut client, 1, GLOBE_FIXED, 0xFFFF);
        let packets = ticks(engine, &mut client, 1);
        assert_eq!(tops(&packets), Vec::<u16>::new(), "not in combat");
        assert_eq!(toplevel(engine, client.pid), FIXED);
        assert!(
            messages(&packets).iter().any(|m| m == BUSY),
            "the refusal; said {:?}",
            messages(&packets)
        );

        // Eight ticks after the last hit, it opens.
        ticks(engine, &mut client, 8);
        button(&mut client, 1, GLOBE_FIXED, 0xFFFF);
        let packets = ticks(engine, &mut client, 1);
        assert_eq!(tops(&packets), vec![WORLDMAP], "out of combat");
    });
}

#[test]
fn the_map_will_not_open_while_busy() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "wm_busy", 1);
        let clock = engine.clock;
        {
            let player = engine.harness_player_mut(client.pid).unwrap();
            player.state.delayed = true;
            player.state.delayed_until = clock + 5;
        }
        button(&mut client, 1, GLOBE_FIXED, 0xFFFF);
        let packets = ticks(engine, &mut client, 1);
        assert_eq!(tops(&packets), Vec::<u16>::new(), "not while delayed");
        assert!(
            messages(&packets).iter().any(|m| m == BUSY),
            "the refusal; said {:?}",
            messages(&packets)
        );
    });
}

/// After the map closes, its buttons are not on screen: a press on one (a
/// stale or forged click) runs nothing.
#[test]
fn map_buttons_are_refused_when_the_map_is_shut() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "wm_shut", 1);
        button(&mut client, 1, GLOBE_FIXED, 0xFFFF);
        ticks(engine, &mut client, 1);
        assert_eq!(toplevel(engine, client.pid), WORLDMAP, "open");
        button(&mut client, 1, CLOSE, 0xFFFF);
        ticks(engine, &mut client, 1);
        assert_eq!(toplevel(engine, client.pid), FIXED, "shut");

        button(&mut client, 1, CLOSE, 0xFFFF);
        button(&mut client, 1, SORT, 0xFFFF);
        let packets = ticks(engine, &mut client, 1);
        assert_eq!(tops(&packets), Vec::<u16>::new(), "nothing re-opens");
        assert_eq!(toplevel(engine, client.pid), FIXED);
    });
}

/// Others see the player reading the map while it is open (Jagex news 2419,
/// 9 November 2009): seq 5354, void-634's open_map. Closing it ends the pose.
#[test]
fn the_map_shows_the_player_reading() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "wm_reading", 1);
        set_recording(true);
        forget_calls();
        button(&mut client, 1, GLOBE_FIXED, 0xFFFF);
        ticks(engine, &mut client, 1);
        let opened = calls_for("anim");
        forget_calls();
        button(&mut client, 1, CLOSE, 0xFFFF);
        ticks(engine, &mut client, 1);
        let closed = calls_for("anim");
        set_recording(false);
        assert_eq!(opened, vec!["5354"], "reading the map");
        assert_eq!(closed, vec!["-1"], "and no longer");
    });
}
/// Each IF_SETEVENTS as `(com, from, to, events)`: `p2_alt2(from)
/// p4_alt3(events) p4_alt3(com) p2_alt3(to)`.
fn events(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, u16, u16, i32)> {
    packets
        .iter()
        .filter(|(op, _)| *op == ServerProt::IfSetEvents as u16)
        .map(|(_, p)| {
            let from = u16::from_be_bytes([p[0], p[1].wrapping_sub(128)]);
            let ops = i32::from_be_bytes([p[3], p[2], p[5], p[4]]);
            let com = i32::from_be_bytes([p[7], p[6], p[9], p[8]]);
            let to = u16::from_le_bytes([p[10].wrapping_sub(128), p[11]]);
            (com, from, to, ops)
        })
        .collect()
}

fn varp(engine: &Engine, pid: u16, id: u16) -> i32 {
    engine.harness_player(pid).unwrap().vars.get(id).as_int()
}

/// Opens the map from the fixed globe.
fn open_map(engine: &mut Engine, client: &mut HarnessClient) -> Packets {
    button(client, 1, GLOBE_FIXED, 0xFFFF);
    let packets = ticks(engine, client, 1);
    assert_eq!(toplevel(engine, client.pid), WORLDMAP, "the map is open");
    packets
}

/// The key's option lines are dynamic children of 755:49 (script285), slots
/// 0-19; the client sends their ops only for the slots the server enables.
#[test]
fn opening_allows_the_key_options() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "wm_events", 1);
        let packets = open_map(engine, &mut client);
        let ev = events(&packets);
        assert!(
            ev.iter()
                .any(|&(c, f, t, ops)| c == KEY_LIST && f == 0 && t == 19 && ops & (1 << 1) != 0),
            "755:49 slots 0-19, op1; sent {ev:?}"
        );
    });
}

/// "Re-sort key" steps Traditional (0), Alphabetical (1), Categorised (2) and
/// back (clientscripts 692 worldmap_keysort_button, 1839), in varbit 5367.
#[test]
fn resort_steps_traditional_alphabetical_categorised() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "wm_sort", 1);
        open_map(engine, &mut client);
        assert_eq!(varp(engine, client.pid, KEY_VARP) & 3, 0, "Traditional");
        for want in [1, 2, 0] {
            button(&mut client, 1, SORT, 0xFFFF);
            ticks(engine, &mut client, 1);
            assert_eq!(varp(engine, client.pid, KEY_VARP) & 3, want);
        }
    });
}

/// script285 lays each option line out as two clickable children: "You are
/// here" 0 and 1 (varbit 6174, bit 2), intra-map links 3 and 4 (6175, bit 3),
/// map labels 11 and 12 (6176, bit 4), icon tooltips 14 and 16 (6177, bit 5).
/// The client relabels the line; the server flips the bit.
#[test]
fn each_key_option_flips_its_own_bit() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "wm_options", 1);
        open_map(engine, &mut client);
        for (slot, bit) in [
            (0, 2),
            (1, 2),
            (3, 3),
            (4, 3),
            (11, 4),
            (12, 4),
            (14, 5),
            (16, 5),
        ] {
            let before = varp(engine, client.pid, KEY_VARP);
            button(&mut client, 1, KEY_LIST, slot);
            ticks(engine, &mut client, 1);
            let after = varp(engine, client.pid, KEY_VARP);
            assert_eq!(after ^ before, 1 << bit, "slot {slot} flips bit {bit}");
        }
        for slot in [2, 5, 6, 7, 8, 9, 10, 13, 15, 17, 20, 40] {
            let before = varp(engine, client.pid, KEY_VARP);
            button(&mut client, 1, KEY_LIST, slot);
            ticks(engine, &mut client, 1);
            assert_eq!(
                varp(engine, client.pid, KEY_VARP),
                before,
                "slot {slot} changes nothing"
            );
        }
    });
}

/// The key's order and options last the session, as cache 786 has varp 463:
/// nothing from 2011 shows them outliving a logout (void-634 saves them).
#[test]
fn the_key_settings_last_the_session() {
    runner::on_loaded_engine(|engine| {
        let varp = engine.cache.varps.get_by_id(KEY_VARP).expect("varp 463");
        assert!(varp.scope == VarPlayerScope::Temp, "not saved");
    });
}
