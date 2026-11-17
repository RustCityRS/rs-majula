use crate::archive;
use crate::pack::names::{AssetNames, archive_name};
#[cfg(all(since_530, before_647))]
use crate::pack::other::worldmap_layers::{self, DETAILS, LAYERS, SQUARE};
use crate::pack::other::{asset, material};
#[cfg(since_647)]
use crate::pack::other::{wma, worldmap_source};
use anyhow::{Context, Result, ensure};
#[cfg(since_647)]
use rs_io::js5::Entry;
use rs_io::js5::Js5LocalCache;
#[cfg(all(since_530, before_647))]
use rs_io::js5::encode_group_entries;
use rs_io::stringtools::hash32;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;
use std::fs;
use std::path::Path;

/// Writes every archive's versions and file ids, and gives the archive count
/// and the groups its table lists without data.
pub fn unpack_metadata(
    store: &mut Js5LocalCache,
    pack_dir: &Path,
    content_dir: &Path,
) -> Result<(u32, Vec<String>)> {
    let dir = content_dir.join("versions");
    fs::create_dir_all(&dir)?;
    fs::create_dir_all(pack_dir)?;
    let mut hashes: BTreeMap<&str, BTreeMap<u32, i32>> = BTreeMap::new();
    let mut count = 0;
    let mut absent_groups = Vec::new();
    for &aid in archive::ARCHIVE_IDS {
        let Some(table) = store.index(aid).cloned() else {
            ensure!(
                store.group_id_limit(aid).unwrap_or(0) == 0,
                "Missing archive {aid} reference table"
            );
            continue;
        };
        count += 1;
        let stem = archive::versions_filename(aid).context("Unknown archive metadata filename")?;
        let mut versions = format!("# table_version={}\n", table.index_version);
        let mut files = String::new();
        for group in &table.groups {
            let absent = matches!(store.read_raw_group_bytes(aid, group.id), Ok(None));
            if absent {
                absent_groups.push(format!("{stem}/{}", group.id));
            }
            writeln!(
                versions,
                "{}",
                archive::group_version_line(group, absent, table.named)
            )?;
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
        match_names(pack_dir, name, &groups)?;
        let mut text = String::new();
        for (id, hash) in groups {
            writeln!(text, "{id}={hash}")?;
        }
        fs::write(pack_dir.join(format!("{name}.hashes")), text)?;
    }
    Ok((count, absent_groups))
}

/// Keeps each name whose group still carries the hash it was named under, or
/// that hashes to its group, and moves a name to the group that now carries
/// its hash: a revision can renumber and replace what a named archive holds.
/// The names under a group (interface.pack's `interface:component=name`) go
/// where its name goes.
fn match_names(pack_dir: &Path, kind: &str, hashes: &BTreeMap<u32, i32>) -> Result<()> {
    let path = pack_dir.join(format!("{kind}.pack"));
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let previous: HashMap<u32, i32> = fs::read_to_string(pack_dir.join(format!("{kind}.hashes")))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let (id, hash) = line.split_once('=')?;
            Some((id.parse().ok()?, hash.parse().ok()?))
        })
        .collect();
    let mut names = BTreeMap::new();
    let mut children: HashMap<u32, Vec<&str>> = HashMap::new();
    for line in text.lines() {
        let Some((key, name)) = line.split_once('=') else {
            continue;
        };
        if let Some((group, _)) = key.split_once(':') {
            if let Ok(id) = group.parse() {
                children
                    .entry(id)
                    .or_default()
                    .push(&line[group.len() + 1..]);
            }
        } else if let Ok(id) = key.parse::<u32>() {
            names.insert(id, name);
        }
    }
    let mut by_hash = HashMap::new();
    for (&id, &name) in &names {
        by_hash.insert(hash32(&name.to_lowercase()), (id, name));
        if let Some(&hash) = previous.get(&id) {
            by_hash.entry(hash).or_insert((id, name));
        }
    }
    let found: Vec<(u32, Option<(u32, &str)>)> = hashes
        .iter()
        .map(|(&id, hash)| (id, by_hash.get(hash).copied()))
        .collect();
    let moved: HashSet<u32> = found
        .iter()
        .filter_map(|&(_, name)| Some(name?.0))
        .collect();
    let mut matched = String::new();
    for (id, name) in found {
        let from = match name {
            Some((from, name)) => {
                writeln!(matched, "{id}={name}")?;
                from
            }
            None if moved.contains(&id) => continue,
            None => id,
        };
        for child in children.get(&from).into_iter().flatten() {
            writeln!(matched, "{id}:{child}")?;
        }
    }
    if matched != text {
        fs::write(path, matched)?;
    }
    Ok(())
}

