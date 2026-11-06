#[cfg(since_647)]
use crate::pack::config::param::type_name_to_char;
use crate::pack::pack_registry::{PackFile, PackRegistry};
#[cfg(since_647)]
use crate::pack::packed_data::PackedData;
use crate::pack::util::colour::RecolType;
#[cfg(before_647)]
use crate::types::ScriptVarType;
use rs_io::crc;
#[cfg(since_647)]
use std::array::from_fn;
#[cfg(since_647)]
use std::collections::BTreeMap;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub mod colour;
pub mod media;
pub mod palette;

pub fn parse_number<T, F>(value: &str, callback: F)
where
    T: FromStr + Into<T> + Copy,
    <T as FromStr>::Err: Display,
    F: FnOnce(T),
{
    match value.parse::<T>() {
        Ok(val) => {
            callback(val);
        }
        Err(e) => panic!("Invalid numeric value '{value}': {e}"),
    }
}

pub fn parse_hex<F>(value: &str, callback: F)
where
    F: FnOnce(i32),
{
    let hex_str = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);

    match i32::from_str_radix(hex_str, 16) {
        Ok(val) => callback(val),
        Err(e) => panic!("Invalid hex value '{value}': {e}"),
    }
}

pub fn parse_bool<F>(value: &str, callback: F)
where
    F: FnOnce(bool),
{
    callback(match value {
        "true" | "yes" | "1" => true,
        "false" | "no" | "0" => false,
        _ => panic!("Invalid boolean value '{value}'"),
    });
}

pub fn parse_ref(pack: &PackFile, kind: &str, value: &str, callback: impl FnOnce(u16)) {
    let id = if value == "null" {
        u16::MAX
    } else if let Some(id) = pack.get_by_debugname(value) {
        #[cfg(since_647)]
        let id =
            u16::try_from(id).unwrap_or_else(|_| panic!("{kind} reference exceeds 65535: {value}"));
        id
    } else {
        let number = value
            .strip_prefix(kind)
            .and_then(|suffix| suffix.strip_prefix('_'))
            .and_then(|suffix| suffix.split('_').next())
            .unwrap_or(value);
        number
            .parse()
            .unwrap_or_else(|_| panic!("Unknown {kind}: {value}"))
    };
    callback(id);
}

pub fn parse_model_kind<F>(registry: &PackRegistry, kind: &str, key: &str, value: &str, callback: F)
where
    F: FnOnce((usize, u16)),
{
    if let Some(rest) = key.strip_prefix(kind)
        && let Ok(idx) = rest.parse::<usize>()
    {
        parse_ref(&registry.model, "model", value, |v| {
            callback((idx, v));
        });
        return;
    }
    panic!("Invalid model format for key: '{key}' with value: '{value}'");
}

pub fn parse_recol<F>(key: &str, value: &str, callback: F)
where
    F: FnOnce(RecolType),
{
    if let Some(rest) = key.strip_prefix("recol") {
        let (num_part, suffix) = rest.split_at(rest.len().saturating_sub(1));
        if let (Ok(idx), Ok(val)) = (num_part.parse::<usize>(), value.parse::<u16>()) {
            match suffix {
                "s" => return callback(RecolType::S(idx, val)),
                "d" => return callback(RecolType::D(idx, val)),
                _ => {}
            }
        }
    }
    panic!("Invalid recol format for key: '{key}' with value: '{value}'");
}

pub fn parse_retex<F>(registry: &PackRegistry, key: &str, value: &str, callback: F)
where
    F: FnOnce(RecolType),
{
    if let Some(rest) = key.strip_prefix("retex") {
        let (num_part, suffix) = rest.split_at(rest.len().saturating_sub(1));
        parse_ref(&registry.texture, "texture", value, |v| {
            if let Ok(idx) = num_part.parse::<usize>() {
                match suffix {
                    "s" => callback(RecolType::S(idx, v)),
                    "d" => callback(RecolType::D(idx, v)),
                    _ => {}
                }
            }
        });
        return;
    }
    panic!("Invalid retex format for key: '{key}' with value: '{value}'");
}

pub fn parse_script_var_type<F>(value: &str, callback: F)
where
    F: FnOnce(char),
{
    #[cfg(before_647)]
    callback(ScriptVarType::from_type_name(value).to_char());
    #[cfg(since_647)]
    callback(type_name_to_char(value));
}

