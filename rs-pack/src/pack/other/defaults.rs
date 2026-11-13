use crate::pack::names::AssetNames;
use crate::pack::packed_data::PackedData;
use anyhow::{Context, Result, bail, ensure};
use rs_io::Packet;
use std::fmt::Write;

const GROUP_UNKNOWN1: u32 = 1;
const GROUP_GRAPHICS: u32 = 3;
const GROUP_WEARPOS: u32 = 6;
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
                writeln!(
                    out,
                    "performancemetricsmodel={}",
                    names.name("model", buf.g2() as u32)
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
                out.p2(names.id("model", value)? as u16);
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
            _ => bail!("Unknown defaults {group} property {key}"),
        }
    }
    out.p1(0);
    Ok(out.dat)
}
