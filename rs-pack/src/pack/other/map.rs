#[cfg(before_647)]
use super::worldmap::WorldmapBuilder;
#[cfg(since_647)]
use crate::archive::{self, ArchiveVersions};
#[cfg(since_647)]
use crate::cache::js5::PackedCache;
use crate::cache::map::TerrainTile;
#[cfg(since_647)]
use crate::cache::map::{
    CameraData, CameraPlane, EnvParams, EnvironmentData, LightData, LocEntry, NpcSpawn, ObjSpawn,
    SkyboxData, TerrainData, load_xtea_keys,
};
#[cfg(since_647)]
use crate::pack::js5::prepare_group;
#[cfg(since_647)]
use crate::pack::js5::{PreparedGroup, group_version, insert_archive};
#[cfg(since_647)]
use crate::pack::pack_registry::PackFile;
#[cfg(since_647)]
use crate::pack::packed_data::PackedData;
#[cfg(before_647)]
use crate::types::{MapSquareCrcs, MapSquareCsv, MapSquares};
#[cfg(before_647)]
use crate::zone_csv::load_csv_zones;
#[cfg(before_647)]
pub(super) use crate::zone_csv::zone_key;
#[cfg(since_647)]
use anyhow::Result;
#[cfg(since_647)]
use rayon::prelude::*;
#[cfg(before_647)]
use rs_io::Packet;
#[cfg(before_647)]
use rs_io::bz2::bz2_compress_with_size;
#[cfg(before_647)]
use rs_io::crc;
#[cfg(since_647)]
use rs_io::js5::compression::{CompressionType, Container, compress_xtea};
#[cfg(since_647)]
use rs_io::js5::compute_group_crc;
#[cfg(since_647)]
use rs_io::js5::java_string_hash;
#[cfg(since_647)]
use rs_io::js5::reference::FileInfo;
#[cfg(since_647)]
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
#[cfg(since_647)]
use std::path::PathBuf;
#[cfg(before_647)]
use std::sync::Arc;
#[cfg(before_647)]
use tracing::debug;
#[cfg(since_647)]
use tracing::{info, warn};

#[cfg(before_647)]
type PackedMapSquare = (
    MapSquares,
    MapSquareCrcs,
    MapSquareCsv,
    MapSquareCsv,
    WorldmapBuilder,
);

#[cfg(before_647)]
pub(super) struct LocPlacement {
    pub(super) y: u8,
    pub(super) x: u8,
    pub(super) z: u8,
    pub(super) shape: u8,
    pub(super) angle: u8,
}

#[cfg(before_647)]
pub(super) struct NpcPlacement {
    pub(super) y: u8,
    pub(super) x: u8,
    pub(super) z: u8,
}

#[cfg(before_647)]
pub(super) struct ObjPlacement {
    pub(super) y: u8,
    pub(super) x: u8,
    pub(super) z: u8,
    count: u16,
}

#[cfg(before_647)]
pub struct ParsedMap {
    pub(super) locs: HashMap<u16, Vec<LocPlacement>>,
    pub(super) npcs: HashMap<u16, Vec<NpcPlacement>>,
    pub(super) objs: HashMap<u16, Vec<ObjPlacement>>,
}

#[cfg(before_647)]
pub(super) const TILE_COUNT: usize = 4 << 12;

#[cfg(before_647)]
pub(super) fn tile_index(y: usize, x: usize, z: usize) -> usize {
    y << 12 | x << 6 | z
}

#[cfg(before_647)]
fn fast_parse_int(s: &[u8]) -> i32 {
    let mut i = 0;
    let neg = i < s.len() && s[i] == b'-';
    if neg {
        i += 1;
    }
    let mut n: i32 = 0;
    while i < s.len() && s[i] >= b'0' && s[i] <= b'9' {
        n = n * 10 + (s[i] - b'0') as i32;
        i += 1;
    }
    if neg { -n } else { n }
}

#[cfg(before_647)]
fn fast_parse_usize(s: &[u8]) -> usize {
    let mut n: usize = 0;
    for &b in s {
        if b.is_ascii_digit() {
            n = n * 10 + (b - b'0') as usize;
        } else {
            break;
        }
    }
    n
}

#[cfg(before_647)]
fn next_word(bytes: &[u8], start: usize) -> (usize, usize) {
    let mut i = start;
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    let begin = i;
    while i < bytes.len() && bytes[i] != b' ' && bytes[i] != b'\n' && bytes[i] != b'\r' {
        i += 1;
    }
    (begin, i)
}

