use crate::pack::names::AssetNames;
use crate::pack::text::{Number, TextCodec};
use anyhow::{Context, Result, bail, ensure};
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::{Component, Path};

pub const EXTENSION: &str = "worldmap";
pub const SQUARE: &str = "square";
/// The group holding every map's details.
pub const DETAILS: &str = "details";
/// What a map's group calls its layer files, which are also their formats.
pub const LAYERS: [&str; 4] = ["loc", "underlay", "overlay", "overlay2"];

pub struct Square {
    pub name: String,
    pub text: String,
}

/// The group a map's details name for its layers, and their text.
pub fn unpack_details(data: &[u8], names: &AssetNames) -> Result<(String, String)> {
    let mut c = TextCodec::unpack(data, names);
    c.label("format", DETAILS)?;
    let group = details(&mut c)?;
    Ok((group, c.finish()?.0))
}

pub fn unpack_label(data: &[u8], names: &AssetNames) -> Result<String> {
    let mut c = TextCodec::unpack(data, names);
    c.label("format", "label")?;
    label(&mut c)?;
    Ok(c.finish()?.0)
}

/// A file's text: its format, and a square line for each layer square it holds.
pub fn pack(path: &Path, text: &str, names: &AssetNames) -> Result<Vec<u8>> {
    let mut header = String::new();
    let mut squares = Vec::new();
    for line in text.lines() {
        match line.trim().strip_prefix("square=") {
            Some(square) => squares.push(square),
            None => writeln!(header, "{line}")?,
        }
    }
    let mut c = TextCodec::pack(&header, names)?;
    let format = c.label("format", "")?;
    if let Some(layer) = LAYERS.into_iter().find(|&layer| layer == format) {
        c.finish()?;
        let mut data = Vec::new();
        for square in squares {
            encode_square(layer, &read_square(path, square)?, names, &mut data)
                .with_context(|| format!("World map square {square}"))?;
        }
        return Ok(data);
    }
    ensure!(squares.is_empty(), "Only map layers have squares");
    match format.as_str() {
        DETAILS => {
            details(&mut c)?;
        }
        "label" => label(&mut c)?,
        _ => bail!("Unknown world map format: {format}"),
    }
    Ok(c.finish()?.1)
}

fn details(c: &mut TextCodec<'_>) -> Result<String> {
    let group = c.string("group")?;
    c.string("name")?;
    c.array("origin", Number::U16, 2)?;
    c.mapped(
        "background",
        Number::U32,
        |value| Ok(format!("0x{value:08x}")),
        |text| {
            let digits = text.strip_prefix("0x").context("Expected a hex colour")?;
            Ok(i64::from_str_radix(digits, 16)?)
        },
    )?;
    c.choice("listed", Number::U8, &["no", "yes"])?;
    c.number("zoom", Number::U8)?;
    let areas = c.number("areas", Number::U8)?;
    for _ in 0..areas {
        c.array("area", Number::U16, 4)?;
    }
    Ok(group)
}

fn label(c: &mut TextCodec<'_>) -> Result<()> {
    c.string("text")?;
    c.number("unknown1", Number::I8)?;
    c.array("coord", Number::U16, 2)?;
    c.number("unknown2", Number::I32)?;
    Ok(())
}

fn read_square(path: &Path, square: &str) -> Result<String> {
    let relative = Path::new(square.trim());
    ensure!(
        relative.extension().is_some_and(|ext| ext == SQUARE)
            && relative
                .components()
                .all(|part| matches!(part, Component::Normal(_))),
        "Invalid world map square path: {square}"
    );
    let path = path
        .parent()
        .context("World map has no parent")?
        .join(relative);
    fs::read_to_string(&path)
        .with_context(|| format!("Reading world map square {}", path.display()))
}

/// Every tile a square record holds, in the order it holds them: a whole map
/// square, or one zone of it.
fn tiles(map: (u8, u8), zone: Option<(u8, u8)>) -> impl Iterator<Item = (u32, u32)> {
    let (x, z) = (u32::from(map.0) << 6, u32::from(map.1) << 6);
    let (x, z, size) = match zone {
        Some((zx, zz)) => (x + (u32::from(zx) << 3), z + (u32::from(zz) << 3), 8),
        None => (x, z, 64),
    };
    (x..x + size).flat_map(move |x| (z..z + size).map(move |z| (x, z)))
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn g1(&mut self) -> Result<u8> {
        let value = *self
            .data
            .get(self.pos)
            .context("Truncated world map square")?;
        self.pos += 1;
        Ok(value)
    }

    fn g3(&mut self) -> Result<u32> {
        Ok(u32::from(self.g1()?) << 16 | u32::from(self.g1()?) << 8 | u32::from(self.g1()?))
    }
}

