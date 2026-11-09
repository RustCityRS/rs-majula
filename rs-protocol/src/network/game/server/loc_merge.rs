use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(LocMerge, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct LocMerge {
    pub coord: u8,
    pub shape_angle: u8,
    pub id: u16,
    pub start: u16,
    pub end: u16,
    pub pid: u16,
    pub east: i8,
    pub south: i8,
    pub west: i8,
    pub north: i8,
}

#[cfg(before_647)]
impl ServerProtMessage for LocMerge {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p1(self.shape_angle);
        buf.p2(self.id);
        buf.p2(self.start);
        buf.p2(self.end);
        buf.p2(self.pid);
        buf.p1(self.east as u8);
        buf.p1(self.south as u8);
        buf.p1(self.west as u8);
        buf.p1(self.north as u8);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of_val(&self.shape_angle)
            + size_of_val(&self.id)
            + size_of_val(&self.start)
            + size_of_val(&self.end)
            + size_of_val(&self.pid)
            + size_of_val(&self.south)
            + size_of_val(&self.east)
            + size_of_val(&self.north)
            + size_of_val(&self.west)
    }
}

/// Swaps the models, recolours and retextures of the loc on `coord`'s
/// `shape_angle` layer for the ones given; none of them resets it. The client
/// takes each list's length from loc type `id`'s models for that shape and its
/// recolour and retexture destinations.
#[cfg(since_647)]
#[server_prot(LocMerge, Immediate, VarByte)]
#[derive(Debug, Clone)]
pub struct LocMerge {
    pub coord: u8,
    pub id: u16,
    pub shape_angle: u8,
    pub models: Option<Box<[u16]>>,
    pub recol: Option<Box<[u16]>>,
    pub retex: Option<Box<[u16]>>,
}

#[cfg(since_647)]
impl LocMerge {
    fn sections(&self) -> [&Option<Box<[u16]>>; 3] {
        [&self.models, &self.recol, &self.retex]
    }
}

#[cfg(since_647)]
impl ServerProtMessage for LocMerge {
    fn encode(&self, buf: &mut Packet) {
        let flags = self
            .sections()
            .iter()
            .enumerate()
            .fold(0, |flags, (i, section)| {
                flags | (section.is_some() as u8) << (i + 1)
            });
        buf.p1_alt3(if flags == 0 { 1 } else { flags });
        buf.p1(self.coord);
        buf.p2(self.id);
        buf.p1_alt2(self.shape_angle);
        for section in self.sections().into_iter().flatten() {
            for &value in section.iter() {
                buf.p2(value);
            }
        }
    }

    fn sizeof(&self) -> usize {
        1 // flags
            + size_of_val(&self.coord)
            + size_of_val(&self.id)
            + size_of_val(&self.shape_angle)
            + self.sections().into_iter().flatten().map(|section| section.len() << 1).sum::<usize>()
    }
}
