use super::UnpackedPacks;
use crate::cache::param::ParamTypeRaw;
use std::collections::HashMap;

pub fn emit_param_props(
    def: &ParamTypeRaw,
    opcode: u8,
    _data: &[u8],
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if opcode == 1 {
        p.push((1, "type".into(), char_to_type_name(def.var_type as u8)));
    }
    #[cfg(before_647)]
    if opcode == 2 {
        p.push((2, "defaultint".into(), def.default_int.to_string()));
    }
    #[cfg(since_647)]
    if opcode == 2 {
        let ty = char_to_type_name(def.var_type as u8);
        p.push((
            2,
            "default".into(),
            registry.typed_value(&ty, def.default_int),
        ));
    }
    if !def.autodisable {
        p.push((4, "autodisable".into(), "no".into()));
    }
    if let Some(ds) = &def.default_str {
        #[cfg(before_647)]
        p.push((5, "defaultstring".into(), ds.to_string()));
        #[cfg(since_647)]
        p.push((5, "default".into(), ds.to_string()));
    }
    #[cfg(before_647)]
    let _ = registry;
    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}

pub(super) fn char_to_type_name(c: u8) -> String {
    let name: Option<&'static str> = match c {
        b'i' => Some("int"),
        b's' => Some("string"),
        b'c' => Some("coord"),
        #[cfg(since_647)]
        b'd' => Some("graphic"),
        #[cfg(since_647)]
        0xa7 => Some("long"),
        #[cfg(since_647)]
        b'@' => Some("cursor"),
        #[cfg(since_647)]
        b'`' => Some("wma"),
        #[cfg(since_647)]
        b'M' => Some("midi"),
        #[cfg(since_647)]
        b'j' => Some("jingle"),
        #[cfg(since_647)]
        b'k' => Some("chatcat"),
        #[cfg(since_647)]
        b'e' => Some("chatphrase"),
        #[cfg(since_647)]
        0xab => Some("vorbis"),
        #[cfg(since_647)]
        b'x' => Some("texture"),
        #[cfg(since_647)]
        0x80 => Some("bas"),
        #[cfg(since_647)]
        b'a' => Some("interface"),
        #[cfg(since_647)]
        b'f' => Some("fontmetrics"),
        b'o' => Some("obj"),
        b'O' => Some("namedobj"),
        b'n' => Some("npc"),
        b'l' => Some("loc"),
        b'I' => Some("component"),
        b'1' => Some("boolean"),
        b'g' => Some("enum"),
        b'J' => Some("struct"),
        b'S' => Some("stat"),
        b'A' => Some("seq"),
        b'P' => Some("synth"),
        b'v' => Some("inv"),
        b't' => Some("spotanim"),
        b'V' => Some("varp"),
        b'm' => Some("model"),
        0xb5 => Some("mapelement"),
        b'y' => Some("category"),
        b'K' => Some("idkit"),
        b'p' => Some("player_uid"),
        b'N' => Some("npc_uid"),
        0xd0 => Some("dbrow"),
        _ => None,
    };
    match name {
        Some(n) => n.to_string(),
        None => format!("byte:0x{c:02x}"),
    }
}
