#[cfg(since_647)]
use crate::network::game::server::update_friendchat_channel_full::FriendchatMember;
#[cfg(since_530)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_530)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_530)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_530)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_530)]
use rs_protocol_macros::server_prot;
#[cfg(all(since_530, before_647))]
use rs_ver::STRING_TERMINATOR;

#[cfg(all(since_530, before_647))]
#[server_prot(UpdateFriendchatChannelSingleuser, Buffered, VarByte)]
pub struct UpdateFriendchatChannelSingleuser<'a> {
    pub user37: i64,
    pub world: u16,
    pub rank: i8,
    pub world_name: &'a str,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for UpdateFriendchatChannelSingleuser<'_> {
    fn encode(&self, buf: &mut Packet) {
        let removed = self.rank == -128;
        buf.p8(self.user37 | (removed as i64) << 63);
        buf.p2(self.world);
        buf.p1(self.rank as u8);
        if !removed {
            buf.pjstr(self.world_name, STRING_TERMINATOR);
        }
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.user37)
            + size_of_val(&self.world)
            + size_of_val(&self.rank)
            + if self.rank == -128 {
                0
            } else {
                Packet::sizejstr(self.world_name)
            }
    }
}

#[cfg(since_647)]
pub const FRIENDCHAT_REMOVED: i8 = -128;

#[cfg(since_647)]
#[server_prot(UpdateFriendchatChannelSingleuser, Buffered, VarByte)]
pub struct UpdateFriendchatChannelSingleuser {
    pub member: FriendchatMember,
}

#[cfg(since_647)]
impl ServerProtMessage for UpdateFriendchatChannelSingleuser {
    fn encode(&self, buf: &mut Packet) {
        self.member
            .encode(buf, self.member.rank != FRIENDCHAT_REMOVED);
    }

    fn sizeof(&self) -> usize {
        self.member.sizeof(self.member.rank != FRIENDCHAT_REMOVED)
    }
}
