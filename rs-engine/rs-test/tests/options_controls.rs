//! 289's options and controls tabs on 2011's tabs.
//!
//! - Options (261): Toggle Number of Mouse Buttons (com6) flips varp 170, and
//!   Toggle Chat Effects (com4) varp 171, as 289's pairs set them; Open Chat
//!   Setup (com5) puts Chat Setup (982) in the tab, whose No split (com41)
//!   and private colours (com49-66) set varp 287 to 0 and 1-18, and whose
//!   Close (com5) puts the options back.
//! - Emotes (464): the Mime's Glass Wall (com29) is locked until varp 313's
//!   bit 0 is set, as 289's controls tab locks it.
//!
//! - 289's set_sail on 2011's ship journey (299, route varp 75), and 289's
//!   logout button as the logout tab's Exit to Login (182 com10).
//! - Exit to Lobby (182 com5), and five idle minutes, send LOGOUT (62): 2011's
//!   client takes a connection closed without it for a lost one and
//!   reconnects. Exit to Login (com10) sends LOGOUT_FULL (128), which the
//!   client takes to the login screen even when it came through the lobby.
//! - `::readerstall` sends one IF_SETTEXT whose body fills 2011's client
//!   packet buffer (15000 bytes): the client takes it only once its 15000-byte
//!   socket buffer is full, which empties it, so its reader thread stalls and
//!   15 s later the client reconnects.
//!
//! A tab button is IF_BUTTON1.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::player::StaffModLevel;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::names::com;
use rs_test::runner;
use rs_test::server;
use rs_ver::TypeId;

const OPTIONS: u16 = 261;
#[cfg(since_647)]
const PMSETUP: u16 = 982;
const EMOTES: u16 = 464;

fn player(engine: &mut Engine, name: &str) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    engine.harness_telejump(client.pid, 3222, 0, 3218);
    for _ in 0..12 {
        engine.cycle();
    }
    client.packets();
    client
}

fn click(engine: &mut Engine, client: &mut HarnessClient, com: i32) -> Vec<(u16, Vec<u8>)> {
    client.if_button(1, com, 0xFFFF, TypeId::NULL);
    let mut packets = Vec::new();
    for _ in 0..2 {
        engine.cycle();
        packets.extend(client.packets());
    }
    packets
}

fn varp(engine: &Engine, client: &HarnessClient, id: u16) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(id)
        .as_int()
}

fn is_open(engine: &Engine, client: &HarnessClient, iface: u16) -> bool {
    engine
        .harness_player(client.pid)
        .unwrap()
        .is_interface_open(iface)
}

fn settexts(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    server::settexts(packets)
        .into_iter()
        .map(|(_, t)| t)
        .collect()
}

#[test]
fn the_options_tab_toggles_289_s_options() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "optionsfan");
        assert!(is_open(engine, &client, OPTIONS), "the options tab");

        let mouse = varp(engine, &client, 170);
        click(engine, &mut client, com("options:optmouse"));
        assert_eq!(varp(engine, &client, 170), 1 - mouse, "mouse buttons flip");
        click(engine, &mut client, com("options:optmouse"));
        assert_eq!(varp(engine, &client, 170), mouse, "and back");

        let effects = varp(engine, &client, 171);
        click(engine, &mut client, com("options:optchat"));
        assert_eq!(varp(engine, &client, 171), 1 - effects, "chat effects flip");

        // 530 has no Chat Setup: its button toggles split private chat.
        #[cfg(before_647)]
        {
            let split = varp(engine, &client, 287);
            click(engine, &mut client, com("options:optpm"));
            assert_eq!(varp(engine, &client, 287), 1 - split, "split chat flips");
        }
        #[cfg(since_647)]
        {
            click(engine, &mut client, com("options:optpm"));
            assert!(is_open(engine, &client, PMSETUP), "Chat Setup in the tab");
            click(engine, &mut client, com("pmsetup:colour4"));
            assert_eq!(varp(engine, &client, 287), 4, "split, in the fourth colour");
            click(engine, &mut client, com("pmsetup:nosplit"));
            assert_eq!(varp(engine, &client, 287), 0, "no split");
            click(engine, &mut client, com("pmsetup:close"));
            assert!(
                is_open(engine, &client, OPTIONS),
                "Close puts the options back"
            );
            assert!(!is_open(engine, &client, PMSETUP));
        }
    });
}

#[test]
fn the_mimes_emotes_stay_locked_until_given() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "mimefan");
        assert!(is_open(engine, &client, EMOTES), "the emotes tab");
        let texts = settexts(&click(engine, &mut client, com("emotes:emote_glasswall")));
        assert!(
            texts.iter().any(|t| t == "You can't use that emote yet."),
            "Glass Wall is locked; sent {texts:?}"
        );
        engine.harness_close_modal(client.pid);
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .vars
            .set(313, VarValue::Int(1));
        let texts = settexts(&click(engine, &mut client, com("emotes:emote_glasswall")));
        assert!(
            !texts.iter().any(|t| t == "You can't use that emote yet."),
            "Glass Wall given; sent {texts:?}"
        );
    });
}

