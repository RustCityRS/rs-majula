//! `ClanchannelKickUser` — rev-647 CLANCHANNEL_KICKUSER (opcode 88, var_byte).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(VarByte { min: 0 }, UserEvent)]
pub struct ClanchannelKickUser {
    pub name: String,
}

impl ClientProtMessage for ClanchannelKickUser {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ClanchannelKickUser { name: buf.gjstr(0) }
    }
}
