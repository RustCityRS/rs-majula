use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

pub struct ClanChannelUser {
    pub user_hash: Option<i64>,
    pub display_name: Option<String>,
    pub real_name: Option<String>,
    pub rank: u8,
    pub status: u8,
}

pub struct ClanChannelData {
    pub flags: u8,
    pub link_id: i64,
    pub update_num: i64,
    pub name: String,
    pub rank_kick: i8,
    pub rank_talk: i8,
    pub users: Vec<ClanChannelUser>,
}

#[server_prot(ClanchannelFull, Buffered, VarShort)]
pub struct ClanchannelFull {
    pub affined: bool,
    pub channel: Option<ClanChannelData>,
}

impl ServerProtMessage for ClanchannelFull {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(if self.affined { 1 } else { 0 });
        if let Some(ref ch) = self.channel {
            buf.p1(ch.flags);
            buf.p8(ch.link_id);
            buf.p8(ch.update_num);
            buf.pjstr(&ch.name, 0);
            buf.p1(0);
            buf.p1(ch.rank_kick as u8);
            buf.p1(ch.rank_talk as u8);
            buf.p2(ch.users.len() as u16);
            let use_hashes = ch.flags & 1 != 0;
            let use_names = ch.flags & 2 != 0;
            for user in &ch.users {
                if use_hashes {
                    buf.p8(user.user_hash.unwrap_or(0));
                }
                if use_names {
                    buf.pjstr(user.display_name.as_deref().unwrap_or(""), 0);
                    buf.pjstr(user.real_name.as_deref().unwrap_or(""), 0);
                }
                buf.p1(user.rank);
                buf.p1(user.status);
            }
        }
    }

    fn sizeof(&self) -> usize {
        match &self.channel {
            None => 1,
            Some(ch) => {
                let use_hashes = ch.flags & 1 != 0;
                let use_names = ch.flags & 2 != 0;
                let header = 1 + 1 + 8 + 8 + ch.name.len() + 1 + 1 + 1 + 1 + 2;
                let users: usize = ch
                    .users
                    .iter()
                    .map(|u| {
                        let mut sz = 1 + 1;
                        if use_hashes {
                            sz += 8;
                        }
                        if use_names {
                            sz += u.display_name.as_ref().map_or(1, |s| s.len() + 1);
                            sz += u.real_name.as_ref().map_or(1, |s| s.len() + 1);
                        }
                        sz
                    })
                    .sum();
                header + users
            }
        }
    }
}
