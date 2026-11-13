use super::decode::read_optional_id;
use super::provider::CacheType;
use rs_io::Packet;

pub struct FluType {
    pub id: u16,
    pub color: i32,
    pub texture: Option<u16>,
    pub size: u16,
    pub op4: bool,
    pub op5: bool,
}

impl CacheType for FluType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            color: 0,
            texture: None,
            size: 128,
            op4: false,
            op5: false,
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.color = buf.g3(),
            2 => self.texture = read_optional_id(buf),
            3 => self.size = buf.g2(),
            4 => self.op4 = true,
            5 => self.op5 = true,
            _ => crate::unknown_opcode!("flu", code),
        }
    }
}
