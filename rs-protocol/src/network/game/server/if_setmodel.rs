use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
use rs_ver::TypeId;

#[cfg(before_647)]
#[server_prot(IfSetModel, Buffered, Fixed)]
pub struct IfSetModel {
    pub com: u16,
    pub model: TypeId,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetModel {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.model.g2());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of::<u16>()
    }
}

#[cfg(since_647)]
#[server_prot(IfSetModel, Buffered, Fixed)]
pub struct IfSetModel {
    pub com: i32,
    pub model: TypeId,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetModel {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p2_alt3(self.model.g2());
            buf.p4(self.com);
        }
        #[cfg(since_742)]
        {
            buf.p4_alt1(self.model.int());
            buf.p4_alt1(self.com);
        }
    }

    fn sizeof(&self) -> usize {
        #[cfg(before_742)]
        let model = size_of::<u16>();
        #[cfg(since_742)]
        let model = size_of::<i32>();
        size_of_val(&self.com) + model
    }
}
