#![cfg(all(since_530, before_647))]

use rs_io::Packet;
use rs_protocol::network::game::client::ClientProtMessage;
use rs_protocol::network::game::client::event_mouse_move::{Cursor, MouseMoved, MouseSample};
use rs_protocol::network::game::client::move_minimapclick::MinimapTrailer;
use rs_protocol::network::game::client::pack_coord;
use rs_protocol::network::game::client_prot::*;
use rs_protocol::network::game::client_prot_frame::ClientProtFrame;
use rs_ver::TypeId;
use std::any::type_name;

const X: u16 = 3222;
const Z: u16 = 3218;
const LOC: TypeId = TypeId::new(0x1A2B);
const OBJ: TypeId = TypeId::new(0x0F1E);
const OBJ2: TypeId = TypeId::new(0x0995);
const NID: u16 = 0x0123;
const PID: u16 = 0x07D1;
const COM: i32 = 0x01020304;
const COM2: i32 = 0x05060708;
const SLOT: u16 = 11;
const SLOT2: u16 = 28;
const USER37: i64 = 0x2_E3F4_9C3B;
const NAME37: [u8; 8] = [0x00, 0x00, 0x00, 0x02, 0xE3, 0xF4, 0x9C, 0x3B];

const FRAMES: [(u8, i8); 100] = [
    (3, 2),
    (4, 2),
    (6, 8),
    (9, 6),
    (10, 4),
    (17, 0),
    (20, 4),
    (21, 4),
    (22, 1),
    (23, 4),
    (27, 16),
    (30, 2),
    (33, 6),
    (34, 8),
    (39, -1),
    (44, -1),
    (48, 6),
    (53, 6),
    (55, 8),
    (57, 8),
    (64, 6),
    (65, -1),
    (66, 6),
    (68, 2),
    (71, 2),
    (72, 2),
    (73, 12),
    (75, 6),
    (77, -1),
    (78, 2),
    (79, 12),
    (81, 8),
    (82, 12),
    (84, 6),
    (85, 8),
    (92, 2),
    (93, 0),
    (94, 2),
    (98, 4),
    (99, 10),
    (101, 14),
    (104, 8),
    (106, 2),
    (109, 6),
    (110, 0),
    (111, 2),
    (114, 2),
    (115, 10),
    (117, -1),
    (120, 8),
    (123, -1),
    (124, 6),
    (131, 10),
    (132, 6),
    (133, 2),
    (134, 14),
    (135, 8),
    (137, 4),
    (148, 2),
    (153, 8),
    (154, 8),
    (155, 6),
    (156, 8),
    (157, 3),
    (161, 8),
    (162, 8),
    (163, -1),
    (166, 6),
    (167, -1),
    (168, 6),
    (170, 6),
    (175, 2),
    (177, 2),
    (178, -1),
    (179, 4),
    (180, 2),
    (184, 0),
    (188, 9),
    (194, 6),
    (195, 8),
    (196, 6),
    (199, 6),
    (201, -1),
    (206, 8),
    (213, 8),
    (215, -1),
    (218, 2),
    (228, 6),
    (231, 9),
    (233, 12),
    (234, 6),
    (237, -1),
    (239, 8),
    (243, 6),
    (244, 8),
    (245, 0),
    (247, 6),
    (248, 10),
    (253, 14),
    (254, 6),
];

fn decode<M: ClientProtMessage>(op: u8, bytes: &[u8]) -> M {
    let prot = ClientProt::try_from(op).unwrap();
    assert_eq!(
        format!("{prot:?}"),
        type_name::<M>().rsplit("::").next().unwrap()
    );
    assert!(prot.validate(bytes), "{prot:?}");
    let mut buf = Packet::from(bytes.to_vec());
    let msg = M::decode(&mut buf, bytes.len());
    assert_eq!(buf.pos, bytes.len(), "{prot:?}");
    msg
}

