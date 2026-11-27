#![cfg(all(since_530, before_647))]

use rs_io::Packet;
use rs_protocol::network::game::info::health;
use rs_protocol::network::game::info::npc::anim::NpcInfoAnim;
use rs_protocol::network::game::info::npc::changetype::NpcInfoChangeType;
use rs_protocol::network::game::info::npc::damage::{NpcInfoDamage, NpcInfoDamage2};
use rs_protocol::network::game::info::npc::face_coord::NpcInfoFaceCoord;
use rs_protocol::network::game::info::npc::face_entity::NpcInfoFaceEntity;
use rs_protocol::network::game::info::npc::model_anims::NpcInfoModelAnims;
use rs_protocol::network::game::info::npc::say::NpcInfoSay;
use rs_protocol::network::game::info::npc::spotanim::NpcInfoSpotAnim;
use rs_protocol::network::game::info::player::anim::PlayerInfoAnim;
use rs_protocol::network::game::info::player::appearance::{
    Appearance, Look, PlayerInfoAppearance, Wear,
};
use rs_protocol::network::game::info::player::chat::PlayerInfoChat;
use rs_protocol::network::game::info::player::damage::{PlayerInfoDamage, PlayerInfoDamage2};
use rs_protocol::network::game::info::player::exactmove::PlayerInfoExactMove;
use rs_protocol::network::game::info::player::face_coord::PlayerInfoFaceCoord;
use rs_protocol::network::game::info::player::face_entity::PlayerInfoFaceEntity;
use rs_protocol::network::game::info::player::model_anims::{ModelAnim, PlayerInfoModelAnims};
use rs_protocol::network::game::info::player::say::PlayerInfoSay;
use rs_protocol::network::game::info::player::spotanim::PlayerInfoSpotAnim;
use rs_protocol::network::game::info_prot::{NpcInfoProt, PlayerInfoProt};
use rs_protocol::network::game::info_prot_message::InfoMessage;
use rs_protocol::network::game::server::ServerProtMessage;
use rs_protocol::network::game::server::cam_forceangle::CamForceangle;
use rs_protocol::network::game::server::cam_lookat::CamLookat;
use rs_protocol::network::game::server::cam_moveto::CamMoveto;
use rs_protocol::network::game::server::cam_reset::CamReset;
use rs_protocol::network::game::server::cam_shake::CamShake;
use rs_protocol::network::game::server::chat_filter_settings::ChatFilterSettings;
use rs_protocol::network::game::server::friendlist_loaded::FriendListLoaded;
use rs_protocol::network::game::server::hint_arrow::{HintArrow, HintArrowKind};
use rs_protocol::network::game::server::if_closesub::IfClosesub;
use rs_protocol::network::game::server::if_movesub::IfMovesub;
use rs_protocol::network::game::server::if_opensub::IfOpensub;
use rs_protocol::network::game::server::if_opentop::IfOpentop;
use rs_protocol::network::game::server::if_setangle::IfSetAngle;
use rs_protocol::network::game::server::if_setanim::IfSetAnim;
use rs_protocol::network::game::server::if_setcolour::IfSetColour;
use rs_protocol::network::game::server::if_setevents::IfSetEvents;
use rs_protocol::network::game::server::if_sethide::IfSetHide;
use rs_protocol::network::game::server::if_setmodel::IfSetModel;
use rs_protocol::network::game::server::if_setnpchead::IfSetNpcHead;
use rs_protocol::network::game::server::if_setobject::IfSetObject;
use rs_protocol::network::game::server::if_setplayerhead::IfSetPlayerHead;
use rs_protocol::network::game::server::if_setposition::IfSetPosition;
use rs_protocol::network::game::server::if_setscrollpos::IfSetScrollPos;
use rs_protocol::network::game::server::if_settargetparam::IfSetTargetParam;
use rs_protocol::network::game::server::if_settext::IfSetText;
use rs_protocol::network::game::server::last_login_info::LastLoginInfo;
use rs_protocol::network::game::server::loc_add_change::LocAddChange;
use rs_protocol::network::game::server::loc_anim::LocAnim;
use rs_protocol::network::game::server::loc_anim_specific::LocAnimSpecific;
use rs_protocol::network::game::server::loc_del::LocDel;
use rs_protocol::network::game::server::loc_merge::LocMerge;
use rs_protocol::network::game::server::logout::Logout;
use rs_protocol::network::game::server::map_anim::MapAnim;
use rs_protocol::network::game::server::map_projanim::MapProjAnim;
use rs_protocol::network::game::server::map_projanim_halfsq::{
    MapProjAnimHalfsq, MapProjAnimHalfsqSrc,
};
use rs_protocol::network::game::server::message_friendchat::MessageFriendchat;
use rs_protocol::network::game::server::message_game::MessageGame;
use rs_protocol::network::game::server::message_private::MessagePrivate;
use rs_protocol::network::game::server::message_private_echo::MessagePrivateEcho;
use rs_protocol::network::game::server::message_quickchat_friendchat::MessageQuickchatFriendchat;
use rs_protocol::network::game::server::message_quickchat_private::MessageQuickchatPrivate;
use rs_protocol::network::game::server::message_quickchat_private_echo::MessageQuickchatPrivateEcho;
use rs_protocol::network::game::server::midi_jingle::MidiJingle;
use rs_protocol::network::game::server::midi_song::MidiSong;
use rs_protocol::network::game::server::minimap_toggle::MinimapToggle;
use rs_protocol::network::game::server::npc_anim_specific::NpcAnimSpecific;
use rs_protocol::network::game::server::npc_info::add_direction;
use rs_protocol::network::game::server::obj_add::ObjAdd;
use rs_protocol::network::game::server::obj_count::ObjCount;
use rs_protocol::network::game::server::obj_del::ObjDel;
use rs_protocol::network::game::server::obj_reveal::ObjReveal;
use rs_protocol::network::game::server::rebuild_normal::RebuildNormal;
use rs_protocol::network::game::server::rebuild_region::RebuildRegion;
use rs_protocol::network::game::server::reflection_checker::{ReflectionCheck, ReflectionChecker};
use rs_protocol::network::game::server::reset_anims::ResetAnims;
use rs_protocol::network::game::server::reset_client_varcache::ResetClientVarCache;
use rs_protocol::network::game::server::run_client_script::{RunClientScript, ScriptParam};
use rs_protocol::network::game::server::set_moveaction::SetMoveaction;
use rs_protocol::network::game::server::set_player_op::SetPlayerOp;
use rs_protocol::network::game::server::sound_area::SoundArea;
use rs_protocol::network::game::server::spotanim_specific::SpotanimSpecific;
use rs_protocol::network::game::server::synth_sound::SynthSound;
use rs_protocol::network::game::server::unset_map_flag::UnsetMapFlag;
use rs_protocol::network::game::server::update_friendchat_channel_full::{
    FriendchatMember, UpdateFriendchatChannelFull,
};
use rs_protocol::network::game::server::update_friendchat_channel_singleuser::UpdateFriendchatChannelSingleuser;
use rs_protocol::network::game::server::update_friendlist::UpdateFriendList;
use rs_protocol::network::game::server::update_ignorelist::UpdateIgnoreList;
use rs_protocol::network::game::server::update_inv_full::{InvItem, UpdateInvFull};
use rs_protocol::network::game::server::update_inv_partial::UpdateInvPartial;
use rs_protocol::network::game::server::update_inv_stop_transmit::UpdateInvStopTransmit;
use rs_protocol::network::game::server::update_reboot_timer::UpdateRebootTimer;
use rs_protocol::network::game::server::update_runenergy::UpdateRunEnergy;
use rs_protocol::network::game::server::update_runweight::UpdateRunWeight;
use rs_protocol::network::game::server::update_stat::UpdateStat;
use rs_protocol::network::game::server::update_zone_full_follows::UpdateZoneFullFollows;
use rs_protocol::network::game::server::update_zone_partial_enclosed::UpdateZonePartialEnclosed;
use rs_protocol::network::game::server::update_zone_partial_follows::UpdateZonePartialFollows;
use rs_protocol::network::game::server::url_open::UrlOpen;
use rs_protocol::network::game::server::varc_large::VarcLarge;
use rs_protocol::network::game::server::varc_small::VarcSmall;
use rs_protocol::network::game::server::varc_str_large::VarcStrLarge;
use rs_protocol::network::game::server::varc_str_small::VarcStrSmall;
use rs_protocol::network::game::server::varp_large::VarpLarge;
use rs_protocol::network::game::server::varp_small::VarpSmall;
use rs_protocol::network::game::server::varp_sync::VarpSync;
use rs_protocol::network::game::server::varpbit_large::VarpBitLarge;
use rs_protocol::network::game::server::varpbit_small::VarpBitSmall;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_protocol::network::game::server_prot_message::ServerProtMessageInfo;
use rs_protocol::network::game::zone_prot::ZoneProt;
use rs_ver::TypeId;

