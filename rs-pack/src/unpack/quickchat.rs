use super::binary_assets::unpack_binary_archive;
use anyhow::Result;
use rs_dat2::CacheStore;
use rs_dat2::archive;
use std::path::Path;
use tracing::info;

pub fn unpack_quickchat(store: &mut CacheStore, content_dir: &Path) -> Result<u32> {
    let mut total = 0;
    for &aid in &[archive::QUICKCHAT, archive::QUICKCHAT_GLOBAL] {
        if !store.has_archive(aid) {
            continue;
        }
        let n = unpack_binary_archive(store, aid, content_dir)?;
        let name = archive::archive_format(aid)
            .map(|f| f.content_dir)
            .unwrap_or("?");
        info!("  {} (archive {}): {} groups", name, aid, n);
        total += n;
    }
    Ok(total)
}
