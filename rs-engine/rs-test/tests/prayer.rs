//! Prayer ported from 289: burying bones, praying at an altar, and 289's
//! fifteen prayers on 2011's prayer book.
//!
//! Each case drives the engine as the client would: Bury is the backpack's
//! first op (IF_BUTTON1, `Class199.method1317`: `p4(com) ip2_alt1(obj)
//! ip2(slot)`), praying is OPLOC1 on the altar
//! (`ip2(z) ip2(loc) p1(ctrl) p2_alt1(x)`), and a prayer is IF_BUTTON1 on
//! the book's com8 at the prayer's slot (enum 2279's order).

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

/// `inventory:inv`.
const BACKPACK: i32 = 149 << 16;
const PRAYER: usize = 5;
/// The prayer book (271): its prayers are com8's children.
const BOOK: i32 = 271 << 16 | 8;
/// 2011's active prayers, a bit each in 289's order (varbits 5942-5956).
const ACTIVE: u16 = 1395;
/// Book slots (enum 2279) and their bits.
const THICK_SKIN: (u16, u32) = (0, 0);
const ROCK_SKIN: (u16, u32) = (5, 3);
const PROTECT_FROM_MELEE: (u16, u32) = (19, 14);
/// Synths: 289's thick_skin, cancel_prayer and prayer_off at 2011.
const THICK_SKIN_SOUND: u16 = 2690;
const CANCEL_PRAYER: u16 = 2663;
const PRAYER_OFF: u16 = 2673;

#[test]
fn bones_bury_and_altars_recharge() {
    runner::on_loaded_engine(|engine| {
        bury(engine);
        altar(engine);
    });
}

/// 289's `bury_bone.rs2`: the hole is dug at once, and the bones go a tick
/// later with their `bone_exp` (45 tenths of an xp for plain bones).
fn bury(engine: &mut Engine) {
    let bones = *engine.cache.objs.debugnames.get("bones").unwrap();
    engine.harness_reset();
    let mut client = engine.harness_login_returning("buryer").expect("login");
    engine.cycle();
    client.packets();
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.invs.get_mut(&inv).unwrap().set(0, bones, 1);
    let xp = player.stats.xp[PRAYER];

    let mut p = ClientPacket::new();
    p.p4(BACKPACK).ip2_alt1(bones as i32).ip2(0);
    client.send(ClientProt::IfButton1, &p.finish());
    engine.cycle();
    assert_eq!(
        HarnessClient::game_messages(&client.packets()),
        [(0, "You dig a hole in the ground...".to_string())]
    );
    let player = engine.harness_player(client.pid).unwrap();
    assert!(
        player.invs.get(&inv).unwrap().has_at(0, bones),
        "not buried yet"
    );

    engine.cycle();
    assert_eq!(
        HarnessClient::game_messages(&client.packets()),
        [(0, "You bury the bones.".to_string())]
    );
    let player = engine.harness_player(client.pid).unwrap();
    assert!(
        player.invs.get(&inv).unwrap().get(0).is_none(),
        "the bones are gone"
    );
    assert_eq!(player.stats.xp[PRAYER] - xp, 45, "plain bones' bone_exp");
}

/// 289's `altar.rs2`, at the altar 2011's Lumbridge church has (36972; 289's
/// was 409, gone with the 2009 rebuild).
fn altar(engine: &mut Engine) {
    engine.harness_reset();
    let mut client = engine.harness_login_returning("prayer").expect("login");
    engine.cycle();
    engine.harness_telejump(client.pid, 3243, 0, 3210);
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.stats.base_levels[PRAYER] = 20;
    player.stats.levels[PRAYER] = 3;
    engine.cycle();
    client.packets();

    assert_eq!(
        pray(engine, &mut client),
        "You recharge your Prayer points."
    );
    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.stats.levels[PRAYER], 20, "back to the base level");

    assert_eq!(
        pray(engine, &mut client),
        "You already have full Prayer points."
    );
}

/// Clicks Pray on the Lumbridge altar and waits for the answer.
fn pray(engine: &mut Engine, client: &mut HarnessClient) -> String {
    let mut p = ClientPacket::new();
    p.ip2(3206).ip2(36972).p1(0).p2_alt1(3243);
    client.send(ClientProt::OpLoc1, &p.finish());
    for _ in 0..10 {
        engine.cycle();
        if let Some((_, text)) = HarnessClient::game_messages(&client.packets()).pop() {
            return text;
        }
    }
    panic!("the altar never answered");
}