#[cfg(before_647)]
fn read_map(bytes: &[u8], tiles: &mut [TerrainTile; TILE_COUNT]) -> ParsedMap {
    tiles.fill(TerrainTile::default());

    let mut locs: HashMap<u16, Vec<LocPlacement>> = HashMap::new();
    let mut npcs: HashMap<u16, Vec<NpcPlacement>> = HashMap::new();
    let mut objs: HashMap<u16, Vec<ObjPlacement>> = HashMap::new();
    let mut section: u8 = 0;

    let mut pos = 0;
    let len = bytes.len();

    while pos < len {
        let line_start = pos;
        while pos < len && bytes[pos] != b'\n' {
            pos += 1;
        }
        let mut line_end = pos;
        if line_end > line_start && bytes[line_end - 1] == b'\r' {
            line_end -= 1;
        }
        if pos < len {
            pos += 1;
        }

        if line_start == line_end {
            continue;
        }
        let lb = &bytes[line_start..line_end];

        if lb[0] == b'=' {
            if memchr3(b'M', lb).is_some() {
                section = 1;
            } else if memchr3(b'L', lb).is_some() {
                section = 2;
            } else if memchr3(b'N', lb).is_some() {
                section = 3;
            } else if memchr3(b'O', lb).is_some() {
                section = 4;
            } else {
                section = 0;
            }
            continue;
        }

        if section == 0 {
            continue;
        }

        let Some(colon) = lb.iter().position(|&b| b == b':') else {
            continue;
        };
        let coords = &lb[..colon];
        let data = &lb[colon + 1..];

        let (s, e) = next_word(coords, 0);
        let y = fast_parse_usize(&coords[s..e]);
        let (s, e) = next_word(coords, e);
        let x = fast_parse_usize(&coords[s..e]);
        let (s, e) = next_word(coords, e);
        let z = fast_parse_usize(&coords[s..e]);

        if section == 1 {
            if y >= 4 || x >= 64 || z >= 64 {
                continue;
            }
            let tile = &mut tiles[tile_index(y, x, z)];
            let mut wpos = 0;
            loop {
                let (ws, we) = next_word(data, wpos);
                if ws >= we {
                    break;
                }
                wpos = we;
                let word = &data[ws..we];
                match word[0] {
                    b'h' => {
                        let h = fast_parse_int(&word[1..]);
                        tile.height = (h != 0).then_some(h as u8);
                    }
                    b'o' => {
                        let rest = &word[1..];
                        let mut semi1 = rest.len();
                        let mut semi2 = rest.len();
                        for (i, &b) in rest.iter().enumerate() {
                            if b == b';' {
                                if semi1 == rest.len() {
                                    semi1 = i;
                                } else {
                                    semi2 = i;
                                    break;
                                }
                            }
                        }
                        tile.overlay_id = Some(fast_parse_int(&rest[..semi1]) as u8);
                        if semi1 < rest.len() {
                            tile.overlay_shape =
                                fast_parse_int(&rest[semi1 + 1..semi2.min(rest.len())]) as u8;
                        }
                        if semi2 < rest.len() {
                            tile.overlay_rotation = fast_parse_int(&rest[semi2 + 1..]) as u8;
                        }
                    }
                    b'f' => tile.flags = Some(fast_parse_int(&word[1..]) as u8),
                    b'u' => tile.underlay_id = Some(fast_parse_int(&word[1..]) as u8),
                    _ => {}
                }
            }
        } else if section == 2 {
            let (s0, e0) = next_word(data, 0);
            if s0 >= e0 {
                continue;
            }
            let id = fast_parse_int(&data[s0..e0]) as u16;
            let (s1, e1) = next_word(data, e0);
            let shape = if s1 < e1 {
                fast_parse_int(&data[s1..e1]) as u8
            } else {
                10
            };
            let (s2, e2) = next_word(data, e1);
            let angle = if s2 < e2 {
                fast_parse_int(&data[s2..e2]) as u8
            } else {
                0
            };

            locs.entry(id).or_default().push(LocPlacement {
                y: y as u8,
                x: x as u8,
                z: z as u8,
                shape,
                angle,
            });
        } else if section == 3 {
            let (s0, e0) = next_word(data, 0);
            if s0 >= e0 {
                continue;
            }
            let id = fast_parse_int(&data[s0..e0]) as u16;

            npcs.entry(id).or_default().push(NpcPlacement {
                y: y as u8,
                x: x as u8,
                z: z as u8,
            });
        } else if section == 4 {
            let (s0, e0) = next_word(data, 0);
            if s0 >= e0 {
                continue;
            }
            let id = fast_parse_int(&data[s0..e0]) as u16;
            let (s1, e1) = next_word(data, e0);
            let count = if s1 < e1 {
                fast_parse_int(&data[s1..e1]) as u16
            } else {
                1
            };

            objs.entry(id).or_default().push(ObjPlacement {
                y: y as u8,
                x: x as u8,
                z: z as u8,
                count,
            });
        }
    }

    ParsedMap { locs, npcs, objs }
}

#[cfg(before_647)]
fn memchr3(needle: u8, haystack: &[u8]) -> Option<usize> {
    haystack.iter().position(|&b| b == needle)
}

