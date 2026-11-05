use crate::pack::pack::parse_config_sections;
use std::collections::{BTreeMap, HashMap};
use std::mem;
use std::path::Path;

use anyhow::Result;
use tracing::info;

use crate::pack::packed_data::PackedData;

pub fn pack_generic_config(source_dir: &Path, type_name: &str) -> Result<PackedData> {
    let all_file = source_dir.join("_unpack").join(format!("all.{type_name}"));
    let sections = parse_config_sections(&[all_file], &HashMap::new());
    let prefix = format!("{type_name}_");
    let entries_map: BTreeMap<_, _> = sections
        .iter()
        .map(|(name, props)| {
            let id = name
                .strip_prefix(&prefix)
                .or_else(|| name.strip_prefix("entry_"))
                .unwrap_or(name)
                .parse::<u32>()
                .expect("Invalid generic config ID");
            (id, props.as_slice())
        })
        .collect();
    let max_id = entries_map.last_key_value().map_or(0, |(&id, _)| id + 1);

    let mut packed = PackedData::new(max_id);

    for id in 0..max_id {
        if let Some(props) = entries_map.get(&id) {
            packed.start_entry();
            for (key, value) in *props {
                pack_generic_prop(&mut packed, key, value);
            }
            packed.finish_entry();
        } else {
            packed.skip_entry();
        }
    }

    info!("  {type_name}: {} entries packed", entries_map.len());
    Ok(packed)
}

/// Pack a single key=value property into the PackedData buffer.
///
/// Key format patterns produced by the unpackers:
/// - `rawN=<hex>` — raw opcode + hex data bytes
/// - `opN_flag=yes` — bare opcode, no data
/// - `opN_g1=<u8>` — opcode + 1 byte
/// - `opN_g2=<u16>` — opcode + 2 bytes
/// - `opN_g2b=<i16>` — opcode + 2 signed bytes
/// - `opN_g3=<u32>` — opcode + 3 bytes
/// - `opN_g4=<i32>` — opcode + 4 bytes
/// - `opN_str=<string>` — opcode + null-terminated string
/// - `opN_str2=<string>` — opcode + version-prefixed string
/// - `opN_g1b=<i8>` — opcode + 1 signed byte
/// - `opN_g2_g2=a,b` — opcode + multi-value
/// - `opN_g2_g2_g4_g4=a,b,c,d` — opcode + multi-value
/// - `opN_g1_g1_g1=a,b,c` — opcode + multi-value
/// - `opN_g2b_g2b=a,b` — opcode + multi-value (signed)
/// - `opN_g1_Mxg2b=body,v1,...,vM` — opcode + g1 + M fixed g2b values
/// - `opN_counted_g1=count,v1,...` — opcode + g1 count + count values
/// - `opN_counted_g2=count,v1,...` — opcode + g1 count + count g2 values
/// - `opN_counted_g4=count,v1,...` — opcode + g1 count + count g4 values
/// - `opN_counted_g2_g1=count,v1,v2,...` — opcode + g1 count + count*(g2,g1)
/// - `opN_counted_g2_g4_g4=count,v1,v2,v3,...` — opcode + g1 count + count*(g2,g4,g4)
/// - `opN_counted_g1_g1=count,v1,v2,...` — opcode + g1 count + count*(g1,g1)
/// - `opN_counted_g4_g4_g4_str=count,v1,v2,v3,s,...` — opcode + g1 count + count*(g4,g4,g4,gjstr)
/// - `opN_g1_g2=body,val` — opcode + g1 + g2
/// - `opN_complex=...` — MEL op15 complex format
/// - `opN_params=count,is_str,key,val,...` — op249 params
fn pack_generic_prop(packed: &mut PackedData, key: &str, value: &str) {
    // Raw hex opcode: "rawN=hex_bytes"
    if let Some(op_str) = key.strip_prefix("raw") {
        let Ok(opcode) = op_str.parse::<u8>() else {
            return;
        };
        packed.p1(opcode);
        let hex = value.trim();
        let mut i = 0;
        while i + 1 < hex.len() {
            if let Ok(byte) = u8::from_str_radix(&hex[i..i + 2], 16) {
                packed.p1(byte);
            }
            i += 2;
        }
        return;
    }

    // Named opcode: "opN_*=value"
    if let Some(rest) = key.strip_prefix("op") {
        let num_end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        if num_end == 0 {
            return;
        }
        let Ok(opcode) = rest[..num_end].parse::<u8>() else {
            return;
        };
        let suffix = &rest[num_end..];
        let suffix = suffix.strip_prefix('_').unwrap_or(suffix);

        packed.p1(opcode);
        pack_opcode_data(packed, suffix, value);
    }
}

