use super::provider::{CacheType, TypeProvider};
use anyhow::{Result, bail, ensure};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rustc_hash::FxHashMap;

pub type AreaTypeProvider = TypeProvider<AreaType>;

/// The area painted on multi-combat tiles.
pub const MULTIWAY: &str = "multiway";

pub const TILE_AREAS: usize = 5;

/// The areas painted on a tile, `u16::MAX` in the slots left.
pub type TileAreas = [u16; TILE_AREAS];

pub const NO_AREAS: TileAreas = [u16::MAX; TILE_AREAS];

pub struct AreaType {
    pub id: u16,
    pub trigger: bool,
    debugname: Option<Box<str>>,
}

impl CacheType for AreaType {
    type Context = ();

    fn new(id: u16) -> Self {
        AreaType {
            id,
            trigger: false,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code = buf.g1();
            if code == 0 {
                break;
            }
            match code {
                1 => {
                    buf.g3();
                }
                2 => self.trigger = true,
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("area", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}

/// Each painted map square level holds, per tile, an index into the distinct
/// sets of areas painted; 0 is none.
pub struct AreaMap {
    squares: FxHashMap<u32, Box<[u16]>>,
    sets: Vec<TileAreas>,
    set_ids: FxHashMap<TileAreas, u16>,
}

impl Default for AreaMap {
    fn default() -> Self {
        AreaMap {
            squares: FxHashMap::default(),
            sets: vec![NO_AREAS],
            set_ids: FxHashMap::from_iter([(NO_AREAS, 0)]),
        }
    }
}

impl AreaMap {
    pub fn paint(&mut self, level: u8, x: u16, z: u16, area: u16) -> Result<()> {
        ensure!(area != u16::MAX, "Invalid area id: {area}");
        let tiles = self
            .squares
            .entry(square(level, x, z))
            .or_insert_with(|| vec![0; 1 << 12].into_boxed_slice());
        let set = &mut tiles[tile(x, z)];
        let mut areas = self.sets[*set as usize];
        ensure!(!areas.contains(&area), "Area {area} painted twice");
        let Some(slot) = areas.iter_mut().find(|slot| **slot == u16::MAX) else {
            bail!("More than {TILE_AREAS} areas on one tile");
        };
        *slot = area;
        let next = self.sets.len();
        ensure!(next <= u16::MAX as usize, "Too many sets of areas");
        *set = *self.set_ids.entry(areas).or_insert_with(|| {
            self.sets.push(areas);
            next as u16
        });
        Ok(())
    }

    pub fn get(&self, level: u8, x: u16, z: u16) -> TileAreas {
        self.squares
            .get(&square(level, x, z))
            .map_or(NO_AREAS, |tiles| self.sets[tiles[tile(x, z)] as usize])
    }

    pub fn len(&self) -> usize {
        self.squares
            .values()
            .map(|tiles| tiles.iter().filter(|&&set| set != 0).count())
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.squares.is_empty()
    }
}

fn square(level: u8, x: u16, z: u16) -> u32 {
    (((level & 0x3) as u32) << 16) | (((x >> 6) as u32) << 8) | ((z >> 6) as u32)
}

fn tile(x: u16, z: u16) -> usize {
    (((x & 0x3F) as usize) << 6) | (z & 0x3F) as usize
}