#[cfg(before_647)]
pub fn encode_locs(map: &ParsedMap, out: &mut Packet) {
    out.pos = 0;

    let mut loc_ids: Vec<u16> = map.locs.keys().copied().collect();
    loc_ids.sort();

    let mut last_loc_id: i32 = -1;

    for &loc_id in &loc_ids {
        out.psmart1or2(loc_id as i32 - last_loc_id);
        last_loc_id = loc_id as i32;

        let entries = map.locs.get(&loc_id).unwrap();
        let mut last_pos: i32 = 0;

        for e in entries {
            let current_pos = ((e.y as i32) << 12) | ((e.x as i32) << 6) | (e.z as i32);
            out.psmart1or2(current_pos - last_pos + 1);
            last_pos = current_pos;

            let loc_info = (e.shape << 2) | (e.angle & 0x3);
            out.p1(loc_info);
        }

        out.psmart1or2(0);
    }

    out.psmart1or2(0);
}

#[cfg(before_647)]
pub fn encode_npcs(map: &ParsedMap, out: &mut Packet) {
    out.pos = 0;

    let mut npc_ids: Vec<u16> = map.npcs.keys().copied().collect();
    npc_ids.sort();

    let mut last_npc_id: i32 = -1;

    for &npc_id in &npc_ids {
        out.psmart1or2(npc_id as i32 - last_npc_id);
        last_npc_id = npc_id as i32;

        let entries = map.npcs.get(&npc_id).unwrap();
        let mut last_pos: i32 = 0;

        for e in entries {
            let current_pos = ((e.y as i32) << 12) | ((e.x as i32) << 6) | (e.z as i32);
            out.psmart1or2(current_pos - last_pos + 1);
            last_pos = current_pos;
        }

        out.psmart1or2(0);
    }

    out.psmart1or2(0);
}

#[cfg(before_647)]
pub fn encode_objs(map: &ParsedMap, out: &mut Packet) {
    out.pos = 0;

    let mut obj_ids: Vec<u16> = map.objs.keys().copied().collect();
    obj_ids.sort();

    let mut last_obj_id: i32 = -1;

    for &obj_id in &obj_ids {
        out.psmart1or2(obj_id as i32 - last_obj_id);
        last_obj_id = obj_id as i32;

        let entries = map.objs.get(&obj_id).unwrap();
        let mut last_pos: i32 = 0;

        for e in entries {
            let current_pos = ((e.y as i32) << 12) | ((e.x as i32) << 6) | (e.z as i32);
            out.psmart1or2(current_pos - last_pos + 1);
            last_pos = current_pos;

            out.p2(e.count);
        }

        out.psmart1or2(0);
    }

    out.psmart1or2(0);
}

#[cfg(since_244)]
#[cfg(before_647)]
pub fn encode_jm2(jm2_bytes: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut tiles = [TerrainTile::default(); TILE_COUNT];
    let parsed = read_map(jm2_bytes, &mut tiles);

    let mut terrain = Vec::with_capacity(TILE_COUNT * 7);
    encode_tiles(&tiles, &mut terrain);

    let mut loc = Packet::new(64 * 1024);
    encode_locs(&parsed, &mut loc);

    (terrain, loc.data[..loc.pos].to_vec())
}

#[cfg(before_647)]
pub fn pack_maps(content_dir: &Path) -> PackedMapSquare {
    let maps_dir = content_dir.join("maps");
    let mut mapsquares = MapSquares::default();
    let mut mapcrcs = MapSquareCrcs::new();

    let multimap = load_csv_zones(&maps_dir.join("multiway.csv"));
    let freemap = load_csv_zones(&maps_dir.join("free2play.csv"));
    debug!(
        "Zone flags: {} multi, {} free-to-play",
        multimap.len(),
        freemap.len()
    );
    let mut worldmap = WorldmapBuilder::new(load_csv_zones(&maps_dir.join("ignore.csv")));

    if !maps_dir.exists() {
        return (mapsquares, mapcrcs, multimap, freemap, worldmap);
    }

    let Ok(entries) = fs::read_dir(&maps_dir) else {
        return (mapsquares, mapcrcs, multimap, freemap, worldmap);
    };

    let mut jm2_files: Vec<_> = entries
        .flatten()
        .filter(|e| {
            let path = e.path();
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("jm2"))
        })
        .collect();
    jm2_files.sort_by_key(|e| e.file_name());

    let mut tiles = [TerrainTile::default(); TILE_COUNT];
    let mut terrain_buf = Vec::with_capacity(TILE_COUNT * 7);
    let mut loc_buf = Packet::new(64 * 1024);
    let mut npc_buf = Packet::new(16 * 1024);
    let mut obj_buf = Packet::new(16 * 1024);

    let mut count = 0;

    for entry in jm2_files {
        let path = entry.path();
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        if !stem.starts_with('m') {
            continue;
        }
        let parts: Vec<&str> = stem[1..].split('_').collect();
        if parts.len() != 2 {
            continue;
        }
        let Ok(map_x) = parts[0].parse::<u8>() else {
            continue;
        };
        let Ok(map_z) = parts[1].parse::<u8>() else {
            continue;
        };

        let Ok(file_bytes) = fs::read(&path) else {
            continue;
        };

        let map_data = read_map(&file_bytes, &mut tiles);
        worldmap.add_square(map_x, map_z, &tiles, &map_data);

        terrain_buf.clear();
        encode_tiles(&tiles, &mut terrain_buf);
        let terrain_compressed = bz2_compress_with_size(&terrain_buf);
        let terrain_crc = crc::getcrc(&terrain_compressed, 0, terrain_compressed.len());
        mapsquares.insert(('m', map_x, map_z), Arc::from(terrain_compressed));
        mapcrcs.insert(('m', map_x, map_z), terrain_crc);

        encode_locs(&map_data, &mut loc_buf);
        let loc_compressed = bz2_compress_with_size(&loc_buf.data[..loc_buf.pos]);
        let loc_crc = crc::getcrc(&loc_compressed, 0, loc_compressed.len());
        mapsquares.insert(('l', map_x, map_z), Arc::from(loc_compressed));
        mapcrcs.insert(('l', map_x, map_z), loc_crc);

        if !map_data.npcs.is_empty() {
            encode_npcs(&map_data, &mut npc_buf);
            let npc_compressed = bz2_compress_with_size(&npc_buf.data[..npc_buf.pos]);
            let npc_crc = crc::getcrc(&npc_compressed, 0, npc_compressed.len());
            mapsquares.insert(('n', map_x, map_z), Arc::from(npc_compressed));
            mapcrcs.insert(('n', map_x, map_z), npc_crc);
        }

        if !map_data.objs.is_empty() {
            encode_objs(&map_data, &mut obj_buf);
            let obj_compressed = bz2_compress_with_size(&obj_buf.data[..obj_buf.pos]);
            let obj_crc = crc::getcrc(&obj_compressed, 0, obj_compressed.len());
            mapsquares.insert(('o', map_x, map_z), Arc::from(obj_compressed));
            mapcrcs.insert(('o', map_x, map_z), obj_crc);
        }

        count += 1;
    }

    debug!("Packed {} map squares ({} files)", count, mapsquares.len());
    (mapsquares, mapcrcs, multimap, freemap, worldmap)
}

