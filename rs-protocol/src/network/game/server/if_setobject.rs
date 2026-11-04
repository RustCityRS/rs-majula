use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(IfSetObject, Buffered, Fixed)]
pub struct IfSetObject {
    pub com: u16,
    pub obj: u16,
    pub scale: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetObject {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.obj);
        buf.p2(self.scale);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.obj) + size_of_val(&self.scale)
    }
}

// `IfSetObject` — show an item model on an interface component.
// Rev-647 opcode 93 (IF_SETOBJECT), fixed 10.
// Layout: `[ip4 component][p4_alt3 zoom][p2 item_id]`.

#[cfg(since_647)]
#[server_prot(IfSetObject, Buffered, Fixed)]
pub struct IfSetObject {
    pub component_hash: i32,
    pub item_id: u16,
    pub zoom: i32,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetObject {
    fn encode(&self, buf: &mut Packet) {
        buf.ip4(self.component_hash);
        buf.p4_alt3(self.zoom);
        buf.p2(self.item_id);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.component_hash) + size_of_val(&self.item_id) + size_of_val(&self.zoom)
    }
}
