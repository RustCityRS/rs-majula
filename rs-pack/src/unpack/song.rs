#[cfg(all(since_244, before_647))]
use super::model::load_existing_pack;
#[cfg(since_244)]
use crate::archive;
#[cfg(since_647)]
use crate::cache::midi::decode_jagex_midi;
#[cfg(since_647)]
use crate::pack::pack_registry::PackFile;
#[cfg(all(since_244, before_647))]
use crate::versionlist::VersionList;
#[cfg(since_647)]
use anyhow::Result;
#[cfg(before_647)]
use rs_io::bz2::bz2_decompress;
#[cfg(since_244)]
use rs_io::js5::Js5Store;
#[cfg(since_647)]
use rs_io::js5::compression::{Container, decompress};
#[cfg(since_647)]
use serde_json::json;
use std::fs;
use std::path::Path;
#[cfg(before_647)]
use tracing::debug;
#[cfg(since_647)]
use tracing::{info, warn};

#[cfg(before_647)]
pub fn unpack_songs(songs_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
    let out_dir = output_dir.join("songs");
    fs::create_dir_all(&out_dir)?;

    let Ok(entries) = fs::read_dir(songs_dir) else {
        return Ok(());
    };

    let mut files: Vec<_> = entries.flatten().collect();
    files.sort_by_key(|e| e.file_name());

    let mut count = 0;
    for entry in files {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let key = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let data = fs::read(&path)?;
        if data.len() < 4 {
            continue;
        }

        let uncompressed_size = ((data[0] as u32) << 24)
            | ((data[1] as u32) << 16)
            | ((data[2] as u32) << 8)
            | (data[3] as u32);

        let midi = bz2_decompress(&data[4..], uncompressed_size as usize, true, 0);

        let mid_name = key_to_filename(&key);
        fs::write(out_dir.join(&mid_name), &midi)?;
        count += 1;
    }

    debug!("Unpacked {} songs", count);
    Ok(())
}

#[cfg(before_647)]
fn key_to_filename(key: &str) -> String {
    if key.len() < 12
        && let Some(stem) = key.strip_suffix("_mid")
    {
        return format!("{stem}.mid");
    }
    format!("{key}.mid")
}

#[cfg(all(since_244, before_647))]
pub fn unpack_midi(
    cache: &mut Js5Store,
    version_list: &VersionList,
    output_dir: &Path,
    pack_dir: &Path,
) -> anyhow::Result<()> {
    let songs_dir = output_dir.join("songs");
    let jingles_dir = output_dir.join("jingles");
    fs::create_dir_all(&songs_dir)?;
    fs::create_dir_all(&jingles_dir)?;

    let existing_names = load_existing_pack(pack_dir, "midi");
    let count = cache.group_count(archive::MIDI).unwrap_or(0) as usize;

    let mut pack_lines = Vec::with_capacity(count);
    let mut songs = 0;
    let mut jingles = 0;
    for id in 0..count {
        let name = existing_names
            .get(&(id as u16))
            .cloned()
            .unwrap_or_else(|| format!("midi_{id}"));
        pack_lines.push(format!("{id}={name}"));

        let Some(data) = cache
            .read_group_raw_decompressed(archive::MIDI, id as u32)
            .ok()
            .flatten()
            .filter(|d| !d.is_empty())
        else {
            continue;
        };

        let is_jingle = version_list.midi_flags.get(id).copied().unwrap_or(0) != 0;
        let dir = if is_jingle { &jingles_dir } else { &songs_dir };
        fs::write(dir.join(format!("{name}.mid")), &data)?;
        if is_jingle {
            jingles += 1;
        } else {
            songs += 1;
        }
    }

    fs::write(pack_dir.join("midi.pack"), pack_lines.join("\n") + "\n")?;
    debug!("Unpacked {songs} songs + {jingles} jingles");
    Ok(())
}

#[cfg(since_647)]
pub fn unpack_songs(store: &mut Js5Store, pack_dir: &Path, content_dir: &Path) -> Result<u32> {
    let song_pack = PackFile::load(&pack_dir.join("song.pack")).ok();
    unpack_midi_archive(
        store,
        archive::SONGS,
        "songs",
        "mid",
        content_dir,
        song_pack.as_ref(),
    )
}

#[cfg(since_647)]
fn unpack_midi_archive(
    store: &mut Js5Store,
    archive_id: u8,
    dir_name: &str,
    extension: &str,
    content_dir: &Path,
    pack: Option<&PackFile>,
) -> Result<u32> {
    let archive_dir = content_dir.join(dir_name).join("_unpack");
    fs::create_dir_all(&archive_dir)?;

    let group_count = store.group_count(archive_id).unwrap_or(0);
    let mut groups = Vec::new();

    let mut written = 0u32;
    let mut failed = 0u32;

    for gid in 0..group_count {
        let Some(raw) = store.read_raw_group_bytes(archive_id, gid)? else {
            continue;
        };
        if raw.len() < 5 {
            continue;
        }

        let container = Container::parse(&raw)?;

        let decompressed = match decompress(&raw) {
            Ok(d) => d,
            Err(e) => {
                warn!(
                    "archive {} group {}: decompress failed ({})",
                    archive_id, gid, e
                );
                failed += 1;
                continue;
            }
        };

        let midi_bytes = match decode_jagex_midi(&decompressed) {
            Ok(m) => m,
            Err(e) => {
                warn!(
                    "archive {} group {}: MIDI decode failed ({})",
                    archive_id, gid, e
                );
                failed += 1;
                continue;
            }
        };

        let name = pack
            .and_then(|p| p.get_by_id(gid).map(|s| s.to_string()))
            .unwrap_or_else(|| gid.to_string());
        let path = archive_dir.join(format!("{}.{}", name, extension));
        fs::write(&path, &midi_bytes)?;

        groups.push(json!({
            "group_id": gid,
            "file_name": format!("{}.{}", name, extension),
            "compression": container.compression as u8,
            "version": container.version.unwrap_or(0),
            "compressed_len": container.compressed_len,
            "decompressed_len": container.uncompressed_len,
            "crc": container.crc(),
        }));
        written += 1;
    }

    let manifest = json!({
        "archive_id": archive_id,
        "archive_name": dir_name,
        "file_extension": extension,
        "groups": groups,
    });
    let manifest_path = archive_dir.join("manifest.json");
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    if failed > 0 {
        warn!("  {} groups failed to decode", failed);
    }
    info!("  {} {} decoded to standard MIDI", written, dir_name);
    Ok(written)
}
