//! `MapBuildLoading` — rev-647 opcode 13 (fixed 4).
//!
//! Sent once an in-game rebuild (client state 11) has built the scene, before
//! MAP_BUILD_COMPLETE: `p4(1057001181)`, a constant (`Class20_Sub1`).
//! void-634 decodes the same packet as `RegionLoading` and discards it.
use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(4), ClientEvent)]
pub struct MapBuildLoading {
    pub constant: i32,
}

impl ClientProtMessage for MapBuildLoading {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        MapBuildLoading {
            constant: buf.g4s(),
        }
    }
}
