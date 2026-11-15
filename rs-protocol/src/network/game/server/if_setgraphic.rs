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
use rs_ver::TypeId;

#[cfg(since_647)]
#[server_prot(IfSetGraphic, Buffered, Fixed)]
pub struct IfSetGraphic {
    pub com: i32,
    pub graphic: TypeId,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetGraphic {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p4_alt1(self.com);
            buf.p2_alt2(self.graphic.g2());
        }
        #[cfg(since_742)]
        {
            buf.p4_alt2(self.graphic.int());
            buf.p4(self.com);
        }
    }

    fn sizeof(&self) -> usize {
        #[cfg(before_742)]
        let graphic = size_of::<u16>();
        #[cfg(since_742)]
        let graphic = size_of::<i32>();
        size_of_val(&self.com) + graphic
    }
}
