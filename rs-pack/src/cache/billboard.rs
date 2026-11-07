use super::provider::CacheType;
use rs_io::Packet;

pub struct BillboardType {
    pub id: u16,
    pub material: u16,
    pub width_minus_one: u16,
    pub height_minus_one: u16,
    pub unused: u8,
    pub blend_src: u8,
    pub blend_dst: u8,
    pub opcode_order: Vec<u8>,
}

impl CacheType for BillboardType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            material: 0,
            width_minus_one: 0,
            height_minus_one: 0,
            unused: 0,
            blend_src: 0,
            blend_dst: 0,
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
                1 => self.material = buf.g2(),
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

    fn debugname(&self) -> Option<&str> {
        None
    }
}
