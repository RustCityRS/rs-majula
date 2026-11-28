use crate::archive;
use crate::pack::names::{AssetNames, archive_name};
use anyhow::{Context, Result};
use rs_io::js5::Js5LocalCache;
use std::{fs, path::Path};

#[cfg(since_647)]
pub fn unpack_libraries(
    store: &mut Js5LocalCache,
    content_dir: &Path,
    pack_dir: &Path,
) -> Result<u32> {
    unpack_raw(store, archive::DLLS, content_dir, pack_dir, "")
}

/// Writes each group of `aid` as the file it holds, named from its registry,
/// with `extension` appended when the name carries none.
pub fn unpack_raw(
    store: &mut Js5LocalCache,
    aid: u8,
    content_dir: &Path,
    pack_dir: &Path,
    extension: &str,
) -> Result<u32> {
    let names = AssetNames::load(pack_dir)?;
    let format = archive::archive_format(aid).context("Unknown raw archive")?;
    let groups = store
        .index(aid)
        .context("Missing raw archive table")?
        .groups
        .clone();
    for info in &groups {
        #[cfg(all(since_530, before_647))]
        let Some(data) = store.read_group_raw_decompressed(aid, info.id)? else {
            continue;
        };
        #[cfg(since_647)]
        let data = store
            .read_group_raw_decompressed(aid, info.id)?
            .context("Missing raw group")?;
        let mut name = names.name(archive_name(aid), info.id);
        if !extension.is_empty() {
            name = format!("{name}.{extension}");
        }
        let path = content_dir.join(format.content_dir).join(name);
        fs::create_dir_all(path.parent().context("Invalid raw asset name")?)?;
        fs::write(path, data)?;
    }
    Ok(groups.len() as u32)
}
