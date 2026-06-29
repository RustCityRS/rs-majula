use crate::network::game::client::anticheat_cyclelogic1::AnticheatCycleLogic1;
use crate::network::game::client::anticheat_cyclelogic2::AnticheatCycleLogic2;
use crate::network::game::client::anticheat_cyclelogic3::AnticheatCycleLogic3;
use crate::network::game::client::anticheat_cyclelogic4::AnticheatCycleLogic4;
use crate::network::game::client::anticheat_cyclelogic5::AnticheatCycleLogic5;
use crate::network::game::client::anticheat_cyclelogic6::AnticheatCycleLogic6;
#[cfg(since_254)]
use crate::network::game::client::anticheat_cyclelogic7::AnticheatCycleLogic7;
use crate::network::game::client::anticheat_oplogic1::AnticheatOpLogic1;
use crate::network::game::client::anticheat_oplogic2::AnticheatOpLogic2;
use crate::network::game::client::anticheat_oplogic3::AnticheatOpLogic3;
use crate::network::game::client::anticheat_oplogic4::AnticheatOpLogic4;
use crate::network::game::client::anticheat_oplogic5::AnticheatOpLogic5;
use crate::network::game::client::anticheat_oplogic6::AnticheatOpLogic6;
use crate::network::game::client::anticheat_oplogic7::AnticheatOpLogic7;
use crate::network::game::client::anticheat_oplogic8::AnticheatOpLogic8;
use crate::network::game::client::anticheat_oplogic9::AnticheatOpLogic9;
use crate::network::game::client::chat_setmode::ChatSetMode;
use crate::network::game::client::client_cheat::ClientCheat;
use crate::network::game::client::close_modal::CloseModal;
#[cfg(since_254)]
use crate::network::game::client::event_applet_focus::EventAppletFocus;
#[cfg(any(rev = "225", since_254))]
use crate::network::game::client::event_camera_position::EventCameraPosition;
#[cfg(since_254)]
use crate::network::game::client::event_mouse_click::EventMouseClick;
#[cfg(since_254)]
use crate::network::game::client::event_mouse_move::EventMouseMove;
#[cfg(before_274)]
use crate::network::game::client::event_tracking::EventTracking;
use crate::network::game::client::friendlist_add::FriendListAdd;
use crate::network::game::client::friendlist_del::FriendListDel;
use crate::network::game::client::idk_savedesign::IdkSaveDesign;
use crate::network::game::client::idle_timer::IdleTimer;
use crate::network::game::client::if_button::IfButton;
use crate::network::game::client::ignorelist_add::IgnoreListAdd;
use crate::network::game::client::ignorelist_del::IgnoreListDel;
use crate::network::game::client::inv_button1::InvButton1;
use crate::network::game::client::inv_button2::InvButton2;
use crate::network::game::client::inv_button3::InvButton3;
use crate::network::game::client::inv_button4::InvButton4;
use crate::network::game::client::inv_button5::InvButton5;
use crate::network::game::client::inv_buttond::InvButtonD;
#[cfg(since_254)]
use crate::network::game::client::map_build_complete::MapBuildComplete;
use crate::network::game::client::message_private::MessagePrivate;
use crate::network::game::client::message_public::MessagePublic;
use crate::network::game::client::move_gameclick::MoveGameClick;
use crate::network::game::client::move_minimapclick::MoveMinimapClick;
use crate::network::game::client::move_opclick::MoveOpClick;
use crate::network::game::client::no_timeout::NoTimeout;
use crate::network::game::client::opheld1::OpHeld1;
use crate::network::game::client::opheld2::OpHeld2;
use crate::network::game::client::opheld3::OpHeld3;
use crate::network::game::client::opheld4::OpHeld4;
use crate::network::game::client::opheld5::OpHeld5;
use crate::network::game::client::opheldt::OpHeldT;
use crate::network::game::client::opheldu::OpHeldU;
use crate::network::game::client::oploc1::OpLoc1;
use crate::network::game::client::oploc2::OpLoc2;
use crate::network::game::client::oploc3::OpLoc3;
use crate::network::game::client::oploc4::OpLoc4;
use crate::network::game::client::oploc5::OpLoc5;
use crate::network::game::client::oploct::OpLocT;
use crate::network::game::client::oplocu::OpLocU;
use crate::network::game::client::opnpc1::OpNpc1;
use crate::network::game::client::opnpc2::OpNpc2;
use crate::network::game::client::opnpc3::OpNpc3;
use crate::network::game::client::opnpc4::OpNpc4;
use crate::network::game::client::opnpc5::OpNpc5;
use crate::network::game::client::opnpct::OpNpcT;
use crate::network::game::client::opnpcu::OpNpcU;
use crate::network::game::client::opobj1::OpObj1;
use crate::network::game::client::opobj2::OpObj2;
use crate::network::game::client::opobj3::OpObj3;
use crate::network::game::client::opobj4::OpObj4;
use crate::network::game::client::opobj5::OpObj5;
use crate::network::game::client::opobjt::OpObjT;
use crate::network::game::client::opobju::OpObjU;
use crate::network::game::client::opplayer1::OpPlayer1;
use crate::network::game::client::opplayer2::OpPlayer2;
use crate::network::game::client::opplayer3::OpPlayer3;
use crate::network::game::client::opplayer4::OpPlayer4;
#[cfg(since_254)]
use crate::network::game::client::opplayer5::OpPlayer5;
use crate::network::game::client::opplayert::OpPlayerT;
use crate::network::game::client::opplayeru::OpPlayerU;
#[cfg(rev = "225")]
use crate::network::game::client::rebuild_get_maps::RebuildGetMaps;
use crate::network::game::client::resume_p_countdialog::ResumePCountDialog;
use crate::network::game::client::resume_pause_button::ResumePauseButton;
use crate::network::game::client::send_snapshot::SendSnapshot;
use crate::network::game::client::tut_clickside::TutClickSide;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::PacketFrame;

