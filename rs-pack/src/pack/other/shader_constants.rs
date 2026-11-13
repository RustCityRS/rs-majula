use anyhow::{Context, Result, bail, ensure};
use std::collections::{BTreeMap, HashMap};

const CLASSES: &[&str] = &[
    "scalar",
    "vector",
    "matrix_rows",
    "matrix_columns",
    "object",
    "struct",
];
const TYPES: &[&str] = &[
    "void",
    "bool",
    "int",
    "float",
    "string",
    "texture",
    "texture1d",
    "texture2d",
    "texture3d",
    "texturecube",
    "sampler",
    "sampler1d",
    "sampler2d",
    "sampler3d",
    "samplercube",
    "pixelshader",
    "vertexshader",
    "pixelfragment",
    "vertexfragment",
];
const REGISTERS: &[&str] = &["b", "i", "c", "s"];

fn u16_at(data: &[u8], pos: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        data.get(pos..pos + 2)
            .context("Truncated shader constant table")?
            .try_into()?,
    ))
}
fn u32_at(data: &[u8], pos: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        data.get(pos..pos + 4)
            .context("Truncated shader constant table")?
            .try_into()?,
    ))
}
fn string_at(data: &[u8], pos: usize) -> Result<String> {
    let bytes = data.get(pos..).context("Invalid shader string offset")?;
    let length = bytes
        .iter()
        .position(|b| *b == 0)
        .context("Unterminated shader constant name")?;
    Ok(std::str::from_utf8(&bytes[..length])?.to_owned())
}

pub fn unpack(data: &[u8], version: u32) -> Result<String> {
    ensure!(
        u32_at(data, 0)? == 28 && u32_at(data, 8)? == version && u32_at(data, 16)? == 28,
        "Unsupported shader constant table header"
    );
    let count = u32_at(data, 12)? as usize;
    let flags = u32_at(data, 20)?;
    let mut strings = BTreeMap::new();
    for (name, offset) in [("creator", u32_at(data, 4)?), ("target", u32_at(data, 24)?)] {
        strings.insert(offset, (name.to_owned(), string_at(data, offset as usize)?));
    }
    let mut types = BTreeMap::new();
    let mut constants = Vec::new();
    for i in 0..count {
        let pos = 28 + i * 20;
        let offset = u32_at(data, pos)?;
        let name = string_at(data, offset as usize)?;
        strings.insert(offset, (name.clone(), name.clone()));
        let register = REGISTERS
            .get(u16_at(data, pos + 4)? as usize)
            .context("Unknown shader register set")?;
        let index = u16_at(data, pos + 6)?;
        let count = u16_at(data, pos + 8)?;
        let reserved = u16_at(data, pos + 10)?;
        let type_offset = u32_at(data, pos + 12)?;
        ensure!(
            u32_at(data, pos + 16)? == 0,
            "Shader constant defaults are not supported"
        );
        let mut values = Vec::new();
        for j in 0..6 {
            values.push(u16_at(data, type_offset as usize + j * 2)?);
        }
        ensure!(
            values[5] == 0 && u32_at(data, type_offset as usize + 12)? == 0,
            "Shader struct constants are not supported"
        );
        types.insert(type_offset, values);
        constants.push((
            name,
            format!("{register}{index}"),
            count,
            reserved,
            type_offset,
        ));
    }
    let type_names: HashMap<_, _> = types
        .keys()
        .enumerate()
        .map(|(i, &offset)| (offset, format!("type{i}")))
        .collect();
    let mut out = format!("ctab.flags={flags}\n");
    for (name, register, count, reserved, kind) in constants {
        out.push_str(&format!(
            "ctab.constant={name},{register},{count},{reserved},{}\n",
            type_names[&kind]
        ));
    }
    let mut blocks = Vec::new();
    for (offset, (label, value)) in strings {
        blocks.push((
            offset as usize,
            value.len() + 1,
            format!("ctab.string={label},{}\n", serde_json::to_string(&value)?),
        ));
    }
    for (offset, values) in types {
        let class = CLASSES
            .get(values[0] as usize)
            .context("Unknown shader constant class")?;
        let kind = TYPES
            .get(values[1] as usize)
            .context("Unknown shader constant type")?;
        let line = format!(
            "ctab.type={},{class},{kind},{},{},{}\n",
            type_names[&offset], values[2], values[3], values[4]
        );
        blocks.push((offset as usize, 16, line));
    }
    blocks.sort_by_key(|b| b.0);
    let mut pos = 28 + count * 20;
    for (offset, length, text) in blocks {
        ensure!(offset >= pos, "Overlapping shader constant table records");
        padding(data, pos, offset, &mut out)?;
        out.push_str(&text);
        pos = offset + length;
    }
    padding(data, pos, data.len(), &mut out)?;
    Ok(out)
}

