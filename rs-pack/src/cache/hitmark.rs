use super::decode::{read_big_id, read_versioned_str};
use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::TypeId;

pub struct HitmarkType {
    pub id: TypeId,
    pub damagefont: Option<TypeId>,
    pub damagecolour: i32,
    pub icon_sprite: Option<TypeId>,
    pub leftgraphic: Option<TypeId>,
    pub background_sprite: Option<TypeId>,
    pub rightgraphic: Option<TypeId>,
    pub scrolltooffsetx: i16,
    pub damageformat: String,
    pub duration: u16,
    pub scrolltooffsety: i16,
    pub replacemode: Option<u8>,
    pub damageyof: i16,
    pub fadeout: Option<u16>,
}

impl CacheType for HitmarkType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        Self {
            id,
            damagefont: None,
            damagecolour: 0xffffff,
            icon_sprite: None,
            leftgraphic: None,
            background_sprite: None,
            rightgraphic: None,
            scrolltooffsetx: 0,
            damageformat: String::new(),
            duration: 70,
            scrolltooffsety: 0,
            replacemode: None,
            damageyof: 0,
            fadeout: None,
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.damagefont = Some(read_big_id(buf)),
            2 => self.damagecolour = buf.g3(),
            3 => self.icon_sprite = Some(read_big_id(buf)),
            4 => self.leftgraphic = Some(read_big_id(buf)),
            5 => self.background_sprite = Some(read_big_id(buf)),
            6 => self.rightgraphic = Some(read_big_id(buf)),
            7 => self.scrolltooffsetx = buf.g2s(),
            8 => self.damageformat = read_versioned_str(buf),
            9 => self.duration = buf.g2(),
            10 => self.scrolltooffsety = buf.g2s(),
            11 => self.fadeout = Some(0),
            12 => self.replacemode = Some(buf.g1()),
            13 => self.damageyof = buf.g2s(),
            14 => self.fadeout = Some(buf.g2()),
            _ => crate::unknown_opcode!("hitmark", code),
        }
    }
}