pub fn unpack_archive(
    store: &mut Js5LocalCache,
    aid: u8,
    pack_dir: &Path,
    content_dir: &Path,
) -> Result<u32> {
    #[cfg(since_647)]
    if aid == archive::WORLDMAP {
        write_map_names(store, pack_dir)?;
    }
    let names = AssetNames::load(pack_dir)?;
    let format = archive::archive_format(aid).context("Unknown asset archive")?;
    let extension = asset::extension(aid).context("Unknown editable asset format")?;
    let dir = content_dir.join(format.content_dir);
    fs::create_dir_all(&dir)?;
    #[cfg(all(since_530, before_647))]
    if aid == archive::WORLDMAP {
        return unpack_world_map_layers(store, &dir, &names);
    }
    let groups = store
        .index(aid)
        .context("Missing asset reference table")?
        .groups
        .clone();
    #[cfg(since_647)]
    let world_maps = if aid == archive::WORLDMAP {
        unpack_world_maps(store, content_dir, &names)?
    } else {
        Vec::new()
    };
    for info in &groups {
        #[cfg(since_647)]
        if world_maps.contains(&info.id) {
            continue;
        }
        let path = dir.join(format!(
            "{}.{}",
            names.name(archive_name(aid), info.id),
            extension
        ));
        let Some(group) = store.read_group(aid, info.id)? else {
            continue;
        };
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
            #[cfg(since_647)]
            let body = if aid == archive::WORLDMAP {
                write_worldmap_squares(&path, entry.id, &body)?
            } else {
                body
            };
            text.push_str(&body);
            if asset::has_sections(aid) {
                text.push('\n');
            }
        }
        let original = store
            .read_group_raw_decompressed(aid, info.id)?
            .context("Missing asset payload")?;
        #[cfg(since_647)]
        let source = (aid == archive::WORLDMAP)
            .then(|| worldmap_source::read_squares(&path, &text))
            .transpose()?;
        #[cfg(since_647)]
        let source = source.as_deref().unwrap_or(&text);
        #[cfg(all(since_530, before_647))]
        let source = &text;
        let packed = asset::pack_group(aid, info.id, source, &names)?;
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

/// Writes a file for each material, once they read back to the archive's bytes.
pub fn unpack_materials(
    store: &mut Js5LocalCache,
    pack_dir: &Path,
    content_dir: &Path,
) -> Result<u32> {
    let names = AssetNames::load(pack_dir)?;
    let aid = archive::MATERIALS;
    let format = archive::archive_format(aid).context("Unknown asset archive")?;
    let dir = content_dir.join(format.content_dir);
    fs::create_dir_all(&dir)?;
    let original = store
        .read_group_raw_decompressed(aid, 0)?
        .context("Missing materials")?;
    let materials = material::unpack(&original, &names)?;
    ensure!(
        material::pack(&materials, &names)? == original,
        "Text round trip changed the materials"
    );
    for (&id, text) in &materials {
        let name = names.name(material::KIND, id);
        fs::write(dir.join(format!("{name}.{}", material::EXTENSION)), text)?;
    }
    Ok(materials.len() as u32)
}

/// Names each map's id in `wma.pack` by the group its details name, which is its position,
/// and names the archive groups that hash to a map's own name or its elements group.
#[cfg(since_647)]
fn write_map_names(store: &mut Js5LocalCache, pack_dir: &Path) -> Result<()> {
    let names = AssetNames::load(pack_dir)?;
    let kind = archive_name(archive::WORLDMAP);
    let details = names.id(kind, wma::DETAILS)?;
    let group = store
        .read_group(archive::WORLDMAP, details)?
        .context("Missing world map details")?;
    let mut text = String::new();
    let mut candidates = HashMap::new();
    for entry in &group.entries {
        let map = wma::decode_details(entry.data());
        writeln!(text, "{}={}", entry.id, map.group)?;
        for name in [map.group.to_lowercase(), wma::elements_group(&map.group)] {
            candidates.insert(hash32(&name), name);
        }
    }
    let path = pack_dir.join(format!("{}.pack", wma::KIND));
    if fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
        fs::write(path, text)?;
    }

    let path = pack_dir.join(format!("{kind}.pack"));
    let mut text = fs::read_to_string(&path).unwrap_or_default();
    let known: HashSet<u32> = text
        .lines()
        .filter_map(|line| line.split_once('=')?.0.parse().ok())
        .collect();
    let table = store
        .index(archive::WORLDMAP)
        .context("Missing world map table")?;
    let mut added = false;
    for info in &table.groups {
        if let Some(name) = candidates.get(&info.name_hash)
            && !known.contains(&info.id)
        {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            writeln!(text, "{}={name}", info.id)?;
            added = true;
        }
    }
    if added {
        fs::write(path, text)?;
    }
    Ok(())
}

