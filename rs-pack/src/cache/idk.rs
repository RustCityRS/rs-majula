#[cfg(since_530)]
use super::decode::read_pairs;
use super::decode::{read_array, read_big_id};
use super::provider::{CacheType, TypeProvider};
use crate::types::BodyType;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;

pub type IdkTypeProvider = TypeProvider<IdkType>;

pub struct IdkType {
    pub id: TypeId,
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
    pub id: TypeId,
    pub body_type: BodyType,
    pub models: Option<Box<[TypeId]>>,
    pub disable: bool,
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
    #[cfg(since_530)]
    pub retex_s: Option<Box<[u16]>>,
    #[cfg(since_530)]
    pub retex_d: Option<Box<[u16]>>,
    #[cfg(since_742)]
    pub recolindices: Option<u16>,
    #[cfg(since_742)]
    pub retexindices: Option<u16>,
    pub heads: Option<Box<[TypeId]>>,
    debugname: Box<str>,
}

impl CacheType for IdkTypeRaw {
    type Context = ();

    fn new(id: TypeId) -> Self {
        IdkTypeRaw {
            id,
            body_type: BodyType::ManHair,
            models: None,
            disable: false,
            recol_s: None,
            recol_d: None,
            #[cfg(since_530)]
            retex_s: None,
            #[cfg(since_530)]
            retex_d: None,
            #[cfg(since_742)]
            recolindices: None,
            #[cfg(since_742)]
            retexindices: None,
            heads: None,
            debugname: Box::default(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.body_type = BodyType::try_from(buf.g1()).unwrap(),
            2 => self.models = Some(read_array(buf, read_big_id)),
            3 => self.disable = true,
            #[cfg(before_530)]
            40..=49 => {
                self.recol_s
                    .get_or_insert_with(|| vec![0; 10].into_boxed_slice())[code as usize - 40] =
                    buf.g2();
            }
            #[cfg(since_530)]
            40 | 41 => {
                let (s, d) = if code == 40 {
                    (&mut self.recol_s, &mut self.recol_d)
                } else {
                    (&mut self.retex_s, &mut self.retex_d)
                };
                read_pairs(buf, s, d);
            }
            #[cfg(since_742)]
            44 => self.recolindices = Some(buf.g2()),
            #[cfg(since_742)]
            45 => self.retexindices = Some(buf.g2()),
            #[cfg(before_530)]
            50..=59 => {
                self.recol_d
                    .get_or_insert_with(|| vec![0; 10].into_boxed_slice())[code as usize - 50] =
                    buf.g2();
            }
            60..=69 => {
                self.heads
                    .get_or_insert_with(|| vec![TypeId::default(); 10].into_boxed_slice())
                    [code as usize - 60] = read_big_id(buf);
            }
            250 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
            _ => crate::unknown_opcode!("idk", code),
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}
