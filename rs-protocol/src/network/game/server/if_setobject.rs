use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
use rs_ver::TypeId;

#[cfg(before_530)]
#[server_prot(IfSetObject, Buffered, Fixed)]
pub struct IfSetObject {
    pub com: u16,
    pub obj: TypeId,
    pub scale: u16,
}

#[cfg(before_530)]
impl ServerProtMessage for IfSetObject {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.obj.g2());
        buf.p2(self.scale);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of::<u16>() + size_of_val(&self.scale)
    }
}

#[cfg(since_530)]
#[server_prot(IfSetObject, Buffered, Fixed)]
pub struct IfSetObject {
    pub com: i32,
    pub obj: TypeId,
    pub scale: i32,
    #[cfg(all(since_530, before_647))]
    pub verify: u16,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for IfSetObject {
    fn encode(&self, buf: &mut Packet) {
        buf.p4(self.scale);
        buf.p4_alt3(self.com);
        buf.p2_alt3(self.obj.g2());
        buf.p2_alt1(self.verify);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com)
            + size_of_val(&self.obj)
            + size_of_val(&self.scale)
            + size_of_val(&self.verify)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetObject {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p4_alt1(self.com);
            buf.p4_alt3(self.scale);
            buf.p2(self.obj.g2());
        }
        #[cfg(since_742)]
        {
            buf.p4(self.scale);
            buf.p2_alt1(self.obj.g2());
            buf.p4_alt3(self.com);
        }
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of::<u16>() + size_of_val(&self.scale)
    }
}
