//! `ReflectionCheckReply` — rev-647 opcode 90 (var_byte).
//!
//! The client's answer to a reflection check: for each field or method the
//! check asked about, its value or an error code (`AClass31_Sub1.method2357`,
//! drained from the queue of privileged requests). The client sends one only
//! for a check the server asked for. Kept as its frame.
use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(VarByte { min: 0 }, ClientEvent)]
pub struct ReflectionCheckReply {
    pub payload: Vec<u8>,
}

impl ClientProtMessage for ReflectionCheckReply {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let payload = (0..len).map(|_| buf.g1()).collect();
        ReflectionCheckReply { payload }
    }
}
