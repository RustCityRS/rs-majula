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
use rs_pack::types::PlayerStat;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;
use rs_util::base37::to_userhash;
use std::iter;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

#[test]
fn social_packets_reach_their_647_handlers() {
    runner::on_loaded_engine(|engine| {
        privacy_filters(engine);
        a_filter_out_of_range_drops_the_packet(engine);
        chat_mode_routes_typed_chat(engine);
        friends_and_ignores_go_by_name(engine);
        quick_chat_fills_in_what_the_server_knows(engine);
        quick_chat_takes_what_the_dialogs_offer(engine);
        private_chat_echoes_to_the_sender(engine);
    });
}

/// chatphrase_8, "My Attack level is <stat_base(attack)>.": the client sends
/// a 0 for the level.
const ATTACK_LEVEL: u16 = 8;
/// chatphrase_2, "Let's go to Agility course: <listdialog(enum_1505)>?":
/// enum_1505 lists 11 courses.
const AGILITY_COURSE: u16 = 2;
/// chatphrase_209, "I like your item: <objdialog()>.".
const LIKE_ITEM: u16 = 209;
/// chatphrase_108, "Would you like to borrow: <objtradedialog()>?".
const BORROW_ITEM: u16 = 108;
/// Between the last phrase of the quickchat archive and the global archive's.
const NO_PHRASE: u16 = 1500;

fn obj(engine: &Engine, name: &str) -> i32 {
    engine.cache.objs.get_by_debugname(name).expect(name).id as i32
}

/// What a `MESSAGE_QUICKCHAT_PUBLIC` (CS2 op 5059: `p1(chat) p2(phrase)`,
/// then the phrase's dialog values) stages for broadcast, if anything.
fn quick_chat(
    engine: &mut Engine,
    client: &mut HarnessClient,
    chat: i32,
    phrase: u16,
    values: &mut ClientPacket,
) -> Option<Vec<u8>> {
    let mut p = ClientPacket::new();
    p.p1(chat).p2(phrase as i32);
    let mut payload = p.finish();
    payload.extend(values.finish());
    client.send(ClientProt::MessageQuickchatPublic, &payload);
    engine.harness_inputs();
    let staged = engine
        .harness_player(client.pid)
        .unwrap()
        .info
        .chat_bytes
        .clone();
    engine.cycle();
    staged.map(Vec::from)
}

/// The server writes the player's level over the client's 0, and the phrase
/// goes out as `MESSAGE_PUBLIC` with the colour byte's top bit set. A muted
/// player can still use quick chat.
fn quick_chat_fills_in_what_the_server_knows(engine: &mut Engine) {
    let (mut client, _ether) = session(engine, "social_quickchat");
    let pid = client.pid;
    engine.harness_player_mut(pid).unwrap().stats.base_levels[PlayerStat::Attack as usize] = 63;
    engine.get_player_mut(pid).unwrap().muted_until =
        Some(Instant::now() + Duration::from_secs(60));

    client.send(
        ClientProt::MessageQuickchatPublic,
        &[0, 0, ATTACK_LEVEL as u8, 0],
    );
    engine.harness_inputs();
    let player = engine.harness_player(pid).unwrap();
    assert_eq!(player.info.chat_colour, Some(0x80), "the phrase bit");
    assert_eq!(
        player.info.chat_bytes.as_deref(),
        Some(&[0, ATTACK_LEVEL as u8, 63][..]),
        "the phrase, then the level"
    );

    client.packets();
    engine.cycle();
    let heard: Vec<_> = client
        .packets()
        .into_iter()
        .filter(|(op, _)| *op == ServerProt::MessagePublic as u16)
        .map(|(_, body)| body)
        .collect();
    assert_eq!(
        heard,
        [vec![
            (pid >> 8) as u8,
            pid as u8,
            0x80,
            0,
            0,
            0,
            ATTACK_LEVEL as u8,
            63
        ]],
        "pid, colour and effect, rights, phrase, level"
    );
}

