use crate::cs2::opcodes::{is_branch, opcode_name};
use crate::cs2::script::Script;
use crate::pack::names::AssetNames;
use anyhow::{Context, Result, bail, ensure};
use rs_io::Packet;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::OnceLock;

fn opcode_table() -> &'static HashMap<String, u16> {
    static TABLE: OnceLock<HashMap<String, u16>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut counts = HashMap::new();
        for op in 0..=u16::MAX {
            *counts.entry(opcode_name(op)).or_insert(0) += 1;
        }
        (0..=u16::MAX)
            .filter_map(|op| {
                let name = opcode_name(op);
                if name.is_empty() || name == "unknown" {
                    None
                } else {
                    Some((
                        if counts[name] == 1 {
                            name.to_owned()
                        } else {
                            format!("{name}_{op}")
                        },
                        op,
                    ))
                }
            })
            .collect()
    })
}

fn domain(op: u16) -> Option<&'static str> {
    match op {
        1 | 2 => Some("varp"),
        25 | 27 => Some("varbit"),
        40 => Some("clientscript"),
        42 | 43 => Some("varc_int"),
        47 | 48 => Some("varc_string"),
        _ => None,
    }
}

pub fn unpack(
    data: &[u8],
    names: &AssetNames,
    format: &runec::clientscript::format::Format,
) -> Result<String> {
    let script = Script::decode(data, format)?;
    let mut text = format!("name={}\n", serde_json::to_string(&script.name)?);
    for (key, value) in [
        ("local_int_count", script.local_int_count),
        ("local_string_count", script.local_string_count),
        ("local_long_count", script.local_long_count),
        ("int_param_count", script.int_param_count),
        ("string_param_count", script.string_param_count),
        ("long_param_count", script.long_param_count),
    ] {
        writeln!(text, "{key}={value}")?;
    }
    text.push('\n');
    for (i, &op) in script.opcodes.iter().enumerate() {
        let name = opcode_name(op);
        let name = if opcode_table().get(name) == Some(&op) {
            name.to_owned()
        } else if opcode_table().contains_key(&format!("{name}_{op}")) {
            format!("{name}_{op}")
        } else {
            format!("op{op}")
        };
        let value = if op == 3 {
            serde_json::to_string(script.string_operands[i].as_ref().unwrap())?
        } else if op == 54 {
            script.long_operands[i].unwrap().to_string()
        } else if is_branch(op) {
            format!("L{}", i as i64 + 1 + script.int_operands[i] as i64)
        } else if op == 51 {
            format!("switch{}", script.int_operands[i])
        } else if let Some(domain) = domain(op) {
            names.reference(domain, script.int_operands[i] as i64)
        } else {
            script.int_operands[i].to_string()
        };
        writeln!(text, "L{i}: {name} {value}")?;
    }
    writeln!(text, "L{}:", script.opcodes.len())?;
    for (i, table) in script.switches.iter().enumerate() {
        let anchors: Vec<_> = script
            .opcodes
            .iter()
            .enumerate()
            .filter_map(|(pc, &op)| (op == 51 && script.int_operands[pc] == i as i32).then_some(pc))
            .collect();
        if let [anchor] = anchors.as_slice() {
            writeln!(text, "\n[switch{i}@L{anchor}]")?;
            for (key, offset) in table {
                writeln!(text, "{key}=L{}", *anchor as i64 + 1 + *offset as i64)?;
            }
        } else {
            writeln!(text, "\n[switch{i}]")?;
            for (key, offset) in table {
                writeln!(text, "{key}={offset}")?;
            }
        }
    }
    Ok(text)
}

