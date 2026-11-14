use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
use rs_ver::TypeId;

#[server_prot(ObjReveal, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct ObjReveal {
    pub coord: u8,
    pub id: TypeId,
    pub count: u16,
    pub receiver: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for ObjReveal {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.id.g2());
        buf.p2(self.count);
        buf.p2(self.receiver);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of::<u16>()
            + size_of_val(&self.count)
            + size_of_val(&self.receiver)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for ObjReveal {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p2_alt3(self.id.g2());
            buf.p2(self.receiver);
            buf.p1_alt2(self.coord);
            buf.p2_alt3(self.count);
        }
        #[cfg(since_742)]
        {
            buf.p2_alt1(self.count);
            buf.p1_alt1(self.coord);
            buf.p2_alt1(self.receiver);
            buf.p2(self.id.g2());
        }
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of::<u16>()
            + size_of_val(&self.count)
            + size_of_val(&self.receiver)
    }
}
