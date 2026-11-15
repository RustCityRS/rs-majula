use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::TypeId;

#[client_prot(Fixed(2), UserEvent)]
pub struct ResumePObjDialog {
    pub id: TypeId,
}

impl ClientProtMessage for ResumePObjDialog {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ResumePObjDialog {
            id: TypeId::from_g2(buf.g2()),
        }
    }
}
