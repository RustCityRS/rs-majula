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
pub struct ClanChannelUser {
    pub user_hash: Option<i64>,
    pub display_name: Option<String>,
    pub real_name: Option<String>,
    pub rank: u8,
    pub status: u8,
    #[cfg(since_742)]
    pub world: u16,
}

#[cfg(since_647)]
pub struct ClanChannelData {
    pub flags: u8,
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
        if let Some(ref ch) = self.channel {
            buf.p1(ch.flags);
            buf.p8(ch.clan_hash);
            buf.p8(ch.update_num);
            buf.pjstr(&ch.name, STRING_TERMINATOR);
            buf.p1(0);
            buf.p1(ch.rank_kick as u8);
            buf.p1(ch.rank_talk as u8);
            buf.p2(ch.users.len() as u16);
            let use_user_hashes = ch.flags & 0x1 != 0;
            let use_display_names = ch.flags & 0x2 != 0;
            for user in &ch.users {
                if use_user_hashes {
                    buf.p8(user.user_hash.unwrap_or(0));
                }
                if use_display_names {
                    buf.pjstr(
                        user.display_name.as_deref().unwrap_or(""),
                        STRING_TERMINATOR,
                    );
                    #[cfg(before_742)]
                    buf.pjstr(user.real_name.as_deref().unwrap_or(""), STRING_TERMINATOR);
                }
                buf.p1(user.rank);
                #[cfg(before_742)]
                buf.p1(user.status);
                #[cfg(since_742)]
                buf.p2(user.world);
            }
        }
    }

    fn sizeof(&self) -> usize {
        match &self.channel {
            None => size_of_val(&self.affined),
            Some(ch) => {
                let use_user_hashes = ch.flags & 0x1 != 0;
                let use_display_names = ch.flags & 0x2 != 0;
                let header = size_of_val(&self.affined)
                    + size_of_val(&ch.flags)
                    + size_of_val(&ch.clan_hash)
                    + size_of_val(&ch.update_num)
                    + Packet::sizejstr(&ch.name)
                    + 1
                    + size_of_val(&ch.rank_kick)
                    + size_of_val(&ch.rank_talk)
                    + 2;
                let users: usize = ch
                    .users
                    .iter()
                    .map(|u| {
                        #[cfg(before_742)]
                        let mut sz = size_of_val(&u.rank) + size_of_val(&u.status);
                        #[cfg(since_742)]
                        let mut sz = size_of_val(&u.rank) + size_of_val(&u.world);
                        if use_user_hashes {
                            sz += size_of_val(&u.user_hash.unwrap_or(0));
                        }
                        if use_display_names {
                            sz += u.display_name.as_ref().map_or(1, |s| Packet::sizejstr(s));
                            #[cfg(before_742)]
                            {
                                sz += u.real_name.as_ref().map_or(1, |s| Packet::sizejstr(s));
                            }
                        }
                        sz
                    })
                    .sum();
                header + users
            }
        }
    }
}
