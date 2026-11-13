use crate::archive;
use crate::pack::names::{AssetNames, archive_name};
use crate::pack::packed_data::PackedData;
use anyhow::{Context, Result, bail, ensure};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use std::collections::BTreeMap;
use std::fmt::Write;

/// The config kind: `.wma` files, with ids from `wma.pack`.
pub const KIND: &str = "wma";
/// The `_unpack` config unpacking writes every map to.
pub const SOURCE: &str = "all.wma";
/// The group holding every map's details.
pub const DETAILS: &str = "details";
/// What a map's zoom is when it doesn't choose one.
const DEFAULT_ZOOM: u8 = 255;

pub struct SubArea {
    pub plane: u8,
    pub source: [u16; 4],
    pub display: [u16; 4],
}

pub struct Element {
    pub coord: u32,
    pub element: u16,
    pub members: u8,
}

/// A world map: its details, and the elements that never move on it.
#[derive(Default)]
pub struct WorldMap {
    pub group: String,
    pub name: String,
    pub origin: u32,
    pub background: i32,
    pub listed: bool,
    pub zoom: u8,
    pub buildarea: u8,
    pub subareas: Vec<SubArea>,
    pub elements: Vec<Element>,
}

/// The group holding a map's elements, by what a map's group is named.
pub fn elements_group(map: &str) -> String {
    format!("{}_staticelements", map.to_lowercase())
}

fn format_coord(coord: u32) -> String {
    let x = (coord >> 14) & 0x3fff;
    let z = coord & 0x3fff;
    format!(
        "{}_{}_{}_{}_{}",
        coord >> 28,
        x >> 6,
        z >> 6,
        x & 0x3f,
        z & 0x3f
    )
}

fn parse_coord(text: &str) -> Result<u32> {
    let parts = text
        .split('_')
        .map(str::parse)
        .collect::<std::result::Result<Vec<u32>, _>>()?;
    let [plane, mx, mz, lx, lz] = parts[..] else {
        bail!("Expected plane_mx_mz_lx_lz: {text}");
    };
    ensure!(
        plane < 16 && mx < 256 && mz < 256 && lx < 64 && lz < 64,
        "Coordinate out of range: {text}"
    );
    Ok((plane << 28) | ((mx << 6 | lx) << 14) | (mz << 6 | lz))
}

pub fn decode_details(data: &[u8]) -> WorldMap {
    let mut buf = Packet::from(data.to_vec());
    let group = buf.gjstr(STRING_TERMINATOR);
    let name = buf.gjstr(STRING_TERMINATOR);
    let origin = buf.g4s() as u32;
    let background = buf.g4s();
    let listed = buf.g1() == 1;
    let zoom = buf.g1();
    let buildarea = buf.g1();
    let subareas = (0..buf.g1())
        .map(|_| SubArea {
            plane: buf.g1(),
            source: [buf.g2(), buf.g2(), buf.g2(), buf.g2()],
            display: [buf.g2(), buf.g2(), buf.g2(), buf.g2()],
        })
        .collect();
    WorldMap {
        group,
        name,
        origin,
        background,
        listed,
        zoom,
        buildarea,
        subareas,
        elements: Vec::new(),
    }
}

pub fn encode_details(map: &WorldMap) -> Vec<u8> {
    let mut out = PackedData::default();
    out.pjstr(&map.group);
    out.pjstr(&map.name);
    out.p4(map.origin as i32);
    out.p4(map.background);
    out.p1(map.listed as u8);
    out.p1(map.zoom);
    out.p1(map.buildarea);
    out.p1(map.subareas.len() as u8);
    for area in &map.subareas {
        out.p1(area.plane);
        for value in area.source.iter().chain(&area.display) {
            out.p2(*value);
        }
    }
    out.dat
}

pub fn decode_element(data: &[u8]) -> Element {
    let mut buf = Packet::from(data.to_vec());
    Element {
        coord: buf.g4s() as u32,
        element: buf.g2(),
        members: buf.g1(),
    }
}

pub fn encode_element(element: &Element) -> Vec<u8> {
    let mut out = PackedData::default();
    out.p4(element.coord as i32);
    out.p2(element.element);
    out.p1(element.members);
    out.dat
}

