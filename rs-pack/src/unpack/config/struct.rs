use super::UnpackedPacks;
use crate::cache::r#struct::StructTypeRaw;
use crate::types::ParamValue;
use std::collections::HashMap;

pub fn emit_struct_props(
    def: &StructTypeRaw,
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();
    for (key, val) in def.params.as_deref().unwrap_or_default() {
        let pname = registry
            .config_name("param", *key as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("param_{key}"));
        match val {
            ParamValue::Int(i) => {
                props.push(("param".into(), format!("{pname},{i}")));
            }
            ParamValue::String(s) => {
                props.push(("param".into(), format!("{pname},{s}")));
            }
        }
    }
    props
}
