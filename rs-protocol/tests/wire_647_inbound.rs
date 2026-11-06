//! Rev-647 **inbound** wire-format regression tests.
//!
//! The sibling suite `wire_647.rs` covers the outbound direction: our
//! `ServerProtMessage::encode` read back by a port of the client's `Packet`
//! reader. This one is its mirror. Each test writes a packet's bytes with a
//! **faithful port of the rev-647 client's `Packet` writer**
//! (the deob-647 client's `Packet.java`) in the exact order the
//! client's send site writes them, hands the bytes to our
//! `ClientProtMessage::decode`, and asserts the field values come back.
//!
//! Why this exists: a decoder can read the right *number* of bytes with every
//! field pointing at the wrong one, and nothing catches it. `IfButtonD` shipped
//! that way -- source and target component, slot and obj were all transposed
//! against `SceneGraphNode_Sub6_Sub1.method3715`. A same-inventory drag hid it
//! because a slot swap is symmetric in its own arguments, so it survived
//! in-client testing and would only have surfaced once banking made the two
//! endpoints differ. Neither the compiler, a size check, nor the CRC gate can
//! see a bug of that shape; reading the client's own write order back does.
//!
//! Each test names the client file and method it was transcribed from. When a
//! test fails, the client is right and we are wrong: go read the cited method.

#![cfg(since_647)]

use rs_io::Packet;
use rs_protocol::network::game::client::ClientProtMessage;
use rs_protocol::network::game::client::move_minimapclick::{MinimapTrailer, MoveMinimapClick};

// The client `Packet` writer port, shared with the headless handler tests.
#[path = "support/client_packet.rs"]
mod client_packet;
use client_packet::ClientPacket;

/// Runs a decoder over bytes the client would have written.
fn decode<M: ClientProtMessage>(p: &ClientPacket) -> M {
    let bytes = p.finish();
    let len = bytes.len();
    let mut buf = Packet::from(bytes);
    assert!(M::validate(&buf.data));
    let msg = M::decode(&mut buf, len);
    assert_eq!(
        buf.pos, len,
        "decoder consumed {} of {} bytes -- a decoder that stops short or overruns \
         will desynchronise every packet after it in the same read",
        buf.pos, len
    );
    msg
}

// ---------------------------------------------------------------------------
// Interface drag. `SceneGraphNode_Sub6_Sub1.method3715`.
// ---------------------------------------------------------------------------

/// The packet that was wrong. `method3715(target, source)` writes the **source**
/// component and slot first, then the **target** slot and component, then the
/// source obj and the target obj -- an order that reads backwards next to the
/// argument list, which is how it came to be transposed.
#[test]
fn if_buttond_matches_client() {
    use rs_protocol::network::game::client::if_buttond::IfButtonD;

    // A backpack drag: slot 3 onto slot 20 of interface 149's inventory.
    // Both ends are component 0 of interface 149.
    let source_com = 149 << 16;
    let target_com = 149 << 16;
    let mut p = ClientPacket::new();
    p.p4(source_com) // arg1.id      -- source component
        .p2_alt3(3) // arg1.child_id -- source slot
        .p2_alt3(20 + 28) // arg0.child_id -- target slot (bank 2; see login.rs2)
        .p4_alt2(target_com) // arg0.id       -- target component
        .p2_alt1(1511) // arg1.object_id -- source obj
        .p2(-1); // arg0.object_id -- target obj (empty backing sprite)

    let m: IfButtonD = decode(&p);
    assert_eq!(m.com, source_com as u32, "source component");
    assert_eq!(m.slot, 3, "source slot");
    assert_eq!(m.slot2, 48, "target slot");
    assert_eq!(m.com2, target_com as u32, "target component");
    assert_eq!(m.obj, 1511, "source obj");
}

// ---------------------------------------------------------------------------
// Loc interaction. `Statics1.do_action`, the OP_LOC1..5 branches.
// ---------------------------------------------------------------------------

