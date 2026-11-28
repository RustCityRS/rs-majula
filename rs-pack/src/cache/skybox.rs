use super::decode::{read_big_id, read_id};
use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::TypeId;

pub struct SkyboxType {
    pub id: TypeId,
    pub material: Option<TypeId>,
    pub op2: Option<Vec<u16>>,
    pub op3: Option<u8>,
    pub fillmode: Option<u8>,
    pub staticmodel: Option<TypeId>,
}

impl CacheType for SkyboxType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        Self {
            id,
            material: None,
            op2: None,
            op3: None,
            fillmode: None,
            staticmodel: None,
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.material = Some(read_id(buf)),
            2 => {
                let count = buf.g1();
                self.op2 = Some((0..count).map(|_| buf.g2()).collect());
            }
            3 => self.op3 = Some(buf.g1()),
            4 => self.fillmode = Some(buf.g1()),
            5 => self.staticmodel = Some(read_big_id(buf)),
            _ => crate::unknown_opcode!("skybox", code),
        }
    }
}