#[test]
fn client_prot_frames_match_the_530_writers() {
    for (op, size) in FRAMES {
        let prot = ClientProt::try_from(op).unwrap();
        match prot.info().frame {
            ClientProtFrame::Fixed(len) => assert_eq!(len as i8, size, "{prot:?}"),
            ClientProtFrame::VarByte { .. } => assert_eq!(size, -1, "{prot:?}"),
            ClientProtFrame::VarShort { .. } => panic!("{prot:?}"),
        }
    }
    assert_eq!(ClientProt::ALL.len(), FRAMES.len());
}

#[test]
fn loc_ops_match_the_menu_writers() {
    let m: OpLoc1 = decode(254, &[0x96, 0x0C, 0x1A, 0xAB, 0x0C, 0x92]);
    assert_eq!((m.x, m.z, m.loc), (X, Z, LOC));
    let m: OpLoc2 = decode(194, &[0x12, 0x0C, 0x96, 0x0C, 0x1A, 0x2B]);
    assert_eq!((m.x, m.z, m.loc), (X, Z, LOC));
    let m: OpLoc3 = decode(84, &[0xAB, 0x1A, 0x12, 0x0C, 0x96, 0x0C]);
    assert_eq!((m.x, m.z, m.loc), (X, Z, LOC));
    let m: OpLoc4 = decode(247, &[0x92, 0x0C, 0x16, 0x0C, 0x1A, 0x2B]);
    assert_eq!((m.x, m.z, m.loc), (X, Z, LOC));
    let m: OpLoc5 = decode(170, &[0xAB, 0x1A, 0x16, 0x0C, 0x12, 0x0C]);
    assert_eq!((m.x, m.z, m.loc), (X, Z, LOC));
    let m: OpLoc6 = decode(94, &[0xAB, 0x1A]);
    assert_eq!(m.loc, LOC);
    let m: OpLocT = decode(
        233,
        &[
            0x12, 0x0C, 0x0C, 0x16, 0x8B, 0x00, 0x03, 0x04, 0x01, 0x02, 0x1A, 0xAB,
        ],
    );
    assert_eq!((m.x, m.z, m.loc, m.slot, m.com), (X, Z, LOC, SLOT, COM));
    let m: OpLocU = decode(
        134,
        &[
            0x0C, 0x16, 0x0F, 0x1E, 0x92, 0x0C, 0x00, 0x0B, 0x02, 0x01, 0x04, 0x03, 0x1A, 0xAB,
        ],
    );
    assert_eq!(
        (m.x, m.z, m.loc, m.obj, m.slot, m.com),
        (X, Z, LOC, OBJ, SLOT, COM)
    );
}

#[test]
fn obj_ops_match_the_menu_writers() {
    let m: OpObj1 = decode(228, &[0x0F, 0x1E, 0x96, 0x0C, 0x12, 0x0C]);
    assert_eq!((m.x, m.z, m.obj), (X, Z, OBJ));
    let m: OpObj2 = decode(109, &[0x92, 0x0C, 0x0C, 0x96, 0x9E, 0x0F]);
    assert_eq!((m.x, m.z, m.obj), (X, Z, OBJ));
    let m: OpObj3 = decode(66, &[0x96, 0x0C, 0x0F, 0x1E, 0x12, 0x0C]);
    assert_eq!((m.x, m.z, m.obj), (X, Z, OBJ));
    let m: OpObj4 = decode(33, &[0x0F, 0x1E, 0x0C, 0x96, 0x92, 0x0C]);
    assert_eq!((m.x, m.z, m.obj), (X, Z, OBJ));
    let m: OpObj5 = decode(48, &[0x0C, 0x16, 0x9E, 0x0F, 0x92, 0x0C]);
    assert_eq!((m.x, m.z, m.obj), (X, Z, OBJ));
    let m: OpObj6 = decode(92, &[0x9E, 0x0F]);
    assert_eq!(m.obj, OBJ);
    let m: OpObjT = decode(
        73,
        &[
            0x03, 0x04, 0x01, 0x02, 0x0C, 0x92, 0x9E, 0x0F, 0x16, 0x0C, 0x0B, 0x00,
        ],
    );
    assert_eq!((m.x, m.z, m.obj, m.slot, m.com), (X, Z, OBJ, SLOT, COM));
    let m: OpObjU = decode(
        101,
        &[
            0x16, 0x0C, 0x0B, 0x00, 0x95, 0x09, 0x1E, 0x0F, 0x12, 0x0C, 0x02, 0x01, 0x04, 0x03,
        ],
    );
    assert_eq!(
        (m.x, m.z, m.obj, m.use_obj, m.slot, m.com),
        (X, Z, OBJ, OBJ2, SLOT, COM)
    );
}

