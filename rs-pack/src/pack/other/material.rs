use crate::pack::names::AssetNames;
use crate::pack::text::{Number, TextCodec};
use anyhow::{Context, Result, ensure};
use rustc_hash::FxHashMap as HashMap;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

/// A material's id is its texture's, so its file takes the texture's name.
pub const KIND: &str = "texture";
pub const EXTENSION: &str = "material";

const FIELDS: [(&str, Number); 19] = [
    ("highdetail", Number::U8),
    ("lowmem", Number::U8),
    ("lowdetail", Number::U8),
    ("brightness", Number::I8),
    ("intensity", Number::I8),
    ("effect", Number::I8),
    ("effect_param", Number::I8),
    ("averagecolour", Number::U16),
    ("scroll_u", Number::I8),
    ("scroll_v", Number::I8),
    ("combinealpha", Number::U8),
    ("flag95", Number::U8),
    ("mipmapping", Number::I8),
    ("repeat_s", Number::U8),
    ("repeat_t", Number::U8),
    ("hdr", Number::U8),
    ("effect_param2", Number::U8),
    ("effect_param3", Number::U32),
    ("alphamode", Number::U8),
];

/// Each material's text by id, from the archive's one file.
pub fn unpack(data: &[u8], names: &AssetNames) -> Result<BTreeMap<u32, String>> {
    let mut c = TextCodec::unpack(data, names);
    transcode(&mut c)?;
    let mut materials: BTreeMap<u32, String> = BTreeMap::new();
    // `count` and `present` are which materials have files.
    for line in c.finish()?.0.lines().skip(2) {
        let (id, field) = line
            .split_once('.')
            .context("Material field without an id")?;
        writeln!(materials.entry(id.parse()?).or_default(), "{field}")?;
    }
    Ok(materials)
}

/// The archive's one file, from the `.material` files among `paths`.
pub fn pack_files(paths: &[PathBuf], names: &AssetNames) -> Result<Vec<u8>> {
    let mut materials = BTreeMap::new();
    for path in paths
        .iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == EXTENSION))
    {
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .context("Invalid material filename")?;
        let id = names
            .id(KIND, name)
            .with_context(|| path.display().to_string())?;
        let text = fs::read_to_string(path).with_context(|| path.display().to_string())?;
        ensure!(
            materials.insert(id, text).is_none(),
            "Duplicate material {id}: {}",
            path.display()
        );
    }
    pack(&materials, names)
}

pub fn pack(materials: &BTreeMap<u32, String>, names: &AssetNames) -> Result<Vec<u8>> {
    let count = materials
        .last_key_value()
        .map_or(0, |(&id, _)| id as usize + 1);
    let mut present = vec![0; count];
    let mut fields = HashMap::default();
    fields.insert("count".to_owned(), count.to_string());
    for (&id, text) in materials {
        present[id as usize] = 1;
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }
            let (field, value) = line
                .split_once('=')
                .with_context(|| format!("Material {id}: expected field=value"))?;
            ensure!(
                fields
                    .insert(format!("{id}.{}", field.trim()), value.trim().to_owned())
                    .is_none(),
                "Material {id}: duplicate {field}"
            );
        }
    }
    let mut arrays = HashMap::default();
    arrays.insert("present".to_owned(), present);
    let mut c = TextCodec::from_fields(fields, arrays, names);
    transcode(&mut c)?;
    Ok(c.finish()?.1)
}

fn transcode(c: &mut TextCodec<'_>) -> Result<()> {
    let count = c.number("count", Number::U16)? as usize;
    let present = c.array("present", Number::U8, count)?;
    let ids: Vec<_> = (0..count).filter(|&id| present[id] == 1).collect();
    for (field, kind) in FIELDS {
        for id in &ids {
            c.number(&format!("{id}.{field}"), kind)?;
        }
    }
    Ok(())
}
