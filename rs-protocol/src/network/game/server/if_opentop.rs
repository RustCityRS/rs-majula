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

// Rev-647 IF_OPENTOP (opcode 61), read by MapAreaConfig.java:563 as `ig2
// interface` (little-endian, which `p2_alt1` writes) and `g1_alt3 mode` (the
// client's name for `byte - 128`, which `p1_alt1` writes as `mode + 128`).
// Mode 2 also runs ImageSourceTextureOp.method3619:
// client state 11, the "Loading - please wait." screen that rebuilds the
// scene, then a cache trim and a garbage collection.
#[cfg(since_530)]
#[server_prot(IfOpentop, Buffered, Fixed)]
pub struct IfOpentop {
    pub interface: u16,
    pub mode: u8,
    #[cfg(all(since_530, before_647))]
    pub verify: u16,
    #[cfg(since_742)]
    pub xtea_keys: [i32; 4],
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for IfOpentop {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt3(self.interface);
        buf.p1_alt1(self.mode);
        buf.p2_alt3(self.verify);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.interface) + size_of_val(&self.mode) + size_of_val(&self.verify)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for IfOpentop {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p2_alt1(self.interface);
            buf.p1_alt1(self.mode);
        }
        #[cfg(since_742)]
        {
            buf.p1(self.mode);
            buf.p4_alt1(self.xtea_keys[0]);
            buf.p2_alt1(self.interface);
            buf.p4(self.xtea_keys[2]);
            buf.p4_alt1(self.xtea_keys[3]);
            buf.p4_alt2(self.xtea_keys[1]);
        }
    }

    fn sizeof(&self) -> usize {
        #[cfg(before_742)]
        let keys = 0;
        #[cfg(since_742)]
        let keys = size_of_val(&self.xtea_keys);
        size_of_val(&self.interface) + size_of_val(&self.mode) + keys
    }
}
