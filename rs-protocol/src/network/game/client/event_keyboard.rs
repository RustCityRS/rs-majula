use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(before_742)]
const SAMPLE_LEN: usize = 3;
#[cfg(since_742)]
const SAMPLE_LEN: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeySample {
    pub key: u8,
    pub gap: u32,
}

#[cfg_attr(before_742, client_prot(VarByte { min: 0 }, ClientEvent))]
#[cfg_attr(since_742, client_prot(VarShort { min: 0 }, ClientEvent))]
pub struct EventKeyboard {
    pub keys: Vec<KeySample>,
}

impl ClientProtMessage for EventKeyboard {
    fn validate(data: &[u8]) -> bool {
        data.len().is_multiple_of(SAMPLE_LEN)
    }

    fn decode(buf: &mut Packet, len: usize) -> Self {
        let keys = (0..len / SAMPLE_LEN)
            .map(|_| KeySample {
                key: buf.g1(),
                #[cfg(before_742)]
                gap: buf.g2() as u32,
                #[cfg(since_742)]
                gap: buf.g3() as u32,
            })
            .collect();
        EventKeyboard { keys }
    }
}
