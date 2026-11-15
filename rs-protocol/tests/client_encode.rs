//! Every client message the content tests send round-trips through its
//! encoder: decoding what `encode` writes gives back the decoded message, at
//! the frame's own size.
#![cfg(since_530)]

use rs_io::Packet;
use rs_protocol::network::game::client::apcoordt::ApCoordT;
use rs_protocol::network::game::client::chat_setmode::ChatSetMode;
#[cfg(since_647)]
use rs_protocol::network::game::client::click_world_map::ClickWorldMap;
use rs_protocol::network::game::client::client_cheat::ClientCheat;
use rs_protocol::network::game::client::close_modal::CloseModal;
#[cfg(before_647)]
use rs_protocol::network::game::client::if_button::IfButton;
use rs_protocol::network::game::client::if_button1::IfButton1;
use rs_protocol::network::game::client::if_button2::IfButton2;
use rs_protocol::network::game::client::if_button3::IfButton3;
use rs_protocol::network::game::client::if_button4::IfButton4;
use rs_protocol::network::game::client::if_button5::IfButton5;
use rs_protocol::network::game::client::if_button6::IfButton6;
use rs_protocol::network::game::client::if_button7::IfButton7;
use rs_protocol::network::game::client::if_button8::IfButton8;
use rs_protocol::network::game::client::if_button9::IfButton9;
use rs_protocol::network::game::client::if_button10::IfButton10;
use rs_protocol::network::game::client::if_buttond::IfButtonD;
use rs_protocol::network::game::client::if_buttont::IfButtonT;
#[cfg(before_647)]
use rs_protocol::network::game::client::inv_buttond::InvButtonD;
use rs_protocol::network::game::client::map_build_complete::MapBuildComplete;
use rs_protocol::network::game::client::message_private::MessagePrivate;
use rs_protocol::network::game::client::message_public::MessagePublic;
use rs_protocol::network::game::client::message_quickchat_private::MessageQuickchatPrivate;
use rs_protocol::network::game::client::message_quickchat_public::MessageQuickchatPublic;
use rs_protocol::network::game::client::move_gameclick::MoveGameClick;
use rs_protocol::network::game::client::no_timeout::NoTimeout;
#[cfg(before_647)]
use rs_protocol::network::game::client::opheld1::OpHeld1;
#[cfg(before_647)]
use rs_protocol::network::game::client::opheld2::OpHeld2;
#[cfg(before_647)]
use rs_protocol::network::game::client::opheld3::OpHeld3;
#[cfg(before_647)]
use rs_protocol::network::game::client::opheld4::OpHeld4;
#[cfg(before_647)]
use rs_protocol::network::game::client::opheld5::OpHeld5;
#[cfg(before_647)]
use rs_protocol::network::game::client::opheldt::OpHeldT;
#[cfg(before_647)]
use rs_protocol::network::game::client::opheldu::OpHeldU;
use rs_protocol::network::game::client::oploc1::OpLoc1;
use rs_protocol::network::game::client::oploc2::OpLoc2;
use rs_protocol::network::game::client::oploc6::OpLoc6;
use rs_protocol::network::game::client::oploct::OpLocT;
#[cfg(before_647)]
use rs_protocol::network::game::client::oplocu::OpLocU;
use rs_protocol::network::game::client::opnpc1::OpNpc1;
use rs_protocol::network::game::client::opnpc2::OpNpc2;
use rs_protocol::network::game::client::opnpc3::OpNpc3;
use rs_protocol::network::game::client::opnpc4::OpNpc4;
use rs_protocol::network::game::client::opnpc5::OpNpc5;
use rs_protocol::network::game::client::opnpc6::OpNpc6;
use rs_protocol::network::game::client::opnpct::OpNpcT;
#[cfg(before_647)]
use rs_protocol::network::game::client::opnpcu::OpNpcU;
use rs_protocol::network::game::client::opobj3::OpObj3;
use rs_protocol::network::game::client::opobj6::OpObj6;
use rs_protocol::network::game::client::opobjt::OpObjT;
#[cfg(before_647)]
use rs_protocol::network::game::client::opobju::OpObjU;
use rs_protocol::network::game::client::opplayer4::OpPlayer4;
use rs_protocol::network::game::client::opplayer6::OpPlayer6;
use rs_protocol::network::game::client::opplayer7::OpPlayer7;
use rs_protocol::network::game::client::opplayer8::OpPlayer8;
#[cfg(since_647)]
use rs_protocol::network::game::client::opplayer9::OpPlayer9;
#[cfg(since_647)]
use rs_protocol::network::game::client::opplayer10::OpPlayer10;
use rs_protocol::network::game::client::opplayert::OpPlayerT;
#[cfg(before_647)]
use rs_protocol::network::game::client::opplayeru::OpPlayerU;
use rs_protocol::network::game::client::reflection_check_reply::ReflectionCheckReply;
use rs_protocol::network::game::client::resume_p_countdialog::ResumePCountDialog;
use rs_protocol::network::game::client::resume_pause_button::ResumePauseButton;
use rs_protocol::network::game::client::send_snapshot::SendSnapshot;
#[cfg(since_647)]
use rs_protocol::network::game::client::set_chatfiltersettings::SetChatfiltersettings;
use rs_protocol::network::game::client::url_request::UrlRequest;
use rs_protocol::network::game::client::window_status::WindowStatus;
use rs_protocol::network::game::client::{ClientProtEncode, ClientProtMessage};
use rs_protocol::network::game::client_prot_frame::ClientProtFrame;
use rs_protocol::network::game::client_prot_message::ClientProtMessageInfo;
use rs_ver::STRING_TERMINATOR;
use std::fmt::Debug;

