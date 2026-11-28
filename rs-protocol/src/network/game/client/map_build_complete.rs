#[cfg(since_530)]
use crate::network::game::client::ClientProtEncode;
#[cfg(since_254)]
use crate::network::game::client::ClientProtMessage;
#[cfg(since_530)]
use crate::network::game::client_prot::ClientProt;
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
#[client_prot(Fixed(0), ClientEvent)]
#[derive(Debug)]
pub struct MapBuildComplete;

#[cfg(since_254)]
impl ClientProtMessage for MapBuildComplete {
    fn decode(_: &mut Packet, _: usize) -> Self {
        MapBuildComplete
    }
}

#[cfg(since_530)]
impl ClientProtEncode for MapBuildComplete {
    const PROT: ClientProt = ClientProt::MapBuildComplete;

    fn encode(&self, _: &mut Packet) {}
}
