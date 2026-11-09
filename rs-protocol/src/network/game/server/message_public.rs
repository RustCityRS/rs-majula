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

#[cfg(since_647)]
#[server_prot(MessagePublic, Buffered, VarByte)]
pub struct MessagePublic<'a> {
    pub from_index: u16,
    pub effects: u16,
    pub staffmodlevel: u8,
    pub char_count: usize,
    pub huffman_data: &'a [u8],
}

#[cfg(since_647)]
impl ServerProtMessage for MessagePublic<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.from_index);
        buf.p2(self.effects);
        buf.p1(self.staffmodlevel);
        buf.psmart1or2(self.char_count as i32);
        buf.pdata(self.huffman_data, 0, self.huffman_data.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.from_index)
            + size_of_val(&self.effects)
            + size_of_val(&self.staffmodlevel)
            + if (self.char_count as i32) < 128 { 1 } else { 2 }
            + self.huffman_data.len()
    }
}