/// The archive groups the maps pack into, by group id: every map's details, in `wma.pack`
/// order, then each map's elements.
pub fn groups(maps: Vec<WorldMap>, names: &AssetNames) -> Result<Vec<(u32, Vec<Vec<u8>>)>> {
    let mut ordered = BTreeMap::new();
    for map in maps {
        let id = names.id(KIND, &map.group)?;
        if let Some(previous) = ordered.insert(id, map) {
            bail!("Duplicate {KIND}: {}", previous.group);
        }
    }
    // Details are stored by position, which is the map's id.
    if let Some((missing, _)) = (0..).zip(ordered.keys()).find(|(id, have)| id != *have) {
        bail!("Missing {KIND}: {}", names.name(KIND, missing));
    }
    let kind = archive_name(archive::WORLDMAP);
    let mut groups = vec![(
        names.id(kind, DETAILS)?,
        ordered.values().map(encode_details).collect(),
    )];
    for map in ordered.values().filter(|map| !map.elements.is_empty()) {
        groups.push((
            names.id(kind, &elements_group(&map.group))?,
            map.elements.iter().map(encode_element).collect(),
        ));
    }
    Ok(groups)
}

pub fn emit(maps: &[WorldMap], names: &AssetNames) -> Result<String> {
    let mut out = String::new();
    for map in maps {
        writeln!(out, "[{}]", map.group)?;
        writeln!(out, "name={}", map.name)?;
        writeln!(out, "origin={}", format_coord(map.origin))?;
        if map.background as u32 > 0xffffff {
            writeln!(out, "background=0x{:08x}", map.background as u32)?;
        } else {
            writeln!(out, "background=0x{:06x}", map.background)?;
        }
        writeln!(out, "listed={}", if map.listed { "yes" } else { "no" })?;
        if map.zoom == DEFAULT_ZOOM {
            writeln!(out, "zoom=default")?;
        } else {
            writeln!(out, "zoom={}", map.zoom)?;
        }
        writeln!(out, "buildarea={}", map.buildarea)?;
        for area in &map.subareas {
            let [a, b, c, d] = area.source;
            let [e, f, g, h] = area.display;
            writeln!(
                out,
                "subarea={},{a},{b},{c},{d},{e},{f},{g},{h}",
                area.plane
            )?;
        }
        for element in &map.elements {
            writeln!(
                out,
                "element={},{},{}",
                format_coord(element.coord),
                names.name("mel", element.element as u32),
                element.members
            )?;
        }
        out.push('\n');
    }
    Ok(out)
}

pub fn parse(text: &str, names: &AssetNames) -> Result<Vec<WorldMap>> {
    let mut maps: Vec<WorldMap> = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if let Some(group) = line.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
            maps.push(WorldMap {
                group: group.to_owned(),
                ..WorldMap::default()
            });
            continue;
        }
        let map = maps
            .last_mut()
            .with_context(|| format!("Property outside a map: {line}"))?;
        let (key, value) = line
            .split_once('=')
            .with_context(|| format!("Invalid line: {line}"))?;
        parse_property(map, key, value, names).with_context(|| format!("Invalid line: {line}"))?;
    }
    Ok(maps)
}

fn parse_property(map: &mut WorldMap, key: &str, value: &str, names: &AssetNames) -> Result<()> {
    match key {
        "name" => map.name = value.to_owned(),
        "origin" => map.origin = parse_coord(value)?,
        "background" => {
            let digits = value.strip_prefix("0x").context("Expected a hex colour")?;
            map.background = u32::from_str_radix(digits, 16)? as i32;
        }
        "listed" => map.listed = value == "yes",
        "zoom" => {
            map.zoom = if value == "default" {
                DEFAULT_ZOOM
            } else {
                value.parse()?
            }
        }
        "buildarea" => map.buildarea = value.parse()?,
        "subarea" => {
            let parts = value
                .split(',')
                .map(str::parse)
                .collect::<std::result::Result<Vec<u16>, _>>()?;
            let [plane, a, b, c, d, e, f, g, h] = parts[..] else {
                bail!("Expected a plane and eight bounds");
            };
            map.subareas.push(SubArea {
                plane: plane as u8,
                source: [a, b, c, d],
                display: [e, f, g, h],
            });
        }
        "element" => {
            let mut parts = value.split(',');
            let coord = parts.next().context("Expected coord,element,members")?;
            let element = parts.next().context("Expected coord,element,members")?;
            let members = parts.next().context("Expected coord,element,members")?;
            map.elements.push(Element {
                coord: parse_coord(coord)?,
                element: names.id("mel", element)? as u16,
                members: members.parse()?,
            });
        }
        _ => bail!("Unknown property"),
    }
    Ok(())
}
