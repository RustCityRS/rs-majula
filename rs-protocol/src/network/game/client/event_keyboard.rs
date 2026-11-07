use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeySample {
    pub key: u8,
    pub gap: u16,
}

#[client_prot(VarByte { min: 0 }, ClientEvent)]
pub struct EventKeyboard {
    pub keys: Vec<KeySample>,
}

impl ClientProtMessage for EventKeyboard {
    fn validate(data: &[u8]) -> bool {
        data.len().is_multiple_of(3)
    }

    fn decode(buf: &mut Packet, len: usize) -> Self {
        let keys = (0..len / 3)
            .map(|_| KeySample {
                key: buf.g1(),
                gap: buf.g2(),
            })
            .collect();
        EventKeyboard { keys }
    }
}
