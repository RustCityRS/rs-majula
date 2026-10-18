use crate::player_save::{
    PlayerProfile, PlayerProfileInv, delete_save_file, load_binary, load_from_file, save_to_file,
};
use rs_db::{Db, DbConfig, NewPlayer, Passwords, PlayerSave};
use rs_util::base37::{to_raw_username, to_screen_name, to_userhash};
use std::path::Path;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};

/// Settings for [`db_client_task`].
pub struct DbClientConfig {
    pub db: DbConfig,
    pub pepper: String,
    pub auto_register: bool,
}

/// A request sent from the engine to the database client task.
pub enum DbRequest {
    Authenticate {
        user37: u64,
        password: Box<str>,
    },
    Save {
        user37: u64,
        username: String,
        profile: Box<PlayerProfile>,
        binary: Vec<u8>,
    },
    Load {
        user37: u64,
    },
}

/// The outcome of a [`DbRequest::Authenticate`].
pub enum AuthResult {
    Accepted { staff_mod_level: u8 },
    InvalidCredentials,
    Banned,
}

/// A response sent from the database client task back to the engine.
pub enum DbResponse {
    DbReady,
    DbDisconnected,
    AuthResponse {
        user37: u64,
        result: AuthResult,
    },
    SaveAck {
        user37: u64,
        username: String,
        success: bool,
    },
    LoadResponse {
        user37: u64,
        profile: Option<PlayerProfile>,
    },
}

/// Long-running async task that owns the database connection for player
/// operations (authentication, save, load).
///
/// Connects with exponential backoff, applies pending migrations, syncs any
/// locally saved player files to the database, signals readiness via
/// [`DbResponse::DbReady`], and then serves requests in order. Requests are
/// handled one at a time so a player's save always lands before a later load.
///
/// If a query fails, a [`DbResponse::DbDisconnected`] is sent and the task
/// reconnects.
///
/// # Call Stack
/// **Calls:** [`sync_local_saves`], [`run_requests`]
pub async fn db_client_task(
    config: DbClientConfig,
    mut request_rx: UnboundedReceiver<DbRequest>,
    response_tx: UnboundedSender<DbResponse>,
) {
    let passwords = Passwords::new(&config.pepper);
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);

    loop {
        match Db::connect(&config.db, 2).await {
            Ok(db) => {
                info!(
                    "DB connected to {}:{}/{}",
                    config.db.host, config.db.port, config.db.name
                );

                if let Err(e) = db.migrate().await {
                    error!("DB migration failed: {}", e);
                    db.close().await;
                    sleep(backoff).await;
                    backoff = (backoff * 2).min(max_backoff);
                    continue;
                }
                backoff = Duration::from_secs(1);

                sync_local_saves(&db).await;
                let _ = response_tx.send(DbResponse::DbReady);

                let result = run_requests(
                    &db,
                    &passwords,
                    config.auto_register,
                    &mut request_rx,
                    &response_tx,
                )
                .await;
                db.close().await;
                match result {
                    Ok(()) => {
                        info!("DB request channel closed -- stopping DB client task");
                        return;
                    }
                    Err(e) => {
                        warn!("DB connection lost: {}", e);
                        let _ = response_tx.send(DbResponse::DbDisconnected);
                    }
                }
            }
            Err(e) => {
                warn!("DB connect failed: {} (retry in {:?})", e, backoff);
            }
        }

        sleep(backoff).await;
        backoff = (backoff * 2).min(max_backoff);
    }
}