pub fn pack(text: &str, names: &AssetNames) -> Result<Vec<u8>> {
    let opcodes = opcode_table();
    let mut metadata = HashMap::new();
    let mut labels = HashMap::new();
    let mut instructions = Vec::new();
    let mut switches: Vec<Vec<(i32, String)>> = Vec::new();
    let mut switch_bases = Vec::new();
    let mut current_switch = None;
    for (line_no, line) in text.lines().enumerate() {
        let mut line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        if let Some(index) = line
            .strip_prefix("[switch")
            .and_then(|s| s.strip_suffix(']'))
        {
            let (index, base) = index
                .split_once('@')
                .map_or((index, None), |(index, base)| {
                    (index, Some(base.to_owned()))
                });
            let index: usize = index.parse()?;
            ensure!(index == switches.len(), "Switch tables must be consecutive");
            switches.push(Vec::new());
            switch_bases.push(base);
            current_switch = Some(index);
            continue;
        }
        if let Some(index) = current_switch {
            let (key, value) = line.split_once('=').context("Invalid switch case")?;
            switches[index].push((key.trim().parse()?, value.trim().to_owned()));
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            if key
                .trim()
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                ensure!(
                    metadata
                        .insert(key.trim().to_owned(), value.trim().to_owned())
                        .is_none(),
                    "Duplicate script property {key}"
                );
                continue;
            }
        }
        if let Some((label, rest)) = line
            .split_once(':')
            .filter(|(label, _)| !label.contains(char::is_whitespace))
        {
            ensure!(
                labels
                    .insert(label.trim().to_owned(), instructions.len())
                    .is_none(),
                "Duplicate label {label}"
            );
            line = rest.trim();
            if line.is_empty() {
                continue;
            }
        }
        let (op, operand) = line
            .split_once(char::is_whitespace)
            .with_context(|| format!("Line {}: expected instruction operand", line_no + 1))?;
        let opcode = if let Some(value) = op.strip_prefix("op").and_then(|v| v.parse::<u16>().ok())
        {
            value
        } else {
            *opcodes
                .get(op)
                .with_context(|| format!("Unknown client opcode {op}"))?
        };
        instructions.push((opcode, operand.trim().to_owned()));
    }
    let mut out = Packet::new(text.len().max(64));
    let name: String =
        serde_json::from_str(&metadata.remove("name").context("Missing script name")?)?;
    out.pjstr(&name, 0);
    for (i, (op, operand)) in instructions.iter().enumerate() {
        out.p2(*op);
        if *op == 3 {
            let value: String = serde_json::from_str(operand)?;
            ensure!(!value.contains('\0'), "NUL in script string");
            out.pjstr(&value, 0);
        } else if *op == 54 {
            let value: i64 = operand.parse()?;
            out.pdata(&value.to_be_bytes(), 0, 8);
        } else {
            let value = if is_branch(*op) {
                if let Ok(offset) = operand.parse::<i64>() {
                    offset
                } else {
                    *labels
                        .get(operand)
                        .with_context(|| format!("Unknown label {operand}"))?
                        as i64
                        - i as i64
                        - 1
                }
            } else if *op == 51 {
                operand.strip_prefix("switch").unwrap_or(operand).parse()?
            } else if let Some(domain) = domain(*op) {
                names.resolve(domain, operand)?
            } else {
                operand.parse()?
            };
            if *op >= 150 || matches!(*op, 21 | 38 | 39) {
                out.p1(u8::try_from(value)?);
            } else {
                out.p4(i32::try_from(value)?);
            }
        }
    }
    out.p4(i32::try_from(instructions.len())?);
    for key in [
        "local_int_count",
        "local_string_count",
        "local_long_count",
        "int_param_count",
        "string_param_count",
        "long_param_count",
    ] {
        out.p2(metadata
            .remove(key)
            .with_context(|| format!("Missing {key}"))?
            .parse::<u16>()?);
    }
    if !metadata.is_empty() {
        bail!("Unknown script properties: {:?}", metadata.keys());
    }
    let start = out.pos;
    out.p1(u8::try_from(switches.len())?);
    for (table, base) in switches.into_iter().zip(switch_bases) {
        out.p2(u16::try_from(table.len())?);
        for (key, target) in table {
            let offset = if let Some(base) = &base {
                let from = labels
                    .get(base)
                    .with_context(|| format!("Unknown switch anchor {base}"))?;
                let to = labels
                    .get(&target)
                    .with_context(|| format!("Unknown switch target {target}"))?;
                i32::try_from(*to as i64 - *from as i64 - 1)?
            } else {
                target.parse()?
            };
            out.p4(key);
            out.p4(offset);
        }
    }
    out.p2(u16::try_from(out.pos - start)?);
    out.data.truncate(out.pos);
    Ok(out.data)
}
