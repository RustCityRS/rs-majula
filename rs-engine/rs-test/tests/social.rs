//! Rev-647 chat and social packets reach their 647 handlers.
//!
//! Each case writes the frame as the 647 client's send site does, queues it on
//! a harness player, and checks what the handler leaves on the player and
//! sends to ether.
//!
//! Until the engine dispatched these with their 647 structs it decoded them
//! with the 225-era ones. `ChatSetMode` read three filter bytes out of its
//! one-byte chat-mode frame, the filters' own packet (`SetChatfiltersettings`)
//! hit the unhandled catch-all, and a friend or ignore edit read a typed name
//! as an 8-byte hash.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::harness::HarnessClient;
use rs_engine::{Engine, EtherOutbound};
use rs_entity::player::{CHAT_MODE_CLAN, CHAT_MODE_PUBLIC};
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;
use rs_util::base37::to_userhash;
use std::iter;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

#[test]
fn social_packets_reach_their_647_handlers() {
    runner::on_loaded_engine(|engine| {
        privacy_filters(engine);
        a_filter_out_of_range_drops_the_packet(engine);
        chat_mode_routes_typed_chat(engine);
        friends_and_ignores_go_by_name(engine);
    });
}

/// A fresh world with one returning player in it, and the engine's ether
/// traffic captured.
fn session(engine: &mut Engine, name: &str) -> (HarnessClient, UnboundedReceiver<EtherOutbound>) {
    engine.harness_reset();
    let (tx, rx) = unbounded_channel();
    engine.ether_tx = Some(tx);
    let client = engine.harness_login_returning(name).expect("login");
    (client, rx)
}

fn drain(rx: &mut UnboundedReceiver<EtherOutbound>) -> Vec<EtherOutbound> {
    iter::from_fn(|| rx.try_recv().ok()).collect()
}

/// `SET_CHATFILTERSETTINGS` (CS2 op 5001): `p1(public) p1(private) p1(trade)`.
fn privacy_filters(engine: &mut Engine) {
    let (mut client, mut ether) = session(engine, "social_filters");
    drain(&mut ether);

    client.send(ClientProt::SetChatfiltersettings, &[1, 2, 1]);
    engine.harness_inputs();

    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(
        (
            player.public_chat(),
            player.private_chat(),
            player.trade_chat()
        ),
        (1, 2, 1),
        "public friends, private off, trade friends"
    );
    let sent = drain(&mut ether);
    assert!(
        sent.iter().any(|m| matches!(m,
            EtherOutbound::ChatModeUpdate { user37, private_mode: 2 }
                if *user37 == to_userhash("social_filters"))),
        "the private filter goes to ether so friends see the change: {sent:?}"
    );
}

/// Public chat has four filters (0..=3), private and trade three. One value
/// out of range drops the whole packet, as it did before 647.
fn a_filter_out_of_range_drops_the_packet(engine: &mut Engine) {
    let (mut client, _ether) = session(engine, "social_range");

    client.send(ClientProt::SetChatfiltersettings, &[1, 3, 1]);
    engine.harness_inputs();

    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.chat_filters, 0, "filters unchanged");
}

/// `CHAT_SETMODE` (CS2 op 5006) picks where typed chat goes. In clan mode a
/// message does not reach public chat: there is no clan chat to be in.
fn chat_mode_routes_typed_chat(engine: &mut Engine) {
    let (mut client, _ether) = session(engine, "social_mode");
    let say = |engine: &Engine| {
        // MESSAGE_PUBLIC: colour, effect, smart char count, huffman text.
        let text = b"Hello";
        let mut huffman = vec![0u8; text.len() * 2 + 1];
        let len = engine.cache.huffman.encode(text, &mut huffman);
        let mut payload = vec![0, 0, text.len() as u8];
        payload.extend_from_slice(&huffman[..len]);
        payload
    };

    client.send(ClientProt::ChatSetMode, &[CHAT_MODE_CLAN]);
    client.send(ClientProt::MessagePublic, &say(engine));
    engine.harness_inputs();
    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.chat_mode, CHAT_MODE_CLAN, "mode set");
    assert!(
        player.info.chat_bytes.is_none(),
        "clan-mode chat stays off public chat"
    );

    client.send(ClientProt::ChatSetMode, &[CHAT_MODE_PUBLIC]);
    client.send(ClientProt::MessagePublic, &say(engine));
    engine.harness_inputs();
    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.chat_mode, CHAT_MODE_PUBLIC, "mode set back");
    assert!(
        player.info.chat_bytes.is_some(),
        "public-mode chat is staged for broadcast"
    );

    client.packets();
    engine.cycle();
    let packets = client.packets();
    let heard: Vec<_> = packets
        .iter()
        .filter(|(op, _)| *op == ServerProt::MessagePublic as u16)
        .collect();
    assert_eq!(heard.len(), 1, "the speaker hears its own chat once");
    let body = &heard[0].1;
    let pid = client.pid;
    assert_eq!(
        body[..4],
        [(pid >> 8) as u8, pid as u8, 0, 0],
        "pid, colour, effect"
    );
    assert_eq!(body[5..], say(engine)[2..], "char count and text");
    assert!(
        engine
            .harness_player(pid)
            .unwrap()
            .info
            .chat_bytes
            .is_none(),
        "and it is gone with the tick"
    );
}

/// `FRIENDLIST_ADD` / `_DEL` and `IGNORELIST_ADD` / `_DEL` carry the name as
/// typed (`pjstr`). Ether is keyed by the base-37 hash of the display name.
fn friends_and_ignores_go_by_name(engine: &mut Engine) {
    let (mut client, mut ether) = session(engine, "social_lists");
    drain(&mut ether);
    let owner = to_userhash("social_lists");
    let other = to_userhash("Mod Ash");

    for prot in [
        ClientProt::FriendListAdd,
        ClientProt::FriendListDel,
        ClientProt::IgnoreListAdd,
        ClientProt::IgnoreListDel,
    ] {
        let mut p = ClientPacket::new();
        p.pjstr("Mod Ash");
        if prot == ClientProt::IgnoreListAdd {
            // `Statics1.method3487` follows the name with a flag byte.
            p.p1(0);
        }
        client.send(prot, &p.finish());
    }
    engine.harness_inputs();

    let sent = drain(&mut ether);
    let expect = ["FriendAdd", "FriendDel", "IgnoreAdd", "IgnoreDel"];
    let got: Vec<&str> = sent
        .iter()
        .filter_map(|m| match m {
            EtherOutbound::FriendAdd { owner37, friend37 }
                if (*owner37, *friend37) == (owner, other) =>
            {
                Some("FriendAdd")
            }
            EtherOutbound::FriendDel { owner37, friend37 }
                if (*owner37, *friend37) == (owner, other) =>
            {
                Some("FriendDel")
            }
            EtherOutbound::IgnoreAdd { owner37, ignore37 }
                if (*owner37, *ignore37) == (owner, other) =>
            {
                Some("IgnoreAdd")
            }
            EtherOutbound::IgnoreDel { owner37, ignore37 }
                if (*owner37, *ignore37) == (owner, other) =>
            {
                Some("IgnoreDel")
            }
            _ => None,
        })
        .collect();
    assert_eq!(got, expect, "each edit names the right account: {sent:?}");
}
