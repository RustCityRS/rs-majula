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
const DISPLAY_NAMES: u8 = 0x2;

#[cfg(since_647)]
pub struct ClanChannelUser {
    pub display_name: String,
    pub rank: i8,
    pub world: u16,
}

#[cfg(since_647)]
pub struct ClanChannelData {
    pub clan_hash: i64,
    pub update_num: i64,
    pub name: String,
    pub rank_kick: i8,
    pub rank_talk: i8,
    pub users: Vec<ClanChannelUser>,
}

#[cfg(since_647)]
#[server_prot(ClanchannelFull, Buffered, VarShort)]
pub struct ClanchannelFull {
    pub affined: bool,
    pub channel: Option<ClanChannelData>,
}

#[cfg(since_647)]
impl ServerProtMessage for ClanchannelFull {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.affined as u8);
        let Some(channel) = &self.channel else {
            return;
        };
        buf.p1(DISPLAY_NAMES);
        buf.p8(channel.clan_hash);
        buf.p8(channel.update_num);
        buf.pjstr(&channel.name, STRING_TERMINATOR);
        buf.p1(0);
        buf.p1(channel.rank_kick as u8);
        buf.p1(channel.rank_talk as u8);
        buf.p2(channel.users.len() as u16);
        for user in &channel.users {
            buf.pjstr(&user.display_name, STRING_TERMINATOR);
            buf.p1(user.rank as u8);
            buf.p2(user.world);
        }
    }

    fn sizeof(&self) -> usize {
        let Some(channel) = &self.channel else {
            return size_of_val(&self.affined);
        };
        let users: usize = channel
            .users
            .iter()
            .map(|u| {
                Packet::sizejstr(&u.display_name) + size_of_val(&u.rank) + size_of_val(&u.world)
            })
            .sum();
        size_of_val(&self.affined)
            + size_of_val(&DISPLAY_NAMES)
            + size_of_val(&channel.clan_hash)
            + size_of_val(&channel.update_num)
            + Packet::sizejstr(&channel.name)
            + size_of::<u8>()
            + size_of_val(&channel.rank_kick)
            + size_of_val(&channel.rank_talk)
            + size_of::<u16>()
            + users
    }
}
