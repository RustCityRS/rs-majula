use crate::network::game::client::{ClientProtMessage, pack_coord};
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

/// The 14 bytes every revision's client adds after a minimap click's path:
/// where on the minimap it clicked, the camera and minimap it saw that
/// through, and where it drew the player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MinimapTrailer {
    /// The click, in minimap pixels from its centre (right and down).
    pub x: i8,
    pub y: i8,
    /// The camera's yaw (0-2047).
    pub yaw: u16,
    /// Always 57.
    pub magic1: u8,
    /// The client's random tilt of the minimap against the camera (-62 to
    /// 62, drifting 2 at a time) and its zoom (-21 to 11, 1 at a time).
    pub angle: i8,
    pub zoom: i8,
    /// Always 89.
    pub magic2: u8,
    /// Where the client drew the player, in 1/128ths of a tile from the
    /// corner of its loaded map.
    pub fine_x: u16,
    pub fine_z: u16,
    /// 1 when the clicked tile couldn't be reached and the path goes next to
    /// it instead.
    pub nearest: u8,
    /// Always 63.
    pub magic3: u8,
}

// Minimum covers ctrl + x + z, plus the 14 trailing camera bytes that the
// waypoint count is measured against.
#[client_prot(VarByte { min: 19 }, UserEvent)]
pub struct MoveMinimapClick {
    pub path: Vec<u32>,
    pub ctrl: bool,
    pub trailer: MinimapTrailer,
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

        // The trailer is the last 14 bytes, however many waypoints were read.
        buf.pos = len - 14;
        let trailer = MinimapTrailer {
            x: buf.g1s(),
            y: buf.g1s(),
            yaw: buf.g2(),
            magic1: buf.g1(),
            angle: buf.g1s(),
            zoom: buf.g1s(),
            magic2: buf.g1(),
            fine_x: buf.g2(),
            fine_z: buf.g2(),
            nearest: buf.g1(),
            magic3: buf.g1(),
        };
        MoveMinimapClick {
            path,
            ctrl: ctrl != 0,
            trailer,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_trailer_after_the_path() {
        // ctrl 0, start (3222, 3218), one waypoint (+2, -1), then the trailer.
        let mut data = vec![0, 0x0c, 0x96, 0x0c, 0x92, 2, 0xff];
        data.extend_from_slice(&[
            0xf6, 20, 0x01, 0x00, 57, 0xc4, 0x05, 89, 0x1a, 0x40, 0x19, 0xc0, 1, 63,
        ]);
        let len = data.len();
        let msg = MoveMinimapClick::decode(&mut Packet::from(data), len);
        assert_eq!(
            msg.path,
            vec![pack_coord(3222, 3218), pack_coord(3224, 3217)]
        );
        assert_eq!(
            msg.trailer,
            MinimapTrailer {
                x: -10,
                y: 20,
                yaw: 256,
                magic1: 57,
                angle: -60,
                zoom: 5,
                magic2: 89,
                fine_x: 0x1a40,
                fine_z: 0x19c0,
                nearest: 1,
                magic3: 63,
            }
        );
    }
}
