#[cfg(since_530)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_530)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_530)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_530)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_530)]
use rs_protocol_macros::server_prot;
use rs_ver::TypeId;

#[cfg(since_530)]
#[server_prot(LocAnimSpecific, Immediate, Fixed)]
pub struct LocAnimSpecific {
    pub seq: TypeId,
    pub shape_angle: u8,
    pub coord: i32,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for LocAnimSpecific {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt3(self.shape_angle);
        buf.p2(self.seq.g2());
        buf.p4(self.coord);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.seq) + size_of_val(&self.shape_angle) + size_of_val(&self.coord)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for LocAnimSpecific {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p2_alt1(self.seq.g2());
            buf.p1_alt3(self.shape_angle);
            buf.p4(self.coord);
        }
        #[cfg(since_742)]
        {
            buf.p1_alt1(self.shape_angle);
            buf.p4(self.coord);
            buf.p4_alt3(self.seq.int());
        }
    }

    fn sizeof(&self) -> usize {
        #[cfg(before_742)]
        let seq = size_of::<u16>();
        #[cfg(since_742)]
        let seq = size_of::<i32>();
        seq + size_of_val(&self.shape_angle) + size_of_val(&self.coord)
    }
}
