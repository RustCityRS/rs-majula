use super::UnpackedPacks;
#[cfg(since_647)]
use super::param::char_to_type_name;
#[cfg(since_647)]
use crate::cache::provider::CacheType;
#[cfg(since_647)]
use crate::cache::varbit::VarbitType;
use crate::cache::varp::VarPlayerTypeRaw;
#[cfg(since_647)]
use crate::types::VarPlayerScope;
#[cfg(since_647)]
use rs_dat2::{CacheStore, archive};
#[cfg(since_647)]
use rs_io::Packet;
use std::collections::HashMap;
#[cfg(since_647)]
use std::collections::HashSet;
#[cfg(since_647)]
use std::path::Path;

pub fn emit_varp_props(
    def: &VarPlayerTypeRaw,
    _registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    #[cfg(since_647)]
    match def.scope {
        VarPlayerScope::Perm => p.push((1, "scope".into(), "perm".into())),
        VarPlayerScope::Temp => {}
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&2) {
        p.push((2, "type".into(), char_to_type_name(def.var_type as u8)));
    }
    #[cfg(since_647)]
    if !def.protect {
        p.push((4, "protect".into(), "no".into()));
    }
    if def.opcode_order.contains(&5) || def.clientcode != 0 {
        p.push((5, "clientcode".into(), def.clientcode.to_string()));
    }
    #[cfg(since_647)]
    if def.transmit || _registry.transmit_varps.contains(&(def.id as u32)) {
        p.push((6, "transmit".into(), "yes".into()));
    }
    #[cfg(since_647)]
    let mut order = def.opcode_order.clone();
    #[cfg(since_647)]
    if !def.transmit && _registry.transmit_varps.contains(&(def.id as u32)) && !order.is_empty() {
        order.push(6);
    }
    #[cfg(before_647)]
    let order = def.opcode_order.as_slice();
    #[cfg(since_647)]
    let order = order.as_slice();
    super::reorder_by_opcode(order, p)
}

#[cfg(since_647)]
pub(super) fn transmit_varps(
    store: &mut CacheStore,
    cs2_dir: &Path,
) -> anyhow::Result<HashSet<u32>> {
    let (mut varps, varbits) = super::super::cs2::scan_cs2_varp_refs(cs2_dir)?;
    for group_id in 0..store.group_count(archive::CONFIG_VARBIT).unwrap_or(0) {
        if let Some(group) = store.read_group(archive::CONFIG_VARBIT, group_id)? {
            for entry in group.entries {
                if !entry.data.is_empty() && varbits.contains(&(group_id * 1024 + entry.id)) {
                    let mut def = VarbitType::new(0);
                    def.decode(&mut Packet::from(entry.data.to_vec()));
                    varps.insert(def.basevar as u32);
                }
            }
        }
    }
    Ok(varps)
}
