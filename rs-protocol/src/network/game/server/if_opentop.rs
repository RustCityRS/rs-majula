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

// Rev-647 IF_OPENTOP (opcode 61): `ig2 interface`, `g1_alt3 mode`
// (MapAreaConfig.java:563). Mode 2 also runs ImageSourceTextureOp.method3619:
// client state 11, the "Loading - please wait." screen that rebuilds the
// scene, then a cache trim and a garbage collection.
#[cfg(since_647)]
#[server_prot(IfOpentop, Buffered, Fixed)]
pub struct IfOpentop {
    pub interface: u16,
    pub mode: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for IfOpentop {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.interface);
        buf.p1_alt1(self.mode);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.interface) + size_of_val(&self.mode)
    }
}
