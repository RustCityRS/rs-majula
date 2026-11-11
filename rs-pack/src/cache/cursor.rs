use super::provider::CacheType;
use rs_io::Packet;

pub struct CursorType {
    pub id: u16,
    pub sprite: u16,
    pub hotspot: (u8, u8),
}

impl CacheType for CursorType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            sprite: 0,
            hotspot: (0, 0),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.sprite = buf.g2(),
            2 => self.hotspot = (buf.g1(), buf.g1()),
            _ => crate::unknown_opcode!("cursor", code),
        }
    }
}
