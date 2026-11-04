//! `CreateCheckName` — rev-647 opcode 49 (var_byte): a name to check while
//! creating an account.
//!
//! CS2 command 4701, sent only from the lobby (client state 7) and for a name
//! of at most 20 characters: `pjstr(name)` (`Class47`). The answer is
//! CREATE_CHECK_NAME, which CS2 4700 reads back.
use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(VarByte { min: 0 }, ClientEvent)]
pub struct CreateCheckName {
    pub name: String,
}

impl ClientProtMessage for CreateCheckName {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        CreateCheckName { name: buf.gjstr(0) }
    }
}
