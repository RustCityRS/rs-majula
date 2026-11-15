use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;

pub type FloTypeProvider = TypeProvider<FloType>;

pub struct FloType {
    pub id: TypeId,
    pub colour: i32,
    pub texture: Option<u16>,
    pub overlay: bool,
    pub occlude: bool,
    pub debugname: Box<str>,
}

pub struct FloTypeRaw {
    pub id: TypeId,
    pub colour: i32,
    pub texture: Option<u16>,
    #[cfg(since_530)]
    pub texture_u8_raw: u8,
    #[cfg(since_530)]
    pub texture_u16_raw: u16,
    pub overlay: bool,
    pub occlude: bool,
    #[cfg(since_530)]
    pub water_color: Option<i32>,
    #[cfg(since_530)]
    pub op8_flomark: bool,
    #[cfg(since_530)]
    pub texturescale: u16,
    #[cfg(since_530)]
    pub hardshadow: bool,
    #[cfg(since_530)]
    pub priority: Vec<u8>,
    #[cfg(since_530)]
    pub smoothedges: bool,
    #[cfg(since_530)]
    pub waterfogcolour: u32,
    #[cfg(since_530)]
    pub waterfogscale: u8,
    #[cfg(since_647)]
    pub waterfogoffset: u8,
    #[cfg(since_742)]
    pub waterfogunknowna: Option<u16>,
    #[cfg(since_742)]
    pub waterfogunknownb: Option<u8>,
    #[cfg(since_742)]
    pub waterfogunknownc: Option<u16>,
    debugname: Box<str>,
}

impl CacheType for FloTypeRaw {
    type Context = ();

    fn new(id: TypeId) -> Self {
        Self {
            id,
            colour: 0,
            texture: None,
            #[cfg(since_530)]
            texture_u8_raw: 0,
            #[cfg(since_530)]
            texture_u16_raw: 0,
            overlay: false,
            occlude: true,
            #[cfg(since_530)]
            water_color: None,
            #[cfg(since_530)]
            op8_flomark: false,
            #[cfg(since_530)]
            texturescale: 128,
            #[cfg(since_530)]
            hardshadow: true,
            #[cfg(since_530)]
            priority: Vec::new(),
            #[cfg(since_530)]
            smoothedges: false,
            #[cfg(since_530)]
            waterfogcolour: 1190717,
            #[cfg(since_530)]
            waterfogscale: 16,
            #[cfg(since_647)]
            waterfogoffset: 127,
            #[cfg(since_742)]
            waterfogunknowna: None,
            #[cfg(since_742)]
            waterfogunknownb: None,
            #[cfg(since_742)]
            waterfogunknownc: None,
            debugname: Box::default(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.colour = buf.g3(),
            2 => {
                let v = buf.g1();
                self.texture = Some(v as u16);
                #[cfg(since_530)]
                {
                    self.texture_u8_raw = v;
                }
            }
            #[cfg(before_530)]
            3 => self.overlay = true,
            #[cfg(since_530)]
            3 => {
                let v = buf.g2();
                self.texture = (v != u16::MAX).then_some(v);
                self.texture_u16_raw = v;
            }
            5 => self.occlude = false,
            6 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
            #[cfg(since_530)]
            7 => self.water_color = Some(buf.g3()),
            #[cfg(since_530)]
            8 => self.op8_flomark = true,
            #[cfg(since_530)]
            9 => self.texturescale = buf.g2(),
            #[cfg(since_530)]
            10 => self.hardshadow = false,
            #[cfg(since_530)]
            11 => self.priority.push(buf.g1()),
            #[cfg(since_530)]
            12 => self.smoothedges = true,
            #[cfg(since_530)]
            13 => self.waterfogcolour = buf.g3() as u32,
            #[cfg(since_530)]
            14 => self.waterfogscale = buf.g1(),
            #[cfg(since_647)]
            16 => self.waterfogoffset = buf.g1(),
            #[cfg(since_742)]
            20 => self.waterfogunknowna = Some(buf.g2()),
            #[cfg(since_742)]
            21 => self.waterfogunknownb = Some(buf.g1()),
            #[cfg(since_742)]
            22 => self.waterfogunknownc = Some(buf.g2()),
            _ => crate::unknown_opcode!("flo", code),
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}

impl From<FloTypeRaw> for FloType {
    fn from(f: FloTypeRaw) -> Self {
        FloType {
            id: f.id,
            colour: f.colour,
            texture: f.texture,
            overlay: f.overlay,
            occlude: f.occlude,
            debugname: f.debugname,
        }
    }
}
