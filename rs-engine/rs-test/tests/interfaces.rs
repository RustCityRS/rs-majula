//! Rev-647 interfaces: what is on screen, how modals mount, and what a click
//! on something not on screen does.
//!
//! The server keeps the client's sub-interface table (`Player::open_subs`),
//! updated by every IF_OPENSUB, IF_CLOSESUB and IF_MOVESUB it sends. Before
//! it did, 647 had no idea what was open: buttons on closed interfaces ran
//! their scripts, main and side modals were never sent at all, and a window
//! switch aborted at its first `if_movesub` (opcode 5012 had no handler),
//! leaving the gameframe on the old root.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const FIXED: u16 = 548;
const RESIZABLE: u16 = 746;
const BACKPACK: u16 = 149;
const MAGIC: u16 = 192;
const RUN_ORB: u16 = 750;
const BANK: u16 = 762;
const BANK_SIDE: u16 = 763;

fn hash(iface: u16, com: u16) -> i32 {
    ((iface as i32) << 16) | com as i32
}

fn fresh(engine: &mut Engine, name: &str) -> HarnessClient {
    engine.harness_reset();
    let client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    client
}

fn open(engine: &Engine, pid: u16, iface: u16) -> bool {
    engine.harness_player(pid).unwrap().is_interface_open(iface)
}

/// `IF_BUTTON1` as `Class199.method1317` writes it: `p4(com) ip2_alt1(obj)
/// ip2(slot)`, with no obj or slot for a plain button.
fn button1(com: i32) -> Vec<u8> {
    let mut p = ClientPacket::new();
    p.p4(com).ip2_alt1(0xFFFF).ip2(0xFFFF);
    p.finish()
}

#[test]
fn the_server_knows_what_is_on_screen() {
    runner::on_loaded_engine(|engine| {
        a_returning_login_opens_the_gameframe(engine);
        a_button_on_a_closed_interface_does_nothing(engine);
        main_and_side_modals_mount_on_their_slots(engine);
        a_window_switch_carries_everything_across(engine);
    });
}

fn a_returning_login_opens_the_gameframe(engine: &mut Engine) {
    let client = fresh(engine, "iface_login");
    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.toplevel, FIXED, "fixed mode opens toplevel");
    for iface in [BACKPACK, MAGIC, RUN_ORB] {
        assert!(
            open(engine, client.pid, iface),
            "{iface} is in the gameframe"
        );
    }
    assert!(!open(engine, client.pid, BANK), "nothing modal yet");
}

/// The run orb toggles run while it is mounted. Closed, the same click is
/// refused and run stays as it was.
fn a_button_on_a_closed_interface_does_nothing(engine: &mut Engine) {
    let mut client = fresh(engine, "iface_button");
    let orb = hash(RUN_ORB, 1);
    client.send(ClientProt::IfButton1, &button1(orb));
    engine.cycle();
    assert!(
        engine.harness_player(client.pid).unwrap().run,
        "the orb turns run on"
    );

    // `toplevel:com_orb_energy`, where `[login]` mounted the orb.
    engine.harness_close_sub(client.pid, hash(FIXED, 185));
    assert!(!open(engine, client.pid, RUN_ORB));
    client.send(ClientProt::IfButton1, &button1(orb));
    engine.cycle();
    assert!(
        engine.harness_player(client.pid).unwrap().run,
        "a click on the closed orb runs nothing"
    );
}

/// `if_openmain` mounts on the main screen and `if_openside` over the side
/// panels, and closing the modal takes both down.
fn main_and_side_modals_mount_on_their_slots(engine: &mut Engine) {
    let client = fresh(engine, "iface_modal");
    engine.harness_open_main(client.pid, BANK);
    engine.harness_open_side(client.pid, BANK_SIDE);
    engine.cycle();
    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(
        player.open_subs.get(&hash(FIXED, 18)),
        Some(&(BANK, 0)),
        "the bank on 548:18, as a modal"
    );
    assert_eq!(
        player.open_subs.get(&hash(FIXED, 198)),
        Some(&(BANK_SIDE, 1)),
        "its inventory over the side panels, 548:198, permanent as void-634's overlay_tab"
    );
    assert!(open(engine, client.pid, BANK) && open(engine, client.pid, BANK_SIDE));

    engine.harness_close_modal(client.pid);
    engine.cycle();
    assert!(!open(engine, client.pid, BANK), "closed");
    assert!(!open(engine, client.pid, BANK_SIDE), "and its side with it");
    assert!(open(engine, client.pid, BACKPACK), "the gameframe stays");
}

/// `WINDOW_STATUS` to resizable runs `[proc,switch_window_mode]`, which opens
/// the other root and moves every sub onto it.
fn a_window_switch_carries_everything_across(engine: &mut Engine) {
    let mut client = fresh(engine, "iface_switch");
    engine.harness_open_main(client.pid, BANK);
    engine.harness_open_side(client.pid, BANK_SIDE);
    engine.cycle();

    let mut p = ClientPacket::new();
    p.p1(2).p2(1024).p2(768).p1(0);
    client.send(ClientProt::WindowStatus, &p.finish());
    engine.cycle();

    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.toplevel, RESIZABLE, "the resizable root");
    for iface in [BACKPACK, MAGIC, RUN_ORB, BANK, BANK_SIDE] {
        assert!(open(engine, client.pid, iface), "{iface} moved across");
    }
    assert_eq!(
        player.open_subs.get(&hash(RESIZABLE, 9)).map(|s| s.0),
        Some(BANK),
        "the bank on 746:9"
    );
    assert_eq!(
        player.open_subs.get(&hash(RESIZABLE, 84)).map(|s| s.0),
        Some(BANK_SIDE),
        "its side on 746:84"
    );
}