/// `p2_alt1(z)`, `p2_alt1(locId)`, `p1(ctrl)`, `p2_alt2(x)`. Note the loc id is the
/// **high** word of the client's 64-bit menu key, so a send built from a bare
/// loc id puts zero here.
#[test]
fn oploc1_matches_client() {
    use rs_protocol::network::game::client::oploc1::OpLoc1;

    let mut p = ClientPacket::new();
    p.p2_alt1(3218).p2_alt1(1530).p1(1).p2_alt2(3222);

    let m: OpLoc1 = decode(&p);
    assert_eq!(m.z, 3218, "z");
    assert_eq!(m.loc, 1530, "loc id");
    assert!(m.ctrl, "ctrl-run");
    assert_eq!(m.x, 3222, "x");
}

// ---------------------------------------------------------------------------
// Window status. `client.method4102` / the resize handler.
// ---------------------------------------------------------------------------

/// `p1(mode)`, `p2(width)`, `p2(height)`, `p1(unused)`. Mode is the client's
/// own: 1 fixed, 2 resizable, 3 fullscreen.
#[test]
fn window_status_matches_client() {
    use rs_protocol::network::game::client::window_status::WindowStatus;

    let mut p = ClientPacket::new();
    p.p1(2).p2(1008).p2(729).p1(0);

    let m: WindowStatus = decode(&p);
    assert_eq!(m.mode, 2, "window mode");
    assert_eq!(m.width, 1008, "width");
    assert_eq!(m.height, 729, "height");
}

// ---------------------------------------------------------------------------
// Op families. `Statics1.do_action`, one branch per MiniMenuAction.
//
// Every op carries the same two or four values -- ctrl-run, an entity id, and
// for loc/obj a world tile -- but each picks a different permutation of field
// order and byte-order variant. There is no rule to derive them from, which is
// why they are transcribed one at a time rather than generated.
// ---------------------------------------------------------------------------

/// An op carrying only ctrl-run and an entity id.
macro_rules! entity_op {
    ($test:ident, $module:ident :: $ty:ident, $field:ident, $writes:expr) => {
        #[test]
        fn $test() {
            use rs_protocol::network::game::client::$module::$ty;
            let mut p = ClientPacket::new();
            let w: fn(&mut ClientPacket) = $writes;
            w(&mut p);
            let m: $ty = decode(&p);
            assert_eq!(m.$field, 4231, stringify!($field));
            assert!(m.ctrl, "ctrl-run");
        }
    };
}

entity_op!(opnpc1_matches_client, opnpc1::OpNpc1, nid, |p| {
    p.p1_alt3(1).p2(4231);
});
entity_op!(opnpc2_matches_client, opnpc2::OpNpc2, nid, |p| {
    p.p1_alt2(1).p2_alt3(4231);
});
entity_op!(opnpc3_matches_client, opnpc3::OpNpc3, nid, |p| {
    p.p1_alt3(1).p2_alt2(4231);
});
entity_op!(opnpc4_matches_client, opnpc4::OpNpc4, nid, |p| {
    p.p1_alt3(1).p2_alt3(4231);
});
entity_op!(opnpc5_matches_client, opnpc5::OpNpc5, nid, |p| {
    p.p1_alt2(1).p2_alt1(4231);
});

entity_op!(opplayer1_matches_client, opplayer1::OpPlayer1, pid, |p| {
    p.p1_alt1(1).p2_alt2(4231);
});
entity_op!(opplayer2_matches_client, opplayer2::OpPlayer2, pid, |p| {
    p.p1_alt2(1).p2(4231);
});
entity_op!(opplayer3_matches_client, opplayer3::OpPlayer3, pid, |p| {
    p.p2_alt1(4231).p1(1);
});
entity_op!(opplayer4_matches_client, opplayer4::OpPlayer4, pid, |p| {
    p.p2_alt2(4231).p1(1);
});
entity_op!(opplayer5_matches_client, opplayer5::OpPlayer5, pid, |p| {
    p.p1_alt1(1).p2_alt1(4231);
});
entity_op!(opplayer6_matches_client, opplayer6::OpPlayer6, pid, |p| {
    p.p1(1).p2_alt2(4231);
});
entity_op!(opplayer7_matches_client, opplayer7::OpPlayer7, pid, |p| {
    p.p1_alt2(1).p2(4231);
});
entity_op!(opplayer8_matches_client, opplayer8::OpPlayer8, pid, |p| {
    p.p1(1).p2_alt1(4231);
});
entity_op!(opplayer9_matches_client, opplayer9::OpPlayer9, pid, |p| {
    p.p2(4231).p1_alt1(1);
});
entity_op!(
    opplayer10_matches_client,
    opplayer10::OpPlayer10,
    pid,
    |p| {
        p.p1_alt2(1).p2(4231);
    }
);

