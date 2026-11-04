//! `WindowStatus` — rev-647 WINDOW_STATUS (opcode 69, fixed 6).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(6), ClientEvent)]
pub struct WindowStatus {
    pub window_mode: u8,
    pub width: u16,
    pub height: u16,
}

impl ClientProtMessage for WindowStatus {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let window_mode = buf.g1();
        let width = buf.g2();
        let height = buf.g2();
        let _unused = buf.g1();
        WindowStatus {
            window_mode,
            width,
            height,
        }
    }
}
