use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessageInfo, jstr_len};
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(UpdateFriendList, Buffered, Fixed)]
pub struct UpdateFriendList {
    pub user37: i64,
    pub node: u8,
}

#[cfg(before_647)]
impl ServerProtMessage for UpdateFriendList {
    fn encode(&self, buf: &mut Packet) {
        buf.p8(self.user37);
        buf.p1(self.node);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.user37) + size_of_val(&self.node)
    }
}

/// One friend-list entry, in rev-647 `UPDATE_FRIENDLIST` field order.
///
/// The client (`MapAreaConfig.java`) reads, per entry:
/// `renamed:u8 · displayName:jstr · previousName:jstr · world:u16 · rank:u8`
/// then, only when `world > 0`, `worldName:jstr · gameQuickChat:u8`.
///
/// So `status` carries the *renamed* flag (0 = not renamed) and `name` is the
/// **shown** name while `previous_name` is the rename-history name (set equal to
/// `name` when there is no rename).
#[cfg(since_647)]
pub struct FriendEntry {
    pub status: u8,
    pub name: String,
    pub previous_name: String,
    pub world: u16,
    pub rank: u8,
    pub world_name: Option<String>,
    pub world_flag: Option<u8>,
}

#[cfg(since_647)]
#[server_prot(UpdateFriendlist, Buffered, VarShort)]
pub struct UpdateFriendlist {
    pub friends: Vec<FriendEntry>,
}

#[cfg(since_647)]
impl ServerProtMessage for UpdateFriendlist {
    fn encode(&self, buf: &mut Packet) {
        for friend in &self.friends {
            buf.p1(friend.status);
            buf.pjstr(&friend.name, 0);
            buf.pjstr(&friend.previous_name, 0);
            buf.p2(friend.world);
            buf.p1(friend.rank);
            if friend.world > 0 {
                buf.pjstr(friend.world_name.as_deref().unwrap_or(""), 0);
                buf.p1(friend.world_flag.unwrap_or(0));
            }
        }
    }

    fn sizeof(&self) -> usize {
        self.friends
            .iter()
            .map(|f| {
                let base = 1
                    + jstr_len(&f.name)
                    + jstr_len(&f.previous_name)
                    + size_of_val(&f.world)
                    + size_of_val(&f.rank);
                if f.world > 0 {
                    base + f.world_name.as_ref().map_or(1, |s| jstr_len(s))
                        + size_of_val(&f.world_flag)
                } else {
                    base
                }
            })
            .sum()
    }
}