#[cfg(since_647)]
pub fn pack_spawns(map_dirs: &[PathBuf]) -> Result<PackedData> {
    let entries = collect(map_dirs, 'n', is_npc_spawns, |text| {
        let spawns = parse_npc_spawn_text(text);
        (!spawns.is_empty()).then(|| encode_npc_spawn_binary(&spawns))
    })?;
    info!("  {} NPC spawn regions packed", entries.len());
    Ok(pack_spawn_entries(&entries))
}

/// Pack global ground-item spawns. Same on-disk container shape as the NPC
/// packer; the engine merges these as the `o` mapsquare layer and loads them
/// as `EntityLifeTime::Respawn` objs.
#[cfg(since_647)]
pub fn pack_obj_spawns(map_dirs: &[PathBuf]) -> Result<PackedData> {
    let entries = collect(map_dirs, 'o', is_obj_spawns, |text| {
        let spawns = parse_obj_spawn_text(text);
        (!spawns.is_empty()).then(|| encode_obj_spawn_binary(&spawns))
    })?;
    info!("  {} obj spawn regions packed", entries.len());
    Ok(pack_spawn_entries(&entries))
}

#[cfg(since_647)]
type Entry = (u8, u8, Vec<u8>);

#[cfg(since_647)]
fn collect(
    map_dirs: &[PathBuf],
    prefix: char,
    is_kind: fn(&str) -> bool,
    encode: impl Fn(&str) -> Option<Vec<u8>>,
) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for (stem, path) in squares(map_dirs) {
        if !is_kind(&stem) {
            continue;
        }
        let stripped = stem.strip_prefix(prefix).unwrap_or(&stem);
        let Some((mx, mz)) = stripped.split_once('_') else {
            continue;
        };
        let (Ok(mx), Ok(mz)) = (mx.parse::<u8>(), mz.parse::<u8>()) else {
            continue;
        };
        if let Some(binary) = encode(&fs::read_to_string(&path)?) {
            entries.push((mx, mz, binary));
        }
    }
    entries.sort_by_key(|(mx, mz, _)| (*mx as u16) << 8 | *mz as u16);
    Ok(entries)
}

#[cfg(since_647)]
fn pack_spawn_entries(entries: &[Entry]) -> PackedData {
    let mut packed = PackedData::new(entries.len() as u32);
    for (mx, mz, binary) in entries {
        packed.start_entry();
        packed.p1(*mx);
        packed.p1(*mz);
        packed.dat.extend_from_slice(binary);
        packed.finish_entry_raw();
    }
    packed
}

