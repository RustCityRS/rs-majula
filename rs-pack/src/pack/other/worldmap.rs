use super::map::{ParsedMap, TILE_COUNT, TileData, tile_index, zone_key};
use crate::cache::flo::FloType;
use crate::cache::loc::LocTypeRaw;
use crate::cache::npc::NpcTypeRaw;
use crate::cache::provider::{CacheType, TypeProvider};
use crate::pack::util::media::{emit_group, read_group};
use crate::pack::util::palette;
use crate::types::{LocShape, MapSquareCsv};
use rs_io::jag::{JagCompression, JagFile};
use std::collections::HashMap;
use std::path::Path;
use tracing::debug;

/// The mapscene drawn for this is a stray sprite, so locs using it are left off.
const HIDDEN_MAPSCENE: u16 = 22;

const FONTS: [&str; 8] = ["f11", "f12", "f14", "f17", "f19", "f22", "f26", "f30"];

/// A loc on the square's drawn level, in packed loc-file order: the first
/// wall on a tile wins, the last mapscene/mapfunction wins.
struct SquareLoc {
    id: u16,
    x: u8,
    z: u8,
    shape: u8,
    angle: u8,
}

/// The square's npc spawns on its drawn level: (npc id, tile).
type SquareNpcs = Vec<(u16, usize)>;

/// Collects map squares while `pack_maps` parses them; `build` adds the
/// config-dependent parts once locs and npcs are packed.
pub struct WorldmapBuilder {
    ignore: MapSquareCsv,
    squares: Vec<(u8, u8)>,
    underlay: Vec<u8>,
    overlay: Vec<u8>,
    obj: Vec<u8>,
    locs: Vec<(u8, u8, Vec<SquareLoc>)>,
    npcs: Vec<(u8, u8, SquareNpcs)>,
}

impl WorldmapBuilder {
    pub(super) fn new(ignore: MapSquareCsv) -> Self {
        Self {
            ignore,
            squares: Vec::new(),
            underlay: Vec::new(),
            overlay: Vec::new(),
            obj: Vec::new(),
            locs: Vec::new(),
            npcs: Vec::new(),
        }
    }

    pub(super) fn add_square(
        &mut self,
        mx: u8,
        mz: u8,
        tiles: &[TileData; TILE_COUNT],
        map: &ParsedMap,
    ) {
        if self
            .ignore
            .contains(&zone_key(0, (mx as u16) << 6, (mz as u16) << 6))
        {
            return;
        }
        self.squares.push((mx, mz));

        let level: u8 = if mx == 33 && (71..=73).contains(&mz) {
            1
        } else {
            0
        };
        let bridged = |x: usize, z: usize| tiles[tile_index(1, x, z)].flags.max(0) & 0x2 != 0;

        self.underlay.extend_from_slice(&[mx, mz]);
        self.overlay.extend_from_slice(&[mx, mz]);
        for x in 0..64 {
            for z in 0..64 {
                let tile = &tiles[tile_index(level as usize + bridged(x, z) as usize, x, z)];
                self.underlay.push(tile.underlay.max(0) as u8);
                if tile.overlay == -1 {
                    self.overlay.push(0);
                } else {
                    self.overlay.push(tile.overlay as u8);
                    self.overlay
                        .push(((tile.shape.max(0) << 2) + tile.rotation.max(0)) as u8);
                }
            }
        }

        let mut ids: Vec<u16> = map.locs.keys().copied().collect();
        ids.sort_unstable();
        let mut locs = Vec::new();
        for id in ids {
            for loc in &map.locs[&id] {
                let (x, z) = (loc.x as usize, loc.z as usize);
                if loc.y.checked_sub(bridged(x, z) as u8) == Some(level) {
                    locs.push(SquareLoc {
                        id,
                        x: loc.x,
                        z: loc.z,
                        shape: loc.shape,
                        angle: loc.angle,
                    });
                }
            }
        }
        self.locs.push((mx, mz, locs));

        if !map.npcs.is_empty() {
            let npcs = map
                .npcs
                .iter()
                .flat_map(|(&id, spawns)| {
                    spawns
                        .iter()
                        .filter(|npc| npc.y == level)
                        .map(move |npc| (id, (npc.x as usize) << 6 | npc.z as usize))
                })
                .collect();
            self.npcs.push((mx, mz, npcs));
        }

        if !map.objs.is_empty() {
            let mut present = [false; 64 * 64];
            for obj in map.objs.values().flatten().filter(|obj| obj.y == level) {
                present[(obj.x as usize) << 6 | obj.z as usize] = true;
            }
            self.obj.extend_from_slice(&[mx, mz]);
            self.obj.extend(present.map(u8::from));
        }
    }

