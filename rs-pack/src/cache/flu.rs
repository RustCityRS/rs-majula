use super::decode::read_optional_id;
use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::TypeId;

pub struct FluType {
    pub id: TypeId,
    pub color: i32,
    pub material: Option<TypeId>,
    pub texture_scale: u16,
    pub op4: bool,
    #[cfg(since_647)]
    pub op5: bool,
}

impl CacheType for FluType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        Self {
            id,
            color: 0,
            material: None,
            texture_scale: 128,
            op4: false,
            #[cfg(since_647)]
            op5: false,
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.color = buf.g3(),
            2 => self.material = read_optional_id(buf),
            3 => self.texture_scale = buf.g2(),
            4 => self.op4 = true,
            #[cfg(since_647)]
            5 => self.op5 = true,
            _ => crate::unknown_opcode!("flu", code),
        }
    }
}
