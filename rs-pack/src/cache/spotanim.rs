use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;

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
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
    debugname: Option<Box<str>>,
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
            recol_s: None,
            recol_d: None,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => self.model = buf.g2(),
                2 => self.anim = Some(buf.g2()),
                3 => self.hasalpha = true,
                4 => self.resizeh = buf.g2(),
                5 => self.resizev = buf.g2(),
                6 => self.angle = buf.g2(),
                7 => self.ambient = buf.g1(),
                8 => self.contrast = buf.g1(),
                40..=49 => {
                    self.recol_s
                        .get_or_insert_with(|| vec![0; 10].into_boxed_slice())
                        [code as usize - 40] = buf.g2()
                }
                50..=59 => {
                    self.recol_d
                        .get_or_insert_with(|| vec![0; 10].into_boxed_slice())
                        [code as usize - 50] = buf.g2();
                }
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => panic!("Unrecognized spotanim config code: {code}"),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
