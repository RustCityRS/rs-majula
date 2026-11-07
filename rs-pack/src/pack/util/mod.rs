#[cfg(since_647)]
use crate::pack::config::param::type_name_to_char;
use crate::pack::pack_registry::{PackFile, PackRegistry};
#[cfg(since_647)]
use crate::pack::packed_data::PackedData;
use crate::pack::util::colour::RecolType;
#[cfg(before_647)]
use crate::types::ScriptVarType;
#[cfg(since_647)]
use anyhow::{Context, Result, bail};
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

/// What the dumps call an unnamed config of a table.
#[cfg(since_647)]
pub fn placeholder_prefix(kind: &str) -> &str {
    match kind {
        "flo" => "overlay",
        "flu" => "underlay",
        "idk" => "idkit",
        "mel" => "mapelement",
        "particleeffector" => "pef",
        "particleemitter" => "pem",
        "quickchatcat" => "chatcat",
        "quickchatphrase" => "chatphrase",
        "sprites" => "graphic",
        "texture" => "material",
        "varbit" => "varplayerbit",
        "varc" | "varcs" => "varclient",
        "varcon" => "config43",
        "varn" => "varnpc",
        "varnbit" => "varnpcbit",
        "varobj" => "varobject",
        "varp" => "varplayer",
        "vars" => "varworld",
        _ => kind,
    }
}