macro_rules! client_prot {
    ( $( $variant:ident = $id:expr ),* $(,)? ) => {
        #[repr(u8)]
        #[derive(Debug)]
        pub enum ClientProt {
            $( $variant = $id, )*
        }

        impl TryFrom<u8> for ClientProt {
            type Error = ();

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    $( $id => Ok(ClientProt::$variant), )*
                    _ => Err(()),
                }
            }
        }

        impl ClientProt {
            pub fn info(&self) -> ClientProtInfo {
                match self {
                    $( ClientProt::$variant => ClientProtInfo {
                        frame: <$variant>::FRAME,
                        category: <$variant>::CATEGORY,
                    }, )*
                }
            }
        }
    };
}

#[cfg(rev = "225")]
client_prot! {
    AnticheatOpLogic8 = 2,
    ClientCheat = 4, // NXT naming
    InvButton5 = 6, // NXT has "IF_BUTTON5" but for our interface system, this makes more sense
    AnticheatOpLogic1 = 7,
    OpNpc2 = 8, // NXT naming
    OpLocT = 9, // NXT naming
    FriendListDel = 11, // NXT naming
    AnticheatOpLogic7 = 17,
    OpNpc3 = 27, // NXT naming
    AnticheatOpLogic3 = 30,
    InvButton1 = 31, // NXT has "IF_BUTTON1" but for our interface system, this makes more sense
    InvButton4 = 38, // NXT has "IF_BUTTON4" but for our interface system, this makes more sense
    OpObj2 = 40, // NXT naming
    OpHeldT = 48, // name based on runescript trigger
    IdkSaveDesign = 52,
    OpPlayer2 = 53,
    InvButton2 = 59, // NXT has "IF_BUTTON2" but for our interface system, this makes more sense
    AnticheatOpLogic6 = 66,
    IdleTimer = 70,
    OpHeld2 = 71, // name based on runescript trigger
    OpLocU = 75, // NXT naming
    IgnoreListAdd = 79, // NXT naming
    EventTracking = 81,
    AnticheatCycleLogic5 = 85,
    AnticheatOpLogic2 = 88,
    MoveOpClick = 93, // comes with OP packets, name based on other MOVE packets
    OpLoc3 = 96, // NXT naming
    OpLoc4 = 97, // NXT naming
    OpNpc5 = 100, // NXT naming
    NoTimeout = 108, // NXT naming
    OpNpc4 = 113, // NXT naming
    OpLoc5 = 116, // NXT naming
    FriendListAdd = 118, // NXT naming
    OpHeldU = 130, // name based on runescript trigger
    OpHeld3 = 133, // name based on runescript trigger
    OpNpcT = 134, // NXT naming
    OpObjT = 138, // NXT naming
    OpObj1 = 140, // NXT naming
    AnticheatCycleLogic2 = 146,
    MessagePrivate = 148, // NXT naming
    RebuildGetMaps = 150,
    IfButton = 155, // NXT naming
    OpHeld4 = 157, // name based on runescript trigger
    MessagePublic = 158, // NXT naming
    InvButtonD = 159, // NXT has "IF_BUTTOND" but for our interface system, this makes more sense
    OpPlayer1 = 164, // NXT naming
    MoveMinimapClick = 165, // NXT naming
    IgnoreListDel = 171, // NXT naming
    OpLoc2 = 172, // NXT naming
    TutClickSide = 175,
    AnticheatOpLogic4 = 176,
    OpPlayerT = 177, // NXT naming
    OpObj4 = 178, // NXT naming
    MoveGameClick = 181, // NXT naming
    OpPlayer3 = 185, // NXT naming
    EventCameraPosition = 189, // NXT naming
    SendSnapshot = 190,
    OpNpc1 = 194, // NXT naming
    OpHeld1 = 195, // name based on runescript trigger
    OpObj3 = 200, // NXT naming
    OpNpcU = 202, // NXT naming
    OpPlayer4 = 206, // NXT naming
    OpHeld5 = 211, // name based on runescript trigger
    InvButton3 = 212, // NXT has "IF_BUTTON3" but for our interface system, this makes more sense
    AnticheatCycleLogic3 = 215,
    AnticheatCycleLogic6 = 219,
    AnticheatOpLogic5 = 220,
    CloseModal = 231, // NXT naming
    AnticheatCycleLogic1 = 233,
    ResumePauseButton = 235, // NXT naming
    AnticheatCycleLogic4 = 236,
    ResumePCountDialog = 237, // NXT naming
    AnticheatOpLogic9 = 238,
    OpObjU = 239, // NXT naming
    ChatSetMode = 244, // NXT naming
    OpLoc1 = 245, // NXT naming
    OpObj5 = 247, // NXT naming
    OpPlayerU = 248, // NXT naming
}

