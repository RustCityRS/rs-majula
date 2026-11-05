use super::decode::read_array;
#[cfg(since_647)]
use super::decode::read_pairs;
use super::provider::{CacheType, TypeProvider};
use crate::types::BodyType;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

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
    #[cfg(since_647)]
    pub retex_s: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub retex_d: Option<Box<[u16]>>,
    pub heads: Option<Box<[u16]>>,
    debugname: Option<Box<str>>,
    pub opcode_order: Vec<u8>,
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
            #[cfg(since_647)]
            retex_s: None,
            #[cfg(since_647)]
            retex_d: None,
            heads: None,
            debugname: None,
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
                1 => self.body_type = BodyType::try_from(buf.g1()).unwrap(),
                2 => self.models = Some(read_array(buf, Packet::g2)),
                3 => self.disable = true,
                #[cfg(before_647)]
                40..=49 => {
                    self.recol_s
                        .get_or_insert_with(|| vec![0; 10].into_boxed_slice())
                        [code as usize - 40] = buf.g2();
                }
                #[cfg(since_647)]
                40 | 41 => {
                    let (s, d) = if code == 40 {
                        (&mut self.recol_s, &mut self.recol_d)
                    } else {
                        (&mut self.retex_s, &mut self.retex_d)
                    };
                    read_pairs(buf, s, d);
                }
                #[cfg(before_647)]
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
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("idk", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
