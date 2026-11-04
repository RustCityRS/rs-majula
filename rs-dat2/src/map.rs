//! Rev-647 cache-format decoders for map groups (`m_X_Z` / `l_X_Z`).
//!
//! Parses tile settings and loc-placement bytes into plain Rust
//! records. **No engine state, no collision flags, no world side
//! effects** — consumers (see `rs-engine::game_map`) iterate the
//! returned records and apply collision / build indexes themselves.
//!
//! Mirrors 2004scape's decode path in `engine/cache/loader/Js5MapStore`
//! plus the rev-647-specific bridge-tile adjustment.

use rs_io::Packet;

/// Tile-settings bit: tile is impassable.
pub const TILE_BLOCKED: u8 = 0x1;
/// Tile-settings bit: level 1 is a bridge — locs on level 1 actually
/// belong to level 0 for pathing / collision purposes.
pub const TILE_BRIDGE: u8 = 0x2;

/// Decoded placement of a single loc (world object) in a map group.
#[derive(Debug, Clone, Copy)]
pub struct LocPlacement {
    pub loc_id: u16,
    pub x: u16,
    pub z: u16,
    pub level: u8,
    pub shape: u8,
    pub rotation: u8,
}

/// Minimal loc-definition info needed to apply collision for a
/// decoded placement. Populated from the LocType cache definition on
/// the engine side.
pub struct LocInfo {
    pub blockwalk: bool,
    pub blockrange: bool,
    pub width: u8,
    pub length: u8,
    /// Whether this loc is interactive (has options or is a
    /// centrepiece). Only interactive ground decor adds floor
    /// collision.
    pub active: bool,
    /// Whether this loc breaks routefinding (opcode 74). Adds extra
    /// collision flags that prevent pathfinding through the loc.
    pub breakroutefinding: bool,
}

/// Decode tile settings from an `m_X_Z` map group.
///
/// Returns a flat 4 × 64 × 64 = 16384-byte array, indexed by
/// `(level * 64 + x) * 64 + z`. Only the settings nibble (bit 0 =
/// blocked, bit 1 = bridge) is captured; heights, underlays, and
/// shape/rotation bytes are read past but discarded.
pub fn decode_tile_settings(data: &[u8]) -> [u8; 16384] {
    let mut settings = [0u8; 16384];
    let mut pos = 0usize;

    for index in 0..16384 {
        loop {
            if pos >= data.len() {
                return settings;
            }
            let config = data[pos] as u16;
            pos += 1;
            if config == 0 {
                break;
            } else if config == 1 {
                // height byte follows
                if pos < data.len() {
                    pos += 1;
                }
                break;
            } else if config <= 49 {
                // overlay (shape+rotation packed) byte follows
                if pos < data.len() {
                    pos += 1;
                }
            } else if config <= 81 {
                settings[index] = (config - 49) as u8;
            }
            // 82+ = underlay id, no extra byte — fall through to next
            // inner iteration reading more attributes for this tile.
        }
    }

    settings
}

/// Decode all loc placements from an `l_X_Z` landscape group.
///
/// Pure cache-format decoder: returns `Vec<LocPlacement>` with
/// absolute tile coords (`base_x` + local offset, same for z).
/// Consumers apply collision / build indexes on the returned list.
pub fn decode_locs(
    data: &[u8],
    settings: &[u8; 16384],
    base_x: u32,
    base_z: u32,
) -> Vec<LocPlacement> {
    let mut out = Vec::new();
    let mut pkt = Packet::from(data.to_vec());
    let mut obj_id: i32 = -1;

    loop {
        let delta = glargesmart(&mut pkt);
        if delta == 0 {
            break;
        }
        obj_id += delta;

        let mut tile_delta = 0i32;
        loop {
            let offset = gsigned_smart(&mut pkt);
            if offset == 0 {
                break;
            }
            tile_delta += offset - 1;

            let level_raw = (tile_delta >> 12) & 0x3;
            let local_x = (tile_delta >> 6) & 0x3f;
            let local_z = tile_delta & 0x3f;

            // Bridge tiles shift collision down one level.
            let mut level = level_raw;
            let bridged = if level_raw == 1 {
                let idx = ((level_raw * 64 + local_x) * 64 + local_z) as usize;
                (settings[idx] & TILE_BRIDGE) != 0
            } else {
                let bridge_idx = ((64 + local_x) * 64 + local_z) as usize;
                bridge_idx < 16384 && (settings[bridge_idx] & TILE_BRIDGE) != 0
            };
            if bridged {
                level -= 1;
            }

            if pkt.remaining() <= 0 {
                break;
            }
            let packed = pkt.g1();
            let shape = packed >> 2;
            let rotation = packed & 0x3;

            if level < 0 {
                continue;
            }

            let abs_x = base_x as i32 + local_x;
            let abs_z = base_z as i32 + local_z;

            out.push(LocPlacement {
                loc_id: obj_id as u16,
                x: abs_x as u16,
                z: abs_z as u16,
                level: level as u8,
                shape,
                rotation,
            });
        }
    }

    out
}

