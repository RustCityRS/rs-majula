use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
#[cfg(since_647)]
use crate::network::game::client_prot_message::smart_end;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg_attr(before_647, client_prot(VarByte { min: 2 }, UserEvent))]
#[cfg_attr(since_647, client_prot(VarByte { min: 3 }, UserEvent))]
pub struct MessagePublic {
    pub colour: u8,
    pub effect: u8,
    #[cfg(since_647)]
    pub chars: u16,
    pub bytes: Vec<u8>,
}

#[cfg(before_647)]
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

#[cfg(since_647)]
impl ClientProtMessage for MessagePublic {
    fn validate(data: &[u8]) -> bool {
        smart_end(data, 2).is_some()
    }

    fn decode(buf: &mut Packet, len: usize) -> Self {
        let colour = buf.g1();
        let effect = buf.g1();
        let chars = buf.gsmart1or2() as u16;
        let bytes = buf.data[buf.pos..len].to_vec();
        buf.pos = len;
        MessagePublic {
            colour,
            effect,
            chars,
            bytes,
        }
    }
}
