use rs_io::Packet;
use rs_protocol::network::game::client::ClientProtMessage;
use rs_protocol::network::game::client::anticheat_cyclelogic1::AnticheatCycleLogic1;
use rs_protocol::network::game::client::anticheat_cyclelogic2::AnticheatCycleLogic2;
use rs_protocol::network::game::client::anticheat_cyclelogic3::AnticheatCycleLogic3;
use rs_protocol::network::game::client::anticheat_cyclelogic4::AnticheatCycleLogic4;
use rs_protocol::network::game::client::anticheat_cyclelogic5::AnticheatCycleLogic5;
use rs_protocol::network::game::client::anticheat_cyclelogic6::AnticheatCycleLogic6;
#[cfg(since_254)]
use rs_protocol::network::game::client::anticheat_cyclelogic7::AnticheatCycleLogic7;
use rs_protocol::network::game::client::anticheat_oplogic1::AnticheatOpLogic1;
use rs_protocol::network::game::client::anticheat_oplogic2::AnticheatOpLogic2;
use rs_protocol::network::game::client::anticheat_oplogic3::AnticheatOpLogic3;
use rs_protocol::network::game::client::anticheat_oplogic4::AnticheatOpLogic4;
use rs_protocol::network::game::client::anticheat_oplogic5::AnticheatOpLogic5;
use rs_protocol::network::game::client::anticheat_oplogic6::AnticheatOpLogic6;
use rs_protocol::network::game::client::anticheat_oplogic7::AnticheatOpLogic7;
use rs_protocol::network::game::client::anticheat_oplogic8::AnticheatOpLogic8;
use rs_protocol::network::game::client::anticheat_oplogic9::AnticheatOpLogic9;
use rs_protocol::network::game::client::chat_setmode::ChatSetMode;
use rs_protocol::network::game::client::client_cheat::ClientCheat;
use rs_protocol::network::game::client::close_modal::CloseModal;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_applet_focus::EventAppletFocus;
#[cfg(any(rev = "225", since_254))]
use rs_protocol::network::game::client::event_camera_position::EventCameraPosition;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_mouse_click::EventMouseClick;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_mouse_move::EventMouseMove;
#[cfg(before_274)]
use rs_protocol::network::game::client::event_tracking::EventTracking;
use rs_protocol::network::game::client::friendlist_add::FriendListAdd;
use rs_protocol::network::game::client::friendlist_del::FriendListDel;
use rs_protocol::network::game::client::idk_savedesign::IdkSaveDesign;
use rs_protocol::network::game::client::idle_timer::IdleTimer;
use rs_protocol::network::game::client::if_button::IfButton;
use rs_protocol::network::game::client::ignorelist_add::IgnoreListAdd;
use rs_protocol::network::game::client::ignorelist_del::IgnoreListDel;
use rs_protocol::network::game::client::inv_button1::InvButton1;
use rs_protocol::network::game::client::inv_button2::InvButton2;
use rs_protocol::network::game::client::inv_button3::InvButton3;
use rs_protocol::network::game::client::inv_button4::InvButton4;
use rs_protocol::network::game::client::inv_button5::InvButton5;
use rs_protocol::network::game::client::inv_buttond::InvButtonD;
#[cfg(since_254)]
use rs_protocol::network::game::client::map_build_complete::MapBuildComplete;
use rs_protocol::network::game::client::message_private::MessagePrivate;
use rs_protocol::network::game::client::message_public::MessagePublic;
use rs_protocol::network::game::client::move_gameclick::MoveGameClick;
use rs_protocol::network::game::client::move_minimapclick::MoveMinimapClick;
use rs_protocol::network::game::client::move_opclick::MoveOpClick;
use rs_protocol::network::game::client::no_timeout::NoTimeout;
use rs_protocol::network::game::client::opheld1::OpHeld1;
use rs_protocol::network::game::client::opheld2::OpHeld2;
use rs_protocol::network::game::client::opheld3::OpHeld3;
use rs_protocol::network::game::client::opheld4::OpHeld4;
use rs_protocol::network::game::client::opheld5::OpHeld5;
use rs_protocol::network::game::client::opheldt::OpHeldT;
use rs_protocol::network::game::client::opheldu::OpHeldU;
use rs_protocol::network::game::client::oploc1::OpLoc1;
use rs_protocol::network::game::client::oploc2::OpLoc2;
use rs_protocol::network::game::client::oploc3::OpLoc3;
use rs_protocol::network::game::client::oploc4::OpLoc4;
use rs_protocol::network::game::client::oploc5::OpLoc5;
use rs_protocol::network::game::client::oploct::OpLocT;
use rs_protocol::network::game::client::oplocu::OpLocU;
use rs_protocol::network::game::client::opnpc1::OpNpc1;
use rs_protocol::network::game::client::opnpc2::OpNpc2;
use rs_protocol::network::game::client::opnpc3::OpNpc3;
use rs_protocol::network::game::client::opnpc4::OpNpc4;
use rs_protocol::network::game::client::opnpc5::OpNpc5;
use rs_protocol::network::game::client::opnpct::OpNpcT;
use rs_protocol::network::game::client::opnpcu::OpNpcU;
use rs_protocol::network::game::client::opobj1::OpObj1;
use rs_protocol::network::game::client::opobj2::OpObj2;
use rs_protocol::network::game::client::opobj3::OpObj3;
use rs_protocol::network::game::client::opobj4::OpObj4;
use rs_protocol::network::game::client::opobj5::OpObj5;
use rs_protocol::network::game::client::opobjt::OpObjT;
use rs_protocol::network::game::client::opobju::OpObjU;
use rs_protocol::network::game::client::opplayer1::OpPlayer1;
use rs_protocol::network::game::client::opplayer2::OpPlayer2;
use rs_protocol::network::game::client::opplayer3::OpPlayer3;
use rs_protocol::network::game::client::opplayer4::OpPlayer4;
#[cfg(since_254)]
use rs_protocol::network::game::client::opplayer5::OpPlayer5;
use rs_protocol::network::game::client::opplayert::OpPlayerT;
use rs_protocol::network::game::client::opplayeru::OpPlayerU;
#[cfg(rev = "225")]
use rs_protocol::network::game::client::rebuild_get_maps::RebuildGetMaps;
use rs_protocol::network::game::client::resume_p_countdialog::ResumePCountDialog;
use rs_protocol::network::game::client::resume_pause_button::ResumePauseButton;
use rs_protocol::network::game::client::send_snapshot::SendSnapshot;
use rs_protocol::network::game::client::tut_clickside::TutClickSide;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::client_prot_frame::ClientProtFrame;
use rs_protocol::network::game::client_prot_message::ClientProtMessageInfo;
use std::collections::BTreeSet;