const SIZES: [i8; 256] = [
    -1, 0, 8, 0, 2, 0, 0, 0, 0, 12, 0, 1, 0, 3, 7, 0, 15, 6, 0, 0, 4, 7, -2, -1, 2, 0, 2, 8, 0, 0,
    0, 0, -2, 5, 0, 0, 8, 3, 6, 0, 0, 0, -1, 0, -1, 0, 0, 6, -2, 0, 12, 0, 0, 0, -1, -2, 10, 0, 0,
    0, 3, 0, -1, 0, 0, 5, 6, 0, 0, 8, -1, -1, 0, 8, 0, 0, 0, 0, 0, 0, 0, -1, 0, 0, 6, 2, 0, 0, 0,
    0, 1, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 5, 0, 0, -2, 0, 0, 0, 0, 0, 12, 2, 0, -2, -2, 20, 0, 0,
    10, 0, 15, 0, -1, 0, 8, -2, 0, 0, 0, 8, 0, 12, 0, 0, 7, 0, 0, 0, 0, 0, -1, -1, 0, 4, 5, 0, 0,
    0, 6, 0, 0, 0, 0, 8, 9, 0, 0, 0, 2, -1, 0, -2, 0, 4, 14, 0, 0, 0, 24, 0, -2, 5, 0, 0, 0, 10, 0,
    0, 4, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 2, 1, 0, 0, 2, -1, 1, 0, 0, 0, 0, 14, 0, 0, 0, 0, 10, 5,
    0, 0, 0, 0, 0, -2, 0, 0, 9, 0, 0, 8, 0, 0, 0, 0, -2, 6, 0, 0, 0, -2, 0, 3, 0, 1, 7, 0, 0, 0, 0,
    3, 0, 0, 0, 0, 0, 0, -1, 0, 0, 0, 0, 0, 3, 0, 0,
];

const COM: i32 = 0x01020304;
const VERIFY: u16 = 0x0A0B;

fn encode<M: ServerProtMessage>(msg: &M) -> Vec<u8> {
    let mut buf = Packet::new(msg.sizeof());
    msg.encode(&mut buf);
    assert_eq!(buf.pos, msg.sizeof());
    let size = SIZES[M::PROT.id() as usize];
    if size < 0 {
        assert_eq!(M::FRAME as i8, -size, "{:?} frame", M::PROT);
    } else {
        assert_eq!(M::FRAME as i8, 0, "{:?} frame", M::PROT);
        assert_eq!(buf.pos, size as usize, "{:?} size", M::PROT);
    }
    buf.data
}

#[test]
fn rebuild_normal_matches_the_login_reader() {
    let rebuild = RebuildNormal {
        player_x: 3222,
        player_z: 3218,
        player_plane: 1,
        xtea_keys: vec![[1, 2, 3, 4]],
    };
    assert_eq!(RebuildNormal::PROT.id(), 162);
    assert_eq!(
        encode(&rebuild),
        [
            0, 182, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 127, 1, 146, 1, 18, 0, 178
        ]
    );
}

#[test]
fn rebuild_region_puts_the_level_first_and_zone_z_after_the_keys() {
    let mut templates = vec![None; 4 * 13 * 13];
    templates[0] = Some((1 << 24) | (400 << 14) | (402 << 3) | (1 << 1));
    let rebuild = RebuildRegion {
        player_x: 3222,
        player_z: 3218,
        player_plane: 0,
        templates,
        xtea_keys: vec![[5, 6, 7, 8]],
    };
    assert_eq!(RebuildRegion::PROT.id(), 214);
    let bytes = encode(&rebuild);
    assert_eq!(&bytes[..7], [182, 0, 18, 1, 128, 178, 0]);
    let bits = &bytes[7..7 + 88];
    let bit = |pos: usize| (bits[pos >> 3] >> (7 - (pos & 7))) & 1;
    assert_eq!(bit(0), 1);
    let template = (1..27).fold(0u32, |value, pos| (value << 1) | u32::from(bit(pos)));
    assert_eq!(template, (1 << 24) | (400 << 14) | (402 << 3) | (1 << 1));
    assert!((27..27 + 675).all(|pos| bit(pos) == 0));
    assert_eq!(
        &bytes[7 + 88..],
        [0, 0, 5, 0, 0, 0, 6, 0, 0, 0, 7, 0, 0, 0, 8, 0, 1, 146]
    );
}

#[test]
fn update_inv_partial_sends_the_component_key_then_the_inv() {
    let other = UpdateInvPartial {
        com: -1,
        inv_id: TypeId::new(93),
        other: true,
        objs: &[(0, Some((TypeId::new(995), 1000))), (200, None)],
    };
    assert_eq!(UpdateInvPartial::PROT.id(), 22);
    assert_eq!(
        encode(&other),
        [
            0xFF, 0xFE, 0xEE, 0x8F, 0, 93, 0, 3, 228, 255, 0, 0, 3, 232, 0x80, 200, 0, 0
        ]
    );
    let own = UpdateInvPartial {
        com: -1,
        inv_id: TypeId::new(93),
        other: false,
        objs: &[(1, Some((TypeId::new(0), 5)))],
    };
    assert_eq!(encode(&own), [255, 255, 255, 255, 0, 93, 1, 0, 1, 5]);
    let backpack = UpdateInvPartial {
        com: 149 << 16,
        inv_id: TypeId::new(93),
        other: false,
        objs: &[(1, Some((TypeId::new(0), 5)))],
    };
    assert_eq!(encode(&backpack), [0, 149, 0, 0, 0, 93, 1, 0, 1, 5]);
}

