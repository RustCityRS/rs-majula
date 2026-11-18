use super::decode::{read_big_id, read_id};
use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::TypeId;
use std::array::from_fn;

pub struct BasType {
    pub id: TypeId,
    pub readyanim: Option<TypeId>,
    pub walkanim: Option<TypeId>,
    pub anims: Vec<(u8, TypeId)>,
    pub hill_rotate: (u8, u8),
    pub transform: Vec<(u8, [i16; 6])>,
    #[cfg(since_647)]
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
    pub overlayheight: Option<u16>,
    #[cfg(since_647)]
    pub random_ready_anims: Vec<(TypeId, u8)>,
    #[cfg(since_647)]
    pub noshadow: bool,
    #[cfg(since_647)]
    pub hill_rotate_limit: (u8, u8),
    #[cfg(since_647)]
    pub op55: Vec<(u8, u16)>,
    #[cfg(since_647)]
    pub slotoffset: Vec<(u8, [i16; 3])>,
}

impl CacheType for BasType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        Self {
            id,
            readyanim: None,
            walkanim: None,
            anims: Vec::new(),
            hill_rotate: (0, 0),
            transform: Vec::new(),
            #[cfg(since_647)]
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
            overlayheight: None,
            #[cfg(since_647)]
            random_ready_anims: Vec::new(),
            #[cfg(since_647)]
            noshadow: false,
            #[cfg(since_647)]
            hill_rotate_limit: (0, 0),
            #[cfg(since_647)]
            op55: Vec::new(),
            #[cfg(since_647)]
            slotoffset: Vec::new(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => {
                self.readyanim = read_big_id(buf).non_null();
                self.walkanim = read_big_id(buf).non_null();
            }
            2..=9 | 38..=42 => self.anims.push((code, read_big_id(buf))),
            43 | 44 => self.anims.push((code, read_id(buf))),
            26 => self.hill_rotate = (buf.g1(), buf.g1()),
            27 => {
                let slot = buf.g1();
                self.transform.push((slot, from_fn(|_| buf.g2s())));
            }
            #[cfg(since_647)]
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
            45 => self.overlayheight = Some(buf.g2()),
            #[cfg(since_647)]
            46..=51 => self.anims.push((code, read_big_id(buf))),
            #[cfg(since_647)]
            52 => {
                self.random_ready_anims = (0..buf.g1())
                    .map(|_| (read_big_id(buf), buf.g1()))
                    .collect();
            }
            #[cfg(since_647)]
            53 => self.noshadow = true,
            #[cfg(since_647)]
            54 => self.hill_rotate_limit = (buf.g1(), buf.g1()),
            #[cfg(since_647)]
            55 => self.op55.push((buf.g1(), buf.g2())),
            #[cfg(since_647)]
            56 => {
                let slot = buf.g1();
                self.slotoffset.push((slot, from_fn(|_| buf.g2s())));
            }
            _ => crate::unknown_opcode!("bas", code),
        }
    }
}