#[cfg(rev = "244")]
client_prot! {
    OpHeld4 = 6, // name based on runescript trigger
    AnticheatOpLogic5 = 7,
    IdkSaveDesign = 8, // IF_PLAYERDESIGN
    FriendListAdd = 9, // NXT naming
    ResumePauseButton = 11, // NXT naming
    OpObj4 = 17, // NXT naming
    OpLoc3 = 19, // NXT naming
    OpObjT = 25, // NXT naming
    OpObj3 = 27, // NXT naming
    AnticheatOpLogic4 = 34,
    AnticheatOpLogic3 = 37,
    OpLoc2 = 38, // NXT naming
    IfButton = 39, // NXT naming
    AnticheatCycleLogic4 = 41,
    OpPlayer4 = 43, // NXT naming
    AnticheatCycleLogic1 = 46,
    AnticheatOpLogic1 = 47,
    OpPlayerU = 48, // NXT naming
    AnticheatOpLogic7 = 50,
    OpNpcU = 52, // NXT naming
    OpLoc4 = 55, // NXT naming
    MoveMinimapClick = 56, // NXT naming
    OpHeldU = 58, // name based on runescript trigger
    MoveGameClick = 63, // NXT naming
    OpPlayer3 = 64, // NXT naming
    FriendListDel = 69, // NXT naming
    OpPlayerT = 73, // NXT naming
    ClientCheat = 76, // NXT naming
    InvButtonD = 81, // NXT has "IF_BUTTOND" but our interface system differs
    OpNpc2 = 84, // NXT naming
    ChatSetMode = 98, // NXT naming
    AnticheatOpLogic8 = 100,
    OpNpcT = 101, // NXT naming
    OpNpc5 = 102, // NXT naming
    OpLocU = 106, // NXT naming
    NoTimeout = 107, // NXT naming
    OpObj2 = 110, // NXT naming
    OpObjU = 111, // NXT naming
    OpNpc3 = 132, // NXT naming
    OpHeld5 = 133, // name based on runescript trigger
    OpHeldT = 143, // name based on runescript trigger
    AnticheatCycleLogic3 = 144,
    IdleTimer = 146,
    AnticheatCycleLogic2 = 148,
    InvButton1 = 153, // NXT has "IF_BUTTON1" but our interface system differs
    InvButton3 = 158, // NXT has "IF_BUTTON3" but our interface system differs
    OpHeld2 = 166, // name based on runescript trigger
    MoveOpClick = 167, // comes with OP packets, name based on other MOVE packets
    AnticheatOpLogic9 = 169,
    MessagePrivate = 170, // NXT naming
    MessagePublic = 171, // NXT naming
    AnticheatOpLogic6 = 177,
    OpLocT = 182, // NXT naming
    CloseModal = 187, // NXT naming
    ResumePCountDialog = 190, // NXT naming
    InvButton2 = 193, // NXT has "IF_BUTTON2" but our interface system differs
    IgnoreListAdd = 203, // NXT naming
    InvButton4 = 204, // NXT has "IF_BUTTON4" but our interface system differs
    IgnoreListDel = 207, // NXT naming
    OpPlayer1 = 211, // NXT naming
    InvButton5 = 212, // NXT has "IF_BUTTON5" but our interface system differs
    AnticheatCycleLogic6 = 215,
    EventTracking = 217,
    AnticheatOpLogic2 = 218,
    OpPlayer2 = 219, // NXT naming
    OpHeld3 = 221, // name based on runescript trigger
    OpNpc1 = 222, // NXT naming
    OpObj5 = 225, // NXT naming
    OpHeld1 = 228, // name based on runescript trigger
    OpNpc4 = 229, // NXT naming
    OpObj1 = 231, // NXT naming
    AnticheatCycleLogic5 = 232,
    TutClickSide = 233,
    OpLoc1 = 238, // NXT naming
    OpLoc5 = 243, // NXT naming
    SendSnapshot = 251, // REPORT_ABUSE
}

