//! 2011's combat tab answers as 289's attack tabs did.
//!
//! 289 sends one attack tab per weapon category, each with its own style
//! buttons and special bar, and turns auto retaliate on and off from its
//! controls tab. 2011 has one tab, combat_side (884), which lays itself out
//! from the worn weapon in its own scripts and reads three varps: the style
//! (%com_mode, 43, script 1134), auto retaliate (%option_nodef, 172, 1139) and
//! the special attack (%sa_attack, 301). The case clicks each of its buttons
//! as the client does (`IF_BUTTON1`) on an unarmed player.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const COMBAT_SIDE: u16 = 884;
const SPECIAL_BAR: u16 = 4;
const STYLES: [u16; 4] = [11, 12, 13, 14];
const RETALIATE: u16 = 15;

const COM_MODE: u16 = 43;
const OPTION_NODEF: u16 = 172;
const SA_ATTACK: u16 = 301;

/// `IF_BUTTON1` as `Class199.method1317` writes it: `p4(com) ip2_alt1(obj)
/// ip2(slot)`, with no obj or slot for a plain button.
fn click(engine: &mut Engine, client: &mut HarnessClient, com: u16) {
    let mut p = ClientPacket::new();
    p.p4(((COMBAT_SIDE as i32) << 16) | com as i32)
        .ip2_alt1(0xFFFF)
        .ip2(0xFFFF);
    client.send(ClientProt::IfButton1, &p.finish());
    engine.cycle();
}

fn varp(engine: &Engine, client: &HarnessClient, id: u16) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(id)
        .as_int()
}

#[test]
fn the_combat_tab_answers_its_buttons() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("brawler").expect("login");
        engine.cycle();
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(COMBAT_SIDE),
            "login opens combat_side"
        );

        // Unarmed has three styles (punch, kick, block): the fourth button
        // keeps the last, as 289's ~player_combat_stat clamps it.
        for (com, style) in STYLES.into_iter().zip([0, 1, 2, 2]) {
            click(engine, &mut client, com);
            assert_eq!(varp(engine, &client, COM_MODE), style, "style button {com}");
        }

        let retaliate = varp(engine, &client, OPTION_NODEF);
        click(engine, &mut client, RETALIATE);
        assert_ne!(varp(engine, &client, OPTION_NODEF), retaliate, "toggled");
        click(engine, &mut client, RETALIATE);
        assert_eq!(varp(engine, &client, OPTION_NODEF), retaliate, "and back");

        click(engine, &mut client, SPECIAL_BAR);
        assert_eq!(varp(engine, &client, SA_ATTACK), 1, "the special bar arms");
        click(engine, &mut client, SPECIAL_BAR);
        assert_eq!(varp(engine, &client, SA_ATTACK), 0, "and disarms");
    });
}
