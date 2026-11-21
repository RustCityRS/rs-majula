use crate::active_player::ActivePlayer;
use crate::handlers::ClientGameHandler;
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::CreateCheckName;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client_prot::{ClanJoinchatLeavechat, ClanKickUser, FriendSetRank};
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

#[cfg(all(since_530, before_647))]
unsupported!(
    ClanJoinchatLeavechat => "friends chat",
    ClanKickUser => "friends chat",
    FriendSetRank => "friends-chat ranks",
);

#[cfg(since_647)]
unsupported!(
    CreateCheckName => "lobby name checks",
);
