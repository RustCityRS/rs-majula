use super::decode::read_big_id;
use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::TypeId;

pub struct MsiType {
    pub id: TypeId,
    pub sprite: TypeId,
    pub tint: i32,
    pub resize: bool,
    pub nosprite: bool,
}

impl CacheType for MsiType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        Self {
            id,
            sprite: TypeId::default(),
            tint: 0,
            resize: false,
            nosprite: false,
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.sprite = read_big_id(buf),
            2 => self.tint = buf.g3(),
            3 => self.resize = true,
            4 => self.nosprite = true,
            _ => crate::unknown_opcode!("msi", code),
        }
    }
}
