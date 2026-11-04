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
use rs_protocol::network::game::server::map_projanim::MapProjAnim;

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
    fn g1_alt1(&mut self) -> i32 {
        self.pos += 1;
        -self.b(self.pos - 1) & 0xFF
    }

    /// `128 - buffer[pos++] & 0xFF`
    fn g1_alt2(&mut self) -> i32 {
        self.pos += 1;
        (128 - self.b(self.pos - 1)) & 0xFF
    }

    /// `buffer[pos++] - 128 & 0xFF`
    fn g1_alt3(&mut self) -> i32 {
        self.pos += 1;
        (self.b(self.pos - 1) - 128) & 0xFF
    }

    /// Big-endian 32.
    fn g4(&mut self) -> i32 {
        self.pos += 4;
        (self.b(self.pos - 1) & 0xFF)
            + ((self.b(self.pos - 2) & 0xFF) << 8)
            + ((self.b(self.pos - 3) & 0xFF) << 16)
            + ((self.b(self.pos - 4) & 0xFF) << 24)
    }

    /// `buffer[pos++]`, signed
    fn g1b(&mut self) -> i32 {
        self.pos += 1;
        self.b(self.pos - 1)
    }

    /// `g2`, then `-= 0x10000` above `0x7fff`
    fn g2b(&mut self) -> i32 {
        let v = self.g2();
        if v > 0x7fff { v - 0x10000 } else { v }
    }

    /// `(b[-2] & 0xFF) << 8) + (b[-1] & 0xFF)`
    fn g2(&mut self) -> i32 {
        self.pos += 2;
        ((self.b(self.pos - 2) & 0xFF) << 8) + (self.b(self.pos - 1) & 0xFF)
    }

    /// `(b[-1] - 128 & 0xFF) + ((b[-2] & 0xFF) << 8)`
    fn g2_alt1(&mut self) -> i32 {
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
    fn ig2(&mut self) -> i32 {
        self.pos += 2;
        ((self.b(self.pos - 1) & 0xFF) << 8) + (self.b(self.pos - 2) & 0xFF)
    }

    /// `(b[-1] & 0xFF) << 8) + (b[-2] - 128 & 0xFF)`
    fn ig2_alt1(&mut self) -> i32 {
        self.pos += 2;
        ((self.b(self.pos - 1) & 0xFF) << 8) + ((self.b(self.pos - 2) - 128) & 0xFF)
    }

    /// `b[-2]<<16 | b[-1]<<24 | b[-3]<<8 | b[-4]` (little-endian-ish)
    fn ig4(&mut self) -> i32 {
        self.pos += 4;
        ((self.b(self.pos - 2) & 0xFF) << 16)
            + ((self.b(self.pos - 1) & 0xFF) << 24)
            + ((self.b(self.pos - 3) & 0xFF) << 8)
            + (self.b(self.pos - 4) & 0xFF)
    }

    /// `b[-2] | b[-3]<<24 | b[-4]<<16 | b[-1]<<8`
    fn g4_alt1(&mut self) -> i32 {
        self.pos += 4;
        (self.b(self.pos - 2) & 0xFF)
            + ((self.b(self.pos - 3) & 0xFF) << 24)
            + ((self.b(self.pos - 4) & 0xFF) << 16)
            + ((self.b(self.pos - 1) & 0xFF) << 8)
    }

    /// `b[-4]<<8 | b[-2]<<24 | b[-1]<<16 | b[-3]`
    fn g4_alt2(&mut self) -> i32 {
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
        // g1b_alt2 == g1_alt2 sign-extended
        let raw = p.g1_alt2();
        let got = if raw > 127 { raw - 256 } else { raw };
        assert_eq!(got, value, "value round-trip for {value}");
        assert_eq!(p.g2(), 1240, "varp id");
        p.done(bytes.len());
    }
}

