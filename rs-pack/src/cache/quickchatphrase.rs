use super::decode::read_array;
use super::provider::CacheType;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

/// A value the sender of a phrase fills in, as the client's
/// `QuickChatDynamicCommand` lists them: its id, its name in a phrase's text,
/// and the config table of each parameter a phrase gives it.
pub struct QuickChatDynamicCommand {
    pub id: u16,
    pub name: &'static str,
    pub params: &'static [&'static str],
}

pub const DYNAMIC_COMMANDS: &[QuickChatDynamicCommand] = &[
    QuickChatDynamicCommand::new(0, "listdialog", &["enum"]),
    QuickChatDynamicCommand::new(1, "objdialog", &[]),
    QuickChatDynamicCommand::new(2, "countdialog", &[]),
    QuickChatDynamicCommand::new(4, "stat_base", &["stat"]),
    QuickChatDynamicCommand::new(6, "enum_string", &["enum", "varp"]),
    QuickChatDynamicCommand::new(7, "enum_string_clan", &["enum"]),
    QuickChatDynamicCommand::new(8, "tostring_varp", &["varp"]),
    QuickChatDynamicCommand::new(9, "tostring_varbit", &["varpbit"]),
    QuickChatDynamicCommand::new(10, "objtradedialog", &[]),
    QuickChatDynamicCommand::new(11, "enum_string_statbase", &["enum", "stat"]),
    QuickChatDynamicCommand::new(12, "acc_getcount_world", &[]),
    QuickChatDynamicCommand::new(13, "acc_getmeancombatlevel", &[]),
    QuickChatDynamicCommand::new(14, "tostring_shared", &["vars"]),
    QuickChatDynamicCommand::new(15, "activecombatlevel", &[]),
];

impl QuickChatDynamicCommand {
    const fn new(id: u16, name: &'static str, params: &'static [&'static str]) -> Self {
        Self { id, name, params }
    }

    pub fn from_id(id: u16) -> &'static Self {
        DYNAMIC_COMMANDS
            .iter()
            .find(|command| command.id == id)
            .unwrap_or_else(|| panic!("Unknown quickchat command {id}"))
    }

    pub fn from_name(name: &str) -> Option<&'static Self> {
        DYNAMIC_COMMANDS.iter().find(|command| command.name == name)
    }
}

pub struct QuickChatCommand {
    pub id: u16,
    pub params: Box<[u16]>,
}

pub struct QuickChatPhraseType {
    pub id: u16,
    pub text: Option<Box<str>>,
    pub responses: Box<[u16]>,
    pub commands: Box<[QuickChatCommand]>,
}

impl CacheType for QuickChatPhraseType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            text: None,
            responses: Box::default(),
            commands: Box::default(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.text = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
            2 => self.responses = read_array(buf, Packet::g2),
            3 => {
                self.commands = read_array(buf, |buf| {
                    let id = buf.g2();
                    let params = QuickChatDynamicCommand::from_id(id)
                        .params
                        .iter()
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