/// Write the data payload for an opcode based on the suffix descriptor.
fn pack_opcode_data(packed: &mut PackedData, suffix: &str, value: &str) {
    // Handle specific complex patterns first
    if suffix == "params" {
        pack_params(packed, value);
        return;
    }
    if suffix == "complex" {
        pack_mel_op15(packed, value);
        return;
    }

    // Counted arrays: "counted_<inner_types>"
    if let Some(inner) = suffix.strip_prefix("counted_") {
        pack_counted_value(packed, value, inner);
        return;
    }

    // Fixed-repeat patterns: "g1_Nxg2b" (e.g. "g1_6xg2b", "g1_3xg2b")
    if let Some(rest) = suffix.strip_prefix("g1_") {
        if let Some(repeat_part) = rest.strip_suffix("xg2b") {
            if let Ok(repeat_count) = repeat_part.parse::<usize>() {
                let values: Vec<&str> = value.split(',').map(str::trim).collect();
                // First value is g1
                packed.p1(values.first().and_then(|v| v.parse().ok()).unwrap_or(0u8));
                // Next N values are g2b
                for i in 0..repeat_count {
                    let v: i16 = values.get(1 + i).and_then(|v| v.parse().ok()).unwrap_or(0);
                    packed.p2(v as u16);
                }
                return;
            }
        }
    }

    // Simple and multi-value types
    match suffix {
        "" | "flag" => {
            // Flag: no data bytes
        }
        "g1" | "u8" => {
            packed.p1(value.parse::<u8>().unwrap_or(0));
        }
        "g1b" | "i8" => {
            packed.p1(value.parse::<i8>().unwrap_or(0) as u8);
        }
        "g2" | "u16" => {
            packed.p2(value.parse::<u16>().unwrap_or(0));
        }
        "g2b" | "i16" => {
            packed.p2(value.parse::<i16>().unwrap_or(0) as u16);
        }
        "g3" | "u24" => {
            packed.p3((value.parse::<u32>().unwrap_or(0)) as i32);
        }
        "g4" | "i32" => {
            packed.p4(value.parse::<i32>().unwrap_or(0));
        }
        "str" | "gjstr" => {
            packed.pjstr(value);
        }
        "str2" | "gjstr2" => {
            packed.p1(0); // version prefix byte
            packed.pjstr(value);
        }
        "bool" => {
            let v: u8 = if value == "true" || value == "1" || value == "yes" {
                1
            } else {
                0
            };
            packed.p1(v);
        }
        other => {
            // Multi-value: parse underscore-separated type descriptors
            // E.g. "g2_g2" → read 2 comma-separated values as g2, g2
            // E.g. "g1_g1_g1" → read 3 comma-separated values as g1, g1, g1
            // E.g. "g2_g1_g1" → read 3 comma-separated values
            if other.contains('_') {
                pack_multi_value(packed, value, other);
            }
            // else: unknown suffix, skip
        }
    }
}

/// Split a comma-separated string, treating `\,` as a literal comma within a value.
fn split_escaped_csv(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b',' {
            current.push(',');
            i += 2;
        } else if bytes[i] == b',' {
            result.push(mem::take(&mut current));
            i += 1;
        } else {
            current.push(bytes[i] as char);
            i += 1;
        }
    }
    result.push(current);
    result
}

/// Pack a multi-value property: comma-separated values with types from suffix.
fn pack_multi_value(packed: &mut PackedData, value: &str, types_str: &str) {
    let types: Vec<&str> = types_str.split('_').collect();
    let values = split_escaped_csv(value);

    for (i, ty) in types.iter().enumerate() {
        let v = values.get(i).map(|s| s.trim()).unwrap_or("0");
        pack_single_typed_value(packed, ty, v);
    }
}

