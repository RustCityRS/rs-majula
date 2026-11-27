use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
#[cfg(since_530)]
use rs_ver::STRING_TERMINATOR;

#[cfg(before_530)]
#[server_prot(UpdateFriendList, Buffered, Fixed)]
pub struct UpdateFriendList {
    pub user37: i64,
    pub node: u8,
}

#[cfg(before_530)]
impl ServerProtMessage for UpdateFriendList {
    fn encode(&self, buf: &mut Packet) {
        buf.p8(self.user37);
        buf.p1(self.node);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.user37) + size_of_val(&self.node)
    }
}

#[cfg(all(since_530, before_647))]
#[server_prot(UpdateFriendList, Buffered, VarByte)]
pub struct UpdateFriendList<'a> {
    pub user37: i64,
    pub world: u16,
    pub rank: u8,
    pub world_name: &'a str,
    pub world_flag: u8,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for UpdateFriendList<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p8(self.user37 | ((self.world_flag == 0) as i64) << 63);
        buf.p2(self.world);
        buf.p1(self.rank);
        if self.world > 0 {
            buf.pjstr(self.world_name, STRING_TERMINATOR);
        }
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.user37)
            + size_of_val(&self.world)
            + size_of_val(&self.rank)
            + if self.world > 0 {
                Packet::sizejstr(self.world_name)
            } else {
                0
            }
    }
}

#[cfg(since_647)]
pub struct FriendEntry {
    pub renamed: bool,
    pub name: String,
    pub previous_name: String,
    pub world: u16,
    pub rank: u8,
    #[cfg(since_742)]
    pub referred: bool,
    pub world_name: String,
    pub platform: u8,
    #[cfg(since_742)]
    pub world_flags: i32,
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
            buf.p1(friend.renamed as u8);
            buf.pjstr(&friend.name, STRING_TERMINATOR);
            buf.pjstr(&friend.previous_name, STRING_TERMINATOR);
            buf.p2(friend.world);
            buf.p1(friend.rank);
            #[cfg(since_742)]
            buf.p1(friend.referred as u8);
            if friend.world > 0 {
                buf.pjstr(&friend.world_name, STRING_TERMINATOR);
                buf.p1(friend.platform);
                #[cfg(since_742)]
                buf.p4(friend.world_flags);
            }
        }
    }

    fn sizeof(&self) -> usize {
        self.friends
            .iter()
            .map(|f| {
                let base = size_of::<u8>()
                    + Packet::sizejstr(&f.name)
                    + Packet::sizejstr(&f.previous_name)
                    + size_of_val(&f.world)
                    + size_of_val(&f.rank);
                #[cfg(since_742)]
                let base = base + size_of::<u8>();
                if f.world == 0 {
                    return base;
                }
                let online = base + Packet::sizejstr(&f.world_name) + size_of_val(&f.platform);
                #[cfg(since_742)]
                let online = online + size_of_val(&f.world_flags);
                online
            })
            .sum()
    }
}