/// An op carrying ctrl-run, an entity id and a world tile.
macro_rules! tile_op {
    ($test:ident, $module:ident :: $ty:ident, $field:ident, $writes:expr) => {
        #[test]
        fn $test() {
            use rs_protocol::network::game::client::$module::$ty;
            let mut p = ClientPacket::new();
            let w: fn(&mut ClientPacket) = $writes;
            w(&mut p);
            let m: $ty = decode(&p);
            assert_eq!(m.$field, 1530, stringify!($field));
            assert_eq!(m.x, 3222, "x");
            assert_eq!(m.z, 3218, "z");
            assert!(m.ctrl, "ctrl-run");
        }
    };
}

// Loc ops. `oploc4` and `oploc5` write x and the loc id with the *same*
// variant back to back, so only the field assignment tells them apart -- a
// transposition there survives any check that looks at byte layout alone.
tile_op!(oploc2_matches_client, oploc2::OpLoc2, loc, |p| {
    p.p2_alt1(1530).p1_alt2(1).p2_alt2(3218).p2_alt3(3222);
});
tile_op!(oploc3_matches_client, oploc3::OpLoc3, loc, |p| {
    p.p2_alt2(3222).p1_alt3(1).p2_alt1(3218).p2(1530);
});
tile_op!(oploc4_matches_client, oploc4::OpLoc4, loc, |p| {
    p.p2_alt3(3218).p2_alt2(3222).p2_alt2(1530).p1_alt1(1);
});
tile_op!(oploc5_matches_client, oploc5::OpLoc5, loc, |p| {
    p.p2_alt2(1530).p2_alt2(3222).p1_alt1(1).p2_alt1(3218);
});

// Ground-obj ops.
tile_op!(opobj1_matches_client, opobj1::OpObj1, obj, |p| {
    p.p1_alt3(1).p2_alt2(1530).p2_alt3(3222).p2_alt1(3218);
});
tile_op!(opobj2_matches_client, opobj2::OpObj2, obj, |p| {
    p.p2_alt1(3218).p1_alt1(1).p2_alt2(3222).p2_alt1(1530);
});
tile_op!(opobj3_matches_client, opobj3::OpObj3, obj, |p| {
    p.p2_alt1(1530).p2_alt2(3218).p1(1).p2(3222);
});
tile_op!(opobj4_matches_client, opobj4::OpObj4, obj, |p| {
    p.p2_alt1(3222).p2(3218).p1_alt3(1).p2_alt1(1530);
});
tile_op!(opobj5_matches_client, opobj5::OpObj5, obj, |p| {
    p.p2_alt1(3222).p1_alt2(1).p2_alt1(1530).p2(3218);
});

// ---------------------------------------------------------------------------
// Targeted ops -- "use this item on that", "cast this spell on that".
//
// The three source fields are static globals at the send site, set by
// `Js5TextureSource.set_target`:
//
//   Class285.anInt2320        = arg0.id        -> from_com
//   AClass3.anInt123          = arg0.object_id -> from_obj
//   TextureOp_Sub24.anInt5794 = arg0.child_id  -> from_slot
//
// Nothing in the bytes says which is which. Get the mapping wrong and all four
// targeted ops are wrong *together and consistently*, so a round-trip against
// our own encoder would still agree with itself. Only the client's assignment
// site settles it, which is why it is cited here rather than inferred.
// ---------------------------------------------------------------------------

const FROM_OBJ: i32 = 1511;
const FROM_SLOT: i32 = 7;
const FROM_COM: i32 = 149 << 16;