/// Decodes `raw`, encodes the message and decodes that again: both decodes
/// must agree, and a fixed frame must come back at its size.
fn round_trip<M: ClientProtMessage + ClientProtEncode + Debug>(raw: &[u8]) {
    let name = std::any::type_name::<M>();
    assert!(M::validate(raw), "{name}: the test payload is invalid");
    let first = M::decode(&mut Packet::from(raw.to_vec()), raw.len());
    let mut buf = Packet::new(raw.len() + 64);
    first.encode(&mut buf);
    let written = &buf.data[..buf.pos];
    assert!(
        M::validate(written),
        "{name}: encode wrote an invalid frame"
    );
    let second = M::decode(&mut Packet::from(written.to_vec()), written.len());
    assert_eq!(format!("{first:?}"), format!("{second:?}"), "{name}");
    if let ClientProtFrame::Fixed(len) = <M as ClientProtMessageInfo>::FRAME {
        assert_eq!(written.len(), len as usize, "{name}: the frame's size");
    }
}

/// A fixed frame's payload, every byte different so a field written in the
/// wrong place or form reads back differently.
fn fixed<M: ClientProtMessageInfo>() -> Vec<u8> {
    let ClientProtFrame::Fixed(len) = M::FRAME else {
        panic!("{} is not a fixed frame", std::any::type_name::<M>());
    };
    (0..len as usize).map(|i| (i * 37 + 11) as u8).collect()
}

fn bytes(write: impl FnOnce(&mut Packet)) -> Vec<u8> {
    let mut buf = Packet::new(256);
    write(&mut buf);
    buf.data[..buf.pos].to_vec()
}

macro_rules! fixed {
    ( $( $ty:ident ),* $(,)? ) => {
        $( round_trip::<$ty>(&fixed::<$ty>()); )*
    };
}

#[test]
fn fixed_frames_round_trip() {
    fixed!(
        ApCoordT,
        ChatSetMode,
        CloseModal,
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
        MapBuildComplete,
        NoTimeout,
        OpLoc1,
        OpLoc2,
        OpLoc6,
        OpLocT,
        OpNpc1,
        OpNpc2,
        OpNpc3,
        OpNpc4,
        OpNpc5,
        OpNpc6,
        OpNpcT,
        OpObj3,
        OpObj6,
        OpObjT,
        OpPlayer4,
        OpPlayer6,
        OpPlayer7,
        OpPlayer8,
        OpPlayerT,
        ResumePCountDialog,
        ResumePauseButton,
        WindowStatus,
    );
    #[cfg(since_647)]
    fixed!(
        ClickWorldMap,
        MoveGameClick,
        OpPlayer9,
        OpPlayer10,
        SetChatfiltersettings
    );
    #[cfg(before_647)]
    round_trip::<SendSnapshot>(&fixed::<SendSnapshot>());
    // 530's if1 components and backpack.
    #[cfg(before_647)]
    fixed!(
        IfButton, InvButtonD, OpHeld1, OpHeld2, OpHeld3, OpHeld4, OpHeld5, OpHeldT, OpHeldU,
        OpLocU, OpNpcU, OpObjU, OpPlayerU,
    );
}

#[test]
fn variable_frames_round_trip() {
    let text = [3, 1, 4, 1, 5];
    round_trip::<ClientCheat>(&bytes(|b| {
        #[cfg(since_647)]
        {
            b.p1(1);
            b.p1(0);
        }
        b.pjstr("tele 3222 3218", STRING_TERMINATOR);
    }));
    round_trip::<UrlRequest>(&bytes(|b| {
        b.pjstr("forums.ws", STRING_TERMINATOR);
        b.pjstr("forum", STRING_TERMINATOR);
        #[cfg(since_742)]
        b.pjstr("extra", STRING_TERMINATOR);
        b.p1(2);
    }));
    round_trip::<MessagePublic>(&bytes(|b| {
        b.p1(1);
        b.p1(2);
        b.psmart1or2(300);
        b.pdata(&text, 0, text.len());
    }));
    round_trip::<MessagePrivate>(&bytes(|b| {
        #[cfg(before_647)]
        b.p8(0x1234_5678_9abc);
        #[cfg(since_647)]
        b.pjstr("Mod Ash", STRING_TERMINATOR);
        b.psmart1or2(5);
        b.pdata(&text, 0, text.len());
    }));
    round_trip::<MessageQuickchatPrivate>(&bytes(|b| {
        #[cfg(before_647)]
        b.p8(0x1234_5678_9abc);
        #[cfg(since_647)]
        b.pjstr("Mod Ash", STRING_TERMINATOR);
        b.p2(8);
        b.pdata(&text, 0, text.len());
    }));
    round_trip::<MessageQuickchatPublic>(&bytes(|b| {
        b.p1(0);
        b.p2(8);
        b.pdata(&text, 0, text.len());
    }));
    round_trip::<ReflectionCheckReply>(&bytes(|b| {
        b.p4(7);
        b.pdata(&text, 0, text.len());
        b.p4(-99);
    }));
    #[cfg(since_647)]
    round_trip::<SendSnapshot>(&bytes(|b| {
        b.pjstr("Mod Ash", STRING_TERMINATOR);
        b.p1(3);
        b.p1(1);
        b.pjstr("spam", STRING_TERMINATOR);
    }));
    #[cfg(before_647)]
    round_trip::<MoveGameClick>(&bytes(|b| {
        b.p1_alt1(1);
        b.p2(3222);
        b.p2_alt2(3218);
        b.p1_alt1(2);
        b.p1_alt3(0xfe);
        b.p1_alt1(5);
        b.p1_alt3(3);
    }));
}
