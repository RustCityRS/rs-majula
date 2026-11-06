#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_647)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;
use rs_ver::STRING_TERMINATOR;

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
                buf.p2(member.world);
            } else {
                buf.p1(0);
            }
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
                let display = m
                    .display_name
                    .as_ref()
                    .map_or(0, |d| Packet::sizejstr(d) + 2);
                base + display + size_of_val(&m.rank) + Packet::sizejstr(&m.title)
            })
            .sum();
        header + members
    }
}
