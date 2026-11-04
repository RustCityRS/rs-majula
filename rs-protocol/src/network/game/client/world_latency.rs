//! `WorldLatency` — rev-647 opcode 53 (fixed 2).
//!
//! The client pings the address of the world it is on itself and reports the
//! result, at most every 30 seconds (`VarbitType`, `Class228.method1489`):
//! `p2(result)`. void-634 decodes it as `Latency` and discards it.
use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(2), ClientEvent)]
pub struct WorldLatency {
    pub latency: u16,
}

impl ClientProtMessage for WorldLatency {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        WorldLatency { latency: buf.g2() }
    }
}
