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
#[server_prot(TextCoord, Immediate, VarByte)]
#[derive(Debug, Clone)]
pub struct TextCoord {
    pub coord: u8,
    pub duration: u16,
    pub height: u8,
    pub colour: i32,
    pub text: String,
}

#[cfg(since_647)]
impl ServerProtMessage for TextCoord {
    fn encode(&self, buf: &mut Packet) {
        buf.p1((self.sizeof() - 1) as u8);
        buf.p1(self.coord);
        buf.p2(self.duration);
        buf.p1(self.height);
        buf.p3(self.colour);
        buf.pjstr(&self.text, STRING_TERMINATOR);
    }

    fn sizeof(&self) -> usize {
        1 // len
            + size_of_val(&self.coord)
            + size_of_val(&self.duration)
            + size_of_val(&self.height)
            + 3 // colour
            + Packet::sizejstr(&self.text)
    }
}
