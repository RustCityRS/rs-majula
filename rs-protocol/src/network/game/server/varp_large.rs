use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[server_prot(VarpLarge, Immediate, Fixed)]
pub struct VarpLarge {
    pub id: u16,
    pub val: i32,
}

#[cfg(before_530)]
impl ServerProtMessage for VarpLarge {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.id);
        buf.p4(self.val);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.val)
    }
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for VarpLarge {
    fn encode(&self, buf: &mut Packet) {
        buf.p4(self.val);
        buf.p2_alt2(self.id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.val)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for VarpLarge {
    #[cfg(before_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.id);
        buf.p4_alt2(self.val);
    }

    #[cfg(since_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt1(self.val);
        buf.p2_alt2(self.id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.val)
    }
}