#[cfg(rev = "245.2")]
client_prot! {
    OpLoc1 = 1, // NXT naming
    IgnoreListDel = 4, // NXT naming
    InvButtonD = 7, // NXT has "IF_BUTTOND" but our interface system differs
    ChatSetMode = 8, // NXT naming
    OpHeld5 = 9, // name based on runescript trigger
    ClientCheat = 11, // NXT naming
    InvButton1 = 13, // NXT has "IF_BUTTON1" but our interface system differs
    OpNpcU = 14, // NXT naming
    OpObj4 = 17, // NXT naming
    EventTracking = 19,
    IgnoreListAdd = 20, // NXT naming
    OpNpc5 = 43, // NXT naming
    InvButton3 = 48, // NXT has "IF_BUTTON3" but our interface system differs
    OpPlayerT = 52, // NXT naming
    OpPlayer4 = 54, // NXT naming
    OpObj3 = 55, // NXT naming
    InvButton2 = 58, // NXT has "IF_BUTTON2" but our interface system differs
    FriendListDel = 61, // NXT naming
    AnticheatCycleLogic5 = 63,
    AnticheatOpLogic5 = 74,
    MessagePublic = 78, // NXT naming
    OpLoc5 = 86, // NXT naming
    AnticheatOpLogic1 = 87,
    AnticheatCycleLogic4 = 94,
    AnticheatOpLogic2 = 95,
    MessagePrivate = 99, // NXT naming
    IdleTimer = 102, // NXT naming
    OpHeld1 = 104, // name based on runescript trigger
    OpNpc4 = 107, // NXT naming
    AnticheatCycleLogic6 = 112,
    OpObj1 = 113, // NXT naming
    OpHeld3 = 115, // name based on runescript trigger
    FriendListAdd = 116, // NXT naming
    AnticheatOpLogic7 = 119,
    OpObjT = 122, // NXT naming
    OpHeldU = 126, // name based on runescript trigger
    OpPlayer1 = 135, // NXT naming
    AnticheatCycleLogic1 = 136,
    OpNpcT = 141, // NXT naming
    OpObjU = 143, // NXT naming
    AnticheatOpLogic3 = 146,
    OpLocU = 147, // NXT naming
    IdkSaveDesign = 150, // IF_PLAYERDESIGN
    OpPlayer2 = 165, // NXT naming
    AnticheatOpLogic8 = 171,
    OpPlayer3 = 172, // NXT naming
    IfButton = 177, // NXT naming
    OpNpc1 = 180, // NXT naming
    AnticheatCycleLogic3 = 181,
    MoveGameClick = 182, // NXT naming
    InvButton4 = 183, // NXT has "IF_BUTTON4" but our interface system differs
    AnticheatOpLogic4 = 186,
    OpHeldT = 188, // name based on runescript trigger
    OpHeld2 = 193, // name based on runescript trigger
    OpHeld4 = 194, // name based on runescript trigger
    OpNpc3 = 196, // NXT naming
    MoveMinimapClick = 198, // NXT naming
    OpLoc4 = 204, // NXT naming
    SendSnapshot = 205, // REPORT_ABUSE
    NoTimeout = 206, // NXT naming
    OpLocT = 208, // NXT naming
    OpPlayerU = 210, // NXT naming
    MoveOpClick = 216, // comes with OP packets, name based on other MOVE packets
    OpLoc2 = 219, // NXT naming
    AnticheatCycleLogic2 = 223,
    OpLoc3 = 226, // NXT naming
    AnticheatOpLogic9 = 233,
    OpObj2 = 238, // NXT naming
    ResumePauseButton = 239, // NXT naming
    ResumePCountDialog = 241, // NXT naming
    InvButton5 = 242, // NXT has "IF_BUTTON5" but our interface system differs
    TutClickSide = 243,
    CloseModal = 245, // NXT naming
    OpObj5 = 247, // NXT naming
    AnticheatOpLogic6 = 250,
    OpNpc2 = 252, // NXT naming
}

