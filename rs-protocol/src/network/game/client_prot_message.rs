use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;

pub trait ClientProtMessageInfo {
    const FRAME: ClientProtFrame;
    const CATEGORY: ClientProtCategory;
}

pub trait ClientProtMessage: ClientProtMessageInfo {
    fn decode(buf: &mut Packet, len: usize) -> Self;
}
