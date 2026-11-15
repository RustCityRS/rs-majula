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

#[cfg(since_530)]
#[server_prot(IfOpensub, Buffered, Fixed)]
pub struct IfOpensub {
    pub interface: u16,
    pub com: i32,
    pub walkable: u8,
    #[cfg(all(since_530, before_647))]
    pub verify: u16,
    #[cfg(since_742)]
    pub xtea_keys: [i32; 4],
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for IfOpensub {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.walkable);
        buf.p4_alt3(self.com);
        buf.p2_alt2(self.verify);
        buf.p2(self.interface);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.interface)
            + size_of_val(&self.com)
            + size_of_val(&self.walkable)
            + size_of_val(&self.verify)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for IfOpensub {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p2_alt3(self.interface);
            buf.p4_alt3(self.com);
            buf.p1_alt1(self.walkable);
        }
        #[cfg(since_742)]
        {
            buf.p4(self.xtea_keys[0]);
            buf.p4_alt3(self.xtea_keys[1]);
            buf.p4_alt2(self.com);
            buf.p1_alt3(self.walkable);
            buf.p2(self.interface);
            buf.p4(self.xtea_keys[3]);
            buf.p4_alt1(self.xtea_keys[2]);
        }
    }

    fn sizeof(&self) -> usize {
        #[cfg(before_742)]
        let keys = 0;
        #[cfg(since_742)]
        let keys = size_of_val(&self.xtea_keys);
        size_of_val(&self.interface) + size_of_val(&self.com) + size_of_val(&self.walkable) + keys
    }
}