#[test]
fn oploct_matches_client() {
    use rs_protocol::network::game::client::oploct::OpLocT;

    let mut p = ClientPacket::new();
    p.p2_alt2(FROM_OBJ)
        .p2_alt2(FROM_SLOT)
        .p2(3218)
        .p4_alt2(FROM_COM)
        .p2_alt2(3222)
        .p1_alt1(1)
        .p2_alt1(1530);

    let m: OpLocT = decode(&p);
    assert_eq!(m.obj, FROM_OBJ as u16, "source obj");
    assert_eq!(m.slot, FROM_SLOT as u16, "source slot");
    assert_eq!(m.z, 3218, "z");
    assert_eq!(m.com, FROM_COM as u32, "source component");
    assert_eq!(m.x, 3222, "x");
    assert!(m.ctrl, "ctrl-run");
    assert_eq!(m.loc, 1530, "loc id");
}

#[test]
fn opnpct_matches_client() {
    use rs_protocol::network::game::client::opnpct::OpNpcT;

    // Note the component arrives `p4_alt3` here and `p4_alt2` for loc and obj.
    let mut p = ClientPacket::new();
    p.p2_alt2(FROM_OBJ)
        .p2_alt2(FROM_SLOT)
        .p2_alt1(4231)
        .p4_alt3(FROM_COM)
        .p1_alt2(1);

    let m: OpNpcT = decode(&p);
    assert_eq!(m.obj, FROM_OBJ as u16, "source obj");
    assert_eq!(m.slot, FROM_SLOT as u16, "source slot");
    assert_eq!(m.nid, 4231, "npc id");
    assert_eq!(m.com, FROM_COM as u32, "source component");
    assert!(m.ctrl, "ctrl-run");
}

#[test]
fn opobjt_matches_client() {
    use rs_protocol::network::game::client::opobjt::OpObjT;

    // Two `p2_alt1` reads in a row open with the source obj and the *target* obj.
    let mut p = ClientPacket::new();
    p.p2_alt1(FROM_OBJ)
        .p2_alt1(1530)
        .p2(FROM_SLOT)
        .p4_alt2(FROM_COM)
        .p1_alt1(1)
        .p2_alt2(3218)
        .p2_alt2(3222);

    let m: OpObjT = decode(&p);
    assert_eq!(m.obj2, FROM_OBJ as u16, "source obj");
    assert_eq!(m.obj, 1530, "target obj");
    assert_eq!(m.slot, FROM_SLOT as u16, "source slot");
    assert_eq!(m.com, FROM_COM as u32, "source component");
    assert!(m.ctrl, "ctrl-run");
    assert_eq!(m.z, 3218, "z");
    assert_eq!(m.x, 3222, "x");
}

#[test]
fn opplayert_matches_client() {
    use rs_protocol::network::game::client::opplayert::OpPlayerT;

    // The only one of the four that sends the component plain (`p4`).
    let mut p = ClientPacket::new();
    p.p2_alt2(4231)
        .p2_alt1(FROM_SLOT)
        .p4(FROM_COM)
        .p1_alt3(1)
        .p2_alt1(FROM_OBJ);

    let m: OpPlayerT = decode(&p);
    assert_eq!(m.pid, 4231, "player id");
    assert_eq!(m.slot, FROM_SLOT as u16, "source slot");
    assert_eq!(m.com, FROM_COM as u32, "source component");
    assert!(m.ctrl, "ctrl-run");
    assert_eq!(m.obj, FROM_OBJ as u16, "source obj");
}

// ---------------------------------------------------------------------------
// Interface buttons. `Class141.method876` -> `Class199.method1317`.
// ---------------------------------------------------------------------------

/// `method1317(msg, object_id, child_id, id)` writes `p4(id)`,
/// `p2_alt3(object_id)`, `p2_alt1(child_id)` -- the **obj before the slot**, which
/// reads backwards against the call site's own argument order and is easy to
/// transcribe the wrong way round. All ten IfButton ops share this one writer,
/// so a transposition here breaks every inventory op at once.
macro_rules! button_op {
    ($test:ident, $module:ident :: $ty:ident) => {
        #[test]
        fn $test() {
            use rs_protocol::network::game::client::$module::$ty;
            let mut p = ClientPacket::new();
            p.p4(FROM_COM).p2_alt3(FROM_OBJ).p2_alt1(FROM_SLOT);
            let m: $ty = decode(&p);
            assert_eq!(m.com, FROM_COM as u32, "component");
            // IfButton keeps these signed: an empty slot arrives as -1.
            assert_eq!(m.obj, FROM_OBJ as i16, "obj");
            assert_eq!(m.slot, FROM_SLOT as i16, "slot");
        }
    };
}