#[test]
fn appearance_matches_the_player_reader() {
    let mut slots = [Wear::None; 12];
    slots[0] = Wear::Kit(0);
    slots[1] = Wear::Obj(5);
    let appearance = Appearance {
        gender: 1,
        headicons: 1,
        look: Look::Worn(slots),
        colours: &[1, 2, 3, 4, 5],
        username37: 12345,
        bas: TypeId::new(1426),
        combat_level: 3,
        skill_level: 0,
    };
    let mut buf = Packet::new(64);
    appearance.encode(&mut buf);
    let mut expected = vec![1, 0, 255, 1, 0, 0x80, 5];
    expected.extend_from_slice(&[0; 10]);
    expected.extend_from_slice(&[
        1, 2, 3, 4, 5, 5, 146, 0, 0, 0, 0, 0, 0, 0x30, 0x39, 3, 0, 255, 0,
    ]);
    assert_eq!(&buf.data[..buf.pos], expected.as_slice());
    let block = PlayerInfoAppearance {
        bytes: &buf.data[..buf.pos],
    };
    let mut framed = Packet::new(block.sizeof());
    block.encode(&mut framed);
    assert_eq!(framed.data[0], buf.pos as u8 + 128);
    assert_eq!(&framed.data[1..], expected.as_slice());
}

#[test]
fn server_prot_ids_follow_the_530_table() {
    assert_eq!(ServerProt::PlayerInfo.id(), 225);
    assert_eq!(ServerProt::NpcInfo.id(), 32);
    assert_eq!(ServerProt::LastLoginInfo.id(), 164);
    assert_eq!(ServerProt::NAMES.len(), 97);
}

#[test]
fn if_tree_packets_carry_the_verify_id() {
    let opentop = IfOpentop {
        interface: 548,
        mode: 2,
        verify: VERIFY,
    };
    assert_eq!(IfOpentop::PROT.id(), 145);
    assert_eq!(encode(&opentop), [0xA4, 2, 0x82, 0x8B, 0x0A]);
    let opensub = IfOpensub {
        interface: 548,
        com: COM,
        walkable: 1,
        verify: VERIFY,
    };
    assert_eq!(IfOpensub::PROT.id(), 155);
    assert_eq!(encode(&opensub), [1, 2, 1, 4, 3, 0x0A, 0x8B, 2, 0x24]);
    let closesub = IfClosesub {
        com: COM,
        verify: VERIFY,
    };
    assert_eq!(IfClosesub::PROT.id(), 149);
    assert_eq!(encode(&closesub), [0x0A, 0x0B, 1, 2, 3, 4]);
    let movesub = IfMovesub {
        com: COM,
        com2: 0x05060708,
        verify: VERIFY,
    };
    assert_eq!(IfMovesub::PROT.id(), 176);
    assert_eq!(encode(&movesub), [3, 4, 1, 2, 0x0A, 0x8B, 7, 8, 5, 6]);
}

#[test]
fn if_setters_match_the_530_readers() {
    let text = IfSetText {
        com: COM,
        text: "Hi",
        verify: VERIFY,
    };
    assert_eq!(IfSetText::PROT.id(), 171);
    assert_eq!(encode(&text), [2, 1, 4, 3, b'H', b'i', 0, 0x0A, 0x8B]);
    let hide = IfSetHide {
        com: COM,
        hidden: true,
        verify: VERIFY,
    };
    assert_eq!(IfSetHide::PROT.id(), 21);
    assert_eq!(encode(&hide), [0xFF, 0x0A, 0x0B, 4, 3, 2, 1]);
    let anim = IfSetAnim {
        com: COM,
        seq: TypeId::new(808),
        verify: VERIFY,
    };
    assert_eq!(IfSetAnim::PROT.id(), 36);
    assert_eq!(encode(&anim), [2, 1, 4, 3, 0x28, 3, 0x0A, 0x8B]);
    let colour = IfSetColour {
        com: COM,
        colour: 0x7C1F,
        verify: VERIFY,
    };
    assert_eq!(IfSetColour::PROT.id(), 2);
    assert_eq!(encode(&colour), [3, 4, 1, 2, 0x0A, 0x8B, 0x9F, 0x7C]);
    let model = IfSetModel {
        com: COM,
        model: TypeId::new(0x1234),
        verify: VERIFY,
    };
    assert_eq!(IfSetModel::PROT.id(), 130);
    assert_eq!(encode(&model), [4, 3, 2, 1, 0x8B, 0x0A, 0x12, 0xB4]);
    let npchead = IfSetNpcHead {
        com: COM,
        npc: TypeId::new(0x1234),
        verify: VERIFY,
    };
    assert_eq!(IfSetNpcHead::PROT.id(), 73);
    assert_eq!(encode(&npchead), [0x12, 0xB4, 4, 3, 2, 1, 0x0B, 0x0A]);
    let playerhead = IfSetPlayerHead {
        com: COM,
        verify: VERIFY,
    };
    assert_eq!(IfSetPlayerHead::PROT.id(), 66);
    assert_eq!(encode(&playerhead), [0x8B, 0x0A, 3, 4, 1, 2]);
    let object = IfSetObject {
        com: COM,
        obj: TypeId::new(0x1234),
        scale: 100,
        verify: VERIFY,
    };
    assert_eq!(IfSetObject::PROT.id(), 50);
    assert_eq!(
        encode(&object),
        [0, 0, 0, 100, 2, 1, 4, 3, 0xB4, 0x12, 0x0B, 0x0A]
    );
    let angle = IfSetAngle {
        com: COM,
        x: 0x0102,
        y: 0x0304,
        zoom: 0x0506,
        verify: VERIFY,
    };
    assert_eq!(IfSetAngle::PROT.id(), 132);
    assert_eq!(
        encode(&angle),
        [1, 2, 0x0A, 0x8B, 0x86, 5, 0x84, 3, 1, 2, 3, 4]
    );
    let position = IfSetPosition {
        com: COM,
        x: -10i16 as u16,
        y: 20,
        verify: VERIFY,
    };
    assert_eq!(IfSetPosition::PROT.id(), 119);
    assert_eq!(
        encode(&position),
        [0x0A, 0x8B, 4, 3, 2, 1, 0xFF, 0xF6, 0, 0x94]
    );
    let scrollpos = IfSetScrollPos {
        com: COM,
        y: 0x0102,
        verify: VERIFY,
    };
    assert_eq!(IfSetScrollPos::PROT.id(), 220);
    assert_eq!(encode(&scrollpos), [2, 1, 4, 3, 2, 1, 0x0A, 0x0B]);
}

