use super::provider::CacheType;
use rs_io::Packet;

pub struct MsiType {
    pub id: u16,
    pub sprite: u16,
    pub colour: i32,
    pub locsize: bool,
    pub nosprite: bool,
}

impl CacheType for MsiType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            sprite: 0,
            colour: 0,
            locsize: false,
            nosprite: false,
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.sprite = buf.g2(),
            2 => self.colour = buf.g3(),
            3 => self.locsize = true,
            4 => self.nosprite = true,
            _ => crate::unknown_opcode!("msi", code),
        }
    }
}