button_op!(if_button1_matches_client, if_button1::IfButton1);
button_op!(if_button2_matches_client, if_button2::IfButton2);
button_op!(if_button8_matches_client, if_button8::IfButton8);
button_op!(if_button10_matches_client, if_button10::IfButton10);

/// "Use this interface item on that one" -- the cross-interface drag, and the
/// shape a bank withdraw/deposit takes.
#[test]
fn if_buttont_matches_client() {
    use rs_protocol::network::game::client::if_buttont::IfButtonT;

    let to_com = (762 << 16) | 93;
    let mut p = ClientPacket::new();
    p.p4(to_com)
        .p2_alt1(12)
        .p2_alt1(FROM_SLOT)
        .p2_alt2(FROM_OBJ)
        .p4_alt1(FROM_COM)
        .p2(995);

    let m: IfButtonT = decode(&p);
    assert_eq!(m.com2, to_com as u32, "target component");
    assert_eq!(m.slot2, 12, "target slot");
    assert_eq!(m.slot, FROM_SLOT as u16, "source slot");
    assert_eq!(m.obj, FROM_OBJ as u16, "source obj");
    assert_eq!(m.com, FROM_COM as u32, "source component");
    assert_eq!(m.obj2, 995, "target obj");
}

// ---------------------------------------------------------------------------
// Movement.
// ---------------------------------------------------------------------------

/// Rev-647 sends one destination tile. The pre-647 packet of the same name
/// carried a waypoint path, and both structs live in the same module, so the
/// import here doubles as a check that the 647 build picks the right one.
#[test]
fn move_gameclick_matches_client() {
    use rs_protocol::network::game::client::move_gameclick::MoveGameClick;

    let mut p = ClientPacket::new();
    p.p2_alt1(3222).p1_alt1(1).p2(3218);

    let m: MoveGameClick = decode(&p);
    assert_eq!(m.x, 3222, "x");
    assert!(m.ctrl, "ctrl-run");
    assert_eq!(m.z, 3218, "z");
}

#[test]
fn move_minimapclick_matches_client() {
    let mut p = ClientPacket::new();
    p.p2_alt1(3222).p1_alt1(1).p2(3218);
    p.p1(-1)
        .p1(-1)
        .p2(12345)
        .p1(57)
        .p1(-55)
        .p1(7)
        .p1(89)
        .p2(6464)
        .p2(6208)
        .p1(63);

    let m: MoveMinimapClick = decode(&p);
    assert_eq!(m.x, 3222, "x");
    assert!(m.ctrl, "ctrl-run");
    assert_eq!(m.z, 3218, "z");
    assert_eq!(
        m.trailer,
        MinimapTrailer {
            x: -1,
            y: -1,
            yaw: 12345,
            magic1: 57,
            angle: -55,
            zoom: 7,
            magic2: 89,
            fine_x: 6464,
            fine_z: 6208,
            magic3: 63,
        }
    );
}

// ---------------------------------------------------------------------------
// Social edits. Each sends the name as typed (`pjstr`); only the ignore add
// follows it with anything.
// ---------------------------------------------------------------------------

/// `Statics1.method3487` writes `pjstr(name)` then `p1(flag)`, and sizes the
/// frame `get_pack_size(name) + 1` to hold it. A decoder that stops at the
/// name leaves the flag unread.
#[test]
fn ignorelist_add_matches_client() {
    use rs_protocol::network::game::client::ignorelist_add::IgnoreListAdd;

    for (flag, expect) in [(0, false), (1, true)] {
        let mut p = ClientPacket::new();
        p.pjstr("Mod Ash").p1(flag);

        let m: IgnoreListAdd = decode(&p);
        assert_eq!(m.name, "Mod Ash", "name");
        assert_eq!(m.temporary, expect, "flag");
    }
}

