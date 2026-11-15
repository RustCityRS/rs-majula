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
    pub title: String,
}

#[cfg(since_647)]
#[server_prot(UpdateFriendchatChannelFull, Buffered, VarShort)]
pub struct UpdateFriendchatChannelFull {
    pub owner: String,
    pub alt_owner: Option<String>,
    pub channel_id: i64,
    pub min_rank: i8,
    pub members: Vec<FriendchatMember>,
}

#[cfg(since_647)]
impl ServerProtMessage for UpdateFriendchatChannelFull {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.owner, STRING_TERMINATOR);
        if let Some(ref alt) = self.alt_owner {
            buf.p1(1);
            buf.pjstr(alt, STRING_TERMINATOR);
        } else {
            buf.p1(0);
        }
        buf.p8(self.channel_id);
        buf.p1(self.min_rank as u8);
        buf.p1(self.members.len() as u8);
        for member in &self.members {
            buf.pjstr(&member.name, STRING_TERMINATOR);
            if let Some(ref display) = member.display_name {
                buf.p1(1);
                buf.pjstr(display, STRING_TERMINATOR);
                #[cfg(before_742)]
                buf.p2(member.world);
            } else {
                buf.p1(0);
            }
            #[cfg(since_742)]
            buf.p2(member.world);
            buf.p1(member.rank as u8);
            buf.pjstr(&member.title, STRING_TERMINATOR);
        }
    }

    fn sizeof(&self) -> usize {
        let header = self.owner.len()
            + 1
            + 1
            + self.alt_owner.as_ref().map_or(0, |s| Packet::sizejstr(s))
            + size_of_val(&self.channel_id)
            + size_of_val(&self.min_rank)
            + 1;
        let members: usize = self
            .members
            .iter()
            .map(|m| {
                let base = Packet::sizejstr(&m.name) + 1;
                #[cfg(before_742)]
                let display = m
                    .display_name
                    .as_ref()
                    .map_or(0, |d| Packet::sizejstr(d) + 2);
                #[cfg(since_742)]
                let display = m.display_name.as_ref().map_or(0, |d| Packet::sizejstr(d))
                    + size_of_val(&m.world);
                base + display + size_of_val(&m.rank) + Packet::sizejstr(&m.title)
            })
            .sum();
        header + members
    }
}
