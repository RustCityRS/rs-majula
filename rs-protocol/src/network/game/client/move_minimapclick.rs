use crate::network::game::client::{ClientProtMessage, pack_coord};
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

// Minimum covers ctrl + x + z, plus the 14 trailing camera bytes that the
// waypoint count is measured against.
#[client_prot(VarByte { min: 19 }, UserEvent)]
pub struct MoveMinimapClick {
    pub path: Vec<u32>,
    pub ctrl: bool,
}

impl ClientProtMessage for MoveMinimapClick {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let ctrl = buf.g1();
        let x = buf.g2();
        let z = buf.g2();

        let waypoints = (len - buf.pos - 14) / 2;
        let mut path = Vec::with_capacity(1 + waypoints.min(24));
        path.push(pack_coord(x, z));

        for _ in 1..=waypoints.min(24) {
            path.push(pack_coord(
                x.wrapping_add_signed(buf.g1s() as i16),
                z.wrapping_add_signed(buf.g1s() as i16),
            ));
        }
        MoveMinimapClick {
            path,
            ctrl: ctrl != 0,
        }
    }
}