/// `AClass43_Sub5`, `AClass9_Sub1_Sub1` and `Class230` write the name alone.
#[test]
fn friend_and_ignore_edits_carry_the_name_alone() {
    use rs_protocol::network::game::client::friendlist_add::FriendListAdd;
    use rs_protocol::network::game::client::friendlist_del::FriendListDel;
    use rs_protocol::network::game::client::ignorelist_del::IgnoreListDel;

    let mut p = ClientPacket::new();
    p.pjstr("Mod Ash");
    assert_eq!(decode::<FriendListAdd>(&p).name, "Mod Ash");
    assert_eq!(decode::<FriendListDel>(&p).name, "Mod Ash");
    assert_eq!(decode::<IgnoreListDel>(&p).name, "Mod Ash");
}

// ---------------------------------------------------------------------------
// The opcodes that were "EventTracking" placeholders, and the two decoders
// whose order was not the client's.
// ---------------------------------------------------------------------------

/// CS2 5002 (`Class47`): `pjstr(offender) p1(rule - 1) p1(mute) pjstr(text)`.
/// The 289 frame was a fixed-size hash of the name.
#[test]
fn send_snapshot_matches_client() {
    use rs_protocol::network::game::client::send_snapshot::SendSnapshot;

    let mut p = ClientPacket::new();
    p.pjstr("Mod Ash").p1(4).p1(1).pjstr("bot");

    let m: SendSnapshot = decode(&p);
    assert_eq!(m.offender, "Mod Ash", "offender");
    assert_eq!(m.reason, 4, "rule");
    assert!(m.mute, "mute");
    assert_eq!(m.text, "bot", "text");
}

/// `Statics1.do_action`, `FACE_SQUARE`: `p2_alt2(x) p2_alt1(z)`.
#[test]
fn face_square_matches_client() {
    use rs_protocol::network::game::client::face_square::FaceSquare;

    let mut p = ClientPacket::new();
    p.p2_alt2(3222).p2_alt1(3218);

    let m: FaceSquare = decode(&p);
    assert_eq!((m.x, m.z), (3222, 3218));
}

/// The world map loop in `client`: `p4_alt2(level << 28 | x << 14 | z)`.
/// Read as a plain `g4`, the coord came out byte-swapped.
#[test]
fn click_world_map_matches_client() {
    use rs_protocol::network::game::client::click_world_map::ClickWorldMap;

    let coord = 1 << 28 | 3222 << 14 | 3218;
    let mut p = ClientPacket::new();
    p.p4_alt2(coord);

    let m: ClickWorldMap = decode(&p);
    assert_eq!(m.coord, coord as u32);
}

/// `Statics1.do_action`, `TGT_GROUND`: `p2_alt1(x) p2(slot) p2_alt2(obj)
/// p2_alt2(z) p4(com)`. The middle two were read as a camera yaw and pitch.
#[test]
fn apcoordt_matches_client() {
    use rs_protocol::network::game::client::apcoordt::ApCoordT;

    let mut p = ClientPacket::new();
    p.p2_alt1(3222)
        .p2(FROM_SLOT)
        .p2_alt2(FROM_OBJ)
        .p2_alt2(3218)
        .p4(FROM_COM);

    let m: ApCoordT = decode(&p);
    assert_eq!((m.x, m.z), (3222, 3218), "tile");
    assert_eq!(m.slot, FROM_SLOT as u16, "source slot");
    assert_eq!(m.obj, FROM_OBJ as u16, "source obj");
    assert_eq!(m.com, FROM_COM as u32, "source component");
}

/// `VarbitType`: `p2(flag << 15 | time) p4_alt2(x | y << 16)`.
#[test]
fn event_mouse_click_matches_client() {
    use rs_protocol::network::game::client::event_mouse_click::EventMouseClick;

    let mut p = ClientPacket::new();
    p.p2(1 << 15 | 40).p4_alt2(512 | 300 << 16);

    let m: EventMouseClick = decode(&p);
    assert_eq!(m.info, 1 << 15 | 40, "flag and time");
    assert_eq!(m.position, 512 | 300 << 16, "position");
}
