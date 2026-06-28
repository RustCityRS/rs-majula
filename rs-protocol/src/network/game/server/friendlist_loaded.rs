#[cfg(since_254)]
use crate::network::game::server::ServerProtMessage;
#[cfg(since_254)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_254)]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(since_254)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_254)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_254)]
use rs_protocol_macros::server_prot;

/// Tells the client the loading state of its friend list.
///
/// Status values: `0` loading, `1` connecting to the friend server, `2` online
/// (loaded). Anything else shows "Please wait...".
#[cfg(since_254)]
#[server_prot(FriendListLoaded, Buffered, Fixed)]
pub struct FriendListLoaded {
    pub status: u8,
}

#[cfg(since_254)]
impl ServerProtMessage for FriendListLoaded {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.status);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.status)
    }
}
