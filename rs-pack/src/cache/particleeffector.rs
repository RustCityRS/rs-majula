use super::provider::CacheType;
use rs_io::Packet;

pub struct ParticleEffectorType {
    pub id: u16,
    pub angle: u16,
    pub unused: u8,
    pub force: [i32; 3],
    pub falloff_type: u8,
    pub falloff: i32,
    pub effect_type: u8,
    pub opcode_order: Vec<u8>,
}

impl CacheType for ParticleEffectorType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            angle: 0,
            unused: 0,
            force: [0; 3],
            falloff_type: 0,
            falloff: 0,
            effect_type: 0,
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
                1 => self.angle = buf.g2(),
                2 => self.unused = buf.g1(),
                3 => self.force = [buf.g4s(), buf.g4s(), buf.g4s()],
                4 => {
                    self.falloff_type = buf.g1();
                    self.falloff = buf.g4s();
                }
                6 => self.effect_type = buf.g1(),
                8..=10 => {}
                _ => crate::unknown_opcode!("particleeffector", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        None
    }
}