// ── Cache-format smart reads ────────────────────────────────────────────────
//
// These aren't quite the same as rs-io's packet `gsmart` because map
// groups use a slightly different encoding (large smart reads accumulate
// `32767` segments). Wrapping in dedicated helpers here keeps the
// decoder readable and keeps raw bit tricks off-engine.

fn gsigned_smart(pkt: &mut Packet) -> i32 {
    if pkt.remaining() <= 0 {
        return 0;
    }
    let peek = pkt.data[pkt.pos];
    if peek < 128 {
        pkt.g1() as i32
    } else {
        pkt.g2() as i32 - 32768
    }
}

fn glargesmart(pkt: &mut Packet) -> i32 {
    let mut total = 0i32;
    loop {
        let seg = gsigned_smart(pkt);
        if seg != 32767 {
            return total + seg;
        }
        total += 32767;
    }
}

#[cfg(test)]
mod tests {
    // Tile indices are written as `(level * 64 + x) * 64 + z` for
    // readability, even when `level == 1` makes the multiply an identity.
    #![allow(clippy::identity_op)]

    use super::*;

    /// Encode `value` as a gsigned_smart (1 or 2 bytes).
    fn encode_signed_smart(value: i32) -> Vec<u8> {
        if value < 128 {
            vec![value as u8]
        } else {
            let v = (value + 32768) as u16;
            vec![(v >> 8) as u8, v as u8]
        }
    }

    /// Build minimal loc-format bytes placing one loc at the given tile.
    fn encode_single_loc(loc_id: i32, level: i32, local_x: i32, local_z: i32) -> Vec<u8> {
        let mut out = Vec::new();
        // loc_id delta (glargesmart): first loc, so delta = loc_id + 1
        out.extend(encode_signed_smart(loc_id + 1));
        // tile coord delta (gsigned_smart): offset = tile_delta + 1
        let tile_delta = (level << 12) | (local_x << 6) | local_z;
        out.extend(encode_signed_smart(tile_delta + 1));
        // packed shape/rotation byte: shape=0, rotation=0
        out.push(0x00);
        // end inner loop (tile offset 0)
        out.push(0x00);
        // end outer loop (loc delta 0)
        out.push(0x00);
        out
    }

    #[test]
    fn test_bridge_shifts_level1_loc_to_level0() {
        let mut settings = [0u8; 16384];
        // Set BRIDGE flag on level=1, x=10, z=5
        settings[(1 * 64 + 10) * 64 + 5] = TILE_BRIDGE;

        let data = encode_single_loc(1, 1, 10, 5);
        let placements = decode_locs(&data, &settings, 3200, 3200);

        assert_eq!(placements.len(), 1);
        assert_eq!(
            placements[0].level, 0,
            "bridge on level 1 must shift loc to level 0"
        );
    }

    #[test]
    fn test_bridge_drops_level0_loc_under_bridge() {
        let mut settings = [0u8; 16384];
        // Bridge on level 1 at (10, 5)
        settings[(1 * 64 + 10) * 64 + 5] = TILE_BRIDGE;

        let data = encode_single_loc(1, 0, 10, 5);
        let placements = decode_locs(&data, &settings, 3200, 3200);

        assert!(
            placements.is_empty(),
            "level 0 loc under bridge shifts to -1 and is dropped"
        );
    }

    #[test]
    fn test_no_bridge_preserves_level() {
        let settings = [0u8; 16384]; // no bridge flags

        let data = encode_single_loc(1, 0, 10, 5);
        let placements = decode_locs(&data, &settings, 3200, 3200);

        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].level, 0, "no bridge → level unchanged");
    }

    #[test]
    fn test_bridge_shifts_level2_loc_to_level1() {
        let mut settings = [0u8; 16384];
        // Bridge flag on level 1 at (20, 30)
        settings[(1 * 64 + 20) * 64 + 30] = TILE_BRIDGE;

        let data = encode_single_loc(5, 2, 20, 30);
        let placements = decode_locs(&data, &settings, 3200, 3200);

        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].level, 1, "bridge shifts level 2 → level 1");
    }

    #[test]
    fn test_decode_tile_settings_bridge_flag() {
        // Minimal m_X_Z data: set tile at level=1, x=10, z=5 to BRIDGE|BLOCKED
        let mut data = Vec::new();
        let target_index = (1 * 64 + 10) * 64 + 5;

        for i in 0..16384 {
            if i == target_index {
                // settings value 3 = BLOCKED|BRIDGE → opcode 49 + 3 = 52
                data.push(52u8);
                data.push(0); // end tile
            } else {
                data.push(0); // empty tile
            }
        }

        let settings = decode_tile_settings(&data);
        assert_eq!(settings[target_index], 3);
        assert_eq!(settings[target_index] & TILE_BLOCKED, TILE_BLOCKED);
        assert_eq!(settings[target_index] & TILE_BRIDGE, TILE_BRIDGE);
        assert_eq!(settings[0], 0, "other tiles should be clear");
    }
}