/// A layer's square records, each as the text of its non-empty tiles.
pub fn unpack_squares(layer: &str, data: &[u8], names: &AssetNames) -> Result<Vec<Square>> {
    let mut buf = Reader { data, pos: 0 };
    let mut squares = Vec::new();
    while buf.pos < data.len() {
        let kind = buf.g1()?;
        ensure!(kind <= 1, "Unknown world map square kind {kind}");
        let zone = if kind == 1 {
            Some((buf.g1()?, buf.g1()?))
        } else {
            None
        };
        let map = (buf.g1()?, buf.g1()?);
        let mut name = format!("m{}_{}", map.0, map.1);
        let mut text = format!("map_x={}\nmap_z={}\n", map.0, map.1);
        if let Some((zx, zz)) = zone {
            ensure!(zx < 8 && zz < 8, "World map zone out of range");
            write!(name, "_{zx}_{zz}")?;
            write!(text, "chunk_x={zx}\nchunk_z={zz}\n")?;
        }
        for (x, z) in tiles(map, zone) {
            if let Some(value) = unpack_tile(layer, &mut buf, names)? {
                writeln!(text, "tile{x}_{z}={value}")?;
            }
        }
        squares.push(Square { name, text });
    }
    Ok(squares)
}

fn unpack_tile(layer: &str, buf: &mut Reader<'_>, names: &AssetNames) -> Result<Option<String>> {
    let value = buf.g1()?;
    if value == 0 {
        return Ok(None);
    }
    Ok(Some(match layer {
        "underlay" => names.reference("flu", i64::from(value) - 1),
        "overlay" | "overlay2" => {
            let shape = buf.g1()?;
            format!(
                "{},{},{}",
                names.reference("flo", i64::from(value) - 1),
                shape >> 2,
                shape & 3
            )
        }
        _ => {
            ensure!(value < 8, "Unknown world map loc flags {value}");
            let mut parts = Vec::new();
            if value & 1 != 0 {
                parts.push(format!("unknown1:{}", buf.g1()?));
            }
            if value & 2 != 0 {
                parts.push(format!("unknown2:0x{:06x}", buf.g3()?));
            }
            if value & 4 != 0 {
                parts.push(format!(
                    "loc:{}",
                    names.reference("loc", i64::from(buf.g3()?) - 1)
                ));
            }
            parts.join(",")
        }
    }))
}

fn encode_square(layer: &str, text: &str, names: &AssetNames, out: &mut Vec<u8>) -> Result<()> {
    let mut properties = HashMap::new();
    let mut tiles_by_coord = HashMap::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .with_context(|| format!("Expected key=value: {line}"))?;
        if let Some(coord) = key.strip_prefix("tile") {
            let (x, z) = coord.split_once('_').context("Expected tileX_Z")?;
            let coord: (u32, u32) = (x.parse()?, z.parse()?);
            ensure!(
                tiles_by_coord.insert(coord, value).is_none(),
                "Duplicate tile {coord:?}"
            );
        } else {
            ensure!(
                matches!(key, "map_x" | "map_z" | "chunk_x" | "chunk_z"),
                "Unknown world map square property {key}"
            );
            ensure!(
                properties.insert(key, value.parse::<u8>()?).is_none(),
                "Duplicate {key}"
            );
        }
    }
    let map = (
        *properties.get("map_x").context("Missing map_x")?,
        *properties.get("map_z").context("Missing map_z")?,
    );
    let zone = match (properties.get("chunk_x"), properties.get("chunk_z")) {
        (Some(&zx), Some(&zz)) => {
            ensure!(zx < 8 && zz < 8, "World map zone out of range");
            Some((zx, zz))
        }
        (None, None) => None,
        _ => bail!("A zone needs chunk_x and chunk_z"),
    };
    match zone {
        Some((zx, zz)) => out.extend_from_slice(&[1, zx, zz]),
        None => out.push(0),
    }
    out.extend_from_slice(&[map.0, map.1]);
    for coord in tiles(map, zone) {
        match tiles_by_coord.remove(&coord) {
            Some(value) => encode_tile(layer, value, names, out)
                .with_context(|| format!("tile{}_{}", coord.0, coord.1))?,
            None => out.push(0),
        }
    }
    if let Some((x, z)) = tiles_by_coord.keys().next() {
        bail!("tile{x}_{z} is outside the square");
    }
    Ok(())
}

