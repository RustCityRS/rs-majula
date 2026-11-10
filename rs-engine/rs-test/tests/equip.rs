//! Wield requirements ported from 289 reach the items 647 renamed.
//!
//! The level checks are `[opheld2,<obj>]` triggers written with 289's obj
//! names. Where 647 renamed the obj -- `black_axe` is `black_hatchet`, the
//! dragonhide set is `*_d_hide_*`, the Saradomin rune set lost its `rune_`
//! prefix -- the trigger's subject did not exist, so it never dispatched and
//! anyone could wield the item at any level. Each case clicks Wield on the
//! item in the backpack, as the client does, and checks the level gate holds.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;
use rs_vm::ops::test::{calls_for, forget_calls};

/// `inventory:inv`.
const BACKPACK: i32 = 149 << 16;

fn id(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

/// Logs a returning player in holding `obj` in backpack slot 0, with every
/// stat at `level`, and clicks its Wield (`IF_BUTTON2`, `Class199.method1317`:
/// `p4(com) p2_alt3(obj) p2_alt1(slot)`). Returns whether the obj is still in
/// the backpack afterwards.
fn wield(engine: &mut Engine, name: &str, level: u16) -> bool {
    let obj = id(engine, name);
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let player = engine.harness_player_mut(client.pid).unwrap();
    for stat in 0..player.stats.base_levels.len() {
        player.stats.base_levels[stat] = level;
        player.stats.levels[stat] = level;
    }
    player.invs.get_mut(&inv).expect("backpack").set(0, obj, 1);
    forget_calls();

    let mut p = ClientPacket::new();
    p.p4(BACKPACK).p2_alt3(obj as i32).p2_alt1(0);
    client.send(ClientProt::IfButton2, &p.finish());
    engine.cycle();

    engine
        .harness_player(client.pid)
        .and_then(|p| p.invs.get(&inv))
        .is_some_and(|backpack| backpack.has_at(0, obj))
}

/// Whether the wield trigger answered at all. The refusal is the 289 port's:
/// a level message, or for the dragonhide body the Dragon Slayer one that
/// 289 checks first.
fn refused() -> bool {
    !calls_for("mes").is_empty()
}

#[test]
fn the_renamed_items_keep_their_wield_levels() {
    runner::on_loaded_engine(|engine| {
        for name in ["black_axe", "dragonhide_body", "rune_platebody_saradomin"] {
            assert!(
                wield(engine, name, 1),
                "{name} stays in the backpack at level 1"
            );
            assert!(refused(), "{name}: the wield trigger answered");
        }

        // And the gate lets a player of the level through: Wield moves it.
        assert!(!wield(engine, "black_axe", 10), "wielded at attack 10");
        assert!(!refused(), "no refusal at the required level");
    });
}

/// 289's wield requirements that 647's first port dropped, because the names
/// did not resolve then: rune claws (40 Attack), the rune halberd (40 Attack
/// and 20 Strength), the trimmed rune platebody (Dragon Slayer and 40
/// Defence), and the Fremennik helms (The Fremennik Trials and 45 Defence):
/// a berserker helm stays in the backpack even at 99 without the quest.
#[test]
fn the_wield_requirements_the_first_port_dropped_hold() {
    runner::on_loaded_engine(|engine| {
        for name in [
            "rune_claws",
            "rune_halberd",
            "rune_platebody_trim",
            "viking_helmet_crush",
        ] {
            assert!(
                wield(engine, name, 1),
                "{name} stays in the backpack at level 1"
            );
            assert!(refused(), "{name}: the wield trigger answered");
        }
        assert!(
            wield(engine, "viking_helmet_crush", 99),
            "no helm without the Trials"
        );
        assert!(
            calls_for("mes")
                .iter()
                .any(|m| m.contains("Only Fremenniks are worthy of wearing this helm.")),
            "289's Fremennik message"
        );
        assert!(!wield(engine, "rune_claws", 40), "rune claws at Attack 40");
    });
}

/// Two lines the PR's newer 289 has and the first re-port missed: dragon
/// platelegs need 60 Defence (`tier60.rs2`), and the dragon dagger's quest
/// message spells "Lost City Of Zanaris" (`levelrequire.rs2`).
#[test]
fn the_newer_289_wield_lines_hold() {
    runner::on_loaded_engine(|engine| {
        assert!(
            wield(engine, "dragon_platelegs", 1),
            "dragon platelegs stay in the backpack at level 1"
        );
        assert!(refused(), "dragon_platelegs: the wield trigger answered");
        assert!(
            !wield(engine, "dragon_platelegs", 60),
            "dragon platelegs at Defence 60"
        );

        assert!(
            wield(engine, "dragon_dagger", 99),
            "no dragon dagger without Lost City"
        );
        assert!(
            calls_for("mes")
                .iter()
                .any(|m| m.contains("You need to complete the Lost City Of Zanaris Quest.")),
            "289's Zanaris message"
        );
    });
}

/// A wielded weapon brings its 2011 stance: the render anim (BAS) its cache
/// param 644 names (`void-634`'s RENDER_EMOTE), which the appearance carries in
/// place of 289's seven base animations. A two-handed sword stands as 124, a
/// sword as 1381; nothing wielded is 1426 (void-634's `human_stand`).
#[test]
fn a_wielded_weapon_brings_its_2011_stance() {
    runner::on_loaded_engine(|engine| {
        let two_handed = id(engine, "bronze_2h_sword");
        let sword = id(engine, "bronze_sword");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        engine.harness_reset();
        let mut client = engine.harness_login_returning("stance").expect("login");
        engine.cycle();
        let player = engine.harness_player_mut(client.pid).unwrap();
        let backpack = player.invs.get_mut(&inv).expect("backpack");
        backpack.set(0, two_handed, 1);
        backpack.set(1, sword, 1);

        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(two_handed as i32).p2_alt1(0);
        client.send(ClientProt::IfButton2, &p.finish());
        engine.cycle();
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(
            player.info.readyanim,
            Some(124),
            "the two-handed sword's stance"
        );

        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(sword as i32).p2_alt1(1);
        client.send(ClientProt::IfButton2, &p.finish());
        engine.cycle();
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.info.readyanim, Some(1381), "the sword's stance");
    });
}