#[cfg(since_647)]
pub fn pack_archive(
    store: &mut PackedCache,
    content_dir: &Path,
    pack_dir: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
) -> Result<()> {
    let aid = archive::MAPS;
    let format = archive::archive_format(aid).unwrap();
    // Opaque squares (no key exists) are cache output, so only `_unpack`
    // holds them; text squares may come from any source directory.
    let map_dir = content_dir.join("maps").join("_unpack");
    let map_dirs = source_dirs(content_dir);
    let sources = squares(&map_dirs);

    let map_pack = PackFile::load(&pack_dir.join("map.pack")).unwrap_or_else(|_| PackFile::empty());

    let keys_candidates = [content_dir.join("keys.json")];
    let xtea_keys = if let Some(path) = keys_candidates.iter().find(|p| p.exists()) {
        load_xtea_keys(path).unwrap_or_default()
    } else {
        anyhow::bail!("No keys.json found in output dir or content dir");
    };

    let mut groups: Vec<(u32, String)> = map_pack
        .id_to_debugname
        .iter()
        .map(|(&id, name)| (id, name.clone()))
        .collect();
    groups.sort_by_key(|(id, _)| *id);

    let prepared: Vec<PreparedGroup> = groups
        .par_iter()
        .filter_map(|(gid, name)| {
            let ver = group_version(versions, aid, gid);
            let version = ver as u16;
            let name_hash = java_string_hash(name);

            let is_landscape = is_landscape(name);
            let is_npc = is_npc_spawns(name);

            let (raw, crc) = if is_landscape {
                let text_path = sources.get(name);
                let encrypted_path = map_dir.join(format!("encrypted/{}.bin", gid));

                if let Some(text_path) = text_path {
                    let text = fs::read_to_string(text_path).ok()?;
                    let locs = parse_landscape_text(&text);
                    let binary = encode_landscape_binary(&locs);
                    let key = xtea_keys.get(&name_hash).copied().unwrap_or([0; 4]);
                    let container =
                        compress_xtea(&binary, CompressionType::Gzip, version, &key).ok()?;
                    let crc = compute_group_crc(&container[..container.len() - 2]);
                    (container, crc)
                } else if encrypted_path.exists() {
                    let raw = fs::read(&encrypted_path).ok()?;
                    let crc = Container::parse(&raw).ok()?.crc();
                    (raw, crc)
                } else {
                    warn!(
                        "  archive 5 group {}: missing file for landscape {}",
                        gid, name
                    );
                    return None;
                }
            } else if is_npc {
                let text_path = sources.get(name)?;
                let text = fs::read_to_string(text_path).ok()?;
                let spawns = parse_npc_spawn_text(&text);
                let binary = encode_npc_spawn_binary(&spawns);
                prepare_group(aid, *gid, &binary, version, false).ok()?
            } else {
                let text_path = sources.get(name)?;
                let text = fs::read_to_string(text_path).ok()?;
                let terrain = parse_terrain_text(&text);
                let binary = encode_terrain_binary(&terrain);
                prepare_group(aid, *gid, &binary, version, false).ok()?
            };

            Some(PreparedGroup {
                id: *gid,
                raw,
                crc,
                version: ver,
                size: 0,
                name_hash,
                whirlpool: None,
                files: vec![FileInfo {
                    id: 0,
                    name_hash: None,
                }],
            })
        })
        .collect();

    insert_archive(
        store,
        aid,
        format,
        versions,
        prepared,
        "from text→binary encode",
    )?;

    Ok(())
}

fn encode_tiles(tiles: &[TerrainTile], out: &mut Vec<u8>) {
    for tile in tiles {
        if let Some(id) = tile.overlay_id {
            let opcode = 2u8 + (tile.overlay_shape << 2) + tile.overlay_rotation;
            out.push(opcode);
            out.push(id);
        }

        if let Some(f) = tile.flags {
            out.push(f + 49);
        }

        if let Some(u) = tile.underlay_id {
            out.push(u + 81);
        }

        if let Some(h) = tile.height {
            out.push(1);
            out.push(h);
        } else {
            out.push(0);
        }
    }
}

#[cfg(since_647)]
pub fn source_dirs(content_dir: &Path) -> [PathBuf; 3] {
    let maps = content_dir.join("maps");
    [
        maps.join("_unpack"),
        maps.join("spawns"),
        maps.join("donor"),
    ]
}

#[cfg(since_647)]
pub fn find_square(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    dirs.iter()
        .rev()
        .map(|dir| dir.join(format!("{name}.jm2")))
        .find(|path| path.is_file())
}

#[cfg(since_647)]
pub fn squares(dirs: &[PathBuf]) -> BTreeMap<String, PathBuf> {
    let mut out = BTreeMap::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jm2") {
                continue;
            }
            if !entry
                .file_type()
                .is_ok_and(|kind| kind.is_file() || kind.is_symlink() && path.is_file())
            {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                out.insert(stem.to_string(), path);
            }
        }
    }
    out
}

#[cfg(since_647)]
pub fn is_landscape(name: &str) -> bool {
    name.starts_with("l") || name.starts_with("ul")
}

#[cfg(since_647)]
pub fn is_npc_spawns(name: &str) -> bool {
    name.starts_with("n") && !name.starts_with("nl")
}

#[cfg(since_647)]
pub fn is_obj_spawns(name: &str) -> bool {
    name.starts_with('o')
}

