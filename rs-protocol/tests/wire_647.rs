//! Rev-647 wire-format regression tests.
//!
//! Every test here encodes a packet with our `ServerProtMessage::encode`, then
//! decodes the resulting bytes using a **faithful port of the rev-647 client's
//! own `Packet` readers** (the deob-647 client's `Packet.java`)
//! in the exact order the client's decode branch reads them
//! (`MapAreaConfig.java` for server prots, `AClass17.java` for ZoneProt), and
//! asserts the original field values come back.
//!
//! This is deliberately *not* a table of hand-computed expected bytes: the
//! reader port makes the client contract explicit and self-documenting, so a
//! future edit that silently swaps an `alt` variant or a field order fails
//! here instead of in-client.
//!
//! Provenance: this suite was written after an audit that found six encoders
//! with the correct byte COUNT but the wrong byte ORDER -- a class of bug that
//! neither the compiler, the CRC gate, nor a size check can catch.

#![cfg(since_647)]

use rs_io::Packet;
use rs_protocol::network::game::server::ServerProtMessage;
use rs_protocol::network::game::server::loc_anim::LocAnim;
use rs_protocol::network::game::server::loc_merge::LocMerge;
use rs_protocol::network::game::server::loc_prefetch::LocPrefetch;
use rs_protocol::network::game::server::map_projanim::MapProjAnim;
use rs_protocol::network::game::server::map_projanim_halfsq::MapProjAnimHalfsq;
use rs_protocol::network::game::server::sound_area::SoundArea;
use rs_protocol::network::game::server::sound_vorbis_area::SoundVorbisArea;
use rs_protocol::network::game::server::text_coord::TextCoord;
use rs_protocol::network::game::server_prot_message::ServerProtMessageInfo;
use rs_protocol::network::game::zone_prot::ZoneProt;

// ---------------------------------------------------------------------------
// Client `Packet` reader port. Line-for-line from the deob-647 client.
// ---------------------------------------------------------------------------

struct ClientPacket<'a> {
    buf: &'a [u8],
    pos: usize,
}

// The reader port is deliberately complete (it mirrors the client's `Packet`),
// so some readers have no caller until the matching packet gains a test.
#[allow(dead_code)]
impl<'a> ClientPacket<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn b(&self, i: usize) -> i32 {
        self.buf[i] as i8 as i32
    }

    /// `buffer[pos++] & 0xFF`
    fn g1(&mut self) -> i32 {
        self.pos += 1;
        self.b(self.pos - 1) & 0xFF
    }

    /// `-buffer[pos++] & 0xFF`
    fn g1_alt2(&mut self) -> i32 {
        self.pos += 1;
        -self.b(self.pos - 1) & 0xFF
    }

    /// `128 - buffer[pos++] & 0xFF`
    fn g1_alt3(&mut self) -> i32 {
        self.pos += 1;
        (128 - self.b(self.pos - 1)) & 0xFF
    }

    /// `buffer[pos++] - 128 & 0xFF`
    fn g1_alt1(&mut self) -> i32 {
        self.pos += 1;
        (self.b(self.pos - 1) - 128) & 0xFF
    }

    /// Big-endian 32.
    fn g4s(&mut self) -> i32 {
        self.pos += 4;
        (self.b(self.pos - 1) & 0xFF)
            + ((self.b(self.pos - 2) & 0xFF) << 8)
            + ((self.b(self.pos - 3) & 0xFF) << 16)
            + ((self.b(self.pos - 4) & 0xFF) << 24)
    }

    /// `buffer[pos++]`, signed
    fn g1s(&mut self) -> i32 {
        self.pos += 1;
        self.b(self.pos - 1)
    }

    /// `g2`, then `-= 0x10000` above `0x7fff`
    fn g2s(&mut self) -> i32 {
        let v = self.g2();
        if v > 0x7fff { v - 0x10000 } else { v }
    }

    /// `(b[-2] & 0xFF) << 8) + (b[-1] & 0xFF)`
    fn g2(&mut self) -> i32 {
        self.pos += 2;
        ((self.b(self.pos - 2) & 0xFF) << 8) + (self.b(self.pos - 1) & 0xFF)
    }

    /// `(b[-1] - 128 & 0xFF) + ((b[-2] & 0xFF) << 8)`
    fn g2_alt2(&mut self) -> i32 {
        self.pos += 2;
        ((self.b(self.pos - 1) - 128) & 0xFF) + ((self.b(self.pos - 2) & 0xFF) << 8)
    }

    /// One byte below 128, else `g2() - 32768`.
    fn gsmart1or2(&mut self) -> i32 {
        if self.buf[self.pos] >= 128 {
            self.g2() - 32768
        } else {
            self.g1()
        }
    }

    /// `(b[-1] & 0xFF) << 8) + (b[-2] & 0xFF)`
    fn g2_alt1(&mut self) -> i32 {
        self.pos += 2;
        ((self.b(self.pos - 1) & 0xFF) << 8) + (self.b(self.pos - 2) & 0xFF)
    }

    /// `(b[-1] & 0xFF) << 8) + (b[-2] - 128 & 0xFF)`
    fn g2_alt3(&mut self) -> i32 {
        self.pos += 2;
        ((self.b(self.pos - 1) & 0xFF) << 8) + ((self.b(self.pos - 2) - 128) & 0xFF)
    }

    /// `(b[-3] & 0xFF) << 16) + ((b[-2] & 0xFF) << 8) + (b[-1] & 0xFF)`
    fn g3(&mut self) -> i32 {
        self.pos += 3;
        ((self.b(self.pos - 3) & 0xFF) << 16)
            + ((self.b(self.pos - 2) & 0xFF) << 8)
            + (self.b(self.pos - 1) & 0xFF)
    }

    /// `b[-2]<<16 | b[-1]<<24 | b[-3]<<8 | b[-4]` (little-endian-ish)
    fn g4s_alt1(&mut self) -> i32 {
        self.pos += 4;
        ((self.b(self.pos - 2) & 0xFF) << 16)
            + ((self.b(self.pos - 1) & 0xFF) << 24)
            + ((self.b(self.pos - 3) & 0xFF) << 8)
            + (self.b(self.pos - 4) & 0xFF)
    }

    /// `b[-2] | b[-3]<<24 | b[-4]<<16 | b[-1]<<8`
    fn g4s_alt3(&mut self) -> i32 {
        self.pos += 4;
        (self.b(self.pos - 2) & 0xFF)
            + ((self.b(self.pos - 3) & 0xFF) << 24)
            + ((self.b(self.pos - 4) & 0xFF) << 16)
            + ((self.b(self.pos - 1) & 0xFF) << 8)
    }

    /// `b[-4]<<8 | b[-2]<<24 | b[-1]<<16 | b[-3]`
    fn g4s_alt2(&mut self) -> i32 {
        self.pos += 4;
        ((self.b(self.pos - 4) & 0xFF) << 8)
            + ((self.b(self.pos - 2) & 0xFF) << 24)
            + ((self.b(self.pos - 1) & 0xFF) << 16)
            + (self.b(self.pos - 3) & 0xFF)
    }

    /// `gjstr`: bytes up to a 0, the 0 consumed.
    fn gjstr(&mut self) -> String {
        let start = self.pos;
        while self.buf[self.pos] != 0 {
            self.pos += 1;
        }
        self.pos += 1;
        String::from_utf8_lossy(&self.buf[start..self.pos - 1]).into_owned()
    }

    fn done(&self, expected_len: usize) {
        assert_eq!(
            self.pos, expected_len,
            "client read {} bytes but the encoder wrote {}",
            self.pos, expected_len
        );
    }
}