/// Client: `g2 varp`, `g4_alt2 value`.
#[test]
fn varp_large_matches_client() {
    use rs_protocol::network::game::server::varp_large::VarpLarge;
    let bytes = encode(&VarpLarge {
        id: 1240,
        val: 0x0123_4567,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g2(), 1240);
    assert_eq!(p.g4_alt2(), 0x0123_4567);
    p.done(bytes.len());
}

// ---------------------------------------------------------------------------
// Zone headers -- the base coord is a ZONE INDEX; the client shifts it << 3.
// ---------------------------------------------------------------------------

/// Client: `g1_alt1 level`, `g1b x << 3`, `g1b_alt2 z << 3`.
#[test]
fn update_zone_partial_follows_matches_client() {
    use rs_protocol::network::game::server::update_zone_partial_follows::UpdateZonePartialFollows;
    let bytes = encode(&UpdateZonePartialFollows { x: 6, z: 9, y: 2 });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g1_alt1(), 2, "level");
    assert_eq!(p.g1() << 3, 48, "x base tiles (zone 6 -> 48)");
    assert_eq!(p.g1_alt2() << 3, 72, "z base tiles (zone 9 -> 72)");
    p.done(bytes.len());
}

/// Client: `g1_alt3 level`, `g1b x`, `g1b_alt1 z`.
#[test]
fn update_zone_full_follows_matches_client() {
    use rs_protocol::network::game::server::update_zone_full_follows::UpdateZoneFullFollows;
    let bytes = encode(&UpdateZoneFullFollows { x: 6, z: 9, y: 2 });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g1_alt3(), 2, "level");
    assert_eq!(p.g1(), 6, "x zone index");
    assert_eq!(p.g1_alt1(), 9, "z zone index");
    p.done(bytes.len());
}

// ---------------------------------------------------------------------------
// Zone payloads (ZoneProt, AClass17.java)
// ---------------------------------------------------------------------------

/// Client OBJ_ADD: `ig2_alt1 count`, `g1 coord`, `ig2_alt1 obj_id`.
#[test]
fn obj_add_matches_client() {
    use rs_protocol::network::game::server::obj_add::ObjAdd;
    let bytes = encode(&ObjAdd {
        coord: 0x35,
        id: 995,
        count: 1000,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.ig2_alt1(), 1000, "count");
    assert_eq!(p.g1(), 0x35, "packed local coord");
    assert_eq!(p.ig2_alt1(), 995, "obj id");
    p.done(bytes.len());
}

/// Client OBJ_DEL: `g1_alt1 coord`, `ig2_alt1 obj_id`.
#[test]
fn obj_del_matches_client() {
    use rs_protocol::network::game::server::obj_del::ObjDel;
    let bytes = encode(&ObjDel {
        coord: 0x35,
        id: 995,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g1_alt1(), 0x35, "packed local coord");
    assert_eq!(p.ig2_alt1(), 995, "obj id");
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

/// Client IF_OPENSUB: `ig2_alt1 sub`, `g4_alt1 parent_hash`, `g1_alt3 walkable`.
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
    assert_eq!(p.ig2_alt1(), 747, "sub interface id");
    assert_eq!(p.g4_alt1(), parent_hash, "parent component hash");
    assert_eq!(p.g1_alt3(), 1, "walkable");
    p.done(bytes.len());
}

/// Client IF_OPENTOP: `ig2 interface`, `g1_alt3 flag`.
#[test]
fn if_opentop_matches_client() {
    use rs_protocol::network::game::server::if_opentop::IfOpentop;
    let bytes = encode(&IfOpentop { interface: 746 });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.ig2(), 746, "toplevel interface id");
    assert_eq!(p.g1_alt3(), 0, "mode flag");
    p.done(bytes.len());
}

/// Client IF_SETCOLOUR: `g4_alt2 hash`, `ig2_alt1 colour`.
///
/// Regression: previously `ip4` + `p2` -- right byte count, wrong order.
#[test]
fn if_setcolour_matches_client() {
    use rs_protocol::network::game::server::if_setcolour::IfSetColour;
    let hash = (548i32 << 16) | 42;
    let bytes = encode(&IfSetColour {
        com: hash,
        colour: 0x7ABC,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g4_alt2(), hash, "component hash");
    assert_eq!(p.ig2_alt1(), 0x7ABC, "colour");
    p.done(bytes.len());
}

/// Client IF_SETPOSITION: `g4_alt1 hash`, `ig2b_alt1 x`, `ig2b_alt1 y`.
///
/// Regression: previously `ip4` + `p2` + `p2`.
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
    assert_eq!(p.g4_alt1(), hash, "component hash");
    assert_eq!(p.ig2_alt1(), 120, "x");
    assert_eq!(p.ig2_alt1(), 250, "y");
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
        assert_eq!(p.g1_alt1(), response as i32, "response {response}");
        p.done(bytes.len());
    }
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

/// Client UPDATE_STAT: `g4_alt2 xp`, `g1 skill`, `g1_alt1 level`.
#[test]
fn update_stat_matches_client() {
    use rs_protocol::network::game::server::update_stat::UpdateStat;
    let bytes = encode(&UpdateStat {
        stat: 23, // summoning
        lvl: 42,
        exp: 123_456,
    });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g4_alt2(), 123_456, "xp");
    assert_eq!(p.g1(), 23, "skill id");
    assert_eq!(p.g1_alt1(), 42, "level");
    p.done(bytes.len());
}

