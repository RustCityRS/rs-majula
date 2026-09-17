use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(VarByte { min: 0 }, RestrictedEvent)]
pub struct RebuildGetMaps {
    pub maps: Vec<i32>,
}

impl ClientProtMessage for RebuildGetMaps {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        RebuildGetMaps {
            maps: (0..len / 3).map(|_| buf.g3()).collect(),
        }
    }
}
