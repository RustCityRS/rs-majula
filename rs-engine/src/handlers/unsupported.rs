use crate::active_player::ActivePlayer;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client_prot::{
    ApCoordT, ClanJoinchatLeavechat, ClanKickUser, ClanchannelKickUser, CreateCheckName,
    FriendSetRank, MessagePrivate, MessageQuickchatPrivate, MessageQuickchatPublic, OpPlayer6,
    OpPlayer7, OpPlayer8, OpPlayer9, OpPlayer10, ReflectionCheckReply, ResumePClanforumqfcDialog,
    ResumePHslDialog, ResumePNameDialog, ResumePObjDialog, UrlRequest, WorldlistFetch,
};
use rs_vm::ScriptError;
use tracing::debug;

macro_rules! unsupported {
    ($($packet:ty => $system:literal),* $(,)?) => {$(
        impl ClientGameHandler for $packet {
            fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
                debug!(packet = stringify!($packet), system = $system, "Unsupported client request");
                Ok(())
            }
        }
    )*};
}

unsupported!(
    ApCoordT => "ground-target script triggers",
    ClanJoinchatLeavechat => "friends chat",
    ClanKickUser => "friends chat",
    ClanchannelKickUser => "clan channels",
    CreateCheckName => "lobby name checks",
    FriendSetRank => "friends-chat ranks",
    MessagePrivate => "647 private messaging through ether",
    MessageQuickchatPrivate => "quick chat",
    MessageQuickchatPublic => "quick chat",
    OpPlayer6 => "player operation 6 triggers",
    OpPlayer7 => "player operation 7 triggers",
    OpPlayer8 => "player operation 8 triggers",
    OpPlayer9 => "player operation 9 triggers",
    OpPlayer10 => "player operation 10 triggers",
    ReflectionCheckReply => "pending reflection checks",
    ResumePClanforumqfcDialog => "clan-forum dialogs",
    ResumePHslDialog => "colour dialogs",
    ResumePNameDialog => "name dialogs",
    ResumePObjDialog => "object dialogs",
    UrlRequest => "URL request routing",
    WorldlistFetch => "world-list service",
);