pub fn parse_coord(value: &str) -> Option<i32> {
    let parts: Vec<&str> = value.split('_').collect();
    if parts.len() != 5 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let mx: i32 = parts[1].parse().ok()?;
    let mz: i32 = parts[2].parse().ok()?;
    let lx: i32 = parts[3].parse().ok()?;
    let lz: i32 = parts[4].parse().ok()?;

    if lz < 0 || lx < 0 || mz < 0 || mx < 0 || y < 0 {
        return None;
    }
    if lz > 63 || lx > 63 || mz > 255 || mx > 255 || y > 3 {
        return None;
    }

    let x = (mx << 6) + lx;
    let z = (mz << 6) + lz;
    Some(z | (x << 14) | (y << 28))
}

pub fn walk(path: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let Ok(entries) = fs::read_dir(path) else {
        return results;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            results.extend(walk(&path));
        } else {
            results.push(path);
        }
    }
    results
}

pub fn load_order(path: &Path) -> Vec<u16> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|l| l.trim().parse::<u16>().ok())
        .collect()
}

pub fn parse_csv_values(s: &str) -> Vec<String> {
    let mut vals = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in s.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch == ',' && !in_quotes {
            vals.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    vals.push(current.trim().to_string());
    vals
}

#[cfg(since_647)]
pub fn pack_pairs(
    packed: &mut PackedData,
    code: u8,
    source: &[(usize, u16)],
    dest: &[(usize, u16)],
) {
    if source.is_empty() {
        return;
    }
    assert_eq!(
        source.len(),
        dest.len(),
        "Mismatched recolour or retexture pairs"
    );
    packed.p1(code);
    packed.p1(u8::try_from(source.len()).expect("Too many recolour or retexture pairs"));
    for ((_, source), (_, dest)) in source.iter().zip(dest) {
        packed.p2(*source);
        packed.p2(*dest);
    }
}

#[cfg(since_647)]
pub fn config_key_index(key: &str, prefix: &str) -> Option<usize> {
    key.strip_prefix(prefix)?.parse().ok()
}

#[cfg(since_647)]
pub fn parse_numbers<T: FromStr, const N: usize>(value: &str) -> [T; N] {
    let mut values = value.split(',');
    let result = from_fn(|_| {
        values
            .next()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or_else(|| panic!("Invalid numeric tuple: {value}"))
    });
    assert!(values.next().is_none(), "Too many values: {value}");
    result
}

#[cfg(since_647)]
pub fn pack_indexed_pairs(
    packed: &mut PackedData,
    code: u8,
    props: &[(String, String)],
    first: char,
) {
    let mut pairs = BTreeMap::new();
    for (key, value) in props {
        let index: u8 = key[5..key.len() - 1].parse().expect("Invalid pair index");
        assert!(index > 0, "Pair index starts at 1: {key}");
        let pair = pairs.entry(index).or_insert([0u16; 2]);
        pair[usize::from(!key.ends_with(first))] =
            value.trim().parse().expect("Invalid pair value");
    }
    if let Some((&count, _)) = pairs.last_key_value() {
        packed.p1(code);
        packed.p1(count);
        for index in 1..=count {
            for value in pairs.get(&index).copied().unwrap_or_default() {
                packed.p2(value);
            }
        }
    }
}

#[cfg(since_647)]
pub fn pack_indexed_values<F: Fn(&mut PackedData, &str)>(
    packed: &mut PackedData,
    code: u8,
    props: &[(String, String)],
    prefix: &str,
    write: F,
) {
    let values: BTreeMap<_, _> = props
        .iter()
        .map(|(key, value)| {
            let index = u8::try_from(config_key_index(key, prefix).expect("Invalid value index"))
                .expect("Too many values");
            assert!(index > 0, "Value index starts at 1: {key}");
            (index, value.as_str())
        })
        .collect();
    if let Some((&count, _)) = values.last_key_value() {
        packed.p1(code);
        packed.p1(count);
        for index in 1..=count {
            write(packed, values.get(&index).copied().unwrap_or("0"));
        }
    }
}

pub fn verify_crc(name: &str, data: &[u8], expected: i32) {
    let actual = crc::getcrc(data, 0, data.len());
    assert_eq!(
        actual, expected,
        "CRC mismatch ['{name}']: Got: {actual}, Expected: {expected}"
    );
}