const SPREAD: usize = 40;
const PAD: usize = 512;

const FILLS: [fn(usize) -> Vec<u8>; 3] = [
    |n| vec![0x00; n],
    |n| vec![0xff; n],
    |n| {
        (0..n)
            .map(|i| if i % 3 == 0 { 10 } else { i as u8 })
            .collect()
    },
];

fn check<M: ClientProtMessage>(seen: &mut BTreeSet<String>, name: &str) {
    let lens: Vec<usize> = match <M as ClientProtMessageInfo>::FRAME {
        ClientProtFrame::Fixed(len) => vec![len as usize],
        ClientProtFrame::VarByte { min } | ClientProtFrame::VarShort { min } => {
            (min as usize..=min as usize + SPREAD).collect()
        }
    };
    for len in lens {
        for (i, fill) in FILLS.iter().enumerate() {
            let mut data = fill(len);
            data.resize(len + PAD, 10);
            let mut buf = Packet::from(data);
            let _ = M::decode(&mut buf, len);
            assert!(
                buf.pos <= len,
                "{name}: decode() consumed {} byte(s) of a {len}-byte payload (fill {i})",
                buf.pos,
            );
        }
    }
    seen.insert(name.to_string());
}

macro_rules! check {
    ( $seen:expr, $( $ty:ident ),* $(,)? ) => {
        $( check::<$ty>(&mut $seen, stringify!($ty)); )*
    };
}

