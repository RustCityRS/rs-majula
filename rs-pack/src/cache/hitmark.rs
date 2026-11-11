use super::decode::read_versioned_str;
use super::provider::CacheType;
use rs_io::Packet;

pub struct HitmarkType {
    pub id: u16,
    pub damagefont: Option<u16>,
    pub damagecolour: i32,
    pub classgraphic: Option<u16>,
    pub leftgraphic: Option<u16>,
    pub middlegraphic: Option<u16>,
    pub rightgraphic: Option<u16>,
    pub scrolltooffsetx: i16,
    pub damageformat: String,
    pub sticktime: u16,
    pub scrolltooffsety: i16,
    pub replacemode: Option<u8>,
    pub damageyof: i16,
    pub fadeout: Option<u16>,
}

impl CacheType for HitmarkType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            damagefont: None,
            damagecolour: 0xffffff,
            classgraphic: None,
            leftgraphic: None,
            middlegraphic: None,
            rightgraphic: None,
            scrolltooffsetx: 0,
            damageformat: String::new(),
            sticktime: 70,
            scrolltooffsety: 0,
            replacemode: None,
            damageyof: 0,
            fadeout: None,
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.damagefont = Some(buf.g2()),
            2 => self.damagecolour = buf.g3(),
            3 => self.classgraphic = Some(buf.g2()),
            4 => self.leftgraphic = Some(buf.g2()),
            5 => self.middlegraphic = Some(buf.g2()),
            6 => self.rightgraphic = Some(buf.g2()),
            7 => self.scrolltooffsetx = buf.g2s(),
            8 => self.damageformat = read_versioned_str(buf),
            9 => self.sticktime = buf.g2(),
            10 => self.scrolltooffsety = buf.g2s(),
            11 => self.fadeout = Some(0),
            12 => self.replacemode = Some(buf.g1()),
            13 => self.damageyof = buf.g2s(),
            14 => self.fadeout = Some(buf.g2()),
            _ => crate::unknown_opcode!("hitmark", code),
        }
    }
}
