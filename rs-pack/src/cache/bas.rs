use super::decode::read_optional_id;
use super::provider::CacheType;
use rs_io::Packet;
use std::array::from_fn;

pub struct BasType {
    pub id: u16,
    pub readyanim: Option<u16>,
    pub walkanim: Option<u16>,
    pub anims: Vec<(u8, u16)>,
    pub rotate: (u8, u8),
    pub transform: Vec<(u8, [i16; 6])>,
    pub op28: Vec<Option<u8>>,
    pub turnspeed: u8,
    pub turnmax: u16,
    pub rollspeed: u8,
    pub rollmax: u16,
    pub roll: i16,
    pub pitchspeed: u8,
    pub pitchmax: u16,
    pub pitch: i16,
    pub walkspeed: Option<u8>,
    pub readyanim_random: Vec<(u16, u8)>,
    pub noshadow: bool,
    pub op54: (u8, u8),
    pub op55: Vec<(u8, u16)>,
    pub slotoffset: Vec<(u8, [i16; 3])>,
    pub opcode_order: Vec<u8>,
}

impl CacheType for BasType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            readyanim: None,
            walkanim: None,
            anims: Vec::new(),
            rotate: (0, 0),
            transform: Vec::new(),
            op28: Vec::new(),
            turnspeed: 0,
            turnmax: 0,
            rollspeed: 0,
            rollmax: 0,
            roll: 0,
            pitchspeed: 0,
            pitchmax: 0,
            pitch: 0,
            walkspeed: None,
            readyanim_random: Vec::new(),
            noshadow: false,
            op54: (0, 0),
            op55: Vec::new(),
            slotoffset: Vec::new(),
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
                1 => {
                    self.readyanim = read_optional_id(buf);
                    self.walkanim = read_optional_id(buf);
                }
                2..=9 | 38..=51 => self.anims.push((code, buf.g2())),
                26 => self.rotate = (buf.g1(), buf.g1()),
                27 => {
                    let slot = buf.g1();
                    self.transform.push((slot, from_fn(|_| buf.g2s())));
                }
                28 => {
                    self.op28 = (0..buf.g1())
                        .map(|_| {
                            let value = buf.g1();
                            (value != u8::MAX).then_some(value)
                        })
                        .collect();
                }
                29 => self.turnspeed = buf.g1(),
                30 => self.turnmax = buf.g2(),
                31 => self.rollspeed = buf.g1(),
                32 => self.rollmax = buf.g2(),
                33 => self.roll = buf.g2s(),
                34 => self.pitchspeed = buf.g1(),
                35 => self.pitchmax = buf.g2(),
                36 => self.pitch = buf.g2s(),
                37 => self.walkspeed = Some(buf.g1()),
                52 => {
                    self.readyanim_random = (0..buf.g1()).map(|_| (buf.g2(), buf.g1())).collect();
                }
                53 => self.noshadow = true,
                54 => self.op54 = (buf.g1(), buf.g1()),
                55 => self.op55.push((buf.g1(), buf.g2())),
                56 => {
                    let slot = buf.g1();
                    self.slotoffset.push((slot, from_fn(|_| buf.g2s())));
                }
                _ => crate::unknown_opcode!("bas", code),
            }
        }
    }
}