#[test]
fn decode_stays_in_bounds() {
    let mut seen = BTreeSet::new();

    #[cfg(since_254)]
    check!(seen, AnticheatCycleLogic7);
    #[cfg(since_254)]
    check!(seen, EventAppletFocus);
    #[cfg(any(rev = "225", since_254))]
    check!(seen, EventCameraPosition);
    #[cfg(since_254)]
    check!(seen, EventMouseClick);
    #[cfg(since_254)]
    check!(seen, EventMouseMove);
    #[cfg(before_274)]
    check!(seen, EventTracking);
    #[cfg(since_254)]
    check!(seen, MapBuildComplete);
    #[cfg(since_254)]
    check!(seen, OpPlayer5);
    #[cfg(rev = "225")]
    check!(seen, RebuildGetMaps);
    check!(
        seen,
        AnticheatCycleLogic1,
        AnticheatCycleLogic2,
        AnticheatCycleLogic3,
        AnticheatCycleLogic4,
        AnticheatCycleLogic5,
        AnticheatCycleLogic6,
        AnticheatOpLogic1,
        AnticheatOpLogic2,
        AnticheatOpLogic3,
        AnticheatOpLogic4,
        AnticheatOpLogic5,
        AnticheatOpLogic6,
        AnticheatOpLogic7,
        AnticheatOpLogic8,
        AnticheatOpLogic9,
        ChatSetMode,
        ClientCheat,
        CloseModal,
        FriendListAdd,
        FriendListDel,
        IdkSaveDesign,
        IdleTimer,
        IfButton,
        IgnoreListAdd,
        IgnoreListDel,
        InvButton1,
        InvButton2,
        InvButton3,
        InvButton4,
        InvButton5,
        InvButtonD,
        MessagePrivate,
        MessagePublic,
        MoveGameClick,
        MoveMinimapClick,
        MoveOpClick,
        NoTimeout,
        OpHeld1,
        OpHeld2,
        OpHeld3,
        OpHeld4,
        OpHeld5,
        OpHeldT,
        OpHeldU,
        OpLoc1,
        OpLoc2,
        OpLoc3,
        OpLoc4,
        OpLoc5,
        OpLocT,
        OpLocU,
        OpNpc1,
        OpNpc2,
        OpNpc3,
        OpNpc4,
        OpNpc5,
        OpNpcT,
        OpNpcU,
        OpObj1,
        OpObj2,
        OpObj3,
        OpObj4,
        OpObj5,
        OpObjT,
        OpObjU,
        OpPlayer1,
        OpPlayer2,
        OpPlayer3,
        OpPlayer4,
        OpPlayerT,
        OpPlayerU,
        ResumePCountDialog,
        ResumePauseButton,
        SendSnapshot,
        TutClickSide,
    );

    let declared: BTreeSet<String> = ClientProt::ALL.iter().map(|p| format!("{p:?}")).collect();
    let missing: Vec<&String> = declared.difference(&seen).collect();
    assert!(
        missing.is_empty(),
        "rev {}: {} client packet(s) have no decode-bounds case: {:?}",
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

/// The bounded scan that replaced `gjstr` in `ClientCheat::decode` has to stay
/// behaviourally identical for well-formed input, terminator included.
#[test]
fn client_cheat_matches_gjstr() {
    for text in ["", "setlevel attack 99", "teleto 3200 3200"] {
        let mut wire = text.as_bytes().to_vec();
        wire.push(10);
        let len = wire.len();

        let mut expect = Packet::from(wire.clone());
        let expect_str = expect.gjstr(10);

        let mut buf = Packet::from(wire);
        assert_eq!(ClientCheat::decode(&mut buf, len).cheat, expect_str);
        assert_eq!(buf.pos, expect.pos);
    }

    let mut buf = Packet::from(b"abc".to_vec());
    assert_eq!(ClientCheat::decode(&mut buf, 3).cheat, "abc");
    assert_eq!(buf.pos, 3);
}