#[cfg(rev = "254")]
client_prot! {
    AnticheatCycleLogic3 = 4,
    MoveGameClick = 6, // NXT naming
    EventAppletFocus = 8,
    FriendListAdd = 9, // NXT naming
    IdkSaveDesign = 13, // IF_PLAYERDESIGN
    OpPlayer2 = 17, // NXT naming
    OpPlayer3 = 18, // NXT naming
    OpLocT = 26, // NXT naming
    AnticheatOpLogic1 = 28,
    OpLoc1 = 33, // NXT naming
    AnticheatCycleLogic6 = 36,
    OpObj4 = 47, // NXT naming
    AnticheatCycleLogic1 = 51,
    AnticheatOpLogic3 = 56,
    CloseModal = 58, // NXT naming
    InvButton3 = 59, // NXT has "IF_BUTTON3" but our interface system differs
    InvButton5 = 62, // NXT has "IF_BUTTON5" but our interface system differs
    OpObj2 = 67, // NXT naming
    OpPlayerT = 68, // NXT naming
    OpNpc3 = 69, // NXT naming
    InvButton2 = 70, // NXT has "IF_BUTTON2" but our interface system differs
    OpPlayer4 = 72, // NXT naming
    OpHeld5 = 74, // name based on runescript trigger
    AnticheatOpLogic2 = 77,
    OpHeld3 = 80, // name based on runescript trigger
    MessagePublic = 83, // NXT naming
    FriendListDel = 84, // NXT naming
    ClientCheat = 86, // NXT naming
    OpLoc4 = 87, // NXT naming
    EventCameraPosition = 91, // NXT naming
    OpObj5 = 97, // NXT naming
    OpLoc3 = 98, // NXT naming
    AnticheatCycleLogic5 = 100,
    OpHeldT = 102, // name based on runescript trigger
    OpPlayerU = 113, // NXT naming
    OpNpc5 = 118, // NXT naming
    OpNpcU = 119, // NXT naming
    AnticheatOpLogic4 = 121,
    OpNpc4 = 122, // NXT naming
    MoveOpClick = 127, // comes with OP packets, name based on other MOVE packets
    ChatSetMode = 129, // NXT naming
    AnticheatOpLogic6 = 131,
    MapBuildComplete = 134, // NXT naming
    OpObj1 = 141, // NXT naming
    EventTracking = 142,
    OpNpc1 = 143, // NXT naming
    IdleTimer = 144, // NXT naming
    ResumePauseButton = 146, // NXT naming
    OpLoc5 = 147, // NXT naming
    InvButton4 = 160, // NXT has "IF_BUTTON4" but our interface system differs
    ResumePCountDialog = 161, // NXT naming
    AnticheatOpLogic9 = 162,
    OpHeld4 = 163, // name based on runescript trigger
    InvButtonD = 176, // NXT has "IF_BUTTOND" but our interface system differs
    OpObj3 = 178, // NXT naming
    InvButton1 = 181, // NXT has "IF_BUTTON1" but our interface system differs
    AnticheatCycleLogic7 = 182,
    AnticheatOpLogic7 = 187,
    IgnoreListAdd = 189, // NXT naming
    OpPlayer1 = 192, // NXT naming
    IgnoreListDel = 193, // NXT naming
    OpNpc2 = 195, // NXT naming
    OpHeldU = 200, // name based on runescript trigger
    TutClickSide = 201,
    OpObjT = 202, // NXT naming
    SendSnapshot = 203, // REPORT_ABUSE
    AnticheatOpLogic8 = 206,
    OpLoc2 = 213, // NXT naming
    MessagePrivate = 214, // NXT naming
    MoveMinimapClick = 220, // NXT naming
    AnticheatCycleLogic2 = 225,
    AnticheatCycleLogic4 = 226,
    OpHeld2 = 228, // name based on runescript trigger
    OpPlayer5 = 230, // NXT naming
    OpNpcT = 231, // NXT naming
    EventMouseMove = 232,
    AnticheatOpLogic5 = 233,
    EventMouseClick = 234, // NXT naming
    NoTimeout = 239, // NXT naming
    OpLocU = 240, // NXT naming
    OpHeld1 = 243, // name based on runescript trigger
    IfButton = 244, // NXT naming
    OpObjU = 245, // NXT naming
}

