use super::provider::CacheType;
use rs_io::Packet;

pub struct LightType {
    pub id: u16,
    pub function: u8,
    pub speed: u16,
    pub amplitude: u16,
    pub base: i16,
    pub opcode_order: Vec<u8>,
}

impl CacheType for LightType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            function: 0,
            speed: 2048,
            amplitude: 2048,
            base: 0,
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
                1 => self.function = buf.g1(),
                2 => self.speed = buf.g2(),
                3 => self.amplitude = buf.g2(),
                4 => self.base = buf.g2s(),
                _ => crate::unknown_opcode!("light", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        None
    }
}
