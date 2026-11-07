use super::map::{is_landscape, is_npc_spawns, is_obj_spawns};
use crate::pack::names::AssetNames;
use anyhow::{Context, Result};

pub fn convert(text: &str, name: &str, names: &AssetNames, unpack: bool) -> Result<String> {
    let domain = if is_landscape(name) {
        Some("loc")
    } else if is_npc_spawns(name) {
        Some("npc")
    } else if is_obj_spawns(name) {
        Some("obj")
    } else {
        None
    };
    let reference = |kind: &str, value: &str, bias: i64| -> Result<String> {
        if unpack {
            Ok(names.reference(kind, value.parse::<i64>()? - bias))
        } else {
            Ok((names.resolve(kind, value)? + bias).to_string())
        }
    };
    let mut result = String::new();
    let mut section = "tiles";
    for line in text.lines() {
        let line = line.trim();
        let converted = if line.starts_with("====") {
            section = if line.contains("LIGHTS") {
                "lights"
            } else if line.contains("SKYBOX") {
                "skybox"
            } else if line.contains("ENV") {
                "env"
            } else {
                "other"
            };
            line.to_owned()
        } else if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            line.to_owned()
        } else if let Some(kind) = domain {
            let (coords, rest) = line.split_once(':').context("Invalid map placement")?;
            let mut fields = rest.split_whitespace();
            let id = reference(kind, fields.next().context("Missing map asset")?, 0)?;
            format!(
                "{coords}: {id}{}",
                fields.map(|v| format!(" {v}")).collect::<String>()
            )
        } else if let Some(value) = line.strip_prefix("env_map=") {
            format!(
                "env_map={}",
                value
                    .split(',')
                    .map(|v| reference("texture", v, 0))
                    .collect::<Result<Vec<_>>>()?
                    .join(",")
            )
        } else if section == "tiles" && line.contains(':') {
            let (coords, fields) = line.split_once(':').unwrap();
            let fields = fields
                .split_whitespace()
                .map(|field| -> Result<String> {
                    let kind = match field.as_bytes().first() {
                        Some(b'u') => "flu",
                        Some(b'o') => "flo",
                        _ => return Ok(field.to_owned()),
                    };
                    let (id, extra) = field[1..].split_once(';').unwrap_or((&field[1..], ""));
                    Ok(format!(
                        "{}{}{}",
                        &field[..1],
                        reference(kind, id, 1)?,
                        if extra.is_empty() {
                            String::new()
                        } else {
                            format!(";{extra}")
                        }
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            format!("{coords}: {}", fields.join(" "))
        } else if section == "lights" {
            line.split_whitespace()
                .map(|v| -> Result<String> {
                    if let Some(id) = v.strip_prefix("ref=") {
                        Ok(format!("ref={}", reference("light", id, 0)?))
                    } else {
                        Ok(v.to_owned())
                    }
                })
                .collect::<Result<Vec<_>>>()?
                .join(" ")
        } else {
            line.to_owned()
        };
        result.push_str(&converted);
        result.push('\n');
    }
    Ok(result)
}
