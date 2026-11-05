use std::path::Path;
#[cfg(since_647)]
use std::path::PathBuf;

#[cfg(since_647)]
use anyhow::Result;
#[cfg(since_647)]
use tracing::info;

#[cfg(since_647)]
use crate::map_format;
#[cfg(since_647)]
use crate::pack::packed_data::PackedData;

#[cfg(before_647)]
use super::worldmap::WorldmapBuilder;
#[cfg(before_647)]
use crate::types::{MapSquareCrcs, MapSquareCsv, MapSquares};
#[cfg(before_647)]
use crate::zone_csv::load_csv_zones;
#[cfg(before_647)]
pub(super) use crate::zone_csv::zone_key;
#[cfg(before_647)]
use rs_io::Packet;
#[cfg(before_647)]
use rs_io::bz2::bz2_compress_with_size;
#[cfg(before_647)]
use rs_io::crc;
#[cfg(before_647)]
use std::collections::HashMap;
#[cfg(before_647)]
use std::sync::Arc;
#[cfg(before_647)]
use tracing::debug;

#[cfg(before_647)]
type PackedMapSquare = (
    MapSquares,
    MapSquareCrcs,
    MapSquareCsv,
    MapSquareCsv,
    WorldmapBuilder,
);

#[derive(Clone, Copy)]
#[cfg(before_647)]
pub(super) struct TileData {
    height: i32,
    pub(super) overlay: i32,
    pub(super) shape: i32,
    pub(super) rotation: i32,
    pub(super) flags: i32,
    pub(super) underlay: i32,
}

#[cfg(before_647)]
impl TileData {
    const EMPTY: Self = Self {
        height: 0,
        overlay: -1,
        shape: -1,
        rotation: -1,
        flags: -1,
        underlay: -1,
    };
}

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
fn read_map(bytes: &[u8], tiles: &mut [TileData; TILE_COUNT]) -> ParsedMap {
    tiles.fill(TileData::EMPTY);

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
                    b'h' => tile.height = fast_parse_int(&word[1..]),
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
                        tile.overlay = fast_parse_int(&rest[..semi1]);
                        if semi1 < rest.len() {
                            tile.shape = fast_parse_int(&rest[semi1 + 1..semi2.min(rest.len())]);
                        }
                        if semi2 < rest.len() {
                            tile.rotation = fast_parse_int(&rest[semi2 + 1..]);
                        }
                    }
                    b'f' => tile.flags = fast_parse_int(&word[1..]),
                    b'u' => tile.underlay = fast_parse_int(&word[1..]),
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
fn encode_terrain(tiles: &[TileData; TILE_COUNT], out: &mut Packet) {
    out.pos = 0;

    for y in 0..4 {
        for x in 0..64 {
            for z in 0..64 {
                let tile = &tiles[tile_index(y, x, z)];
                let height = tile.height;
                let overlay = tile.overlay;
                let shape = tile.shape;
                let rotation = tile.rotation;
                let flags = tile.flags;
                let underlay = tile.underlay;

                if height == 0 && overlay == -1 && flags == -1 && underlay == -1 {
                    out.p1(0);
                    continue;
                }

                if overlay != -1 {
                    let mut opcode: i32 = 2;
                    if shape != -1 {
                        opcode += shape << 2;
                    }
                    if rotation != -1 {
                        opcode += rotation;
                    }
                    out.p1(opcode as u8);
                    out.p1(overlay as u8);
                }

                if flags != -1 {
                    out.p1((flags + 49) as u8);
                }

                if underlay != -1 {
                    out.p1((underlay + 81) as u8);
                }

                if height != 0 {
                    out.p1(1);
                    out.p1(height as u8);
                } else {
                    out.p1(0);
                }
            }
        }
    }
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
    let mut tiles = [TileData::EMPTY; TILE_COUNT];
    let parsed = read_map(jm2_bytes, &mut tiles);

    let mut terrain = Packet::new(TILE_COUNT * 7);
    encode_terrain(&tiles, &mut terrain);

    let mut loc = Packet::new(64 * 1024);
    encode_locs(&parsed, &mut loc);

    (
        terrain.data[..terrain.pos].to_vec(),
        loc.data[..loc.pos].to_vec(),
    )
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

    let Ok(entries) = std::fs::read_dir(&maps_dir) else {
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

    let mut tiles = [TileData::EMPTY; TILE_COUNT];
    let mut terrain_buf = Packet::new(TILE_COUNT * 7);
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

        let Ok(file_bytes) = std::fs::read(&path) else {
            continue;
        };

        let map_data = read_map(&file_bytes, &mut tiles);
        worldmap.add_square(map_x, map_z, &tiles, &map_data);

        encode_terrain(&tiles, &mut terrain_buf);
        let terrain_compressed = bz2_compress_with_size(&terrain_buf.data[..terrain_buf.pos]);
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
pub fn pack_spawns(map_dirs: &[PathBuf], server_dir: &Path) -> Result<()> {
    let entries = collect(map_dirs, 'n', map_format::is_npc_spawns, |text| {
        let spawns = map_format::parse_npc_spawn_text(text);
        (!spawns.is_empty()).then(|| map_format::encode_npc_spawn_binary(&spawns))
    })?;
    save(&entries, server_dir, "map")?;
    info!("  {} NPC spawn regions packed", entries.len());
    Ok(())
}

/// Pack global ground-item spawns. Same on-disk container shape as the NPC
/// packer; the engine merges these as the `o` mapsquare layer and loads them
/// as `EntityLifeTime::Respawn` objs.
#[cfg(since_647)]
pub fn pack_obj_spawns(map_dirs: &[PathBuf], server_dir: &Path) -> Result<()> {
    let entries = collect(map_dirs, 'o', map_format::is_obj_spawns, |text| {
        let spawns = map_format::parse_obj_spawn_text(text);
        (!spawns.is_empty()).then(|| map_format::encode_obj_spawn_binary(&spawns))
    })?;
    save(&entries, server_dir, "objspawn")?;
    info!("  {} obj spawn regions packed", entries.len());
    Ok(())
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
    for (stem, path) in map_format::squares(map_dirs) {
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
        if let Some(binary) = encode(&std::fs::read_to_string(&path)?) {
            entries.push((mx, mz, binary));
        }
    }
    entries.sort_by_key(|(mx, mz, _)| (*mx as u16) << 8 | *mz as u16);
    Ok(entries)
}

#[cfg(since_647)]
fn save(entries: &[Entry], server_dir: &Path, stem: &str) -> Result<()> {
    let mut packed = PackedData::new(entries.len() as u32);
    for (mx, mz, binary) in entries {
        packed.start_entry();
        packed.p1(*mx);
        packed.p1(*mz);
        packed.dat.extend_from_slice(binary);
        packed.finish_entry_raw();
    }
    packed.save(server_dir, stem)
}
