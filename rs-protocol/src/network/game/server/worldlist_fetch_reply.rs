use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

const LOCATION: &str = "Otherworld";
const HOST: &str = "127.0.0.1";

/// The world list with this world alone, in one location.
#[server_prot(WorldlistFetchReply, Immediate, VarShort)]
pub struct WorldlistFetchReply {
    pub world: u16,
}

impl ServerProtMessage for WorldlistFetchReply {
    fn encode(&self, buf: &mut Packet) {
        let world = self.world as i32;
        buf.p1(1);
        buf.p1(2);
        buf.p1(1);

        buf.psmart1or2(1);
        buf.psmart1or2(0);
        buf.p1(0);
        buf.pjstr(LOCATION, 0);

        buf.psmart1or2(world);
        buf.psmart1or2(world);
        buf.psmart1or2(1);

        buf.psmart1or2(0);
        buf.p1(0);
        buf.p4(1);
        buf.p1(0);
        buf.pjstr("", 0);
        buf.p1(0);
        buf.pjstr(HOST, 0);

        buf.p4(0);

        buf.psmart1or2(0);
        buf.p2(10);
    }

    fn sizeof(&self) -> usize {
        let smart = if self.world < 128 { 1 } else { 2 };
        22 + 2 * smart + Packet::sizejstr(LOCATION) + Packet::sizejstr("") + Packet::sizejstr(HOST)
    }
}
