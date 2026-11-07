use super::UnpackedPacks;
use super::param::char_to_type_name;
use crate::cache::r#enum::EnumTypeRaw;
use crate::types::{ParamValue, ScriptVarType};
use std::collections::HashMap;

pub fn emit_enum_props(
    def: &EnumTypeRaw,
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();

    // Emit opcodes in the original decode order. This is critical because
    // the Jagex cache sometimes has entries (op 5/6) before defaults (op 3/4),
    // and the packer must reproduce the exact same byte order.
    let mut entries_emitted = false;
    for &op in &def.opcode_order {
        match op {
            1 => {
                props.push(("inputtype".into(), char_to_type_name(def.inputtype as u8)));
            }
            2 => {
                props.push(("outputtype".into(), char_to_type_name(def.outputtype as u8)));
            }
            3 => {
                // Client defaults to "null", so any op 3 present in the binary
                // is meaningful — even if the string is empty.
                #[cfg(before_647)]
                let key = "defaultstring";
                #[cfg(since_647)]
                let key = "default";
                props.push((
                    key.into(),
                    def.default_str.as_deref().unwrap_or("").to_string(),
                ));
            }
            4 => {
                #[cfg(before_647)]
                props.push(("defaultint".into(), def.default_int.to_string()));
                #[cfg(since_647)]
                props.push((
                    "default".into(),
                    registry.typed_value(&char_to_type_name(def.outputtype as u8), def.default_int),
                ));
            }
            5 | 6 if !entries_emitted => {
                entries_emitted = true;
                // Entries — emit in decode order, using ordered_entries
                // which preserves duplicates with their original values.
                #[cfg(since_647)]
                let (input, output) = (
                    char_to_type_name(def.inputtype as u8),
                    char_to_type_name(def.outputtype as u8),
                );
                for (key, value) in &def.ordered_entries {
                    #[cfg(since_647)]
                    let key = registry.typed_value(&input, *key);
                    match value {
                        ParamValue::String(s) => {
                            props.push(("val".into(), format!("{key},{s}")));
                        }
                        #[cfg(before_647)]
                        ParamValue::Int(v) => {
                            props.push(("val".into(), format!("{key},{v}")));
                        }
                        #[cfg(since_647)]
                        ParamValue::Int(v) => {
                            let v = registry.typed_value(&output, *v);
                            props.push(("val".into(), format!("{key},{v}")));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    #[cfg(before_647)]
    let _ = registry;

    // Fallback: if opcode_order is empty (e.g. hand-written configs), use
    // the legacy fixed order.
    if def.opcode_order.is_empty() {
        if def.inputtype != ScriptVarType::Int {
            props.push(("inputtype".into(), char_to_type_name(def.inputtype as u8)));
        }
        if def.outputtype != ScriptVarType::Int {
            props.push(("outputtype".into(), char_to_type_name(def.outputtype as u8)));
        }
        if def.default_str.as_deref().is_some_and(|s| !s.is_empty()) {
            props.push((
                "defaultstring".into(),
                def.default_str.as_deref().unwrap_or("").to_string(),
            ));
        }
        if def.default_int != 0 {
            props.push(("defaultint".into(), def.default_int.to_string()));
        }
        for (key, value) in &def.ordered_entries {
            match value {
                ParamValue::String(s) => {
                    props.push(("val".into(), format!("{key},{s}")));
                }
                ParamValue::Int(v) => {
                    props.push(("val".into(), format!("{key},{v}")));
                }
            }
        }
    }
    props
}
