use crate::pack::names::AssetNames;
use crate::pack::packed_data::PackedData;
#[cfg(since_742)]
use crate::types::WearPos;
use anyhow::{Context, Result, bail, ensure};
use rs_io::Packet;
#[cfg(since_742)]
use rs_ver::TypeId;
use std::fmt::Write;

const GROUP_UNKNOWN1: u32 = 1;
const GROUP_GRAPHICS: u32 = 3;
#[cfg(since_742)]
const GROUP_UNKNOWN4: u32 = 4;
const GROUP_WEARPOS: u32 = 6;
#[cfg(since_742)]
const GROUP_UNKNOWN7: u32 = 7;
#[cfg(since_742)]
const GROUP_UNKNOWN8: u32 = 8;
/// How many lists the graphics group's opcode 7 holds.
#[cfg(since_742)]
const SPLAT_SETS: usize = 10 << 2;
/// How many hitmarks a graphics group holds offsets for when it doesn't say.
const DEFAULT_HITMARKS: usize = 4;

pub fn unpack(group: u32, data: &[u8], names: &AssetNames) -> Result<String> {
    let mut buf = Packet::from(data.to_vec());
    let mut out = String::new();
    let mut hitmarks = DEFAULT_HITMARKS;
    loop {
        let code = buf.g1();
        match (group, code) {
            (_, 0) => break,
            // 1
            (GROUP_UNKNOWN1, 1) => {
                let textures: Vec<_> = (0..6)
                    .map(|_| names.name("texture", buf.g2() as u32))
                    .collect();
                writeln!(out, "unknown1={}", textures.join(","))?;
            }
            // 4 and 5
            (GROUP_UNKNOWN1, 4 | 5) => {
                let enums: Vec<_> = (0..buf.g1())
                    .map(|_| names.name("enum", buf.g2() as u32))
                    .collect();
                writeln!(out, "unknown{code}={}", enums.join(","))?;
            }
            // 1
            (GROUP_GRAPHICS, 1) => {
                for i in 0..hitmarks {
                    writeln!(out, "hitmarkoffset{i}={},{}", buf.g2s(), buf.g2s())?;
                }
            }
            // 2
            (GROUP_GRAPHICS, 2) => {
                #[cfg(before_742)]
                let model = buf.g2() as i64;
                #[cfg(since_742)]
                let model = buf.gsmart2or4null() as i64;
                writeln!(
                    out,
                    "performancemetricsmodel={}",
                    names.reference("model", model)
                )?;
            }
            // 3
            (GROUP_GRAPHICS, 3) => {
                hitmarks = buf.g1() as usize;
                writeln!(out, "maxhitmarks={hitmarks}")?;
            }
            // 1
            (GROUP_WEARPOS, 1) => {
                let types: Vec<_> = (0..buf.g1()).map(|_| buf.g1().to_string()).collect();
                writeln!(out, "wearpostype={}", types.join(","))?;
            }
            #[cfg(since_742)]
            (GROUP_WEARPOS, 3 | 4) => {
                let key = if code == 3 {
                    "replaceheldleft"
                } else {
                    "replaceheldright"
                };
                writeln!(out, "{key}={}", WearPos::try_from(buf.g1())?.config_str())?;
            }
            #[cfg(since_742)]
            (GROUP_WEARPOS, 5 | 6) => {
                let key = if code == 5 {
                    "replaceheldleftreset"
                } else {
                    "replaceheldrightreset"
                };
                let slots = (0..buf.g1())
                    .map(|_| Ok(WearPos::try_from(buf.g1())?.config_str()))
                    .collect::<Result<Vec<_>>>()?;
                writeln!(out, "{key}={}", slots.join(","))?;
            }
            #[cfg(since_742)]
            (GROUP_GRAPHICS, 7) => {
                for _ in 0..SPLAT_SETS {
                    write!(out, "unknown7={}", buf.g2())?;
                    for _ in 0..buf.g2() {
                        write!(out, ",{}", buf.g2())?;
                    }
                    out.push('\n');
                }
            }
            #[cfg(since_742)]
            (_, code) if field(group, code).is_some() => {
                let value = unpack_field(&mut buf, field(group, code).unwrap());
                writeln!(out, "unknown{code}={value}")?;
            }
            _ => bail!("Unknown defaults {group} opcode {code}"),
        }
    }
    ensure!(
        buf.remaining() == 0,
        "Defaults {group} has {} bytes left over",
        buf.remaining()
    );
    Ok(out)
}

