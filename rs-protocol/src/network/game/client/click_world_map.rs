//! `ClickWorldMap` — rev-647 CLICKWORLDMAP (opcode 60, fixed 4).
//!
//! A click on the world map, at the square it shows (`client`, the world map
//! loop): `p4_alt2(level << 28 | x << 14 | z)`. Staff holding ctrl teleport
//! there locally instead. void-634 (`WorldMap.kt`) sets the custom map marker
//! on a second click at the same square.

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(4), UserEvent)]
pub struct ClickWorldMap {
    pub coord: u32,
}

impl ClientProtMessage for ClickWorldMap {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ClickWorldMap {
            coord: buf.g4_alt2() as u32,
        }
    }
}