/// Client SET_MAP_FLAG: `g1_alt2 x`, `g1_alt2 z`.
#[test]
fn set_map_flag_matches_client() {
    use rs_protocol::network::game::server::set_map_flag::SetMapFlag;
    let bytes = encode(&SetMapFlag { x: 33, z: 61 });
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.g1_alt2(), 33, "x");
    assert_eq!(p.g1_alt2(), 61, "z");
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
        assert_eq!(p.g1_alt1(), 0x35);
        assert_eq!(p.g1(), 10 << 2 | 1);
        assert_eq!(p.ig2(), seq as i32);
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
    assert_eq!(c.g1b(), -4, "dst dx");
    assert_eq!(c.g1b(), 6, "dst dz");
    assert_eq!(c.g2b(), -43, "target");
    assert_eq!(c.g2(), 91, "spotanim");
    assert_eq!(c.g1(), 43, "src height");
    assert_eq!(c.g1(), 31, "dst height");
    assert_eq!(c.g2(), 51, "start");
    assert_eq!(c.g2(), 70, "end");
    assert_eq!(c.g1(), 16, "angle");
    assert_eq!(c.g2(), 0x1234, "distance offset");
    c.done(16);
}

// ---------------------------------------------------------------------------
// Camera, and the targeted spotanim. `MapAreaConfig`'s branches for each.
// ---------------------------------------------------------------------------

/// `AClass13.method622(const_speed, z, x, var_speed, height)` is fed
/// `g1_alt1 x; ig2_alt1 height << 2; g1_alt2 z; g1_alt1 const; g1_alt2 var`.
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
    assert_eq!(c.g1_alt1(), 40, "x");
    assert_eq!(c.ig2_alt1() << 2, 300 << 2, "height, scaled by the client");
    assert_eq!(c.g1_alt2(), 61, "z");
    assert_eq!(c.g1_alt1(), 5, "const speed");
    assert_eq!(c.g1_alt2(), 100, "var speed: 100 and up snaps");
    c.done(6);
}

/// `Class36.method269(var, x, true, const, z, height)` is fed
/// `g1_alt3 x; g1_alt1 z; g1_alt3 var; g2 height << 2; g1_alt3 const`.
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
    assert_eq!(c.g1_alt3(), 40, "x");
    assert_eq!(c.g1_alt1(), 61, "z");
    assert_eq!(c.g1_alt3(), 100, "var speed: 100 and up snaps");
    assert_eq!(c.g2() << 2, 300 << 2, "height, scaled by the client");
    assert_eq!(c.g1_alt3(), 5, "const speed");
    c.done(6);
}

/// `g1_alt1 jitter; g1 amplitude; ig2_alt1 cycle; g1 type; g1_alt3
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
    assert_eq!(c.g1_alt1(), 3, "jitter");
    assert_eq!(c.g1(), 25, "amplitude");
    assert_eq!(c.ig2_alt1(), 0, "starting cycle");
    assert_eq!(c.g1(), 4, "axis");
    assert_eq!(c.g1_alt3(), 60, "frequency");
    c.done(6);
}

