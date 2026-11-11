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
    pub op9_u16: u16,
    #[cfg(since_647)]
    pub op10_bool34_false: bool,
    #[cfg(since_647)]
    pub op11_values: Vec<u8>,
    #[cfg(since_647)]
    pub op12_bool33_true: bool,
    #[cfg(since_647)]
    pub op13_u24: u32,
    #[cfg(since_647)]
    pub op14_u8: u8,
    #[cfg(since_647)]
    pub op16_u8: u8,
    debugname: Box<str>,
    pub opcode_order: Vec<u8>,
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
            op9_u16: 128,
            #[cfg(since_647)]
            op10_bool34_false: false,
            #[cfg(since_647)]
            op11_values: Vec::new(),
            #[cfg(since_647)]
            op12_bool33_true: false,
            #[cfg(since_647)]
            op13_u24: 1190717,
            #[cfg(since_647)]
            op14_u8: 16,
            #[cfg(since_647)]
            op16_u8: 127,
            debugname: Box::default(),
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
                9 => self.op9_u16 = buf.g2(),
                #[cfg(since_647)]
                10 => self.op10_bool34_false = true,
                #[cfg(since_647)]
                11 => self.op11_values.push(buf.g1()),
                #[cfg(since_647)]
                12 => self.op12_bool33_true = true,
                #[cfg(since_647)]
                13 => self.op13_u24 = buf.g3() as u32,
                #[cfg(since_647)]
                14 => self.op14_u8 = buf.g1(),
                #[cfg(since_647)]
                16 => self.op16_u8 = buf.g1(),
                _ => crate::unknown_opcode!("flo", code),
            }
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
