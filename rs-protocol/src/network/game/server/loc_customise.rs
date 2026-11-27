use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
use rs_ver::TypeId;

/// Swaps the models, recolours and retextures of the loc on `coord`'s
/// `shape_angle` layer for the ones given; none of them resets it. The client
/// takes each list's length from loc type `id`'s models for that shape and its
/// recolour and retexture destinations.
#[server_prot(LocCustomise, Immediate, VarByte)]
#[derive(Debug, Clone)]
pub struct LocCustomise {
    pub coord: u8,
    pub id: TypeId,
    pub shape_angle: u8,
    pub models: Option<Box<[TypeId]>>,
    pub recol: Option<Box<[u16]>>,
    pub retex: Option<Box<[u16]>>,
}

impl LocCustomise {
    fn flags(&self) -> u8 {
        let flags = (self.models.is_some() as u8) << 1
            | (self.recol.is_some() as u8) << 2
            | (self.retex.is_some() as u8) << 3;
        if flags == 0 { 1 } else { flags }
    }

    fn colours(&self) -> [&Option<Box<[u16]>>; 2] {
        [&self.recol, &self.retex]
    }
}

impl ServerProtMessage for LocCustomise {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p1_alt3(self.flags());
            buf.p1(self.coord);
            buf.p2(self.id.g2());
            buf.p1_alt2(self.shape_angle);
        }
        #[cfg(since_742)]
        {
            buf.p1_alt3(self.shape_angle);
            buf.p1_alt3(self.flags());
            buf.p4_alt3(self.id.int());
            buf.p1_alt3(self.coord);
        }
        for &model in self.models.as_deref().unwrap_or_default() {
            #[cfg(before_742)]
            buf.p2(model.g2());
            #[cfg(since_742)]
            buf.p4(model.int());
        }
        for section in self.colours().into_iter().flatten() {
            for &value in section.iter() {
                buf.p2(value);
            }
        }
    }

    fn sizeof(&self) -> usize {
        #[cfg(before_742)]
        let (id, model_shift) = (size_of::<u16>(), 1);
        #[cfg(since_742)]
        let (id, model_shift) = (size_of::<i32>(), 2);
        1 // flags
            + size_of_val(&self.coord)
            + id
            + size_of_val(&self.shape_angle)
            + self.models.as_deref().map_or(0, |models| models.len() << model_shift)
            + self.colours().into_iter().flatten().map(|section| section.len() << 1).sum::<usize>()
    }
}