pub fn pack(group: u32, text: &str, names: &AssetNames) -> Result<Vec<u8>> {
    let mut out = PackedData::default();
    let mut offsets = false;
    #[cfg(since_742)]
    let mut splats = 0;
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .with_context(|| format!("Invalid line: {line}"))?;
        let list = value.split(',');
        match (group, key) {
            (GROUP_UNKNOWN1, "unknown1") => {
                out.p1(1);
                let textures = list
                    .map(|texture| names.id("texture", texture))
                    .collect::<Result<Vec<_>>>()?;
                ensure!(textures.len() == 6, "unknown1 takes six textures");
                for texture in textures {
                    out.p2(texture as u16);
                }
            }
            (GROUP_UNKNOWN1, "unknown4" | "unknown5") => {
                out.p1(if key == "unknown4" { 4 } else { 5 });
                let enums = list
                    .map(|id| names.id("enum", id))
                    .collect::<Result<Vec<_>>>()?;
                out.p1(enums.len() as u8);
                for id in enums {
                    out.p2(id as u16);
                }
            }
            (GROUP_GRAPHICS, "maxhitmarks") => {
                out.p1(3);
                out.p1(value.parse()?);
            }
            (GROUP_GRAPHICS, "performancemetricsmodel") => {
                out.p1(2);
                #[cfg(before_742)]
                out.p2(names.id("model", value)? as u16);
                #[cfg(since_742)]
                out.pbigid(TypeId::from_int(names.resolve("model", value)? as i32));
            }
            #[cfg(since_742)]
            (GROUP_GRAPHICS, "unknown7") => {
                if splats % SPLAT_SETS == 0 {
                    out.p1(7);
                }
                splats += 1;
                let values = list
                    .map(str::parse)
                    .collect::<std::result::Result<Vec<u16>, _>>()?;
                out.p2(values[0]);
                out.p2(values.len() as u16 - 1);
                for &value in &values[1..] {
                    out.p2(value);
                }
            }
            (GROUP_GRAPHICS, _) if key.starts_with("hitmarkoffset") => {
                if !offsets {
                    out.p1(1);
                    offsets = true;
                }
                let (x, y) = value.split_once(',').context("Expected x,y")?;
                out.p2(x.parse::<i16>()? as u16);
                out.p2(y.parse::<i16>()? as u16);
            }
            (GROUP_WEARPOS, "wearpostype") => {
                out.p1(1);
                let types = list
                    .map(str::parse)
                    .collect::<std::result::Result<Vec<u8>, _>>()?;
                out.p1(types.len() as u8);
                for ty in types {
                    out.p1(ty);
                }
            }
            #[cfg(since_742)]
            (GROUP_WEARPOS, "replaceheldleft" | "replaceheldright") => {
                out.p1(if key == "replaceheldleft" { 3 } else { 4 });
                out.p1(WearPos::from_config_str(value) as u8);
            }
            #[cfg(since_742)]
            (GROUP_WEARPOS, "replaceheldleftreset" | "replaceheldrightreset") => {
                out.p1(if key == "replaceheldleftreset" { 5 } else { 6 });
                let slots: Vec<_> = list
                    .filter(|slot| !slot.is_empty())
                    .map(|slot| WearPos::from_config_str(slot) as u8)
                    .collect();
                out.p1(slots.len() as u8);
                for slot in slots {
                    out.p1(slot);
                }
            }
            #[cfg(since_742)]
            (_, _) if field_key(group, key).is_some() => {
                let (code, kind) = field_key(group, key).unwrap();
                out.p1(code);
                pack_field(&mut out, kind, value)?;
            }
            _ => bail!("Unknown defaults {group} property {key}"),
        }
    }
    #[cfg(since_742)]
    ensure!(
        splats % SPLAT_SETS == 0,
        "unknown7 takes {SPLAT_SETS} lines"
    );
    out.p1(0);
    Ok(out.dat)
}