#[cfg(rev = "274")]
client_prot! {
    AnticheatOpLogic8 = 0,
    OpHeld2 = 2, // name based on runescript trigger
    IfButton = 9, // NXT naming
    AnticheatCycleLogic1 = 12,
    FriendListAdd = 13, // NXT naming
    EventMouseClick = 20, // NXT naming
    AnticheatOpLogic9 = 24,
    AnticheatOpLogic7 = 25,
    OpPlayerU = 36, // NXT naming
    OpObjU = 39, // NXT naming
    AnticheatOpLogic3 = 41,
    OpHeld5 = 42, // name based on runescript trigger
    InvButton5 = 46, // NXT has "IF_BUTTON5" but our interface system differs
    CloseModal = 51, // NXT naming
    AnticheatCycleLogic3 = 52,
    EventCameraPosition = 53, // NXT naming
    OpLocU = 60, // NXT naming
    OpObj4 = 62, // NXT naming
    ResumePauseButton = 72, // NXT naming
    EventAppletFocus = 73,
    InvButton1 = 74, // NXT has "IF_BUTTON1" but our interface system differs
    AnticheatOpLogic4 = 80,
    InvButton2 = 82, // NXT has "IF_BUTTON2" but our interface system differs
    MoveMinimapClick = 86, // NXT naming
    AnticheatCycleLogic7 = 89,
    OpObjT = 91, // NXT naming
    InvButtonD = 93, // NXT has "IF_BUTTOND" but our interface system differs
    TutClickSide = 94,
    OpPlayer4 = 98, // NXT naming
    AnticheatCycleLogic5 = 100,
    IgnoreListDel = 101, // NXT naming
    ResumePCountDialog = 102, // NXT naming
    OpLoc2 = 103, // NXT naming
    FriendListDel = 106, // NXT naming
    OpObj3 = 108, // NXT naming
    OpPlayer1 = 109, // NXT naming
    OpObj5 = 117, // NXT naming
    NoTimeout = 120, // NXT naming
    OpHeld3 = 123, // name based on runescript trigger
    IdkSaveDesign = 125, // IF_PLAYERDESIGN
    OpLoc5 = 127, // NXT naming
    OpHeldT = 135, // name based on runescript trigger
    OpHeldU = 136, // name based on runescript trigger
    SendSnapshot = 137, // REPORT_ABUSE
    MoveOpClick = 138, // comes with OP packets, name based on other MOVE packets
    MessagePrivate = 139, // NXT naming
    OpNpc4 = 147, // NXT naming
    AnticheatCycleLogic2 = 149,
    OpNpcU = 150, // NXT naming
    ChatSetMode = 154, // NXT naming
    OpLoc4 = 157, // NXT naming
    OpPlayer2 = 166, // NXT naming
    OpObj2 = 169, // NXT naming
    OpPlayer5 = 174, // NXT naming
    InvButton4 = 179, // NXT has "IF_BUTTON4" but our interface system differs
    OpNpcT = 181, // NXT naming
    OpHeld1 = 185, // name based on runescript trigger
    OpLoc3 = 187, // NXT naming
    AnticheatCycleLogic6 = 188,
    OpNpc5 = 189, // NXT naming
    OpPlayer3 = 196, // NXT naming
    AnticheatOpLogic2 = 201,
    MoveGameClick = 207, // NXT naming
    IdleTimer = 209, // NXT naming
    OpLocT = 213, // NXT naming
    MapBuildComplete = 214, // NXT naming
    OpLoc1 = 215, // NXT naming
    OpHeld4 = 216, // name based on runescript trigger
    AnticheatOpLogic1 = 219,
    EventMouseMove = 222,
    OpNpc3 = 223, // NXT naming
    ClientCheat = 224, // NXT naming
    AnticheatCycleLogic4 = 230,
    OpNpc2 = 233, // NXT naming
    AnticheatOpLogic5 = 235,
    OpNpc1 = 236, // NXT naming
    InvButton3 = 239, // NXT has "IF_BUTTON3" but our interface system differs
    OpPlayerT = 240, // NXT naming
    OpObj1 = 247, // NXT naming
    AnticheatOpLogic6 = 250,
    MessagePublic = 253, // NXT naming
    IgnoreListAdd = 255, // NXT naming
}

pub struct ClientProtInfo {
    pub frame: (PacketFrame, Option<u8>),
    pub category: ClientProtCategory,
}