fn of(packets: &[(u16, Vec<u8>)], prot: ServerProt) -> impl Iterator<Item = &Vec<u8>> {
    let opcode = prot as u16;
    packets
        .iter()
        .filter(move |(op, _)| *op == opcode)
        .map(|(_, p)| p)
}

/// Each IF_SETEVENTS: `p2_alt1(from) p4_alt3(events) p4_alt3(com) ip2_alt1(to)`.
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

/// Each varc set, `(varc, value)`: VARC_SMALL `p1_alt1 value, ip2 varc`;
/// VARC_LARGE `ip4 value, ip2_alt1 varc`.
fn varcs(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, i32)> {
    let mut out: Vec<(u16, i32)> = of(packets, ServerProt::VarcSmall)
        .map(|p| {
            (
                u16::from_le_bytes([p[1], p[2]]),
                0u8.wrapping_sub(p[0]) as i8 as i32,
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

/// Each SYNTH_SOUND's synth (`p2 synth` first).
fn synths(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    of(packets, ServerProt::SynthSound)
        .map(|p| u16::from_be_bytes([p[0], p[1]]))
        .collect()
}

/// The varps sent: VARP_SMALL `p1_alt2 value, p2 varp`, VARP_LARGE
/// `p2 varp, p4_alt2 value`.
fn varps_sent(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    let mut out: Vec<u16> = of(packets, ServerProt::VarpSmall)
        .map(|p| u16::from_be_bytes([p[1], p[2]]))
        .collect();
    out.extend(of(packets, ServerProt::VarpLarge).map(|p| u16::from_be_bytes([p[0], p[1]])));
    out
}

/// Each IF_SETTEXT's text (`p4 com` first).
fn texts(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    of(packets, ServerProt::IfSetText)
        .map(|p| {
            let end = p[4..]
                .iter()
                .position(|&b| b == 0)
                .map_or(p.len(), |i| 4 + i);
            String::from_utf8_lossy(&p[4..end]).into_owned()
        })
        .collect()
}

/// A player with `level` Prayer, their login sent.
fn prayer_player(engine: &mut Engine, name: &str, level: u16) -> HarnessClient {
    engine.harness_reset();
    let client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.stats.base_levels[PRAYER] = level;
    player.stats.levels[PRAYER] = level;
    engine.cycle();
    client
}

fn active(engine: &Engine, client: &HarnessClient) -> u32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(ACTIVE)
        .as_int() as u32
}

/// Clicks Activate on a book slot and runs two ticks.
fn click(engine: &mut Engine, client: &mut HarnessClient, slot: u16) -> Vec<(u16, Vec<u8>)> {
    let mut p = ClientPacket::new();
    p.p4(BOOK).ip2_alt1(-1).ip2(slot as i32);
    client.send(ClientProt::IfButton1, &p.finish());
    let mut packets = Vec::new();
    for _ in 0..2 {
        engine.cycle();
        packets.extend(client.packets());
    }
    packets
}

/// 2011's book draws all thirty prayers; Activate goes to 289's fifteen.
#[test]
fn the_book_unlocks_289s_prayers() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("praybook").expect("login");
        engine.cycle();
        let packets = client.packets();
        // The book offers Activate only while varc 181 (selecting quick
        // prayers) is 0; the client starts it at -1.
        let quick: Vec<(u16, i32)> = varcs(&packets)
            .into_iter()
            .filter(|(varc, _)| [181, 182].contains(varc))
            .collect();
        assert_eq!(quick, [(181, 0), (182, 0)]);
        let mut slots: Vec<(u16, u16)> = events(&packets)
            .into_iter()
            .filter(|&(com, ..)| com == BOOK)
            .map(|(_, from, to, ops)| {
                assert_eq!(ops & 0x7fe, 1 << 1, "op1 alone");
                (from, to)
            })
            .collect();
        slots.sort();
        assert_eq!(slots, [(0, 2), (5, 10), (13, 15), (17, 19)]);
    });
}

