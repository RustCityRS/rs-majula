use super::decode::read_versioned_str;
use super::provider::CacheType;
use rs_io::Packet;

pub struct HitmarkType {
    pub id: u16,
    pub damagefont: u16,
    pub damagecolour: i32,
    pub classgraphic: u16,
    pub leftgraphic: u16,
    pub middlegraphic: u16,
    pub rightgraphic: u16,
    pub scrolltooffsetx: i16,
    pub damageformat: String,
    pub sticktime: u16,
    pub scrolltooffsety: i16,
    pub replacemode: u8,
    pub damageyof: i16,
    pub fadeout: u16,
    pub opcode_order: Vec<u8>,
}

impl CacheType for HitmarkType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            damagefont: u16::MAX,
            damagecolour: 0,
            classgraphic: u16::MAX,
            leftgraphic: u16::MAX,
            middlegraphic: u16::MAX,
            rightgraphic: u16::MAX,
            scrolltooffsetx: 0,
            damageformat: String::new(),
            sticktime: 0,
            scrolltooffsety: 0,
            replacemode: 0,
            damageyof: 0,
            fadeout: 0,
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
                1 => self.damagefont = buf.g2(),
                2 => self.damagecolour = buf.g3(),
                3 => self.classgraphic = buf.g2(),
                4 => self.leftgraphic = buf.g2(),
                5 => self.middlegraphic = buf.g2(),
                6 => self.rightgraphic = buf.g2(),
                7 => self.scrolltooffsetx = buf.g2s(),
                8 => self.damageformat = read_versioned_str(buf),
                9 => self.sticktime = buf.g2(),
                10 => self.scrolltooffsety = buf.g2s(),
                11 => self.fadeout = 0,
                12 => self.replacemode = buf.g1(),
                13 => self.damageyof = buf.g2s(),
                14 => self.fadeout = buf.g2(),
                _ => crate::unknown_opcode!("hitmark", code),
            }
        }
    }
}
