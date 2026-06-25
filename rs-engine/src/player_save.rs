use rs_entity::player::{
    ChatSettingsPrivate, ChatSettingsPublic, ChatSettingsTradeDuel, Player, StaffModLevel,
};
use rs_grid::CoordGrid;
use rs_inv::{Inventory, StackMode};
use rs_io::{Packet, crc};
use rs_pack::cache::inv::InvScope;
use rs_pack::cache::varp::VarPlayerScope;
use rs_pack::cache::{CacheStore, VarValue};
use rs_pack::types::PlayerStat;
use rs_stat::{get_exp_by_level, get_level_by_exp};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::error;

/// Magic number at the start of every binary save file for format validation.
const SAV_MAGIC: u16 = 0x2004;

/// Current save file format version. Incremented when the binary layout changes.
const SAV_VERSION: u16 = 7;

/// Total number of player stats (skills).
const STAT_COUNT: usize = 21;

// ---- PlayerProfile: shared representation for DB and binary ----

/// Represents a single inventory's contents within a player profile,
/// storing the inventory type ID and a list of (slot, obj_id, count) tuples.
pub struct PlayerProfileInv {
    pub inv_type: u16,
    pub items: Vec<(u16, u16, u32)>, // (slot, obj_id, count)
}

/// A serialisable snapshot of a player's persistent state.
///
/// Used as the intermediate representation between the live [`Player`] entity
/// and both the database and binary save file formats.
pub struct PlayerProfile {
    pub x: u16,
    pub z: u16,
    pub y: u8,
    pub body: [i32; 7],
    pub colors: [u8; 5],
    pub gender: u8,
    pub runenergy: u16,
    pub playtime: i32,
    pub stats: [i32; 21],
    pub levels: [u8; 21],
    pub varps: Vec<(u16, i32)>,
    pub invs: Vec<PlayerProfileInv>,
    pub afk_zones: [u32; 2],
    pub last_afk_zone: u16,
    pub public_chat: u8,
    pub private_chat: u8,
    pub trade_chat: u8,
    pub last_date: i64,
    pub staff_mod_level: u8,
}

/// Extracts a [`PlayerProfile`] from a live [`Player`] entity.
///
/// Only persistent-scope varps and inventories are included (temp-scope
/// data is discarded).
///
/// # Arguments
/// * `player` - The live player entity to extract from.
/// * `cache` - The game cache, used to determine varp and inventory scopes.
///
/// # Returns
/// A [`PlayerProfile`] containing all persistent player state.
pub fn extract_profile(player: &Player, cache: &CacheStore) -> PlayerProfile {
    let mut varps = Vec::new();
    for i in 0..player.vars.len() {
        let id = i as u16;
        let scope = cache
            .varps
            .get_by_id(id)
            .map(|v| v.scope)
            .unwrap_or(VarPlayerScope::Temp);
        if scope == VarPlayerScope::Perm {
            let value = player.vars.get(id).as_int();
            if value != 0 {
                varps.push((id, value));
            }
        }
    }

    let mut invs = Vec::new();
    for (&type_id, inventory) in &player.invs {
        let scope = cache
            .invs
            .get_by_id(type_id)
            .map(|v| v.scope)
            .unwrap_or(InvScope::Temp);
        if scope != InvScope::Perm {
            continue;
        }
        let mut items = Vec::new();
        for (slot, item) in inventory.slots.iter().enumerate() {
            if let Some(item) = item {
                items.push((slot as u16, item.obj, item.num));
            }
        }
        if !items.is_empty() {
            invs.push(PlayerProfileInv {
                inv_type: type_id,
                items,
            });
        }
    }

    PlayerProfile {
        x: player.pathing.coord.x(),
        z: player.pathing.coord.z(),
        y: player.pathing.coord.y(),
        body: player.body,
        colors: player.colours,
        gender: player.gender,
        runenergy: player.runenergy,
        playtime: player.playtime,
        stats: player.stats.xp,
        levels: player.stats.levels,
        varps,
        invs,
        afk_zones: player.afk_zones,
        last_afk_zone: player.last_afk_zone,
        public_chat: player.public as u8,
        private_chat: player.private as u8,
        trade_chat: player.trade as u8,
        last_date: player.last_date,
        staff_mod_level: player.staff_mod_level as u8,
    }
}

