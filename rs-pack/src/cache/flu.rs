use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub struct FluType {
    pub id: u16,
    pub color: i32,
    pub texture: u16,
    pub size: u16,
    pub op4: bool,
    pub op5: bool,
    debugname: Option<Box<str>>,
    pub opcode_order: Vec<u8>,
}

impl CacheType for FluType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            color: 0,
            texture: u16::MAX,
            size: 128,
            op4: false,
            op5: false,
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
                1 => self.color = buf.g3(),
                2 => self.texture = buf.g2(),
                3 => self.size = buf.g2(),
                4 => self.op4 = true,
                5 => self.op5 = true,
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("flu", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