#[test]
fn if_events_and_targetparam_send_the_end_slot_first() {
    let events = IfSetEvents {
        com: COM,
        slot1: 0,
        slot2: 27,
        settings: 1026,
        verify: VERIFY,
    };
    assert_eq!(IfSetEvents::PROT.id(), 165);
    assert_eq!(
        encode(&events),
        [0x0B, 0x0A, 27, 0, 1, 2, 3, 4, 0, 0x80, 4, 2, 0, 0]
    );
    let targetparam = IfSetTargetParam {
        com: COM,
        slot1: 0,
        slot2: 27,
        inv: TypeId::new(93),
        verify: VERIFY,
    };
    assert_eq!(IfSetTargetParam::PROT.id(), 9);
    assert_eq!(
        encode(&targetparam),
        [0xDD, 0, 4, 3, 2, 1, 0x0A, 0x8B, 27, 0, 0, 0x80]
    );
}

#[test]
fn vars_match_the_530_readers() {
    assert_eq!(VarpSmall::PROT.id(), 60);
    assert_eq!(encode(&VarpSmall { id: 0x0123, val: 5 }), [1, 0xA3, 0xFB]);
    assert_eq!(VarpLarge::PROT.id(), 226);
    assert_eq!(
        encode(&VarpLarge {
            id: 0x0123,
            val: COM
        }),
        [1, 2, 3, 4, 1, 0xA3]
    );
    assert_eq!(VarpBitSmall::PROT.id(), 37);
    assert_eq!(
        encode(&VarpBitSmall { id: 0x0123, val: 5 }),
        [0x85, 0x23, 1]
    );
    assert_eq!(VarpBitLarge::PROT.id(), 84);
    assert_eq!(
        encode(&VarpBitLarge {
            id: 0x0123,
            val: COM
        }),
        [4, 3, 2, 1, 0xA3, 1]
    );
    let varc = VarcSmall {
        id: 0x0123,
        val: 200,
        verify: VERIFY,
    };
    assert_eq!(VarcSmall::PROT.id(), 65);
    assert_eq!(encode(&varc), [0x0B, 0x0A, 56, 0xA3, 1]);
    let varc = VarcLarge {
        id: 0x0123,
        val: COM,
        verify: VERIFY,
    };
    assert_eq!(VarcLarge::PROT.id(), 69);
    assert_eq!(encode(&varc), [0x8B, 0x0A, 1, 2, 3, 4, 1, 0xA3]);
    let varcstr = VarcStrSmall {
        id: 0x0123,
        val: "ab",
        verify: VERIFY,
    };
    assert_eq!(VarcStrSmall::PROT.id(), 123);
    assert_eq!(encode(&varcstr), [0x23, 1, 0x0A, 0x8B, b'a', b'b', 0]);
    assert_eq!(VarcStrSmall::sizeof_str("ab"), varcstr.sizeof());
    let varcstr = VarcStrLarge {
        id: 0x0123,
        val: "ab",
        verify: VERIFY,
    };
    assert_eq!(VarcStrLarge::PROT.id(), 48);
    assert_eq!(encode(&varcstr), [0x0A, 0x0B, b'a', b'b', 0, 0xA3, 1]);
}

#[test]
fn camera_packets_lead_with_the_verify_id() {
    let shake = CamShake {
        axis: 1,
        ran: 2,
        amp: 3,
        rate: 4,
        cycle: 0x0506,
        verify: VERIFY,
    };
    assert_eq!(CamShake::PROT.id(), 27);
    assert_eq!(encode(&shake), [0x0A, 0x0B, 1, 2, 3, 4, 5, 6]);
    let lookat = CamLookat {
        local_x: 10,
        local_z: 20,
        height: 400,
        rate: 5,
        rate2: 100,
        verify: VERIFY,
    };
    assert_eq!(CamLookat::PROT.id(), 125);
    assert_eq!(encode(&lookat), [0x0A, 0x0B, 10, 20, 1, 0x90, 5, 100]);
    let moveto = CamMoveto {
        local_x: 10,
        local_z: 20,
        height: 400,
        rate: 5,
        rate2: 100,
        verify: VERIFY,
    };
    assert_eq!(CamMoveto::PROT.id(), 154);
    assert_eq!(encode(&moveto), [0x0A, 0x0B, 10, 20, 1, 0x90, 5, 100]);
    let forceangle = CamForceangle {
        yaw: 0x0102,
        pitch: 0x0304,
        verify: VERIFY,
    };
    assert_eq!(CamForceangle::PROT.id(), 187);
    assert_eq!(encode(&forceangle), [2, 1, 0x0A, 0x0B, 3, 4]);
    assert_eq!(CamReset::PROT.id(), 24);
    assert_eq!(encode(&CamReset { verify: VERIFY }), [0x0A, 0x0B]);
}

#[test]
fn run_client_script_reads_the_params_last_first() {
    let script = RunClientScript {
        script_id: 108,
        params: vec![ScriptParam::Int(5), ScriptParam::Str("ab".into())],
        verify: VERIFY,
    };
    assert_eq!(RunClientScript::PROT.id(), 115);
    assert_eq!(
        encode(&script),
        [
            0x0A, 0x0B, b'i', b's', 0, b'a', b'b', 0, 0, 0, 0, 5, 0, 0, 0, 108
        ]
    );
}

#[test]
fn player_menu_and_stats_match_the_530_readers() {
    let op = SetPlayerOp {
        op: 3,
        value: "Attack",
        primary: 0,
    };
    assert_eq!(SetPlayerOp::PROT.id(), 44);
    assert_eq!(
        encode(&op),
        [0x7F, 0xFF, 0, 3, b'A', b't', b't', b'a', b'c', b'k', 0]
    );
    let stat = UpdateStat {
        stat: 3,
        exp: COM,
        lvl: 99,
    };
    assert_eq!(UpdateStat::PROT.id(), 38);
    assert_eq!(encode(&stat), [0xE3, 3, 4, 1, 2, 3]);
    assert_eq!(SetMoveaction::PROT.id(), 160);
    let label = SetMoveaction { label: "Go".into() };
    assert_eq!(encode(&label), [b'G', b'o', 0]);
    let default = SetMoveaction {
        label: String::new(),
    };
    assert!(encode(&default).is_empty());
    assert_eq!(UnsetMapFlag::PROT.id(), 153);
    assert!(encode(&UnsetMapFlag).is_empty());
}

#[test]
fn update_inv_full_sends_the_count_before_the_obj() {
    let slots = vec![
        Some(InvItem {
            id: TypeId::new(995),
            count: 1000,
        }),
        None,
        Some(InvItem {
            id: TypeId::new(4151),
            count: 1,
        }),
    ];
    let own = UpdateInvFull {
        com: 149 << 16,
        inv_id: TypeId::new(93),
        other: false,
        capacity: 3,
        slots: slots.clone(),
    };
    assert_eq!(UpdateInvFull::PROT.id(), 105);
    assert_eq!(
        encode(&own),
        [
            0, 149, 0, 0, 0, 93, 0, 3, 0x81, 0, 0, 3, 232, 3, 228, 0x80, 0, 0, 0x7F, 0x10, 0x38
        ]
    );
    let other = UpdateInvFull {
        com: -1,
        inv_id: TypeId::new(93),
        other: true,
        capacity: 3,
        slots,
    };
    assert_eq!(encode(&other)[..4], [0xFF, 0xFE, 0xEE, 0x8F]);
    assert_eq!(UpdateInvStopTransmit::PROT.id(), 191);
    assert_eq!(encode(&UpdateInvStopTransmit { inv_id: 93 }), [93, 0]);
}

