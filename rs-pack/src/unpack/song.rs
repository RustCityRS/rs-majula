use std::path::Path;

use rs_io::bz2::bz2_decompress;
#[cfg(since_244)]
use rs_io::js5::Js5Store;
use tracing::debug;

pub fn unpack_songs(songs_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
    let out_dir = output_dir.join("songs");
    std::fs::create_dir_all(&out_dir)?;

    let Ok(entries) = std::fs::read_dir(songs_dir) else {
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

        let data = std::fs::read(&path)?;
        if data.len() < 4 {
            continue;
        }

        let uncompressed_size = ((data[0] as u32) << 24)
            | ((data[1] as u32) << 16)
            | ((data[2] as u32) << 8)
            | (data[3] as u32);

        let midi = bz2_decompress(&data[4..], uncompressed_size as usize, true, 0);

        let mid_name = key_to_filename(&key);
        std::fs::write(out_dir.join(&mid_name), &midi)?;
        count += 1;
    }

    debug!("Unpacked {} songs", count);
    Ok(())
}

fn key_to_filename(key: &str) -> String {
    if key.len() < 12
        && let Some(stem) = key.strip_suffix("_mid")
    {
        return format!("{stem}.mid");
    }
    format!("{key}.mid")
}

#[cfg(since_244)]
pub fn unpack_midi(
    cache: &Js5Store,
    version_list: &crate::version_list::VersionList,
    output_dir: &Path,
    pack_dir: &Path,
) -> anyhow::Result<()> {
    let songs_dir = output_dir.join("songs");
    let jingles_dir = output_dir.join("jingles");
    std::fs::create_dir_all(&songs_dir)?;
    std::fs::create_dir_all(&jingles_dir)?;

    let existing_names = super::model::load_existing_pack(pack_dir, "midi");
    let count = cache.count(3);

    let mut pack_lines = Vec::with_capacity(count);
    let mut songs = 0;
    let mut jingles = 0;
    for id in 0..count {
        let name = existing_names
            .get(&(id as u16))
            .cloned()
            .unwrap_or_else(|| format!("midi_{id}"));
        pack_lines.push(format!("{id}={name}"));

        let Some(data) = cache.read(3, id, true).filter(|d| !d.is_empty()) else {
            continue;
        };

        let is_jingle = version_list.midi_flags.get(id).copied().unwrap_or(0) != 0;
        let dir = if is_jingle { &jingles_dir } else { &songs_dir };
        std::fs::write(dir.join(format!("{name}.mid")), &data)?;
        if is_jingle {
            jingles += 1;
        } else {
            songs += 1;
        }
    }

    std::fs::write(pack_dir.join("midi.pack"), pack_lines.join("\n") + "\n")?;
    debug!("Unpacked {songs} songs + {jingles} jingles");
    Ok(())
}
