use super::r#enum::EnumTypeRaw;
use super::loc::LocTypeRaw;
use super::npc::NpcTypeRaw;
use super::obj::{ObjContext, ObjTypeRaw};
use super::param::ParamTypeRaw;
use super::provider::{CacheType, TypeProvider};
use super::r#struct::StructTypeRaw;
use super::varbit::VarbitType;
use super::varp::VarPlayerTypeRaw;
use crate::types::{MapSquareCrcs, MapSquares};
use anyhow::{Result, anyhow, bail};
use rs_crypto::huffman::Huffman;
use rs_dat2::{
    CacheStore, archive,
    compression::{CompressionType, compress},
    xtea::{XteaKeyStore, java_string_hash},
};
use rs_io::Packet;
use std::{collections::HashMap, fs, path::Path, sync::Arc};
use tracing::{debug, info, warn};

pub fn load_all_npcs(cache: &mut CacheStore) -> Result<TypeProvider<NpcTypeRaw>> {
    load_strided_archive(cache, archive::CONFIG_NPC, 128, ())
}

pub fn load_all_objs(cache: &mut CacheStore, members: bool) -> Result<TypeProvider<ObjTypeRaw>> {
    let autodisable_params = if members {
        Box::new([]) as Box<[bool]>
    } else {
        load_config_group::<ParamTypeRaw>(cache, archive::CONFIG_GROUP_PARAM, ())?
            .types
            .iter()
            .map(|p| p.autodisable)
            .collect()
    };
    load_strided_archive(
        cache,
        archive::CONFIG_OBJ,
        256,
        ObjContext {
            members,
            autodisable_params,
        },
    )
}

pub fn load_all_locs(cache: &mut CacheStore) -> Result<TypeProvider<LocTypeRaw>> {
    load_strided_archive(cache, archive::CONFIG_LOC, 256, ())
}

pub fn load_all_enums(cache: &mut CacheStore) -> Result<TypeProvider<EnumTypeRaw>> {
    load_strided_archive(cache, archive::CONFIG_ENUM, 256, ())
}

pub fn load_all_varbits(cache: &mut CacheStore) -> Result<TypeProvider<VarbitType>> {
    load_strided_archive(cache, archive::CONFIG_VARBIT, 1024, ())
}

pub fn load_all_structs(cache: &mut CacheStore) -> Result<TypeProvider<StructTypeRaw>> {
    load_config_group(cache, archive::CONFIG_GROUP_STRUCT, ())
}

pub fn load_all_varps(cache: &mut CacheStore) -> Result<TypeProvider<VarPlayerTypeRaw>> {
    load_config_group(cache, archive::CONFIG_GROUP_VARP, ())
}

fn load_strided_archive<T: CacheType>(
    cache: &mut CacheStore,
    archive_id: u8,
    stride: u32,
    ctx: T::Context,
) -> Result<TypeProvider<T>> {
    let group_count = cache
        .group_count(archive_id)
        .ok_or_else(|| anyhow!("archive {} not found", archive_id))?;

    let mut types = if group_count == 0 {
        Vec::new()
    } else {
        vec![T::new(0)]
    };
    let mut loaded = 0usize;

    for group_id in 0..group_count {
        let group = match cache.read_group(archive_id, group_id) {
            Ok(Some(g)) => g,
            Ok(None) => continue,
            Err(e) => {
                warn!(
                    "Failed to read group {} from archive {}: {}",
                    group_id, archive_id, e
                );
                continue;
            }
        };

        for entry in &group.entries {
            let id = (group_id * stride + entry.id) as usize;
            if id >= types.len() {
                types.extend((types.len()..=id).map(|i| T::new(i as u16)));
            }
            let mut buf = Packet::from(entry.data.to_vec());
            types[id].decode(&mut buf);
            loaded += 1;
        }
    }

    debug!(
        "Loaded {} entries from archive {} ({} groups)",
        loaded, archive_id, group_count
    );

    Ok(TypeProvider::from_types(types, ctx))
}

fn load_config_group<T: CacheType>(
    cache: &mut CacheStore,
    group_id: u32,
    ctx: T::Context,
) -> Result<TypeProvider<T>> {
    let group = match cache.read_group(archive::CONFIG, group_id) {
        Ok(Some(g)) => g,
        Ok(None) => {
            return Ok(TypeProvider::default());
        }
        Err(e) => {
            return Err(anyhow!(
                "Failed to read group {} from archive 2 (CONFIG): {}",
                group_id,
                e
            ));
        }
    };

    let max_id = group.entries.iter().map(|e| e.id).max().unwrap_or(0) as usize;

    let count = max_id + 1;
    let mut types: Vec<T> = (0..count).map(|i| T::new(i as u16)).collect();

    for entry in &group.entries {
        let id = entry.id as usize;
        if id < count {
            let mut buf = Packet::from(entry.data.to_vec());
            types[id].decode(&mut buf);
        }
    }

    debug!(
        "Loaded {} entries from archive 2 / group {}",
        group.entries.len(),
        group_id
    );

    Ok(TypeProvider::from_types(types, ctx))
}

