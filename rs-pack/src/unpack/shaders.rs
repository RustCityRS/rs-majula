use crate::{
    archive,
    pack::{names::AssetNames, other::shader},
};
use anyhow::{Context, Result, ensure};
use rs_io::js5::{Js5LocalCache, encode_group_entries};
use std::{fs, path::Path};

pub fn unpack_shaders(
    store: &mut Js5LocalCache,
    content_dir: &Path,
    pack_dir: &Path,
) -> Result<u32> {
    let names = AssetNames::load(pack_dir)?;
    let groups = store
        .index(archive::SHADERS)
        .context("Missing shader table")?
        .groups
        .clone();
    for info in &groups {
        let Some(group) = store.read_group(archive::SHADERS, info.id)? else {
            continue;
        };
        let group_name = names.name("shaders", info.id);
        let dir = content_dir.join("shaders").join(&group_name);
        fs::create_dir_all(&dir)?;
        let mut encoded = Vec::new();
        for entry in group.entries {
            let data = entry.data();
            let name = names.name(&format!("shaders_{group_name}"), entry.id);
            let (extension, source, packed) =
                if data.get(2..4) == Some(&[0xfe, 0xff]) || data.get(2..4) == Some(&[0xff, 0xff]) {
                    let text = shader::unpack(data)?;
                    let packed = shader::pack(&text)?;
                    ("shader", text.into_bytes(), packed)
                } else if cfg!(since_742) && data.starts_with(&[4, 0]) {
                    ("program", data.to_vec(), data.to_vec())
                } else {
                    std::str::from_utf8(data)?;
                    (
                        if data.starts_with(b"!!ARB") {
                            "arb"
                        } else {
                            "glsl"
                        },
                        data.to_vec(),
                        data.to_vec(),
                    )
                };
            ensure!(
                packed == data,
                "Shader {}/{} text round trip changed bytes",
                info.id,
                entry.id
            );
            fs::write(dir.join(format!("{name}.{extension}")), source)?;
            encoded.push((entry.id, packed));
        }
        let refs: Vec<_> = encoded
            .iter()
            .map(|(id, data)| (*id, data.as_slice()))
            .collect();
        let original = store
            .read_group_raw_decompressed(archive::SHADERS, info.id)?
            .context("Missing shader payload")?;
        ensure!(
            encode_group_entries(&refs) == original,
            "Shader group {} round trip changed bytes",
            info.id
        );
    }
    Ok(groups.len() as u32)
}