/// Scans the `data/players/` directory for locally saved `.sav` files and
/// uploads each one to the database.
///
/// Files that are successfully synced are deleted. Files whose player is not
/// registered are kept. This handles the case where the database was
/// unavailable when a save was attempted.
///
/// # Call Stack
/// **Called by:** [`db_client_task`]
async fn sync_local_saves(db: &Db) {
    let dir = Path::new("data").join("players");
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut synced = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(ext) = path.extension() else {
            continue;
        };
        if ext != "sav" {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let username = stem.to_string();
        let user37 = to_userhash(&username);

        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                warn!("Failed to read local save '{}': {}", username, e);
                continue;
            }
        };

        let profile = match load_binary(&data) {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to parse local save '{}': {}", username, e);
                continue;
            }
        };

        match db.store_save(user37, &to_save(&profile)).await {
            Ok(true) => {
                let _ = std::fs::remove_file(&path);
                synced += 1;
            }
            Ok(false) => {
                warn!(
                    "No registered player '{}' - keeping local save file",
                    username
                );
            }
            Err(e) => {
                warn!("Failed to sync local save '{}' to DB: {}", username, e);
            }
        }
    }

    if synced > 0 {
        info!("Synced {} local save(s) to database", synced);
    }
}

/// Serves requests until the channel closes or a query fails.
///
/// On save failure, falls back to writing a local `.sav` file.
///
/// # Returns
/// `Ok(())` when the channel closes, or the database error that caused the
/// connection to be considered lost. A request that hits an error gets no
/// response; the engine rejects pending logins on [`DbResponse::DbDisconnected`].
///
/// # Call Stack
/// **Called by:** [`db_client_task`]
/// **Calls:** [`authenticate`]
async fn run_requests(
    db: &Db,
    passwords: &Passwords,
    auto_register: bool,
    request_rx: &mut UnboundedReceiver<DbRequest>,
    response_tx: &UnboundedSender<DbResponse>,
) -> Result<(), rs_db::Error> {
    while let Some(req) = request_rx.recv().await {
        match req {
            DbRequest::Authenticate { user37, password } => {
                let result = authenticate(db, passwords, auto_register, user37, &password).await?;
                let _ = response_tx.send(DbResponse::AuthResponse { user37, result });
            }
            DbRequest::Save {
                user37,
                username,
                profile,
                binary,
            } => {
                let result = db.store_save(user37, &to_save(&profile)).await;
                let success = matches!(result, Ok(true));
                if !success {
                    if let Err(e) = &result {
                        error!("DB save failed for '{}': {}", username, e);
                    }
                    save_to_file(&username, &binary);
                }
                let _ = response_tx.send(DbResponse::SaveAck {
                    user37,
                    username,
                    success,
                });
                result?;
            }
            DbRequest::Load { user37 } => {
                let profile = db.load_save(user37).await?.map(from_save);
                let _ = response_tx.send(DbResponse::LoadResponse { user37, profile });
            }
        }
    }
    Ok(())
}

/// Authenticates a login against the `players` table.
///
/// A registered player is accepted if the password verifies and they are not
/// banned (the password is checked first so a ban is never revealed to a
/// wrong password). An unknown name is rejected, unless `auto_register` is
/// set, in which case the player is registered with this password and any
/// local save file for the name is imported.
///
/// # Call Stack
/// **Called by:** [`run_requests`]
async fn authenticate(
    db: &Db,
    passwords: &Passwords,
    auto_register: bool,
    user37: u64,
    password: &str,
) -> Result<AuthResult, rs_db::Error> {
    if let Some(creds) = db.find_credentials(user37).await? {
        if !passwords.verify(password, &creds.password_hash) {
            return Ok(AuthResult::InvalidCredentials);
        }
        if creds.banned {
            return Ok(AuthResult::Banned);
        }
        return Ok(AuthResult::Accepted {
            staff_mod_level: creds.staff_mod_level as u8,
        });
    }

    if !auto_register {
        return Ok(AuthResult::InvalidCredentials);
    }

    let Some(password_hash) = passwords.hash(password) else {
        error!("Failed to hash password for user37={}", user37);
        return Ok(AuthResult::InvalidCredentials);
    };
    // Dev builds hand auto-registered players full staff rights.
    let staff_mod_level = if cfg!(debug_assertions) { 3 } else { 0 };
    let username = to_raw_username(user37);
    let created = db
        .create_player(&NewPlayer {
            name_hash: user37,
            display_name: &to_screen_name(&username),
            password_hash: &password_hash,
            staff_mod_level,
        })
        .await?;
    if !created {
        // Registered concurrently (another world, or the website).
        return Ok(AuthResult::InvalidCredentials);
    }
    info!("Auto-registered player '{}'", username);

    if let Some(data) = load_from_file(&username)
        && let Ok(profile) = load_binary(&data)
        && db.store_save(user37, &to_save(&profile)).await?
    {
        delete_save_file(&username);
        info!("Imported local save for '{}'", username);
    }

    Ok(AuthResult::Accepted {
        staff_mod_level: staff_mod_level as u8,
    })
}