#[cfg(since_647)]
pub fn encode_terrain_binary(terrain: &TerrainData) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 << 12);

    encode_tiles(&terrain.tiles, &mut out);

    for &opcode in &terrain.env_opcode_order {
        match opcode {
            0 => {
                if let Some(env) = &terrain.environment {
                    out.push(0);
                    encode_environment(env, &mut out);
                }
            }
            1 => {
                if !terrain.lights.is_empty() {
                    out.push(1);
                    out.push(terrain.lights.len() as u8);
                    for light in &terrain.lights {
                        encode_light(light, &mut out);
                    }
                }
            }
            2 => {
                if let Some(params) = &terrain.env_params {
                    out.push(2);
                    out.extend_from_slice(&params.values);
                }
            }
            128 => {
                if let Some(sky) = &terrain.skybox {
                    out.push(128);
                    write_u16(&mut out, sky.param0);
                    write_i16(&mut out, sky.param1);
                    write_i16(&mut out, sky.param2);
                    write_i16(&mut out, sky.param3);
                    write_u16(&mut out, sky.param4);
                }
            }
            129 => {
                if let Some(cam) = &terrain.camera {
                    out.push(129);
                    for plane in &cam.planes {
                        match plane {
                            CameraPlane::Clear => out.push(0),
                            CameraPlane::Data(cells) => {
                                out.push(1);
                                for &c in cells {
                                    out.push(c as u8);
                                }
                            }
                            CameraPlane::CopyPrev => out.push(2),
                        }
                    }
                }
            }
            _ => {}
        }
    }

    out
}

#[cfg(since_647)]
fn encode_environment(env: &EnvironmentData, out: &mut Vec<u8>) {
    let mut flags = 0u8;
    if env.sun_color.is_some() {
        flags |= 0x01;
    }
    if env.ambient.is_some() {
        flags |= 0x02;
    }
    if env.sunlight.is_some() {
        flags |= 0x04;
    }
    if env.backlight.is_some() {
        flags |= 0x08;
    }
    if env.sun_angle.is_some() {
        flags |= 0x10;
    }
    if env.fog_color.is_some() {
        flags |= 0x20;
    }
    if env.fog_depth.is_some() {
        flags |= 0x40;
    }
    if env.env_map.is_some() {
        flags |= 0x80;
    }
    out.push(flags);
    if let Some(v) = env.sun_color {
        write_u32(out, v);
    }
    if let Some(v) = env.ambient {
        write_u16(out, v);
    }
    if let Some(v) = env.sunlight {
        write_u16(out, v);
    }
    if let Some(v) = env.backlight {
        write_u16(out, v);
    }
    if let Some(a) = env.sun_angle {
        write_i16(out, a[0]);
        write_i16(out, a[1]);
        write_i16(out, a[2]);
    }
    if let Some(v) = env.fog_color {
        write_u32(out, v);
    }
    if let Some(v) = env.fog_depth {
        write_u16(out, v);
    }
    if let Some(m) = env.env_map {
        for v in m {
            write_u16(out, v);
        }
    }
}

#[cfg(since_647)]
fn encode_light(light: &LightData, out: &mut Vec<u8>) {
    out.push(light.flags_raw);
    write_u16(out, light.x);
    write_u16(out, light.y);
    write_u16(out, light.z);
    out.push(light.radius);
    for &c in &light.colors {
        write_u16(out, c);
    }
    write_u16(out, light.hsl);
    out.push(light.type_packed);
    if let Some(r) = light.light_type_ref {
        write_u16(out, r);
    }
}

#[cfg(since_647)]
fn write_u16(out: &mut Vec<u8>, v: u16) {
    out.push((v >> 8) as u8);
    out.push(v as u8);
}

#[cfg(since_647)]
fn write_i16(out: &mut Vec<u8>, v: i16) {
    write_u16(out, v as u16);
}

#[cfg(since_647)]
fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.push((v >> 24) as u8);
    out.push((v >> 16) as u8);
    out.push((v >> 8) as u8);
    out.push(v as u8);
}

#[cfg(since_647)]
pub fn encode_landscape_binary(locs: &[LocEntry]) -> Vec<u8> {
    let mut sorted: Vec<&LocEntry> = locs.iter().collect();
    sorted.sort_by_key(|l| (l.loc_id, l.packed_pos()));

    let mut out = Vec::new();
    let mut last_id: i32 = -1;
    let mut i = 0;

    while i < sorted.len() {
        let id = sorted[i].loc_id as i32;
        write_lsmart(&mut out, id - last_id);
        last_id = id;

        let mut last_pos: i32 = 0;
        while i < sorted.len() && sorted[i].loc_id as i32 == id {
            let loc = sorted[i];
            let pos = loc.packed_pos();
            write_smart(&mut out, pos - last_pos + 1);
            last_pos = pos;
            out.push((loc.shape << 2) | loc.rotation);
            i += 1;
        }

        write_smart(&mut out, 0);
    }

    write_smart(&mut out, 0);
    out
}

