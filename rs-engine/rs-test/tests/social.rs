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
use rs_engine::social::{ChannelMember, FriendChannel, NamedFriend, NamedIgnore};
use rs_engine::{Engine, EtherInbound, EtherOutbound};
use rs_entity::player::{CHAT_MODE_CLAN, CHAT_MODE_FRIENDS_CHAT, CHAT_MODE_PUBLIC};
use rs_pack::types::PlayerStat;
use rs_protocol::network::game::client::chat_setmode::ChatSetMode;
use rs_protocol::network::game::client::message_private::MessagePrivate;
use rs_protocol::network::game::client::message_public::MessagePublic;
use rs_protocol::network::game::client::message_quickchat_private::MessageQuickchatPrivate;
use rs_protocol::network::game::client::message_quickchat_public::MessageQuickchatPublic;
use rs_protocol::network::game::client::set_chatfiltersettings::SetChatfiltersettings;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;
use rs_util::base37::to_userhash;
use rs_ver::TypeId;
use std::iter;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

#[test]
fn social_packets_reach_their_647_handlers() {
    runner::on_loaded_engine(|engine| {
        privacy_filters(engine);
        a_filter_out_of_range_drops_the_packet(engine);
        chat_mode_routes_typed_chat(engine);
        friends_and_ignores_go_by_name(engine);
        quick_chat_fills_in_what_the_server_knows(engine);
        quick_chat_takes_what_the_dialogs_offer(engine);
        private_chat_goes_through_ether(engine);
        lists_come_named_from_ether(engine);
        friends_chat_follows_ether(engine);
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
    engine
        .cache
        .objs
        .get_by_debugname(name)
        .expect(name)
        .id
        .int()
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
    client.send_msg(&MessageQuickchatPublic {
        chat: chat as u8,
        id: TypeId::new(phrase),
        bytes: values.finish(),
    });
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
        [(
            11,
            "You are not currently in a friends chat channel.".to_string()
        )]
    );
}

/// `MESSAGE_PRIVATE` (CS2 op 5009: `pjstr(name)`, smart char count, huffman)
/// and `MESSAGE_QUICKCHAT_PRIVATE` (op 5060: `pjstr(name) p2(phrase)`, then
/// the values) go to ether by the name as the client compares names; a muted
/// sender only sends its quick chat. The echo waits for ether's verdict: the
/// filtered text once delivered, else why it wasn't.
fn private_chat_goes_through_ether(engine: &mut Engine) {
    let (mut client, mut ether, inbound) = session_with_inbound(engine, "social_private");
    let pid = client.pid;
    let me = to_userhash("social_private");
    engine.harness_player_mut(pid).unwrap().stats.base_levels[PlayerStat::Attack as usize] = 63;
    let huffman = |engine: &Engine, text: &[u8]| {
        let mut bytes = vec![0u8; text.len() * 2 + 1];
        let len = engine.cache.huffman.encode(text, &mut bytes);
        let mut chat = vec![text.len() as u8];
        chat.extend_from_slice(&bytes[..len]);
        chat
    };
    let typed = |engine: &Engine| {
        let chat = huffman(engine, b"hello");
        MessagePrivate {
            name: "Mod Ash".into(),
            chars: chat[0] as u16,
            bytes: chat[1..].to_vec(),
        }
    };
    let phrase = MessageQuickchatPrivate {
        name: "Mod Ash".into(),
        id: TypeId::new(ATTACK_LEVEL),
        bytes: vec![0],
    };
    drain(&mut ether);
    client.packets();

    client.send_msg(&typed(engine));
    engine.harness_inputs();
    let sent = drain(&mut ether);
    let (seq, payload) = match &sent[..] {
        [
            EtherOutbound::PmSend {
                sender37,
                seq,
                tier: 0,
                kind: 0,
                name,
                payload,
            },
        ] if *sender37 == me && name == "mod_ash" => (*seq, payload.clone()),
        other => panic!("one private message for ether: {other:?}"),
    };
    assert_eq!(
        payload,
        huffman(engine, b"Hello"),
        "the filtered text, capitalised"
    );
    engine.cycle();
    assert!(
        !client
            .packets()
            .iter()
            .any(|(op, _)| *op == ServerProt::MessagePrivateEcho as u16),
        "no echo before ether delivers it"
    );

    let result = |seq, code, kind, payload: Vec<u8>| EtherInbound::PmResult {
        sender37: me,
        seq,
        target37: to_userhash("Mod Ash"),
        code,
        kind,
        target: "Mod Ash".into(),
        payload,
    };
    inbound.send(result(seq, 0, 0, payload.clone())).unwrap();
    engine.cycle();
    let mut name = ClientPacket::new();
    name.pjstr("Mod Ash");
    let mut echo = name.finish();
    echo.extend(payload);
    let packets = client.packets();
    assert!(
        packets.contains(&(ServerProt::MessagePrivateEcho as u16, echo)),
        "the echo once delivered"
    );

    inbound
        .send(result(seq.wrapping_add(1), 1, 0, vec![1, 2]))
        .unwrap();
    engine.cycle();
    assert!(
        HarnessClient::game_messages(&client.packets()).contains(&(
            30,
            "That player is offline, or has privacy mode enabled.".to_string()
        )),
        "why it wasn't delivered"
    );

    engine.get_player_mut(pid).unwrap().muted_until =
        Some(Instant::now() + Duration::from_secs(60));
    client.send_msg(&typed(engine));
    engine.harness_inputs();
    engine.cycle();
    client.send_msg(&phrase);
    engine.harness_inputs();
    let sent = drain(&mut ether);
    assert!(
        matches!(&sent[..], [EtherOutbound::PmSend { kind: 1, name, payload, .. }]
            if name == "mod_ash" && payload == &[0, ATTACK_LEVEL as u8, 63]),
        "only the phrase, with the level: {sent:?}"
    );
}

/// Ether's named lists become the client's: the friends list loads even when
/// empty, worlds read "RuneScape N" and lobbies "Lobby N", and temporary
/// ignores keep their flag.
fn lists_come_named_from_ether(engine: &mut Engine) {
    let (mut client, _ether, inbound) = session_with_inbound(engine, "social_named");
    let me = to_userhash("social_named");
    client.packets();

    inbound
        .send(EtherInbound::Friends {
            target37: me,
            friends: vec![],
        })
        .unwrap();
    inbound
        .send(EtherInbound::Friends {
            target37: me,
            friends: vec![
                NamedFriend {
                    renamed: false,
                    name: "Mod Ash".into(),
                    prev: String::new(),
                    world: 1100,
                    members: false,
                    rank: 2,
                },
                NamedFriend {
                    renamed: false,
                    name: "Zezima".into(),
                    prev: "Old".into(),
                    world: 0,
                    members: false,
                    rank: 0,
                },
            ],
        })
        .unwrap();
    inbound
        .send(EtherInbound::Ignores {
            target37: me,
            full: true,
            ignores: vec![
                NamedIgnore {
                    flags: 0,
                    name: "Spammer".into(),
                    prev: String::new(),
                },
                NamedIgnore {
                    flags: 2,
                    name: "Rude".into(),
                    prev: String::new(),
                },
            ],
        })
        .unwrap();
    engine.cycle();
    let packets = client.packets();
    let friends: Vec<_> = packets
        .iter()
        .filter(|(op, _)| *op == ServerProt::UpdateFriendlist as u16)
        .map(|(_, body)| body.clone())
        .collect();
    assert_eq!(friends.len(), 2, "the empty list too");
    assert!(friends[0].is_empty());
    let mut entry = ClientPacket::new();
    entry.p1(0);
    entry.pjstr("Mod Ash");
    entry.pjstr("");
    entry.p2(1100);
    entry.p1(2);
    #[cfg(since_742)]
    entry.p1(0);
    entry.pjstr("Lobby 1");
    entry.p1(1);
    #[cfg(since_742)]
    entry.p4(0);
    let entry = entry.finish();
    assert_eq!(friends[1][..entry.len()], entry[..], "a lobby friend");

    #[cfg(before_742)]
    {
        let full: Vec<_> = packets
            .iter()
            .filter(|(op, _)| *op == ServerProt::UpdateIgnorelistFull as u16)
            .collect();
        let partial: Vec<_> = packets
            .iter()
            .filter(|(op, _)| *op == ServerProt::UpdateIgnorelistPartial as u16)
            .collect();
        assert_eq!(full.len(), 1);
        assert_eq!(full[0].1[0], 1, "the permanent ignore");
        assert_eq!(partial.len(), 1);
        assert_eq!(partial[0].1[0], 2, "then the temporary one, flagged");
    }
    #[cfg(since_742)]
    {
        let lists: Vec<_> = packets
            .iter()
            .filter(|(op, _)| *op == ServerProt::UpdateIgnoreList as u16)
            .collect();
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].1[0], 0);
    }
}

