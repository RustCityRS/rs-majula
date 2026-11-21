//! Rev-742 wire-format regression tests for the social packets.
//!
//! Each test encodes a packet and reads it back in the order the 742 gamepack's
//! handler reads it (`ms.ia`, transcribed in `rs-742/server_pseudo.txt`), or writes
//! the bytes in the order the gamepack's send site writes them and decodes them.

#![cfg(since_742)]

use rs_io::Packet;
use rs_protocol::network::game::client::ClientProtMessage;
use rs_protocol::network::game::server::ServerProtMessage;

struct ClientPacket<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> ClientPacket<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn g1(&mut self) -> i32 {
        self.pos += 1;
        self.buf[self.pos - 1] as i32
    }

    fn g1s(&mut self) -> i32 {
        self.pos += 1;
        self.buf[self.pos - 1] as i8 as i32
    }

    /// `ajf.bh`: `0 - b & 255`.
    fn g1_neg(&mut self) -> i32 {
        self.pos += 1;
        (0 - self.buf[self.pos - 1] as i32) & 0xFF
    }

    fn g2(&mut self) -> i32 {
        self.pos += 2;
        ((self.buf[self.pos - 2] as i32) << 8) + self.buf[self.pos - 1] as i32
    }

    fn g4s(&mut self) -> i32 {
        self.pos += 4;
        i32::from_be_bytes(self.buf[self.pos - 4..self.pos].try_into().unwrap())
    }

    fn g8(&mut self) -> i64 {
        self.pos += 8;
        i64::from_be_bytes(self.buf[self.pos - 8..self.pos].try_into().unwrap())
    }

    fn gjstr(&mut self) -> String {
        let start = self.pos;
        while self.buf[self.pos] != 0 {
            self.pos += 1;
        }
        self.pos += 1;
        String::from_utf8_lossy(&self.buf[start..self.pos - 1]).into_owned()
    }

    fn done(&self) {
        assert_eq!(self.pos, self.buf.len(), "client read a different length");
    }
}

fn encode<M: ServerProtMessage>(msg: &M) -> Vec<u8> {
    let mut pkt = Packet::new(256);
    msg.encode(&mut pkt);
    assert_eq!(msg.sizeof(), pkt.pos, "sizeof disagrees with encode");
    pkt.data[..pkt.pos].to_vec()
}

