#[cfg(since_647)]
use crate::archive;
#[cfg(since_647)]
use anyhow::{Context, Result};
#[cfg(since_647)]
use rs_io::js5::java_string_hash;
#[cfg(since_647)]
use serde::Deserialize;
#[cfg(since_647)]
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{Arc, OnceLock},
};
#[cfg(since_647)]
use tracing::info;

#[derive(Debug, Clone)]
pub struct TerrainData {
    pub levels: usize,
    pub tiles: Vec<TerrainTile>,
    #[cfg(since_647)]
    pub env_opcode_order: Vec<u8>,
    #[cfg(since_647)]
    pub environment: Option<EnvironmentData>,
    #[cfg(since_647)]
    pub lights: Vec<LightData>,
    #[cfg(since_647)]
    pub env_params: Option<EnvParams>,
    #[cfg(since_647)]
    pub skybox: Option<SkyboxData>,
    #[cfg(since_647)]
    pub camera: Option<CameraData>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TerrainTile {
    pub height: Option<u8>,
    pub overlay_id: Option<u8>,
    pub overlay_shape: u8,
    pub overlay_rotation: u8,
    pub flags: Option<u8>,
    pub underlay_id: Option<u8>,
}

#[cfg(since_647)]
#[derive(Debug, Clone)]
pub struct EnvironmentData {
    pub sun_color: Option<u32>,
    pub ambient: Option<u16>,
    pub sunlight: Option<u16>,
    pub backlight: Option<u16>,
    pub sun_angle: Option<[i16; 3]>,
    pub fog_color: Option<u32>,
    pub fog_depth: Option<u16>,
    pub env_map: Option<[u16; 6]>,
}

#[cfg(since_647)]
#[derive(Debug, Clone)]
pub struct LightData {
    pub flags_raw: u8,
    pub x: u16,
    pub y: u16,
    pub z: u16,
    pub radius: u8,
    pub colors: Vec<u16>,
    pub hsl: u16,
    pub type_packed: u8,
    pub light_type_ref: Option<u16>,
}

#[cfg(since_647)]
#[derive(Debug, Clone)]
pub struct EnvParams {
    pub values: [u8; 3],
}

#[cfg(since_647)]
#[derive(Debug, Clone)]
pub struct SkyboxData {
    pub param0: u16,
    pub param1: i16,
    pub param2: i16,
    pub param3: i16,
    pub param4: u16,
}

#[cfg(since_647)]
#[derive(Debug, Clone)]
pub struct CameraData {
    pub planes: [CameraPlane; 4],
}

#[cfg(since_647)]
#[derive(Debug, Clone)]
pub enum CameraPlane {
    Clear,
    Data(Vec<i8>),
    CopyPrev,
}

#[cfg(since_647)]
#[derive(Debug, Clone)]
pub struct NpcSpawn {
    pub level: u8,
    pub x: u8,
    pub z: u8,
    pub npc_id: u16,
}

#[cfg(since_647)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjSpawn {
    pub level: u8,
    pub x: u8,
    pub z: u8,
    pub obj_id: u16,
    pub count: u16,
}

