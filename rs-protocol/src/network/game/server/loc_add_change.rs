use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
use rs_ver::TypeId;

#[server_prot(LocAddChange, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct LocAddChange {
    pub coord: u8,
    pub shape_angle: u8,
    pub id: TypeId,
}

#[cfg(before_530)]
impl ServerProtMessage for LocAddChange {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p1(self.shape_angle);
        buf.p2(self.id.g2());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.shape_angle) + size_of::<u16>()
    }
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for LocAddChange {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt1(self.shape_angle);
        buf.p1(self.coord);
        buf.p2_alt2(self.id.g2());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.shape_angle) + size_of_val(&self.id)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for LocAddChange {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p1_alt3(self.coord);
            buf.p1_alt2(self.shape_angle);
            buf.p2_alt2(self.id.g2());
        }
        #[cfg(since_742)]
        {
            buf.p4_alt3(self.id.int());
            buf.p1_alt3(self.coord);
            buf.p1_alt1(self.shape_angle);
        }
    }

    fn sizeof(&self) -> usize {
        #[cfg(before_742)]
        let id = size_of::<u16>();
        #[cfg(since_742)]
        let id = size_of::<i32>();
        size_of_val(&self.coord) + size_of_val(&self.shape_angle) + id
    }
}
