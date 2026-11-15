use super::UnpackedPacks;
use crate::cache::inv::InvTypeRaw;
use crate::types::InvScope;
use std::collections::HashMap;

pub fn emit_inv_props(
    def: &InvTypeRaw,
    opcode: u8,
    _data: &[u8],
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();
    if opcode == 1 {
        let s = match def.scope {
            InvScope::Temp => "temp".to_string(),
            InvScope::Perm => "perm".to_string(),
            InvScope::Shared => "shared".to_string(),
        };
        props.push(("scope".into(), s));
    }
    if opcode == 2 {
        props.push(("size".into(), def.size.to_string()));
    }
    if opcode == 3 && def.stackall {
        props.push(("stackall".into(), "yes".into()));
    }
    let stockobj = def.stockobj.as_deref().unwrap_or_default();
    let stockcount = def.stockcount.as_deref().unwrap_or_default();
    let stockrate = def.stockrate.as_deref().unwrap_or_default();
    if opcode == 4 && !stockobj.is_empty() {
        for i in 0..stockobj.len() {
            let obj_name = if stockobj[i].is_null() {
                "null".to_string()
            } else {
                registry
                    .config_name("obj", stockobj[i].raw())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("obj_{}", stockobj[i].raw()))
            };
            let value = if stockrate[i] != 0 {
                format!("{},{},{}", obj_name, stockcount[i], stockrate[i])
            } else {
                format!("{},{}", obj_name, stockcount[i])
            };
            props.push((format!("stock{}", i + 1), value));
        }
    }
    if opcode == 5 && def.restock {
        props.push(("restock".into(), "yes".into()));
    }
    if opcode == 6 && def.allstock {
        props.push(("allstock".into(), "yes".into()));
    }
    if opcode == 7 && !def.protect {
        props.push(("protect".into(), "no".into()));
    }
    if opcode == 8 && def.runweight {
        props.push(("runweight".into(), "yes".into()));
    }
    if opcode == 9 && def.dummyinv {
        props.push(("dummyinv".into(), "yes".into()));
    }
    props
}
