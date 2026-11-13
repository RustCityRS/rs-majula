use crate::{archive, pack::names::AssetNames};
use anyhow::{Context, Result};
use rs_io::js5::Js5Store;
use std::{fs, path::Path};

pub fn unpack_libraries(store: &mut Js5Store, content_dir: &Path, pack_dir: &Path) -> Result<u32> {
    let names = AssetNames::load(pack_dir)?;
    let groups = store
        .reference_table(archive::DLLS)
        .context("Missing native library table")?
        .groups
        .clone();
    for info in &groups {
        let data = store
            .read_group_raw_decompressed(archive::DLLS, info.id)?
            .context("Missing native library")?;
        let path = content_dir.join("dlls").join(names.name("dlls", info.id));
        fs::create_dir_all(path.parent().context("Invalid native library name")?)?;
        fs::write(path, data)?;
    }
    Ok(groups.len() as u32)
}
