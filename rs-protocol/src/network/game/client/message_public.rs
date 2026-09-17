use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

// Minimum covers colour + effect.
#[client_prot(VarByte { min: 2 }, UserEvent)]
pub struct MessagePublic {
    pub colour: u8,
    pub effect: u8,
    pub bytes: Vec<u8>,
}

impl ClientProtMessage for MessagePublic {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let colour = buf.g1();
        let effect = buf.g1();
        let mut bytes = vec![0; len - buf.pos];
        buf.gdata(&mut bytes, 0, len - buf.pos);
        MessagePublic {
            colour,
            effect,
            bytes,
        }
    }
}
