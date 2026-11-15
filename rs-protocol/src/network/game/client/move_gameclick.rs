use crate::network::game::client::ClientProtMessage;
#[cfg(before_647)]
use crate::network::game::client::pack_coord;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(before_647)]
#[client_prot(VarByte { min: 5 }, UserEvent)]
pub struct MoveGameClick {
    pub path: Vec<u32>,
    pub ctrl: bool,
}

#[cfg(before_530)]
impl ClientProtMessage for MoveGameClick {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let ctrl = buf.g1();
        let x = buf.g2();
        let z = buf.g2();

        let waypoints = (len - buf.pos) / 2;
        let mut path = Vec::with_capacity(1 + waypoints.min(24));
        path.push(pack_coord(x, z));

        for _ in 1..=waypoints.min(24) {
            path.push(pack_coord(
                x.wrapping_add_signed(buf.g1s() as i16),
                z.wrapping_add_signed(buf.g1s() as i16),
            ));
        }
        MoveGameClick {
            path,
            ctrl: ctrl != 0,
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for MoveGameClick {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let ctrl = buf.g1_alt1();
        let x = buf.g2();
        let z = buf.g2_alt2();

        let waypoints = (len - buf.pos) / 2;
        let mut path = Vec::with_capacity(1 + waypoints.min(24));
        path.push(pack_coord(x, z));

        for _ in 1..=waypoints.min(24) {
            path.push(pack_coord(
                x.wrapping_add_signed(buf.g1_alt1() as i8 as i16),
                z.wrapping_add_signed(buf.g1_alt3() as i8 as i16),
            ));
        }
        MoveGameClick {
            path,
            ctrl: ctrl != 0,
        }
    }
}

#[cfg(since_647)]
#[client_prot(Fixed(5), UserEvent)]
pub struct MoveGameClick {
    pub x: u16,
    pub z: u16,
    pub ctrl: bool,
}

#[cfg(since_647)]
impl ClientProtMessage for MoveGameClick {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        #[cfg(before_742)]
        let (x, ctrl, z) = (buf.g2_alt1(), buf.g1_alt1() != 0, buf.g2());
        #[cfg(since_742)]
        let (x, z, ctrl) = (buf.g2_alt2(), buf.g2_alt3(), buf.g1_alt3() != 0);
        MoveGameClick { x, z, ctrl }
    }
}