/// Writes the `.wma` config and gives the groups it holds, once it reads back to their bytes.
#[cfg(since_647)]
fn unpack_world_maps(
    store: &mut Js5LocalCache,
    content_dir: &Path,
    names: &AssetNames,
) -> Result<Vec<u32>> {
    let aid = archive::WORLDMAP;
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
    let mut packed = wma::groups(wma::parse(&text, names)?, names)?;
    packed.sort_by_key(|(id, _)| *id);
    original.sort_by_key(|(id, _)| *id);
    ensure!(
        packed == original,
        "World map text round trip changed bytes"
    );
    let dir = content_dir.join("scripts/_unpack");
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(wma::SOURCE), text)?;
    Ok(original.into_iter().map(|(id, _)| id).collect())
}

#[cfg(since_647)]
fn entry_bytes(entries: &[Entry]) -> Vec<Vec<u8>> {
    entries.iter().map(|entry| entry.data().to_vec()).collect()
}

#[cfg(since_647)]
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

/// Writes each world map group, with its layers' squares, once it packs back to the group's bytes.
#[cfg(all(since_530, before_647))]
fn unpack_world_map_layers(
    store: &mut Js5LocalCache,
    dir: &Path,
    names: &AssetNames,
) -> Result<u32> {
    let aid = archive::WORLDMAP;
    let groups = store
        .index(aid)
        .context("Missing asset reference table")?
        .groups
        .clone();
    let details = hash32(DETAILS);
    let mut formats = HashMap::from([(details, DETAILS)]);
    if let Some(info) = groups.iter().find(|info| info.name_hash == details) {
        let group = store
            .read_group(aid, info.id)?
            .context("Missing world map details")?;
        for entry in &group.entries {
            let map = worldmap_layers::unpack_details(entry.data(), names)?
                .0
                .to_lowercase();
            formats.insert(hash32(&map), "map");
            formats.insert(hash32(&format!("{map}_labels")), "label");
        }
    }
    for info in &groups {
        let format = *formats
            .get(&info.name_hash)
            .with_context(|| format!("Unknown world map group {}", info.id))?;
        let stem = names.name(archive_name(aid), info.id);
        let path = dir.join(format!("{stem}.{}", worldmap_layers::EXTENSION));
        let Some(group) = store.read_group(aid, info.id)? else {
            continue;
        };
        let mut text = String::new();
        let mut parts: HashMap<_, u32> = HashMap::new();
        for entry in &group.entries {
            writeln!(text, "[{}]", names.name("file", entry.id))?;
            match format {
                DETAILS => text.push_str(&worldmap_layers::unpack_details(entry.data(), names)?.1),
                "label" => text.push_str(&worldmap_layers::unpack_label(entry.data(), names)?),
                _ => {
                    let hash = info
                        .files
                        .iter()
                        .find(|file| file.id == entry.id)
                        .and_then(|file| file.name_hash);
                    let layer = LAYERS
                        .into_iter()
                        .find(|&layer| Some(hash32(layer)) == hash)
                        .with_context(|| {
                            format!("Unknown world map layer {}/{}", info.id, entry.id)
                        })?;
                    writeln!(text, "format={layer}")?;
                    fs::create_dir_all(dir.join(&stem).join(layer))?;
                    for square in worldmap_layers::unpack_squares(layer, entry.data(), names)? {
                        let part = parts.entry((layer, square.name.clone())).or_default();
                        *part += 1;
                        let suffix = if *part == 1 {
                            String::new()
                        } else {
                            format!("_{part}")
                        };
                        let relative = format!("{stem}/{layer}/{}{suffix}.{SQUARE}", square.name);
                        fs::write(dir.join(&relative), square.text)?;
                        writeln!(text, "square={relative}")?;
                    }
                }
            }
            text.push('\n');
        }
        fs::write(&path, text)?;
        let entries = asset::pack_file(aid, info.id, &path, names)?;
        let refs: Vec<_> = entries
            .iter()
            .map(|(id, data)| (*id, data.as_slice()))
            .collect();
        let original = store
            .read_group_raw_decompressed(aid, info.id)?
            .context("Missing asset payload")?;
        ensure!(
            original == encode_group_entries(&refs),
            "Text round trip changed archive {aid} group {}",
            info.id
        );
    }
    Ok(groups.len() as u32)
}