#[cfg(since_647)]
pub fn parse_npc_spawn_text(text: &str) -> Vec<NpcSpawn> {
    let mut spawns = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((coords, rest)) = line.split_once(':') else {
            continue;
        };
        let mut cp = coords.split_whitespace();
        let level: u8 = cp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let x: u8 = cp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let z: u8 = cp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let npc_id: u16 = rest.trim().parse().unwrap_or(0);
        spawns.push(NpcSpawn {
            level,
            x,
            z,
            npc_id,
        });
    }
    spawns
}

#[cfg(since_647)]
pub fn encode_npc_spawn_binary(spawns: &[NpcSpawn]) -> Vec<u8> {
    let mut out = Vec::new();
    for s in spawns {
        let key = ((s.level as u16) << 14) | ((s.x as u16) << 7) | (s.z as u16);
        write_u16(&mut out, key);
        write_u16(&mut out, s.npc_id);
    }
    out
}

#[cfg(since_647)]
pub fn parse_obj_spawn_text(text: &str) -> Vec<ObjSpawn> {
    let mut spawns = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((coords, rest)) = line.split_once(':') else {
            continue;
        };
        let mut cp = coords.split_whitespace();
        let level: u8 = cp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let x: u8 = cp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let z: u8 = cp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let mut rp = rest.split_whitespace();
        let obj_id: u16 = rp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let count: u16 = rp.next().and_then(|s| s.parse().ok()).unwrap_or(1);
        spawns.push(ObjSpawn {
            level,
            x,
            z,
            obj_id,
            count,
        });
    }
    spawns
}

#[cfg(since_647)]
pub fn encode_obj_spawn_binary(spawns: &[ObjSpawn]) -> Vec<u8> {
    let mut out = Vec::new();
    for s in spawns {
        let key = ((s.level as u16) << 12) | ((s.x as u16) << 6) | (s.z as u16);
        write_u16(&mut out, key);
        write_u16(&mut out, s.obj_id);
        write_u16(&mut out, s.count);
    }
    out
}

