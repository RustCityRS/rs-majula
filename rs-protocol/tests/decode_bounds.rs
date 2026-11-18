//! The client packets of revisions 225 to 530. Rev 647's decoders are
//! bounds-checked in `wire_647_inbound.rs`.
#![cfg(before_647)]

use rs_io::Packet;
use rs_protocol::network::game::client::ClientProtMessage;
use rs_protocol::network::game::client_prot::*;
use rs_protocol::network::game::client_prot_frame::ClientProtFrame;
use rs_protocol::network::game::client_prot_message::ClientProtMessageInfo;
use rs_ver::STRING_TERMINATOR;
use std::collections::BTreeSet;

const SPREAD: usize = 40;
const PAD: usize = 512;

const FILLS: [fn(usize) -> Vec<u8>; 3] = [
    |n| vec![0x00; n],
    |n| vec![0xff; n],
    |n| {
        (0..n)
            .map(|i| {
                if i % 3 == 0 {
                    STRING_TERMINATOR
                } else {
                    i as u8
                }
            })
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
            if !M::validate(&data) {
                continue;
            }
            data.resize(len + PAD, STRING_TERMINATOR);
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

    #[cfg(all(since_254, before_530))]
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
        ChatSetMode,
        ClientCheat,
        CloseModal,
        FriendListAdd,
        FriendListDel,
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
    );
    #[cfg(before_530)]
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
        IdkSaveDesign,
        TutClickSide,
    );
    #[cfg(since_530)]
    check!(
        seen,
        ApCoordT,
        ClanJoinchatLeavechat,
        ClanKickUser,
        ClientDetailoptionsStatus,
        FaceSquare,
        FriendSetRank,
        GameloginContinue,
        IfButton1,
        IfButton2,
        IfButton3,
        IfButton4,
        IfButton5,
        IfButton6,
        IfButton7,
        IfButton8,
        IfButton9,
        IfButton10,
        IfButtonD,
        IfButtonT,
        DetectModifiedClient,
        MessageQuickchatPrivate,
        MessageQuickchatPublic,
        OpLoc6,
        OpNpc6,
        OpObj6,
        OpPlayer6,
        OpPlayer7,
        OpPlayer8,
        ReflectionCheckReply,
        ResumePNameDialog,
        ResumePObjDialog,
        ResumePStringDialog,
        SoundSongPreloaded,
        TransmitvarVerifyId,
        UrlRequest,
        WindowStatus,
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
        wire.push(STRING_TERMINATOR);
        let len = wire.len();

        let mut expect = Packet::from(wire.clone());
        let expect_str = expect.gjstr(STRING_TERMINATOR);

        let mut buf = Packet::from(wire);
        assert_eq!(ClientCheat::decode(&mut buf, len).cheat, expect_str);
        assert_eq!(buf.pos, expect.pos);
    }

    let mut buf = Packet::from(b"abc".to_vec());
    assert_eq!(ClientCheat::decode(&mut buf, 3).cheat, "abc");
    assert_eq!(buf.pos, 3);
}
