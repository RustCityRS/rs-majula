use crate::pack::names::AssetNames;
use crate::pack::text::{Number, TextCodec};
use anyhow::{Context, Result, ensure};

pub fn unpack(data: &[u8], names: &AssetNames) -> Result<String> {
    let modern = data.ends_with(&[255, 255]);
    let header = if modern { 23 } else { 18 };
    ensure!(data.len() >= header, "Truncated model header");
    let start = data.len() - header;
    let versioned = modern && data[start + 5] & 8 != 0;
    let mut rotated = data[start..data.len() - if modern { 2 } else { 0 }].to_vec();
    if versioned {
        rotated.push(data[start - 1]);
    }
    rotated.extend_from_slice(&data[..start - usize::from(versioned)]);
    let mut c = TextCodec::unpack(&rotated, names);
    transcode(&mut c, modern)?;
    super::geometry::readable(&format!(
        "format={}\n{}",
        if modern { "new" } else { "old" },
        c.finish()?.0
    ))
}

pub fn pack(text: &str, names: &AssetNames) -> Result<Vec<u8>> {
    let (mut fields, arrays) = super::geometry::encoded(text)?;
    let format = fields.remove("format").context("Missing model format")?;
    ensure!(
        matches!(format.as_str(), "new" | "old"),
        "Invalid model format"
    );
    let modern = format == "new";
    let mut c = TextCodec::from_fields(fields, arrays, names);
    transcode(&mut c, modern)?;
    let rotated = c.finish()?.1;
    let header = if modern { 21 } else { 18 };
    let versioned = modern && rotated[5] & 8 != 0;
    let mut result = rotated[header + usize::from(versioned)..].to_vec();
    if versioned {
        result.push(rotated[header]);
    }
    result.extend_from_slice(&rotated[..header]);
    if modern {
        result.extend_from_slice(&[255, 255]);
    }
    Ok(result)
}

