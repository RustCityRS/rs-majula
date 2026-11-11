use super::UnpackedPacks;
use crate::cache::r#struct::StructTypeRaw;
use std::collections::HashMap;

pub fn emit_struct_props(
    _def: &StructTypeRaw,
    code: u8,
    data: &[u8],
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    if code == 249 {
        super::emit_params(data, registry)
            .into_iter()
            .map(|(_, key, value)| (key, value))
            .collect()
    } else {
        Vec::new()
    }
}
