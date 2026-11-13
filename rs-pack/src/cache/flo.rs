use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub type FloTypeProvider = TypeProvider<FloType>;

pub struct FloType {
    pub id: u16,
    pub colour: i32,
    pub texture: Option<u16>,
    pub overlay: bool,
    pub occlude: bool,
    pub debugname: Box<str>,
}

pub struct FloTypeRaw {
    pub id: u16,
    pub colour: i32,
    pub texture: Option<u16>,
    #[cfg(since_647)]
    pub texture_u8_raw: u8,
    #[cfg(since_647)]
    pub texture_u16_raw: u16,
    pub overlay: bool,
    pub occlude: bool,
    #[cfg(since_647)]
    pub water_color: Option<i32>,
    #[cfg(since_647)]
    pub op8_flomark: bool,
    #[cfg(since_647)]
    pub texturescale: u16,
    #[cfg(since_647)]
    pub hardshadow: bool,
    #[cfg(since_647)]
    pub priority: Vec<u8>,
    #[cfg(since_647)]
    pub smoothedges: bool,
    #[cfg(since_647)]
    pub waterfogcolour: u32,
    #[cfg(since_647)]
    pub waterfogscale: u8,
    #[cfg(since_647)]
    pub waterfogoffset: u8,
    debugname: Box<str>,
}

impl CacheType for FloTypeRaw {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            colour: 0,
            texture: None,
            #[cfg(since_647)]
            texture_u8_raw: 0,
            #[cfg(since_647)]
            texture_u16_raw: 0,
            overlay: false,
            occlude: true,
            #[cfg(since_647)]
            water_color: None,
            #[cfg(since_647)]
            op8_flomark: false,
            #[cfg(since_647)]
            texturescale: 128,
            #[cfg(since_647)]
            hardshadow: true,
            #[cfg(since_647)]
            priority: Vec::new(),
            #[cfg(since_647)]
            smoothedges: false,
            #[cfg(since_647)]
            waterfogcolour: 1190717,
            #[cfg(since_647)]
            waterfogscale: 16,
            #[cfg(since_647)]
            waterfogoffset: 127,
            debugname: Box::default(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.colour = buf.g3(),
            2 => {
                let v = buf.g1();
                self.texture = Some(v as u16);
                #[cfg(since_647)]
                {
                    self.texture_u8_raw = v;
                }
            }
            #[cfg(before_647)]
            3 => self.overlay = true,
            #[cfg(since_647)]
            3 => {
                let v = buf.g2();
                self.texture = (v != u16::MAX).then_some(v);
                self.texture_u16_raw = v;
            }
            5 => self.occlude = false,
            6 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
            #[cfg(since_647)]
            7 => self.water_color = Some(buf.g3()),
            #[cfg(since_647)]
            8 => self.op8_flomark = true,
            #[cfg(since_647)]
            9 => self.texturescale = buf.g2(),
            #[cfg(since_647)]
            10 => self.hardshadow = false,
            #[cfg(since_647)]
            11 => self.priority.push(buf.g1()),
            #[cfg(since_647)]
            12 => self.smoothedges = true,
            #[cfg(since_647)]
            13 => self.waterfogcolour = buf.g3() as u32,
            #[cfg(since_647)]
            14 => self.waterfogscale = buf.g1(),
            #[cfg(since_647)]
            16 => self.waterfogoffset = buf.g1(),
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
