use crate::pack::names::AssetNames;
use anyhow::{Context, Result, bail, ensure};

fn number(value: &str) -> Result<i64> {
    let value = value.trim();
    Ok(if let Some(hex) = value.strip_prefix("0x") {
        i64::from_str_radix(hex, 16)?
    } else {
        value.parse()?
    })
}

fn bytes(value: &str) -> Result<Vec<u8>> {
    ensure!(value.len().is_multiple_of(2), "Invalid texture field");
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect()
}

fn hex(data: &[u8]) -> String {
    data.iter().map(|v| format!("{v:02x}")).collect()
}

fn read(data: &[u8], pos: &mut usize, width: usize, signed: bool) -> Result<i64> {
    let slice = data
        .get(*pos..*pos + width)
        .context("Truncated texture field")?;
    *pos += width;
    let mut n = 0i64;
    for byte in slice {
        n = (n << 8) | i64::from(*byte);
    }
    if signed && n & (1 << (width * 8 - 1)) != 0 {
        n -= 1 << (width * 8);
    }
    Ok(n)
}

fn write(data: &mut Vec<u8>, value: i64, width: usize, signed: bool) -> Result<()> {
    let min = if signed { -(1 << (width * 8 - 1)) } else { 0 };
    let max = (1i64 << (width * 8 - usize::from(signed))) - 1;
    ensure!(
        (min..=max).contains(&value),
        "Texture value out of range: {value}"
    );
    data.extend_from_slice(&value.to_be_bytes()[8 - width..]);
    Ok(())
}

fn decode_field(key: &str, data: &[u8]) -> Result<String> {
    let mut pos = 0;
    let mut values = Vec::new();
    match key {
        "markers" => {
            let mode = read(data, &mut pos, 1, false)?;
            values.push(match mode {
                0 => "linear".into(),
                1 => "cosine".into(),
                2 => "cubic".into(),
                _ => mode.to_string(),
            });
            let count = read(data, &mut pos, 1, false)?;
            for _ in 0..count {
                values.push(format!(
                    "{},{}",
                    read(data, &mut pos, 2, false)?,
                    read(data, &mut pos, 2, false)?
                ));
            }
        }
        "stops" => {
            let preset = read(data, &mut pos, 1, false)?;
            values.push(if preset == 0 {
                "custom".into()
            } else {
                format!("preset_{preset}")
            });
            if preset == 0 {
                let count = read(data, &mut pos, 1, false)?;
                for _ in 0..count {
                    values.push(format!(
                        "{},0x{:06x}",
                        read(data, &mut pos, 2, false)?,
                        read(data, &mut pos, 3, false)?
                    ));
                }
            }
        }
        "persistence" => {
            while pos < data.len() {
                values.push(read(data, &mut pos, 2, true)?.to_string());
            }
        }
        "shapes" => {
            let count = read(data, &mut pos, 1, false)?;
            for _ in 0..count {
                let kind = read(data, &mut pos, 1, false)? as usize;
                let name = ["line", "bezier", "rectangle", "ellipse"]
                    .get(kind)
                    .context("Unknown texture shape")?;
                let mut fields = vec![name.to_string()];
                for _ in 0..if kind == 1 { 8 } else { 4 } {
                    fields.push(read(data, &mut pos, 2, true)?.to_string());
                }
                for _ in 0..if kind >= 2 { 2 } else { 1 } {
                    fields.push(format!("0x{:06x}", read(data, &mut pos, 3, false)?));
                }
                fields.push(read(data, &mut pos, 1, false)?.to_string());
                values.push(fields.join(","));
            }
        }
        _ => {
            pos = data.len();
            values.extend(data.iter().map(u8::to_string));
        }
    }
    ensure!(pos == data.len(), "Unread texture field data: {key}");
    Ok(values.join(";"))
}

fn encode_field(key: &str, value: &str) -> Result<Vec<u8>> {
    let values: Vec<_> = value.split(';').filter(|s| !s.is_empty()).collect();
    let mut data = Vec::new();
    match key {
        "markers" | "stops" => {
            let first = values.first().context("Missing texture curve mode")?;
            let mode = match *first {
                "linear" | "custom" => 0,
                "cosine" => 1,
                "cubic" => 2,
                _ => number(first.strip_prefix("preset_").unwrap_or(first))?,
            };
            write(&mut data, mode, 1, false)?;
            if key == "markers" || mode == 0 {
                write(&mut data, (values.len() - 1) as i64, 1, false)?;
                for value in &values[1..] {
                    let (a, b) = value.split_once(',').context("Expected position,value")?;
                    write(&mut data, number(a)?, 2, false)?;
                    write(
                        &mut data,
                        number(b)?,
                        if key == "markers" { 2 } else { 3 },
                        false,
                    )?;
                }
            } else {
                ensure!(
                    values.len() == 1,
                    "A gradient preset cannot have custom stops"
                );
            }
        }
        "persistence" => {
            for value in values {
                write(&mut data, number(value)?, 2, true)?;
            }
        }
        "shapes" => {
            write(&mut data, values.len() as i64, 1, false)?;
            for value in values {
                let fields: Vec<_> = value.split(',').collect();
                let kind = ["line", "bezier", "rectangle", "ellipse"]
                    .iter()
                    .position(|n| *n == fields[0])
                    .context("Unknown texture shape")?;
                write(&mut data, kind as i64, 1, false)?;
                let coordinates = if kind == 1 { 8 } else { 4 };
                let colours = if kind >= 2 { 2 } else { 1 };
                ensure!(
                    fields.len() == coordinates + colours + 2,
                    "Invalid texture shape fields"
                );
                for value in &fields[1..=coordinates] {
                    write(&mut data, number(value)?, 2, true)?;
                }
                for value in &fields[coordinates + 1..=coordinates + colours] {
                    write(&mut data, number(value)?, 3, false)?;
                }
                write(&mut data, number(fields.last().unwrap())?, 1, false)?;
            }
        }
        _ => {
            for value in values {
                write(&mut data, number(value)?, 1, false)?;
            }
        }
    }
    Ok(data)
}

pub fn convert(text: &str, names: &AssetNames, unpack: bool) -> Result<String> {
    let mut out = String::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            out.push_str(line);
            out.push('\n');
            continue;
        };
        let (key, value) = if key == "sprite" || key == "texture_id" {
            let kind = if key == "sprite" {
                "sprites"
            } else {
                "texture"
            };
            (
                key.to_owned(),
                if unpack {
                    names.name(kind, value.parse()?)
                } else {
                    names.id(kind, value)?.to_string()
                },
            )
        } else if [
            "markers",
            "stops",
            "shapes",
            "persistence",
            "trailing",
            "legacy_material",
        ]
        .contains(&key)
        {
            (
                if key == "trailing" {
                    "legacy_material"
                } else if key == "legacy_material" {
                    "trailing"
                } else {
                    key
                }
                .to_owned(),
                if unpack {
                    decode_field(key, &bytes(value)?)?
                } else {
                    hex(&encode_field(key, value)?)
                },
            )
        } else if key.starts_with("field_") && unpack {
            ensure!(value.is_empty(), "Undecoded texture opcode: {key}");
            ("ignored_opcode".to_owned(), key[6..].to_owned())
        } else if key == "ignored_opcode" && !unpack {
            let opcode: u8 = value.parse()?;
            (format!("field_{opcode}"), String::new())
        } else if key.starts_with("field_") {
            bail!("Use an editable texture property");
        } else {
            (key.to_owned(), value.to_owned())
        };
        out.push_str(&format!("{key}={value}\n"));
    }
    Ok(out)
}
