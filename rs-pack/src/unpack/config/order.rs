#[cfg(since_647)]
use super::UnpackedPacks;
#[cfg(since_647)]
use crate::cache::param::ParamType;
#[cfg(since_647)]
use crate::types::ParamValue;
#[cfg(since_647)]
use rs_io::Packet;
#[cfg(since_647)]
use rustc_hash::FxHashMap;
use std::collections::{HashMap, VecDeque};

pub fn reorder_by_opcode(
    opcode_order: &[u8],
    props: Vec<(u8, String, String)>,
) -> Vec<(String, String)> {
    if opcode_order.is_empty() {
        return props
            .into_iter()
            .map(|(_, key, value)| (key, value))
            .collect();
    }
    let mut by_op: HashMap<u8, VecDeque<(String, String)>> = HashMap::new();
    for (code, key, value) in props {
        by_op.entry(code).or_default().push_back((key, value));
    }
    let mut remaining = [0usize; 256];
    for &code in opcode_order {
        remaining[code as usize] += 1;
    }
    let mut result = Vec::new();
    for &code in opcode_order {
        let Some(queue) = by_op.get_mut(&code).filter(|queue| !queue.is_empty()) else {
            continue;
        };
        let count = &mut remaining[code as usize];
        let take = (queue.len() / *count).max(1);
        result.extend(queue.drain(..take));
        *count -= 1;
    }
    result
}

#[cfg(since_647)]
pub(super) fn emit_params(
    raw_opcodes: &[(u8, Vec<u8>)],
    params: Option<&FxHashMap<i32, ParamValue>>,
    packs: &UnpackedPacks,
) -> Vec<(u8, String, String)> {
    let mut entries = Vec::new();
    for (_, data) in raw_opcodes.iter().filter(|(code, _)| *code == 249) {
        let mut buf = Packet::from(data.clone());
        entries.extend((0..buf.g1()).map(|_| ParamType::decode_param(&mut buf)));
    }
    if !raw_opcodes.iter().any(|(code, _)| *code == 249)
        && let Some(params) = params
    {
        entries.extend(params.iter().map(|(&key, value)| (key, value.clone())));
        entries.sort_by_key(|(key, _)| *key);
    }
    entries
        .into_iter()
        .map(|(key, value)| {
            let name = packs
                .config_name("param", key as u32)
                .map(str::to_string)
                .unwrap_or_else(|| format!("param_{key}"));
            let value = match value {
                ParamValue::Int(value) => {
                    let ty = packs.param_types.get(&(key as u32));
                    packs.typed_value(ty.map_or("int", String::as_str), value)
                }
                ParamValue::String(value) => value.into_string(),
            };
            (249, "param".into(), format!("{name},{value}"))
        })
        .collect()
}