/// Opcode 126 is SPOTANIM_SPECIFIC, not MAP_ANIM (109): `g1_alt2 packed;
/// g1_alt3 channel; g4 target; g2 spotanim; ig2 height; ig2_alt1 delay`.
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
    let packed = c.g1_alt2();
    assert_eq!(packed & 0x7, 2, "rotation");
    assert_eq!(packed >> 3 & 0xF, 15, "no label");
    assert_eq!(c.g1_alt3(), 1, "channel");
    let target = c.g4();
    assert!(target >> 30 == 0 && target >> 29 != 0, "an npc target");
    assert_eq!(target & 0xFFFF, 4231, "npc index");
    assert_eq!(c.g2(), 74, "spotanim");
    assert_eq!(c.ig2(), 92, "height");
    assert_eq!(c.ig2_alt1(), 30, "delay");
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
        text: "@dre@Cook@dbl@'s Assistant".into(),
    };
    let bytes = encode(&settext);
    assert_eq!(&bytes[4..], b"<col=800000>Cook<col=000080>'s Assistant\0");
    assert_eq!(settext.sizeof(), bytes.len());

    let message = MessageGame {
        kind: 0,
        text: "@str@Quest complete!".into(),
    };
    let bytes = encode(&message);
    assert_eq!(&bytes[6..], b"<str>Quest complete!\0");
    assert_eq!(message.sizeof(), bytes.len());
}

/// Client (`MapAreaConfig`, MESSAGE_GAME): `gSmart1or2 type`, `g4`, `g1
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
        text: "Bob Smith:tradereq:".into(),
    };
    let bytes = encode(&message);
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.gsmart1or2(), 100, "type");
    assert_eq!(p.g4(), 0);
    assert_eq!(p.g1(), 1, "a name, no second one");
    assert_eq!(p.gjstr(), "Bob Smith");
    assert_eq!(p.gjstr(), "wishes to trade with you.");
    p.done(bytes.len());
    assert_eq!(message.sizeof(), bytes.len());

    let plain = MessageGame {
        kind: 0,
        text: "Sending trade offer...".into(),
    };
    let bytes = encode(&plain);
    let mut p = ClientPacket::new(&bytes);
    assert_eq!(p.gsmart1or2(), 0);
    assert_eq!(p.g4(), 0);
    assert_eq!(p.g1(), 0);
    assert_eq!(p.gjstr(), "Sending trade offer...");
    p.done(bytes.len());
}

// ---------------------------------------------------------------------------
// Inventories
// ---------------------------------------------------------------------------

/// Client (`MapAreaConfig`, UPDATE_INV_FULL): `g2 inv`, `g1 flags` (bit 0:
/// the other player's copy of the inv, which trade's other offer shows),
/// `g2 size`, then per slot `g1 count` (255: `ig4`) and `g2_alt1 obj + 1`.
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
                count = p.ig4();
            }
            slots.push((p.g2_alt1() - 1, count));
        }
        assert_eq!(slots, [(995, 100_000), (-1, 0), (1351, 1)]);
        p.done(bytes.len());
    }
}

// ---------------------------------------------------------------------------
// Clientscripts
// ---------------------------------------------------------------------------

/// Client (`MapAreaConfig`, RUNCLIENTSCRIPT): `gjstr types`, then for each
/// type from the LAST to the first, `gjstr` for an `s` and `g4` otherwise into
/// that type's own position, then `g4 script`. So the type string runs in
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
            ints[i] = p.g4();
        }
    }
    assert_eq!(p.g4(), 150, "script id");
    assert_eq!(&ints[..2], &[621 << 16, 93]);
    assert_eq!(&strs[2..], &["Value".to_string(), "Sell 1".to_string()]);
    p.done(bytes.len());
}

/// 289's protect prayers (head icon bits 3, 4 and 5) as 2011's prayer icon,
/// read as the client reads the appearance (`PlayerEntity.method4076`): after
/// its length, `g1 flags, g1b title, g1b skull, g1b prayer`, -1 for none.
#[test]
fn appearance_draws_289s_protect_prayers_as_2011s_icon() {
    use rs_protocol::network::game::server::appearance::{AppearanceInput, build_appearance_block};
    let equipment = [0i16; 12];
    for (headicons, icon) in [(0u8, -1), (1 << 3, 0), (1 << 4, 1), (1 << 5, 2)] {
        let block = build_appearance_block(&AppearanceInput {
            gender: 0,
            body: [0; 7],
            equipment: &equipment,
            colors: [0; 5],
            skipped_slots: [false; 12],
            render_anim: 1426,
            display_name: "Prayer",
            combat_level: 3,
            headicons,
        });
        let mut p = ClientPacket::new(&block[1..]);
        p.g1();
        p.g1b();
        assert_eq!(p.g1b(), -1, "no skull");
        assert_eq!(p.g1b(), icon, "head icons {headicons:#b}");
    }
}