#[cfg(since_647)]
pub fn parse_terrain_text(text: &str) -> TerrainData {
    let mut levels = 4usize;
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("levels=") {
            levels = v.parse().unwrap_or(4);
            break;
        }
        if line.starts_with("====") || line.contains(':') {
            break;
        }
    }
    let tile_count = levels * 64 * 64;
    let mut tiles = vec![TerrainTile::default(); tile_count];
    let mut env_opcode_order: Vec<u8> = Vec::new();
    let mut environment: Option<EnvironmentData> = None;
    let mut lights: Vec<LightData> = Vec::new();
    let mut env_params: Option<EnvParams> = None;
    let mut skybox: Option<SkyboxData> = None;
    let mut camera: Option<CameraData> = None;

    let mut section = "tiles";

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("====") {
            if line.contains("OPCODE_ORDER") {
                let inner = line.trim_start_matches(|c: char| !c.is_ascii_digit());
                let inner = inner.trim_end_matches(|c: char| !c.is_ascii_digit());
                env_opcode_order = inner
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                continue;
            }
            if line.contains("ENV ====") && !line.contains("ENVPARAMS") {
                section = "env";
                environment = Some(EnvironmentData {
                    sun_color: None,
                    ambient: None,
                    sunlight: None,
                    backlight: None,
                    sun_angle: None,
                    fog_color: None,
                    fog_depth: None,
                    env_map: None,
                });
            } else if line.contains("LIGHTS") {
                section = "lights";
            } else if line.contains("ENVPARAMS") {
                section = "envparams";
            } else if line.contains("SKYBOX") {
                section = "skybox";
            } else if line.contains("CAMERA") {
                section = "camera";
                camera = Some(CameraData {
                    planes: [
                        CameraPlane::Clear,
                        CameraPlane::Clear,
                        CameraPlane::Clear,
                        CameraPlane::Clear,
                    ],
                });
            }
            continue;
        }

        match section {
            "tiles" => {
                if line.starts_with("levels=") {
                    continue;
                }
                let Some((coords, tokens)) = line.split_once(':') else {
                    continue;
                };
                let mut parts = coords.split_whitespace();
                let level: usize = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let x: usize = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let z: usize = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let index = (level * 64 + x) * 64 + z;
                if index >= tile_count {
                    continue;
                }
                let tile = &mut tiles[index];
                for token in tokens.split_whitespace() {
                    if let Some(rest) = token.strip_prefix('h') {
                        tile.height = rest.parse().ok();
                    } else if let Some(rest) = token.strip_prefix('o') {
                        let mut p = rest.split(';');
                        tile.overlay_id = p.next().and_then(|s| s.parse().ok());
                        tile.overlay_shape = p.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                        tile.overlay_rotation = p.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                    } else if let Some(rest) = token.strip_prefix('f') {
                        tile.flags = rest.parse().ok();
                    } else if let Some(rest) = token.strip_prefix('u') {
                        tile.underlay_id = rest.parse().ok();
                    }
                }
            }
            "env" => {
                if let Some(env) = &mut environment {
                    if let Some((key, val)) = line.split_once('=') {
                        match key.trim() {
                            "sun_color" => env.sun_color = val.trim().parse().ok(),
                            "ambient" => env.ambient = val.trim().parse().ok(),
                            "sunlight" => env.sunlight = val.trim().parse().ok(),
                            "backlight" => env.backlight = val.trim().parse().ok(),
                            "sun_angle" => {
                                let v: Vec<i16> = val
                                    .split(',')
                                    .filter_map(|s| s.trim().parse().ok())
                                    .collect();
                                if v.len() == 3 {
                                    env.sun_angle = Some([v[0], v[1], v[2]]);
                                }
                            }
                            "fog_color" => env.fog_color = val.trim().parse().ok(),
                            "fog_depth" => env.fog_depth = val.trim().parse().ok(),
                            "env_map" => {
                                let v: Vec<u16> = val
                                    .split(',')
                                    .filter_map(|s| s.trim().parse().ok())
                                    .collect();
                                if v.len() == 6 {
                                    env.env_map = Some([v[0], v[1], v[2], v[3], v[4], v[5]]);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            "lights" => {
                let Some((coords, rest)) = line.split_once(':') else {
                    continue;
                };
                let mut cp = coords.split_whitespace();
                let flags_raw: u8 = cp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let x: u16 = cp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let y: u16 = cp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let z: u16 = cp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let mut radius = 0u8;
                let mut colors = Vec::new();
                let mut hsl = 0u16;
                let mut type_packed = 0u8;
                let mut light_type_ref = None;
                for token in rest.split_whitespace() {
                    if let Some(v) = token.strip_prefix("radius=") {
                        radius = v.parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("colors=") {
                        colors = v.split(',').filter_map(|s| s.parse().ok()).collect();
                    } else if let Some(v) = token.strip_prefix("hsl=") {
                        hsl = v.parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("type=") {
                        type_packed = v.parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("ref=") {
                        light_type_ref = v.parse().ok();
                    }
                }
                lights.push(LightData {
                    flags_raw,
                    x,
                    y,
                    z,
                    radius,
                    colors,
                    hsl,
                    type_packed,
                    light_type_ref,
                });
            }
            "envparams" => {
                let v: Vec<u8> = line
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if v.len() == 3 {
                    env_params = Some(EnvParams {
                        values: [v[0], v[1], v[2]],
                    });
                }
            }
            "skybox" => {
                let v: Vec<&str> = line.split_whitespace().collect();
                if v.len() == 5 {
                    skybox = Some(SkyboxData {
                        param0: v[0].parse().unwrap_or(0),
                        param1: v[1].parse().unwrap_or(0),
                        param2: v[2].parse().unwrap_or(0),
                        param3: v[3].parse().unwrap_or(0),
                        param4: v[4].parse().unwrap_or(0),
                    });
                }
            }
            "camera" => {
                if let Some(cam) = &mut camera {
                    if let Some((idx, rest)) = line.split_once(':') {
                        let i: usize = idx.trim().parse().unwrap_or(0);
                        if i < 4 {
                            let rest = rest.trim();
                            if rest == "clear" {
                                cam.planes[i] = CameraPlane::Clear;
                            } else if rest == "copy" {
                                cam.planes[i] = CameraPlane::CopyPrev;
                            } else if let Some(data) = rest.strip_prefix("data ") {
                                let cells: Vec<i8> = data
                                    .split(',')
                                    .filter_map(|s| s.trim().parse().ok())
                                    .collect();
                                cam.planes[i] = CameraPlane::Data(cells);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    TerrainData {
        levels,
        tiles,
        env_opcode_order,
        environment,
        lights,
        env_params,
        skybox,
        camera,
    }
}

#[cfg(since_647)]
pub fn parse_landscape_text(text: &str) -> Vec<LocEntry> {
    let mut locs = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((coords, rest)) = line.split_once(':') else {
            continue;
        };

        let mut cparts = coords.split_whitespace();
        let level: u8 = cparts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let x: u8 = cparts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let z: u8 = cparts.next().and_then(|s| s.parse().ok()).unwrap_or(0);

        let mut rparts = rest.split_whitespace();
        let loc_id: u32 = rparts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let shape: u8 = rparts.next().and_then(|s| s.parse().ok()).unwrap_or(10);
        let rotation: u8 = rparts.next().and_then(|s| s.parse().ok()).unwrap_or(0);

        locs.push(LocEntry {
            loc_id,
            level,
            x,
            z,
            shape,
            rotation,
        });
    }

    locs
}

#[cfg(since_647)]
pub(crate) fn write_smart(out: &mut Vec<u8>, value: i32) {
    if value < 128 {
        out.push(value as u8);
    } else {
        let encoded = (value + 32768) as u16;
        out.push((encoded >> 8) as u8);
        out.push(encoded as u8);
    }
}

#[cfg(since_647)]
pub(crate) fn write_lsmart(out: &mut Vec<u8>, value: i32) {
    let mut remaining = value;
    while remaining >= 32767 {
        write_smart(out, 32767);
        remaining -= 32767;
    }
    write_smart(out, remaining);
}
