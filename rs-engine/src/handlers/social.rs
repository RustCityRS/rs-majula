use crate::active_player::ActivePlayer;
#[cfg(before_647)]
use crate::clients::client_ether::{EtherOutbound, MAX_IGNORES, max_friends_cap};
use crate::core::engine;
#[cfg(before_647)]
use crate::counters::AutoBan;
use crate::handlers::ClientGameHandler;
#[cfg(since_647)]
use crate::social::SocialRequest;
#[cfg(since_742)]
use rs_protocol::network::game::client_prot::AffinedClansettingsAddBannedFromChannel;
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::{
    ClanJoinchatLeavechat, ClanKickUser, ClanchannelKickUser, FriendSetRank,
};
use rs_protocol::network::game::client_prot::{
    FriendListAdd, FriendListDel, IgnoreListAdd, IgnoreListDel,
};
#[cfg(before_647)]
use rs_util::base37::is_valid_userhash;
use rs_vm::ScriptError;
#[cfg(before_647)]
use rs_vm::engine::ScriptPlayer;

/// Whether `user37` is a name hash no real client sends, banning the player
/// who sent it. Before 647 the client sends only names it encoded itself or
/// took from its own lists, and checks the lists before it sends a change.
#[cfg(before_647)]
pub(crate) fn forged_name(active: &mut ActivePlayer, user37: i64) -> bool {
    if is_valid_userhash(user37 as u64) {
        return false;
    }
    active.auto_ban(AutoBan::PrivateName);
    true
}

/// Adds a friend by name hash. The world keeps the list and the ether stores
/// it and shows the friend's world. The client won't add a name on its
/// ignore list, so one comes from a modified client.
#[cfg(before_647)]
impl ClientGameHandler for FriendListAdd {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let user37 = self.user37;
        if forged_name(active, user37) {
            return Ok(());
        }
        if active.ignores.contains(&user37) {
            active.auto_ban(AutoBan::PrivateName);
            return Ok(());
        }
        if active.uid().username37() == user37 as u64
            || active.friends.contains(&user37)
            || active.friends.len() >= max_friends_cap(active.player.is_member) as usize
        {
            return Ok(());
        }
        active.friends.push(user37);
        engine().send_ether(EtherOutbound::FriendAdd {
            owner37: active.uid().username37(),
            friend37: user37 as u64,
        });
        Ok(())
    }
}

/// Removes a friend by name hash. The client drops the name from its own
/// list as it sends this.
#[cfg(before_647)]
impl ClientGameHandler for FriendListDel {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let user37 = self.user37;
        if forged_name(active, user37)
            || active.uid().username37() == user37 as u64
            || !active.friends.contains(&user37)
        {
            return Ok(());
        }
        active.friends.retain(|&f| f != user37);
        active.watch.forget_friend(user37 as u64);
        engine().send_ether(EtherOutbound::FriendDel {
            owner37: active.uid().username37(),
            friend37: user37 as u64,
        });
        Ok(())
    }
}

/// Ignores a player by name hash; an ignore has no self check. The client
/// won't ignore a name on its friends list, so one comes from a modified
/// client.
#[cfg(before_647)]
impl ClientGameHandler for IgnoreListAdd {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let user37 = self.user37;
        if forged_name(active, user37) {
            return Ok(());
        }
        if active.friends.contains(&user37) {
            active.auto_ban(AutoBan::PrivateName);
            return Ok(());
        }
        if active.ignores.contains(&user37) || active.ignores.len() >= MAX_IGNORES {
            return Ok(());
        }
        active.ignores.push(user37);
        engine().send_ether(EtherOutbound::IgnoreAdd {
            owner37: active.uid().username37(),
            ignore37: user37 as u64,
        });
        Ok(())
    }
}

/// Stops ignoring a player by name hash.
#[cfg(before_647)]
impl ClientGameHandler for IgnoreListDel {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let user37 = self.user37;
        if forged_name(active, user37) || !active.ignores.contains(&user37) {
            return Ok(());
        }
        active.ignores.retain(|&i| i != user37);
        engine().send_ether(EtherOutbound::IgnoreDel {
            owner37: active.uid().username37(),
            ignore37: user37 as u64,
        });
        Ok(())
    }
}

/// From 647 the client sends names as typed and leaves the checks to the
/// server: each request goes to ether as its [`SocialRequest`] message, and
/// ether answers with the change or the reason it can't be made.
#[cfg(since_647)]
macro_rules! social_requests {
    ($($(#[$meta:meta])* $request:ty),* $(,)?) => {$(
        $(#[$meta])*
        impl ClientGameHandler for $request {
            fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
                if let Some(message) = self.ether(active) {
                    engine().send_ether(message);
                }
                Ok(())
            }
        }
    )*};
}

#[cfg(since_647)]
social_requests!(
    FriendListAdd,
    FriendListDel,
    IgnoreListAdd,
    IgnoreListDel,
    FriendSetRank,
    ClanJoinchatLeavechat,
    ClanKickUser,
    ClanchannelKickUser,
    #[cfg(since_742)]
    AffinedClansettingsAddBannedFromChannel,
);
