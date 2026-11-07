use crate::archive;
use crate::pack::names::{AssetNames, archive_name};
use crate::pack::other::{asset, wma, worldmap_source};
use anyhow::{Context, Result, ensure};
use rs_io::js5::{Entry, Js5Store};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;

pub fn unpack_metadata(store: &Js5Store, pack_dir: &Path, content_dir: &Path) -> Result<u32> {
    let dir = content_dir.join("versions");
    fs::create_dir_all(&dir)?;
    fs::create_dir_all(pack_dir)?;
    let mut hashes: BTreeMap<&str, BTreeMap<u32, i32>> = BTreeMap::new();
    let mut count = 0;
    for &aid in archive::ARCHIVE_IDS {
        let Some(table) = store.reference_table(aid) else {
            ensure!(
                store.group_count(aid).unwrap_or(0) == 0,
                "Missing archive {aid} reference table"
            );
            continue;
        };
        count += 1;
        let stem = archive::versions_filename(aid).context("Unknown archive metadata filename")?;
        let mut versions = format!("# table_version={}\n", table.version);
        let mut files = String::new();
        for group in &table.groups {
            writeln!(versions, "{}={}", group.id, group.version)?;
            if group.files.len() != 1
                || group
                    .files
                    .iter()
                    .any(|file| file.id != 0 || file.name_hash.is_some())
            {
                let entries = group
                    .files
                    .iter()
                    .map(|file| {
                        file.name_hash.map_or_else(
                            || file.id.to_string(),
                            |hash| format!("{}:{hash}", file.id),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                writeln!(files, "{}={entries}", group.id)?;
            }
            if table.named {
                let registry = hashes.entry(archive_name(aid)).or_default();
                if let Some(previous) = registry.insert(group.id, group.name_hash) {
                    ensure!(
                        previous == group.name_hash,
                        "Conflicting shared archive name hash for {aid}/{}",
                        group.id
                    );
                }
            }
        }
        fs::write(dir.join(format!("{stem}.versions")), versions)?;
        let path = dir.join(format!("{stem}.fileids"));
        if !files.is_empty() {
            fs::write(path, files)?;
        } else if path.exists() {
            fs::remove_file(path)?;
        }
    }
    for (name, groups) in hashes {
        let mut text = String::new();
        for (id, hash) in groups {
            writeln!(text, "{id}={hash}")?;
        }
        fs::write(pack_dir.join(format!("{name}.hashes")), text)?;
    }
    Ok(count)
}

pub fn unpack_archive(
    store: &mut Js5Store,
    aid: u8,
    pack_dir: &Path,
    content_dir: &Path,
) -> Result<u32> {
    let names = AssetNames::load(pack_dir)?;
    let format = archive::archive_format(aid).context("Unknown asset archive")?;
    let extension = asset::extension(aid).context("Unknown editable asset format")?;
    let dir = content_dir.join(format.content_dir);
    fs::create_dir_all(&dir)?;
    let groups = store
        .reference_table(aid)
        .context("Missing asset reference table")?
        .groups
        .clone();
    let world_maps = if aid == archive::WORLDMAP_DATA {
        unpack_world_maps(store, &dir, &names)?
    } else {
        Vec::new()
    };
    for info in &groups {
        if world_maps.contains(&info.id) {
            continue;
        }
        let path = dir.join(format!(
            "{}.{}",
            names.name(archive_name(aid), info.id),
            extension
        ));
        let group = store
            .read_group(aid, info.id)?
            .context("Missing asset group")?;
        let mut text = String::new();
        for entry in &group.entries {
            let kind = asset::entry_kind(aid, info.id);
            let name = names.name(
                kind.unwrap_or(archive_name(aid)),
                if kind.is_some() { entry.id } else { info.id },
            );
            if asset::has_sections(aid) {
                text.push_str(&format!("[{name}]\n"));
            }
            let body = asset::unpack(aid, info.id, entry.data(), &names)
                .with_context(|| format!("{aid}/{}/{}", info.id, entry.id))?;
            if aid == archive::WORLDMAP_DATA {
                text.push_str(&write_worldmap_squares(&path, entry.id, &body)?);
            } else {
                text.push_str(&body);
            }
            if asset::has_sections(aid) {
                text.push('\n');
            }
        }
        let original = store
            .read_group_raw_decompressed(aid, info.id)?
            .context("Missing asset payload")?;
        let source = (aid == archive::WORLDMAP_DATA)
            .then(|| worldmap_source::read_squares(&path, &text))
            .transpose()?;
        let packed = asset::pack_group(aid, info.id, source.as_deref().unwrap_or(&text), &names)?;
        ensure!(
            original == packed,
            "Text round trip changed archive {aid} group {}",
            info.id
        );
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, text)?;
    }
    Ok(groups.len() as u32)
}

/// Writes the `.wma` source and gives the groups it holds, once it reads back to their bytes.
fn unpack_world_maps(store: &mut Js5Store, dir: &Path, names: &AssetNames) -> Result<Vec<u32>> {
    let aid = archive::WORLDMAP_DATA;
    let kind = archive_name(aid);
    let details = names.id(kind, wma::DETAILS)?;
    let group = store
        .read_group(aid, details)?
        .context("Missing world map details")?;
    let mut original = vec![(details, entry_bytes(&group.entries))];
    let mut maps: Vec<_> = group
        .entries
        .iter()
        .map(|entry| wma::decode_details(entry.data()))
        .collect();
    for map in &mut maps {
        let Ok(id) = names.id(kind, &wma::elements_group(&map.group)) else {
            continue;
        };
        let Some(group) = store.read_group(aid, id)? else {
            continue;
        };
        map.elements = group
            .entries
            .iter()
            .map(|entry| wma::decode_element(entry.data()))
            .collect();
        original.push((id, entry_bytes(&group.entries)));
    }
    let text = wma::emit(&maps, names)?;
    let maps = wma::parse(&text, names)?;
    let mut packed = vec![maps.iter().map(wma::encode_details).collect::<Vec<_>>()];
    packed.extend(
        maps.iter()
            .filter(|map| !map.elements.is_empty())
            .map(|map| map.elements.iter().map(wma::encode_element).collect()),
    );
    ensure!(
        packed.len() == original.len() && packed.iter().zip(&original).all(|(a, (_, b))| a == b),
        "World map text round trip changed bytes"
    );
    fs::write(dir.join(wma::SOURCE), text)?;
    Ok(original.into_iter().map(|(id, _)| id).collect())
}

fn entry_bytes(entries: &[Entry]) -> Vec<Vec<u8>> {
    entries.iter().map(|entry| entry.data().to_vec()).collect()
}

pub fn write_worldmap_squares(path: &Path, file: u32, text: &str) -> Result<String> {
    let mut starts: Vec<_> = text
        .match_indices("\nchunk_kind=")
        .map(|(i, _)| i + 1)
        .collect();
    if starts.is_empty() {
        return Ok(text.to_owned());
    }
    starts.push(text.len());
    let mut result = text[..starts[0]].to_owned();
    let mut squares: Vec<(String, usize, usize)> = Vec::new();
    for bounds in starts.windows(2) {
        let mut lines = text[bounds[0]..bounds[1]].lines().skip(1);
        let x: u8 = lines
            .next()
            .and_then(|line| line.strip_prefix("map_x="))
            .context("Missing world map square x")?
            .parse()?;
        let z: u8 = lines
            .next()
            .and_then(|line| line.strip_prefix("map_z="))
            .context("Missing world map square z")?
            .parse()?;
        let name = format!("m{x}_{z}");
        if let Some((_, _, end)) = squares.last_mut().filter(|(last, _, _)| *last == name) {
            *end = bounds[1];
        } else {
            squares.push((name, bounds[0], bounds[1]));
        }
    }
    let parent = path.parent().context("World map source has no parent")?;
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .context("Invalid world map name")?;
    let dir = format!("{stem}/file_{file}");
    fs::create_dir_all(parent.join(&dir))?;
    let mut parts = BTreeMap::new();
    for (name, start, end) in squares {
        let part = parts.entry(name.clone()).or_insert(0);
        *part += 1;
        let suffix = if *part == 1 {
            String::new()
        } else {
            format!("_{part}")
        };
        let relative = format!("{dir}/{name}{suffix}.fms");
        fs::write(parent.join(&relative), &text[start..end])?;
        writeln!(result, "square={relative}")?;
    }
    Ok(result)
}
