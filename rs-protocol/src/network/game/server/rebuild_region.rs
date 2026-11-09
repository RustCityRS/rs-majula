#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_647)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::packet::BitWriter;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;

#[cfg(since_647)]
#[server_prot(RebuildRegion, Immediate, VarShort)]
pub struct RebuildRegion {
    pub player_x: u16,
    pub player_z: u16,
    pub size: u8,
    pub map_type: u8,
    pub force: bool,
    pub templates: Vec<Option<u32>>,
    pub xtea_keys: Vec<[i32; 4]>,
}

#[cfg(since_647)]
impl RebuildRegion {
    fn template_bytes(&self) -> usize {
        let bits: usize = self
            .templates
            .iter()
            .map(|template| if template.is_some() { 27 } else { 1 })
            .sum();
        (bits + 7) >> 3
    }
}

#[cfg(since_647)]
impl ServerProtMessage for RebuildRegion {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt1(self.size);
        buf.p1_alt3(self.map_type);
        buf.p2_alt1(self.player_x >> 3);
        buf.p2_alt1(self.player_z >> 3);
        buf.p1_alt1(self.force as u8);

        let bytes = self.template_bytes();
        let mut bits = Packet::new(bytes);
        let mut writer = BitWriter::new();
        for template in &self.templates {
            match template {
                Some(template) => {
                    writer.pbit::<1>(&mut bits, 1);
                    writer.pbit::<26>(&mut bits, *template as i32);
                }
                None => writer.pbit::<1>(&mut bits, 0),
            }
        }
        writer.finish(&mut bits);
        buf.pdata(&bits.data, 0, bytes);

        for keys in &self.xtea_keys {
            for &key in keys {
                buf.p4(key);
            }
        }
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.size)
            + size_of_val(&self.map_type)
            + size_of_val(&self.player_x)
            + size_of_val(&self.player_z)
            + size_of_val(&self.force)
            + self.template_bytes()
            + (self.xtea_keys.len() << 4)
    }
}
