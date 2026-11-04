use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(IfSetModel, Buffered, Fixed)]
pub struct IfSetModel {
    pub com: u16,
    pub model: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetModel {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.model);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.model)
    }
}

// `IfSetModel` — display a 3D model on an interface component.
// Rev-647 opcode 91 (IF_SETMODEL), fixed 6.
// Layout: `[ip2_alt1 model_id][p4 component_hash]`.
//
// Client read order (from decompiled rev-647 `MapAreaConfig`):
// ```text
// int model = input_packet.ig2_alt1();     // -1 == 65535
// int hash  = input_packet.g4();
// ```

#[cfg(since_647)]
#[server_prot(IfSetModel, Buffered, Fixed)]
pub struct IfSetModel {
    pub component_hash: i32,
    /// Model id. Pass `-1` / `65535` to clear the component's model.
    pub model_id: i16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetModel {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2_alt1(self.model_id as u16);
        buf.p4(self.component_hash);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.component_hash) + size_of_val(&self.model_id)
    }
}
