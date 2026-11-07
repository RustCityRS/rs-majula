use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};

pub trait ServerProtMessageInfo {
    const PROT: ServerProt;
    const PRIORITY: ServerProtPriority;
    const FRAME: PacketFrame;
}

pub trait ServerProtMessage: ServerProtMessageInfo {
    fn encode(&self, buf: &mut Packet);
    fn sizeof(&self) -> usize;
}