/// Applies a loaded [`PlayerProfile`] onto a live [`Player`] entity,
/// restoring all persistent state.
///
/// Sets coordinates, appearance, stats, levels, varps, inventories,
/// chat settings, and recalculates base levels and combat level.
///
/// # Arguments
/// * `profile` - The profile data to apply.
/// * `player` - The live player entity to modify.
/// * `cache` - The game cache, used for varp types and inventory sizes.
///
/// # Side Effects
/// * Overwrites player coordinates, stats, levels, appearance, varps,
///   inventories, and chat settings.
/// * Recalculates `base_levels` from stats and updates `combat_level`.
///
/// # Call Stack
/// **Calls:** [`get_level_by_exp`]
pub fn apply_profile(profile: &PlayerProfile, player: &mut Player, cache: &CacheStore) {
    player.pathing.coord = CoordGrid::new(profile.x, profile.y, profile.z);
    player.body = profile.body;
    player.colours = profile.colors;
    player.gender = profile.gender;
    player.runenergy = profile.runenergy;
    player.playtime = profile.playtime;
    player.stats.xp = profile.stats;
    player.stats.levels = profile.levels;
    for i in 0..STAT_COUNT {
        player.stats.base_levels[i] = get_level_by_exp(profile.stats[i]);
    }
    player.combat_level = player.get_combat_level();
    player.afk_zones = profile.afk_zones;
    player.last_afk_zone = profile.last_afk_zone;
    player.last_date = profile.last_date;
    player.last_login_date = profile.last_date;
    player.staff_mod_level = StaffModLevel::from_u8(profile.staff_mod_level);

    player.public = ChatSettingsPublic::from_u8(profile.public_chat);
    player.private = ChatSettingsPrivate::from_u8(profile.private_chat);
    player.trade = ChatSettingsTradeDuel::from_u8(profile.trade_chat);

    for &(id, value) in &profile.varps {
        if (id as usize) < player.vars.len()
            && let Some(varp_type) = cache.varps.get_by_id(id)
        {
            player
                .vars
                .set(id, VarValue::from_int(varp_type.var_type, value));
        }
    }

    for inv_profile in &profile.invs {
        let inv_type = cache.invs.get_by_id(inv_profile.inv_type);
        if inv_type.map(|t| t.scope).unwrap_or(InvScope::Temp) != InvScope::Perm {
            continue;
        }
        let capacity = inv_type.map(|t| t.size as usize).unwrap_or(28);
        let stack_mode = if inv_type.is_some_and(|t| t.stackall) {
            StackMode::Always
        } else {
            StackMode::Normal
        };
        let mut inv = Inventory::with_stack_mode(capacity, stack_mode);
        for &(slot, obj_id, count) in &inv_profile.items {
            if (slot as usize) < inv.capacity && cache.objs.get_by_id(obj_id).is_some() {
                inv.set(slot, obj_id, count);
            }
        }
        player.invs.insert(inv_profile.inv_type, inv);
    }
}

// ---- Binary serialization (local file fallback, TS-compatible) ----

/// Serializes a [`PlayerProfile`] into the binary `.sav` file format.
///
/// Writes the magic number, version, coordinates, appearance, stats,
/// varps, inventories, AFK zones, chat settings, last login date, and a
/// CRC32 checksum.
///
/// # Arguments
/// * `profile` - The player profile to serialise.
/// * `cache` - The game cache, used for varp count and inventory sizes.
///
/// # Returns
/// A `Vec<u8>` containing the complete binary save data.
pub fn save_binary(profile: &PlayerProfile, cache: &CacheStore) -> Vec<u8> {
    let mut sav = Packet::new(5000);

    sav.p2(SAV_MAGIC);
    sav.p2(SAV_VERSION);

    sav.p2(profile.x);
    sav.p2(profile.z);
    sav.p1(profile.y);

    for i in 0..7 {
        sav.p1(profile.body[i] as u8);
    }
    for i in 0..5 {
        sav.p1(profile.colors[i]);
    }
    sav.p1(profile.gender);

    sav.p2(profile.runenergy);
    sav.p4(profile.playtime);

    for i in 0..STAT_COUNT {
        sav.p4(profile.stats[i]);
        sav.p1(profile.levels[i]);
    }

    let varp_count = cache.varps.count() as u16;
    sav.p2(varp_count);
    for i in 0..varp_count {
        let scope = cache
            .varps
            .get_by_id(i)
            .map(|v| v.scope)
            .unwrap_or(VarPlayerScope::Temp);
        if scope == VarPlayerScope::Perm {
            let value = profile
                .varps
                .iter()
                .find(|(id, _)| *id == i)
                .map(|(_, v)| *v)
                .unwrap_or(0);
            sav.p4(value);
        } else {
            sav.p4(0);
        }
    }

    let inv_count_pos = sav.pos;
    sav.p1(0);
    let mut inv_count: u8 = 0;

    for inv_profile in &profile.invs {
        let inv_type = cache.invs.get_by_id(inv_profile.inv_type);
        let capacity = inv_type.map(|t| t.size as usize).unwrap_or(28);

        sav.p2(inv_profile.inv_type);
        sav.p2(capacity as u16);
        for slot in 0..capacity {
            if let Some(&(_, obj_id, count)) = inv_profile
                .items
                .iter()
                .find(|(s, _, _)| *s as usize == slot)
            {
                sav.p2(obj_id + 1);
                if count >= 255 {
                    sav.p1(255);
                    sav.p4(count as i32);
                } else {
                    sav.p1(count as u8);
                }
            } else {
                sav.p2(0);
            }
        }
        inv_count += 1;
    }
    sav.data[inv_count_pos] = inv_count;

    sav.p1(profile.afk_zones.len() as u8);
    for &zone in &profile.afk_zones {
        sav.p4(zone as i32);
    }
    sav.p2(profile.last_afk_zone);

    let packed_chat = (profile.public_chat << 4) | (profile.private_chat << 2) | profile.trade_chat;
    sav.p1(packed_chat);

    sav.p8(profile.last_date);
    sav.p1(profile.staff_mod_level);

    let checksum = crc::getcrc(&sav.data, 0, sav.pos);
    sav.p4(checksum);

    sav.data.truncate(sav.pos);
    sav.data
}

