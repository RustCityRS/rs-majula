use super::decode::read_big_id;
use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::TypeId;

pub struct CursorType {
    pub id: TypeId,
    pub sprite: TypeId,
    pub hotspot: (u8, u8),
}

impl CacheType for CursorType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        Self {
            id,
            sprite: TypeId::default(),
            hotspot: (0, 0),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.sprite = read_big_id(buf),
            2 => self.hotspot = (buf.g1(), buf.g1()),
            _ => crate::unknown_opcode!("cursor", code),
        }
    }
}
