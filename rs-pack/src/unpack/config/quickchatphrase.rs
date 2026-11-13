use super::UnpackedPacks;
use crate::archive::QUICKCHAT_GLOBAL_FIRST;
use crate::cache::quickchatphrase::{
    QuickChatCommand, QuickChatDynamicCommand, QuickChatPhraseType,
};
use std::collections::HashMap;

fn command_text(command: &QuickChatCommand, packs: &UnpackedPacks) -> String {
    let dynamic = QuickChatDynamicCommand::from_id(command.id);
    let params: Vec<_> = dynamic
        .params
        .iter()
        .zip(command.params.iter())
        .map(|(kind, &id)| packs.reference(kind, id))
        .collect();
    format!("{}({})", dynamic.name, params.join(","))
}

pub fn emit_quickchatphrase_props(
    def: &QuickChatPhraseType,
    opcode: u8,
    _data: &[u8],
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
    if opcode == 4 {
        p.push((4, "unsearchable".into(), "yes".into()));
    }
    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}
