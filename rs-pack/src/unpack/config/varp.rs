#[cfg(since_530)]
use super::super::cs2::scan_cs2_varp_refs;
use super::UnpackedPacks;
#[cfg(since_530)]
use super::param::char_to_type_name;
#[cfg(since_530)]
use crate::archive;
#[cfg(since_530)]
use crate::cache::provider::CacheType;
use crate::cache::varp::VarPlayerTypeRaw;
#[cfg(since_530)]
use crate::cache::varpbit::VarpBitType;
#[cfg(since_530)]
use crate::types::VarPlayerScope;
#[cfg(since_530)]
use rs_io::Packet;
#[cfg(since_530)]
use rs_io::js5::Js5LocalCache;
#[cfg(since_530)]
use rs_ver::TypeId;
use std::collections::HashMap;
#[cfg(since_530)]
use std::collections::HashSet;

pub fn emit_varp_props(
    def: &VarPlayerTypeRaw,
    code: u8,
    _data: &[u8],
    _registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    #[cfg(since_530)]
    match def.scope {
        VarPlayerScope::Perm => p.push((1, "scope".into(), "perm".into())),
        VarPlayerScope::Temp => {}
    }
    #[cfg(since_530)]
    if code == 2 {
        p.push((2, "type".into(), char_to_type_name(def.var_type as u8)));
    }
    #[cfg(since_530)]
    if !def.protect {
        p.push((4, "protect".into(), "no".into()));
    }
    if code == 5 {
        p.push((5, "clientcode".into(), def.clientcode.to_string()));
    }
    #[cfg(since_530)]
    if def.transmit {
        p.push((6, "transmit".into(), "yes".into()));
    }
    p.into_iter()
        .filter(|(opcode, _, _)| *opcode == code)
        .map(|(_, key, value)| (key, value))
        .collect()
}

#[cfg(since_530)]
pub(super) fn transmit_varps(store: &mut Js5LocalCache) -> anyhow::Result<HashSet<u32>> {
    let (mut varps, varpbits) = scan_cs2_varp_refs(store)?;
    for group_id in 0..store.group_id_limit(archive::CONFIG_VARPBIT).unwrap_or(0) {
        if let Some(group) = store.read_group(archive::CONFIG_VARPBIT, group_id)? {
            for entry in group.entries {
                if !entry.data().is_empty() && varpbits.contains(&(group_id * 1024 + entry.id)) {
                    let mut def = VarpBitType::new(TypeId::new(0));
                    def.decode(&mut Packet::from(entry.data().to_vec()));
                    varps.insert(def.basevar as u32);
                }
            }
        }
    }
    Ok(varps)
}