/// A friends chat as ether keeps it: the channel arrives whole, members join
/// and leave one at a time (never before the channel, which the client can't
/// take), typed chat in friends chat mode goes to ether, and its quick chat
/// counts the channel's members.
fn friends_chat_follows_ether(engine: &mut Engine) {
    let (mut client, mut ether, inbound) = session_with_inbound(engine, "social_fc");
    let me = to_userhash("social_fc");
    let member = |name: &str, world, rank| ChannelMember {
        name: name.into(),
        world,
        rank,
        combat: 100,
    };
    client.packets();

    inbound
        .send(EtherInbound::FriendchatUser {
            target37: me,
            member: member("Zezima", 1, 0),
        })
        .unwrap();
    engine.cycle();
    assert!(
        !client
            .packets()
            .iter()
            .any(|(op, _)| *op == ServerProt::UpdateFriendchatChannelSingleuser as u16),
        "no member before the channel"
    );

    inbound
        .send(EtherInbound::FriendchatChannel {
            target37: me,
            channel: Some(FriendChannel {
                owner37: to_userhash("Mod Ash"),
                owner: "Mod Ash".into(),
                channel37: to_userhash("Spoons") as i64,
                min_kick: 7,
                members: vec![member("Social Fc", 1, 0)],
            }),
        })
        .unwrap();
    inbound
        .send(EtherInbound::FriendchatUser {
            target37: me,
            member: member("Zezima", 1100, 3),
        })
        .unwrap();
    inbound
        .send(EtherInbound::FriendchatUser {
            target37: me,
            member: member("Zezima", 1100, -128),
        })
        .unwrap();
    engine.cycle();
    let ops: Vec<u16> = client.packets().into_iter().map(|(op, _)| op).collect();
    let fc: Vec<u16> = ops
        .into_iter()
        .filter(|op| {
            [
                ServerProt::UpdateFriendchatChannelFull as u16,
                ServerProt::UpdateFriendchatChannelSingleuser as u16,
            ]
            .contains(op)
        })
        .collect();
    assert_eq!(
        fc,
        [
            ServerProt::UpdateFriendchatChannelFull as u16,
            ServerProt::UpdateFriendchatChannelSingleuser as u16,
            ServerProt::UpdateFriendchatChannelSingleuser as u16
        ],
        "the channel, the join, the leave"
    );

    drain(&mut ether);
    client.send_msg(&ChatSetMode {
        mode: CHAT_MODE_FRIENDS_CHAT,
    });
    let text = b"Hello";
    let mut huffman = vec![0u8; text.len() * 2 + 1];
    let len = engine.cache.huffman.encode(text, &mut huffman);
    client.send_msg(&MessagePublic {
        colour: 0,
        effect: 0,
        chars: text.len() as u16,
        bytes: huffman[..len].to_vec(),
    });
    engine.harness_inputs();
    let sent = drain(&mut ether);
    assert!(
        matches!(&sent[..], [EtherOutbound::FriendchatSay { user37, kind: 0, .. }] if *user37 == me),
        "friends chat mode talks to ether: {sent:?}"
    );
    assert!(
        engine
            .harness_player(client.pid)
            .unwrap()
            .info
            .chat_bytes
            .is_none(),
        "and not in public chat"
    );
    engine.cycle();

    // chatphrase_1093: "There are <acc_getcount_world> amount of people in my friends chat."
    client.send(
        ClientProt::MessageQuickchatPublic,
        &[1, (1093 >> 8) as u8, 1093u16 as u8],
    );
    engine.harness_inputs();
    let sent = drain(&mut ether);
    assert!(
        matches!(&sent[..], [EtherOutbound::FriendchatSay { kind: 1, payload, .. }]
            if payload == &[(1093 >> 8) as u8, 1093u16 as u8, 1]),
        "the member count: {sent:?}"
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

/// [`session`], with a way to send the engine ether frames.
fn session_with_inbound(
    engine: &mut Engine,
    name: &str,
) -> (
    HarnessClient,
    UnboundedReceiver<EtherOutbound>,
    UnboundedSender<EtherInbound>,
) {
    let (client, rx) = session(engine, name);
    let (tx, inbound) = unbounded_channel();
    engine.ether_rx = inbound;
    (client, rx, tx)
}

fn drain(rx: &mut UnboundedReceiver<EtherOutbound>) -> Vec<EtherOutbound> {
    iter::from_fn(|| rx.try_recv().ok()).collect()
}

/// `SET_CHATFILTERSETTINGS` (CS2 op 5001): `p1(public) p1(private) p1(trade)`.
fn privacy_filters(engine: &mut Engine) {
    let (mut client, mut ether) = session(engine, "social_filters");
    drain(&mut ether);

    client.send_msg(&SetChatfiltersettings {
        public: 1,
        private: 2,
        trade: 1,
    });
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

    client.send_msg(&SetChatfiltersettings {
        public: 1,
        private: 3,
        trade: 1,
    });
    engine.harness_inputs();

    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.chat_filters, 0, "filters unchanged");
}

/// `CHAT_SETMODE` (CS2 op 5006) picks where typed chat goes. In friends chat
/// mode a message does not reach public chat, and with no channel the player
/// is told so; in clan mode with no clan it goes nowhere. A mode past the
/// guest clan chat is ignored.
fn chat_mode_routes_typed_chat(engine: &mut Engine) {
    let (mut client, _ether) = session(engine, "social_mode");
    let say = |engine: &Engine| {
        // MESSAGE_PUBLIC: colour, effect, smart char count, huffman text.
        let text = b"Hello";
        let mut huffman = vec![0u8; text.len() * 2 + 1];
        let len = engine.cache.huffman.encode(text, &mut huffman);
        MessagePublic {
            colour: 0,
            effect: 0,
            chars: text.len() as u16,
            bytes: huffman[..len].to_vec(),
        }
    };

    client.packets();
    client.send_msg(&ChatSetMode {
        mode: CHAT_MODE_FRIENDS_CHAT,
    });
    client.send_msg(&say(engine));
    engine.harness_inputs();
    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.chat_mode, CHAT_MODE_FRIENDS_CHAT, "mode set");
    assert!(
        player.info.chat_bytes.is_none(),
        "friends chat stays off public chat"
    );
    engine.cycle();
    assert!(HarnessClient::game_messages(&client.packets()).contains(&(
        11,
        "You are not currently in a friends chat channel.".to_string()
    )));

    client.send_msg(&ChatSetMode {
        mode: CHAT_MODE_CLAN,
    });
    client.send_msg(&say(engine));
    client.send_msg(&ChatSetMode { mode: 4 });
    engine.harness_inputs();
    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.chat_mode, CHAT_MODE_CLAN, "mode 4 ignored");
    assert!(
        player.info.chat_bytes.is_none(),
        "clan chat stays off public chat"
    );
    engine.cycle();

    client.send_msg(&ChatSetMode {
        mode: CHAT_MODE_PUBLIC,
    });
    client.send_msg(&say(engine));
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
    let said = say(engine);
    let mut text = vec![said.chars as u8];
    text.extend(&said.bytes);
    assert_eq!(body[5..], text[..], "char count and text");
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
/// typed (`pjstr`). Ether finds the account by its display name, so they go
/// by the name as the client compares names; a temporary ignore says so.
fn friends_and_ignores_go_by_name(engine: &mut Engine) {
    let (mut client, mut ether) = session(engine, "social_lists");
    drain(&mut ether);
    let owner = to_userhash("social_lists");

    for prot in [
        ClientProt::FriendListAdd,
        ClientProt::FriendListDel,
        ClientProt::IgnoreListAdd,
        ClientProt::IgnoreListDel,
    ] {
        let mut p = ClientPacket::new();
        p.pjstr(" Mod Ash");
        if prot == ClientProt::IgnoreListAdd {
            // `Statics1.method3487` follows the name with a flag byte.
            p.p1(1);
        }
        client.send(prot, &p.finish());
    }
    let mut nameless = ClientPacket::new();
    nameless.pjstr("__");
    client.send(ClientProt::FriendListAdd, &nameless.finish());
    engine.harness_inputs();

    let sent = drain(&mut ether);
    let expect = ["FriendAdd", "FriendDel", "IgnoreAdd", "IgnoreDel"];
    let got: Vec<&str> = sent
        .iter()
        .filter_map(|m| match m {
            EtherOutbound::FriendAddName { owner37, name }
                if *owner37 == owner && name == "mod_ash" =>
            {
                Some("FriendAdd")
            }
            EtherOutbound::FriendDelName { owner37, name }
                if *owner37 == owner && name == "mod_ash" =>
            {
                Some("FriendDel")
            }
            EtherOutbound::IgnoreAddName {
                owner37,
                name,
                temporary: true,
            } if *owner37 == owner && name == "mod_ash" => Some("IgnoreAdd"),
            EtherOutbound::IgnoreDelName { owner37, name }
                if *owner37 == owner && name == "mod_ash" =>
            {
                Some("IgnoreDel")
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        got, expect,
        "each edit names the account, and no name sends nothing: {sent:?}"
    );
}
