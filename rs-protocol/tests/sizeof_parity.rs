use std::any::type_name;
use std::collections::BTreeSet;

use rs_io::Packet;
use rs_protocol::network::game::server::*;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_protocol::network::game::server_prot_message::ServerProtMessage;

const PAD: usize = 512;

fn check<M: ServerProtMessage>(seen: &mut BTreeSet<String>, msg: M) {
    let sizeof = msg.sizeof();
    let mut buf = Packet::new(sizeof + PAD);
    msg.encode(&mut buf);
    assert_eq!(
        sizeof,
        buf.pos,
        "{}: sizeof() reports {} byte(s) but encode() wrote {}",
        type_name::<M>(),
        sizeof,
        buf.pos,
    );
    seen.insert(format!("{:?}", M::PROT));
}

const ASCII: &str = "Sir Prysin says you can have his sword.";
const NON_ASCII: &str = "cafe\u{301} \u{a3}5 \u{bd} \u{dc}nicode \u{2603}";

#[test]
fn sizeof_matches_encode_for_every_server_packet() {
    let seen = &mut BTreeSet::new();

    // Camera
    check(
        seen,
        cam_look_at::CamLookAt {
            x: 3,
            z: 9,
            height: 420,
            rate: 2,
            rate2: 5,
        },
    );
    check(
        seen,
        cam_move_to::CamMoveTo {
            x: 7,
            z: 1,
            height: 96,
            rate: 11,
            rate2: 3,
        },
    );
    check(seen, cam_reset::CamReset);
    check(
        seen,
        cam_shake::CamShake {
            direction: 2,
            jitter: 4,
            amplitude: 8,
            frequency: 16,
        },
    );

    // Chat / messaging
    check(
        seen,
        chat_filter_settings::ChatFilterSettings {
            public: 0,
            private: 1,
            trade: 2,
        },
    );
    check(seen, message_game::MessageGame { text: ASCII });
    check(seen, message_game::MessageGame { text: NON_ASCII });
    check(seen, message_game::MessageGame { text: "" });
    check(
        seen,
        message_private::MessagePrivate {
            user37: 0x1234_5678_9abc_def0,
            id: 77,
            level: 2,
            bytes: &[0xde, 0xad, 0xbe, 0xef],
        },
    );

    // Interfaces
    check(seen, if_close::IfClose);
    check(seen, if_openchat::IfOpenChat { com: 1234 });
    check(seen, if_openmain::IfOpenMain { com: 2000 });
    check(seen, if_openmain_side::IfOpenMainSide { com: 10, side: 20 });
    check(seen, if_openside::IfOpenSide { com: 3213 });
    check(seen, if_setanim::IfSetAnim { com: 5, seq: 808 });
    check(
        seen,
        if_setcolour::IfSetColour {
            com: 6,
            colour: 0x7fff,
        },
    );
    check(seen, if_sethide::IfSetHide { com: 7, hide: true });
    check(
        seen,
        if_setmodel::IfSetModel {
            com: 8,
            model: 1337,
        },
    );
    check(seen, if_setnpchead::IfSetNpcHead { com: 9, npc: 44 });
    check(
        seen,
        if_setobject::IfSetObject {
            com: 10,
            obj: 995,
            scale: 100,
        },
    );
    check(seen, if_setplayerhead::IfSetPlayerHead { com: 11 });
    check(
        seen,
        if_setposition::IfSetPosition {
            com: 12,
            x: 13,
            y: 14,
        },
    );
    check(seen, if_settab::IfSetTab { com: 15, tab: 3 });
    check(seen, if_settab_active::IfSetTabActive { tab: 6 });
    check(
        seen,
        if_settext::IfSetText {
            com: 16,
            text: ASCII,
        },
    );
    check(
        seen,
        if_settext::IfSetText {
            com: 16,
            text: NON_ASCII,
        },
    );
    check(seen, if_settext::IfSetText { com: 16, text: "" });
    #[cfg(before_245_2)]
    check(
        seen,
        if_setrecol::IfSetRecol {
            com: 17,
            src: 100,
            dst: 200,
        },
    );
    #[cfg(since_245_2)]
    check(seen, if_setscrollpos::IfSetScrollPos { com: 18, y: 250 });
    #[cfg(since_244)]
    check(seen, if_openoverlay::IfOpenOverlay { com: 19 });

    // Tutorial / hints
    check(seen, tut_flash::TutFlash { tab: 1 });
    check(seen, tut_open::TutOpen { com: 2 });
    check(
        seen,
        hint_arrow::HintArrow {
            hint: 2,
            arg1: 100,
            arg2: 200,
            arg3: 64,
        },
    );
    check(seen, p_countdialog::PCountDialog);

    // Zone: locs
    check(
        seen,
        loc_add_change::LocAddChange {
            coord: 0x42,
            shape_angle: 0x1f,
            id: 1276,
        },
    );
    check(
        seen,
        loc_anim::LocAnim {
            coord: 0x11,
            shape_angle: 0x22,
            seq: 33,
        },
    );
    check(
        seen,
        loc_del::LocDel {
            coord: 0x33,
            shape_angle: 0x44,
        },
    );
    check(
        seen,
        loc_merge::LocMerge {
            coord: 0x55,
            shape_angle: 0x66,
            id: 77,
            start: 88,
            end: 99,
            pid: 1,
            east: -2,
            south: -3,
            west: 4,
            north: 5,
        },
    );

    // Zone: objs
    check(
        seen,
        obj_add::ObjAdd {
            coord: 0x12,
            id: 995,
            count: 65535,
        },
    );
    check(
        seen,
        obj_count::ObjCount {
            coord: 0x13,
            id: 995,
            old_count: 1,
            new_count: 2,
        },
    );
    check(
        seen,
        obj_del::ObjDel {
            coord: 0x14,
            id: 995,
        },
    );
    check(
        seen,
        obj_reveal::ObjReveal {
            coord: 0x15,
            id: 995,
            count: 3,
            receiver: 4,
        },
    );

    // Zone: effects
    check(
        seen,
        map_anim::MapAnim {
            coord: 0x16,
            spotanim: 111,
            height: 92,
            delay: 5,
        },
    );
    check(
        seen,
        map_projanim::MapProjAnim {
            coord: 0x17,
            dx: -1,
            dz: 2,
            target: -3,
            spotanim: 222,
            src_height: 30,
            dst_height: 40,
            start_delay: 10,
            end_delay: 20,
            peak: 15,
            arc: 64,
        },
    );
    #[cfg(since_289)]
    check(
        seen,
        sound_area::SoundArea {
            coord: 0x18,
            sound: 55,
            info: 0x21,
            delay: 7,
        },
    );

    // Zone framing
    check(
        seen,
        update_zone_full_follows::UpdateZoneFullFollows { x: 40, z: 50 },
    );
    check(
        seen,
        update_zone_partial_follows::UpdateZonePartialFollows { x: 41, z: 51 },
    );
    check(
        seen,
        update_zone_partial_enclosed::UpdateZonePartialEnclosed {
            x: 42,
            z: 52,
            bytes: &[1, 2, 3, 4, 5],
        },
    );
    check(
        seen,
        update_zone_partial_enclosed::UpdateZonePartialEnclosed {
            x: 42,
            z: 52,
            bytes: &[],
        },
    );

    // Entity info blocks
    check(seen, player_info::PlayerInfo { bytes: &[0; 37] });
    check(seen, player_info::PlayerInfo { bytes: &[] });
    check(seen, npc_info::NpcInfo { bytes: &[0; 21] });
    check(seen, npc_info::NpcInfo { bytes: &[] });

    // Inventory. The `count >= u8::MAX` arm swaps a 1-byte count for a 5-byte
    // one, so both arms plus the empty/hole/trailing-hole shapes matter.
    let stacks: &[Option<(u16, i32)>] = &[
        Some((995, 1)),
        None,
        Some((995, 254)),
        Some((995, 255)),
        Some((995, i32::MAX)),
        None,
    ];
    check(
        seen,
        update_inv_full::UpdateInvFull {
            com: 3214,
            objs: stacks,
        },
    );
    check(
        seen,
        update_inv_full::UpdateInvFull {
            com: 3214,
            objs: &[],
        },
    );
    check(
        seen,
        update_inv_full::UpdateInvFull {
            com: 3214,
            objs: &[None, None],
        },
    );
    let partial: &[(u16, Option<(u16, i32)>)] = &[
        (0, Some((995, 1))),
        (7, None),
        (27, Some((995, 255))),
        (28, Some((995, i32::MAX))),
    ];
    check(
        seen,
        update_inv_partial::UpdateInvPartial {
            com: 3214,
            objs: partial,
        },
    );
    check(
        seen,
        update_inv_partial::UpdateInvPartial {
            com: 3214,
            objs: &[],
        },
    );
    check(
        seen,
        update_inv_stop_transmit::UpdateInvStopTransmit { com: 3214 },
    );

    // Social
    check(
        seen,
        update_friendlist::UpdateFriendList {
            user37: 1234567,
            node: 10,
        },
    );
    check(
        seen,
        update_ignorelist::UpdateIgnoreList {
            users37: &[1, 2, 3],
        },
    );
    check(seen, update_ignorelist::UpdateIgnoreList { users37: &[] });
    #[cfg(since_254)]
    check(seen, friendlist_loaded::FriendListLoaded { status: 2 });
    #[cfg(since_254)]
    check(
        seen,
        set_player_op::SetPlayerOp {
            op: 3,
            value: ASCII,
            primary: 1,
        },
    );
    #[cfg(since_254)]
    check(
        seen,
        set_player_op::SetPlayerOp {
            op: 3,
            value: NON_ASCII,
            primary: 0,
        },
    );

    // Player state
    check(
        seen,
        update_stat::UpdateStat {
            stat: 3,
            exp: 14_000_000,
            lvl: 99,
        },
    );
    check(seen, update_runenergy::UpdateRunEnergy { energy: 100 });
    check(seen, update_runweight::UpdateRunWeight { kg: 64 });
    check(seen, varp_small::VarpSmall { id: 175, val: 200 });
    check(
        seen,
        varp_large::VarpLarge {
            id: 176,
            val: -123456,
        },
    );
    check(seen, reset_client_varcache::ResetClientVarCache);
    check(seen, reset_anims::ResetAnims);
    check(seen, set_multiway::SetMultiway { hide: true });
    check(seen, unset_map_flag::UnsetMapFlag);
    check(seen, logout::Logout);
    check(seen, update_reboot_timer::UpdateRebootTimer { clocks: 500 });

    // Audio
    check(
        seen,
        synth_sound::SynthSound {
            synth: 100,
            loops: 1,
            delay: 20,
        },
    );
    #[cfg(rev = "225")]
    check(
        seen,
        midi_jingle::MidiJingle {
            delay: 5,
            bytes: &[0x4d, 0x54, 0x68, 0x64],
        },
    );
    #[cfg(since_244)]
    check(seen, midi_jingle::MidiJingle { id: 90, delay: 5 });
    #[cfg(rev = "225")]
    check(
        seen,
        midi_song::MidiSong {
            name: "scape_main",
            crc: -12345,
            len: 4096,
        },
    );
    #[cfg(rev = "225")]
    check(
        seen,
        midi_song::MidiSong {
            name: NON_ASCII,
            crc: 1,
            len: 2,
        },
    );
    #[cfg(since_244)]
    check(seen, midi_song::MidiSong { id: 62 });

    // Login / session
    #[cfg(rev = "225")]
    check(
        seen,
        last_login_info::LastLoginInfo {
            ip: 0x7f00_0001,
            login: 300,
            recovery: 14,
            messages: 2,
        },
    );
    #[cfg(since_244)]
    check(
        seen,
        last_login_info::LastLoginInfo {
            ip: 0x7f00_0001,
            login: 300,
            recovery: 14,
            messages: 2,
            warn_members_in_non_members: true,
        },
    );
    #[cfg(rev = "225")]
    check(seen, update_pid::UpdatePid { pid: 1 });
    #[cfg(since_244)]
    check(
        seen,
        update_pid::UpdatePid {
            pid: 1,
            members: true,
        },
    );
    #[cfg(before_274)]
    check(seen, enable_tracking::EnableTracking);
    #[cfg(before_274)]
    check(seen, finish_tracking::FinishTracking);
    #[cfg(since_274)]
    check(seen, minimap_toggle::MinimapToggle { minimap_type: 2 });

    // Map loading
    #[cfg(rev = "225")]
    {
        use std::collections::{HashMap, HashSet};
        let mapsquares: HashSet<u16> = [(50u16 << 8) | 50, (50u16 << 8) | 51].into_iter().collect();
        let mut crcs: HashMap<(char, u8, u8), i32> = HashMap::new();
        crcs.insert(('m', 50, 50), 1234);
        // ('l', 50, 50) deliberately absent: encode writes a 0 placeholder for a
        // missing crc, so the size must not depend on which crcs are present.
        crcs.insert(('m', 50, 51), 5678);
        crcs.insert(('l', 50, 51), 9012);
        check(
            seen,
            rebuild_normal::RebuildNormal {
                zone_x: 400,
                zone_z: 400,
                mapsquares,
                crcs,
            },
        );
        check(
            seen,
            rebuild_normal::RebuildNormal {
                zone_x: 400,
                zone_z: 400,
                mapsquares: HashSet::new(),
                crcs: HashMap::new(),
            },
        );
    }
    #[cfg(since_244)]
    check(
        seen,
        rebuild_normal::RebuildNormal {
            zone_x: 400,
            zone_z: 400,
        },
    );

    // On-demand map streaming (225 only)
    #[cfg(rev = "225")]
    {
        check(
            seen,
            data_land::DataLand {
                x: 50,
                z: 50,
                off: 0,
                len: 4,
                data: &[1, 2, 3, 4],
            },
        );
        check(seen, data_land_done::DataLandDone { x: 50, z: 50 });
        check(
            seen,
            data_loc::DataLoc {
                x: 50,
                z: 50,
                off: 0,
                len: 4,
                data: &[1, 2, 3, 4],
            },
        );
        check(seen, data_loc_done::DataLocDone { x: 50, z: 50 });
    }

    coverage_is_complete(seen);
}

/// Fails if any `ServerProt` variant for the active revision has no case above.
/// `ServerProt::NAMES` is generated by the same macro that defines the enum, so
/// adding a packet without a parity case is a test failure, not a silent gap.
fn coverage_is_complete(seen: &BTreeSet<String>) {
    let declared: BTreeSet<String> = ServerProt::NAMES.iter().map(|s| s.to_string()).collect();
    let missing: Vec<&String> = declared.difference(seen).collect();
    assert!(
        missing.is_empty(),
        "rev {}: {} server packet(s) have no sizeof/encode parity case: {:?}",
        option_env!("REV").unwrap_or("default"),
        missing.len(),
        missing,
    );
    let unknown: Vec<&String> = seen.difference(&declared).collect();
    assert!(
        unknown.is_empty(),
        "covered packets not declared for this rev: {unknown:?}"
    );
}
