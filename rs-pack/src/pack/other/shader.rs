use crate::archive::{self, ArchiveVersions};
use crate::cache::js5::PackedCache;
use crate::pack::{
    js5::{PreparedGroup, insert_archive, load_name_hashes},
    names::AssetNames,
};
use anyhow::{Context, Result, bail, ensure};
use rs_io::js5::encode_group_entries;
use std::{collections::HashMap, fs, path::Path};

pub fn pack_archive(
    store: &mut PackedCache,
    content_dir: &Path,
    pack_dir: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
) -> Result<()> {
    let names = AssetNames::load(pack_dir)?;
    let root = content_dir.join("shaders");
    let mut groups = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("Invalid shader directory"))?;
        let gid = names.id("shaders", &name)?;
        let files = names.files(
            &entry.path(),
            &format!("shaders_{name}"),
            &["arb", "glsl", "shader"],
        )?;
        let mut entries = Vec::new();
        for (id, path) in files {
            let data = if path.extension().is_some_and(|e| e == "shader") {
                pack(&fs::read_to_string(path)?)?
            } else {
                fs::read(path)?
            };
            entries.push((id, data));
        }
        let refs: Vec<_> = entries
            .iter()
            .map(|(id, data)| (*id, data.as_slice()))
            .collect();
        let data = encode_group_entries(&refs);
        let mut group = PreparedGroup::new(archive::SHADERS, gid, &data, versions)?;
        group.set_files(entries.iter().map(|(id, _)| *id));
        group.name_hash = load_name_hashes(pack_dir, archive::SHADERS, &[gid])
            .get(&gid)
            .copied()
            .unwrap_or(-1);
        groups.push(group);
    }
    insert_archive(
        store,
        archive::SHADERS,
        archive::archive_format(archive::SHADERS).unwrap(),
        versions,
        groups,
        "from shader sources",
    )
}

const OPS: &[(u16, &str, usize)] = &[
    (0, "nop", 0),
    (1, "mov", 2),
    (2, "add", 3),
    (3, "sub", 3),
    (4, "mad", 4),
    (5, "mul", 3),
    (6, "rcp", 2),
    (7, "rsq", 2),
    (8, "dp3", 3),
    (9, "dp4", 3),
    (10, "min", 3),
    (11, "max", 3),
    (12, "slt", 3),
    (13, "sge", 3),
    (14, "exp", 2),
    (15, "log", 2),
    (16, "lit", 2),
    (17, "dst", 3),
    (18, "lrp", 4),
    (19, "frc", 2),
    (20, "m4x4", 3),
    (21, "m4x3", 3),
    (22, "m3x4", 3),
    (23, "m3x3", 3),
    (24, "m3x2", 3),
    (31, "dcl", 2),
    (32, "pow", 3),
    (35, "abs", 2),
    (36, "nrm", 2),
    (46, "mova", 2),
    (66, "texld", 3),
    (81, "def", 5),
];

fn register(token: u32, destination: bool) -> Result<String> {
    let kind = ((token >> 28) & 7) | ((token >> 8) & 24);
    let index = token & 2047;
    let name = match (kind, index) {
        (0, _) => format!("r{index}"),
        (1, _) => format!("v{index}"),
        (2, _) => format!("c{index}"),
        (3, _) => format!("a{index}"),
        (4, 0) => "oPos".into(),
        (4, 1) => "oFog".into(),
        (4, 2) => "oPts".into(),
        (5, _) => format!("oD{index}"),
        (6, _) => format!("oT{index}"),
        (7, _) => format!("i{index}"),
        (8, _) => format!("oC{index}"),
        (9, _) => format!("oDepth{index}"),
        (10, _) => format!("s{index}"),
        (14, _) => format!("b{index}"),
        (15, _) => format!("aL{index}"),
        (17, _) => format!("p{index}"),
        _ => bail!("Unsupported shader register {kind}/{index}"),
    };
    let mut suffix = String::new();
    if destination {
        for (i, c) in ['x', 'y', 'z', 'w'].into_iter().enumerate() {
            if token & (1 << (16 + i)) != 0 {
                suffix.push(c);
            }
        }
    } else {
        for i in 0..4 {
            suffix.push(['x', 'y', 'z', 'w'][((token >> (16 + 2 * i)) & 3) as usize]);
        }
    }
    let mut result = format!("{name}.{suffix}");
    if destination {
        let modifier = (token >> 20) & 15;
        ensure!(
            modifier <= 1 && token & 0x0f002000 == 0,
            "Unsupported shader destination modifier"
        );
        if modifier == 1 {
            result.push_str("_sat");
        }
    } else {
        let modifier = (token >> 24) & 15;
        ensure!(
            modifier <= 1 && token & 0x2000 == 0,
            "Unsupported shader source modifier"
        );
        if modifier == 1 {
            result.insert(0, '-');
        }
    }
    ensure!(
        encode_register(&result, destination)? == token,
        "Unrepresented shader parameter bits"
    );
    Ok(result)
}

