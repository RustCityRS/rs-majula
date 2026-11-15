use crate::pack::names::AssetNames;
use crate::pack::packed_data::PackedData;
#[cfg(since_742)]
use crate::pack::util::parse_colour;
#[cfg(since_742)]
use crate::types::WearPos;
use anyhow::{Context, Result, bail, ensure};
use rs_io::Packet;
#[cfg(since_742)]
use rs_ver::TypeId;
use std::fmt::Write;

const GROUP_MAP: u32 = 1;
const GROUP_GRAPHICS: u32 = 3;
#[cfg(since_742)]
const GROUP_AUDIO: u32 = 4;
const GROUP_WEARPOS: u32 = 6;
#[cfg(since_742)]
const GROUP_MINIMENU: u32 = 7;
#[cfg(since_742)]
const GROUP_CUTSCENE: u32 = 8;
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
            (GROUP_MAP, 1) => {
                let textures: Vec<_> = (0..6)
                    .map(|_| names.name("texture", buf.g2() as u32))
                    .collect();
                writeln!(out, "environmentmap={}", textures.join(","))?;
            }
            // 4 and 5
            (GROUP_MAP, 4 | 5) => {
                let enums: Vec<_> = (0..buf.g1())
                    .map(|_| names.name("enum", buf.g2() as u32))
                    .collect();
                let key = if code == 4 { "titles" } else { "femaletitles" };
                writeln!(out, "{key}={}", enums.join(","))?;
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
            (GROUP_CUTSCENE, 1) => {
                writeln!(out, "cancel={}", unpack_input(&mut buf)?)?;
            }
            #[cfg(since_742)]
            (GROUP_MINIMENU, 1 | 3 | 4) => {
                let key = match code {
                    1 => "primaryclick",
                    3 => "tertiaryclick",
                    _ => "selectclick",
                };
                writeln!(out, "{key}={}", unpack_input(&mut buf)?)?;
            }
            #[cfg(since_742)]
            (GROUP_MINIMENU, 6 | 7) => {
                let key = if code == 6 {
                    "secondarymodifier"
                } else {
                    "tertiarymodifier"
                };
                writeln!(out, "{key}={}", unpack_keys(&mut buf))?;
            }
            #[cfg(since_742)]
            (GROUP_MINIMENU, 12 | 13) => {
                let key = if code == 12 {
                    "membersobjectcolour"
                } else {
                    "freeobjectcolour"
                };
                writeln!(out, "{key}=0x{:06x}", buf.g4s())?;
            }
            #[cfg(since_742)]
            (GROUP_AUDIO, 1) => {
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
                let value = unpack_field(&mut buf, field(group, code).unwrap())?;
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
            (GROUP_MAP, "environmentmap") => {
                out.p1(1);
                let textures = list
                    .map(|texture| names.id("texture", texture))
                    .collect::<Result<Vec<_>>>()?;
                ensure!(textures.len() == 6, "environmentmap takes six textures");
                for texture in textures {
                    out.p2(texture as u16);
                }
            }
            (GROUP_MAP, "titles" | "femaletitles") => {
                out.p1(if key == "titles" { 4 } else { 5 });
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
            (GROUP_CUTSCENE, "cancel") => {
                out.p1(1);
                pack_input(&mut out, value)?;
            }
            #[cfg(since_742)]
            (GROUP_MINIMENU, "primaryclick" | "tertiaryclick" | "selectclick") => {
                out.p1(match key {
                    "primaryclick" => 1,
                    "tertiaryclick" => 3,
                    _ => 4,
                });
                pack_input(&mut out, value)?;
            }
            #[cfg(since_742)]
            (GROUP_MINIMENU, "secondarymodifier" | "tertiarymodifier") => {
                out.p1(if key == "secondarymodifier" { 6 } else { 7 });
                pack_bytes(&mut out, &parse_keys(list)?);
            }
            #[cfg(since_742)]
            (GROUP_MINIMENU, "membersobjectcolour" | "freeobjectcolour") => {
                out.p1(if key == "membersobjectcolour" { 12 } else { 13 });
                out.p4(parse_colour(value));
            }
            #[cfg(since_742)]
            (GROUP_AUDIO, "titlescreensong") => {
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
    ByteList,
    Value,
}

#[cfg(since_742)]
fn field(group: u32, code: u8) -> Option<Field> {
    Some(match (group, code) {
        (GROUP_GRAPHICS, 4 | 8 | 10) | (GROUP_MINIMENU, 11) => Field::Flag,
        (GROUP_GRAPHICS, 9 | 11 | 13..=15) => Field::U8,
        (GROUP_MINIMENU, 5) => Field::ByteList,
        (GROUP_MINIMENU, 2 | 8..=10) => Field::Value,
        _ => return None,
    })
}

#[cfg(since_742)]
fn field_key(group: u32, key: &str) -> Option<(u8, Field)> {
    let code = key.strip_prefix("unknown")?.parse().ok()?;
    Some((code, field(group, code)?))
}

#[cfg(since_742)]
fn unpack_field(buf: &mut Packet, kind: Field) -> Result<String> {
    Ok(match kind {
        Field::Flag => "yes".to_owned(),
        Field::U8 => buf.g1().to_string(),
        Field::ByteList => unpack_bytes(buf),
        Field::Value => unpack_input(buf)?,
    })
}

#[cfg(since_742)]
fn pack_field(out: &mut PackedData, kind: Field, value: &str) -> Result<()> {
    match kind {
        Field::Flag => {}
        Field::U8 => out.p1(value.parse()?),
        Field::ByteList => pack_bytes(out, &parse_bytes(value.split(','))?),
        Field::Value => pack_input(out, value)?,
    }
    Ok(())
}

/// An input is `mouse,mouse_<n>,<n>` then its keys, `keypress,key_<n>,<n>`,
/// or tag 2 and its numbers.
#[cfg(since_742)]
fn unpack_input(buf: &mut Packet) -> Result<String> {
    Ok(match buf.g1() {
        0 => {
            let mut input = format!("mouse,mouse_{},{}", buf.g1(), buf.g1());
            let keys = unpack_keys(buf);
            if !keys.is_empty() {
                write!(input, ",{keys}")?;
            }
            input
        }
        1 => format!("keypress,key_{},{}", buf.g1(), buf.g1()),
        2 => format!("2,{}", unpack_bytes(buf)),
        tag => bail!("Unknown input tag {tag}"),
    })
}

#[cfg(since_742)]
fn pack_input(out: &mut PackedData, value: &str) -> Result<()> {
    let parts: Vec<_> = value.split(',').collect();
    match parts.as_slice() {
        ["mouse", button, arg, keys @ ..] => {
            out.p1(0);
            out.p1(numbered(button, "mouse_")?);
            out.p1(arg.parse()?);
            pack_bytes(out, &parse_keys(keys.iter().copied())?);
        }
        ["keypress", key, arg] => {
            out.p1(1);
            out.p1(numbered(key, "key_")?);
            out.p1(arg.parse()?);
        }
        ["2", items @ ..] => {
            out.p1(2);
            pack_bytes(out, &parse_bytes(items.iter().copied())?);
        }
        _ => bail!("Invalid input {value}"),
    }
    Ok(())
}

#[cfg(since_742)]
fn unpack_bytes(buf: &mut Packet) -> String {
    (0..buf.g1())
        .map(|_| buf.g1().to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(since_742)]
fn unpack_keys(buf: &mut Packet) -> String {
    (0..buf.g1())
        .map(|_| format!("key_{}", buf.g1()))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(since_742)]
fn parse_bytes<'a>(values: impl IntoIterator<Item = &'a str>) -> Result<Vec<u8>> {
    values
        .into_iter()
        .filter(|value| !value.is_empty())
        .map(|value| Ok(value.parse()?))
        .collect()
}

#[cfg(since_742)]
fn parse_keys<'a>(keys: impl IntoIterator<Item = &'a str>) -> Result<Vec<u8>> {
    keys.into_iter()
        .filter(|key| !key.is_empty())
        .map(|key| numbered(key, "key_"))
        .collect()
}

#[cfg(since_742)]
fn numbered(value: &str, prefix: &str) -> Result<u8> {
    let number = value
        .strip_prefix(prefix)
        .with_context(|| format!("Expected {prefix}<n>, got {value}"))?;
    Ok(number.parse()?)
}

#[cfg(since_742)]
fn pack_bytes(out: &mut PackedData, values: &[u8]) {
    out.p1(values.len() as u8);
    for &value in values {
        out.p1(value);
    }
}
