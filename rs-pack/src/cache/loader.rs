use super::r#enum::EnumTypeRaw;
use super::loc::LocTypeRaw;
use super::npc::NpcTypeRaw;
use super::obj::{ObjContext, ObjTypeRaw};
use super::param::ParamTypeRaw;
use super::provider::{CacheType, TypeProvider};
use super::r#struct::StructTypeRaw;
use super::varbit::VarbitType;
use super::varp::VarPlayerTypeRaw;
use crate::archive;
use crate::cache::js5::PackedCache;
use crate::cache::map::XteaKeyStore;
use crate::pack::incremental::{self, Hash};
use crate::pack::packed_data::PackedData;
use crate::types::{MapSquareCrcs, MapSquares};
use anyhow::{Context, Result, anyhow, bail};
use rayon::prelude::*;
use rs_crypto::huffman::Huffman;
use rs_io::Packet;
use rs_io::js5::{
    Js5Store,
    compression::{CompressionType, compress},
    java_string_hash,
};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, info, warn};

pub fn load_all_npcs(cache: &mut Js5Store) -> Result<TypeProvider<NpcTypeRaw>> {
    load_strided_archive(cache, archive::CONFIG_NPC, 128, ())
}

pub fn load_all_objs(cache: &mut Js5Store, members: bool) -> Result<TypeProvider<ObjTypeRaw>> {
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

pub fn load_all_locs(cache: &mut Js5Store) -> Result<TypeProvider<LocTypeRaw>> {
    load_strided_archive(cache, archive::CONFIG_LOC, 256, ())
}

pub fn load_all_enums(cache: &mut Js5Store) -> Result<TypeProvider<EnumTypeRaw>> {
    load_strided_archive(cache, archive::CONFIG_ENUM, 256, ())
}

pub fn load_all_varbits(cache: &mut Js5Store) -> Result<TypeProvider<VarbitType>> {
    load_strided_archive(cache, archive::CONFIG_VARBIT, 1024, ())
}

pub fn load_all_structs(cache: &mut Js5Store) -> Result<TypeProvider<StructTypeRaw>> {
    load_config_group(cache, archive::CONFIG_GROUP_STRUCT, ())
}

pub fn load_all_varps(cache: &mut Js5Store) -> Result<TypeProvider<VarPlayerTypeRaw>> {
    load_config_group(cache, archive::CONFIG_GROUP_VARP, ())
}

fn load_strided_archive<T: CacheType>(
    cache: &mut Js5Store,
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
            let mut buf = Packet::from(entry.data().to_vec());
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
    cache: &mut Js5Store,
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
            let mut buf = Packet::from(entry.data().to_vec());
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

pub(crate) fn load_mapsquares(
    cache: &PackedCache,
    xtea: &XteaKeyStore,
) -> Result<(MapSquares, MapSquareCrcs)> {
    let ref_table = match cache.reference_table(archive::MAPS) {
        Some(rt) => rt,
        None => {
            warn!("No reference table for archive 5 (MAPS)");
            return Ok((Default::default(), Default::default()));
        }
    };

    let name_lookup = build_map_name_lookup();
    let mut mapsquares = MapSquares::default();
    let mut mapcrcs = MapSquareCrcs::default();

    let squares: Vec<_> = ref_table
        .groups
        .par_iter()
        .filter_map(|group| {
            let &(kind, mx, mz) = name_lookup.get(&group.name_hash)?;
            let keys = xtea.get(mx as u32, mz as u32);
            let mut inputs = Hash::new("mapsquare");
            inputs.bytes(cache.groups.get(&(archive::MAPS, group.id))?);
            if kind == 'l' {
                for key in keys {
                    inputs.int(u64::from(key as u32));
                }
            }
            let compressed = incremental::memo(inputs.key(), || {
                let data = if kind == 'l' {
                    cache.read_group_xtea(archive::MAPS, group.id, &keys)
                } else {
                    cache.read_group_raw_decompressed(archive::MAPS, group.id)
                }?
                .context("Missing map group")?;
                Ok(compress(&data, CompressionType::Gzip).unwrap_or_default())
            })
            .ok()?;
            Some(((kind, mx, mz), Arc::from(compressed), group.crc as i32))
        })
        .collect();
    for (square, data, crc) in squares {
        mapsquares.insert(square, data);
        mapcrcs.insert(square, crc);
    }

    info!(
        "Loaded {} mapsquare groups from JS5 cache",
        mapsquares.len()
    );
    Ok((mapsquares, mapcrcs))
}

pub(crate) fn merge_server_layer(
    packed: &PackedData,
    stem: &str,
    kind: char,
    mapsquares: &mut MapSquares,
) {
    let entries = match PackedData::entries(&packed.dat, &packed.idx) {
        Ok(entries) => entries,
        Err(error) => {
            warn!("Invalid server {stem} layer: {error}");
            return;
        }
    };
    let mut loaded = 0;
    for (_, data) in entries.filter(|(_, data)| data.len() >= 2) {
        let container = compress(&data[2..], CompressionType::None).unwrap();
        mapsquares.insert((kind, data[0], data[1]), Arc::from(container));
        loaded += 1;
    }

    info!("Loaded {loaded} {stem} regions from server {stem}.dat");
}

pub(crate) fn load_huffman(cache: &PackedCache) -> Result<Huffman> {
    let target_hash = java_string_hash("huffman");
    let ref_table = cache
        .reference_table(archive::BINARY)
        .ok_or_else(|| anyhow!("No reference table for archive 10 (BINARY)"))?;

    for group in &ref_table.groups {
        if group.name_hash != target_hash {
            continue;
        }
        if let Some(bits) = cache.read_group_raw_decompressed(archive::BINARY, group.id)? {
            info!("Loaded Huffman table ({} symbols) from cache", bits.len());
            return Ok(Huffman::new(&bits));
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
