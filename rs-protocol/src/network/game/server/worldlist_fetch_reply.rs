use crate::network::game::MAX_PACKET_SIZE;
use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use crate::world_list::WorldList;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

impl WorldList {
    /// The reply to a fetch from a client that holds the list with
    /// `checksum`.
    pub fn reply(&self, checksum: i32) -> impl Iterator<Item = WorldlistFetchReply<'_>> {
        let chunks = self.bytes(checksum).chunks(MAX_PACKET_SIZE - 5);
        let count = chunks.len();
        chunks
            .enumerate()
            .map(move |(i, bytes)| WorldlistFetchReply {
                last: i + 1 == count,
                bytes,
            })
    }
}

#[server_prot(WorldlistFetchReply, Immediate, VarShort)]
pub struct WorldlistFetchReply<'a> {
    pub last: bool,
    pub bytes: &'a [u8],
}

impl ServerProtMessage for WorldlistFetchReply<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.last as u8);
        buf.pdata(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        1 + self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_list::WorldEntry;

    #[test]
    fn world_list_fragments_fit_wire_frames_and_end_once() {
        let worlds: Vec<_> = (1..=256)
            .map(|id| WorldEntry {
                id,
                address: format!("world{id}.{}", "example.".repeat(18)),
                players: id,
                members: true,
                location: "RuneScape".into(),
                country: 225,
                activity: None,
            })
            .collect();
        let list = WorldList::new(&worlds);
        let chunks: Vec<_> = list.reply(0).collect();
        assert!(chunks.len() > 1);
        assert_eq!(chunks.iter().filter(|c| c.last).count(), 1);
        assert!(chunks.last().unwrap().last);
        assert!(chunks.iter().all(|c| c.sizeof() + 4 <= MAX_PACKET_SIZE));
        assert_eq!(
            chunks
                .iter()
                .flat_map(|c| c.bytes.iter().copied())
                .collect::<Vec<_>>(),
            list.bytes(0)
        );
        assert!(WorldList::new(&[]).reply(0).next().unwrap().last);
    }
}
