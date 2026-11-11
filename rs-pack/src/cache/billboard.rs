use super::decode::read_optional_id;
use super::provider::CacheType;
use rs_io::Packet;

pub struct BillboardType {
    pub id: u16,
    pub material: Option<u16>,
    pub width_minus_one: u16,
    pub height_minus_one: u16,
    pub unused: u8,
    pub blend_src: u8,
    pub blend_dst: u8,
}

impl CacheType for BillboardType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            material: None,
            width_minus_one: 63,
            height_minus_one: 63,
            unused: 0,
            blend_src: 2,
            blend_dst: 1,
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.material = read_optional_id(buf),
            2 => {
                self.width_minus_one = buf.g2();
                self.height_minus_one = buf.g2();
            }
            3 => self.unused = buf.g1(),
            4 => self.blend_src = buf.g1(),
            5 => self.blend_dst = buf.g1(),
            6 | 7 => {}
            _ => crate::unknown_opcode!("billboard", code),
        }
    }
}
