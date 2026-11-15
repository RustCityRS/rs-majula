use super::decode::read_big_id;
use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::TypeId;

pub struct MsiType {
    pub id: TypeId,
    pub sprite: TypeId,
    pub colour: i32,
    pub locsize: bool,
    pub nosprite: bool,
}

impl CacheType for MsiType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        Self {
            id,
            sprite: TypeId::default(),
            colour: 0,
            locsize: false,
            nosprite: false,
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.sprite = read_big_id(buf),
            2 => self.colour = buf.g3(),
            3 => self.locsize = true,
            4 => self.nosprite = true,
            _ => crate::unknown_opcode!("msi", code),
        }
    }
}
