#[cfg(since_647)]
use super::decode::read_pairs;
use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub type SpotAnimTypeProvider = TypeProvider<SpotAnimType>;

pub struct SpotAnimType {
    pub id: u16,
}

impl From<SpotAnimTypeRaw> for SpotAnimType {
    fn from(raw: SpotAnimTypeRaw) -> Self {
        SpotAnimType { id: raw.id }
    }
}

pub struct SpotAnimTypeRaw {
    pub id: u16,
    pub model: u16,
    pub anim: Option<u16>,
    pub hasalpha: bool,
    pub resizeh: u16,
    pub resizev: u16,
    pub angle: u16,
    pub ambient: u8,
    pub contrast: u8,
    #[cfg(since_647)]
    pub hillskew: u8,
    #[cfg(since_647)]
    pub hillskew_param: Option<i32>,
    #[cfg(since_647)]
    pub allowloop: bool,
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub retex_d: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub retex_s: Option<Box<[u16]>>,
    debugname: Box<str>,
}

impl CacheType for SpotAnimTypeRaw {
    type Context = ();

    fn new(id: u16) -> Self {
        SpotAnimTypeRaw {
            id,
            model: 0,
            anim: None,
            hasalpha: false,
            resizeh: 128,
            resizev: 128,
            angle: 0,
            ambient: 0,
            contrast: 0,
            #[cfg(since_647)]
            hillskew: 0,
            #[cfg(since_647)]
            hillskew_param: None,
            #[cfg(since_647)]
            allowloop: false,
            recol_s: None,
            recol_d: None,
            #[cfg(since_647)]
            retex_d: None,
            #[cfg(since_647)]
            retex_s: None,
            debugname: Box::default(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.model = buf.g2(),
            2 => self.anim = Some(buf.g2()),
            #[cfg(before_647)]
            3 => self.hasalpha = true,
            4 => self.resizeh = buf.g2(),
            5 => self.resizev = buf.g2(),
            6 => self.angle = buf.g2(),
            7 => self.ambient = buf.g1(),
            8 => self.contrast = buf.g1(),
            #[cfg(since_647)]
            9 => {
                self.hillskew = 3;
                self.hillskew_param = Some(8224);
            }
            #[cfg(since_647)]
            10 => self.allowloop = true,
            #[cfg(since_647)]
            11 => self.hillskew = 1,
            #[cfg(since_647)]
            12 => self.hillskew = 4,
            #[cfg(since_647)]
            13 => self.hillskew = 5,
            #[cfg(since_647)]
            14 => {
                self.hillskew = 2;
                self.hillskew_param = Some((buf.g1() as i32) << 8);
            }
            #[cfg(since_647)]
            15 => {
                self.hillskew = 3;
                self.hillskew_param = Some(buf.g2() as i32);
            }
            #[cfg(since_647)]
            16 => {
                self.hillskew = 3;
                let param = buf.g4s();
                self.hillskew_param = (param != -1).then_some(param);
            }
            #[cfg(since_647)]
            40 | 41 => {
                let (s, d) = if code == 40 {
                    (&mut self.recol_s, &mut self.recol_d)
                } else {
                    (&mut self.retex_s, &mut self.retex_d)
                };
                read_pairs(buf, d, s);
            }
            #[cfg(before_647)]
            40..=49 => {
                self.recol_s
                    .get_or_insert_with(|| vec![0; 10].into_boxed_slice())[code as usize - 40] =
                    buf.g2()
            }
            #[cfg(before_647)]
            50..=59 => {
                self.recol_d
                    .get_or_insert_with(|| vec![0; 10].into_boxed_slice())[code as usize - 50] =
                    buf.g2();
            }
            250 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
            _ => crate::unknown_opcode!("spotanim", code),
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}
