use crate::pack::names::AssetNames;
use crate::pack::packed_data::PackedData;
#[cfg(since_742)]
use crate::pack::util::parse_colour;
#[cfg(since_742)]
use crate::types::WearPos;
#[cfg(since_742)]
use anyhow::bail;
use anyhow::{Context, Result, ensure};
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
/// How many hitmarks a graphics group holds offsets for when it doesn't say.
const DEFAULT_HITMARKS: usize = 4;
/// How many player recolours the graphics group holds: ten sets of four slots.
#[cfg(since_742)]
const PLAYER_RECOLS: usize = 10 << 2;

/// How a defaults property holds its value.
#[derive(Clone, Copy)]
enum Kind {
    Byte,
    Bytes,
    Textures,
    Enums,
    Model,
    #[cfg(since_742)]
    Flag,
    #[cfg(since_742)]
    Interface,
    #[cfg(since_742)]
    Song,
    #[cfg(since_742)]
    Size,
    #[cfg(since_742)]
    Colour,
    #[cfg(since_742)]
    WearPos,
    #[cfg(since_742)]
    WearPosList,
    #[cfg(since_742)]
    Input,
    #[cfg(since_742)]
    Keys,
}

/// A property's group, opcode, key and kind. Hitmark offsets and player
/// recolours span several lines, so `unpack` and `pack` handle them.
type Property = (u32, u8, &'static str, Kind);

const PROPERTIES: &[Property] = &[
    (GROUP_MAP, 1, "environmentmap", Kind::Textures),
    (GROUP_MAP, 4, "titles", Kind::Enums),
    (GROUP_MAP, 5, "femaletitles", Kind::Enums),
    (GROUP_GRAPHICS, 2, "performancemetricsmodel", Kind::Model),
    (GROUP_GRAPHICS, 3, "maxhitmarks", Kind::Byte),
    (GROUP_WEARPOS, 1, "wearpostype", Kind::Bytes),
];

#[cfg(before_742)]
const PROPERTIES_742: &[Property] = &[];

#[cfg(since_742)]
const PROPERTIES_742: &[Property] = &[
    (GROUP_GRAPHICS, 4, "unknown4", Kind::Flag),
    (GROUP_GRAPHICS, 5, "titleinterface", Kind::Interface),
    (GROUP_GRAPHICS, 6, "lobbyinterface", Kind::Interface),
    (GROUP_GRAPHICS, 8, "unknown8", Kind::Flag),
    (GROUP_GRAPHICS, 9, "npcchattimeout", Kind::Byte),
    (GROUP_GRAPHICS, 10, "unknown10", Kind::Flag),
    (GROUP_GRAPHICS, 11, "unknown11", Kind::Byte),
    (GROUP_GRAPHICS, 12, "initialwindowsize", Kind::Size),
    (GROUP_GRAPHICS, 13, "maxheadbarssimultaneous", Kind::Byte),
    (GROUP_GRAPHICS, 14, "maxheadbarsqueued", Kind::Byte),
    (GROUP_GRAPHICS, 15, "unknown15", Kind::Byte),
    (GROUP_AUDIO, 1, "titlescreensong", Kind::Song),
    (GROUP_WEARPOS, 3, "replaceheldleft", Kind::WearPos),
    (GROUP_WEARPOS, 4, "replaceheldright", Kind::WearPos),
    (GROUP_WEARPOS, 5, "replaceheldleftreset", Kind::WearPosList),
    (GROUP_WEARPOS, 6, "replaceheldrightreset", Kind::WearPosList),
    (GROUP_MINIMENU, 1, "primaryclick", Kind::Input),
    (GROUP_MINIMENU, 2, "unknown2", Kind::Input),
    (GROUP_MINIMENU, 3, "tertiaryclick", Kind::Input),
    (GROUP_MINIMENU, 4, "selectclick", Kind::Input),
    (GROUP_MINIMENU, 5, "unknown5", Kind::Bytes),
    (GROUP_MINIMENU, 6, "secondarymodifier", Kind::Keys),
    (GROUP_MINIMENU, 7, "tertiarymodifier", Kind::Keys),
    (GROUP_MINIMENU, 8, "unknown8", Kind::Input),
    (GROUP_MINIMENU, 9, "unknown9", Kind::Input),
    (GROUP_MINIMENU, 10, "unknown10", Kind::Input),
    (GROUP_MINIMENU, 11, "unknown11", Kind::Flag),
    (GROUP_MINIMENU, 12, "membersobjectcolour", Kind::Colour),
    (GROUP_MINIMENU, 13, "freeobjectcolour", Kind::Colour),
    (GROUP_CUTSCENE, 1, "cancel", Kind::Input),
];

pub fn unpack(group: u32, data: &[u8], names: &AssetNames) -> Result<String> {
    let mut buf = Packet::from(data.to_vec());
    let mut out = String::new();
    let mut hitmarks = DEFAULT_HITMARKS;
    loop {
        let code = buf.g1();
        match (group, code) {
            (_, 0) => break,
            (GROUP_GRAPHICS, 1) => {
                for i in 0..hitmarks {
                    writeln!(out, "hitmarkoffset{i}={},{}", buf.g2s(), buf.g2s())?;
                }
            }
            (GROUP_GRAPHICS, 3) => {
                hitmarks = buf.g1() as usize;
                writeln!(out, "maxhitmarks={hitmarks}")?;
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
            _ => {
                let (key, kind) = property(group, code)
                    .with_context(|| format!("Unknown defaults {group} opcode {code}"))?;
                writeln!(out, "{key}={}", unpack_value(&mut buf, kind, names)?)?;
            }
        }
    }
    ensure!(
        buf.get_space() == 0,
        "Defaults {group} has {} bytes left over",
        buf.get_space()
    );
    Ok(out)
}

pub fn pack(group: u32, text: &str, names: &AssetNames) -> Result<Vec<u8>> {
    let mut out = PackedData::default();
    let mut offsets = false;
    #[cfg(since_742)]
    let mut recols = false;
    for line in text.lines().filter(|line| !line.is_empty()) {
        let (key, value) = line
            .split_once('=')
            .with_context(|| format!("Invalid line: {line}"))?;
        match (group, key) {
            (GROUP_GRAPHICS, _) if key.starts_with("hitmarkoffset") => {
                if !offsets {
                    offsets = true;
                    out.p1(1);
                }
                let (x, y) = value.split_once(',').context("Expected x,y")?;
                out.p2(x.parse::<i16>()? as u16);
                out.p2(y.parse::<i16>()? as u16);
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
            _ => {
                let (code, kind) = property_code(group, key)
                    .with_context(|| format!("Unknown defaults {group} property {key}"))?;
                out.p1(code);
                pack_value(&mut out, kind, value, names)?;
            }
        }
    }
    out.p1(0);
    Ok(out.dat)
}

fn property(group: u32, code: u8) -> Option<(&'static str, Kind)> {
    PROPERTIES
        .iter()
        .chain(PROPERTIES_742)
        .find(|&&(g, c, ..)| g == group && c == code)
        .map(|&(_, _, key, kind)| (key, kind))
}

fn property_code(group: u32, key: &str) -> Option<(u8, Kind)> {
    PROPERTIES
        .iter()
        .chain(PROPERTIES_742)
        .find(|&&(g, _, k, _)| g == group && k == key)
        .map(|&(_, code, _, kind)| (code, kind))
}

fn unpack_value(buf: &mut Packet, kind: Kind, names: &AssetNames) -> Result<String> {
    Ok(match kind {
        Kind::Byte => buf.g1().to_string(),
        Kind::Bytes => unpack_list(buf, |buf| Ok(buf.g1().to_string()))?,
        Kind::Textures => {
            let textures: Vec<_> = (0..6)
                .map(|_| names.name("texture", buf.g2() as u32))
                .collect();
            textures.join(",")
        }
        Kind::Enums => unpack_list(buf, |buf| Ok(names.name("enum", buf.g2() as u32)))?,
        Kind::Model => {
            #[cfg(before_742)]
            let model = buf.g2() as i64;
            #[cfg(since_742)]
            let model = buf.gsmart2or4s() as i64;
            names.reference("model", model)
        }
        #[cfg(since_742)]
        Kind::Flag => "yes".to_owned(),
        #[cfg(since_742)]
        Kind::Interface => names.name("interface", buf.g3() as u32),
        #[cfg(since_742)]
        Kind::Song => names.name("songs", buf.g2() as u32),
        #[cfg(since_742)]
        Kind::Size => format!("{},{}", buf.g2(), buf.g2()),
        #[cfg(since_742)]
        Kind::Colour => format!("0x{:06x}", buf.g4s()),
        #[cfg(since_742)]
        Kind::WearPos => WearPos::try_from(buf.g1())?.config_str().to_owned(),
        #[cfg(since_742)]
        Kind::WearPosList => unpack_list(buf, |buf| {
            Ok(WearPos::try_from(buf.g1())?.config_str().to_owned())
        })?,
        #[cfg(since_742)]
        Kind::Input => unpack_input(buf)?,
        #[cfg(since_742)]
        Kind::Keys => unpack_list(buf, |buf| Ok(format!("key_{}", buf.g1())))?,
    })
}

fn pack_value(out: &mut PackedData, kind: Kind, value: &str, names: &AssetNames) -> Result<()> {
    let list = value.split(',').filter(|item| !item.is_empty());
    match kind {
        Kind::Byte => out.p1(value.parse()?),
        Kind::Bytes => pack_list(out, list.map(|item| Ok(item.parse()?)))?,
        Kind::Textures => {
            let textures = list
                .map(|texture| names.id("texture", texture))
                .collect::<Result<Vec<_>>>()?;
            ensure!(textures.len() == 6, "Expected six textures");
            for texture in textures {
                out.p2(texture as u16);
            }
        }
        Kind::Enums => {
            let enums = list
                .map(|id| names.id("enum", id))
                .collect::<Result<Vec<_>>>()?;
            out.p1(enums.len() as u8);
            for id in enums {
                out.p2(id as u16);
            }
        }
        Kind::Model => {
            #[cfg(before_742)]
            out.p2(names.id("model", value)? as u16);
            #[cfg(since_742)]
            out.pbigid(TypeId::from_int(names.resolve("model", value)? as i32));
        }
        #[cfg(since_742)]
        Kind::Flag => {}
        #[cfg(since_742)]
        Kind::Interface => out.p3(names.id("interface", value)? as i32),
        #[cfg(since_742)]
        Kind::Song => out.p2(names.id("songs", value)? as u16),
        #[cfg(since_742)]
        Kind::Size => {
            let (width, height) = value.split_once(',').context("Expected width,height")?;
            out.p2(width.parse()?);
            out.p2(height.parse()?);
        }
        #[cfg(since_742)]
        Kind::Colour => out.p4(parse_colour(value)),
        #[cfg(since_742)]
        Kind::WearPos => out.p1(WearPos::from_config_str(value) as u8),
        #[cfg(since_742)]
        Kind::WearPosList => pack_list(
            out,
            list.map(|slot| Ok(WearPos::from_config_str(slot) as u8)),
        )?,
        #[cfg(since_742)]
        Kind::Input => pack_input(out, value)?,
        #[cfg(since_742)]
        Kind::Keys => pack_list(out, list.map(|key| numbered(key, "key_")))?,
    }
    Ok(())
}

/// A byte count, then that many items.
fn unpack_list(
    buf: &mut Packet,
    mut item: impl FnMut(&mut Packet) -> Result<String>,
) -> Result<String> {
    let count = buf.g1();
    let items = (0..count).map(|_| item(buf)).collect::<Result<Vec<_>>>()?;
    Ok(items.join(","))
}

fn pack_list(out: &mut PackedData, items: impl Iterator<Item = Result<u8>>) -> Result<()> {
    let items = items.collect::<Result<Vec<_>>>()?;
    out.p1(items.len() as u8);
    for item in items {
        out.p1(item);
    }
    Ok(())
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

/// An input is `mouse,mouse_<n>,<n>` then its keys, `keypress,key_<n>,<n>`,
/// or tag 2 and its numbers.
#[cfg(since_742)]
fn unpack_input(buf: &mut Packet) -> Result<String> {
    Ok(match buf.g1() {
        0 => {
            let mut input = format!("mouse,mouse_{},{}", buf.g1(), buf.g1());
            let keys = unpack_list(buf, |buf| Ok(format!("key_{}", buf.g1())))?;
            if !keys.is_empty() {
                write!(input, ",{keys}")?;
            }
            input
        }
        1 => format!("keypress,key_{},{}", buf.g1(), buf.g1()),
        2 => format!("2,{}", unpack_list(buf, |buf| Ok(buf.g1().to_string()))?),
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
            pack_list(out, keys.iter().map(|key| numbered(key, "key_")))?;
        }
        ["keypress", key, arg] => {
            out.p1(1);
            out.p1(numbered(key, "key_")?);
            out.p1(arg.parse()?);
        }
        ["2", items @ ..] => {
            out.p1(2);
            let items = items.iter().filter(|item| !item.is_empty());
            pack_list(out, items.map(|item| Ok(item.parse()?)))?;
        }
        _ => bail!("Invalid input {value}"),
    }
    Ok(())
}

#[cfg(since_742)]
fn numbered(value: &str, prefix: &str) -> Result<u8> {
    let number = value
        .strip_prefix(prefix)
        .with_context(|| format!("Expected {prefix}<n>, got {value}"))?;
    Ok(number.parse()?)
}