/// 75 CHAT_FILTER_SETTINGS: `client.sj` (trade) then `client.ss` (public), both negated.
#[test]
fn chat_filter_settings_sends_trade_first() {
    use rs_protocol::network::game::server::chat_filter_settings::ChatFilterSettings;
    let bytes = encode(&ChatFilterSettings {
        trade: 3,
        public: 1,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g1_neg(), 3, "trade");
    assert_eq!(p.g1_neg(), 1, "public");
    p.done();
}

/// 140 UPDATE_FRIENDLIST: per entry `g1 renamed, gjstr name, gjstr prev, g2 world, g1 rank,
/// g1 referred`, then `gjstr world name, g1 platform, g4s world flags` when online.
#[test]
fn update_friendlist_matches_client() {
    use rs_protocol::network::game::server::update_friendlist::{FriendEntry, UpdateFriendlist};
    let entry = |world| FriendEntry {
        renamed: false,
        name: "Mod Ash".to_string(),
        previous_name: String::new(),
        world,
        rank: 2,
        referred: false,
        world_name: "RuneScape 1".to_string(),
        platform: 1,
        world_flags: 1,
    };
    let bytes = encode(&UpdateFriendlist {
        friends: vec![entry(1), entry(0)],
    });
    let mut p = ClientPacket::new(&bytes);
    for world in [1, 0] {
        assert_eq!(p.g1(), 0);
        assert_eq!(p.gjstr(), "Mod Ash");
        assert_eq!(p.gjstr(), "");
        assert_eq!(p.g2(), world);
        assert_eq!(p.g1(), 2);
        assert_eq!(p.g1(), 0, "referred is always present");
        if world > 0 {
            assert_eq!(p.gjstr(), "RuneScape 1");
            assert_eq!(p.g1(), 1);
            assert_eq!(p.g4s(), 1);
        }
    }
    p.done();
}

/// 98 UPDATE_IGNORELIST: per entry `g1 flags` (1 rename, 2 temporary), `gjstr name`,
/// `gjstr prev`, until the end of the frame.
#[test]
fn update_ignorelist_matches_client() {
    use rs_protocol::network::game::server::update_ignorelist::{
        IGNORE_TEMPORARY, IgnoreEntry, UpdateIgnoreList,
    };
    let bytes = encode(&UpdateIgnoreList {
        entries: vec![
            IgnoreEntry {
                flags: 0,
                name: "Spammer".to_string(),
                prev_name: String::new(),
            },
            IgnoreEntry {
                flags: IGNORE_TEMPORARY,
                name: "Rude".to_string(),
                prev_name: String::new(),
            },
        ],
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g1(), 0);
    assert_eq!(p.gjstr(), "Spammer");
    assert_eq!(p.gjstr(), "");
    assert_eq!(p.g1(), 2);
    assert_eq!(p.gjstr(), "Rude");
    assert_eq!(p.gjstr(), "");
    p.done();
}

/// 23 UPDATE_FRIENDCHAT_CHANNEL_FULL: the member world is read whether or not an
/// unfiltered name follows the flag; an empty frame leaves the channel.
#[test]
fn update_friendchat_channel_full_matches_client() {
    use rs_protocol::network::game::server::update_friendchat_channel_full::{
        FriendchatChannel, FriendchatMember, UpdateFriendchatChannelFull,
    };
    let bytes = encode(&UpdateFriendchatChannelFull {
        channel: Some(FriendchatChannel {
            owner: "Mod Ash".to_string(),
            channel37: 42,
            min_kick: 2,
            members: vec![FriendchatMember {
                name: "Zezima".to_string(),
                display_name: None,
                world: 1101,
                rank: 0,
                world_name: "Lobby 2".to_string(),
            }],
        }),
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.gjstr(), "Mod Ash");
    assert_eq!(p.g1(), 0);
    assert_eq!(p.g8(), 42);
    assert_eq!(p.g1s(), 2);
    assert_eq!(p.g1(), 1);
    assert_eq!(p.gjstr(), "Zezima");
    assert_eq!(p.g1(), 0);
    assert_eq!(p.g2(), 1101);
    assert_eq!(p.g1s(), 0);
    assert_eq!(p.gjstr(), "Lobby 2");
    p.done();
    assert!(encode(&UpdateFriendchatChannelFull { channel: None }).is_empty());
}

/// 123 CLANCHANNEL_FULL (`ada.p`): flags bit 1 = display names, read before `g1s rank, g2 world`.
#[test]
fn clanchannel_full_matches_client() {
    use rs_protocol::network::game::server::clanchannel_full::{
        ClanChannelData, ClanChannelUser, ClanchannelFull,
    };
    let bytes = encode(&ClanchannelFull {
        affined: false,
        channel: Some(ClanChannelData {
            clan_hash: 9,
            update_num: 1,
            name: "Clan".to_string(),
            rank_kick: 100,
            rank_talk: -1,
            users: vec![ClanChannelUser {
                display_name: "Zezima".to_string(),
                rank: 126,
                world: 7,
            }],
        }),
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g1(), 0);
    let flags = p.g1();
    assert_eq!(flags & 2, 2, "display names present");
    assert_eq!(flags & 1, 0, "no hashes");
    assert_eq!(p.g8(), 9);
    assert_eq!(p.g8(), 1);
    assert_eq!(p.gjstr(), "Clan");
    p.g1();
    assert_eq!(p.g1s(), 100);
    assert_eq!(p.g1s(), -1);
    assert_eq!(p.g2(), 1);
    assert_eq!(p.gjstr(), "Zezima");
    assert_eq!(p.g1s(), 126);
    assert_eq!(p.g2(), 7);
    p.done();
}

/// Client 15 AFFINEDCLANSETTINGS_ADDBANNED_FROMCHANNEL (`ye.lg`): `p2 slot, pjstr name`.
#[test]
fn affinedclansettings_addbanned_fromchannel_matches_client() {
    use rs_protocol::network::game::client::affinedclansettings_addbanned_fromchannel::AffinedClansettingsAddBannedFromChannel;
    let mut bytes = vec![0x01, 0x02];
    bytes.extend_from_slice(b"Zezima\0");
    let len = bytes.len();
    assert!(AffinedClansettingsAddBannedFromChannel::validate(&bytes));
    let mut buf = Packet::from(bytes);
    let message = AffinedClansettingsAddBannedFromChannel::decode(&mut buf, len);
    assert_eq!(buf.pos, len);
    assert_eq!(message.index, 0x0102);
    assert_eq!(message.name, "Zezima");
    assert!(!AffinedClansettingsAddBannedFromChannel::validate(&[
        0x01, 0x02, b'Z'
    ]));
}