fn encode<M: ServerProtMessage>(msg: &M) -> Vec<u8> {
    let mut pkt = Packet::new(256);
    msg.encode(&mut pkt);
    pkt.data[..pkt.pos].to_vec()
}

// ---------------------------------------------------------------------------
// Vars
// ---------------------------------------------------------------------------

/// Client: `g1b_alt2 value`, `g2 varp`.
///
/// Regression: the small-varp payload is read SIGNED, so only `-128..=127`
/// survives. Sending 200 here made the HP orb read 32740.
#[test]
fn varp_small_matches_client() {
    use rs_protocol::network::game::server::varp_small::VarpSmall;
    for value in [0i32, 1, 127, -1, -128] {
        let bytes = encode(&VarpSmall {
            id: 1240,
            val: value as u8,
        });
        let mut p = ClientPacket::new(&bytes);
        // g1b_alt2 == g1_alt3 sign-extended
        let raw = p.g1_alt3();
        let got = if raw > 127 { raw - 256 } else { raw };
        assert_eq!(got, value, "value round-trip for {value}");
        assert_eq!(p.g2(), 1240, "varp id");
        p.done(bytes.len());
    }
}

/// Client (CLIENT_SETVARCSTR_LARGE, 119, var-short): `g2_alt1 varc` (big-endian, the low byte
/// +128: this port's `g2_alt2`), `gjstr value`. A string over CLIENT_SETVARCSTR_SMALL's one-byte
/// frame goes this way: Unstable Foundations' run hint wrapped the small frame and the client
/// dropped the connection.
#[test]
fn varc_str_large_matches_client() {
    use rs_protocol::network::game::server::varc_str_large::VarcStrLarge;
    use rs_protocol::network::game::server::varc_str_small::VarcStrSmall;
    let long = "x".repeat(300);
    assert!(
        VarcStrSmall::sizeof_str(&long) > 255,
        "too long for the small frame"
    );
    let msg = VarcStrLarge {
        id: 1713,
        val: &long,
    };
    let mut pkt = Packet::new(512);
    msg.encode(&mut pkt);
    let bytes = pkt.data[..pkt.pos].to_vec();
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g2_alt2(), 1713, "varc id");
    assert_eq!(p.gjstr(), long);
    p.done(bytes.len());
    assert_eq!(msg.sizeof(), bytes.len());
}

/// Client: `g2 varp`, `g4s_alt2 value`.
#[test]
fn varp_large_matches_client() {
    use rs_protocol::network::game::server::varp_large::VarpLarge;
    let bytes = encode(&VarpLarge {
        id: 1240,
        val: 0x0123_4567,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g2(), 1240);
    assert_eq!(p.g4s_alt2(), 0x0123_4567);
    p.done(bytes.len());
}

// ---------------------------------------------------------------------------
// Zone headers -- the base coord is a ZONE INDEX; the client shifts it << 3.
// ---------------------------------------------------------------------------

/// Client: `g1_alt2 level`, `g1s x << 3`, `g1b_alt2 z << 3`.
#[test]
fn update_zone_partial_follows_matches_client() {
    use rs_protocol::network::game::server::update_zone_partial_follows::UpdateZonePartialFollows;
    let bytes = encode(&UpdateZonePartialFollows { x: 6, z: 9, y: 2 });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g1_alt2(), 2, "level");
    assert_eq!(p.g1() << 3, 48, "x base tiles (zone 6 -> 48)");
    assert_eq!(p.g1_alt3() << 3, 72, "z base tiles (zone 9 -> 72)");
    p.done(bytes.len());
}

/// Client: `g1_alt1 level`, `g1s x`, `g1b_alt1 z`.
#[test]
fn update_zone_full_follows_matches_client() {
    use rs_protocol::network::game::server::update_zone_full_follows::UpdateZoneFullFollows;
    let bytes = encode(&UpdateZoneFullFollows { x: 6, z: 9, y: 2 });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g1_alt1(), 2, "level");
    assert_eq!(p.g1(), 6, "x zone index");
    assert_eq!(p.g1_alt2(), 9, "z zone index");
    p.done(bytes.len());
}

