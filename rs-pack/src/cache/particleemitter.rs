use super::provider::CacheType;
use rs_io::Packet;

pub struct ParticleEmitterType {
    pub id: u16,
    pub angles: [u16; 4],
    pub unused2: u8,
    pub speed: [i32; 2],
    pub speed_change_type: u8,
    pub speed_change: i8,
    pub size: u16,
    pub colour: [u32; 2],
    pub lifetime: [u16; 2],
    pub rate: [u16; 2],
    pub effectors: Vec<u16>,
    pub effectors10: Vec<u16>,
    pub priority12: i8,
    pub priority13: i8,
    pub max_particles: u16,
    pub material: u16,
    pub cycle_start: u8,
    pub cycle_offset: u16,
    pub cycle_duration: u16,
    pub cycle_repeat: u8,
    pub material17: u16,
    pub target_colour: u32,
    pub colour_percent: u8,
    pub alpha_percent: u8,
    pub speed_percent: u8,
    pub target_speed: i32,
    pub size_percent: u8,
    pub effectors25: Vec<u16>,
    pub target_size: u16,
    pub target_size_percent: u8,
    pub unused29: i16,
    pub size_range: [u16; 2],
    pub opcode_order: Vec<u8>,
}

fn effector_list(buf: &mut Packet) -> Vec<u16> {
    let count = buf.g1();
    (0..count).map(|_| buf.g2()).collect()
}

impl CacheType for ParticleEmitterType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            angles: [0; 4],
            unused2: 0,
            speed: [0; 2],
            speed_change_type: 0,
            speed_change: 0,
            size: 0,
            colour: [0; 2],
            lifetime: [0; 2],
            rate: [0; 2],
            effectors: Vec::new(),
            effectors10: Vec::new(),
            priority12: -2,
            priority13: -2,
            max_particles: 0,
            material: u16::MAX,
            cycle_start: 1,
            cycle_offset: u16::MAX,
            cycle_duration: u16::MAX,
            cycle_repeat: 1,
            material17: u16::MAX,
            target_colour: 0,
            colour_percent: 0,
            alpha_percent: 100,
            speed_percent: 100,
            target_speed: -1,
            size_percent: 100,
            effectors25: Vec::new(),
            target_size: 0,
            target_size_percent: 100,
            unused29: 0,
            size_range: [0; 2],
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
                1 => self.angles = [buf.g2(), buf.g2(), buf.g2(), buf.g2()],
                2 => self.unused2 = buf.g1(),
                3 => self.speed = [buf.g4s(), buf.g4s()],
                4 => {
                    self.speed_change_type = buf.g1();
                    self.speed_change = buf.g1s();
                }
                5 => self.size = buf.g2(),
                6 => self.colour = [buf.g4s() as u32, buf.g4s() as u32],
                7 => self.lifetime = [buf.g2(), buf.g2()],
                8 => self.rate = [buf.g2(), buf.g2()],
                9 => self.effectors = effector_list(buf),
                10 => self.effectors10 = effector_list(buf),
                12 => self.priority12 = buf.g1s(),
                13 => self.priority13 = buf.g1s(),
                14 => self.max_particles = buf.g2(),
                15 => self.material = buf.g2(),
                16 => {
                    self.cycle_start = buf.g1();
                    self.cycle_offset = buf.g2();
                    self.cycle_duration = buf.g2();
                    self.cycle_repeat = buf.g1();
                }
                17 => self.material17 = buf.g2(),
                18 => self.target_colour = buf.g4s() as u32,
                19 => self.colour_percent = buf.g1(),
                20 => self.alpha_percent = buf.g1(),
                21 => self.speed_percent = buf.g1(),
                22 => self.target_speed = buf.g4s(),
                23 => self.size_percent = buf.g1(),
                25 => self.effectors25 = effector_list(buf),
                27 => self.target_size = buf.g2(),
                28 => self.target_size_percent = buf.g1(),
                29 => self.unused29 = buf.g2s(),
                31 => self.size_range = [buf.g2(), buf.g2()],
                24 | 26 | 30 | 32..=34 => {}
                _ => crate::unknown_opcode!("particleemitter", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        None
    }
}