    pub fn build(
        self,
        content_dir: &Path,
        flo_dat: &[u8],
        loc_dat: &[u8],
        npc_dat: &[u8],
        multimap: &MapSquareCsv,
        freemap: &MapSquareCsv,
    ) -> Vec<u8> {
        let flos = TypeProvider::<FloType>::from_bytes::<FloType>(flo_dat, ());
        let locs = TypeProvider::<LocTypeRaw>::from_bytes::<LocTypeRaw>(loc_dat, ());
        let npcs = TypeProvider::<NpcTypeRaw>::from_bytes::<NpcTypeRaw>(npc_dat, ());

        let mut loc = Vec::new();
        for (mx, mz, square) in &self.locs {
            loc.extend_from_slice(&[*mx, *mz]);
            encode_locs(square, &locs, &mut loc);
        }

        let mut npc = Vec::new();
        for (mx, mz, square) in &self.npcs {
            let mut present = [false; 64 * 64];
            for &(id, tile) in square {
                if npcs.get_by_id(id).is_some_and(|npc| npc.minimap) {
                    present[tile] = true;
                }
            }
            npc.extend_from_slice(&[*mx, *mz]);
            npc.extend(present.map(u8::from));
        }

        let multi = zone_flags(&self.squares, multimap);
        let free = zone_flags(&self.squares, freemap);

        let worldmap_dir = content_dir.join("worldmap");
        let sprite_dir = content_dir.join("sprites");
        let font_dir = content_dir.join("fonts");
        let b12 = if font_dir.join("b12_full.tga").is_file() {
            font_dir.join("b12_full.tga")
        } else {
            worldmap_dir.join("b12_full.tga")
        };

        let mut index = Vec::new();
        let mapscene = sprite(&mut index, "mapscene", &sprite_dir.join("mapscene.tga"));
        let mapfunction = sprite(
            &mut index,
            "mapfunction",
            &sprite_dir.join("mapfunction.tga"),
        );
        let b12_full = sprite(&mut index, "b12_full", &b12);
        let mapdots = sprite(&mut index, "mapdots", &sprite_dir.join("mapdots.tga"));

        let mut jag = JagFile::new();
        jag.write("underlay.dat", self.underlay);
        jag.write("overlay.dat", self.overlay);
        jag.write("loc.dat", loc);
        jag.write("obj.dat", self.obj);
        jag.write("npc.dat", npc);
        jag.write("multi.dat", multi);
        jag.write("free.dat", free);
        jag.write("floorcol.dat", floorcol(&worldmap_dir, &flos));
        jag.write("mapscene.dat", mapscene);
        jag.write("mapfunction.dat", mapfunction);
        jag.write("b12_full.dat", b12_full);
        for font in FONTS {
            let path = worldmap_dir.join(format!("{font}.fm"));
            let data = std::fs::read(&path)
                .unwrap_or_else(|e| panic!("Cannot read {}: {e}", path.display()));
            jag.write(&format!("{font}.dat"), data);
        }
        jag.write("mapdots.dat", mapdots);
        jag.write("index.dat", index);
        jag.write(
            "labels.dat",
            labels(&content_dir.join("maps").join("labels.txt")),
        );

        debug!("Packed worldmap Jag ({} map squares)", self.squares.len());
        jag.build(JagCompression::PerFile)
    }
}