// ---------------------------------------------------------------------------
// Zone payloads (ZoneProt, AClass17.java)
// ---------------------------------------------------------------------------

/// Client OBJ_ADD: `g2_alt3 count`, `g1 coord`, `g2_alt3 obj_id`.
#[test]
fn obj_add_matches_client() {
    use rs_protocol::network::game::server::obj_add::ObjAdd;
    let bytes = encode(&ObjAdd {
        coord: 0x35,
        id: 995,
        count: 1000,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g2_alt3(), 1000, "count");
    assert_eq!(p.g1(), 0x35, "packed local coord");
    assert_eq!(p.g2_alt3(), 995, "obj id");
    p.done(bytes.len());
}

/// Client OBJ_DEL: `g1_alt2 coord`, `g2_alt3 obj_id`.
#[test]
fn obj_del_matches_client() {
    use rs_protocol::network::game::server::obj_del::ObjDel;
    let bytes = encode(&ObjDel {
        coord: 0x35,
        id: 995,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g1_alt2(), 0x35, "packed local coord");
    assert_eq!(p.g2_alt3(), 995, "obj id");
    p.done(bytes.len());
}

/// Client OBJ_COUNT: `g1 coord`, `g2 obj_id`, `g2 old`, `g2 new`.
#[test]
fn obj_count_matches_client() {
    use rs_protocol::network::game::server::obj_count::ObjCount;
    let bytes = encode(&ObjCount {
        coord: 0x35,
        id: 995,
        old_count: 5,
        new_count: 9,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g1(), 0x35);
    assert_eq!(p.g2(), 995);
    assert_eq!(p.g2(), 5);
    assert_eq!(p.g2(), 9);
    p.done(bytes.len());
}

// ---------------------------------------------------------------------------
// Interface prots
// ---------------------------------------------------------------------------

/// Client IF_OPENSUB: `g2_alt3 sub`, `g4s_alt3 parent_hash`, `g1_alt1 walkable`.
#[test]
fn if_opensub_matches_client() {
    use rs_protocol::network::game::server::if_opensub::IfOpensub;
    // 746:177 -- the resizable summoning-orb slot.
    let parent_hash = (746i32 << 16) | 177;
    let bytes = encode(&IfOpensub {
        interface: 747,
        com: parent_hash,
        walkable: 1,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g2_alt3(), 747, "sub interface id");
    assert_eq!(p.g4s_alt3(), parent_hash, "parent component hash");
    assert_eq!(p.g1_alt1(), 1, "walkable");
    p.done(bytes.len());
}

/// Client IF_OPENTOP: `g2_alt1 interface`, `g1_alt1 flag`.
#[test]
fn if_opentop_matches_client() {
    use rs_protocol::network::game::server::if_opentop::IfOpentop;
    let bytes = encode(&IfOpentop {
        interface: 746,
        mode: 0,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g2_alt1(), 746, "toplevel interface id");
    assert_eq!(p.g1_alt1(), 0, "mode flag");
    p.done(bytes.len());
}

/// Mode 2, the one that makes the client rebuild its scene behind a "Loading -
/// please wait." screen (ImageSourceTextureOp.method3619), reads back as 2.
#[test]
fn if_opentop_mode_matches_client() {
    use rs_protocol::network::game::server::if_opentop::IfOpentop;
    let bytes = encode(&IfOpentop {
        interface: 548,
        mode: 2,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g2_alt1(), 548, "toplevel interface id");
    assert_eq!(p.g1_alt1(), 2, "mode");
    p.done(bytes.len());
}

/// Client IF_SETCOLOUR: `g4s_alt2 hash`, `g2_alt3 colour`.
///
/// Regression: previously `p4_alt1` + `p2` -- right byte count, wrong order.
#[test]
fn if_setcolour_matches_client() {
    use rs_protocol::network::game::server::if_setcolour::IfSetColour;
    let hash = (548i32 << 16) | 42;
    let bytes = encode(&IfSetColour {
        com: hash,
        colour: 0x7ABC,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g4s_alt2(), hash, "component hash");
    assert_eq!(p.g2_alt3(), 0x7ABC, "colour");
    p.done(bytes.len());
}

/// Client MINIMAP_TOGGLE (`ServerProt.java`: id 40, size 1; `MapAreaConfig.java:808`: `g1`
/// into the minimap state, 2 = blacked out, as 2011's cutscenes show it).
#[test]
fn minimap_toggle_matches_client() {
    use rs_protocol::network::game::server::minimap_toggle::MinimapToggle;
    use rs_protocol::network::game::server_prot::ServerProt;
    assert_eq!(ServerProt::MinimapToggle as u8, 40);
    let message = MinimapToggle { minimap_type: 2 };
    let bytes = encode(&message);
    assert_eq!(message.sizeof(), 1);
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g1(), 2, "minimap state");
    p.done(1);
}

/// Client VORBIS_SPEECH_SOUND (`ServerProt.java`: id 7, size 6; `MapAreaConfig.java:1921`):
/// `g2 clip` (65535 is -1), `g1 loops`, `g2 delay`, `g1 volume`, handed to
/// `AClass55.method3670(volume, 256, delay, true, loops, clip)`, which queues it on the
/// voice-over channel.
#[test]
fn vorbis_speech_sound_matches_client() {
    use rs_protocol::network::game::server::vorbis_speech_sound::VorbisSpeechSound;
    use rs_protocol::network::game::server_prot::ServerProt;
    assert_eq!(ServerProt::VorbisSpeechSound as u8, 7);
    let message = VorbisSpeechSound {
        vorbis: 0x0d8d,
        loops: 1,
        delay: 0x0102,
        volume: 255,
    };
    let bytes = encode(&message);
    assert_eq!(message.sizeof(), 6);
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g2(), 0x0d8d, "clip");
    assert_eq!(p.g1(), 1, "loops");
    assert_eq!(p.g2(), 0x0102, "delay");
    assert_eq!(p.g1(), 255, "volume");
    p.done(6);
}

/// Client VORBIS_SPEECH_STOP (`ServerProt.java`: id 66, size 0; `MapAreaConfig.java:401`):
/// reads nothing and stops every queued voice-over clip.
#[test]
fn vorbis_speech_stop_matches_client() {
    use rs_protocol::network::game::server::vorbis_speech_stop::VorbisSpeechStop;
    use rs_protocol::network::game::server_prot::ServerProt;
    assert_eq!(ServerProt::VorbisSpeechStop as u8, 66);
    assert!(encode(&VorbisSpeechStop).is_empty());
    assert_eq!(VorbisSpeechStop.sizeof(), 0);
}

/// Client IF_SETGRAPHIC (`MapAreaConfig.java`): `ig4 hash` (little-endian, this reader's
/// `g4s_alt1`), then `g2_alt1 graphic` (high byte, low byte + 128: this reader's `g2_alt2`).
#[test]
fn if_setgraphic_matches_client() {
    use rs_protocol::network::game::server::if_setgraphic::IfSetGraphic;
    let hash = (178i32 << 16) | 79;
    let bytes = encode(&IfSetGraphic {
        com: hash,
        graphic: 2379,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g4s_alt1(), hash, "component hash");
    assert_eq!(p.g2_alt2(), 2379, "graphic");
    p.done(bytes.len());
}

/// Client IF_SETPOSITION: `g4s_alt3 hash`, `ig2b_alt1 x`, `ig2b_alt1 y`.
///
/// Regression: previously `p4_alt1` + `p2` + `p2`.
#[test]
fn if_setposition_matches_client() {
    use rs_protocol::network::game::server::if_setposition::IfSetPosition;
    let hash = (548i32 << 16) | 42;
    let bytes = encode(&IfSetPosition {
        com: hash,
        x: 120,
        y: 250,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g4s_alt3(), hash, "component hash");
    assert_eq!(p.g2_alt3(), 120, "x");
    assert_eq!(p.g2_alt3(), 250, "y");
    p.done(bytes.len());
}

/// Client CREATE_CHECK_NAME: `g1b_alt1 response`.
///
/// Regression: previously a plain `p1`, so the value arrived negated.
#[test]
fn create_check_name_matches_client() {
    use rs_protocol::network::game::server::create_check_name::CreateCheckName;
    for response in [0u8, 1, 2] {
        let bytes = encode(&CreateCheckName { response });
        let mut p = ClientPacket::new(&bytes);
        assert_eq!(p.g1_alt2(), response as i32, "response {response}");
        p.done(bytes.len());
    }
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

/// Client UPDATE_STAT: `g4s_alt2 xp`, `g1 skill`, `g1_alt2 level`.
#[test]
fn update_stat_matches_client() {
    use rs_protocol::network::game::server::update_stat::UpdateStat;
    let bytes = encode(&UpdateStat {
        stat: 23, // summoning
        lvl: 42,
        exp: 123_456,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g4s_alt2(), 123_456, "xp");
    assert_eq!(p.g1(), 23, "skill id");
    assert_eq!(p.g1_alt2(), 42, "level");
    p.done(bytes.len());
}

/// Client SET_MAP_FLAG: `g1_alt3 x`, `g1_alt3 z`.
#[test]
fn set_map_flag_matches_client() {
    use rs_protocol::network::game::server::set_map_flag::SetMapFlag;
    let bytes = encode(&SetMapFlag { x: 33, z: 61 });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g1_alt3(), 33, "x");
    assert_eq!(p.g1_alt3(), 61, "z");
    p.done(bytes.len());
}

// ---------------------------------------------------------------------------
// Zone messages. `AClass17` decodes them; `ZoneProt.java` sizes them.
// ---------------------------------------------------------------------------

#[test]
fn loc_anim_matches_client() {
    for seq in [0, 733, 0x1234, u16::MAX] {
        let message = LocAnim {
            coord: 0x35,
            shape_angle: 10 << 2 | 1,
            seq,
        };
        let bytes = encode(&message);
        let mut p = ClientPacket::new(&bytes);
        assert_eq!(p.g1_alt2(), 0x35);
        assert_eq!(p.g1(), 10 << 2 | 1);
        assert_eq!(p.g2_alt1(), seq as i32);
        p.done(4);
        assert_eq!(message.sizeof(), bytes.len());
    }
}

fn projanim() -> MapProjAnim {
    MapProjAnim {
        coord: 0x35,
        dx: -4,
        dz: 6,
        target: -(42 + 1),
        spotanim: 91,
        src_height: 43,
        dst_height: 31,
        start_delay: 51,
        end_delay: 70,
        peak: 16,
        arc: 0x1234,
    }
}

/// `AClass17`'s `MAP_PROJANIM` branch. Its coord byte is not the usual
/// `x << 4 | z`: x sits in bits 3-5 under a flag in bit 7.
#[test]
fn map_projanim_matches_client() {
    let message = projanim();
    let bytes = encode(&message);
    assert_eq!(message.sizeof(), bytes.len());

    let mut c = ClientPacket::new(&bytes);
    let coord = c.g1();
    assert_eq!(coord & 0x80, 0, "flag");
    assert_eq!(coord >> 3 & 0x7, 3, "x within the zone");
    assert_eq!(coord & 0x7, 5, "z within the zone");
    assert_eq!(c.g1s(), -4, "dst dx");
    assert_eq!(c.g1s(), 6, "dst dz");
    assert_eq!(c.g2s(), -43, "target");
    assert_eq!(c.g2(), 91, "spotanim");
    assert_eq!(c.g1(), 43, "src height");
    assert_eq!(c.g1(), 31, "dst height");
    assert_eq!(c.g2(), 51, "start");
    assert_eq!(c.g2(), 70, "end");
    assert_eq!(c.g1(), 16, "angle");
    assert_eq!(c.g2(), 0x1234, "distance offset");
    c.done(16);
}

/// `AClass17`'s `MAP_PROJANIM_HALFSQ` branch: half-square coords, then a flag
/// byte, and a source before the target.
#[test]
fn map_projanim_halfsq_matches_client() {
    let message = MapProjAnimHalfsq {
        coord: 0xa7,
        ground: true,
        wearpos: Some(3),
        dx: -9,
        dz: 12,
        source: 43,
        target: -(42 + 1),
        spotanim: 91,
        src_height: -20i8 as u8,
        dst_height: 31,
        start_delay: 51,
        end_delay: 70,
        peak: 16,
        arc: 0x1234,
    };
    let bytes = encode(&message);
    assert_eq!(message.sizeof(), bytes.len());

    let mut c = ClientPacket::new(&bytes);
    let coord = c.g1();
    assert_eq!(coord >> 4 & 0xF, 10, "x half-square within the zone");
    assert_eq!(coord & 0xF, 7, "z half-square within the zone");
    let flags = c.g1();
    assert_eq!(flags & 0x1, 0x1, "ground");
    assert_eq!(flags & 0x2, 0x2, "wearpos present");
    assert_eq!(flags >> 2, 3, "wearpos");
    assert_eq!(c.g1s(), -9, "dst dx");
    assert_eq!(c.g1s(), 12, "dst dz");
    assert_eq!(c.g2s(), 43, "source");
    assert_eq!(c.g2s(), -43, "target");
    assert_eq!(c.g2(), 91, "spotanim");
    assert_eq!(c.g1s(), -20, "src height off the wearpos");
    assert_eq!(c.g1(), 31, "dst height");
    assert_eq!(c.g2(), 51, "start");
    assert_eq!(c.g2(), 70, "end");
    assert_eq!(c.g1(), 16, "angle");
    assert_eq!(c.g2(), 0x1234, "distance offset");
    c.done(19);
}

/// `AClass17`'s `LOC_PREFETCH` branch.
#[test]
fn loc_prefetch_matches_client() {
    let message = LocPrefetch {
        id: 0x1234,
        shape: 10,
    };
    let bytes = encode(&message);
    assert_eq!(message.sizeof(), bytes.len());

    let mut c = ClientPacket::new(&bytes);
    assert_eq!(c.g2(), 0x1234, "loc");
    assert_eq!(c.g1(), 10, "shape");
    c.done(3);
}

/// `AClass17`'s `TEXT_COORD` branch skips its leading length byte.
#[test]
fn text_coord_matches_client() {
    let message = TextCoord {
        coord: 0x35,
        duration: 300,
        height: 120,
        colour: 0xff9040,
        text: "Bank".to_string(),
    };
    let bytes = encode(&message);
    assert_eq!(message.sizeof(), bytes.len());

    let mut c = ClientPacket::new(&bytes);
    assert_eq!(c.g1() as usize, bytes.len() - 1, "length");
    let coord = c.g1();
    assert_eq!(coord >> 4 & 0x7, 3, "x within the zone");
    assert_eq!(coord & 0x7, 5, "z within the zone");
    assert_eq!(c.g2(), 300, "duration");
    assert_eq!(c.g1(), 120, "height");
    assert_eq!(c.g3(), 0xff9040, "colour");
    assert_eq!(c.gjstr(), "Bank");
    c.done(13);
}

/// `AClass17`'s two area sound branches read the same eight bytes. The Java
/// client's names are swapped: its `SOUND_VORBIS_AREA` (zone 10, server 72)
/// plays a synth and its `SOUND_AREA` (zone 2, server 54) a vorbis sample.
#[test]
fn sound_areas_match_client() {
    assert_eq!(ZoneProt::SoundArea as u8, 10);
    assert_eq!(SoundArea::PROT as u8, 72);
    assert_eq!(ZoneProt::SoundVorbisArea as u8, 2);
    assert_eq!(SoundVorbisArea::PROT as u8, 54);

    let area = SoundArea {
        coord: 0x35,
        sound: 2017,
        info: 5 << 4 | 2,
        delay: 30,
        volume: 200,
        speed: 256,
    };
    let vorbis = SoundVorbisArea {
        coord: 0x35,
        sound: 2017,
        info: 5 << 4 | 2,
        delay: 30,
        volume: 200,
        speed: 256,
    };
    assert_eq!(area.sizeof(), 8);
    assert_eq!(vorbis.sizeof(), 8);

    for bytes in [encode(&area), encode(&vorbis)] {
        let mut c = ClientPacket::new(&bytes);
        let coord = c.g1();
        assert_eq!(coord >> 4 & 0x7, 3, "x within the zone");
        assert_eq!(coord & 0x7, 5, "z within the zone");
        assert_eq!(c.g2(), 2017, "sound");
        let info = c.g1();
        assert_eq!(info >> 4 & 0xF, 5, "range");
        assert_eq!(info & 0x7, 2, "loops");
        assert_eq!(c.g1(), 30, "delay");
        assert_eq!(c.g1(), 200, "volume");
        assert_eq!(c.g2(), 256, "speed");
        c.done(8);
    }
}

/// `AClass17`'s `LOC_CUSTOMISE` branch reads flags (`g1_alt2`, 128 - v),
/// coord, loc, shape (`g1_alt1`, -v), then a g2 list per flag, sized by the
/// loc type. Flag 1 alone resets the loc.
#[test]
fn loc_merge_matches_client() {
    assert_eq!(ZoneProt::LocMerge as u8, 0);
    assert_eq!(LocMerge::PROT as u8, 20);

    let merge = LocMerge {
        coord: 0x35,
        id: 0x1234,
        shape_angle: 10 << 2 | 3,
        models: None,
        recol: Some(Box::new([960, 8353])),
        retex: Some(Box::new([40])),
    };
    let bytes = encode(&merge);
    assert_eq!(merge.sizeof(), bytes.len());
    let mut c = ClientPacket::new(&bytes);
    assert_eq!(c.g1_alt3(), 0x4 | 0x8, "recolours and retextures");
    let coord = c.g1();
    assert_eq!(coord >> 4 & 0x7, 3, "x within the zone");
    assert_eq!(coord & 0x7, 5, "z within the zone");
    assert_eq!(c.g2(), 0x1234, "loc");
    assert_eq!(c.g1_alt2() >> 2, 10, "shape");
    assert_eq!(c.g2(), 960, "recol 0");
    assert_eq!(c.g2(), 8353, "recol 1");
    assert_eq!(c.g2(), 40, "retex 0");
    c.done(11);

    let reset = LocMerge {
        models: None,
        recol: None,
        retex: None,
        ..merge
    };
    let bytes = encode(&reset);
    let mut c = ClientPacket::new(&bytes);
    assert_eq!(c.g1_alt3(), 1, "reset");
    c.g1();
    c.g2();
    c.g1_alt2();
    c.done(5);
}

// ---------------------------------------------------------------------------
// Camera, and the targeted spotanim. `MapAreaConfig`'s branches for each.
// ---------------------------------------------------------------------------

/// `AClass13.method622(const_speed, z, x, var_speed, height)` is fed
/// `g1_alt2 x; g2_alt3 height << 2; g1_alt3 z; g1_alt2 const; g1_alt3 var`.
#[test]
fn cam_lookat_matches_client() {
    use rs_protocol::network::game::server::cam_lookat::CamLookat;
    let bytes = encode(&CamLookat {
        local_x: 40,
        local_z: 61,
        height: 300,
        const_speed: 5,
        var_speed: 100,
    });
    let mut c = ClientPacket::new(&bytes);
    assert_eq!(c.g1_alt2(), 40, "x");
    assert_eq!(c.g2_alt3() << 2, 300 << 2, "height, scaled by the client");
    assert_eq!(c.g1_alt3(), 61, "z");
    assert_eq!(c.g1_alt2(), 5, "const speed");
    assert_eq!(c.g1_alt3(), 100, "var speed: 100 and up snaps");
    c.done(6);
}

/// `Class36.method269(var, x, true, const, z, height)` is fed
/// `g1_alt1 x; g1_alt2 z; g1_alt1 var; g2 height << 2; g1_alt1 const`.
#[test]
fn cam_moveto_matches_client() {
    use rs_protocol::network::game::server::cam_moveto::CamMoveto;
    let bytes = encode(&CamMoveto {
        local_x: 40,
        local_z: 61,
        height: 300,
        const_speed: 5,
        var_speed: 100,
    });
    let mut c = ClientPacket::new(&bytes);
    assert_eq!(c.g1_alt1(), 40, "x");
    assert_eq!(c.g1_alt2(), 61, "z");
    assert_eq!(c.g1_alt1(), 100, "var speed: 100 and up snaps");
    assert_eq!(c.g2() << 2, 300 << 2, "height, scaled by the client");
    assert_eq!(c.g1_alt1(), 5, "const speed");
    c.done(6);
}

/// `g1_alt2 jitter; g1 amplitude; g2_alt3 cycle; g1 type; g1_alt1
/// frequency`, stored per axis and read back by `Class237`'s
/// `random(-jitter..=jitter) + sin(frequency / 100 * cycle) * amplitude`.
#[test]
fn cam_shake_matches_client() {
    use rs_protocol::network::game::server::cam_shake::CamShake;
    let bytes = encode(&CamShake {
        direction: 4,
        jitter: 3,
        amplitude: 25,
        frequency: 60,
        cycle: 0,
    });
    let mut c = ClientPacket::new(&bytes);
    assert_eq!(c.g1_alt2(), 3, "jitter");
    assert_eq!(c.g1(), 25, "amplitude");
    assert_eq!(c.g2_alt3(), 0, "starting cycle");
    assert_eq!(c.g1(), 4, "axis");
    assert_eq!(c.g1_alt1(), 60, "frequency");
    c.done(6);
}

/// Opcode 126 is SPOTANIM_SPECIFIC, not MAP_ANIM (109): `g1_alt3 packed;
/// g1_alt1 channel; g4s target; g2 spotanim; g2_alt1 height; g2_alt3 delay`.
#[test]
fn spotanim_specific_matches_client() {
    use rs_protocol::network::game::server::spotanim_specific::SpotanimSpecific;
    use rs_protocol::network::game::server_prot::ServerProt;
    use rs_protocol::network::game::server_prot_message::ServerProtMessageInfo;
    assert_eq!(SpotanimSpecific::PROT as u8, 126);
    assert_eq!(ServerProt::CamSmoothreset as u8, 12);
    let npc = 1 << 29 | 4231;
    let bytes = encode(&SpotanimSpecific {
        packed: 15 << 3 | 2,
        channel: 1,
        target: npc,
        spotanim: 74,
        height: 92,
        delay: 30,
    });
    let mut c = ClientPacket::new(&bytes);
    let packed = c.g1_alt3();
    assert_eq!(packed & 0x7, 2, "rotation");
    assert_eq!(packed >> 3 & 0xF, 15, "no label");
    assert_eq!(c.g1_alt1(), 1, "channel");
    let target = c.g4s();
    assert!(target >> 30 == 0 && target >> 29 != 0, "an npc target");
    assert_eq!(target & 0xFFFF, 4231, "npc index");
    assert_eq!(c.g2(), 74, "spotanim");
    assert_eq!(c.g2_alt1(), 92, "height");
    assert_eq!(c.g2_alt3(), 30, "delay");
    c.done(12);
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

/// 289's `@xxx@` colour codes reach the 647 client as the tags its renderer
/// reads (`AClass14`: `<col=rrggbb>`, `<str>`), in interface text and game
/// messages alike, and each message's size counts the text it writes.
#[test]
fn text_carries_2011_markup() {
    use rs_protocol::network::game::server::if_settext::IfSetText;
    use rs_protocol::network::game::server::message_game::MessageGame;

    let settext = IfSetText {
        com: 275 << 16 | 2,
        text: "@dre@Cook@dbl@'s Assistant",
    };
    let bytes = encode(&settext);
    assert_eq!(&bytes[4..], b"<col=800000>Cook<col=000080>'s Assistant\0");
    assert_eq!(settext.sizeof(), bytes.len());

    let message = MessageGame {
        kind: 0,
        text: "@str@Quest complete!",
    };
    let bytes = encode(&message);
    assert_eq!(&bytes[6..], b"<str>Quest complete!\0");
    assert_eq!(message.sizeof(), bytes.len());
}

/// Client (`MapAreaConfig`, MESSAGE_GAME): `gSmart1or2 type`, `g4s`, `g1
/// mask`, a name when mask bit 0 is set (and a second when bit 1 is), then
/// the text. 289's client read a trade request off the end of a game message
/// (`<name>:tradereq:`); 2011's is message type 100 from a name, which its
/// chat box (clientscript 84) gives the op "Accept trade". A game message
/// that is no request goes as before.
#[test]
fn trade_request_is_2011s_type_100() {
    use rs_protocol::network::game::server::message_game::MessageGame;

    let message = MessageGame {
        kind: 0,
        text: "Bob Smith:tradereq:",
    };
    let bytes = encode(&message);
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.gsmart1or2(), 100, "type");
    assert_eq!(p.g4s(), 0);
    assert_eq!(p.g1(), 1, "a name, no second one");
    assert_eq!(p.gjstr(), "Bob Smith");
    assert_eq!(p.gjstr(), "wishes to trade with you.");
    p.done(bytes.len());
    assert_eq!(message.sizeof(), bytes.len());

    let plain = MessageGame {
        kind: 0,
        text: "Sending trade offer...",
    };
    let bytes = encode(&plain);
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.gsmart1or2(), 0);
    assert_eq!(p.g4s(), 0);
    assert_eq!(p.g1(), 0);
    assert_eq!(p.gjstr(), "Sending trade offer...");
    p.done(bytes.len());
}

// ---------------------------------------------------------------------------
// Inventories
// ---------------------------------------------------------------------------

/// Client (`MapAreaConfig`, UPDATE_INV_FULL): `g2 inv`, `g1 flags` (bit 0:
/// the other player's copy of the inv, which trade's other offer shows),
/// `g2 size`, then per slot `g1 count` (255: `g4s_alt1`) and `g2_alt2 obj + 1`.
#[test]
fn update_inv_full_matches_client() {
    use rs_protocol::network::game::server::update_inv_full::{InvItem, UpdateInvFull};
    for other in [false, true] {
        let inv = UpdateInvFull {
            inv_id: 90,
            other,
            capacity: 3,
            slots: vec![
                Some(InvItem {
                    id: 995,
                    count: 100_000,
                }),
                None,
                Some(InvItem { id: 1351, count: 1 }),
            ],
        };
        let bytes = encode(&inv);
        let mut p = ClientPacket::new(&bytes);
        assert_eq!(p.g2(), 90);
        assert_eq!(p.g1() & 0x1 == 1, other, "the other flag");
        assert_eq!(p.g2(), 3);
        let mut slots = Vec::new();
        for _ in 0..3 {
            let mut count = p.g1();
            if count == 255 {
                count = p.g4s_alt1();
            }
            slots.push((p.g2_alt2() - 1, count));
        }
        assert_eq!(slots, [(995, 100_000), (-1, 0), (1351, 1)]);
        p.done(bytes.len());
    }
}

// ---------------------------------------------------------------------------
// Clientscripts
// ---------------------------------------------------------------------------

/// Client (`MapAreaConfig`, RUNCLIENTSCRIPT): `gjstr types`, then for each
/// type from the LAST to the first, `gjstr` for an `s` and `g4s` otherwise into
/// that type's own position, then `g4s script`. So the type string runs in
/// argument order and the values come last argument first.
///
/// Regression: the type string went out reversed too. With one type of
/// argument that reads the same; with ints then strings (clientscript 150's
/// inventory grid: component, inv, sizes, then op names) the client read
/// strings as ints and threw in the script.
#[test]
fn run_client_script_matches_client() {
    use rs_protocol::network::game::server::run_client_script::{RunClientScript, ScriptParam};
    let bytes = encode(&RunClientScript {
        script_id: 150,
        params: vec![
            ScriptParam::Int(621 << 16),
            ScriptParam::Int(93),
            ScriptParam::Str("Value".into()),
            ScriptParam::Str("Sell 1".into()),
        ],
    });
    let mut p = ClientPacket::new(&bytes);
    let types = p.gjstr();
    assert_eq!(types, "iiss", "types in argument order");
    let mut ints = [0; 4];
    let mut strs: [String; 4] = Default::default();
    for (i, t) in types
        .chars()
        .enumerate()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        if t == 's' {
            strs[i] = p.gjstr();
        } else {
            ints[i] = p.g4s();
        }
    }
    assert_eq!(p.g4s(), 150, "script id");
    assert_eq!(&ints[..2], &[621 << 16, 93]);
    assert_eq!(&strs[2..], &["Value".to_string(), "Sell 1".to_string()]);
    p.done(bytes.len());
}

// ---------------------------------------------------------------------------
// Hint arrows
// ---------------------------------------------------------------------------

/// What the client keeps of a HINT_ARROW (`MapAreaConfig.java:1219`): `g1`
/// as `slot << 5 | type`, `g1 icon`, then by type: a tile (2-6) `g1 level,
/// g2 x, g2 z, g1 height, g2 range`; an npc (1) or player (10) `g2 index`
/// and six skipped bytes; then `g2 model`. Type 0 clears the slot and reads
/// no more.
#[derive(Debug, PartialEq)]
struct ClientHint {
    slot: i32,
    kind: i32,
    icon: i32,
    tile: Option<(i32, i32, i32, i32, i32)>,
    index: Option<i32>,
    model: i32,
}

fn read_hint(bytes: &[u8]) -> ClientHint {
    let mut p = ClientPacket::new(bytes);
    let head = p.g1();
    let (slot, kind) = (head >> 5, head & 0x1F);
    let mut hint = ClientHint {
        slot,
        kind,
        icon: -1,
        tile: None,
        index: None,
        model: -1,
    };
    if kind == 0 {
        return hint;
    }
    hint.icon = p.g1();
    if kind == 1 || kind == 10 {
        hint.index = Some(p.g2());
        p.pos += 6;
    } else if (2..=6).contains(&kind) {
        let level = p.g1();
        let x = p.g2();
        let z = p.g2();
        let height = p.g1();
        let range = p.g2();
        hint.tile = Some((level, x, z, height, range));
    }
    hint.model = p.g2();
    p.done(bytes.len());
    hint
}

/// A tile arrow carries its slot in the type byte's top three bits, its icon
/// (the colour) and its range, and each of the client's five tile offsets
/// (2 the centre, 3-6 the middle of an edge) comes back as sent.
#[test]
fn hint_arrow_tile_matches_client() {
    use rs_protocol::network::game::server::hint_arrow::{HintArrow, HintArrowKind};
    for offset in 2..=6u8 {
        let bytes = encode(&HintArrow {
            slot: 7,
            icon: 2,
            kind: HintArrowKind::Coord {
                offset,
                x: 3222,
                z: 3218,
                level: 1,
                height: 40,
                range: 65535,
            },
            model_id: 0xFFFF,
        });
        assert_eq!(bytes.len(), 12);
        let hint = read_hint(&bytes);
        assert_eq!(
            hint,
            ClientHint {
                slot: 7,
                kind: offset as i32,
                icon: 2,
                tile: Some((1, 3222, 3218, 40, 65535)),
                index: None,
                model: 0xFFFF,
            },
            "offset {offset}"
        );
    }
}

/// The arrows already sent before slots existed keep their bytes: slot 0,
/// icon 0, a tile's range 0.
#[test]
fn hint_arrow_slot_zero_keeps_its_bytes() {
    use rs_protocol::network::game::server::hint_arrow::{HintArrow, HintArrowKind};
    let tile = encode(&HintArrow {
        slot: 0,
        icon: 0,
        kind: HintArrowKind::Coord {
            offset: 2,
            x: 3094,
            z: 3106,
            level: 0,
            height: 20,
            range: 0,
        },
        model_id: 0xFFFF,
    });
    assert_eq!(
        tile,
        [2, 0, 0, 0x0C, 0x16, 0x0C, 0x22, 20, 0, 0, 0xFF, 0xFF],
        "tile arrow"
    );
    let npc = encode(&HintArrow {
        slot: 0,
        icon: 0,
        kind: HintArrowKind::Npc { npc_index: 300 },
        model_id: 0xFFFF,
    });
    assert_eq!(
        npc,
        [1, 0, 1, 44, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF],
        "npc arrow"
    );
    let player = encode(&HintArrow {
        slot: 0,
        icon: 0,
        kind: HintArrowKind::Player { player_index: 5 },
        model_id: 0xFFFF,
    });
    assert_eq!(
        player,
        [10, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF],
        "player arrow"
    );
    let stop = encode(&HintArrow {
        slot: 0,
        icon: 0,
        kind: HintArrowKind::Stop,
        model_id: 0xFFFF,
    });
    assert_eq!(stop, [0; 12], "stop");
}

/// A stop clears only its own slot.
#[test]
fn hint_arrow_stop_names_its_slot() {
    use rs_protocol::network::game::server::hint_arrow::{HintArrow, HintArrowKind};
    let bytes = encode(&HintArrow {
        slot: 7,
        icon: 0,
        kind: HintArrowKind::Stop,
        model_id: 0xFFFF,
    });
    assert_eq!(bytes.len(), 12);
    let hint = read_hint(&bytes);
    assert_eq!((hint.slot, hint.kind), (7, 0));
}

/// An npc or player arrow takes a slot and an icon as well.
#[test]
fn hint_arrow_entity_matches_client() {
    use rs_protocol::network::game::server::hint_arrow::{HintArrow, HintArrowKind};
    let npc = read_hint(&encode(&HintArrow {
        slot: 3,
        icon: 1,
        kind: HintArrowKind::Npc { npc_index: 1234 },
        model_id: 0xFFFF,
    }));
    assert_eq!(
        (npc.slot, npc.kind, npc.icon, npc.index),
        (3, 1, 1, Some(1234))
    );
    let player = read_hint(&encode(&HintArrow {
        slot: 4,
        icon: 0,
        kind: HintArrowKind::Player { player_index: 2047 },
        model_id: 0xFFFF,
    }));
    assert_eq!(
        (player.slot, player.kind, player.icon, player.index),
        (4, 10, 0, Some(2047))
    );
}
