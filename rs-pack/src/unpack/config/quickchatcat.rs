use super::UnpackedPacks;
use crate::archive::QUICKCHAT_GLOBAL_FIRST;
use crate::cache::quickchatcat::QuickChatCatType;
use std::collections::HashMap;

pub fn emit_quickchatcat_props(
    def: &QuickChatCatType,
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let first = def.id & QUICKCHAT_GLOBAL_FIRST;
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if let Some(desc) = &def.desc {
        p.push((1, "desc".into(), desc.to_string()));
    }
    for (kind, code, key, targets) in [
        ("chatcat", 2, "subcat", &def.subcats),
        ("chatphrase", 3, "phrase", &def.phrases),
    ] {
        for &(id, shortcut) in targets.iter() {
            let target = packs.reference(kind, id + first);
            p.push((code, key.into(), format!("{target},{shortcut}")));
        }
    }
    if def.opcode_order.contains(&4) {
        p.push((4, "unsearchable".into(), "yes".into()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
