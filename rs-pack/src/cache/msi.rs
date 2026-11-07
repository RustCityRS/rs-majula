use super::provider::CacheType;
use rs_io::Packet;

pub struct MsiType {
    pub id: u16,
    pub sprite: u16,
    pub colour: i32,
    pub locsize: bool,
    pub nosprite: bool,
    pub opcode_order: Vec<u8>,
}

impl CacheType for MsiType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            sprite: u16::MAX,
            colour: 0,
            locsize: false,
            nosprite: false,
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
                2 => self.colour = buf.g3(),
                3 => self.locsize = true,
                4 => self.nosprite = true,
                _ => crate::unknown_opcode!("msi", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        None
    }
}
