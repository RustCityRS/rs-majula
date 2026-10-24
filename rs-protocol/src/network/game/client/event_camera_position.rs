use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

/// Where the camera points, sent every so often while it's turning.
#[cfg_attr(rev = "225", client_prot(Fixed(6), ClientEvent))]
#[cfg_attr(since_254, client_prot(Fixed(4), ClientEvent))]
pub struct EventCameraPosition {
    pub pitch: u16,
    pub yaw: u16,
    #[cfg(rev = "225")]
    pub minimap_angle: u8,
    #[cfg(rev = "225")]
    pub minimap_zoom: u8,
}

impl ClientProtMessage for EventCameraPosition {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        EventCameraPosition {
            pitch: buf.g2(),
            yaw: buf.g2(),
            #[cfg(rev = "225")]
            minimap_angle: buf.g1(),
            #[cfg(rev = "225")]
            minimap_zoom: buf.g1(),
        }
    }
}