/// DeSerializes a [`PlayerProfile`] from raw binary `.sav` data.
///
/// Validates the magic number, version, and CRC32 checksum before parsing.
/// Supports forward-compatible reading of older save versions (v2..=v6).
///
/// # Arguments
/// * `data` - The raw binary save data.
///
/// # Returns
/// `Ok(profile)` on success, or an `Err` with a static error message if
/// the data is too short, has an invalid magic, unsupported version, or
/// incorrect checksum.
pub fn load_binary(data: &[u8]) -> Result<PlayerProfile, &'static str> {
    if data.len() < 4 {
        return Err("Save data too short");
    }

    let mut sav = Packet::from(data.to_vec());

    let magic = sav.g2();
    if magic != SAV_MAGIC {
        return Err("Invalid save magic");
    }

    let version = sav.g2();
    if version > SAV_VERSION {
        return Err("Unsupported save version");
    }

    let crc_pos = sav.data.len() - 4;
    let stored_crc = i32::from_be_bytes([
        sav.data[crc_pos],
        sav.data[crc_pos + 1],
        sav.data[crc_pos + 2],
        sav.data[crc_pos + 3],
    ]);
    let computed_crc = crc::getcrc(&sav.data, 0, crc_pos);
    if stored_crc != computed_crc {
        return Err("Incorrect save checksum");
    }

    let x = sav.g2();
    let z = sav.g2();
    let y = sav.g1();

    let mut body = [0i32; 7];
    for b in &mut body {
        let v = sav.g1() as i32;
        *b = if v == 255 { -1 } else { v };
    }
    let mut colors = [0u8; 5];
    for c in &mut colors {
        *c = sav.g1();
    }
    let gender = sav.g1();

    let runenergy = sav.g2();
    let playtime = if version >= 2 {
        sav.g4s()
    } else {
        sav.g2() as i32
    };

    let mut stats = [0i32; 21];
    let mut levels = [1u8; 21];
    for i in 0..STAT_COUNT {
        stats[i] = sav.g4s();
        levels[i] = sav.g1();
    }

    let varp_count = sav.g2() as usize;
    let mut varps = Vec::new();
    for i in 0..varp_count {
        let value = sav.g4s();
        if value != 0 {
            varps.push((i as u16, value));
        }
    }

    let inv_count = sav.g1() as usize;
    let mut invs = Vec::new();
    for _ in 0..inv_count {
        let type_id = sav.g2();
        let size = if version >= 5 {
            sav.g2() as usize
        } else {
            return Err("Save version too old for inv capacity");
        };

        let mut items = Vec::new();
        for slot in 0..size {
            let id_raw = sav.g2();
            if id_raw == 0 {
                continue;
            }
            let id = id_raw - 1;
            let count_byte = sav.g1();
            let count = if count_byte == 255 {
                sav.g4s() as u32
            } else {
                count_byte as u32
            };
            items.push((slot as u16, id, count));
        }

        if !items.is_empty() {
            invs.push(PlayerProfileInv {
                inv_type: type_id,
                items,
            });
        }
    }

    let mut afk_zones = [0u32; 2];
    let mut last_afk_zone: u16 = 0;
    if version >= 3 {
        let afk_count = sav.g1() as usize;
        for z in afk_zones.iter_mut().take(afk_count.min(2)) {
            *z = sav.g4s() as u32;
        }
        for _ in 2..afk_count {
            sav.g4s();
        }
        last_afk_zone = sav.g2();
    }

    let (public_chat, private_chat, trade_chat) = if version >= 4 {
        let packed = sav.g1();
        ((packed >> 4) & 0b11, (packed >> 2) & 0b11, packed & 0b11)
    } else {
        (0, 0, 0)
    };

    let last_date = if version >= 6 { sav.g8s() } else { 0 };
    // Saves older than v7 predate staff-level persistence; default to Normal (0).
    let staff_mod_level = if version >= 7 { sav.g1() } else { 0 };

    Ok(PlayerProfile {
        x,
        z,
        y,
        body,
        colors,
        gender,
        runenergy,
        playtime,
        stats,
        levels,
        varps,
        invs,
        afk_zones,
        last_afk_zone,
        public_chat,
        private_chat,
        trade_chat,
        last_date,
        staff_mod_level,
    })
}