#[test]
fn hint_arrow_has_four_slots_and_no_level_or_range() {
    let tile = HintArrow {
        slot: 3,
        icon: 2,
        kind: HintArrowKind::Coord {
            offset: 2,
            x: 3222,
            z: 3218,
            level: 1,
            height: 40,
            range: 65535,
        },
        model_id: TypeId::NULL,
    };
    assert_eq!(HintArrow::PROT.id(), 217);
    assert_eq!(
        encode(&tile),
        [0xC2, 2, 0x0C, 0x96, 0x0C, 0x92, 20, 0xFF, 0xFF]
    );
    let npc = HintArrow {
        slot: 1,
        icon: 0,
        kind: HintArrowKind::Npc { npc_index: 300 },
        model_id: TypeId::NULL,
    };
    assert_eq!(encode(&npc), [0x41, 0, 1, 44, 0, 0, 0, 0xFF, 0xFF]);
    let player = HintArrow {
        slot: 0,
        icon: 0,
        kind: HintArrowKind::Player { player_index: 5 },
        model_id: TypeId::NULL,
    };
    assert_eq!(encode(&player), [10, 0, 0, 5, 0, 0, 0, 0xFF, 0xFF]);
    let stop = HintArrow {
        slot: 2,
        icon: 0,
        kind: HintArrowKind::Stop,
        model_id: TypeId::NULL,
    };
    assert_eq!(encode(&stop), [0x80, 0, 0xFF, 0xFF, 0, 0, 0, 0, 0]);
}

#[test]
fn shared_misc_packets_fit_the_530_size_table() {
    assert_eq!(UpdateRunWeight::PROT.id(), 159);
    assert_eq!(encode(&UpdateRunWeight { kg: 0xFFFE }), [0xFF, 0xFE]);
    assert_eq!(UpdateRunEnergy::PROT.id(), 234);
    assert_eq!(encode(&UpdateRunEnergy { energy: 100 }), [100]);
    assert_eq!(UpdateRebootTimer::PROT.id(), 85);
    assert_eq!(encode(&UpdateRebootTimer { clocks: 500 }), [1, 0xF4]);
    assert_eq!(MinimapToggle::PROT.id(), 192);
    assert_eq!(encode(&MinimapToggle { minimap_type: 2 }), [2]);
    assert_eq!(Logout::PROT.id(), 86);
    assert!(encode(&Logout).is_empty());
    assert_eq!(ResetAnims::PROT.id(), 131);
    assert!(encode(&ResetAnims).is_empty());
    assert_eq!(ResetClientVarCache::PROT.id(), 89);
    assert!(encode(&ResetClientVarCache).is_empty());
    assert_eq!(VarpSync::PROT.id(), 128);
    assert!(encode(&VarpSync).is_empty());
    assert_eq!(UrlOpen::PROT.id(), 42);
    assert_eq!(encode(&UrlOpen { url: "a" }), [b'a']);
    let checks = [ReflectionCheck::FieldValue {
        class: "a".into(),
        field: "b".into(),
    }];
    let reflection = ReflectionChecker {
        id: 7,
        checks: &checks,
    };
    assert_eq!(ReflectionChecker::PROT.id(), 114);
    assert_eq!(encode(&reflection), [1, 0, 0, 0, 7, 0, b'a', 0, b'b', 0]);
}

const USER: i64 = 0x0102030405060708;
const CHANNEL: i64 = 0x1112131415161718;
const UID: i32 = 0x0A0B0C0D;

fn name(user37: i64) -> [u8; 8] {
    user37.to_be_bytes()
}

#[test]
fn private_chat_sends_base37_names_and_a_split_uid() {
    let private = MessagePrivate {
        user37: USER,
        id: UID,
        level: 2,
        bytes: &[3, 0x41, 0x42],
    };
    assert_eq!(MessagePrivate::PROT.id(), 0);
    assert_eq!(
        encode(&private),
        [
            &name(USER)[..],
            &[0, 0x0A, 0x0B, 0x0C, 0x0D, 2, 3, 0x41, 0x42]
        ]
        .concat()
    );
    let echo = MessagePrivateEcho {
        user37: USER,
        bytes: &[1, 0x41],
    };
    assert_eq!(MessagePrivateEcho::PROT.id(), 71);
    assert_eq!(encode(&echo), [&name(USER)[..], &[1, 0x41]].concat());
    let quickchat = MessageQuickchatPrivate {
        user37: USER,
        id: UID,
        level: 1,
        phrase: TypeId::new(0x1234),
        bytes: &[7],
    };
    assert_eq!(MessageQuickchatPrivate::PROT.id(), 247);
    assert_eq!(
        encode(&quickchat),
        [
            &name(USER)[..],
            &[0, 0x0A, 0x0B, 0x0C, 0x0D, 1, 0x12, 0x34, 7]
        ]
        .concat()
    );
    let quickchat_echo = MessageQuickchatPrivateEcho {
        user37: USER,
        phrase: TypeId::new(0x1234),
        bytes: &[7],
    };
    assert_eq!(MessageQuickchatPrivateEcho::PROT.id(), 141);
    assert_eq!(
        encode(&quickchat_echo),
        [&name(USER)[..], &[0x12, 0x34, 7]].concat()
    );
}

#[test]
fn friendchat_messages_skip_a_byte_before_the_channel() {
    let message = MessageFriendchat {
        user37: USER,
        channel37: CHANNEL,
        id: UID,
        level: 3,
        bytes: &[1, 0x41],
    };
    assert_eq!(MessageFriendchat::PROT.id(), 54);
    assert_eq!(
        encode(&message),
        [
            &name(USER)[..],
            &[0],
            &name(CHANNEL),
            &[0, 0x0A, 0x0B, 0x0C, 0x0D, 3, 1, 0x41]
        ]
        .concat()
    );
    let quickchat = MessageQuickchatFriendchat {
        user37: USER,
        channel37: CHANNEL,
        id: UID,
        level: 0,
        phrase: TypeId::new(0x1234),
        bytes: &[],
    };
    assert_eq!(MessageQuickchatFriendchat::PROT.id(), 81);
    assert_eq!(
        encode(&quickchat),
        [
            &name(USER)[..],
            &[0],
            &name(CHANNEL),
            &[0, 0x0A, 0x0B, 0x0C, 0x0D, 0, 0x12, 0x34]
        ]
        .concat()
    );
}

