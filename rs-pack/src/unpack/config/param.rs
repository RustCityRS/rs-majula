use super::UnpackedPacks;
use crate::cache::param::ParamTypeRaw;
use std::collections::HashMap;

pub fn emit_param_props(
    def: &ParamTypeRaw,
    _registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if def.opcode_order.contains(&1) {
        p.push((1, "type".into(), char_to_type_name(def.var_type as u8)));
    }
    if def.opcode_order.contains(&2) {
        p.push((2, "defaultint".into(), def.default_int.to_string()));
    }
    if !def.autodisable {
        p.push((4, "autodisable".into(), "no".into()));
    }
    if let Some(ds) = &def.default_str {
        p.push((5, "defaultstring".into(), ds.to_string()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}

pub(super) fn char_to_type_name(c: u8) -> String {
    let name: Option<&'static str> = match c {
        b'i' => Some("int"),
        b's' => Some("string"),
        b'c' => Some("coord"),
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