pub fn parse_ref(pack: &PackFile, kind: &str, value: &str, callback: impl FnOnce(u16)) {
    #[cfg(since_647)]
    let value = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value);
    #[cfg(since_647)]
    let kind = placeholder_prefix(kind);
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
    #[cfg(since_647)]
    if let Some(rest) = key.strip_prefix("recolhsl") {
        let (num_part, suffix) = rest.split_at(rest.len().saturating_sub(1));
        if let (Ok(idx), Ok(val)) = (num_part.parse::<usize>(), value.parse::<u16>()) {
            match suffix {
                "s" => return callback(RecolType::S(idx, val)),
                "d" => return callback(RecolType::D(idx, val)),
                _ => {}
            }
        }
        panic!("Invalid recol format for key: '{key}' with value: '{value}'");
    }
    #[cfg(since_647)]
    if let Some(rest) = key.strip_prefix("recol") {
        let (num_part, suffix) = rest.split_at(rest.len().saturating_sub(1));
        if let (Ok(idx), Ok(val)) = (num_part.parse::<usize>(), value.parse::<u16>()) {
            let val = colour::rgb15_to_hsl16(val);
            match suffix {
                "s" => return callback(RecolType::S(idx, val)),
                "d" => return callback(RecolType::D(idx, val)),
                _ => {}
            }
        }
    }
    #[cfg(before_647)]
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
/// A colour as `0xRRGGBB` or `0xAARRGGBB`.
#[cfg(since_647)]
pub fn parse_colour(value: &str) -> i32 {
    let hex = value
        .strip_prefix("0x")
        .unwrap_or_else(|| panic!("Invalid colour: {value}"));
    u32::from_str_radix(hex, 16).unwrap_or_else(|_| panic!("Invalid colour: {value}")) as i32
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
/// The recolours of a record, in one opcode 40.
#[cfg(since_647)]
pub fn pack_recol_pairs(packed: &mut PackedData, props: &[(String, String)]) {
    let mut pairs = BTreeMap::new();
    for (key, value) in props {
        parse_recol(key, value, |v| match v {
            RecolType::S(index, v) => pairs.entry(index).or_insert([0u16; 2])[0] = v,
            RecolType::D(index, v) => pairs.entry(index).or_insert([0u16; 2])[1] = v,
        });
    }
    write_pairs(packed, 40, &pairs);
}

/// The retextures of a record, in one opcode 41.
#[cfg(since_647)]
pub fn pack_retex_pairs(
    packed: &mut PackedData,
    props: &[(String, String)],
    registry: &PackRegistry,
) {
    let mut pairs = BTreeMap::new();
    for (key, value) in props {
        parse_retex(registry, key, value, |v| match v {
            RecolType::S(index, v) => pairs.entry(index).or_insert([0u16; 2])[0] = v,
            RecolType::D(index, v) => pairs.entry(index).or_insert([0u16; 2])[1] = v,
        });
    }
    write_pairs(packed, 41, &pairs);
}

#[cfg(since_647)]
fn write_pairs(packed: &mut PackedData, code: u8, pairs: &BTreeMap<usize, [u16; 2]>) {
    if let Some((&count, _)) = pairs.last_key_value() {
        packed.p1(code);
        packed.p1(u8::try_from(count).expect("Too many pairs"));
        for index in 1..=count {
            for value in pairs.get(&index).copied().unwrap_or_default() {
                packed.p2(value);
            }
        }
    }
}

/// A sound by name, and whether it is a vorbis file rather than a synth.
#[cfg(since_647)]
pub fn parse_sound(registry: &PackRegistry, value: &str) -> (u16, bool) {
    if let Some(id) = value.strip_prefix("vorbis_") {
        let id = id
            .parse()
            .unwrap_or_else(|_| panic!("Invalid vorbis: {value}"));
        return (id, true);
    }
    let mut id = u16::MAX;
    parse_ref(&registry.synth, "synth", value, |v| id = v);
    (id, false)
}

/// `op,cursor`, with the op counted from one.
#[cfg(since_647)]
pub fn pack_cursor(packed: &mut PackedData, code: u8, value: &str, registry: &PackRegistry) {
    let (op, cursor) = value
        .split_once(',')
        .unwrap_or_else(|| panic!("Invalid cursor: {value}"));
    let op = op
        .trim()
        .parse::<u8>()
        .unwrap_or_else(|_| panic!("Invalid cursor op: {value}"));
    packed.p1(code);
    packed.p1(op - 1);
    parse_ref(&registry.cursor, "cursor", cursor.trim(), |v| packed.p2(v));
}

/// A record's `multivar` and target lines as one opcode: `codes.0`, or
/// `codes.1` when one of the lines is the default.
#[cfg(since_647)]
pub fn pack_multi(
    packed: &mut PackedData,
    codes: (u8, u8),
    props: &[(String, String)],
    (kind, pack): (&str, &PackFile),
    registry: &PackRegistry,
    name: &str,
) -> Result<()> {
    let mut varbit = u16::MAX;
    let mut varp = u16::MAX;
    let mut default = None;
    let mut targets = BTreeMap::new();
    for (key, value) in props {
        if key == "multivar" {
            let bit = registry
                .varbit
                .get_by_debugname(value)
                .or_else(|| placeholder_number("varbit", value));
            let var = registry
                .varp
                .get_by_debugname(value)
                .or_else(|| placeholder_number("varp", value));
            match (bit, var) {
                (Some(id), None) => varbit = u16::try_from(id)?,
                (None, Some(id)) => varp = u16::try_from(id)?,
                (Some(_), Some(_)) => bail!("{name}: ambiguous multivar '{value}'"),
                (None, None) => bail!("{name}: unknown multivar '{value}'"),
            }
        } else {
            let (index, target) = value
                .split_once(',')
                .with_context(|| format!("{name}: invalid {key} '{value}'"))?;
            let target = target.trim();
            let id = if target == "null" {
                u16::MAX
            } else {
                u16::try_from(
                    pack.get_by_debugname(target)
                        .or_else(|| placeholder_number(kind, target))
                        .with_context(|| format!("{name}: unknown {key} '{target}'"))?,
                )?
            };
            if index.trim() == "default" {
                default = Some(id);
            } else {
                targets.insert(index.trim().parse::<u8>()?, id);
            }
        }
    }
    packed.p1(if default.is_some() { codes.1 } else { codes.0 });
    packed.p2(varbit);
    packed.p2(varp);
    if let Some(default) = default {
        packed.p2(default);
    }
    let count = targets.last_key_value().map_or(0, |(&index, _)| index);
    packed.p1(count);
    for index in 0..=count {
        packed.p2(targets.get(&index).copied().unwrap_or(u16::MAX));
    }
    Ok(())
}

#[cfg(since_647)]
fn placeholder_number<T: FromStr>(kind: &str, value: &str) -> Option<T> {
    value
        .strip_prefix(placeholder_prefix(kind))?
        .strip_prefix('_')?
        .parse()
        .ok()
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