/// A list or obj dialog's value is one the dialog offers: a key the list
/// shows, an obj the search lists (only the stockmarket's for a trade
/// phrase, never a cert or one `enum_1547` leaves out). A stat's must be the
/// client's 0. A clan chat phrase gets 2011's line, with no clan chat to be
/// in; the clan channels' are never offered.
fn quick_chat_takes_what_the_dialogs_offer(engine: &mut Engine) {
    let (mut client, _ether) = session(engine, "social_qcvalues");
    let dagger = obj(engine, "bronze_dagger");
    let cert = obj(engine, "cert_bronze_dagger");
    let coins = obj(engine, "coins");
    let potato = obj(engine, "rotten_potato");
    let course = |key| {
        let mut p = ClientPacket::new();
        p.p2(key);
        p
    };

    let last = quick_chat(engine, &mut client, 0, AGILITY_COURSE, &mut course(10));
    assert_eq!(
        last,
        Some(vec![0, AGILITY_COURSE as u8, 0, 10]),
        "the last course"
    );
    for (phrase, values) in [
        (AGILITY_COURSE, course(11)),
        (AGILITY_COURSE, ClientPacket::new()),
        (LIKE_ITEM, course(cert)),
        (LIKE_ITEM, course(potato)),
        (BORROW_ITEM, course(coins)),
        (ATTACK_LEVEL, {
            let mut p = ClientPacket::new();
            p.p1(99);
            p
        }),
        (NO_PHRASE, ClientPacket::new()),
    ] {
        let mut values = values;
        let staged = quick_chat(engine, &mut client, 0, phrase, &mut values);
        assert_eq!(staged, None, "phrase {phrase} with {:?}", values.finish());
    }
    for (phrase, obj) in [
        (LIKE_ITEM, coins),
        (LIKE_ITEM, dagger),
        (BORROW_ITEM, dagger),
    ] {
        let staged = quick_chat(engine, &mut client, 0, phrase, &mut course(obj));
        let bytes = vec![0, phrase as u8, (obj >> 8) as u8, obj as u8];
        assert_eq!(staged, Some(bytes), "phrase {phrase}, obj {obj}");
    }

    client.packets();
    let clan = quick_chat(engine, &mut client, 1, AGILITY_COURSE, &mut course(0));
    let channel = quick_chat(engine, &mut client, 2, AGILITY_COURSE, &mut course(0));
    assert_eq!((clan, channel), (None, None), "neither reaches public chat");
    let clan_lines: Vec<_> = HarnessClient::game_messages(&client.packets())
        .into_iter()
        .filter(|(kind, _)| *kind == 11)
        .collect();
    assert_eq!(
        clan_lines,
        [(11, "You must be in a clan chat to talk.".to_string())]
    );
}

/// `MESSAGE_PRIVATE` (CS2 op 5009: `pjstr(name)`, smart char count, huffman)
/// and `MESSAGE_QUICKCHAT_PRIVATE` (op 5060: `pjstr(name) p2(phrase)`, then
/// the values) name a friend, who gets nothing yet: ether relays no 647
/// chat. The sender gets the filtered message back; a muted sender only its
/// quick chat. One for a name not on the friends list is dropped.
fn private_chat_echoes_to_the_sender(engine: &mut Engine) {
    let (mut client, mut ether) = session(engine, "social_private");
    let pid = client.pid;
    engine.harness_player_mut(pid).unwrap().stats.base_levels[PlayerStat::Attack as usize] = 63;
    let huffman = |engine: &Engine, text: &[u8]| {
        let mut bytes = vec![0u8; text.len() * 2 + 1];
        let len = engine.cache.huffman.encode(text, &mut bytes);
        let mut chat = vec![text.len() as u8];
        chat.extend_from_slice(&bytes[..len]);
        chat
    };
    let typed = |engine: &Engine| {
        let mut p = ClientPacket::new();
        p.pjstr("Mod Ash");
        let mut payload = p.finish();
        payload.extend(huffman(engine, b"hello"));
        payload
    };
    let mut phrase = ClientPacket::new();
    phrase.pjstr("Mod Ash").p2(ATTACK_LEVEL as i32).p1(0);
    let mut name = ClientPacket::new();
    name.pjstr("Mod Ash");
    let echoes = |client: &mut HarnessClient, engine: &mut Engine| {
        engine.harness_inputs();
        engine.cycle();
        client
            .packets()
            .into_iter()
            .filter(|(op, _)| {
                [
                    ServerProt::MessagePrivateEcho as u16,
                    ServerProt::MessageQuickchatPrivateEcho as u16,
                ]
                .contains(op)
            })
            .collect::<Vec<_>>()
    };
    client.packets();

    client.send(ClientProt::MessagePrivate, &typed(engine));
    client.send(ClientProt::MessageQuickchatPrivate, &phrase.finish());
    assert!(echoes(&mut client, engine).is_empty(), "not a friend");

    engine
        .get_player_mut(pid)
        .unwrap()
        .friends
        .push(to_userhash("Mod Ash") as i64);
    client.send(ClientProt::MessagePrivate, &typed(engine));
    let mut body = name.finish();
    body.extend(huffman(engine, b"Hello"));
    assert_eq!(
        echoes(&mut client, engine),
        [(ServerProt::MessagePrivateEcho as u16, body)],
        "the name, then the text capitalised"
    );

    engine.get_player_mut(pid).unwrap().muted_until =
        Some(Instant::now() + Duration::from_secs(60));
    client.send(ClientProt::MessagePrivate, &typed(engine));
    client.send(ClientProt::MessageQuickchatPrivate, &phrase.finish());
    let mut body = name.finish();
    body.extend([0, ATTACK_LEVEL as u8, 63]);
    assert_eq!(
        echoes(&mut client, engine),
        [(ServerProt::MessageQuickchatPrivateEcho as u16, body)],
        "only the phrase, with the level"
    );
    assert!(
        !drain(&mut ether)
            .iter()
            .any(|m| matches!(m, EtherOutbound::PrivateMessage { .. })),
        "nothing for ether"
    );
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
