use super::decode::read_array;
use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub struct QuickChatCatType {
    pub id: u16,
    pub desc: Option<Box<str>>,
    pub subcats: Box<[(u16, u8)]>,
    pub phrases: Box<[(u16, u8)]>,
    pub opcode_order: Vec<u8>,
}

impl CacheType for QuickChatCatType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            desc: None,
            subcats: Box::default(),
            phrases: Box::default(),
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
                1 => self.desc = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                2 => self.subcats = read_array(buf, |buf| (buf.g2(), buf.g1())),
                3 => self.phrases = read_array(buf, |buf| (buf.g2(), buf.g1())),
                4 => {}
                _ => crate::unknown_opcode!("quickchatcat", code),
            }
        }
    }
}