#[test]
fn friendchat_channel_lists_members_by_base37_name() {
    let members = [
        FriendchatMember {
            user37: CHANNEL,
            world: 2,
            rank: -1,
            world_name: "W2",
        },
        FriendchatMember {
            user37: USER,
            world: 0x0101,
            rank: 7,
            world_name: "",
        },
    ];
    let full = UpdateFriendchatChannelFull {
        owner37: USER,
        channel37: CHANNEL,
        min_rank: -1,
        members: &members,
    };
    assert_eq!(UpdateFriendchatChannelFull::PROT.id(), 55);
    assert_eq!(
        encode(&full),
        [
            &name(USER)[..],
            &name(CHANNEL),
            &[0xFF, 2],
            &name(CHANNEL),
            &[0, 2, 0xFF, b'W', b'2', 0],
            &name(USER),
            &[1, 1, 7, 0],
        ]
        .concat()
    );
    let left = UpdateFriendchatChannelFull {
        owner37: 0,
        channel37: CHANNEL,
        min_rank: 0,
        members: &[],
    };
    assert_eq!(encode(&left), [0; 8]);

    let joined = UpdateFriendchatChannelSingleuser {
        user37: USER,
        world: 2,
        rank: 1,
        world_name: "W2",
    };
    assert_eq!(UpdateFriendchatChannelSingleuser::PROT.id(), 196);
    assert_eq!(
        encode(&joined),
        [&name(USER)[..], &[0, 2, 1, b'W', b'2', 0]].concat()
    );
    let removed = UpdateFriendchatChannelSingleuser {
        user37: USER,
        world: 2,
        rank: -128,
        world_name: "W2",
    };
    assert_eq!(
        encode(&removed),
        [&name(USER | i64::MIN)[..], &[0, 2, 0x80]].concat()
    );
}

#[test]
fn update_friendlist_sends_one_friend_with_the_flag_in_bit_63() {
    let online = UpdateFriendList {
        user37: USER,
        world: 3,
        rank: 4,
        world_name: "W3",
        world_flag: 1,
    };
    assert_eq!(UpdateFriendList::PROT.id(), 62);
    assert_eq!(
        encode(&online),
        [&name(USER)[..], &[0, 3, 4, b'W', b'3', 0]].concat()
    );
    let offline = UpdateFriendList {
        user37: USER,
        world: 0,
        rank: 4,
        world_name: "",
        world_flag: 0,
    };
    assert_eq!(
        encode(&offline),
        [&name(USER | i64::MIN)[..], &[0, 0, 4]].concat()
    );
}

#[test]
fn sound_and_text_packets_match_the_530_readers() {
    assert_eq!(MidiSong::PROT.id(), 4);
    assert_eq!(encode(&MidiSong { id: 0x1234 }), [0xB4, 0x12]);
    assert_eq!(MidiJingle::PROT.id(), 208);
    assert_eq!(
        encode(&MidiJingle {
            id: 0x1234,
            delay: 0x0506
        }),
        [0x06, 0x05, 0, 0x34, 0x12]
    );
    assert_eq!(LastLoginInfo::PROT.id(), 164);
    assert_eq!(
        encode(&LastLoginInfo { login: 0x7F000001 }),
        [0, 1, 0x7F, 0]
    );
    assert_eq!(MessageGame::PROT.id(), 70);
    assert_eq!(
        encode(&MessageGame { text: "@red@Hi" }),
        *b"<col=ff0000>Hi\0"
    );
    assert_eq!(
        encode(&MessageGame {
            text: "Bob:tradereq:"
        }),
        *b"Bob:tradereq:\0"
    );
    let sound = SoundArea {
        coord: 0x35,
        sound: 0x1234,
        info: 5 << 4 | 2,
        delay: 9,
    };
    assert_eq!(SoundArea::PROT.id(), 97);
    assert_eq!(encode(&sound), [0x35, 0x12, 0x34, 0x52, 9]);
}

#[test]
fn zone_headers_have_no_level() {
    assert_eq!(UpdateZonePartialFollows::PROT.id(), 26);
    assert_eq!(
        encode(&UpdateZonePartialFollows { x: 0x30, z: 0x48 }),
        [0xD0, 0x48]
    );
    assert_eq!(UpdateZoneFullFollows::PROT.id(), 112);
    assert_eq!(
        encode(&UpdateZoneFullFollows { x: 0x30, z: 0x48 }),
        [0x30, 0xB8]
    );
    let enclosed = UpdateZonePartialEnclosed {
        x: 0x30,
        z: 0x48,
        bytes: &[ZoneProt::LocDel as u8, 0xD5, 0x35],
    };
    assert_eq!(UpdateZonePartialEnclosed::PROT.id(), 230);
    assert_eq!(encode(&enclosed), [0xC8, 0x50, 195, 0xD5, 0x35]);
}

#[test]
fn zone_prot_ids_are_server_prot_ids() {
    let mut ids = [
        ZoneProt::ObjAdd as u8,
        ZoneProt::ObjDel as u8,
        ZoneProt::ObjCount as u8,
        ZoneProt::ObjReveal as u8,
        ZoneProt::LocAddChange as u8,
        ZoneProt::LocDel as u8,
        ZoneProt::LocAnim as u8,
        ZoneProt::LocMerge as u8,
        ZoneProt::MapAnim as u8,
        ZoneProt::MapProjAnim as u8,
        ZoneProt::SoundArea as u8,
        ZoneProt::MapProjAnimHalfsq as u8,
        ZoneProt::MapProjAnimHalfsqSrc as u8,
    ];
    ids.sort_unstable();
    assert_eq!(
        ids,
        [14, 16, 17, 20, 33, 97, 104, 121, 135, 179, 195, 202, 240]
    );
}

#[test]
fn obj_and_loc_packets_match_the_qd_readers() {
    let add = ObjAdd {
        coord: 0x35,
        id: TypeId::new(0x1234),
        count: 0x0102,
    };
    assert_eq!(encode(&add), [0x34, 0x12, 0x35, 0x01, 0x82]);
    let del = ObjDel {
        coord: 0x35,
        id: TypeId::new(0x1234),
    };
    assert_eq!(encode(&del), [0x4B, 0x12, 0x34]);
    let reveal = ObjReveal {
        coord: 0x35,
        id: TypeId::new(0x1234),
        count: 0x0102,
        receiver: 0x0506,
    };
    assert_eq!(encode(&reveal), [0x86, 0x05, 0xCB, 0x02, 0x01, 0x34, 0x12]);
    let count = ObjCount {
        coord: 0x35,
        id: TypeId::new(0x1234),
        old_count: 1,
        new_count: 2,
    };
    assert_eq!(encode(&count), [0x35, 0x12, 0x34, 0, 1, 0, 2]);
    let change = LocAddChange {
        coord: 0x35,
        shape_angle: 0x2B,
        id: TypeId::new(0x1234),
    };
    assert_eq!(encode(&change), [0xAB, 0x35, 0x12, 0xB4]);
    let loc_del = LocDel {
        coord: 0x35,
        shape_angle: 0x2B,
    };
    assert_eq!(encode(&loc_del), [0xD5, 0x35]);
    let anim = LocAnim {
        coord: 0x35,
        shape_angle: 0x29,
        seq: TypeId::new(0x1234),
    };
    assert_eq!(encode(&anim), [0x4B, 0x57, 0x34, 0x12]);
    let map_anim = MapAnim {
        coord: 0x35,
        spotanim: TypeId::new(0x1234),
        height: 7,
        delay: 0x0102,
    };
    assert_eq!(encode(&map_anim), [0x35, 0x12, 0x34, 7, 1, 2]);
}

