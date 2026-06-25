use super::provider::{CacheType, TypeProvider};
use crate::types::BodyType;
use rs_io::Packet;

pub type IdkTypeProvider = TypeProvider<IdkType>;

pub struct IdkType {
    pub id: u16,
    pub body_type: BodyType,
    pub disable: bool,
}

impl From<IdkTypeRaw> for IdkType {
    fn from(raw: IdkTypeRaw) -> Self {
        IdkType {
            id: raw.id,
            body_type: raw.body_type,
            disable: raw.disable,
        }
    }
}

pub struct IdkTypeRaw {
    pub id: u16,
    pub body_type: BodyType,
    pub models: Option<Box<[u16]>>,
    pub disable: bool,
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
    pub heads: Option<Box<[u16]>>,
    debugname: Option<Box<str>>,
}

impl CacheType for IdkTypeRaw {
    type Context = ();

    fn new(id: u16) -> Self {
        IdkTypeRaw {
            id,
            body_type: BodyType::ManHair,
            models: None,
            disable: false,
            recol_s: None,
            recol_d: None,
            heads: None,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => self.body_type = BodyType::try_from(buf.g1()).unwrap(),
                2 => {
                    let count = buf.g1() as usize;
                    self.models = Some(
                        (0..count)
                            .map(|_| buf.g2())
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    );
                }
                3 => self.disable = true,
                40..=49 => {
                    self.recol_s
                        .get_or_insert_with(|| vec![0; 10].into_boxed_slice())
                        [code as usize - 40] = buf.g2();
                }
                50..=59 => {
                    self.recol_d
                        .get_or_insert_with(|| vec![0; 10].into_boxed_slice())
                        [code as usize - 50] = buf.g2();
                }
                60..=69 => {
                    self.heads
                        .get_or_insert_with(|| vec![0; 10].into_boxed_slice())
                        [code as usize - 60] = buf.g2();
                }
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => panic!("Unrecognized idk config code: {code}"),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