/// Pack a counted array.
///
/// Value format: "count,val1,val2,..."
/// The count byte is written as g1, then `count` repetitions of the inner
/// types (which may be compound like "g2_g4_g4").
fn pack_counted_value(packed: &mut PackedData, value: &str, inner: &str) {
    let values = split_escaped_csv(value);
    let count: u8 = values
        .first()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    packed.p1(count);

    if count == 0 || values.len() <= 1 {
        return;
    }

    let inner_types: Vec<&str> = inner.split('_').collect();
    let stride = inner_types.len();

    let data_values = &values[1..]; // skip count
    for i in 0..count as usize {
        for (j, ty) in inner_types.iter().enumerate() {
            let idx = i * stride + j;
            let v = data_values.get(idx).map(|s| s.trim()).unwrap_or("0");
            pack_single_typed_value(packed, ty, v);
        }
    }
}

/// Pack a single typed value.
fn pack_single_typed_value(packed: &mut PackedData, ty: &str, v: &str) {
    match ty {
        "g1" | "u8" => packed.p1(v.parse::<u8>().unwrap_or(0)),
        "g1b" | "i8" => packed.p1(v.parse::<i8>().unwrap_or(0) as u8),
        "g2" | "u16" => packed.p2(v.parse::<u16>().unwrap_or(0)),
        "g2b" | "i16" => packed.p2(v.parse::<i16>().unwrap_or(0) as u16),
        "g3" | "u24" => packed.p3(v.parse::<i32>().unwrap_or(0)),
        "g4" | "i32" => packed.p4(v.parse::<i32>().unwrap_or(0)),
        "str" | "gjstr" => packed.pjstr(v),
        "str2" | "gjstr2" => {
            packed.p1(0);
            packed.pjstr(v);
        }
        _ => {}
    }
}

/// Pack MEL op15 complex format.
///
/// Value: "count,<count*2 x g2b values>,<g4>,count2,<count2 x g4>,<count x g1b>"
fn pack_mel_op15(packed: &mut PackedData, value: &str) {
    let values: Vec<&str> = value.split(',').map(str::trim).collect();
    let mut idx = 0;

    let count: u8 = values.get(idx).and_then(|v| v.parse().ok()).unwrap_or(0);
    packed.p1(count);
    idx += 1;

    // count * 2 x g2b
    for _ in 0..(count as usize * 2) {
        let v: i16 = values.get(idx).and_then(|v| v.parse().ok()).unwrap_or(0);
        packed.p2(v as u16);
        idx += 1;
    }

    // g4
    let v: i32 = values.get(idx).and_then(|v| v.parse().ok()).unwrap_or(0);
    packed.p4(v);
    idx += 1;

    // count2
    let count2: u8 = values.get(idx).and_then(|v| v.parse().ok()).unwrap_or(0);
    packed.p1(count2);
    idx += 1;

    // count2 x g4
    for _ in 0..count2 {
        let v: i32 = values.get(idx).and_then(|v| v.parse().ok()).unwrap_or(0);
        packed.p4(v);
        idx += 1;
    }

    // count x g1b
    for _ in 0..count {
        let v: i8 = values.get(idx).and_then(|v| v.parse().ok()).unwrap_or(0);
        packed.p1(v as u8);
        idx += 1;
    }
}

/// Pack op249 params.
///
/// Value format: "count,is_str1,key1,val1,is_str2,key2,val2,..."
fn pack_params(packed: &mut PackedData, value: &str) {
    let values = split_escaped_csv(value);
    let count: u8 = values
        .first()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    packed.p1(count);

    if count == 0 || values.len() <= 1 {
        return;
    }

    let mut idx = 1;
    for _ in 0..count {
        let is_string: u8 = values
            .get(idx)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        idx += 1;
        let key: u32 = values
            .get(idx)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        idx += 1;

        packed.p1(is_string);
        packed.p3((key) as i32);

        if is_string == 1 {
            let s = values.get(idx).map(|v| v.trim()).unwrap_or("");
            packed.pjstr(s);
        } else {
            let v: i32 = values
                .get(idx)
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(0);
            packed.p4(v);
        }
        idx += 1;
    }
}
