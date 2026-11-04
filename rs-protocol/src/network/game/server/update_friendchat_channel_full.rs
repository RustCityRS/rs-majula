use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

pub struct FriendchatMember {
    pub name: String,
    pub display_name: Option<String>,
    pub world: u16,
    pub rank: i8,
    pub title: String,
}

#[server_prot(UpdateFriendchatChannelFull, Buffered, VarShort)]
pub struct UpdateFriendchatChannelFull {
    pub owner: String,
    pub alt_owner: Option<String>,
    pub channel_id: i64,
    pub min_rank: i8,
    pub members: Vec<FriendchatMember>,
}

impl ServerProtMessage for UpdateFriendchatChannelFull {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.owner, 0);
        if let Some(ref alt) = self.alt_owner {
            buf.p1(1);
            buf.pjstr(alt, 0);
        } else {
            buf.p1(0);
        }
        buf.p8(self.channel_id);
        buf.p1(self.min_rank as u8);
        buf.p1(self.members.len() as u8);
        for member in &self.members {
            buf.pjstr(&member.name, 0);
            if let Some(ref display) = member.display_name {
                buf.p1(1);
                buf.pjstr(display, 0);
                buf.p2(member.world);
            } else {
                buf.p1(0);
            }
            buf.p1(member.rank as u8);
            buf.pjstr(&member.title, 0);
        }
    }

    fn sizeof(&self) -> usize {
        let header = self.owner.len()
            + 1
            + 1
            + self.alt_owner.as_ref().map_or(0, |s| s.len() + 1)
            + 8
            + 1
            + 1;
        let members: usize = self
            .members
            .iter()
            .map(|m| {
                let base = m.name.len() + 1 + 1;
                let display = m.display_name.as_ref().map_or(0, |d| d.len() + 1 + 2);
                base + display + 1 + m.title.len() + 1
            })
            .sum();
        header + members
    }
}