#[test]
fn npc_ops_match_the_menu_writers() {
    let m: OpNpc1 = decode(78, &[0x23, 0x01]);
    assert_eq!(m.nid, NID);
    let m: OpNpc2 = decode(3, &[0xA3, 0x01]);
    assert_eq!(m.nid, NID);
    let m: OpNpc3 = decode(148, &[0x01, 0xA3]);
    assert_eq!(m.nid, NID);
    let m: OpNpc4 = decode(30, &[0x01, 0x23]);
    assert_eq!(m.nid, NID);
    let m: OpNpc5 = decode(218, &[0x23, 0x01]);
    assert_eq!(m.nid, NID);
    let m: OpNpc6 = decode(72, &[0x01, 0x23]);
    assert_eq!(m.npc, TypeId::new(NID));
    let m: OpNpcT = decode(239, &[0x04, 0x03, 0x02, 0x01, 0x00, 0x8B, 0xA3, 0x01]);
    assert_eq!((m.nid, m.slot, m.com), (NID, SLOT, COM));
    let m: OpNpcU = decode(
        115,
        &[0x02, 0x01, 0x04, 0x03, 0x0B, 0x00, 0x23, 0x01, 0x9E, 0x0F],
    );
    assert_eq!((m.nid, m.obj, m.slot, m.com), (NID, OBJ, SLOT, COM));
}

#[test]
fn player_ops_match_the_menu_writers() {
    let m: OpPlayer1 = decode(68, &[0x51, 0x07]);
    assert_eq!(m.pid, PID);
    let m: OpPlayer2 = decode(106, &[0x07, 0xD1]);
    assert_eq!(m.pid, PID);
    let m: OpPlayer3 = decode(71, &[0x51, 0x07]);
    assert_eq!(m.pid, PID);
    let m: OpPlayer4 = decode(180, &[0x51, 0x07]);
    assert_eq!(m.pid, PID);
    let m: OpPlayer5 = decode(4, &[0xD1, 0x07]);
    assert_eq!(m.pid, PID);
    let m: OpPlayer6 = decode(133, &[0xD1, 0x07]);
    assert_eq!(m.pid, PID);
    let m: OpPlayer7 = decode(114, &[0x51, 0x07]);
    assert_eq!(m.pid, PID);
    let m: OpPlayer8 = decode(175, &[0x07, 0x51]);
    assert_eq!(m.pid, PID);
    let m: OpPlayerT = decode(195, &[0x00, 0x8B, 0x04, 0x03, 0x02, 0x01, 0x51, 0x07]);
    assert_eq!((m.pid, m.slot, m.com), (PID, SLOT, COM));
    let m: OpPlayerU = decode(
        248,
        &[0x51, 0x07, 0x0F, 0x1E, 0x00, 0x0B, 0x02, 0x01, 0x04, 0x03],
    );
    assert_eq!((m.pid, m.obj, m.slot, m.com), (PID, OBJ, SLOT, COM));
}

