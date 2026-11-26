//! Rev-647 interfaces: what is on screen, how modals mount, and what a click
//! on something not on screen does.
//!
//! The server keeps the client's sub-interface table (`Player::subs`),
//! updated by every IF_OPENSUB, IF_CLOSESUB and IF_MOVESUB it sends. Before
//! it did, 647 had no idea what was open: buttons on closed interfaces ran
//! their scripts, main and side modals were never sent at all, and a window
//! switch aborted at its first `if_movesub` (opcode 5012 had no handler),
//! leaving the gameframe on the old root.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::MoveSpeed;
use rs_protocol::network::game::client::window_status::WindowStatus;
use rs_test::names::INVENTORY as BACKPACK;
use rs_test::names::com;
use rs_test::{runner, server};
use rs_ver::TypeId;
use rs_vm::engine::with_engine;

const FIXED: u16 = 548;
const RESIZABLE: u16 = 746;
const MAGIC: u16 = 192;
const RUN_ORB: u16 = 750;
const BANK: u16 = 762;
const BANK_SIDE: u16 = 763;

fn fresh(engine: &mut Engine, name: &str) -> HarnessClient {
    engine.harness_reset();
    let client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    client
}

fn open(engine: &Engine, pid: u16, iface: u16) -> bool {
    engine.harness_player(pid).unwrap().is_interface_open(iface)
}

/// `IF_BUTTON1` as `Class199.method1317` writes it, with no obj or slot for a
/// plain button.
fn button1(client: &mut HarnessClient, com: i32) {
    client.if_button(1, com, 0xFFFF, TypeId::NULL);
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

#[test]
fn a_new_main_modal_clears_the_previous_background_before_it_opens() {
    runner::on_loaded_engine(|engine| {
        let mut client = fresh(engine, "iface_backdrop");
        client.packets();
        with_engine(engine, |engine| {
            let player = engine.get_player_mut(client.pid).unwrap();
            player.open_main_modal(BANK);
            player.varc_transmit(199, 0x204060);
            player.varc_transmit(213, 128);
        });
        engine.cycle();
        let changes: Vec<_> = server::varcs(&client.packets())
            .into_iter()
            .filter(|&(id, _)| id == 199 || id == 213)
            .collect();
        assert_eq!(changes, [(199, -1), (213, -1), (199, 0x204060), (213, 128)]);

        engine.harness_open_side(client.pid, BANK_SIDE);
        engine.cycle();
        assert!(
            server::varcs(&client.packets())
                .iter()
                .all(|&(id, _)| id != 199 && id != 213)
        );
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.varcs[199].as_int(), 0x204060);
        assert_eq!(player.varcs[213].as_int(), 128);

        with_engine(engine, |engine| {
            engine
                .get_player_mut(client.pid)
                .unwrap()
                .open_main_side_modal(BANK, BANK_SIDE);
        });
        engine.cycle();
        let packets = client.packets();
        let mount = packets
            .iter()
            .position(|packet| {
                server::subs(std::slice::from_ref(packet))
                    .contains(&(BANK, com("toplevel:mainmodal")))
            })
            .expect("main modal mounted");
        let backdrop: Vec<_> = server::varcs(&packets[..mount])
            .into_iter()
            .filter(|&(id, _)| id == 199 || id == 213)
            .collect();
        assert_eq!(backdrop, [(199, -1), (213, -1)]);
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
    #[cfg(before_647)]
    let orb = com("topstat_run:button");
    #[cfg(since_647)]
    let orb = com("topstat_run:run_background");
    button1(&mut client, orb);
    engine.cycle();
    assert_eq!(
        engine.harness_player(client.pid).unwrap().move_speed,
        MoveSpeed::Run,
        "the orb turns run on"
    );

    // `toplevel:topstat_run`, where `[login]` mounted the orb.
    engine.harness_close_sub(client.pid, com("toplevel:topstat_run"));
    assert!(!open(engine, client.pid, RUN_ORB));
    button1(&mut client, orb);
    engine.cycle();
    assert_eq!(
        engine.harness_player(client.pid).unwrap().move_speed,
        MoveSpeed::Run,
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
        player.subs.open.get(&com("toplevel:mainmodal")),
        Some(&(BANK, 0)),
        "the bank on 548:18, as a modal"
    );
    assert_eq!(
        player.subs.open.get(&com("toplevel:sidemodal")),
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

    client.send_msg(&WindowStatus {
        mode: 2,
        width: 1024,
        height: 768,
        antialias: 0,
    });
    engine.cycle();

    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.toplevel, RESIZABLE, "the resizable root");
    for iface in [BACKPACK, MAGIC, RUN_ORB, BANK, BANK_SIDE] {
        assert!(open(engine, client.pid, iface), "{iface} moved across");
    }
    assert_eq!(
        player
            .subs
            .open
            .get(&com("toplevel_fullscreen:mainmodal"))
            .map(|s| s.0),
        Some(BANK),
        "the bank on 746:9"
    );
    assert_eq!(
        player
            .subs
            .open
            .get(&com("toplevel_fullscreen:sidemodal"))
            .map(|s| s.0),
        Some(BANK_SIDE),
        "its side on 746:84"
    );
}