impl TerrainTile {
    pub fn has_any(&self) -> bool {
        self.height.is_some()
            || self.overlay_id.is_some()
            || self.flags.is_some()
            || self.underlay_id.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct LocEntry {
    pub loc_id: u32,
    pub level: u8,
    pub x: u8,
    pub z: u8,
    pub shape: u8,
    pub rotation: u8,
}

impl LocEntry {
    pub fn packed_pos(&self) -> i32 {
        ((self.level as i32) << 12) | ((self.x as i32) << 6) | (self.z as i32)
    }
}

pub fn parse_terrain_binary(data: &[u8]) -> TerrainData {
    parse_terrain_binary_levels(data, 4)
}

pub fn parse_terrain_binary_levels(data: &[u8], levels: usize) -> TerrainData {
    let tile_count = levels * 64 * 64;
    let mut tiles = Vec::with_capacity(tile_count);
    let mut pos = 0usize;

    for _ in 0..tile_count {
        let mut tile = TerrainTile::default();

        loop {
            if pos >= data.len() {
                break;
            }
            let opcode = data[pos] as u16;
            pos += 1;

            if opcode == 0 {
                break;
            } else if opcode == 1 {
                if pos < data.len() {
                    tile.height = Some(data[pos]);
                    pos += 1;
                }
                break;
            } else if opcode <= 49 {
                let packed = opcode - 2;
                tile.overlay_shape = (packed >> 2) as u8;
                tile.overlay_rotation = (packed & 3) as u8;
                if pos < data.len() {
                    tile.overlay_id = Some(data[pos]);
                    pos += 1;
                }
            } else if opcode <= 81 {
                tile.flags = Some((opcode - 49) as u8);
            } else {
                tile.underlay_id = Some((opcode - 81) as u8);
            }
        }

        tiles.push(tile);
    }

    #[cfg(since_647)]
    let (env_opcode_order, environment, lights, env_params, skybox, camera) = {
        let mut env_opcode_order = Vec::new();
        let mut environment = None;
        let mut lights = Vec::new();
        let mut env_params = None;
        let mut skybox = None;
        let mut camera = None;

        while pos < data.len() {
            let opcode = data[pos];
            pos += 1;
            env_opcode_order.push(opcode);

            match opcode {
                0 => {
                    environment = Some(parse_environment(data, &mut pos));
                }
                1 => {
                    let count = read_u8(data, &mut pos);
                    for _ in 0..count {
                        lights.push(parse_light(data, &mut pos));
                    }
                }
                2 => {
                    env_params = Some(EnvParams {
                        values: [
                            read_u8(data, &mut pos),
                            read_u8(data, &mut pos),
                            read_u8(data, &mut pos),
                        ],
                    });
                }
                128 => {
                    skybox = Some(SkyboxData {
                        param0: read_u16(data, &mut pos),
                        param1: read_i16(data, &mut pos),
                        param2: read_i16(data, &mut pos),
                        param3: read_i16(data, &mut pos),
                        param4: read_u16(data, &mut pos),
                    });
                }
                129 => {
                    let mut planes = [
                        CameraPlane::Clear,
                        CameraPlane::Clear,
                        CameraPlane::Clear,
                        CameraPlane::Clear,
                    ];
                    for plane in &mut planes {
                        let ptype = read_i8(data, &mut pos);
                        match ptype {
                            0 => *plane = CameraPlane::Clear,
                            1 => {
                                let mut cells = Vec::with_capacity(256);
                                for _ in 0..16 {
                                    for _ in 0..16 {
                                        cells.push(read_i8(data, &mut pos));
                                    }
                                }
                                *plane = CameraPlane::Data(cells);
                            }
                            2 => *plane = CameraPlane::CopyPrev,
                            _ => *plane = CameraPlane::Clear,
                        }
                    }
                    camera = Some(CameraData { planes });
                }
                _ => break,
            }
        }

        (
            env_opcode_order,
            environment,
            lights,
            env_params,
            skybox,
            camera,
        )
    };

    TerrainData {
        levels,
        tiles,
        #[cfg(since_647)]
        env_opcode_order,
        #[cfg(since_647)]
        environment,
        #[cfg(since_647)]
        lights,
        #[cfg(since_647)]
        env_params,
        #[cfg(since_647)]
        skybox,
        #[cfg(since_647)]
        camera,
    }
}

#[cfg(since_647)]
fn parse_environment(data: &[u8], pos: &mut usize) -> EnvironmentData {
    let flags = read_u8(data, pos);
    let sun_color = if flags & 0x01 != 0 {
        Some(read_u32(data, pos))
    } else {
        None
    };
    let ambient = if flags & 0x02 != 0 {
        Some(read_u16(data, pos))
    } else {
        None
    };
    let sunlight = if flags & 0x04 != 0 {
        Some(read_u16(data, pos))
    } else {
        None
    };
    let backlight = if flags & 0x08 != 0 {
        Some(read_u16(data, pos))
    } else {
        None
    };
    let sun_angle = if flags & 0x10 != 0 {
        Some([
            read_i16(data, pos),
            read_i16(data, pos),
            read_i16(data, pos),
        ])
    } else {
        None
    };
    let fog_color = if flags & 0x20 != 0 {
        Some(read_u32(data, pos))
    } else {
        None
    };
    let fog_depth = if flags & 0x40 != 0 {
        Some(read_u16(data, pos))
    } else {
        None
    };
    let env_map = if flags & 0x80 != 0 {
        Some([
            read_u16(data, pos),
            read_u16(data, pos),
            read_u16(data, pos),
            read_u16(data, pos),
            read_u16(data, pos),
            read_u16(data, pos),
        ])
    } else {
        None
    };
    EnvironmentData {
        sun_color,
        ambient,
        sunlight,
        backlight,
        sun_angle,
        fog_color,
        fog_depth,
        env_map,
    }
}

#[cfg(since_647)]
fn parse_light(data: &[u8], pos: &mut usize) -> LightData {
    let flags_raw = read_u8(data, pos);
    let x = read_u16(data, pos);
    let y = read_u16(data, pos);
    let z = read_u16(data, pos);
    let radius = read_u8(data, pos);
    let color_count = (radius as usize) * 2 + 1;
    let mut colors = Vec::with_capacity(color_count);
    for _ in 0..color_count {
        colors.push(read_u16(data, pos));
    }
    let hsl = read_u16(data, pos);
    let type_packed = read_u8(data, pos);
    let light_type = type_packed & 0x1F;
    let light_type_ref = if light_type == 31 {
        Some(read_u16(data, pos))
    } else {
        None
    };
    LightData {
        flags_raw,
        x,
        y,
        z,
        radius,
        colors,
        hsl,
        type_packed,
        light_type_ref,
    }
}

#[cfg(since_647)]
fn read_u8(data: &[u8], pos: &mut usize) -> u8 {
    let v = data.get(*pos).copied().unwrap_or(0);
    *pos += 1;
    v
}

#[cfg(since_647)]
fn read_i8(data: &[u8], pos: &mut usize) -> i8 {
    read_u8(data, pos) as i8
}

#[cfg(since_647)]
fn read_u16(data: &[u8], pos: &mut usize) -> u16 {
    let hi = data.get(*pos).copied().unwrap_or(0) as u16;
    let lo = data.get(*pos + 1).copied().unwrap_or(0) as u16;
    *pos += 2;
    (hi << 8) | lo
}

#[cfg(since_647)]
fn read_i16(data: &[u8], pos: &mut usize) -> i16 {
    read_u16(data, pos) as i16
}

#[cfg(since_647)]
fn read_u32(data: &[u8], pos: &mut usize) -> u32 {
    let a = data.get(*pos).copied().unwrap_or(0) as u32;
    let b = data.get(*pos + 1).copied().unwrap_or(0) as u32;
    let c = data.get(*pos + 2).copied().unwrap_or(0) as u32;
    let d = data.get(*pos + 3).copied().unwrap_or(0) as u32;
    *pos += 4;
    (a << 24) | (b << 16) | (c << 8) | d
}

pub fn parse_landscape_binary(data: &[u8]) -> Vec<LocEntry> {
    let mut locs = Vec::new();
    let mut cur = data;
    let mut loc_id: i32 = -1;

    loop {
        #[cfg(since_647)]
        let delta = glargesmart(&mut cur);
        #[cfg(before_647)]
        let delta = gsmart(&mut cur);
        if delta == 0 {
            break;
        }
        loc_id += delta;

        let mut tile_pos: i32 = 0;
        loop {
            let offset = gsmart(&mut cur);
            if offset == 0 {
                break;
            }
            tile_pos += offset - 1;

            if cur.is_empty() {
                break;
            }
            let packed = cur[0];
            cur = &cur[1..];

            locs.push(LocEntry {
                loc_id: loc_id as u32,
                level: ((tile_pos >> 12) & 0x3) as u8,
                x: ((tile_pos >> 6) & 0x3f) as u8,
                z: (tile_pos & 0x3f) as u8,
                shape: packed >> 2,
                rotation: packed & 3,
            });
        }
    }

    locs
}

#[cfg(since_647)]
pub fn parse_npc_spawn_binary(data: &[u8]) -> Vec<NpcSpawn> {
    let mut spawns = Vec::new();
    let mut pos = 0;
    while pos + 3 < data.len() {
        let pos_key = read_u16(data, &mut pos);
        let npc_id = read_u16(data, &mut pos);
        let level = (pos_key >> 14) as u8;
        let x = ((pos_key >> 7) & 0x3f) as u8;
        let z = (pos_key & 0x3f) as u8;
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
pub fn parse_obj_spawn_binary(data: &[u8]) -> Vec<ObjSpawn> {
    let mut spawns = Vec::new();
    let mut pos = 0;
    while pos + 5 < data.len() {
        let pos_key = read_u16(data, &mut pos);
        let obj_id = read_u16(data, &mut pos);
        let count = read_u16(data, &mut pos);
        spawns.push(ObjSpawn {
            level: (pos_key >> 12) as u8,
            x: ((pos_key >> 6) & 0x3f) as u8,
            z: (pos_key & 0x3f) as u8,
            obj_id,
            count,
        });
    }
    spawns
}

pub(crate) fn gsmart(cur: &mut &[u8]) -> i32 {
    if cur.is_empty() {
        return 0;
    }
    if cur[0] < 128 {
        let v = cur[0] as i32;
        *cur = &cur[1..];
        v
    } else {
        if cur.len() < 2 {
            *cur = &[];
            return 0;
        }
        let v = u16::from_be_bytes([cur[0], cur[1]]) as i32 - 32768;
        *cur = &cur[2..];
        v
    }
}

#[cfg(since_647)]
pub(crate) fn glargesmart(cur: &mut &[u8]) -> i32 {
    let mut total = 0i32;
    loop {
        let seg = gsmart(cur);
        if seg != 32767 {
            return total + seg;
        }
        total += 32767;
    }
}

/// XTEA keys for map region decryption, keyed by name hash of "l{mx}_{mz}".
#[cfg(since_647)]
#[derive(Clone)]
pub struct XteaKeyStore {
    keys: Arc<HashMap<i32, [i32; 4]>>,
}

#[cfg(since_647)]
static GLOBAL: OnceLock<XteaKeyStore> = OnceLock::new();

#[cfg(since_647)]
impl XteaKeyStore {
    /// Load XTEA keys from an OpenRS2-format keys.json file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let map = load_xtea_keys(path)?;
        info!("Loaded {} XTEA keys from {:?}", map.len(), path);
        Ok(Self {
            keys: Arc::new(map),
        })
    }

    /// Install `self` as the process-wide singleton used by encoders.
    ///
    /// First call wins; subsequent calls are ignored (matches
    /// `OnceLock::set`'s semantics). Call this once during server
    /// startup, after loading keys.
    pub fn install_global(self) {
        let _ = GLOBAL.set(self);
    }

    /// Access the global key store, if one has been installed.
    pub fn global() -> Option<&'static XteaKeyStore> {
        GLOBAL.get()
    }

    /// Look up XTEA keys for a landscape group by map square coordinates.
    /// Computes the name hash of "l{mx}_{mz}" and looks it up.
    pub fn get(&self, mx: u32, mz: u32) -> [i32; 4] {
        let name = format!("l{}_{}", mx, mz);
        let hash = java_string_hash(&name);
        self.keys.get(&hash).copied().unwrap_or([0; 4])
    }

    /// Collect XTEA keys for every mapsquare covering the 104-tile build
    /// area centered on `(player_x, player_z)`, in the order the rev-647
    /// client expects (outer loop `mx_lo..=mx_hi`, inner `mz_lo..=mz_hi`).
    pub fn snapshot_build_area(&self, player_x: u16, player_z: u16) -> Vec<[i32; 4]> {
        let chunk_x = (player_x >> 3) as u32;
        let chunk_z = (player_z >> 3) as u32;
        let build_size: u32 = 104;
        let half_chunks = build_size >> 4; // 6

        let mx_lo = (chunk_x.wrapping_sub(half_chunks)) >> 3;
        let mx_hi = (chunk_x + half_chunks) >> 3;
        let mz_lo = (chunk_z.wrapping_sub(half_chunks)) >> 3;
        let mz_hi = (chunk_z + half_chunks) >> 3;

        let mut keys = Vec::with_capacity(((mx_hi - mx_lo + 1) * (mz_hi - mz_lo + 1)) as usize);
        for mx in mx_lo..=mx_hi {
            for mz in mz_lo..=mz_hi {
                keys.push(self.get(mx, mz));
            }
        }
        keys
    }
}

#[cfg(since_647)]
pub fn load_xtea_keys(keys_path: &Path) -> Result<HashMap<i32, [i32; 4]>> {
    #[derive(Deserialize)]
    struct KeyEntry {
        archive: u8,
        key: [i32; 4],
        name_hash: i32,
    }

    let data = fs::read_to_string(keys_path)
        .with_context(|| format!("reading {}", keys_path.display()))?;
    let entries: Vec<KeyEntry> = serde_json::from_str(&data)?;
    let mut map = HashMap::new();
    for entry in entries {
        if entry.archive == archive::MAPS {
            map.insert(entry.name_hash, entry.key);
        }
    }
    Ok(map)
}