pub(super) fn load_mapsquares(cache: &mut CacheStore) -> Result<(MapSquares, MapSquareCrcs)> {
    let ref_table = match cache.reference_table(archive::MAPS) {
        Some(rt) => rt.clone(),
        None => {
            warn!("No reference table for archive 5 (MAPS)");
            return Ok((Default::default(), Default::default()));
        }
    };

    let xtea = XteaKeyStore::global();

    let name_lookup = build_map_name_lookup();
    let mut mapsquares = MapSquares::default();
    let mut mapcrcs = MapSquareCrcs::default();

    for group in &ref_table.groups {
        let Some(&(kind, mx, mz)) = name_lookup.get(&group.name_hash) else {
            continue;
        };

        let data = if kind == 'l' {
            // Landscape groups may be XTEA-encrypted.
            let keys = xtea
                .as_ref()
                .map(|x| x.get(mx as u32, mz as u32))
                .unwrap_or([0; 4]);
            match cache.read_group_xtea(archive::MAPS, group.id, &keys) {
                Ok(Some(d)) => d.to_vec(),
                _ => continue,
            }
        } else {
            match cache.read_group_raw_decompressed(archive::MAPS, group.id) {
                Ok(Some(d)) => d.to_vec(),
                _ => continue,
            }
        };

        // Re-compress for storage (the engine decompresses on demand).
        let compressed = compress(&data, CompressionType::Gzip).unwrap_or_default();

        mapsquares.insert((kind, mx, mz), Arc::from(compressed.as_slice()));
        mapcrcs.insert((kind, mx, mz), group.crc as i32);
    }

    info!(
        "Loaded {} mapsquare groups from JS5 cache",
        mapsquares.len()
    );
    Ok((mapsquares, mapcrcs))
}

pub(super) fn merge_server_layer(
    server_dir: &Path,
    stem: &str,
    kind: char,
    mapsquares: &mut MapSquares,
) {
    let Ok(dat) = fs::read(server_dir.join(format!("{stem}.dat"))) else {
        return;
    };
    let Ok(idx) = fs::read(server_dir.join(format!("{stem}.idx"))) else {
        return;
    };
    if dat.len() < 2 || idx.len() < 2 {
        return;
    }

    let count = u16::from_be_bytes([dat[0], dat[1]]) as usize;
    let mut dat_offset = 2;
    let mut loaded = 0usize;

    for i in 0..count {
        let idx_off = 2 + i * 2;
        if idx_off + 1 >= idx.len() {
            break;
        }
        let len = u16::from_be_bytes([idx[idx_off], idx[idx_off + 1]]) as usize;
        if len >= 2 && dat_offset + len <= dat.len() {
            let mx = dat[dat_offset];
            let mz = dat[dat_offset + 1];
            let spawn_data = &dat[dat_offset + 2..dat_offset + len];

            // Wrap in a no-compression container so the map decompressor can
            // handle it uniformly.
            let data_len = spawn_data.len() as u32;
            let mut container = Vec::with_capacity(5 + spawn_data.len());
            container.push(0); // CompressionType::None
            container.extend_from_slice(&data_len.to_be_bytes());
            container.extend_from_slice(spawn_data);

            mapsquares.insert((kind, mx, mz), Arc::from(container.as_slice()));
            loaded += 1;
        }
        dat_offset += len;
    }

    info!("Loaded {loaded} {stem} regions from server {stem}.dat");
}

pub(super) fn load_huffman(cache: &mut CacheStore) -> Result<Huffman> {
    let target_hash = java_string_hash("huffman");
    let ref_table = cache
        .reference_table(archive::BINARY)
        .ok_or_else(|| anyhow!("No reference table for archive 10 (BINARY)"))?
        .clone();

    for group in &ref_table.groups {
        if group.name_hash != target_hash {
            continue;
        }
        if let Some(g) = cache.read_group(archive::BINARY, group.id)? {
            let bits = &g.entries[0].data;
            info!("Loaded Huffman table ({} symbols) from cache", bits.len());
            return Ok(Huffman::new(bits));
        }
    }
    bail!("Huffman group not found in archive 10 (BINARY)")
}

fn build_map_name_lookup() -> HashMap<i32, (char, u8, u8)> {
    let prefixes: &[(&str, char)] = &[
        ("m", 'm'),
        ("l", 'l'),
        ("n", 'n'),
        ("um", 'u'),
        ("ul", 'U'),
        ("e", 'e'),
    ];
    let mut map = HashMap::new();
    for mx in 0..=255u8 {
        for mz in 0..=255u8 {
            for &(prefix, kind) in prefixes {
                let name = format!("{prefix}{mx}_{mz}");
                let hash = java_string_hash(&name);
                map.insert(hash, (kind, mx, mz));
            }
        }
    }
    map
}