/// One square's loc.dat body: per tile, its wall shape, mapscene and
/// mapfunction (each only if present), then 0.
fn encode_locs(square: &[SquareLoc], locs: &TypeProvider<LocTypeRaw>, out: &mut Vec<u8>) {
    let mut walls = [0u8; 64 * 64];
    let mut mapscenes = [None; 64 * 64];
    let mut mapfunctions = [None; 64 * 64];

    for loc in square {
        let Some(ty) = locs.get_by_id(loc.id) else {
            continue;
        };
        if ty.mapscene == Some(HIDDEN_MAPSCENE) {
            continue;
        }
        let tile = (loc.x as usize) << 6 | loc.z as usize;

        if walls[tile] == 0 {
            walls[tile] = wall(loc.shape, loc.angle, ty.active == Some(true));
        }
        if ty.mapscene.is_some() {
            mapscenes[tile] = ty.mapscene;
        }
        if ty.mapfunction.is_some() {
            mapfunctions[tile] = ty.mapfunction;
        }
    }

    for tile in 0..64 * 64 {
        if walls[tile] != 0 {
            out.push(walls[tile]);
        }
        if let Some(mapscene) = mapscenes[tile] {
            out.push((29 + mapscene) as u8);
        }
        if let Some(mapfunction) = mapfunctions[tile] {
            out.push((160 + mapfunction) as u8);
        }
        out.push(0);
    }
}

/// The viewer's wall code (1-28) for a loc shape, or 0 if it draws no wall:
/// 1-4 straight walls (west/north/east/south), 9-12 square corners, 17-20
/// wall decor, 25-26 diagonals. Active (interactive) walls are offset after
/// each group: +4, or +2 for diagonals.
fn wall(shape: u8, angle: u8, active: bool) -> u8 {
    const WALL_STRAIGHT: u8 = LocShape::WallStraight as u8;
    const WALL_L: u8 = LocShape::WallL as u8;
    const WALLDECOR: u8 = LocShape::WallDecorStraightNoOffset as u8;
    const WALL_DIAGONAL: u8 = LocShape::WallDiagonal as u8;

    let (base, active_offset) = match shape {
        WALL_STRAIGHT => (1 + angle, 4),
        WALL_L => (9 + angle, 4),
        WALLDECOR => (17 + angle, 4),
        WALL_DIAGONAL => (25 + angle % 2, 2),
        _ => return 0,
    };
    if active { base + active_offset } else { base }
}

/// The squares with a flagged zone on any level, each with its level-0 tiles.
fn zone_flags(squares: &[(u8, u8)], zones: &MapSquareCsv) -> Vec<u8> {
    let mut out = Vec::new();
    for &(mx, mz) in squares {
        let (base_x, base_z) = ((mx as u16) << 6, (mz as u16) << 6);
        let flagged =
            |level: u8, x: u16, z: u16| zones.contains(&zone_key(level, base_x + x, base_z + z));

        let any = (0..4).any(|level| {
            (0..64)
                .step_by(8)
                .any(|x| (0..64).step_by(8).any(|z| flagged(level, x, z)))
        });
        if !any {
            continue;
        }

        out.extend_from_slice(&[mx, mz]);
        for x in 0..64 {
            for z in 0..64 {
                out.push(flagged(0, x, z) as u8);
            }
        }
    }
    out
}

