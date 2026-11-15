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
/// How many player recolours the graphics group holds: ten sets of four slots.
#[cfg(since_742)]
const PLAYER_RECOLS: usize = 10 << 2;
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
            (GROUP_GRAPHICS, 5 | 6) => {
                let key = if code == 5 {
                    "titleinterface"
                } else {
                    "lobbyinterface"
                };
                writeln!(out, "{key}={}", names.name("interface", buf.g3() as u32))?;
            }
            #[cfg(since_742)]
            (GROUP_GRAPHICS, 7) => {
                for i in 0..PLAYER_RECOLS {
                    let (set, slot) = (i >> 2, i & 3);
                    let source = buf.g2();
                    let dests: Vec<_> = (0..buf.g2()).map(|_| buf.g2().to_string()).collect();
                    if source != u16::MAX || !dests.is_empty() {
                        writeln!(out, "playerrecol{set}s{slot}={source}")?;
                        writeln!(out, "playerrecol{set}d{slot}={}", dests.join(","))?;
                    }
                }
            }
            #[cfg(since_742)]
            (GROUP_UNKNOWN4, 1) => {
                writeln!(
                    out,
                    "titlescreensong={}",
                    names.name("songs", buf.g2() as u32)
                )?;
            }
            #[cfg(since_742)]
            (GROUP_GRAPHICS, 12) => {
                writeln!(out, "initialwindowsize={},{}", buf.g2(), buf.g2())?;
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
    let mut recols = false;
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
            (GROUP_GRAPHICS, "titleinterface" | "lobbyinterface") => {
                out.p1(if key == "titleinterface" { 5 } else { 6 });
                out.p3(names.id("interface", value)? as i32);
            }
            #[cfg(since_742)]
            (GROUP_GRAPHICS, _) if key.starts_with("playerrecol") => {
                if !recols {
                    recols = true;
                    out.p1(7);
                    for (source, dests) in player_recols(text)? {
                        out.p2(source);
                        out.p2(dests.len() as u16);
                        for dest in dests {
                            out.p2(dest);
                        }
                    }
                }
            }
            #[cfg(since_742)]
            (GROUP_UNKNOWN4, "titlescreensong") => {
                out.p1(1);
                out.p2(names.id("songs", value)? as u16);
            }
            #[cfg(since_742)]
            (GROUP_GRAPHICS, "initialwindowsize") => {
                out.p1(12);
                let (width, height) = value.split_once(',').context("Expected width,height")?;
                out.p2(width.parse()?);
                out.p2(height.parse()?);
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
    out.p1(0);
    Ok(out.dat)
}

/// Every `playerrecol<set>s<slot>` source and `playerrecol<set>d<slot>`
/// destination list; a slot without lines has no source and no destinations.
#[cfg(since_742)]
fn player_recols(text: &str) -> Result<Vec<(u16, Vec<u16>)>> {
    let mut recols = vec![(u16::MAX, Vec::new()); PLAYER_RECOLS];
    for (key, value) in text.lines().filter_map(|line| line.split_once('=')) {
        let Some(rest) = key.strip_prefix("playerrecol") else {
            continue;
        };
        let at = rest
            .find(['s', 'd'])
            .with_context(|| format!("Invalid key {key}"))?;
        let set: usize = rest[..at].parse()?;
        let slot: usize = rest[at + 1..].parse()?;
        ensure!(set < PLAYER_RECOLS >> 2 && slot < 4, "Invalid key {key}");
        let recol = &mut recols[set << 2 | slot];
        if rest.as_bytes()[at] == b's' {
            recol.0 = value.parse()?;
        } else {
            recol.1 = value
                .split(',')
                .filter(|dest| !dest.is_empty())
                .map(str::parse)
                .collect::<std::result::Result<_, _>>()?;
        }
    }
    Ok(recols)
}

/// How a 742 defaults opcode without a name of its own holds its value.
#[cfg(since_742)]
#[derive(Clone, Copy)]
enum Field {
    Flag,
    U8,
    I32,
    ByteList,
    Value,
}

#[cfg(since_742)]
fn field(group: u32, code: u8) -> Option<Field> {
    Some(match (group, code) {
        (GROUP_GRAPHICS, 4 | 8 | 10) | (GROUP_UNKNOWN7, 11) => Field::Flag,
        (GROUP_GRAPHICS, 9 | 11 | 13..=15) => Field::U8,
        (GROUP_UNKNOWN7, 12 | 13) => Field::I32,
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
        Field::I32 => buf.g4s().to_string(),
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
        Field::I32 => out.p4(numbers[0] as i32),
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
