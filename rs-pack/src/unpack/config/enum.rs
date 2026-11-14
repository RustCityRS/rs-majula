use super::UnpackedPacks;
use super::param::char_to_type_name;
use crate::cache::r#enum::EnumTypeRaw;
use crate::types::ParamValue;
use std::collections::HashMap;

pub fn emit_enum_props(
    def: &EnumTypeRaw,
    code: u8,
    _data: &[u8],
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();

    match code {
        1 => {
            props.push(("inputtype".into(), char_to_type_name(def.inputtype as u8)));
        }
        2 => {
            props.push(("outputtype".into(), char_to_type_name(def.outputtype as u8)));
        }
        3 => {
            // Client defaults to "null", so any op 3 present in the binary
            // is meaningful — even if the string is empty.
            #[cfg(before_530)]
            let key = "defaultstring";
            #[cfg(since_530)]
            let key = "default";
            props.push((
                key.into(),
                def.default_str.as_deref().unwrap_or("").to_string(),
            ));
        }
        4 => {
            #[cfg(before_530)]
            props.push(("defaultint".into(), def.default_int.to_string()));
            #[cfg(since_530)]
            props.push((
                "default".into(),
                registry.typed_value(&char_to_type_name(def.outputtype as u8), def.default_int),
            ));
        }
        5..=8 => {
            // Entries, in decode order, duplicates included.
            #[cfg(since_530)]
            let (input, output) = (
                char_to_type_name(def.inputtype as u8),
                char_to_type_name(def.outputtype as u8),
            );
            #[cfg(since_742)]
            let _data = if code >= 7 {
                props.push(("size".into(), def.size.unwrap_or_default().to_string()));
                &_data[2..]
            } else {
                _data
            };
            let count = u16::from_be_bytes([_data[0], _data[1]]) as usize;
            for (key, value) in &def.values[def.values.len() - count..] {
                #[cfg(since_530)]
                let key = registry.typed_value(&input, *key);
                match value {
                    ParamValue::String(s) => {
                        props.push(("val".into(), format!("{key},{s}")));
                    }
                    #[cfg(before_530)]
                    ParamValue::Int(v) => {
                        props.push(("val".into(), format!("{key},{v}")));
                    }
                    #[cfg(since_530)]
                    ParamValue::Int(v) => {
                        let v = registry.typed_value(&output, *v);
                        props.push(("val".into(), format!("{key},{v}")));
                    }
                }
            }
        }
        _ => {}
    }

    #[cfg(before_530)]
    let _ = registry;

    props
}