/// floorcol.dat: per flo, the underlay colour the viewer blends (packed
/// hue/saturation/lightness) and the overlay colour it draws (rgb).
fn floorcol(worldmap_dir: &Path, flos: &TypeProvider<FloType>) -> Vec<u8> {
    let path = worldmap_dir.join("floorcol.txt");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", path.display()));
    let table: HashMap<&str, (u32, u32)> = text
        .lines()
        .map(|line| line.split("//").next().unwrap_or("").trim())
        .filter(|line| !line.is_empty())
        .map(|line| {
            let parse = |hex: &str| {
                u32::from_str_radix(hex.trim(), 16)
                    .unwrap_or_else(|_| panic!("{}: bad colour in '{line}'", path.display()))
            };
            let (name, colours) = line
                .split_once('=')
                .unwrap_or_else(|| panic!("{}: bad line '{line}'", path.display()));
            let (underlay, overlay) = colours
                .split_once(',')
                .unwrap_or_else(|| panic!("{}: bad line '{line}'", path.display()));
            (name.trim(), (parse(underlay), parse(overlay)))
        })
        .collect();

    let mut out = Vec::with_capacity(2 + flos.count() * 8);
    out.extend_from_slice(&(flos.count() as u16).to_be_bytes());
    for flo in flos.types.iter() {
        let (underlay, overlay) = flo
            .debugname()
            .and_then(|name| table.get(name).copied())
            .unwrap_or_else(|| (hsl(flo.colour), flo.colour as u32));
        out.extend_from_slice(&underlay.to_be_bytes());
        out.extend_from_slice(&overlay.to_be_bytes());
    }
    out
}

/// An rgb colour as the viewer's packed hue/saturation/lightness: each in
/// 0..1 scaled by 256/3, hue at bit 20 (signed), saturation at 10. Matches
/// Jagex's table for all but a few hand-tuned flos.
fn hsl(rgb: i32) -> u32 {
    let r = ((rgb >> 16) & 0xff) as f64 / 256.0;
    let g = ((rgb >> 8) & 0xff) as f64 / 256.0;
    let b = (rgb & 0xff) as f64 / 256.0;

    let min = r.min(g).min(b);
    let max = r.max(g).max(b);
    let lightness = (min + max) / 2.0;
    let mut hue = 0.0;
    let mut saturation = 0.0;
    if min != max {
        saturation = if lightness < 0.5 {
            (max - min) / (max + min)
        } else {
            (max - min) / (2.0 - max - min)
        };
        hue = if r == max {
            (g - b) / (max - min)
        } else if g == max {
            2.0 + (b - r) / (max - min)
        } else {
            4.0 + (r - g) / (max - min)
        };
    }
    hue /= 6.0;

    let scale = |v: f64| (v * 256.0 / 3.0) as i32;
    (scale(hue) << 20)
        .wrapping_add(scale(saturation) << 10)
        .wrapping_add(scale(lightness)) as u32
}

fn sprite(index: &mut Vec<u8>, name: &str, path: &Path) -> Vec<u8> {
    let mut group = read_group(path);
    palette::derive_group(name, &mut group);
    emit_group(index, &group)
}

/// labels.dat from `=text,x,z,font` lines.
fn labels(path: &Path) -> Vec<u8> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", path.display()));
    let lines: Vec<&str> = text
        .lines()
        .filter_map(|line| line.strip_prefix('='))
        .collect();

    let mut out = Vec::new();
    out.extend_from_slice(&(lines.len() as u16).to_be_bytes());
    for line in lines {
        let mut parts = line.rsplitn(4, ',');
        let mut number = || {
            parts
                .next()
                .and_then(|n| n.trim().parse::<u16>().ok())
                .unwrap_or_else(|| panic!("{}: bad label '{line}'", path.display()))
        };
        let font = number() as u8;
        let z = number();
        let x = number();
        let name = parts.next().unwrap_or_default();

        out.extend_from_slice(name.as_bytes());
        out.push(b'\n');
        out.extend_from_slice(&x.to_be_bytes());
        out.extend_from_slice(&z.to_be_bytes());
        out.push(font);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::hsl;

    #[test]
    fn test_hsl_matches_jagex_floorcol() {
        assert_eq!(hsl(0x35720a), 0x01611c14); // grassland
        assert_eq!(hsl(0xaaaaaa), 0x00000038); // cliff
        assert_eq!(hsl(0x995566), 0xffd06027); // purple_pink: negative hue
        assert_eq!(hsl(0xff00ff), 0xff21542a); // invisible
        assert_eq!(hsl(0x000000), 0); // textured flos
    }
}