#[test]
fn held_ops_match_the_inventory_writers() {
    let m: OpHeld1 = decode(156, &[0x8B, 0x00, 0x0F, 0x9E, 0x04, 0x03, 0x02, 0x01]);
    assert_eq!((m.obj, m.slot, m.com), (OBJ, SLOT, COM));
    let m: OpHeld2 = decode(55, &[0x1E, 0x0F, 0x00, 0x8B, 0x03, 0x04, 0x01, 0x02]);
    assert_eq!((m.obj, m.slot, m.com), (OBJ, SLOT, COM));
    let m: OpHeld3 = decode(153, &[0x04, 0x03, 0x02, 0x01, 0x0B, 0x00, 0x1E, 0x0F]);
    assert_eq!((m.obj, m.slot, m.com), (OBJ, SLOT, COM));
    let m: OpHeld4 = decode(161, &[0x04, 0x03, 0x02, 0x01, 0x9E, 0x0F, 0x8B, 0x00]);
    assert_eq!((m.obj, m.slot, m.com), (OBJ, SLOT, COM));
    let m: OpHeld5 = decode(135, &[0x0F, 0x9E, 0x00, 0x8B, 0x02, 0x01, 0x04, 0x03]);
    assert_eq!((m.obj, m.slot, m.com), (OBJ, SLOT, COM));
    let m: OpHeldT = decode(
        253,
        &[
            0x08, 0x07, 0x06, 0x05, 0x8B, 0x00, 0x04, 0x03, 0x02, 0x01, 0x0F, 0x9E, 0x1C, 0x00,
        ],
    );
    assert_eq!(
        (m.obj, m.slot, m.com, m.com2, m.slot2),
        (OBJ, SLOT, COM, COM2, SLOT2)
    );
    let m: OpHeldU = decode(
        27,
        &[
            0x00, 0x1C, 0x04, 0x03, 0x02, 0x01, 0x0B, 0x00, 0x08, 0x07, 0x06, 0x05, 0x15, 0x09,
            0x9E, 0x0F,
        ],
    );
    assert_eq!(
        (m.obj, m.slot, m.com, m.obj2, m.slot2, m.com2),
        (OBJ, SLOT, COM, OBJ2, SLOT2, COM2)
    );
}

#[test]
fn if1_buttons_send_an_int_component() {
    let m: IfButton = decode(10, &[0x01, 0x02, 0x03, 0x04]);
    assert_eq!(m.com, COM);
    let m: InvButton1 = decode(81, &[0x00, 0x8B, 0x0F, 0x1E, 0x03, 0x04, 0x01, 0x02]);
    assert_eq!((m.obj, m.slot, m.com), (OBJ, SLOT, COM));
    let m: InvButton2 = decode(206, &[0x0F, 0x9E, 0x0B, 0x00, 0x04, 0x03, 0x02, 0x01]);
    assert_eq!((m.obj, m.slot, m.com), (OBJ, SLOT, COM));
    let m: InvButton3 = decode(154, &[0x0B, 0x00, 0x03, 0x04, 0x01, 0x02, 0x9E, 0x0F]);
    assert_eq!((m.obj, m.slot, m.com), (OBJ, SLOT, COM));
    let m: InvButton4 = decode(85, &[0x03, 0x04, 0x01, 0x02, 0x00, 0x0B, 0x0F, 0x9E]);
    assert_eq!((m.obj, m.slot, m.com), (OBJ, SLOT, COM));
    let m: InvButton5 = decode(6, &[0x01, 0x02, 0x03, 0x04, 0x00, 0x8B, 0x1E, 0x0F]);
    assert_eq!((m.obj, m.slot, m.com), (OBJ, SLOT, COM));
    let m: InvButtonD = decode(231, &[0x00, 0x0B, 0x04, 0x03, 0x02, 0x01, 0x00, 0x9C, 0x7F]);
    assert_eq!((m.com, m.slot, m.slot2, m.mode), (COM, SLOT, SLOT2, 1));
}

macro_rules! if_buttons {
    ($($ty:ident = $op:expr),* $(,)?) => {$(
        let m: $ty = decode($op, &[0x01, 0x02, 0x03, 0x04, 0x00, 0x0B]);
        assert_eq!((m.com, m.slot), (COM, SLOT));
    )*};
}

