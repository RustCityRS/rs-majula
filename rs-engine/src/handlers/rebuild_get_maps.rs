use crate::active_player::ActivePlayer;
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use num_enum::TryFromPrimitive;
use rs_protocol::network::game::client::rebuild_get_maps::RebuildGetMaps;
use rs_vm::ScriptError;

/// Maximum data payload size per chunk packet, accounting for packet header overhead.
const CHUNK_SIZE: usize = 1000 - 1 - 2 - 1 - 1 - 2 - 2;

/// Maximum number of map entries allowed in a single request.
/// Equals 9 mapsquares times 2 (one land + one loc per mapsquare).
const MAPSQUARES_LIMIT: usize = 9 * 2; // 9 mapsquares * 2 (m & l)

/// Discriminates between the two kinds of map data (land heightmap vs. location data).
#[repr(u8)]
#[derive(TryFromPrimitive)]
enum MapKind {
    /// Land / heightmap data (`m` prefix files).
    Land = 0,
    /// Location / scenery data (`l` prefix files).
    Loc = 1,
}

impl MapKind {
    fn prefix(&self) -> char {
        match self {
            MapKind::Land => 'm',
            MapKind::Loc => 'l',
        }
    }
}

/// Sends map data to the client in chunks of [`CHUNK_SIZE`] bytes, followed by
/// a completion marker.
///
/// Large map data files are split into multiple packets to stay within the
/// per-packet size limit. Each chunk includes the mapsquare coordinates, offset,
/// total length, and the data slice. After all chunks are sent, a "done" packet
/// is sent to signal the client to finalize the mapsquare.
///
/// # Arguments
///
/// * `data` - The raw map data bytes to send.
/// * `x` - The mapsquare X coordinate.
/// * `z` - The mapsquare Z coordinate.
/// * `active` - The active player to send the data to.
/// * `kind` - Whether this is land or location data.
///
/// # Side Effects
///
/// * Sends one or more `data_land`/`data_loc` packets followed by a
///   `data_land_done`/`data_loc_done` packet to the player's client.
fn send_chunked(data: &[u8], x: u8, z: u8, active: &mut ActivePlayer, kind: MapKind) {
    let len = data.len();
    for off in (0..len).step_by(CHUNK_SIZE) {
        let end = (off + CHUNK_SIZE).min(len);
        match kind {
            MapKind::Land => active.data_land(x, z, off as u16, len as u16, &data[off..end]),
            MapKind::Loc => active.data_loc(x, z, off as u16, len as u16, &data[off..end]),
        }
    }
    match kind {
        MapKind::Land => active.data_land_done(x, z),
        MapKind::Loc => active.data_loc_done(x, z),
    }
}

/// Handles the `RebuildGetMaps` client protocol message.
///
/// Processes the client's request for map data after a region rebuild. The client
/// sends a list of packed mapsquare identifiers (each encoding a mapsquare ID and
/// a land/loc kind flag). For each requested mapsquare that exists in the player's
/// build area, the corresponding map data is loaded from the cache and sent to the
/// client in chunks via [`send_chunked`].
///
/// After all requested data has been sent, the player's build area zones are
/// rebuilt based on the current player coordinate.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success.
/// * `Err(ScriptError::Client)` if too many maps were requested (exceeds
///   [`MAPSQUARES_LIMIT`]).
///
/// # Side Effects
///
/// * Sends map data packets to the client via [`send_chunked`].
/// * Rebuilds the player's build area zones.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`send_chunked`], `build_area.rebuild_zones`
impl ClientGameHandler for RebuildGetMaps {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if self.maps.len() > MAPSQUARES_LIMIT {
            return Err(ScriptError::Client(format!(
                "Too many maps were requested: {}",
                self.maps.len()
            )));
        }

        let cache = engine().cache;

        for packed in self.maps {
            let mapsquare = (packed & 0xFFFF) as u16;

            if !active.player.build_area.mapsquares.contains(&mapsquare) {
                continue;
            }

            let x = ((mapsquare >> 8) & 0xFF) as u8;
            let z = (mapsquare & 0xFF) as u8;
            let kind = ((packed >> 16) & 0x1) as u8;

            let Ok(kind) = MapKind::try_from(kind) else {
                continue;
            };

            let data = cache.mapsquares.get(&(kind.prefix(), x, z));

            if let Some(data) = data {
                send_chunked(data, x, z, active, kind);
            }
        }

        let coord = active.player.pathing.coord;
        active.player.build_area.rebuild_zones(coord);

        Ok(())
    }
}
