use crate::active_player::ActivePlayer;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client_prot::{ClanJoinchatLeavechat, ClanKickUser, FriendSetRank};
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::{ClanchannelKickUser, CreateCheckName};
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
    ClanJoinchatLeavechat => "friends chat",
    ClanKickUser => "friends chat",
    FriendSetRank => "friends-chat ranks",
);

#[cfg(since_647)]
unsupported!(
    ClanchannelKickUser => "clan channels",
    CreateCheckName => "lobby name checks",
);
