use super::param::ParamType;
use super::provider::CacheType;
use crate::ParamValue;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub struct ShowVar {
    pub varbit: u16,
    pub varp: u16,
    pub min: i32,
    pub max: i32,
}

pub struct Polygon {
    pub points: Vec<(i16, i16)>,
    pub fill: i32,
    pub palette: Vec<i32>,
    pub outline: Vec<i8>,
}

pub struct MelType {
    pub id: u16,
    pub sprite: u16,
    pub mouseovergraphic: u16,
    pub text: String,
    pub textcolour: i32,
    pub textmouseovercolour: i32,
    pub textsize: u8,
    pub show: u8,
    pub mapfunction: u8,
    pub showvar1: Option<ShowVar>,
    pub ops: [Option<String>; 5],
    pub polygon: Option<Polygon>,
    pub notlistable: bool,
    pub opbase: String,
    pub worldmaparrow: u16,
    pub category: u16,
    pub showvar2: Option<ShowVar>,
    pub textbackgroundoutline: i32,
    pub textbackgroundfill: Vec<i32>,
    pub polygonoutlinedash: (u8, u8, u8),
    pub textoffset: (i16, i16),
    pub params: Vec<(i32, ParamValue)>,
    pub opcode_order: Vec<u8>,
}

fn read_show_var(buf: &mut Packet) -> ShowVar {
    ShowVar {
        varbit: buf.g2(),
        varp: buf.g2(),
        min: buf.g4s(),
        max: buf.g4s(),
    }
}

impl CacheType for MelType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            sprite: u16::MAX,
            mouseovergraphic: u16::MAX,
            text: String::new(),
            textcolour: 0,
            textmouseovercolour: 0,
            textsize: 0,
            show: 0,
            mapfunction: 0,
            showvar1: None,
            ops: Default::default(),
            polygon: None,
            notlistable: false,
            opbase: String::new(),
            worldmaparrow: u16::MAX,
            category: u16::MAX,
            showvar2: None,
            textbackgroundoutline: 0,
            textbackgroundfill: Vec::new(),
            polygonoutlinedash: (0, 0, 0),
            textoffset: (0, 0),
            params: Vec::new(),
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
                1 => self.sprite = buf.g2(),
                2 => self.mouseovergraphic = buf.g2(),
                3 => self.text = buf.gjstr(STRING_TERMINATOR),
                4 => self.textcolour = buf.g3(),
                5 => self.textmouseovercolour = buf.g3(),
                6 => self.textsize = buf.g1(),
                7 => self.show = buf.g1(),
                8 => self.mapfunction = buf.g1(),
                9 => self.showvar1 = Some(read_show_var(buf)),
                10..=14 => self.ops[code as usize - 10] = Some(buf.gjstr(STRING_TERMINATOR)),
                15 => {
                    let count = buf.g1() as usize;
                    let points = (0..count).map(|_| (buf.g2s(), buf.g2s())).collect();
                    let fill = buf.g4s();
                    let palette = (0..buf.g1()).map(|_| buf.g4s()).collect();
                    let outline = (0..count).map(|_| buf.g1s()).collect();
                    self.polygon = Some(Polygon {
                        points,
                        fill,
                        palette,
                        outline,
                    });
                }
                16 => self.notlistable = true,
                17 => self.opbase = buf.gjstr(STRING_TERMINATOR),
                18 => self.worldmaparrow = buf.g2(),
                19 => self.category = buf.g2(),
                20 => self.showvar2 = Some(read_show_var(buf)),
                21 => self.textbackgroundoutline = buf.g4s(),
                22 => self.textbackgroundfill.push(buf.g4s()),
                23 => self.polygonoutlinedash = (buf.g1(), buf.g1(), buf.g1()),
                24 => self.textoffset = (buf.g2s(), buf.g2s()),
                249 => {
                    for _ in 0..buf.g1() {
                        self.params.push(ParamType::decode_param(buf));
                    }
                }
                _ => crate::unknown_opcode!("mel", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        None
    }
}