#[test]
fn loc_merge_sends_the_player_as_a_negative_entity() {
    let merge = LocMerge {
        coord: 0x35,
        shape_angle: 0x2B,
        id: TypeId::new(0x1234),
        start: 0x10,
        end: 0x20,
        pid: 5,
        east: -1,
        south: -2,
        west: 1,
        north: 2,
    };
    assert_eq!(LocMerge::PROT.id(), 202);
    assert_eq!(
        encode(&merge),
        [
            0x2B, 0x35, 0x7F, 0x7E, 0x7E, 0x12, 0xB4, 0x20, 0, 1, 0, 0x10, 0x7A, 0xFF
        ]
    );
}

#[test]
fn projectiles_send_the_source_only_in_the_enclosed_form() {
    let tile = MapProjAnim {
        coord: 0x35,
        dx: -2,
        dz: 3,
        target: -5,
        spotanim: TypeId::new(0x1234),
        src_height: 10,
        dst_height: 20,
        start_delay: 0x0102,
        end_delay: 0x0304,
        peak: 16,
        arc: 64,
    };
    assert_eq!(MapProjAnim::PROT.id(), 16);
    assert_eq!(
        encode(&tile),
        [
            0x35, 0xFE, 3, 0xFF, 0xFB, 0x12, 0x34, 10, 20, 1, 2, 3, 4, 16, 64
        ]
    );
    let halfsq = MapProjAnimHalfsq {
        coord: 0xA5,
        dx: -2,
        dz: 3,
        target: -5,
        spotanim: TypeId::new(0x1234),
        src_height: 10,
        dst_height: 20,
        start_delay: 0x0102,
        end_delay: 0x0304,
        peak: 16,
        arc: 64,
    };
    assert_eq!(MapProjAnimHalfsq::PROT.id(), 121);
    assert_eq!(
        encode(&halfsq),
        [
            0xA5, 0xFE, 3, 0xFF, 0xFB, 0x12, 0x34, 10, 20, 1, 2, 3, 4, 16, 64
        ]
    );

    let sourced = |source, wearpos| {
        let projectile = MapProjAnimHalfsqSrc {
            coord: 0xA5,
            dx: -2,
            dz: 3,
            source,
            wearpos,
            target: 7,
            spotanim: TypeId::new(0x1234),
            src_height: -10i8 as u8,
            dst_height: 20,
            start_delay: 0x0102,
            end_delay: 0x0304,
            peak: 16,
            arc: 64,
        };
        let mut buf = Packet::new(projectile.sizeof());
        projectile.encode(&mut buf);
        assert_eq!(buf.pos, 17);
        buf.data
    };
    assert_eq!(MapProjAnimHalfsqSrc::PROT.id(), 104);
    assert_eq!(SIZES[104], 0);
    let tail = [0, 7, 0x12, 0x34, 0xF6, 20, 1, 2, 3, 4, 16, 64];
    assert_eq!(
        sourced(-6, 3),
        [&[0xA5, 0xFE, 3, 0xE7, 0xFA][..], &tail].concat()
    );
    assert_eq!(sourced(6, 0), [&[0xA5, 0xFE, 3, 0, 6][..], &tail].concat());
    assert_eq!(sourced(0, 3), [&[0xA5, 0xFE, 3, 0, 0][..], &tail].concat());
    assert_eq!(
        sourced(2049, 3),
        [&[0xA5, 0xFE, 3, 0, 0][..], &tail].concat()
    );
    assert_eq!(
        sourced(2048, 15),
        [&[0xA5, 0xFE, 3, 0, 0][..], &tail].concat()
    );
    assert_eq!(
        sourced(-2048, 15),
        [&[0xA5, 0xFE, 3, 0x80, 0][..], &tail].concat()
    );
}

#[test]
fn specific_anims_match_the_530_readers() {
    let loc = LocAnimSpecific {
        seq: TypeId::new(0x1234),
        shape_angle: 0x29,
        coord: 1 << 28 | 3222 << 14 | 3218,
    };
    assert_eq!(LocAnimSpecific::PROT.id(), 235);
    assert_eq!(encode(&loc), [0x57, 0x12, 0x34, 0x13, 0x25, 0x8C, 0x92]);
    let spotanim = SpotanimSpecific {
        target: 1 << 29 | 5,
        spotanim: TypeId::new(0x1234),
        height: 0x0304,
        delay: 0x0102,
    };
    assert_eq!(SpotanimSpecific::PROT.id(), 56);
    assert_eq!(encode(&spotanim), [1, 2, 4, 3, 0, 5, 0x20, 0, 0xB4, 0x12]);
    let npc = NpcAnimSpecific {
        npc_uid: 0x0102,
        delay: 5,
        seq: 0x1234,
    };
    assert_eq!(NpcAnimSpecific::PROT.id(), 102);
    assert_eq!(encode(&npc), [2, 1, 0x7B, 0x12, 0x34]);
}

#[test]
fn shared_social_packets_fit_the_530_size_table() {
    assert_eq!(FriendListLoaded::PROT.id(), 197);
    assert_eq!(encode(&FriendListLoaded { status: 2 }), [2]);
    let filters = ChatFilterSettings {
        public: 0,
        private: 1,
        trade: 2,
    };
    assert_eq!(ChatFilterSettings::PROT.id(), 232);
    assert_eq!(encode(&filters), [0, 1, 2]);
    let synth = SynthSound {
        synth: 0x1234,
        loops: 1,
        delay: 0x0102,
    };
    assert_eq!(SynthSound::PROT.id(), 172);
    assert_eq!(encode(&synth), [0x12, 0x34, 1, 1, 2]);
    assert_eq!(UpdateIgnoreList::PROT.id(), 126);
    assert_eq!(encode(&UpdateIgnoreList { users37: &[USER] }), name(USER));
}

fn block(block: &impl InfoMessage) -> Vec<u8> {
    let mut buf = Packet::new(block.sizeof());
    block.encode(&mut buf);
    assert_eq!(buf.pos, block.sizeof());
    buf.data
}