fn encode_register(value: &str, destination: bool) -> Result<u32> {
    let (negative, value) = value
        .strip_prefix('-')
        .map_or((false, value), |v| (true, v));
    let (saturate, value) = value
        .strip_suffix("_sat")
        .map_or((false, value), |v| (true, v));
    let (base, swizzle) = value.split_once('.').unwrap_or((value, "xyzw"));
    let (kind, index) = match base {
        "oPos" => (4, 0),
        "oFog" => (4, 1),
        "oPts" => (4, 2),
        _ => {
            let (prefix, number) = base.split_at(
                base.find(|c: char| c.is_ascii_digit())
                    .context("Missing shader register index")?,
            );
            let kind = match prefix {
                "r" => 0,
                "v" => 1,
                "c" => 2,
                "a" => 3,
                "oD" => 5,
                "oT" => 6,
                "i" => 7,
                "oC" => 8,
                "oDepth" => 9,
                "s" => 10,
                "b" => 14,
                "aL" => 15,
                "p" => 17,
                _ => bail!("Unknown shader register: {base}"),
            };
            (kind, number.parse::<u32>()?)
        }
    };
    ensure!(index < 2048, "Shader register index out of range");
    let mut token = 0x80000000 | ((kind & 7) << 28) | ((kind & 24) << 8) | index;
    if destination {
        ensure!(!negative, "A shader destination cannot be negated");
        for c in swizzle.chars() {
            let index = "xyzw".find(c).context("Invalid shader write mask")?;
            token |= 1 << (16 + index);
        }
        if saturate {
            token |= 1 << 20;
        }
    } else {
        ensure!(!saturate, "Saturation belongs to shader destinations");
        let channels = swizzle.chars().collect::<Vec<_>>();
        ensure!(
            channels.len() == 1 || channels.len() == 4,
            "A shader source swizzle needs one or four channels"
        );
        for i in 0..4 {
            let c = channels[if channels.len() == 1 { 0 } else { i }];
            token |= ("xyzw".find(c).context("Invalid shader swizzle")? as u32) << (16 + i * 2);
        }
        if negative {
            token |= 1 << 24;
        }
    }
    Ok(token)
}

const USAGES: &[&str] = &[
    "position",
    "blendweight",
    "blendindices",
    "normal",
    "psize",
    "texcoord",
    "tangent",
    "binormal",
    "tessfactor",
    "positiont",
    "color",
    "fog",
    "depth",
    "sample",
];

fn declaration(token: u32) -> Result<String> {
    if token & 0x78000000 != 0 {
        return Ok(match token {
            0x90000000 => "sampler2d",
            0x98000000 => "sampler3d",
            0xa0000000 => "samplercube",
            _ => bail!("Unknown shader sampler declaration"),
        }
        .into());
    }
    ensure!(token & 0x7ff0ffe0 == 0, "Unknown shader declaration flags");
    Ok(format!(
        "{}{}",
        USAGES
            .get((token & 31) as usize)
            .context("Unknown shader declaration")?,
        (token >> 16) & 15
    ))
}

fn encode_declaration(value: &str) -> Result<u32> {
    Ok(match value {
        "sampler2d" => 0x90000000,
        "sampler3d" => 0x98000000,
        "samplercube" => 0xa0000000,
        _ => {
            let (usage, index) = value.split_at(
                value
                    .find(|c: char| c.is_ascii_digit())
                    .unwrap_or(value.len()),
            );
            let usage = USAGES
                .iter()
                .position(|u| *u == usage)
                .context("Unknown shader declaration")? as u32;
            let index = if index.is_empty() {
                0
            } else {
                index.parse::<u32>()?
            };
            ensure!(index < 16, "Shader declaration index out of range");
            0x80000000 | usage | (index << 16)
        }
    })
}

pub fn version_name(version: u32) -> Result<String> {
    let kind = match version >> 16 {
        0xfffe => "vs",
        0xffff => "ps",
        _ => bail!("Unknown shader version"),
    };
    Ok(format!("{kind}_{}_{}", (version >> 8) & 255, version & 255))
}

