//! Prayer ported from 289: burying bones, praying at an altar, and 289's
//! fifteen prayers on 2011's prayer book.
//!
//! Each case drives the engine as the client would: Bury is the backpack's
//! first op (IF_BUTTON1), praying is OPLOC1 on the altar, and a prayer is
//! IF_BUTTON1 on the book's com8 at the prayer's slot (enum 2279's order).

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_test::names::BACKPACK;
#[cfg(since_647)]
use rs_test::names::com;
use rs_test::runner;
#[cfg(since_647)]
use rs_test::server;
use rs_ver::TypeId;

const PRAYER: usize = 5;
/// The prayer book (271): its prayers are com8's children.
#[cfg(since_647)]
fn book() -> i32 {
    com("prayer:prayer_buttons")
}

/// 2011's active prayers, a bit each in 289's order (varpbits 5942-5956).
#[cfg(since_647)]
const ACTIVE: u16 = 1395;
/// Book slots (enum 2279) and their bits.
#[cfg(since_647)]
const THICK_SKIN: (u16, u32) = (0, 0);
#[cfg(since_647)]
const ROCK_SKIN: (u16, u32) = (5, 3);
#[cfg(since_647)]
const PROTECT_FROM_MELEE: (u16, u32) = (19, 14);
/// Synths: 289's thick_skin and cancel_prayer at void-634's ids, and
/// prayer_off (2673, which clientscript 2295 plays).
#[cfg(since_647)]
const THICK_SKIN_SOUND: u16 = 2690;
#[cfg(since_647)]
const CANCEL_PRAYER: u16 = 2663;
#[cfg(since_647)]
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
    let bones = engine.cache.objs.get_by_debugname("bones").unwrap().id;
    engine.harness_reset();
    let mut client = engine.harness_login_returning("buryer").expect("login");
    // Past the login's music unlock (530 unlocks Newbie Melody with a message).
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.invs.get_mut(&inv).unwrap().set(0, bones, 1);
    let xp = player.stats.xp[PRAYER];

    client.if_button(1, BACKPACK, 0, bones);
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
/// was 409, which the 786 map no longer places; no source found for when).
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
    client.op_loc(1, 3243, 3206, TypeId::from_int(36972));
    for _ in 0..10 {
        engine.cycle();
        if let Some((_, text)) = HarnessClient::game_messages(&client.packets()).pop() {
            return text;
        }
    }
    panic!("the altar never answered");
}

/// Each IF_SETEVENTS: `p2_alt2(from) p4_alt3(events) p4_alt3(com) p2_alt3(to)`.
#[cfg(since_647)]
fn events(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, u16, u16, i32)> {
    server::events(packets)
}

/// Each varc set, `(varc, value)`: VARC_SMALL `p1_alt2 value, p2_alt1 varc`;
/// VARC_LARGE `p4_alt1 value, p2_alt3 varc`.
#[cfg(since_647)]
fn varcs(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, i32)> {
    server::varcs(packets)
}

/// Each SYNTH_SOUND's synth (`p2 synth` first).
#[cfg(since_647)]
fn synths(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    server::synths(packets)
        .into_iter()
        .map(|(s, _)| s)
        .collect()
}

/// The varps sent: VARP_SMALL `p1_alt3 value, p2 varp`, VARP_LARGE
/// `p2 varp, p4_alt2 value`.
#[cfg(since_647)]
fn varps_sent(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    server::varps(packets).into_iter().map(|(v, _)| v).collect()
}

/// Each IF_SETTEXT's text (`p4 com` first).
#[cfg(since_647)]
fn texts(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    server::settexts(packets)
        .into_iter()
        .map(|(_, t)| t)
        .collect()
}

/// A player with `level` Prayer, their login sent.
#[cfg(since_647)]
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

#[cfg(since_647)]
fn active(engine: &Engine, client: &HarnessClient) -> u32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(ACTIVE)
        .as_int() as u32
}

/// Clicks Activate on a book slot and runs two ticks.
#[cfg(since_647)]
fn click(engine: &mut Engine, client: &mut HarnessClient, slot: u16) -> Vec<(u16, Vec<u8>)> {
    client.if_button(1, book(), slot, TypeId::NULL);
    let mut packets = Vec::new();
    for _ in 0..2 {
        engine.cycle();
        packets.extend(client.packets());
    }
    packets
}

/// 2011's book draws all thirty prayers; Activate goes to 289's fifteen.
#[cfg(since_647)]
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
            .filter(|&(com, ..)| com == book())
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
#[cfg(since_647)]
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
#[cfg(since_647)]
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
#[cfg(since_647)]
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

/// Protect from Melee draws 2011's prayer head icon 0 (the appearance's body:
/// flags, title, skull, prayer on 647; flags, skull, prayer on 742).
#[cfg(since_647)]
#[test]
fn protect_from_melee_shows_its_head_icon() {
    #[cfg(before_742)]
    const PRAYER_ICON: usize = 3;
    #[cfg(since_742)]
    const PRAYER_ICON: usize = 2;
    runner::on_loaded_engine(|engine| {
        let mut client = prayer_player(engine, "protector", 43);
        click(engine, &mut client, PROTECT_FROM_MELEE.0);
        let player = engine.harness_player(client.pid).unwrap();
        let block = player
            .info
            .last_appearance_info
            .as_ref()
            .expect("appearance");
        assert_eq!(block[PRAYER_ICON] as i8, 0, "headicons_prayer[0]");

        click(engine, &mut client, PROTECT_FROM_MELEE.0);
        let player = engine.harness_player(client.pid).unwrap();
        let block = player
            .info
            .last_appearance_info
            .as_ref()
            .expect("appearance");
        assert_eq!(block[PRAYER_ICON] as i8, -1, "none");
    });
}

/// 289's drain at its resistance (60 bare, from update_bonuses at login),
/// and 2011's message when it runs out.
#[cfg(since_647)]
#[test]
fn prayer_runs_out_with_2011s_message() {
    runner::on_loaded_engine(|engine| {
        let mut client = prayer_player(engine, "drained", 43);
        let resistance = engine
            .cache
            .varps
            .get_by_debugname("prayer_drain_resistance")
            .unwrap()
            .id;
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.vars.get(resistance.g2()).as_int(), 60);
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

/// With no points left: void-634's capitalised Prayer (QuickPrayers.kt).
#[cfg(since_647)]
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