/// How a 742 defaults opcode without a name of its own holds its value.
#[cfg(since_742)]
#[derive(Clone, Copy)]
enum Field {
    Flag,
    U8,
    U16,
    U24,
    I32,
    U16Pair,
    ByteList,
    Value,
}

#[cfg(since_742)]
fn field(group: u32, code: u8) -> Option<Field> {
    Some(match (group, code) {
        (GROUP_GRAPHICS, 4 | 8 | 10) | (GROUP_UNKNOWN7, 11) => Field::Flag,
        (GROUP_GRAPHICS, 9 | 11 | 13..=15) => Field::U8,
        (GROUP_UNKNOWN4, 1) => Field::U16,
        (GROUP_GRAPHICS, 5 | 6) => Field::U24,
        (GROUP_UNKNOWN7, 12 | 13) => Field::I32,
        (GROUP_GRAPHICS, 12) => Field::U16Pair,
        (GROUP_UNKNOWN7, 5..=7) => Field::ByteList,
        (GROUP_UNKNOWN7, 1..=4 | 8..=10) | (GROUP_UNKNOWN8, 1) => Field::Value,
        _ => return None,
    })
}

#[cfg(since_742)]
fn field_key(group: u32, key: &str) -> Option<(u8, Field)> {
    let code = key.strip_prefix("unknown")?.parse().ok()?;
    Some((code, field(group, code)?))
}

#[cfg(since_742)]
fn unpack_field(buf: &mut Packet, kind: Field) -> String {
    let list = |buf: &mut Packet| {
        (0..buf.g1())
            .map(|_| buf.g1().to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    match kind {
        Field::Flag => "yes".to_owned(),
        Field::U8 => buf.g1().to_string(),
        Field::U16 => buf.g2().to_string(),
        Field::U24 => buf.g3().to_string(),
        Field::I32 => buf.g4s().to_string(),
        Field::U16Pair => format!("{},{}", buf.g2(), buf.g2()),
        Field::ByteList => list(buf),
        Field::Value => {
            let tag = buf.g1();
            let mut parts = vec![tag.to_string()];
            if tag != 2 {
                parts.push(buf.g1().to_string());
                parts.push(buf.g1().to_string());
            }
            if tag != 1 {
                let items = list(buf);
                parts.push(items);
            }
            parts.join(",")
        }
    }
}

#[cfg(since_742)]
fn pack_field(out: &mut PackedData, kind: Field, value: &str) -> Result<()> {
    let numbers = value
        .split(',')
        .filter(|part| !part.is_empty())
        .map(str::parse)
        .collect::<std::result::Result<Vec<i64>, _>>()?;
    let list = |out: &mut PackedData, values: &[i64]| {
        out.p1(values.len() as u8);
        for &value in values {
            out.p1(value as u8);
        }
    };
    match kind {
        Field::Flag => {}
        Field::U8 => out.p1(numbers[0] as u8),
        Field::U16 => out.p2(numbers[0] as u16),
        Field::U24 => out.p3(numbers[0] as i32),
        Field::I32 => out.p4(numbers[0] as i32),
        Field::U16Pair => {
            out.p2(numbers[0] as u16);
            out.p2(numbers[1] as u16);
        }
        Field::ByteList => list(out, &numbers),
        Field::Value => {
            let tag = *numbers.first().context("Expected a value tag")? as u8;
            out.p1(tag);
            match tag {
                0 => {
                    out.p1(numbers[1] as u8);
                    out.p1(numbers[2] as u8);
                    list(out, &numbers[3..]);
                }
                1 => {
                    out.p1(numbers[1] as u8);
                    out.p1(numbers[2] as u8);
                }
                2 => list(out, &numbers[1..]),
                _ => bail!("Unknown value tag {tag}"),
            }
        }
    }
    Ok(())
}