/// Thick Skin lights with its sound and goes out with 289's cancel.
#[test]
fn a_prayer_turns_on_and_off() {
    runner::on_loaded_engine(|engine| {
        let mut client = prayer_player(engine, "praying", 1);
        client.packets();
        let packets = click(engine, &mut client, THICK_SKIN.0);
        assert_eq!(active(engine, &client), 1 << THICK_SKIN.1);
        assert!(varps_sent(&packets).contains(&ACTIVE), "the book is told");
        assert_eq!(synths(&packets), [THICK_SKIN_SOUND]);

        let packets = click(engine, &mut client, THICK_SKIN.0);
        assert_eq!(active(engine, &client), 0);
        assert_eq!(synths(&packets), [CANCEL_PRAYER]);
    });
}

/// 289's conflicts: Rock Skin puts Thick Skin out.
#[test]
fn rock_skin_puts_thick_skin_out() {
    runner::on_loaded_engine(|engine| {
        let mut client = prayer_player(engine, "rockskin", 10);
        click(engine, &mut client, THICK_SKIN.0);
        click(engine, &mut client, ROCK_SKIN.0);
        assert_eq!(active(engine, &client), 1 << ROCK_SKIN.1);
    });
}

/// Below the level: 2011's wording from the prayer's struct, and no sound,
/// which 2011's client plays itself.
#[test]
fn a_prayer_above_the_level_says_2011s_requirement() {
    runner::on_loaded_engine(|engine| {
        let mut client = prayer_player(engine, "lowpray", 1);
        client.packets();
        let packets = click(engine, &mut client, PROTECT_FROM_MELEE.0);
        assert_eq!(active(engine, &client), 0);
        // The mesbox wraps it over 2011's chat box lines.
        let said = texts(&packets).join(" ");
        assert!(
            said.contains("You need a Prayer level of at least 43 to use Protect from Melee."),
            "{said:?}"
        );
        assert!(!synths(&packets).contains(&PRAYER_OFF));
    });
}

/// Protect from Melee draws 2011's prayer head icon 0 (the appearance's
/// fourth body byte: flags, title, skull, prayer).
#[test]
fn protect_from_melee_shows_its_head_icon() {
    runner::on_loaded_engine(|engine| {
        let mut client = prayer_player(engine, "protector", 43);
        click(engine, &mut client, PROTECT_FROM_MELEE.0);
        let player = engine.harness_player(client.pid).unwrap();
        let block = player
            .info
            .last_appearance_info
            .as_ref()
            .expect("appearance");
        assert_eq!(block[3] as i8, 0, "headicons_prayer[0]");

        click(engine, &mut client, PROTECT_FROM_MELEE.0);
        let player = engine.harness_player(client.pid).unwrap();
        let block = player
            .info
            .last_appearance_info
            .as_ref()
            .expect("appearance");
        assert_eq!(block[3] as i8, -1, "none");
    });
}

/// 289's drain at its resistance (60 bare, from update_bonuses at login),
/// and 2011's message when it runs out.
#[test]
fn prayer_runs_out_with_2011s_message() {
    runner::on_loaded_engine(|engine| {
        let mut client = prayer_player(engine, "drained", 43);
        let resistance = *engine
            .cache
            .varps
            .debugnames
            .get("prayer_drain_resistance")
            .unwrap();
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.vars.get(resistance).as_int(), 60);
        engine.harness_player_mut(client.pid).unwrap().stats.levels[PRAYER] = 1;
        click(engine, &mut client, PROTECT_FROM_MELEE.0);
        assert_eq!(active(engine, &client), 1 << PROTECT_FROM_MELEE.1);
        let mut said = Vec::new();
        for _ in 0..10 {
            engine.cycle();
            said.extend(HarnessClient::game_messages(&client.packets()));
        }
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.stats.levels[PRAYER], 0);
        assert_eq!(active(engine, &client), 0, "all out");
        let out = "You have run out of Prayer points; you can recharge at an altar.";
        assert!(said.contains(&(0, out.to_string())), "{said:?}");
    });
}

/// With no points left: 2011's capitalised Prayer.
#[test]
fn no_points_says_recharge_at_an_altar() {
    runner::on_loaded_engine(|engine| {
        let mut client = prayer_player(engine, "nopoints", 10);
        engine.harness_player_mut(client.pid).unwrap().stats.levels[PRAYER] = 0;
        client.packets();
        let packets = click(engine, &mut client, THICK_SKIN.0);
        assert_eq!(active(engine, &client), 0);
        assert_eq!(
            HarnessClient::game_messages(&packets),
            [(
                0,
                "You need to recharge your Prayer at an altar.".to_string()
            )]
        );
    });
}
