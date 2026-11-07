use crate::pack::names::AssetNames;
use anyhow::{Context, Result};

pub fn convert(text: &str, group: u32, names: &AssetNames, unpack: bool) -> Result<String> {
    let mut result = String::new();
    let reference = |domain: &str, value: &str| -> Result<String> {
        if unpack {
            Ok(names.reference(domain, value.parse()?))
        } else {
            Ok(names.resolve(domain, value)?.to_string())
        }
    };
    for line in text.lines() {
        if let Some(header) = line.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
            let name = if unpack {
                names.component_name(group, names.component_id(group, header)?)
            } else {
                format!("com_{}", names.component_id(group, header)?)
            };
            result.push_str(&format!("[{name}]\n"));
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            result.push_str(line);
            result.push('\n');
            continue;
        };
        let domain = match key {
            "graphic" => Some("sprites"),
            "model" => Some("model"),
            "animation" => Some("seq"),
            "text_font" => Some("fontmetrics"),
            "target_object_id" => Some("obj"),
            "target_enter_cursor" | "target_leave_cursor" => Some("cursor"),
            _ => None,
        };
        let new_value = if let Some(domain) = domain {
            reference(domain, value)?
        } else if key == "parent_id" {
            if unpack {
                names.component_name(group, value.parse()?)
            } else {
                names.component_id(group, value)?.to_string()
            }
        } else if key == "op_cursor" {
            let (index, cursor) = value.split_once(',').context("Invalid operation cursor")?;
            format!("{index},{}", reference("cursor", cursor)?)
        } else if key.ends_with("transmit_list") {
            let domain = match key {
                "vartransmit_list" => "varp",
                "invtransmit_list" => "inv",
                "stattransmit_list" => "stat",
                "varctransmit_list" => "varc_int",
                _ => "varc_string",
            };
            value
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| reference(domain, s))
                .collect::<Result<Vec<_>>>()?
                .join(",")
        } else if key.starts_with("on")
            || key
                .strip_prefix("hook")
                .is_some_and(|s| s.parse::<u8>().is_ok())
        {
            let separator = if value.starts_with('[') {
                value.find(']').map(|i| i + 1)
            } else {
                value.find(',')
            };
            let (script, args) = separator
                .map(|i| {
                    (
                        &value[..i],
                        value[i..].strip_prefix(',').unwrap_or(&value[i..]),
                    )
                })
                .unwrap_or((value, ""));
            let script = if unpack {
                reference("clientscript", script.strip_prefix('i').unwrap_or(script))?
            } else {
                format!("i{}", reference("clientscript", script)?)
            };
            if args.is_empty() {
                script
            } else {
                format!("{script},{args}")
            }
        } else {
            value.to_owned()
        };
        result.push_str(&format!("{key}={new_value}\n"));
    }
    Ok(result)
}
