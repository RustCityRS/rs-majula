use super::decode::read_optional_id;
use super::param::ParamType;
use super::provider::CacheType;
use crate::ParamValue;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub struct ShowVar {
    pub varbit: Option<u16>,
    pub varp: Option<u16>,
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
    pub sprite: Option<u16>,
    pub mouseovergraphic: Option<u16>,
    pub text: String,
    pub textcolour: i32,
    pub textmouseovercolour: Option<i32>,
    pub textsize: u8,
    pub show: u8,
    pub mapfunction: u8,
    pub showvar1: Option<ShowVar>,
    pub ops: [Option<String>; 5],
    pub polygon: Option<Polygon>,
    pub notlistable: bool,
    pub opbase: String,
    pub worldmaparrow: Option<u16>,
    pub category: Option<u16>,
    pub showvar2: Option<ShowVar>,
    pub textbackgroundoutline: i32,
    pub textbackgroundfill: Vec<i32>,
    pub polygonoutlinedash: Option<(u8, u8, u8)>,
    pub textoffset: (i16, i16),
    pub params: Vec<(i32, ParamValue)>,
}

fn read_show_var(buf: &mut Packet) -> ShowVar {
    ShowVar {
        varbit: read_optional_id(buf),
        varp: read_optional_id(buf),
        min: buf.g4s(),
        max: buf.g4s(),
    }
}

impl CacheType for MelType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            sprite: None,
            mouseovergraphic: None,
            text: String::new(),
            textcolour: 0,
            textmouseovercolour: None,
            textsize: 0,
            show: 1,
            mapfunction: 1,
            showvar1: None,
            ops: Default::default(),
            polygon: None,
            notlistable: false,
            opbase: String::new(),
            worldmaparrow: None,
            category: None,
            showvar2: None,
            textbackgroundoutline: 0,
            textbackgroundfill: Vec::new(),
            polygonoutlinedash: None,
            textoffset: (0, 0),
            params: Vec::new(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.sprite = Some(buf.g2()),
            2 => self.mouseovergraphic = Some(buf.g2()),
            3 => self.text = buf.gjstr(STRING_TERMINATOR),
            4 => self.textcolour = buf.g3(),
            5 => self.textmouseovercolour = Some(buf.g3()),
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
            18 => self.worldmaparrow = Some(buf.g2()),
            19 => self.category = Some(buf.g2()),
            20 => self.showvar2 = Some(read_show_var(buf)),
            21 => self.textbackgroundoutline = buf.g4s(),
            22 => self.textbackgroundfill.push(buf.g4s()),
            23 => self.polygonoutlinedash = Some((buf.g1(), buf.g1(), buf.g1())),
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