/// Flattens a [`PlayerProfile`] into its `player_saves` row. Inventory items
/// are spread across the index-aligned `inv_*` arrays, grouped by inventory.
fn to_save(profile: &PlayerProfile) -> PlayerSave {
    let mut save = PlayerSave {
        x: profile.x as i16,
        z: profile.z as i16,
        y: profile.y as i16,
        body: profile.body.iter().map(|&v| v as i16).collect(),
        colors: profile.colors.iter().map(|&v| v as i16).collect(),
        gender: profile.gender as i16,
        runenergy: profile.runenergy as i32,
        playtime: profile.playtime,
        stats: profile.stats.to_vec(),
        levels: profile.levels.iter().map(|&v| v as i16).collect(),
        afk_zones: profile.afk_zones.iter().map(|&v| v as i32).collect(),
        last_afk_zone: profile.last_afk_zone as i16,
        public_chat: profile.public_chat as i16,
        private_chat: profile.private_chat as i16,
        trade_chat: profile.trade_chat as i16,
        last_date: profile.last_date,
        varp_ids: profile.varps.iter().map(|&(id, _)| id as i32).collect(),
        varp_values: profile.varps.iter().map(|&(_, v)| v).collect(),
        ..Default::default()
    };
    for inv in &profile.invs {
        for &(slot, obj_id, count) in &inv.items {
            save.inv_types.push(inv.inv_type as i32);
            save.inv_slots.push(slot as i16);
            save.inv_objs.push(obj_id as i32);
            save.inv_counts.push(count as i32);
        }
    }
    save
}

