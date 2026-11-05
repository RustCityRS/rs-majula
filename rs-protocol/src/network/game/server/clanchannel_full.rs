#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::jstr_len;
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
pub struct ClanChannelUser {
    pub user_hash: Option<i64>,
    pub display_name: Option<String>,
    pub real_name: Option<String>,
    pub rank: u8,
    pub status: u8,
}

#[cfg(since_647)]
pub struct ClanChannelData {
    pub flags: u8,
    pub link_id: i64,
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
        if let Some(ref ch) = self.channel {
            buf.p1(ch.flags);
            buf.p8(ch.link_id);
            buf.p8(ch.update_num);
            buf.pjstr(&ch.name, STRING_TERMINATOR);
            buf.p1(0);
            buf.p1(ch.rank_kick as u8);
            buf.p1(ch.rank_talk as u8);
            buf.p2(ch.users.len() as u16);
            let use_hashes = ch.flags & 0x1 != 0;
            let use_names = ch.flags & 0x2 != 0;
            for user in &ch.users {
                if use_hashes {
                    buf.p8(user.user_hash.unwrap_or(0));
                }
                if use_names {
                    buf.pjstr(
                        user.display_name.as_deref().unwrap_or(""),
                        STRING_TERMINATOR,
                    );
                    buf.pjstr(user.real_name.as_deref().unwrap_or(""), STRING_TERMINATOR);
                }
                buf.p1(user.rank);
                buf.p1(user.status);
            }
        }
    }

    fn sizeof(&self) -> usize {
        match &self.channel {
            None => size_of_val(&self.affined),
            Some(ch) => {
                let use_hashes = ch.flags & 0x1 != 0;
                let use_names = ch.flags & 0x2 != 0;
                let header = size_of_val(&self.affined)
                    + size_of_val(&ch.flags)
                    + size_of_val(&ch.link_id)
                    + size_of_val(&ch.update_num)
                    + jstr_len(&ch.name)
                    + 1
                    + size_of_val(&ch.rank_kick)
                    + size_of_val(&ch.rank_talk)
                    + 2;
                let users: usize = ch
                    .users
                    .iter()
                    .map(|u| {
                        let mut sz = size_of_val(&u.rank) + size_of_val(&u.status);
                        if use_hashes {
                            sz += size_of_val(&u.user_hash.unwrap_or(0));
                        }
                        if use_names {
                            sz += u.display_name.as_ref().map_or(1, |s| jstr_len(s));
                            sz += u.real_name.as_ref().map_or(1, |s| jstr_len(s));
                        }
                        sz
                    })
                    .sum();
                header + users
            }
        }
    }
}
