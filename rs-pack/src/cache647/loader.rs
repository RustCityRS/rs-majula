//! Cache-backed config loaders — read all configs from a JS5 cache into TypeProvider.
//!
//! Each `load_all_*` function reads the appropriate archive/groups from the
//! JS5 cache, decodes entries via CacheType, and returns a `TypeProvider`.
//!
//! Archive layout (rev 647):
//!   2  = CONFIG (group-based: inv, param, varp, struct, idk, flu, flo, …)
//!   16 = CONFIG_LOC
//!   17 = CONFIG_ENUM
//!   18 = CONFIG_NPC
//!   19 = CONFIG_OBJ
//!   20 = CONFIG_SEQ
//!   21 = CONFIG_SPOTANIM
//!   22 = CONFIG_VARBIT

use rs_dat2::CacheStore;
use rs_dat2::archive;
use rs_io::Packet;
use tracing::{debug, warn};

use super::provider::{CacheType, TypeProvider};
use super::{
    EnumType, FloType, FluType, IdkType, LocType, NpcType, ObjContext, ObjType, ParamType, SeqType,
    StructType, VarBitType, VarpType,
};

/// Load all NPC types from archive 18 (CONFIG_NPC, stride 128).
pub fn load_all_npcs(cache: &mut CacheStore) -> anyhow::Result<TypeProvider<NpcType>> {
    load_strided_archive(cache, archive::CONFIG_NPC, 128, ())
}

/// Load all item/obj types from archive 19 (CONFIG_OBJ, stride 256).
pub fn load_all_objs(
    cache: &mut CacheStore,
    members: bool,
) -> anyhow::Result<TypeProvider<ObjType>> {
    load_strided_archive(cache, archive::CONFIG_OBJ, 256, ObjContext { members })
}

/// Load all loc types from archive 16 (CONFIG_LOC, stride 256).
pub fn load_all_locs(cache: &mut CacheStore) -> anyhow::Result<TypeProvider<LocType>> {
    load_strided_archive(cache, archive::CONFIG_LOC, 256, ())
}

/// Load all enum types from archive 17 (CONFIG_ENUM, stride 256).
pub fn load_all_enums(cache: &mut CacheStore) -> anyhow::Result<TypeProvider<EnumType>> {
    load_strided_archive(cache, archive::CONFIG_ENUM, 256, ())
}

/// Load all seq (animation) types from archive 20 (CONFIG_SEQ, stride 128).
pub fn load_all_seqs(cache: &mut CacheStore) -> anyhow::Result<TypeProvider<SeqType>> {
    load_strided_archive(cache, archive::CONFIG_SEQ, 128, ())
}

/// Load all varbit types from archive 22 (CONFIG_VARBIT, stride 1024).
pub fn load_all_varbits(cache: &mut CacheStore) -> anyhow::Result<TypeProvider<VarBitType>> {
    load_strided_archive(cache, archive::CONFIG_VARBIT, 1024, ())
}

/// Load all param types from archive 2, group 11.
pub fn load_all_params(cache: &mut CacheStore) -> anyhow::Result<TypeProvider<ParamType>> {
    load_config_group(cache, archive::CONFIG_GROUP_PARAM, ())
}

/// Load all struct types from archive 2, group 26.
pub fn load_all_structs(cache: &mut CacheStore) -> anyhow::Result<TypeProvider<StructType>> {
    load_config_group(cache, archive::CONFIG_GROUP_STRUCT, ())
}

/// Load all identity kit types from archive 2, group 3.
pub fn load_all_idks(cache: &mut CacheStore) -> anyhow::Result<TypeProvider<IdkType>> {
    load_config_group(cache, archive::CONFIG_GROUP_IDK, ())
}

/// Load all varp types from archive 2, group 16.
pub fn load_all_varps(cache: &mut CacheStore) -> anyhow::Result<TypeProvider<VarpType>> {
    load_config_group(cache, archive::CONFIG_GROUP_VARP, ())
}

/// Load all floor underlay types from archive 2, group 1.
pub fn load_all_flus(cache: &mut CacheStore) -> anyhow::Result<TypeProvider<FluType>> {
    load_config_group(cache, archive::CONFIG_GROUP_FLU, ())
}

/// Load all floor overlay types from archive 2, group 4.
pub fn load_all_flos(cache: &mut CacheStore) -> anyhow::Result<TypeProvider<FloType>> {
    load_config_group(cache, archive::CONFIG_GROUP_FLO, ())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Load a strided archive (16–22) into a TypeProvider.
///
/// Each archive contains groups of entries where `id = group_id * stride + file_id`.
/// We read all groups, compute the max ID, then place decoded entries into
/// a contiguous Vec indexed by ID.
fn load_strided_archive<T: CacheType>(
    cache: &mut CacheStore,
    archive_id: u8,
    stride: u32,
    ctx: T::Context,
) -> anyhow::Result<TypeProvider<T>> {
    let group_count = cache
        .group_count(archive_id)
        .ok_or_else(|| anyhow::anyhow!("archive {} not found", archive_id))?;

    // First pass: determine max ID so we can size the Vec.
    let mut max_id: u32 = 0;
    for group_id in 0..group_count {
        let group = match cache.read_group(archive_id, group_id) {
            Ok(Some(g)) => g,
            Ok(None) | Err(_) => continue,
        };
        for entry in &group.entries {
            let id = group_id * stride + entry.id;
            if id > max_id {
                max_id = id;
            }
        }
    }

    if max_id == 0 && group_count == 0 {
        return Ok(TypeProvider {
            names: Default::default(),
            types: Vec::new().into_boxed_slice(),
        });
    }

    let count = max_id as usize + 1;
    let mut types: Vec<T> = (0..count).map(|i| T::new(i as u16)).collect();
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
            if id < count {
                let mut buf = Packet::from(entry.data.to_vec());
                types[id].decode(&mut buf);
                loaded += 1;
            }
        }
    }

    T::post_decode(&mut types, &ctx);

    // Build name map from debugnames.
    let mut names = std::collections::HashMap::new();
    for (i, t) in types.iter().enumerate() {
        if let Some(name) = t.debugname() {
            names.insert(name.into(), i as u16);
        }
    }

    debug!(
        "Loaded {} entries from archive {} ({} groups)",
        loaded, archive_id, group_count
    );

    Ok(TypeProvider {
        names,
        types: types.into_boxed_slice(),
    })
}

/// Load a single config group from archive 2 (CONFIG) into a TypeProvider.
///
/// Each file in the group represents one config entry; file_id = type_id.
fn load_config_group<T: CacheType>(
    cache: &mut CacheStore,
    group_id: u32,
    ctx: T::Context,
) -> anyhow::Result<TypeProvider<T>> {
    let group = match cache.read_group(archive::CONFIG, group_id) {
        Ok(Some(g)) => g,
        Ok(None) => {
            return Ok(TypeProvider {
                names: Default::default(),
                types: Vec::new().into_boxed_slice(),
            });
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
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

    T::post_decode(&mut types, &ctx);

    let mut names = std::collections::HashMap::new();
    for (i, t) in types.iter().enumerate() {
        if let Some(name) = t.debugname() {
            names.insert(name.into(), i as u16);
        }
    }

    debug!(
        "Loaded {} entries from archive 2 / group {}",
        group.entries.len(),
        group_id
    );

    Ok(TypeProvider {
        names,
        types: types.into_boxed_slice(),
    })
}