fn encode_tile(layer: &str, value: &str, names: &AssetNames, out: &mut Vec<u8>) -> Result<()> {
    let id = |kind: &str, text: &str, max: i64| -> Result<i64> {
        let id = names.resolve(kind, text.trim())? + 1;
        ensure!((0..=max).contains(&id), "{kind} out of range: {text}");
        Ok(id)
    };
    match layer {
        "underlay" => out.push(id("flu", value, 255)? as u8),
        "overlay" | "overlay2" => {
            let mut parts = value.split(',');
            let (Some(floor), Some(shape), Some(rotation), None) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            else {
                bail!("Expected overlay,shape,rotation");
            };
            let floor = id("flo", floor, 255)?;
            let shape: u8 = shape.trim().parse()?;
            let rotation: u8 = rotation.trim().parse()?;
            ensure!(
                floor != 0 && shape < 64 && rotation < 4,
                "Invalid world map overlay: {value}"
            );
            out.extend_from_slice(&[floor as u8, shape << 2 | rotation]);
        }
        _ => {
            let mut values = [None; 3];
            for part in value.split(',') {
                let (key, text) = part.split_once(':').context("Expected key:value")?;
                let (bit, value) = match key.trim() {
                    "unknown1" => (0, i64::from(text.trim().parse::<u8>()?)),
                    "unknown2" => {
                        let text = text.trim();
                        let value = match text.strip_prefix("0x") {
                            Some(digits) => i64::from_str_radix(digits, 16)?,
                            None => text.parse()?,
                        };
                        ensure!((0..1 << 24).contains(&value), "unknown2 out of range");
                        (1, value)
                    }
                    "loc" => (2, id("loc", text, (1 << 24) - 1)?),
                    key => bail!("Unknown world map loc property {key}"),
                };
                ensure!(values[bit].replace(value).is_none(), "Duplicate {key}");
            }
            let flags = (0..3)
                .filter(|&bit| values[bit].is_some())
                .fold(0, |flags, bit| flags | 1 << bit);
            out.push(flags);
            for (bit, value) in values.into_iter().enumerate() {
                match value {
                    Some(value) if bit == 0 => out.push(value as u8),
                    Some(value) => out.extend_from_slice(&(value as u32).to_be_bytes()[1..]),
                    None => {}
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{pack, unpack_details, unpack_label, unpack_squares};
    use crate::pack::names::AssetNames;
    use std::fmt::Write;
    use std::{env, fs, process};

    /// The cache's details and first label, and a zone then a whole square of each layer.
    #[test]
    fn world_map_files_keep_their_cache_bytes() {
        let dir = env::temp_dir().join(format!("rs-majula-worldmap-530-{}", process::id()));
        fs::create_dir_all(dir.join("main")).unwrap();
        let names = AssetNames::load(&dir).unwrap();
        let path = dir.join("main.worldmap");
        let details = b"main\0RuneScape surface\0\x00\x32\x00\x32\xff\xff\xff\xff\x01\x4b\x01\x08\x00\x09\xc0\x0f\x3f\x10\x3f";
        let (group, text) = unpack_details(details, &names).unwrap();
        assert_eq!(group, "main");
        assert_eq!(pack(&path, &text, &names).unwrap(), details);
        let label = b"Baxtorian<br>Falls\0\x08\x09\xd3\x0d\x86\xff\xff\xff\xff";
        let text = unpack_label(label, &names).unwrap();
        assert!(text.contains("coord=2515,3462\n"));
        assert_eq!(pack(&path, &text, &names).unwrap(), label);
        for (layer, tile) in [
            ("underlay", &[48][..]),
            ("overlay", &[171, 6]),
            ("overlay2", &[42, 0]),
            ("loc", &[7, 25, 0x21, 0, 8, 0, 2, 0xe3]),
        ] {
            let mut data = vec![1, 2, 3, 50, 50];
            data.extend_from_slice(tile);
            data.extend_from_slice(&[0; 63]);
            data.extend_from_slice(&[0, 50, 51]);
            data.extend_from_slice(&[0; 4095]);
            data.extend_from_slice(tile);
            let mut text = format!("format={layer}\n");
            for square in unpack_squares(layer, &data, &names).unwrap() {
                let relative = format!("main/{layer}_{}.square", square.name);
                fs::write(dir.join(&relative), square.text).unwrap();
                writeln!(text, "square={relative}").unwrap();
            }
            assert_eq!(pack(&path, &text, &names).unwrap(), data, "{layer}");
        }
        let zone = fs::read_to_string(dir.join("main/loc_m50_50_2_3.square")).unwrap();
        assert_eq!(
            zone,
            "map_x=50\nmap_z=50\nchunk_x=2\nchunk_z=3\ntile3216_3224=unknown1:25,unknown2:0x210008,loc:loc_738\n"
        );
        fs::write(
            dir.join("main/loc_m50_50_2_3.square"),
            zone.replace("tile3216_3224=unknown1:25,", "tile3217_3224="),
        )
        .unwrap();
        let text = "format=loc\nsquare=main/loc_m50_50_2_3.square\n";
        assert_eq!(
            pack(&path, text, &names).unwrap()[5..15],
            [0, 0, 0, 0, 0, 0, 0, 0, 6, 0x21]
        );
        fs::write(
            dir.join("main/loc_m50_50_2_3.square"),
            "map_x=50\nmap_z=50\nchunk_x=2\nchunk_z=3\ntile3200_3200=loc:loc_1\n",
        )
        .unwrap();
        assert!(pack(&path, text, &names).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
