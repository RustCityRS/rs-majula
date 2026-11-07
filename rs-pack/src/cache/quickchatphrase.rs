use super::decode::read_array;
use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub struct QuickChatCommand {
    pub id: u16,
    pub params: Box<[u16]>,
}

impl QuickChatCommand {
    fn param_count(id: u16) -> usize {
        match id {
            0 | 4 | 7..=9 | 14 => 1,
            6 | 11 => 2,
            1 | 2 | 10 | 12 | 13 | 15 => 0,
            _ => panic!("Unknown quickchat command {id}"),
        }
    }
}

pub struct QuickChatPhraseType {
    pub id: u16,
    pub text: Option<Box<str>>,
    pub responses: Box<[u16]>,
    pub commands: Box<[QuickChatCommand]>,
    pub opcode_order: Vec<u8>,
}

impl CacheType for QuickChatPhraseType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            text: None,
            responses: Box::default(),
            commands: Box::default(),
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
                1 => self.text = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                2 => self.responses = read_array(buf, Packet::g2),
                3 => {
                    self.commands = read_array(buf, |buf| {
                        let id = buf.g2();
                        let params = (0..QuickChatCommand::param_count(id))
                            .map(|_| buf.g2())
                            .collect();
                        QuickChatCommand { id, params }
                    });
                }
                4 => {}
                _ => crate::unknown_opcode!("quickchatphrase", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        None
    }
}