fn transcode(c: &mut TextCodec<'_>, modern: bool) -> Result<()> {
    let vertices = c.number("vertices", Number::U16)? as usize;
    let faces = c.number("faces", Number::U16)? as usize;
    let textures = c.number("mappings", Number::U8)? as usize;
    let flags = c.number("flags", Number::U8)?;
    let priority = c.number("priority", Number::U8)?;
    let alpha = c.number("has_alpha", Number::U8)?;
    let face_labels = c.number("has_face_labels", Number::U8)?;
    let materials = if modern {
        c.number("has_materials", Number::U8)?
    } else {
        0
    };
    let vertex_labels = c.number("has_vertex_labels", Number::U8)?;
    let x_length = c.length()?;
    let y_length = c.length()?;
    let z_length = c.length()?;
    let index_length = c.length()?;
    let uv_length = if modern { Some(c.length()?) } else { None };
    let version = if modern && flags & 8 != 0 {
        c.number("version", Number::U8)?
    } else {
        12
    };
    let mappings = if modern {
        (0..textures)
            .map(|i| {
                c.choice(
                    &format!("mapping{i}.type"),
                    Number::U8,
                    &["planar", "cylindrical", "cube", "spherical"],
                )
                .map(|v| v as i64)
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![0; textures]
    };
    ensure!(
        mappings.iter().all(|v| (0..=3).contains(v)),
        "Unsupported texture mapping"
    );
    let vertex_flags = c.array("vertex_flags", Number::U8, vertices)?;
    if modern && flags & 1 != 0 {
        c.array("face_types", Number::U8, faces)?;
    }
    let strategies = c.array("face_strategy", Number::U8, faces)?;
    ensure!(
        strategies.iter().all(|v| (1..=4).contains(v)),
        "Invalid face strategy"
    );
    if priority == 255 {
        c.array("face_priority", Number::I8, faces)?;
    }
    if face_labels == 1 {
        c.array("face_labels", Number::U8, faces)?;
    }
    let old_types = if !modern && flags == 1 {
        c.array("face_types", Number::U8, faces)?
    } else {
        vec![0; faces]
    };
    if vertex_labels == 1 {
        c.array("vertex_labels", Number::U8, vertices)?;
    }
    if alpha == 1 {
        c.array("face_alpha", Number::U8, faces)?;
    }
    let start = c.offset();
    let count = strategies.iter().map(|&s| if s == 1 { 3 } else { 1 }).sum();
    c.array("face_vertex_deltas", Number::SignedSmart, count)?;
    c.set_length(index_length, start)?;
    let mut material_ids = Vec::new();
    if materials == 1 {
        for i in 0..faces {
            material_ids.push(c.reference_offset(
                &format!("face{i}.material"),
                Number::U16,
                "texture",
                1,
            )?);
        }
    }
    if let Some(length) = uv_length {
        let start = c.offset();
        if textures != 0 {
            for (i, &material) in material_ids.iter().enumerate() {
                if material != -1 {
                    c.reference_offset(&format!("face{i}.mapping"), Number::U8, "mapping", 1)?;
                }
            }
        }
        c.set_length(length, start)?;
    }
    for (i, &kind) in old_types.iter().enumerate() {
        if kind & 2 != 0 {
            c.reference(&format!("face{i}.material"), Number::U16, "texture")?;
        } else {
            c.number(&format!("face{i}.colour"), Number::U16)?;
        }
    }
    if !modern {
        for i in 0..textures {
            c.array(&format!("mapping{i}.vertices"), Number::U16, 3)?;
        }
    }
    for (axis, mask, length) in [("x", 1, x_length), ("y", 2, y_length), ("z", 4, z_length)] {
        let start = c.offset();
        c.array(
            &format!("vertex_{axis}_deltas"),
            Number::SignedSmart,
            vertex_flags.iter().filter(|&&f| f & mask != 0).count(),
        )?;
        c.set_length(length, start)?;
    }
    if modern {
        for simple in [true, false] {
            for (i, &mapping) in mappings.iter().enumerate() {
                if (mapping == 0) == simple {
                    c.array(
                        &format!("mapping{i}.{}", if simple { "vertices" } else { "axes" }),
                        Number::U16,
                        3,
                    )?;
                }
            }
        }
        for (i, &mapping) in mappings.iter().enumerate() {
            if mapping == 0 {
                continue;
            }
            for (axis, wide) in [
                ("x", version >= 15),
                ("y", version >= 14),
                ("z", version >= 15),
            ] {
                c.number(
                    &format!("mapping{i}.scale_{axis}"),
                    if wide { Number::U24 } else { Number::U16 },
                )?;
            }
        }
        for field in ["rotation", "direction"] {
            for (i, &mapping) in mappings.iter().enumerate() {
                if mapping != 0 {
                    c.number(&format!("mapping{i}.{field}"), Number::I8)?;
                }
            }
        }
        for (i, &mapping) in mappings.iter().enumerate() {
            if mapping == 0 {
                continue;
            }
            c.number(&format!("mapping{i}.speed"), Number::I8)?;
            if mapping == 2 {
                c.array(&format!("mapping{i}.translation"), Number::I8, 2)?;
            }
        }
        if flags & 2 != 0 {
            let count = c.number("emitters", Number::U8)?;
            for i in 0..count {
                c.reference(&format!("emitter{i}.type"), Number::U16, "particleemitter")?;
                c.number(&format!("emitter{i}.face"), Number::U16)?;
            }
            let count = c.number("effectors", Number::U8)?;
            for i in 0..count {
                c.reference(
                    &format!("effector{i}.type"),
                    Number::U16,
                    "particleeffector",
                )?;
                c.number(&format!("effector{i}.vertex"), Number::U16)?;
            }
        }
        if flags & 4 != 0 {
            let count = c.number("billboards", Number::U8)?;
            for i in 0..count {
                c.reference(&format!("billboard{i}.type"), Number::U16, "billboard")?;
                c.number(&format!("billboard{i}.face"), Number::U16)?;
                c.number(&format!("billboard{i}.skin"), Number::U8)?;
                c.number(&format!("billboard{i}.distance"), Number::I8)?;
            }
        }
    }
    Ok(())
}
