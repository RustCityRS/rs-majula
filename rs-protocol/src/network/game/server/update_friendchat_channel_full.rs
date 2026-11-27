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
use rs_ver::STRING_TERMINATOR;

#[cfg(all(since_530, before_647))]
pub struct FriendchatMember<'a> {
    pub user37: i64,
    pub world: u16,
    pub rank: i8,
    pub world_name: &'a str,
}

#[cfg(all(since_530, before_647))]
#[server_prot(UpdateFriendchatChannelFull, Buffered, VarShort)]
pub struct UpdateFriendchatChannelFull<'a> {
    pub owner37: i64,
    pub channel37: i64,
    pub min_rank: i8,
    pub members: &'a [FriendchatMember<'a>],
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for UpdateFriendchatChannelFull<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p8(self.owner37);
        if self.owner37 == 0 {
            return;
        }
        buf.p8(self.channel37);
        buf.p1(self.min_rank as u8);
        buf.p1(self.members.len() as u8);
        for member in self.members {
            buf.p8(member.user37);
            buf.p2(member.world);
            buf.p1(member.rank as u8);
            buf.pjstr(member.world_name, STRING_TERMINATOR);
        }
    }

    fn sizeof(&self) -> usize {
        if self.owner37 == 0 {
            return size_of_val(&self.owner37);
        }
        let members: usize = self
            .members
            .iter()
            .map(|m| {
                size_of_val(&m.user37)
                    + size_of_val(&m.world)
                    + size_of_val(&m.rank)
                    + Packet::sizejstr(m.world_name)
            })
            .sum();
        size_of_val(&self.owner37)
            + size_of_val(&self.channel37)
            + size_of_val(&self.min_rank)
            + 1
            + members
    }
}

#[cfg(since_647)]
pub struct FriendchatMember {
    pub name: String,
    pub display_name: Option<String>,
    pub world: u16,
    pub rank: i8,
    pub world_name: String,
}

#[cfg(since_647)]
impl FriendchatMember {
    /// Writes the member as the client reads one, with its world's name
    /// unless it is leaving (`with_world_name` false).
    pub fn encode(&self, buf: &mut Packet, with_world_name: bool) {
        buf.pjstr(&self.name, STRING_TERMINATOR);
        if let Some(display) = &self.display_name {
            buf.p1(1);
            buf.pjstr(display, STRING_TERMINATOR);
        } else {
            buf.p1(0);
        }
        buf.p2(self.world);
        buf.p1(self.rank as u8);
        if with_world_name {
            buf.pjstr(&self.world_name, STRING_TERMINATOR);
        }
    }

    pub fn sizeof(&self, with_world_name: bool) -> usize {
        Packet::sizejstr(&self.name)
            + size_of::<u8>()
            + self.display_name.as_deref().map_or(0, Packet::sizejstr)
            + size_of_val(&self.world)
            + size_of_val(&self.rank)
            + if with_world_name {
                Packet::sizejstr(&self.world_name)
            } else {
                0
            }
    }
}

#[cfg(since_647)]
pub struct FriendchatChannel {
    pub owner: String,
    pub channel37: i64,
    pub min_kick: i8,
    pub members: Vec<FriendchatMember>,
}

#[cfg(since_647)]
#[server_prot(UpdateFriendchatChannelFull, Buffered, VarShort)]
pub struct UpdateFriendchatChannelFull {
    pub channel: Option<FriendchatChannel>,
}

#[cfg(since_647)]
impl ServerProtMessage for UpdateFriendchatChannelFull {
    fn encode(&self, buf: &mut Packet) {
        let Some(channel) = &self.channel else {
            return;
        };
        buf.pjstr(&channel.owner, STRING_TERMINATOR);
        buf.p1(0);
        buf.p8(channel.channel37);
        buf.p1(channel.min_kick as u8);
        buf.p1(channel.members.len() as u8);
        for member in &channel.members {
            member.encode(buf, true);
        }
    }

    fn sizeof(&self) -> usize {
        let Some(channel) = &self.channel else {
            return 0;
        };
        let members: usize = channel.members.iter().map(|m| m.sizeof(true)).sum();
        Packet::sizejstr(&channel.owner)
            + size_of::<u8>()
            + size_of_val(&channel.channel37)
            + size_of_val(&channel.min_kick)
            + size_of::<u8>()
            + members
    }
}
