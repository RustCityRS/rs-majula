#[cfg(since_274)]
use crate::network::game::server::ServerProtMessage;
#[cfg(since_274)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_274)]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(since_274)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_274)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_274)]
use rs_protocol_macros::server_prot;

/// Controls the state of the client minimap.
///
/// `minimap_type` values: `0` normal, `1` disable click, `2` blacked out.
#[cfg(since_274)]
#[server_prot(MinimapToggle, Buffered, Fixed)]
pub struct MinimapToggle {
    pub minimap_type: u8,
}

#[cfg(since_274)]
impl ServerProtMessage for MinimapToggle {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.minimap_type);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.minimap_type)
    }
}
