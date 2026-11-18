use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::TypeId;

pub struct LightType {
    pub id: TypeId,
    pub wave_type: u8,
    pub frequency: u16,
    pub amplitude: u16,
    pub offset: i16,
}

impl CacheType for LightType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        Self {
            id,
            wave_type: 0,
            frequency: 2048,
            amplitude: 2048,
            offset: 0,
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.wave_type = buf.g1(),
            2 => self.frequency = buf.g2(),
            3 => self.amplitude = buf.g2(),
            4 => self.offset = buf.g2s(),
            _ => crate::unknown_opcode!("light", code),
        }
    }
}
