//! XTEA key store for map region decryption.
//!
//! Lives in rs-protocol because XTEA keys are wire-protocol data: the
//! REBUILD_NORMAL / REBUILD_REGION encoders need them and nothing in
//! the game engine layer is better-placed to own the lookup. rs-server
//! loads keys from disk at startup and installs the global via
//! [`install_global`]; encoders access it via [`XteaKeyStore::global`].

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use tracing::info;

/// XTEA keys for map region decryption, keyed by name hash of "l{mx}_{mz}".
#[derive(Clone)]
pub struct XteaKeyStore {
    keys: Arc<HashMap<i32, [i32; 4]>>,
}

static GLOBAL: OnceLock<XteaKeyStore> = OnceLock::new();

impl XteaKeyStore {
    /// Load XTEA keys from an OpenRS2-format keys.json file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        #[derive(serde::Deserialize)]
        struct Entry {
            #[allow(dead_code)]
            archive: u8,
            #[allow(dead_code)]
            group: u32,
            key: [i32; 4],
            name_hash: i32,
        }

        let data = std::fs::read_to_string(path)?;
        let entries: Vec<Entry> = serde_json::from_str(&data)?;
        let mut map = HashMap::new();
        for entry in &entries {
            map.insert(entry.name_hash, entry.key);
        }
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

/// Compute Java's String.hashCode() (same as RS2 name hash algorithm).
pub fn java_string_hash(s: &str) -> i32 {
    let mut h: i32 = 0;
    for b in s.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as i32);
    }
    h
}
