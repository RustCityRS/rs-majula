use crate::pack::text::{Number, TextCodec};
use anyhow::{Context, Result, bail, ensure};
use std::fs;
use std::path::{Component, Path};

pub fn read_squares(path: &Path, text: &str) -> Result<String> {
    let parent = path.parent().context("World map source has no parent")?;
    let mut result = String::new();
    for line in text.lines() {
        if let Some(square) = line.trim().strip_prefix("square=") {
            let relative = Path::new(square.trim());
            ensure!(
                relative.extension().is_some_and(|ext| ext == "fms")
                    && relative
                        .components()
                        .all(|part| matches!(part, Component::Normal(_))),
                "Invalid world map square path: {square}"
            );
            let path = parent.join(relative);
            let source = fs::read_to_string(&path)
                .with_context(|| format!("Reading world map square {}", path.display()))?;
            result.push_str(&source);
            if !source.ends_with('\n') {
                result.push('\n');
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    Ok(result)
}

fn tile(c: &mut TextCodec<'_>, key: &str, palettes: &[Vec<i64>; 2]) -> Result<i64> {
    let names = c.names;
    c.mapped(key, Number::U8, |flags| {
        if flags & 1 != 0 {
            return Ok(format!("layers:{}{}{}{}", (flags >> 1 & 3) + 1,
                if flags & 8 != 0 { ",overlay" } else { "" },
                if flags & 16 != 0 { ",locs" } else { "" },
                if flags >> 5 != 0 { format!(",extra:{}", flags >> 5) } else { String::new() }));
        }
        let overlay = flags & 2 != 0;
        let index = (flags >> 2) as usize;
        let kind = if overlay { "overlay" } else { "underlay" };
        if index == 62 { return Ok(if overlay { "empty_overlay" } else { "empty" }.into()); }
        if index == 63 { return Ok(format!("{kind}:direct")); }
        let palette = &palettes[usize::from(overlay)];
        let id = *palette.get(index).context("Invalid world map palette index")?;
        let name = names.reference(if overlay { "flo" } else { "flu" }, id);
        let duplicate = palette.iter().position(|&v| v == id) != Some(index);
        Ok(format!("{kind}:{name}{}", if duplicate { format!("@{index}") } else { String::new() }))
    }, |text| {
        if text == "empty" { return Ok(248); }
        if text == "empty_overlay" { return Ok(250); }
        if let Some(layers) = text.strip_prefix("layers:") {
            let mut parts = layers.split(',');
            let count: i64 = parts.next().unwrap().parse()?;
            ensure!((1..=4).contains(&count), "World map layers must be 1..4");
            let mut flags = 1 | (count - 1) << 1;
            for part in parts {
                match part {
                    "overlay" => flags |= 8,
                    "locs" => flags |= 16,
                    _ => {
                        let extra: i64 = part.strip_prefix("extra:").context("Unknown world map tile option")?.parse()?;
                        ensure!((0..8).contains(&extra), "Invalid world map tile flags");
                        flags |= extra << 5;
                    }
                }
            }
            return Ok(flags);
        }
        let (kind, value) = text.split_once(':').context("Expected underlay:name, overlay:name or layers:count")?;
        let overlay = match kind { "underlay" => false, "overlay" => true, _ => bail!("Unknown world map floor type: {kind}") };
        let palette = &palettes[usize::from(overlay)];
        let index = if value == "direct" { 63 } else {
            let (name, requested) = value.split_once('@').map_or((value,None), |(name,index)| (name,Some(index)));
            let id = names.resolve(if overlay { "flo" } else { "flu" }, name)?;
            let index = if let Some(index) = requested { index.parse()? }
                else { palette.iter().position(|&v| v == id).context("Floor is absent from the world map palette; add it to the palette or use direct encoding")? };
            ensure!(index < 62 && palette.get(index) == Some(&id), "Invalid world map palette reference");
            index
        };
        Ok(((index as i64) << 2) | if overlay { 2 } else { 0 })
    })
}

fn shape(c: &mut TextCodec<'_>, key: &str) -> Result<()> {
    c.mapped(
        key,
        Number::U8,
        |value| Ok(format!("{},{}", value & 63, value >> 6)),
        |text| {
            let (shape, rotation) = text.split_once(',').context("Expected shape,rotation")?;
            let shape: i64 = shape.parse()?;
            let rotation: i64 = rotation.parse()?;
            ensure!(
                (0..64).contains(&shape) && (0..4).contains(&rotation),
                "Invalid world map shape or rotation"
            );
            Ok(shape | rotation << 6)
        },
    )?;
    Ok(())
}

pub fn transcode(gid: u32, c: &mut TextCodec<'_>) -> Result<()> {
    let kind = c.label(
        "format",
        if gid == 1 {
            "details"
        } else if c.source_len() == Some(7) {
            "static_element"
        } else {
            "area"
        },
    )?;
    if kind == "details" {
        c.string("group")?;
        c.string("name")?;
        c.coordinate("origin")?;
        c.number("background", Number::U32)?;
        c.number("active", Number::U8)?;
        c.number("zoom", Number::U8)?;
        c.number("unknown", Number::U8)?;
        let count = c.number("chunks", Number::U8)?;
        for _ in 0..count {
            c.number("plane", Number::U8)?;
            c.array("source_bounds", Number::U16, 4)?;
            c.array("display_bounds", Number::U16, 4)?;
        }
    } else if kind == "static_element" {
        c.coordinate("coord")?;
        c.reference("element", Number::U16, "mel")?;
        c.number("members", Number::U8)?;
    } else {
        ensure!(kind == "area", "Unknown world map format: {kind}");
        let mut palettes = [Vec::new(), Vec::new()];
        for (index, (key, domain)) in [("underlays", "flu"), ("overlays", "flo")]
            .into_iter()
            .enumerate()
        {
            let count = c.number(key, Number::U8)?;
            for _ in 0..count {
                palettes[index].push(c.reference_offset(domain, Number::U8, domain, 1)?);
            }
        }
        while c.more("chunk_kind") {
            let chunk = c.number("chunk_kind", Number::U8)?;
            let mut x = c.number("map_x", Number::U8)? * 64;
            let mut z = c.number("map_z", Number::U8)? * 64;
            if chunk != 0 {
                x += c.number("chunk_x", Number::U8)? * 8;
                z += c.number("chunk_z", Number::U8)? * 8;
            }
            let size = if chunk == 0 { 64 } else { 8 };
            for index in 0..size * size {
                let key = format!("tile{}_{}", x + index / size, z + index % size);
                let flags = tile(c, &key, &palettes)?;
                if flags & 1 == 0 {
                    let index = flags >> 2 & 63;
                    if index == 62 {
                        continue;
                    }
                    if index == 63 {
                        c.reference_offset(
                            "floor",
                            Number::U8,
                            if flags & 2 == 0 { "flu" } else { "flo" },
                            1,
                        )?;
                    }
                    if flags & 2 != 0 {
                        c.reference_offset("underlay", Number::U8, "flu", 1)?;
                    }
                } else {
                    let levels = (flags >> 1 & 3) + 1;
                    for _ in 0..levels {
                        c.reference_offset("underlay", Number::U8, "flu", 1)?;
                        if flags & 8 != 0 {
                            c.reference_offset("overlay", Number::U8, "flo", 1)?;
                            shape(c, "shape_rotation")?;
                        }
                        if flags & 16 != 0 {
                            let count = c.number("locs", Number::U8)?;
                            for _ in 0..count {
                                c.reference("loc", Number::U16, "loc")?;
                                shape(c, "loc_shape_rotation")?;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
