//! `WorldlistFetch` — rev-647 WORLDLIST_FETCH (opcode 45, fixed 4).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(4), ClientEvent)]
pub struct WorldlistFetch {
    pub update_val: i32,
}

impl ClientProtMessage for WorldlistFetch {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        WorldlistFetch {
            update_val: buf.g4s(),
        }
    }
}
