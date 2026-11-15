use super::decode::read_big_id;
use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::TypeId;

pub struct HeadbarType {
    pub id: TypeId,
    pub unknown1: Option<u16>,
    pub showpriority: Option<u8>,
    pub hidepriority: Option<u8>,
    pub sticktime: Option<u16>,
    pub unknown6: Option<u8>,
    pub full: Option<TypeId>,
    pub empty: Option<TypeId>,
    pub fullplayergroup: Option<TypeId>,
    pub emptyplayergroup: Option<TypeId>,
    pub fadeout: Option<u16>,
}

impl CacheType for HeadbarType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        Self {
            id,
            unknown1: None,
            showpriority: None,
            hidepriority: None,
            sticktime: None,
            unknown6: None,
            full: None,
            empty: None,
            fullplayergroup: None,
            emptyplayergroup: None,
            fadeout: None,
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.unknown1 = Some(buf.g2()),
            2 => self.showpriority = Some(buf.g1()),
            3 => self.hidepriority = Some(buf.g1()),
            4 => self.fadeout = Some(0),
            5 => self.sticktime = Some(buf.g2()),
            6 => self.unknown6 = Some(buf.g1()),
            7 => self.full = Some(read_big_id(buf)),
            8 => self.empty = Some(read_big_id(buf)),
            9 => self.fullplayergroup = Some(read_big_id(buf)),
            10 => self.emptyplayergroup = Some(read_big_id(buf)),
            11 => self.fadeout = Some(buf.g2()),
            _ => crate::unknown_opcode!("headbar", code),
        }
    }
}