// ---- New player defaults ----

/// Applies default stats and levels to a newly created player.
///
/// Sets all stats to 0 and all levels to 1, except Hitpoints which is
/// initialized to level 10 with the corresponding experience. Recalculates
/// combat level afterward.
///
/// # Arguments
/// * `player` - The new player entity to initialise.
///
/// # Side Effects
/// * Zeroes all stats and sets all levels to 1.
/// * Sets Hitpoints stat and level to 10.
/// * Recalculates `combat_level`.
///
/// # Call Stack
/// **Calls:** [`get_exp_by_level`]
pub fn apply_new_player_defaults(player: &mut Player) {
    for i in 0..STAT_COUNT {
        player.stats.xp[i] = 0;
        player.stats.levels[i] = 1;
        player.stats.base_levels[i] = 1;
    }
    player.stats.xp[PlayerStat::Hitpoints as usize] = get_exp_by_level(10);
    player.stats.levels[PlayerStat::Hitpoints as usize] = 10;
    player.stats.base_levels[PlayerStat::Hitpoints as usize] = 10;
    player.combat_level = player.get_combat_level();
}

// ---- File I/O ----

/// Builds the on-disk save path `data/players/{username}.sav` for a player.
fn save_path(username: &str) -> PathBuf {
    Path::new("data")
        .join("players")
        .join(format!("{}.sav", username))
}

/// Writes binary save data to a local file at `data/players/{username}.sav`.
///
/// Creates the `data/players/` directory if it does not exist.
///
/// # Arguments
/// * `username` - The player's username (used as the filename stem).
/// * `data` - The binary save data to write.
///
/// # Side Effects
/// * Creates or overwrites the `.sav` file on disk.
/// * Logs an error if the directory or file cannot be created.
pub fn save_to_file(username: &str, data: &[u8]) {
    let dir = Path::new("data").join("players");
    if let Err(e) = fs::create_dir_all(&dir) {
        error!("Failed to create save directory: {}", e);
        return;
    }
    let path = save_path(username);
    match fs::File::create(&path).and_then(|mut f| f.write_all(data)) {
        Ok(()) => {}
        Err(e) => error!("Failed to write save file for '{}': {}", username, e),
    }
}

/// Reads binary save data from the local file `data/players/{username}.sav`.
///
/// # Arguments
/// * `username` - The player's username (used as the filename stem).
///
/// # Returns
/// `Some(data)` if the file exists and is readable, `None` otherwise.
pub fn load_from_file(username: &str) -> Option<Vec<u8>> {
    let path = save_path(username);
    fs::read(&path).ok()
}

/// Deletes the local save file `data/players/{username}.sav` if it exists.
///
/// Silently ignores errors (e.g. file does not exist).
///
/// # Arguments
/// * `username` - The player's username (used as the filename stem).
pub fn delete_save_file(username: &str) {
    let path = save_path(username);
    let _ = fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_player_defaults_hitpoints() {
        let uid = rs_vm::PlayerUid::new("test".into(), 1);
        let vars = rs_var::VarSet::new(std::iter::empty());
        let mut player = Player::new(uid, CoordGrid::new(3222, 0, 3222), vars, false);
        apply_new_player_defaults(&mut player);
        assert_eq!(player.stats.levels[PlayerStat::Hitpoints as usize], 10);
        assert!(player.stats.xp[PlayerStat::Hitpoints as usize] > 0);
        for i in 0..STAT_COUNT {
            if i != PlayerStat::Hitpoints as usize {
                assert_eq!(player.stats.xp[i], 0);
                assert_eq!(player.stats.levels[i], 1);
            }
        }
    }
}