fn padding(data: &[u8], start: usize, end: usize, out: &mut String) -> Result<()> {
    let padding = data
        .get(start..end)
        .context("Invalid shader constant table bounds")?;
    if let Some(&value) = padding.first() {
        ensure!(
            padding.iter().all(|b| *b == value),
            "Unrecognized shader constant table data"
        );
        out.push_str(&format!("ctab.padding={value},{}\n", padding.len()));
    }
    Ok(())
}

pub fn pack(text: &str, version: u32) -> Result<Vec<u8>> {
    let mut flags = None;
    let mut constants = Vec::new();
    let mut blocks = Vec::new();
    for line in text.lines() {
        let (key, value) = line
            .split_once('=')
            .context("Invalid shader constant table property")?;
        match key {
            "ctab.flags" => {
                ensure!(flags.is_none(), "Duplicate shader constant flags");
                flags = Some(value.parse::<u32>()?);
            }
            "ctab.constant" => constants.push(value.split(',').collect::<Vec<_>>()),
            "ctab.string" | "ctab.type" | "ctab.padding" => blocks.push((key, value)),
            _ => bail!("Unknown shader constant table property: {key}"),
        }
    }
    let mut offsets = HashMap::new();
    let mut out = vec![0; 28 + constants.len() * 20];
    for (key, value) in blocks {
        match key {
            "ctab.string" => {
                let (name, string) = value.split_once(',').context("Invalid shader string")?;
                ensure!(
                    offsets.insert(name, out.len() as u32).is_none(),
                    "Duplicate shader constant table name"
                );
                let string: String = serde_json::from_str(string)?;
                ensure!(!string.contains('\0'), "Shader strings cannot contain NUL");
                out.extend_from_slice(string.as_bytes());
                out.push(0);
            }
            "ctab.type" => {
                let fields: Vec<_> = value.split(',').collect();
                ensure!(fields.len() == 6, "Invalid shader constant type");
                ensure!(
                    offsets.insert(fields[0], out.len() as u32).is_none(),
                    "Duplicate shader constant table name"
                );
                let class = CLASSES
                    .iter()
                    .position(|v| *v == fields[1])
                    .context("Unknown shader constant class")? as u16;
                let kind = TYPES
                    .iter()
                    .position(|v| *v == fields[2])
                    .context("Unknown shader constant type")? as u16;
                for value in [
                    class,
                    kind,
                    fields[3].parse()?,
                    fields[4].parse()?,
                    fields[5].parse()?,
                    0,
                ] {
                    out.extend_from_slice(&value.to_le_bytes());
                }
                out.extend_from_slice(&0u32.to_le_bytes());
            }
            "ctab.padding" => {
                let (value, count) = value.split_once(',').context("Invalid shader padding")?;
                let count: usize = count.parse()?;
                ensure!(count <= 1_000_000, "Shader padding is too large");
                out.extend(std::iter::repeat_n(value.parse::<u8>()?, count));
            }
            _ => unreachable!(),
        }
    }
    let offset = |name: &str| -> Result<u32> {
        offsets
            .get(name)
            .copied()
            .with_context(|| format!("Unknown shader constant table name: {name}"))
    };
    let header = [
        28,
        offset("creator")?,
        version,
        constants.len() as u32,
        28,
        flags.context("Missing shader constant flags")?,
        offset("target")?,
    ];
    for (i, value) in header.into_iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    for (i, fields) in constants.iter().enumerate() {
        ensure!(fields.len() == 5, "Invalid shader constant");
        let pos = 28 + i * 20;
        out[pos..pos + 4].copy_from_slice(&offset(fields[0])?.to_le_bytes());
        let register = fields[1];
        let prefix = register
            .get(..1)
            .context("Missing shader constant register")?;
        let set = REGISTERS
            .iter()
            .position(|v| *v == prefix)
            .context("Unknown shader register set")? as u16;
        for (j, value) in [
            set,
            register[1..].parse::<u16>()?,
            fields[2].parse()?,
            fields[3].parse()?,
        ]
        .into_iter()
        .enumerate()
        {
            out[pos + 4 + j * 2..pos + 6 + j * 2].copy_from_slice(&value.to_le_bytes());
        }
        out[pos + 12..pos + 16].copy_from_slice(&offset(fields[4])?.to_le_bytes());
    }
    Ok(out)
}