fn cheat(client: &mut HarnessClient, input: &str) {
    client.client_cheat(input);
}

fn messages(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    HarnessClient::game_messages(packets)
        .into_iter()
        .map(|(_, t)| t)
        .collect()
}

// 289's set_sail on 2011's ship journey (299): varp 75 is the route, and the
// ship lands the player with 289's arrival message.
#[test]
fn set_sail_shows_2011_s_ship_journey() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "sailor");
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .staff_mod_level = StaffModLevel::Maintainer;
        cheat(&mut client, "~sail_test");
        let mut packets = Vec::new();
        for _ in 0..2 {
            engine.cycle();
            packets.extend(client.packets());
        }
        assert!(is_open(engine, &client, 299), "the ship journey");
        assert_eq!(varp(engine, &client, 75), 2, "Entrana to Port Sarim");
        assert!(
            messages(&packets).contains(&"The ship takes you to Port Sarim.".to_string()),
            "{:?}",
            messages(&packets)
        );
        let mut texts = Vec::new();
        for _ in 0..6 {
            engine.cycle();
            texts.extend(settexts(&client.packets()));
        }
        let c = engine.harness_player(client.pid).unwrap().pathing.coord;
        assert_eq!(
            (c.x(), c.y(), c.z()),
            (3048, 1, 3231),
            "on the ship at Port Sarim"
        );
        assert!(
            texts.iter().any(|t| t == "The ship arrives at Port Sarim."),
            "{texts:?}"
        );
    });
}

fn logged_out(packets: &[(u16, Vec<u8>)]) -> bool {
    packets
        .iter()
        .any(|(op, _)| *op == ServerProt::Logout as u16)
}

// 289's logout button is 2011's Exit to Lobby (182 com5) and Exit to Login
// (com10): both log out, and not during a duel. 530's is com6.
#[test]
fn the_logout_tab_logs_out() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "leaver");
        engine.harness_open_side(client.pid, 182);
        engine.cycle();
        // 530's logout button is an if1 one: IF_BUTTON.
        #[cfg(before_647)]
        {
            client.if1_button(com("logout:com_6"));
            engine.cycle();
        }
        #[cfg(since_647)]
        click(engine, &mut client, com("logout:button2"));
        let mut gone = false;
        for _ in 0..6 {
            engine.cycle();
            if engine.harness_player(client.pid).is_none() {
                gone = true;
                break;
            }
        }
        assert!(gone, "logged out");
    });
}

#[cfg(since_647)]
#[test]
fn exit_to_lobby_sends_logout() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "lobbyist");
        engine.harness_open_side(client.pid, 182);
        engine.cycle();
        let packets = click(engine, &mut client, com("logout:button1"));
        assert!(logged_out(&packets), "told to log out");
    });
}

#[cfg(since_647)]
#[test]
fn exit_to_login_sends_logout_full() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "loginward");
        engine.harness_open_side(client.pid, 182);
        engine.cycle();
        let packets = click(engine, &mut client, com("logout:button2"));
        assert!(
            packets
                .iter()
                .any(|(op, _)| *op == ServerProt::LogoutFull as u16),
            "told to log out to the login screen"
        );
        assert!(!logged_out(&packets), "and not to the lobby");
    });
}

#[cfg(since_647)]
#[test]
fn readerstall_fills_the_client_socket_buffer() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "stallfan");
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .staff_mod_level = StaffModLevel::Maintainer;
        cheat(&mut client, "readerstall");
        engine.cycle();
        let packets = client.packets();
        let stall = packets
            .iter()
            .position(|(op, body)| *op == ServerProt::IfSetText as u16 && body.len() == 15000)
            .expect("an IF_SETTEXT whose body fills the client's 15000-byte buffer");
        assert!(
            messages(&packets[stall..])
                .iter()
                .any(|m| m.contains("::readerstall")),
            "a retry line behind it, shown only when the client didn't stall"
        );
    });
}

// 2011's client sends no idle packet, only its keepalive: five minutes (500
// ticks) without an action, the server logs the player out.
#[test]
fn an_idle_player_is_logged_out() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, "idler");
        let idle_from = engine.harness_player(client.pid).unwrap().last_active;
        let mut logout = None;
        while engine.harness_player(client.pid).is_some() && engine.clock < idle_from + 510 {
            let tick = engine.clock;
            client.send(ClientProt::NoTimeout, &[]);
            engine.cycle();
            if logged_out(&client.packets()) {
                logout.get_or_insert(tick - idle_from);
            }
        }
        assert_eq!(logout, Some(500), "told to log out five minutes on");
        assert!(engine.harness_player(client.pid).is_none(), "logged out");
    });
}