pub fn version(value: &str) -> Result<u32> {
    let parts: Vec<_> = value.split('_').collect();
    ensure!(parts.len() == 3, "Invalid shader version");
    let kind = match parts[0] {
        "vs" => 0xfffe0000,
        "ps" => 0xffff0000,
        _ => bail!("Unknown shader stage"),
    };
    Ok(kind | (u32::from(parts[1].parse::<u8>()?) << 8) | u32::from(parts[2].parse::<u8>()?))
}

pub fn unpack(data: &[u8]) -> Result<String> {
    ensure!(
        data.len() >= 8 && data.len().is_multiple_of(4),
        "Invalid Direct3D shader size"
    );
    let words: Vec<_> = data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect();
    let ver = words[0];
    let mut out = format!("version={}\n", version_name(ver)?);
    let mut pos = 1;
    while pos < words.len() {
        let token = words[pos];
        pos += 1;
        let opcode = token as u16;
        if opcode == 65535 {
            ensure!(token == 65535 && pos == words.len(), "Trailing shader data");
            break;
        }
        if opcode == 65534 {
            let count = ((token >> 16) & 32767) as usize;
            let bytes = data
                .get(pos * 4..(pos + count) * 4)
                .context("Truncated shader comment")?;
            ensure!(bytes.starts_with(b"CTAB"), "Unsupported shader comment");
            out.push_str(&super::shader_constants::unpack(&bytes[4..], ver)?);
            pos += count;
            continue;
        }
        let (_, name, count) = OPS
            .iter()
            .find(|(id, _, _)| *id == opcode)
            .context("Unsupported shader instruction")?;
        let expected = u32::from(opcode)
            | if (ver >> 8) & 255 >= 2 {
                (*count as u32) << 24
            } else {
                0
            };
        ensure!(token == expected, "Unsupported shader instruction controls");
        let args = words
            .get(pos..pos + count)
            .context("Truncated shader instruction")?;
        let mut params = Vec::new();
        for (i, &value) in args.iter().enumerate() {
            params.push(if opcode == 31 && i == 0 {
                declaration(value)?
            } else if opcode == 81 && i > 0 {
                let value = f32::from_bits(value);
                ensure!(value.is_finite(), "Non-finite shader constant");
                value.to_string()
            } else {
                register(value, i == 0 || opcode == 31)?
            });
        }
        out.push_str(&format!("{name} {}\n", params.join(", ")));
        pos += count;
    }
    ensure!(words.last() == Some(&65535), "Missing shader end token");
    Ok(out)
}

pub fn pack(text: &str) -> Result<Vec<u8>> {
    let mut ver = None;
    let mut words = Vec::new();
    let mut constants = String::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("//"))
    {
        if let Some(value) = line.strip_prefix("version=") {
            ensure!(ver.is_none(), "Duplicate shader version");
            let v = version(value)?;
            ver = Some(v);
            words.push(v);
            continue;
        }
        let ver = ver.context("Shader version must precede instructions")?;
        if line.starts_with("ctab.") {
            constants.push_str(line);
            constants.push('\n');
            continue;
        }
        if !constants.is_empty() {
            append_constants(&mut words, &constants, ver)?;
            constants.clear();
        }
        let (op, args) = line.split_once(' ').unwrap_or((line, ""));
        let (opcode, _, count) = OPS
            .iter()
            .find(|(_, name, _)| *name == op)
            .context("Unknown shader instruction")?;
        let args: Vec<_> = args
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        ensure!(
            args.len() == *count,
            "Wrong operand count for shader instruction {op}"
        );
        words.push(
            u32::from(*opcode)
                | if (ver >> 8) & 255 >= 2 {
                    (*count as u32) << 24
                } else {
                    0
                },
        );
        for (i, value) in args.into_iter().enumerate() {
            words.push(if *opcode == 31 && i == 0 {
                encode_declaration(value)?
            } else if *opcode == 81 && i > 0 {
                value.parse::<f32>()?.to_bits()
            } else {
                encode_register(value, i == 0 || *opcode == 31)?
            });
        }
    }
    let ver = ver.context("Missing shader version")?;
    if !constants.is_empty() {
        append_constants(&mut words, &constants, ver)?;
    }
    words.push(65535);
    Ok(words.iter().flat_map(|w| w.to_le_bytes()).collect())
}

fn append_constants(words: &mut Vec<u32>, text: &str, ver: u32) -> Result<()> {
    let mut data = b"CTAB".to_vec();
    data.extend_from_slice(&super::shader_constants::pack(text, ver)?);
    ensure!(
        data.len().is_multiple_of(4) && data.len() / 4 < 32768,
        "Invalid shader constant table size"
    );
    words.push(65534 | ((data.len() / 4) as u32) << 16);
    words.extend(
        data.as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b)),
    );
    Ok(())
}
