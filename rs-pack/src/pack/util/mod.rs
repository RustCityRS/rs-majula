use crate::pack::pack_registry::PackRegistry;
use crate::pack::util::colour::RecolType;
use crate::types::ScriptVarType;
use std::fmt::Display;
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
    if value != "true"
        && value != "false"
        && value != "0"
        && value != "1"
        && value != "yes"
        && value != "no"
    {
        panic!("Invalid boolean value '{value}'")
    }
    callback(value == "yes" || value == "true" || value == "1");
}

pub fn parse_anim<F>(registry: &PackRegistry, value: &str, callback: F)
where
    F: FnOnce(u16),
{
    if let Some(val) = registry.anim.get_by_debugname(value) {
        callback(val);
    } else {
        panic!("Anim not found for value: '{value}'");
    }
}

pub fn parse_category<F>(registry: &PackRegistry, value: &str, callback: F)
where
    F: FnOnce(u16),
{
    if let Some(val) = registry.category.get_by_debugname(value) {
        callback(val);
    } else {
        panic!("Category not found for value: '{value}'");
    }
}

pub fn parse_hunt<F>(registry: &PackRegistry, value: &str, callback: F)
where
    F: FnOnce(u16),
{
    if let Some(val) = registry.hunt.get_by_debugname(value) {
        callback(val);
    } else {
        panic!("Hunt not found for value: '{value}'");
    }
}

pub fn parse_inv<F>(registry: &PackRegistry, value: &str, callback: F)
where
    F: FnOnce(u16),
{
    if let Some(val) = registry.inv.get_by_debugname(value) {
        callback(val);
    } else {
        panic!("Inv not found for value: '{value}'");
    }
}

pub fn parse_loc<F>(registry: &PackRegistry, value: &str, callback: F)
where
    F: FnOnce(u16),
{
    if let Some(val) = registry.loc.get_by_debugname(value) {
        callback(val);
    } else {
        panic!("Loc not found for value: '{value}'");
    }
}

pub fn parse_model_kind<F>(registry: &PackRegistry, kind: &str, key: &str, value: &str, callback: F)
where
    F: FnOnce((usize, u16)),
{
    if let Some(rest) = key.strip_prefix(kind)
        && let Ok(idx) = rest.parse::<usize>()
    {
        parse_model(registry, value, |v| {
            callback((idx, v));
        });
        return;
    }
    panic!("Invalid model format for key: '{key}' with value: '{value}'");
}

pub fn parse_model<F>(registry: &PackRegistry, value: &str, callback: F)
where
    F: FnOnce(u16),
{
    if let Some(val) = registry.model.get_by_debugname(value) {
        callback(val);
    } else {
        panic!("Model not found for value: '{value}'");
    }
}

pub fn parse_npc<F>(registry: &PackRegistry, value: &str, callback: F)
where
    F: FnOnce(u16),
{
    if let Some(val) = registry.npc.get_by_debugname(value) {
        callback(val);
    } else {
        panic!("Npc not found for value: '{value}'");
    }
}

pub fn parse_obj<F>(registry: &PackRegistry, value: &str, callback: F)
where
    F: FnOnce(u16),
{
    if let Some(val) = registry.obj.get_by_debugname(value) {
        callback(val);
    } else {
        panic!("Obj not found for value: '{value}'");
    }
}

pub fn parse_param<F>(registry: &PackRegistry, value: &str, callback: F)
where
    F: FnOnce(u16),
{
    if let Some(val) = registry.param.get_by_debugname(value) {
        callback(val);
    } else {
        panic!("Param not found for value: '{value}'");
    }
}

pub fn parse_seq<F>(registry: &PackRegistry, value: &str, callback: F)
where
    F: FnOnce(u16),
{
    if let Some(val) = registry.seq.get_by_debugname(value) {
        callback(val);
    } else {
        panic!("Seq not found for value: '{value}'");
    }
}

pub fn parse_texture<F>(registry: &PackRegistry, value: &str, callback: F)
where
    F: FnOnce(u16),
{
    if let Some(val) = registry.texture.get_by_debugname(value) {
        callback(val);
    } else {
        panic!("Texture not found for value: '{value}'");
    }
}

pub fn parse_varn<F>(registry: &PackRegistry, value: &str, callback: F)
where
    F: FnOnce(u16),
{
    if let Some(val) = registry.varn.get_by_debugname(value) {
        callback(val);
    } else {
        panic!("Varn not found for value: '{value}'");
    }
}

pub fn parse_varp<F>(registry: &PackRegistry, value: &str, callback: F)
where
    F: FnOnce(u16),
{
    if let Some(val) = registry.varp.get_by_debugname(value) {
        callback(val);
    } else {
        panic!("Varp not found for value: '{value}'");
    }
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
        parse_texture(registry, value, |v| {
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
    callback(ScriptVarType::from_type_name(value).to_char());
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
    let Ok(entries) = std::fs::read_dir(path) else {
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
    let Ok(text) = std::fs::read_to_string(path) else {
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
