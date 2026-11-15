#![cfg(since_742)]

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(1), ClientEvent)]
pub struct SimpleToolkitChange;

impl ClientProtMessage for SimpleToolkitChange {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        SimpleToolkitChange
    }
}

#[client_prot(Fixed(1), ClientEvent)]
pub struct CutsceneFinished;

impl ClientProtMessage for CutsceneFinished {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        CutsceneFinished
    }
}

#[client_prot(VarShort { min: 0 }, ClientEvent)]
pub struct Unknown10;

impl ClientProtMessage for Unknown10 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown10
    }
}

#[client_prot(VarShort { min: 0 }, ClientEvent)]
pub struct CreateAccount;

impl ClientProtMessage for CreateAccount {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        CreateAccount
    }
}

#[client_prot(VarByte { min: 0 }, ClientEvent)]
pub struct AffinedClansettingsAddBannedFromChannel;

impl ClientProtMessage for AffinedClansettingsAddBannedFromChannel {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        AffinedClansettingsAddBannedFromChannel
    }
}

#[client_prot(VarShort { min: 0 }, ClientEvent)]
pub struct AddNewEmailAddress;

impl ClientProtMessage for AddNewEmailAddress {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        AddNewEmailAddress
    }
}

#[client_prot(Fixed(4), ClientEvent)]
pub struct SoundSongEnd;

impl ClientProtMessage for SoundSongEnd {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        SoundSongEnd
    }
}

#[client_prot(Fixed(7), ClientEvent)]
pub struct EventNativeMouseClick;

impl ClientProtMessage for EventNativeMouseClick {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        EventNativeMouseClick
    }
}

#[client_prot(Fixed(2), ClientEvent)]
pub struct DirectxFailure;

impl ClientProtMessage for DirectxFailure {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        DirectxFailure
    }
}

#[client_prot(Fixed(15), ClientEvent)]
pub struct AutoSetupResult;

impl ClientProtMessage for AutoSetupResult {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        AutoSetupResult
    }
}

#[client_prot(Fixed(9), ClientEvent)]
pub struct SendPingReply;

impl ClientProtMessage for SendPingReply {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        SendPingReply
    }
}

#[client_prot(Fixed(1), ClientEvent)]
pub struct CreateLogProgress;

impl ClientProtMessage for CreateLogProgress {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        CreateLogProgress
    }
}

#[client_prot(Fixed(4), ClientEvent)]
pub struct PingStatistics;

impl ClientProtMessage for PingStatistics {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        PingStatistics
    }
}

#[client_prot(VarShort { min: 0 }, ClientEvent)]
pub struct BugReport;

impl ClientProtMessage for BugReport {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        BugReport
    }
}

#[client_prot(VarByte { min: 0 }, ClientEvent)]
pub struct EventNativeMouseMove;

impl ClientProtMessage for EventNativeMouseMove {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        EventNativeMouseMove
    }
}

#[client_prot(Fixed(0), ClientEvent)]
pub struct UidPassportResendRequest;

impl ClientProtMessage for UidPassportResendRequest {
    fn decode(_: &mut Packet, _: usize) -> Self {
        UidPassportResendRequest
    }
}

#[client_prot(VarByte { min: 0 }, ClientEvent)]
pub struct SendEmailValidationCode;

impl ClientProtMessage for SendEmailValidationCode {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        SendEmailValidationCode
    }
}

#[client_prot(VarShort { min: 0 }, ClientEvent)]
pub struct ChangeEmailAddress;

impl ClientProtMessage for ChangeEmailAddress {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        ChangeEmailAddress
    }
}