/// 289's `~equip` of the elemental shield (player/scripts/equip.rs2) takes it
/// off for a tick and plays its shimmer: `shield_appear`, synth 1539 (647's
/// synth.pack, the wire id), then puts it back on.
#[test]
fn wearing_the_elemental_shield_plays_its_shimmer() {
    runner::on_loaded_engine(|engine| {
        let obj = id(engine, "elemental_shield");
        engine.harness_reset();
        let mut client = engine.harness_login_returning("elemental").expect("login");
        engine.cycle();
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let worn = *engine.cache.invs.debugnames.get("worn").unwrap();
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .expect("backpack")
            .set(0, obj, 1);
        engine.cycle();
        client.packets();
        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(obj as i32).p2_alt1(0);
        client.send(ClientProt::IfButton2, &p.finish());
        let mut packets = Vec::new();
        for _ in 0..4 {
            engine.cycle();
            packets.extend(client.packets());
        }
        let synth = ServerProt::SynthSound as u16;
        let sounds: Vec<u16> = packets
            .iter()
            .filter(|(op, _)| *op == synth)
            .map(|(_, b)| u16::from_be_bytes([b[0], b[1]]))
            .collect();
        assert!(
            sounds.contains(&1539),
            "shield_appear played; synths {sounds:?}"
        );
        let worn_shield = engine
            .harness_player(client.pid)
            .and_then(|p| p.invs.get(&worn))
            .is_some_and(|w| w.total(obj) == 1);
        assert!(worn_shield, "the shield is worn");
    });
}
