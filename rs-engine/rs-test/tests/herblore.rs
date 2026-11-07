//! Herblore from 289, on 2011's herbs and potions.
//!
//! 289 identifies an unidentified herb and names it ("This herb is a Guam
//! Leaf."); 2011's herbs are already named -- "Grimy guam", op Clean -- and
//! cleaning one says nothing (void-634's `HerbCleaning`, and the 2007 game).
//! The cases clean a grimy guam as the client does (IF_BUTTON1 on the
//! backpack: `p4(com) p2_alt3(obj) p2_alt1(slot)`), put the clean guam into a
//! vial of water, and mix an eye of newt into the result for an attack
//! potion, both by use-on (IF_BUTTONT: `p4(to_com) p2_alt1(to_slot)
//! p2_alt1(from_slot) p2_alt2(from_obj) p4_alt1(from_com) p2(to_obj)`). 289's
//! 3-dose potions share their names with the brewing structs that make them;
//! the potions here are 647's (`attack_potion__3`), which the port missed
//! until the porter learned names that start with a digit. Herblore pays
//! through `~givexp`, so 2011's xp counter counts it.
//!
//! 289's antifire (anti_fire.rs2): drinking a 4-dose antifire leaves the
//! 3-dose, sets `%dragonresist` 600 ticks on, and says 289's two lines.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const HERBLORE: usize = 15;
/// 2011's xp counter, which counts ten times what `~givexp` pays.
const XP_COUNTER: u16 = 1801;

// The cases share one loaded engine and each starts with a reset, which would
// log the others' herbalists out: one at a time.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// `inventory:inv`, the backpack component `[login]` binds `inv` to.
const BACKPACK: i32 = 149 << 16;

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

fn messages(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..ticks {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    said
}

/// Uses the item in backpack slot `from` on the one in slot `to`.
fn use_on(client: &mut HarnessClient, (from, from_obj): (u16, u16), (to, to_obj): (u16, u16)) {
    let mut p = ClientPacket::new();
    p.p4(BACKPACK)
        .p2_alt1(to as i32)
        .p2_alt1(from as i32)
        .p2_alt2(from_obj as i32)
        .p4_alt1(BACKPACK)
        .p2(to_obj as i32);
    client.send(ClientProt::IfButtonT, &p.finish());
}

#[test]
fn cleaning_and_brewing_on_2011s_herbs() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let grimy = obj(engine, "grimy_guam");
        let clean = obj(engine, "clean_guam");
        let water = obj(engine, "vial_of_water");
        let unf = obj(engine, "guam_potion__unf");
        let newt = obj(engine, "eye_of_newt");
        let attack = obj(engine, "attack_potion__3");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();

        engine.harness_reset();
        let mut client = engine.harness_login_returning("herbalist").expect("login");
        engine.cycle();
        let player = engine.harness_player_mut(client.pid).unwrap();
        // Level 3, which guam needs: 174 xp, in tenths, so xp gained keeps it.
        player.stats.xp[HERBLORE] = 1740;
        player.stats.base_levels[HERBLORE] = 3;
        player.stats.levels[HERBLORE] = 3;
        let backpack = player.invs.get_mut(&inv).unwrap();
        backpack.set(0, grimy, 1);
        backpack.set(1, water, 1);
        backpack.set(2, newt, 1);
        let xp = player.stats.xp[HERBLORE];
        let counted = player.vars.get(XP_COUNTER).as_int();
        engine.cycle();
        client.packets();

        // Clean: the herb is cleaned in its slot, for 2.5 xp, in silence.
        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(grimy as i32).p2_alt1(0);
        client.send(ClientProt::IfButton1, &p.finish());
        let said = messages(engine, &mut client, 2);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(
            player.invs.get(&inv).unwrap().has_at(0, clean),
            "the guam is clean; the game said {said:?}"
        );
        assert_eq!(player.stats.xp[HERBLORE] - xp, 25);
        assert_eq!(
            player.vars.get(XP_COUNTER).as_int() - counted,
            250,
            "the xp counter counts the cleaning"
        );
        assert!(said.is_empty(), "cleaning says nothing in 2011: {said:?}");

        // The clean guam into the vial of water.
        use_on(&mut client, (0, clean), (1, water));
        let said = messages(engine, &mut client, 3);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(unf), 1, "the game said {said:?}");
        assert_eq!(backpack.total(clean), 0);
        assert_eq!(backpack.total(water), 0);
        assert_eq!(said, ["You put the Guam Leaf into the vial of water."]);

        // The eye of newt into it: 647's attack potion (3), for 25 xp.
        let slot = (0..28u16)
            .find(|&s| backpack.has_at(s, unf))
            .expect("the unfinished potion's slot");
        let xp = player.stats.xp[HERBLORE];
        let counted = player.vars.get(XP_COUNTER).as_int();
        use_on(&mut client, (2, newt), (slot, unf));
        let said = messages(engine, &mut client, 3);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(attack), 1, "the game said {said:?}");
        assert_eq!(backpack.total(newt), 0);
        assert_eq!(player.stats.xp[HERBLORE] - xp, 250);
        assert_eq!(player.vars.get(XP_COUNTER).as_int() - counted, 2500);
        assert_eq!(said, ["You mix the eye of newt into your potion."]);
    });
}

#[test]
fn drinking_an_antifire_resists_dragonfire_for_600_ticks() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let four = obj(engine, "antifire__4");
        let three = obj(engine, "antifire__3");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let resist = *engine.cache.varps.debugnames.get("dragonresist").unwrap();

        engine.harness_reset();
        let mut client = engine.harness_login_returning("drinker").expect("login");
        engine.cycle();
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&inv).unwrap().set(0, four, 1);
        engine.cycle();
        client.packets();

        // Drink, op1 on the backpack.
        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(four as i32).p2_alt1(0);
        client.send(ClientProt::IfButton1, &p.finish());
        let said = messages(engine, &mut client, 1);
        let drunk_at = engine.clock as i32;
        let player = engine.harness_player(client.pid).unwrap();
        assert!(
            player.invs.get(&inv).unwrap().has_at(0, three),
            "the 3-dose is left in the slot; the game said {said:?}"
        );
        let until = player.vars.get(resist).as_int();
        assert!(
            (595..=600).contains(&(until - drunk_at)),
            "dragonresist {until} at clock {drunk_at}"
        );
        let said: Vec<String> = said
            .into_iter()
            .chain(messages(engine, &mut client, 2))
            .collect();
        assert_eq!(
            said,
            [
                "You drink some of your dragon potion.",
                "You have 3 doses of potion left."
            ]
        );
    });
}
