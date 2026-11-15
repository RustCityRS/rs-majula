use super::decode::{read_big_id, read_id};
use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::TypeId;

pub struct SkyboxType {
    pub id: TypeId,
    pub material: Option<TypeId>,
    pub op2: bool,
    pub staticmodels: Option<[i16; 3]>,
    pub fillmode: Option<u8>,
    pub staticmodel: Option<TypeId>,
    pub op6: Option<i32>,
    pub op7: Option<[i16; 3]>,
}

impl CacheType for SkyboxType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        Self {
            id,
            material: None,
            op2: false,
            staticmodels: None,
            fillmode: None,
            staticmodel: None,
            op6: None,
            op7: None,
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.material = Some(read_id(buf)),
            2 => self.op2 = true,
            3 => self.staticmodels = Some([buf.g2s(), buf.g2s(), buf.g2s()]),
            4 => self.fillmode = Some(buf.g1()),
            5 => self.staticmodel = Some(read_big_id(buf)),
            6 => self.op6 = Some(buf.g3()),
            7 => self.op7 = Some([buf.g2s(), buf.g2s(), buf.g2s()]),
            _ => crate::unknown_opcode!("skybox", code),
        }
    }
}
