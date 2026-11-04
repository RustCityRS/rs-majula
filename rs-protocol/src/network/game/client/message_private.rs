use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

// Minimum covers the base37 username.
#[cfg(before_647)]
#[client_prot(VarByte { min: 8 }, UserEvent)]
pub struct MessagePrivate {
    pub user37: i64,
    pub bytes: Vec<u8>,
}

#[cfg(before_647)]
impl ClientProtMessage for MessagePrivate {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let user37 = buf.g8s();
        let mut bytes = vec![0; len - buf.pos];
        buf.gdata(&mut bytes, 0, len - buf.pos);
        MessagePrivate { user37, bytes }
    }
}

// `MessagePrivate` — rev-647 MESSAGE_PRIVATE (opcode 32, var_byte).
//
// The rev-647 client declares `MESSAGE_PRIVATE(32, -1)` (`ClientProt.java`)
// and writes the size with `psize1` (`Class47.java`), i.e. a single-byte
// length prefix — so the frame is VarByte, NOT VarShort.

#[cfg(since_647)]
#[client_prot(VarByte { min: 0 }, UserEvent)]
pub struct MessagePrivate {
    pub target: String,
    pub payload: Vec<u8>,
}

#[cfg(since_647)]
impl ClientProtMessage for MessagePrivate {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let target = buf.gjstr(0);
        let mut payload = Vec::with_capacity(buf.remaining() as usize);
        while buf.remaining() > 0 {
            payload.push(buf.g1());
        }
        MessagePrivate { target, payload }
    }
}