#[test]
fn if3_buttons_send_the_component_and_slot() {
    if_buttons!(
        IfButton1 = 155,
        IfButton2 = 196,
        IfButton3 = 124,
        IfButton4 = 199,
        IfButton5 = 234,
        IfButton6 = 168,
        IfButton7 = 166,
        IfButton8 = 64,
        IfButton9 = 53,
        IfButton10 = 9,
    );
    let m: IfButtonD = decode(
        79,
        &[
            0x02, 0x01, 0x04, 0x03, 0x1C, 0x00, 0x05, 0x06, 0x07, 0x08, 0x0B, 0x00,
        ],
    );
    assert_eq!((m.com, m.slot, m.com2, m.slot2), (COM, SLOT, COM2, SLOT2));
    let m: IfButtonT = decode(
        82,
        &[
            0x00, 0x0B, 0x07, 0x08, 0x05, 0x06, 0x01, 0x02, 0x03, 0x04, 0x9C, 0x00,
        ],
    );
    assert_eq!((m.com, m.slot, m.com2, m.slot2), (COM, SLOT, COM2, SLOT2));
    let m: ApCoordT = decode(
        131,
        &[0x02, 0x01, 0x04, 0x03, 0x0C, 0x16, 0x8B, 0x00, 0x0C, 0x12],
    );
    assert_eq!((m.x, m.z, m.slot, m.com), (X, Z, SLOT, COM));
    let m: ResumePauseButton = decode(132, &[0x03, 0x04, 0x01, 0x02, 0x06, 0x05]);
    assert_eq!((m.com, m.slot), (COM, 0x0506));
}

#[test]
fn movement_sends_the_path_from_the_first_step() {
    let path = [0x81, 0x0C, 0x96, 0x0C, 0x12, 0x82, 0x81, 0x7E, 0x7D];
    let steps = vec![
        pack_coord(X, Z),
        pack_coord(3224, 3217),
        pack_coord(3220, 3221),
    ];
    let m: MoveGameClick = decode(215, &path);
    assert_eq!((&m.path, m.ctrl), (&steps, true));
    let m: MoveOpClick = decode(77, &path);
    assert_eq!((&m.path, m.ctrl), (&steps, true));

    let mut minimap = path.to_vec();
    minimap.extend([
        0xF6, 0x14, 0x01, 0x00, 0x39, 0xC4, 0x05, 0x59, 0x1A, 0x40, 0x19, 0xC0, 0x01, 0x3F,
    ]);
    let m: MoveMinimapClick = decode(39, &minimap);
    assert_eq!((&m.path, m.ctrl), (&steps, true));
    assert_eq!(
        m.trailer,
        MinimapTrailer {
            x: -10,
            y: 20,
            yaw: 256,
            magic1: 57,
            angle: -60,
            zoom: 5,
            magic2: 89,
            fine_x: 0x1A40,
            fine_z: 0x19C0,
            nearest: 1,
            magic3: 63,
        }
    );
}

#[test]
fn chat_packets_send_a_smart_char_count() {
    let m: MessagePublic = decode(237, &[0x01, 0x02, 0x05, 0x1F, 0x2E]);
    assert_eq!(
        (m.colour, m.effect, m.chars, m.bytes),
        (1, 2, 5, vec![0x1F, 0x2E])
    );

    let mut bytes = NAME37.to_vec();
    bytes.extend([0x05, 0x1F, 0x2E, 0x3D]);
    let m: MessagePrivate = decode(201, &bytes);
    assert_eq!(
        (m.user37, m.chars, m.bytes),
        (USER37, 5, vec![0x1F, 0x2E, 0x3D])
    );

    let mut bytes = NAME37.to_vec();
    bytes.extend([0x80, 0xC8, 0x1F]);
    let m: MessagePrivate = decode(201, &bytes);
    assert_eq!((m.chars, m.bytes), (200, vec![0x1F]));
    assert!(!ClientProt::MessagePrivate.validate(&bytes[..9]));

    let mut bytes = NAME37.to_vec();
    bytes.extend([0x01, 0x02, 0x09, 0x08, 0x07]);
    let m: MessageQuickchatPrivate = decode(178, &bytes);
    assert_eq!(
        (m.user37, m.id, m.bytes),
        (USER37, TypeId::new(0x0102), vec![9, 8, 7])
    );
}

