use crate::active_player::ActivePlayer;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client_prot::{
    ApCoordT, ClanJoinchatLeavechat, ClanKickUser, ClanchannelKickUser, CreateCheckName,
    FriendSetRank, UrlRequest, WorldlistFetch,
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
    UrlRequest => "URL request routing",
    WorldlistFetch => "world-list service",
);