#[test]
fn player_masks_follow_the_530_reader_order() {
    let order = [
        (PlayerInfoProt::Chat, 0x80),
        (PlayerInfoProt::Damage, 0x1),
        (PlayerInfoProt::Anim, 0x8),
        (PlayerInfoProt::Appearance, 0x4),
        (PlayerInfoProt::FaceEntity, 0x2),
        (PlayerInfoProt::ExactMove, 0x400),
        (PlayerInfoProt::Say, 0x20),
        (PlayerInfoProt::Damage2, 0x200),
        (PlayerInfoProt::ModelAnims, 0x800),
        (PlayerInfoProt::SpotAnim, 0x100),
        (PlayerInfoProt::FaceCoord, 0x40),
    ];
    assert_eq!(PlayerInfoProt::COUNT, order.len());
    for (index, (prot, bit)) in order.into_iter().enumerate() {
        assert_eq!(prot as u16, bit);
        assert_eq!(prot.to_index(), index);
    }
    assert_eq!(PlayerInfoProt::BigInfo as u16, 0x10);
    assert_eq!(PlayerInfoProt::header(0x43), ([0x43, 0, 0], 1));
    assert_eq!(PlayerInfoProt::header(0x408), ([0x18, 0x04, 0], 2));
}

#[test]
fn npc_masks_follow_the_530_reader_order_under_a_two_byte_header() {
    let order = [
        (NpcInfoProt::Damage, 0x40),
        (NpcInfoProt::Damage2, 0x2),
        (NpcInfoProt::Anim, 0x10),
        (NpcInfoProt::FaceEntity, 0x4),
        (NpcInfoProt::SpotAnim, 0x80),
        (NpcInfoProt::ChangeType, 0x1),
        (NpcInfoProt::Say, 0x20),
        (NpcInfoProt::ModelAnims, 0x100),
        (NpcInfoProt::FaceCoord, 0x200),
    ];
    assert_eq!(NpcInfoProt::COUNT, order.len());
    for (index, (prot, bit)) in order.into_iter().enumerate() {
        assert_eq!(prot as u16, bit);
        assert_eq!(prot.to_index(), index);
    }
    assert_eq!(NpcInfoProt::BigInfo as u16, 0x8);
    assert_eq!(NpcInfoProt::header(0xF7), ([0xF7, 0, 0], 1));
    assert_eq!(NpcInfoProt::header(0x214), ([0x1C, 0x02, 0], 2));
}

#[test]
fn player_blocks_match_the_530_readers() {
    let chat = PlayerInfoChat {
        colour: 0x83,
        effects: 1,
        crown: 2,
        bytes: &[2, 0x48, 0x69],
    };
    assert_eq!(block(&chat), [1, 0x83, 2, 3, 0x69, 0x48, 2]);
    let damage = PlayerInfoDamage {
        damage: 300,
        damage_type: 1,
        health: health(50, 100),
    };
    assert_eq!(block(&damage), [0x81, 0x2C, 0x81, 1]);
    let small = PlayerInfoDamage {
        damage: 5,
        damage_type: 0,
        health: 255,
    };
    assert_eq!(block(&small), [5, 0x80, 0x81]);
    let past_a_smart = PlayerInfoDamage2 {
        damage: u16::MAX,
        damage_type: 2,
    };
    assert_eq!(block(&past_a_smart), [0xFF, 0xFF, 0x7E]);
    let anim = PlayerInfoAnim {
        seq: TypeId::new(0x1234),
        delay: 5,
    };
    assert_eq!(block(&anim), [0x12, 0x34, 5]);
    let face = PlayerInfoFaceEntity { entity: 0x8005 };
    assert_eq!(block(&face), [0x80, 0x85]);
    let exactmove = PlayerInfoExactMove {
        start_x: 10,
        start_z: 20,
        end_x: 30,
        end_z: 40,
        begin: 0x0102,
        finish: 0x0304,
        dir: 2,
    };
    assert_eq!(block(&exactmove), [0xF6, 20, 0x9E, 40, 2, 1, 4, 3, 0xFE]);
    assert_eq!(block(&PlayerInfoSay { say: "hi" }), [b'h', b'i', 0]);
    let spotanim = PlayerInfoSpotAnim {
        spotanim: TypeId::new(0x0123),
        height: 92,
        delay: 5,
    };
    assert_eq!(block(&spotanim), [0x23, 0x01, 0x5C, 0, 5, 0]);
    let coord = PlayerInfoFaceCoord {
        x: 0x192F,
        z: 0x1925,
    };
    assert_eq!(block(&coord), [0x19, 0x2F, 0xA5, 0x19]);
    let anims = [
        ModelAnim {
            seq: 0x0102,
            delay: 3,
            slots: 0x0801,
        },
        ModelAnim {
            seq: u16::MAX,
            delay: 0,
            slots: 1,
        },
    ];
    let anims = PlayerInfoModelAnims { anims: &anims };
    assert_eq!(
        block(&anims),
        [0xFE, 2, 1, 0x83, 8, 1, 0xFF, 0xFF, 0x80, 0, 1]
    );
}

#[test]
fn npc_blocks_match_the_530_readers() {
    let damage = NpcInfoDamage {
        damage: 50,
        damage_type: 1,
        health: health(200, 255),
    };
    assert_eq!(block(&damage), [50, 0xFF, 0xB8]);
    let damage2 = NpcInfoDamage2 {
        damage: 9,
        damage_type: 3,
    };
    assert_eq!(block(&damage2), [0xF7, 0x7D]);
    let anim = NpcInfoAnim {
        seq: TypeId::new(0x1234),
        delay: 5,
    };
    assert_eq!(block(&anim), [0x12, 0x34, 5]);
    assert_eq!(block(&NpcInfoFaceEntity { entity: 0x8005 }), [0x80, 0x85]);
    let spotanim = NpcInfoSpotAnim {
        spotanim: TypeId::new(0x0123),
        height: 92,
        delay: 5,
    };
    assert_eq!(block(&spotanim), [0x01, 0xA3, 5, 0, 0x5C, 0]);
    assert_eq!(
        block(&NpcInfoChangeType {
            ntype: TypeId::new(0x1087)
        }),
        [0x87, 0x10]
    );
    assert_eq!(block(&NpcInfoSay { say: "hi" }), [b'h', b'i', 0]);
    let anims = [ModelAnim {
        seq: 0x0102,
        delay: 3,
        slots: 0x0801,
    }];
    let anims = NpcInfoModelAnims { anims: &anims };
    assert_eq!(block(&anims), [0xFF, 2, 1, 0x7D, 8, 1]);
    let coord = NpcInfoFaceCoord {
        x: 0x192F,
        z: 0x1925,
    };
    assert_eq!(block(&coord), [0x19, 0xAF, 0x19, 0x25]);
}

#[test]
fn add_entries_face_by_the_classic_direction_index() {
    let angles = [768, 1024, 1280, 512, 1536, 256, 0, 1792];
    for ((dx, dz), angle) in [
        ((0, 2), 0),
        ((2, 2), 256),
        ((2, 0), 512),
        ((2, -2), 768),
        ((0, -2), 1024),
        ((-2, -2), 1280),
        ((-2, 0), 1536),
        ((-2, 2), 1792),
    ] {
        let direction = add_direction(Some((dx, dz)));
        assert_eq!(angles[direction as usize], angle, "({dx}, {dz})");
    }
    assert_eq!(angles[add_direction(None) as usize], 0);
    assert_eq!(angles[add_direction(Some((0, 0))) as usize], 0);
}