/// Rebuilds a [`PlayerProfile`] from its `player_saves` row. Short arrays are
/// padded with defaults; the flattened `inv_*` arrays are regrouped per
/// inventory (in any order).
fn from_save(save: PlayerSave) -> PlayerProfile {
    fn fill<T: Copy, const N: usize>(default: T, values: impl Iterator<Item = T>) -> [T; N] {
        let mut out = [default; N];
        for (slot, v) in out.iter_mut().zip(values) {
            *slot = v;
        }
        out
    }

    let varps = save
        .varp_ids
        .iter()
        .zip(&save.varp_values)
        .map(|(&id, &val)| (id as u16, val))
        .collect();

    let mut invs: Vec<PlayerProfileInv> = Vec::new();
    for (i, inv_type) in save.inv_types.iter().map(|&x| x as u16).enumerate() {
        let item = (
            save.inv_slots.get(i).copied().unwrap_or(0) as u16,
            save.inv_objs.get(i).copied().unwrap_or(0) as u16,
            save.inv_counts.get(i).copied().unwrap_or(0) as u32,
        );
        if let Some(inv) = invs.iter_mut().find(|inv| inv.inv_type == inv_type) {
            inv.items.push(item);
        } else {
            invs.push(PlayerProfileInv {
                inv_type,
                items: vec![item],
            });
        }
    }

    PlayerProfile {
        x: save.x as u16,
        z: save.z as u16,
        y: save.y as u8,
        body: fill(0, save.body.iter().map(|&v| v as i32)),
        colors: fill(0, save.colors.iter().map(|&v| v as u8)),
        gender: save.gender as u8,
        runenergy: save.runenergy as u16,
        playtime: save.playtime,
        stats: fill(0, save.stats.iter().copied()),
        levels: fill(1, save.levels.iter().map(|&v| v as u8)),
        varps,
        invs,
        afk_zones: fill(0, save.afk_zones.iter().map(|&v| v as u32)),
        last_afk_zone: save.last_afk_zone as u16,
        public_chat: save.public_chat as u8,
        private_chat: save.private_chat as u8,
        trade_chat: save.trade_chat as u8,
        last_date: save.last_date,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_round_trips_through_save_row() {
        let profile = PlayerProfile {
            x: 3222,
            z: 3218,
            y: 1,
            body: [0, 10, 18, 26, 33, 36, 42],
            colors: [1, 2, 3, 4, 5],
            gender: 1,
            runenergy: 9000,
            playtime: 600,
            stats: [1154; 21],
            levels: [10; 21],
            varps: vec![(281, 1000), (1000, -1)],
            invs: vec![
                PlayerProfileInv {
                    inv_type: 93,
                    items: vec![(0, 995, u32::MAX >> 1), (27, 1351, 1)],
                },
                PlayerProfileInv {
                    inv_type: 94,
                    items: vec![(3, 1277, 1)],
                },
            ],
            afk_zones: [7, 8],
            last_afk_zone: 9,
            public_chat: 1,
            private_chat: 2,
            trade_chat: 3,
            last_date: 1_700_000_000,
        };

        let back = from_save(to_save(&profile));
        assert_eq!((back.x, back.z, back.y), (3222, 3218, 1));
        assert_eq!(back.body, profile.body);
        assert_eq!(back.colors, profile.colors);
        assert_eq!(back.stats, profile.stats);
        assert_eq!(back.levels, profile.levels);
        assert_eq!(back.varps, profile.varps);
        assert_eq!(back.invs.len(), 2);
        for (a, b) in back.invs.iter().zip(&profile.invs) {
            assert_eq!(a.inv_type, b.inv_type);
            assert_eq!(a.items, b.items);
        }
        assert_eq!(back.afk_zones, profile.afk_zones);
        assert_eq!(
            (back.public_chat, back.private_chat, back.trade_chat),
            (1, 2, 3)
        );
        assert_eq!(back.last_date, profile.last_date);
    }

    const AUTH_PEPPER: &str = "pepper";

    async fn auth(db: &Db, auto_register: bool, name: &str, password: &str) -> AuthResult {
        let passwords = Passwords::new(AUTH_PEPPER);
        authenticate(db, &passwords, auto_register, to_userhash(name), password)
            .await
            .unwrap()
    }

    /// Needs a reachable Postgres:
    /// `DATABASE_URL=postgres://postgres:password@localhost/postgres cargo test -p rs-engine client_db -- --ignored`
    #[sqlx::test(migrator = "rs_db::MIGRATOR")]
    #[ignore = "needs DATABASE_URL"]
    async fn login_policy(pool: sqlx::PgPool) {
        let db = Db::from_pool(pool.clone());

        // Unknown names are rejected unless auto-registering.
        assert!(matches!(
            auth(&db, false, "bob", "secret").await,
            AuthResult::InvalidCredentials
        ));
        assert!(
            db.find_credentials(to_userhash("bob"))
                .await
                .unwrap()
                .is_none()
        );

        // Auto-register creates the player with this password.
        let staff = if cfg!(debug_assertions) { 3 } else { 0 };
        assert!(matches!(
            auth(&db, true, "bob", "secret").await,
            AuthResult::Accepted { staff_mod_level } if staff_mod_level == staff
        ));

        // From then on the password is enforced, auto-register or not, and
        // the name matches case- and space/underscore-insensitively.
        assert!(matches!(
            auth(&db, true, "bob", "wrong").await,
            AuthResult::InvalidCredentials
        ));
        assert!(matches!(
            auth(&db, false, "BOB", "secret").await,
            AuthResult::Accepted { .. }
        ));

        sqlx::query("UPDATE players SET banned_until = now() + interval '1 day'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(matches!(
            auth(&db, false, "bob", "secret").await,
            AuthResult::Banned
        ));
        // A wrong password never learns about the ban.
        assert!(matches!(
            auth(&db, false, "bob", "wrong").await,
            AuthResult::InvalidCredentials
        ));
    }

    #[test]
    fn short_arrays_are_padded() {
        let back = from_save(PlayerSave::default());
        assert_eq!(back.levels, [1; 21]);
        assert_eq!(back.stats, [0; 21]);
        assert!(back.invs.is_empty());
    }
}
