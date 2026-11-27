#[cfg(since_254)]
use crate::network::game::client::ClientProtMessage;
#[cfg(since_254)]
use crate::network::game::client::event_mouse_move::Cursor;
#[cfg(since_254)]
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
#[cfg(since_254)]
use crate::network::game::client_prot_message::ClientProtMessageInfo;
#[cfg(since_254)]
use rs_io::Packet;
#[cfg(since_254)]
use rs_protocol_macros::client_prot;

#[cfg(since_254)]
#[cfg_attr(before_530, client_prot(Fixed(4), ClientEvent))]
#[cfg_attr(since_530, client_prot(Fixed(6), ClientEvent))]
pub struct EventMouseClick {
    pub at: Cursor,
    pub right: bool,
    pub gap: u16,
}

#[cfg(all(since_254, before_530))]
impl ClientProtMessage for EventMouseClick {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let v = buf.g4s() as u32;
        EventMouseClick {
            at: Cursor::unpack(v & 0x7ffff),
            right: (v >> 19) & 1 == 1,
            gap: (v >> 20) as u16,
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for EventMouseClick {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let info = buf.g2_alt3();
        let at = Cursor::unpack(buf.g4_alt3() as u32);
        EventMouseClick {
            at,
            right: info >> 15 == 1,
            gap: info & 0x7fff,
        }
    }
}

#[cfg(since_647)]
impl ClientProtMessage for EventMouseClick {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let info = buf.g2();
        #[cfg(before_742)]
        let at = Cursor::unpack(buf.g4_alt2() as u32);
        #[cfg(since_742)]
        let at = Cursor::unpack(buf.g4s() as u32);
        EventMouseClick {
            at,
            right: info >> 15 == 1,
            gap: info & 0x7fff,
        }
    }
}