#[test]
fn social_packets_send_base37_names() {
    let m: FriendListAdd = decode(120, &NAME37);
    assert_eq!(m.user37, USER37);
    let m: FriendListDel = decode(57, &NAME37);
    assert_eq!(m.user37, USER37);
    let m: IgnoreListAdd = decode(34, &NAME37);
    assert_eq!(m.user37, USER37);
    let m: IgnoreListDel = decode(213, &NAME37);
    assert_eq!(m.user37, USER37);
    let m: ClanJoinchatLeavechat = decode(104, &NAME37);
    assert_eq!(m.user37, USER37);
    let m: ClanJoinchatLeavechat = decode(104, &[0; 8]);
    assert_eq!(m.user37, 0);
    let m: ClanKickUser = decode(162, &NAME37);
    assert_eq!(m.user37, USER37);
    let m: ResumePNameDialog = decode(244, &NAME37);
    assert_eq!(m.user37, USER37);

    let mut bytes = vec![0x83];
    bytes.extend(NAME37);
    let m: FriendSetRank = decode(188, &bytes);
    assert_eq!((m.rank, m.user37), (3, USER37));

    let mut bytes = NAME37.to_vec();
    bytes.extend([4, 1]);
    let m: SendSnapshot = decode(99, &bytes);
    assert_eq!((m.offender, m.reason, m.mute), (USER37, 4, true));
}

#[test]
fn client_events_match_the_530_writers() {
    let m: EventMouseClick = decode(75, &[0xE4, 0x80, 0xC8, 0x00, 0x2C, 0x01]);
    assert_eq!(m.at, Cursor::At { x: 300, y: 200 });
    assert!(m.right);
    assert_eq!(m.gap, 100);

    let m: EventCameraPosition = decode(21, &[0x01, 0xAC, 0xDC, 0x05]);
    assert_eq!((m.pitch, m.yaw), (300, 1500));

    let m: EventMouseMove = decode(123, &[0x39, 0x5C, 0xCA, 0x00, 0xC8, 0x01, 0x2C]);
    assert_eq!(
        m.samples,
        [
            MouseSample {
                gap: 3,
                moved: MouseMoved::By { dx: 5, dy: -4 },
            },
            MouseSample {
                gap: 10,
                moved: MouseMoved::To(Cursor::At { x: 300, y: 200 }),
            },
        ]
    );

    let m: ClientDetailoptionsStatus = decode(98, &[0x12, 0x34, 0x56, 0x78]);
    assert_eq!(m.options, 0x12345678);
    let _: GameloginContinue = decode(17, &[]);
}

#[test]
fn misc_packets_match_the_530_writers() {
    let m: FaceSquare = decode(179, &[0x0C, 0x92, 0x0C, 0x96]);
    assert_eq!((m.x, m.z), (X, Z));

    let m: ClientCheat = decode(
        44,
        &[
            0x74, 0x65, 0x6C, 0x65, 0x20, 0x33, 0x32, 0x32, 0x32, 0x20, 0x33, 0x32, 0x31, 0x38,
            0x00,
        ],
    );
    assert_eq!(m.cheat, "tele 3222 3218");

    let m: ReflectionCheckReply = decode(
        163,
        &[
            0x0A, 0x0B, 0x0C, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x10, 0xF5, 0xC7, 0x90, 0x06, 0x2F,
        ],
    );
    assert_eq!(m.id, 0x0A0B0C0D);
    assert!(m.crc_matches());
}
