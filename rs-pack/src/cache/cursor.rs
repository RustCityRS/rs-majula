use super::provider::CacheType;
use rs_io::Packet;

pub struct CursorType {
    pub id: u16,
    pub sprite: u16,
    pub hotspot: (u8, u8),
    pub opcode_order: Vec<u8>,
}

impl CacheType for CursorType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            sprite: 0,
            hotspot: (0, 0),
            opcode_order: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code = buf.g1();
            if code == 0 {
                break;
            }
            self.opcode_order.push(code);
            match code {
                1 => self.sprite = buf.g2(),
                2 => self.hotspot = (buf.g1(), buf.g1()),
                _ => crate::unknown_opcode!("cursor", code),
            }
        }
    }
}
