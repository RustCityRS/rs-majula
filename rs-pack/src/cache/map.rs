#[cfg(since_647)]
use crate::archive;
#[cfg(since_647)]
use anyhow::{Context, Result};
use rs_io::Packet;
#[cfg(since_647)]
use rs_io::js5::java_string_hash;
#[cfg(since_647)]
use serde_json::Value;
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
    let tile_count = levels << 12;
    let mut tiles = Vec::with_capacity(tile_count);
    let mut buf = Packet::from(data);

    for _ in 0..tile_count {
        let mut tile = TerrainTile::default();

        while buf.remaining() > 0 {
            let opcode = buf.g1();

            if opcode == 0 {
                break;
            } else if opcode == 1 {
                tile.height = Some(buf.g1());
                break;
            } else if opcode <= 49 {
                let packed = opcode - 2;
                tile.overlay_shape = packed >> 2;
                tile.overlay_rotation = packed & 3;
                tile.overlay_id = Some(buf.g1());
            } else if opcode <= 81 {
                tile.flags = Some(opcode - 49);
            } else {
                tile.underlay_id = Some(opcode - 81);
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

        while buf.remaining() > 0 {
            let opcode = buf.g1();
            env_opcode_order.push(opcode);

            match opcode {
                0 => {
                    environment = Some(parse_environment(&mut buf));
                }
                1 => {
                    let count = buf.g1();
                    for _ in 0..count {
                        lights.push(parse_light(&mut buf));
                    }
                }
                2 => {
                    env_params = Some(EnvParams {
                        values: [buf.g1(), buf.g1(), buf.g1()],
                    });
                }
                128 => {
                    skybox = Some(SkyboxData {
                        param0: buf.g2(),
                        param1: buf.g2s(),
                        param2: buf.g2s(),
                        param3: buf.g2s(),
                        param4: buf.g2(),
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
                        let ptype = buf.g1s();
                        match ptype {
                            0 => *plane = CameraPlane::Clear,
                            1 => {
                                let mut cells = Vec::with_capacity(256);
                                for _ in 0..16 {
                                    for _ in 0..16 {
                                        cells.push(buf.g1s());
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
fn parse_environment(buf: &mut Packet<&[u8]>) -> EnvironmentData {
    let flags = buf.g1();
    let sun_color = if flags & 0x01 != 0 {
        Some(buf.g4s() as u32)
    } else {
        None
    };
    let ambient = if flags & 0x02 != 0 {
        Some(buf.g2())
    } else {
        None
    };
    let sunlight = if flags & 0x04 != 0 {
        Some(buf.g2())
    } else {
        None
    };
    let backlight = if flags & 0x08 != 0 {
        Some(buf.g2())
    } else {
        None
    };
    let sun_angle = if flags & 0x10 != 0 {
        Some([buf.g2s(), buf.g2s(), buf.g2s()])
    } else {
        None
    };
    let fog_color = if flags & 0x20 != 0 {
        Some(buf.g4s() as u32)
    } else {
        None
    };
    let fog_depth = if flags & 0x40 != 0 {
        Some(buf.g2())
    } else {
        None
    };
    let env_map = if flags & 0x80 != 0 {
        Some([buf.g2(), buf.g2(), buf.g2(), buf.g2(), buf.g2(), buf.g2()])
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
fn parse_light(buf: &mut Packet<&[u8]>) -> LightData {
    let flags_raw = buf.g1();
    let x = buf.g2();
    let y = buf.g2();
    let z = buf.g2();
    let radius = buf.g1();
    let color_count = ((radius as usize) << 1) + 1;
    let mut colors = Vec::with_capacity(color_count);
    for _ in 0..color_count {
        colors.push(buf.g2());
    }
    let hsl = buf.g2();
    let type_packed = buf.g1();
    let light_type = type_packed & 0x1F;
    let light_type_ref = if light_type == 31 {
        Some(buf.g2())
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

pub fn parse_landscape_binary(data: &[u8]) -> Vec<LocEntry> {
    let mut locs = Vec::new();
    let mut buf = Packet::from(data);
    let mut loc_id: i32 = -1;

    while buf.remaining() > 0 {
        #[cfg(since_647)]
        let delta = buf.gextended1or2();
        #[cfg(before_647)]
        let delta = buf.gsmart1or2();
        if delta == 0 {
            break;
        }
        loc_id += delta;

        let mut tile_pos: i32 = 0;
        while buf.remaining() > 0 {
            let offset = buf.gsmart1or2();
            if offset == 0 {
                break;
            }
            tile_pos += offset - 1;
            let packed = buf.g1();

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
    let mut buf = Packet::from(data);
    while buf.remaining() >= 4 {
        let pos_key = buf.g2();
        let npc_id = buf.g2();
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
    let mut buf = Packet::from(data);
    while buf.remaining() >= 6 {
        let pos_key = buf.g2();
        let obj_id = buf.g2();
        let count = buf.g2();
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
    let data = fs::read_to_string(keys_path)
        .with_context(|| format!("reading {}", keys_path.display()))?;
    let entries: Vec<Value> = serde_json::from_str(&data)?;
    let mut map = HashMap::new();
    for entry in entries {
        let archive: u8 = serde_json::from_value(entry["archive"].clone())?;
        let key: [i32; 4] = serde_json::from_value(entry["key"].clone())?;
        let name_hash: i32 = serde_json::from_value(entry["name_hash"].clone())?;
        if archive == archive::MAPS {
            map.insert(name_hash, key);
        }
    }
    Ok(map)
}
