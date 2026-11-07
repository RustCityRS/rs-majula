use rs_io::Packet;
use rs_protocol::network::game::server::ServerProtMessage;
use rs_protocol::network::game::server::loc_add_change::LocAddChange;
use rs_protocol::network::game::server::loc_anim::LocAnim;
use rs_protocol::network::game::server::loc_del::LocDel;
use rs_protocol::network::game::server::map_anim::MapAnim;
use rs_protocol::network::game::server::map_projanim::MapProjAnim;
use rs_protocol::network::game::server::obj_add::ObjAdd;
use rs_protocol::network::game::server::obj_count::ObjCount;
use rs_protocol::network::game::server::obj_del::ObjDel;
use rs_protocol::network::game::server::obj_reveal::ObjReveal;
#[cfg(since_647)]
use rs_protocol::network::game::server::update_zone_partial_enclosed::UpdateZonePartialEnclosed;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_protocol::network::game::server_prot_message::ServerProtMessageInfo;
use rs_zone::ZoneMessage;

#[test]
fn zone_messages_match_client_ids_and_sizes() {
    let messages = [
        ZoneMessage::LocAddChange(LocAddChange {
            coord: 0x35,
            shape_angle: 41,
            id: 1276,
        }),
        ZoneMessage::LocDel(LocDel {
            coord: 0x35,
            shape_angle: 41,
        }),
        ZoneMessage::LocAnim(LocAnim {
            coord: 0x35,
            shape_angle: 41,
            seq: 733,
        }),
        ZoneMessage::ObjAdd(ObjAdd {
            coord: 0x35,
            id: 1511,
            count: 3,
        }),
        ZoneMessage::ObjDel(ObjDel {
            coord: 0x35,
            id: 1511,
        }),
        ZoneMessage::ObjCount(ObjCount {
            coord: 0x35,
            id: 995,
            old_count: 1,
            new_count: 7,
        }),
        ZoneMessage::ObjReveal(ObjReveal {
            coord: 0x35,
            id: 1511,
            count: 3,
            receiver: 42,
        }),
        ZoneMessage::MapAnim(MapAnim {
            coord: 0x35,
            spotanim: 90,
            height: 100,
            delay: 30,
        }),
        ZoneMessage::MapProjAnim(MapProjAnim {
            coord: 0x35,
            dx: -4,
            dz: 6,
            target: -43,
            spotanim: 91,
            src_height: 43,
            dst_height: 31,
            start_delay: 51,
            end_delay: 70,
            peak: 16,
            arc: 64,
        }),
    ];
    #[cfg(before_647)]
    let expected = [
        (ServerProt::LocAddChange as u8, 4),
        (ServerProt::LocDel as u8, 2),
        (ServerProt::LocAnim as u8, 4),
        (ServerProt::ObjAdd as u8, 5),
        (ServerProt::ObjDel as u8, 3),
        (ServerProt::ObjCount as u8, 7),
        (ServerProt::ObjReveal as u8, 7),
        (ServerProt::MapAnim as u8, 6),
        (ServerProt::MapProjAnim as u8, 15),
    ];
    #[cfg(since_647)]
    let expected = [
        (13, 4),
        (8, 2),
        (3, 4),
        (11, 5),
        (1, 3),
        (4, 7),
        (7, 7),
        (6, 7),
        (14, 16),
    ];

    let mut buf = Packet::new(256);
    for (message, (id, size)) in messages.into_iter().zip(expected) {
        let start = buf.pos;
        message.encode_zone(&mut buf);
        assert_eq!(buf.data[start], id, "{message:?} zone id");
        assert_eq!(buf.pos - start, 1 + size, "{message:?} client size");
        assert_eq!(message.sizeof_zone(), 1 + size, "{message:?} sizeof");
    }
}

#[test]
fn loc_anim_payload_matches_revision() {
    let message = LocAnim {
        coord: 0x35,
        shape_angle: 41,
        seq: 0x1234,
    };
    let mut buf = Packet::new(message.sizeof());
    message.encode(&mut buf);
    #[cfg(before_647)]
    assert_eq!(&buf.data[..buf.pos], &[0x35, 41, 0x12, 0x34]);
    #[cfg(since_647)]
    assert_eq!(&buf.data[..buf.pos], &[0xcb, 41, 0x34, 0x12]);
    assert_eq!(LocAnim::PROT as u8, ServerProt::LocAnim as u8);
}

#[cfg(since_647)]
#[test]
fn enclosed_loc_del_uses_zone_id_in_crash_payload() {
    let message = ZoneMessage::LocDel(LocDel {
        coord: 0x76,
        shape_angle: 2,
    });
    let mut shared = Packet::new(message.sizeof_zone());
    message.encode_zone(&mut shared);
    let enclosed = UpdateZonePartialEnclosed {
        x: 5,
        z: 6,
        y: 1,
        bytes: &shared.data[..shared.pos],
    };
    let mut buf = Packet::new(enclosed.sizeof());
    enclosed.encode(&mut buf);
    assert_eq!(&buf.data[..buf.pos], &[5, 255, 6, 8, 2, 10]);
    assert_eq!(LocDel::PROT as u8, 17);
    assert_eq!(UpdateZonePartialEnclosed::PROT as u8, 111);
}
