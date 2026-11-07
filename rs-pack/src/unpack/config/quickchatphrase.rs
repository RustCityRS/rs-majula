use super::UnpackedPacks;
use crate::archive::QUICKCHAT_GLOBAL_FIRST;
use crate::cache::quickchatphrase::{QuickChatCommand, QuickChatPhraseType};
use std::collections::HashMap;

fn command_text(command: &QuickChatCommand, packs: &UnpackedPacks) -> String {
    let param = |index: usize, kind: &str| packs.reference(kind, command.params[index]);
    match command.id {
        0 => format!("listdialog({})", param(0, "enum")),
        1 => "objdialog()".to_owned(),
        2 => "countdialog()".to_owned(),
        4 => format!("stat_base({})", param(0, "stat")),
        6 => format!("enum_string({},{})", param(0, "enum"), param(1, "varp")),
        7 => format!("enum_string_clan({})", param(0, "enum")),
        8 => format!("tostring_varp({})", param(0, "varp")),
        9 => format!("tostring_varbit({})", param(0, "varbit")),
        10 => "objtradedialog()".to_owned(),
        11 => format!(
            "enum_string_statbase({},{})",
            param(0, "enum"),
            param(1, "stat")
        ),
        12 => "acc_getcount_world()".to_owned(),
        13 => "acc_getmeancombatlevel()".to_owned(),
        14 => format!("tostring_shared({})", param(0, "vars")),
        15 => "activecombatlevel()".to_owned(),
        id => panic!("Unknown quickchat command {id}"),
    }
}

pub fn emit_quickchatphrase_props(
    def: &QuickChatPhraseType,
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let first = def.id & QUICKCHAT_GLOBAL_FIRST;
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if let Some(text) = &def.text {
        // The commands fill the text's `<`s, in order.
        let mut commands = def.commands.iter();
        let mut phrase = String::new();
        for (index, part) in text.split('<').enumerate() {
            if index > 0 {
                match commands.next() {
                    Some(command) => {
                        phrase.push('<');
                        phrase.push_str(&command_text(command, packs));
                        phrase.push('>');
                    }
                    None => phrase.push('<'),
                }
            }
            phrase.push_str(part);
        }
        p.push((1, "phrase".into(), phrase));
    }
    for &id in def.responses.iter() {
        p.push((
            2,
            "autoresponse".into(),
            packs.reference("chatphrase", id + first),
        ));
    }
    if def.opcode_order.contains(&4) {
        p.push((4, "unsearchable".into(), "yes".into()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
