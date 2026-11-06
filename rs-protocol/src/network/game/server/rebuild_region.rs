#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_647)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;

#[cfg(since_647)]
#[server_prot(RebuildRegion, Immediate, VarShort)]
pub struct RebuildRegion {
    pub player_x: u16,
    pub player_z: u16,
    /// Build-area slot index (rev-647 currently uses 0).
    pub build_area_idx: u8,
    /// Instance type (1 for standard instances).
    pub instance_type: u8,
    /// Login flag (0 for in-game rebuilds).
    pub login_flag: u8,
    /// 4 × 13 × 13 chunk templates, each pre-encoded as the 26-bit
    /// `Controller::encode_26bit()` packed value. `None` = empty chunk.
    pub templates: Vec<Option<u32>>,
    /// XTEA key blocks for the unique source mapsquares, in the order
    /// the encoder should emit them. Looked up by the engine.
    pub xtea_keys: Vec<[i32; 4]>,
}

#[cfg(since_647)]
impl ServerProtMessage for RebuildRegion {
    fn encode(&self, buf: &mut Packet) {
        let chunk_x = self.player_x >> 3;
        let chunk_z = self.player_z >> 3;

        // Header
        buf.p1_alt1(self.build_area_idx);
        buf.p1_alt3(self.instance_type);
        buf.p2_alt1(chunk_x);
        buf.p2_alt1(chunk_z);
        buf.p1_alt1(self.login_flag);

        // Bit-packed templates — scratch Packet with pbit, splice via `pdata`.
        let mut bit_count: usize = 0;
        for tmpl in &self.templates {
            bit_count += 1;
            if tmpl.is_some() {
                bit_count += 26;
            }
        }
        let bit_bytes = (bit_count + 7) >> 3;
        let mut bits_pkt = Packet::new(bit_bytes);
        let mut writer = rs_io::packet::BitWriter::new();
        for tmpl in &self.templates {
            match tmpl {
                Some(encoded) => {
                    writer.pbit::<1>(&mut bits_pkt, 1);
                    writer.pbit::<26>(&mut bits_pkt, *encoded as i32);
                }
                None => {
                    writer.pbit::<1>(&mut bits_pkt, 0);
                }
            }
        }
        writer.finish(&mut bits_pkt);
        buf.pdata(&bits_pkt.data, 0, bit_bytes);

        // XTEA keys
        for keys in &self.xtea_keys {
            for &k in keys {
                buf.p4(k);
            }
        }
    }

    fn sizeof(&self) -> usize {
        // 7-byte header + bit-packed templates (1 + 26 bits each populated,
        // 1 bit each empty) + 16 bytes per source mapsquare's XTEA block.
        let mut bit_count: usize = 0;
        for tmpl in &self.templates {
            bit_count += 1;
            if tmpl.is_some() {
                bit_count += 26;
            }
        }
        let template_bytes = (bit_count + 7) >> 3;
        let xtea_bytes = self.xtea_keys.len() * 16;
        7 + template_bytes + xtea_bytes
    }
}
