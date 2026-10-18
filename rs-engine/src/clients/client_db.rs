use crate::player_save::{
    PlayerProfile, PlayerProfileInv, delete_save_file, load_binary, load_from_file, save_to_file,
};
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use ddl::Type;
use rs_util::base37::{to_raw_username, to_userhash};
use std::path::Path;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::{Duration, sleep};
use tokio_postgres::types::ToSql;
use tokio_postgres::{Client, Error, NoTls};
use tracing::{error, info, warn};

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

/// A response sent from the database client task back to the engine.
pub enum DbResponse {
    DbReady,
    DbDisconnected,
    AuthResponse {
        user37: u64,
        success: bool,
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

/// Long-running async task that manages the PostgreSQL database connection
/// for player profile operations (authentication, save, load).
///
/// Connects to the database with exponential backoff on failure. Once
/// connected, ensures the required tables exist, syncs any locally saved
/// player files to the database, signals readiness via [`DbResponse::DbReady`],
/// and then enters the main request loop.
///
/// If the connection is lost, a [`DbResponse::DbDisconnected`] is sent and
/// the task retries the connection.
///
/// # Arguments
/// * `host` - The database hostname.
/// * `port` - The database port.
/// * `name` - The database name.
/// * `user` - The database username.
/// * `pass` - The database password.
/// * `pepper` - A secret pepper string prepended to passwords before hashing.
/// * `request_rx` - Channel receiver for incoming [`DbRequest`] messages.
/// * `response_tx` - Channel sender for outgoing [`DbResponse`] messages.
///
/// # Side Effects
/// * Creates database tables if they do not exist.
/// * Syncs local `.sav` files to the database on connect.
///
/// # Call Stack
/// **Calls:** [`ensure_tables`], [`sync_local_saves`], [`run_requests`]
#[allow(clippy::too_many_arguments)]
pub async fn db_client_task(
    host: String,
    port: u16,
    name: String,
    user: String,
    pass: String,
    pepper: String,
    mut request_rx: UnboundedReceiver<DbRequest>,
    response_tx: UnboundedSender<DbResponse>,
) {
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);

    loop {
        let config = format!(
            "host={} port={} dbname={} user={} password={}",
            host, port, name, user, pass
        );

        match tokio_postgres::connect(&config, NoTls).await {
            Ok((mut client, connection)) => {
                info!("DB connected to {}:{}/{}", host, port, name);
                backoff = Duration::from_secs(1);

                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        error!("DB connection error: {}", e);
                    }
                });

                if let Err(e) = ensure_tables(&client).await {
                    error!("DB failed to create tables: {}", e);
                    sleep(backoff).await;
                    backoff = (backoff * 2).min(max_backoff);
                    continue;
                }

                sync_local_saves(&mut client).await;
                let _ = response_tx.send(DbResponse::DbReady);

                match run_requests(&mut client, &pepper, &mut request_rx, &response_tx).await {
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
/// Files that are successfully synced are deleted. Files whose user has no
/// existing database row are kept until the player authenticates. This handles
/// the case where the database was unavailable when a save was attempted.
///
/// # Arguments
/// * `client` - An active database connection.
///
/// # Side Effects
/// * Uploads player profiles to the database.
/// * Deletes local `.sav` files after successful sync.
///
/// # Call Stack
/// **Called by:** [`db_client_task`]
/// **Calls:** [`load_binary`](load_binary),
/// [`save_profile`]
async fn sync_local_saves(client: &mut Client) {
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

        match save_profile(client, user37, &profile).await {
            Ok(true) => {
                let _ = std::fs::remove_file(&path);
                synced += 1;
            }
            Ok(false) => {
                warn!(
                    "No DB row for '{}' - keeping local file until player authenticates",
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

/// Ensures the `player_saves` table exists with the current schema.
///
/// Varps and inventories are stored inline on the player's single row as
/// index-aligned Postgres arrays (`varp_ids`/`varp_values` and
/// `inv_types`/`inv_slots`/`inv_objs`/`inv_counts`) rather than in separate
/// per-item tables.
///
/// The array columns are added to a pre-existing `player_saves` via
/// `ADD COLUMN IF NOT EXISTS`, and the legacy `player_varps` /
/// `player_inventories` tables are dropped if present (their contents are
/// discarded, not migrated). The whole statement is idempotent and safe to run
/// on every startup.
///
/// The schema is declared as structured data in [`PLAYER_SAVES`] and rendered
/// to SQL by the [`ddl`] builder rather than written as a raw SQL string.
///
/// # Arguments
/// * `client` - An active database connection.
///
/// # Returns
/// `Ok(())` on success, or a database error.
///
/// # Call Stack
/// **Called by:** [`db_client_task`]
async fn ensure_tables(client: &Client) -> Result<(), Error> {
    let mut sql = String::new();
    sql.push_str(&ddl::create_table("player_saves", PLAYER_SAVES));
    sql.push_str(&ddl::add_columns("player_saves", PLAYER_SAVES));
    let staff_default = if cfg!(debug_assertions) { "3" } else { "0" };
    let staff = ddl::Column::new(col::STAFF_MOD_LEVEL, Type::SmallInt)
        .not_null()
        .default(staff_default);
    sql.push_str(&ddl::add_columns("player_saves", &[staff]));

    // Legacy per-item tables, replaced by the inline arrays above.
    sql.push_str(&ddl::drop_table("player_varps"));
    sql.push_str(&ddl::drop_table("player_inventories"));

    client.batch_execute(&sql).await
}

/// Canonical `player_saves` column names -- the single source of truth shared
/// by the schema ([`PLAYER_SAVES`]) and the save/load queries, so a rename here
/// flows through every query.
mod col {
    pub const USER_HASH: &str = "user_hash";
    pub const PASSWORD_HASH: &str = "password_hash";
    pub const X: &str = "x";
    pub const Z: &str = "z";
    pub const Y: &str = "y";
    pub const BODY: &str = "body";
    pub const COLORS: &str = "colors";
    pub const GENDER: &str = "gender";
    pub const RUNENERGY: &str = "runenergy";
    pub const PLAYTIME: &str = "playtime";
    pub const STATS: &str = "stats";
    pub const LEVELS: &str = "levels";
    pub const AFK_ZONES: &str = "afk_zones";
    pub const LAST_AFK_ZONE: &str = "last_afk_zone";
    pub const PUBLIC_CHAT: &str = "public_chat";
    pub const PRIVATE_CHAT: &str = "private_chat";
    pub const TRADE_CHAT: &str = "trade_chat";
    pub const LAST_DATE: &str = "last_date";
    pub const VARP_IDS: &str = "varp_ids";
    pub const VARP_VALUES: &str = "varp_values";
    pub const INV_TYPES: &str = "inv_types";
    pub const INV_SLOTS: &str = "inv_slots";
    pub const INV_OBJS: &str = "inv_objs";
    pub const INV_COUNTS: &str = "inv_counts";
    pub const STAFF_MOD_LEVEL: &str = "staff_mod_level";
    pub const UPDATED_AT: &str = "updated_at";
}

/// Declarative schema for the `player_saves` table. Adding (or changing) a
/// column here is picked up by [`ensure_tables`] for both fresh and existing
/// databases.
///
/// `staff_mod_level` is intentionally absent: its default is build-dependent,
/// so it is appended at runtime in [`ensure_tables`].
const PLAYER_SAVES: &[ddl::Column] = &[
    ddl::Column::new(col::USER_HASH, Type::BigInt).primary_key(),
    ddl::Column::new(col::PASSWORD_HASH, Type::Text).not_null(),
    ddl::Column::new(col::X, Type::SmallInt)
        .not_null()
        .default("3094"),
    ddl::Column::new(col::Z, Type::SmallInt)
        .not_null()
        .default("3106"),
    ddl::Column::new(col::Y, Type::SmallInt)
        .not_null()
        .default("0"),
    ddl::Column::new(col::BODY, Type::SmallIntArray)
        .not_null()
        .default("'{0,10,18,26,33,36,42}'"),
    ddl::Column::new(col::COLORS, Type::SmallIntArray)
        .not_null()
        .default("'{0,0,0,0,0}'"),
    ddl::Column::new(col::GENDER, Type::SmallInt)
        .not_null()
        .default("0"),
    ddl::Column::new(col::RUNENERGY, Type::Int)
        .not_null()
        .default("10000"),
    ddl::Column::new(col::PLAYTIME, Type::Int)
        .not_null()
        .default("0"),
    ddl::Column::new(col::STATS, Type::IntArray)
        .not_null()
        .default("'{0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0}'"),
    ddl::Column::new(col::LEVELS, Type::SmallIntArray)
        .not_null()
        .default("'{1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1}'"),
    ddl::Column::new(col::AFK_ZONES, Type::IntArray)
        .not_null()
        .default("'{0,0}'"),
    ddl::Column::new(col::LAST_AFK_ZONE, Type::SmallInt)
        .not_null()
        .default("0"),
    ddl::Column::new(col::PUBLIC_CHAT, Type::SmallInt)
        .not_null()
        .default("0"),
    ddl::Column::new(col::PRIVATE_CHAT, Type::SmallInt)
        .not_null()
        .default("0"),
    ddl::Column::new(col::TRADE_CHAT, Type::SmallInt)
        .not_null()
        .default("0"),
    ddl::Column::new(col::LAST_DATE, Type::BigInt)
        .not_null()
        .default("0"),
    ddl::Column::new(col::VARP_IDS, Type::IntArray)
        .not_null()
        .default("'{}'"),
    ddl::Column::new(col::VARP_VALUES, Type::IntArray)
        .not_null()
        .default("'{}'"),
    ddl::Column::new(col::INV_TYPES, Type::IntArray)
        .not_null()
        .default("'{}'"),
    ddl::Column::new(col::INV_SLOTS, Type::SmallIntArray)
        .not_null()
        .default("'{}'"),
    ddl::Column::new(col::INV_OBJS, Type::IntArray)
        .not_null()
        .default("'{}'"),
    ddl::Column::new(col::INV_COUNTS, Type::IntArray)
        .not_null()
        .default("'{}'"),
    ddl::Column::new(col::UPDATED_AT, Type::Timestamptz)
        .not_null()
        .default("now()"),
];

/// A small structured DDL builder so the schema is declared as typed Rust data
/// instead of a hand-written SQL string. Only the subset of Postgres DDL this
/// layer needs is modeled.
mod ddl {
    /// A SQL column type used by `player_saves`.
    #[derive(Clone, Copy)]
    pub enum Type {
        SmallInt,
        Int,
        BigInt,
        Text,
        Timestamptz,
        SmallIntArray,
        IntArray,
    }

    impl Type {
        const fn sql(self) -> &'static str {
            match self {
                Type::SmallInt => "SMALLINT",
                Type::Int => "INT",
                Type::BigInt => "BIGINT",
                Type::Text => "TEXT",
                Type::Timestamptz => "TIMESTAMPTZ",
                Type::SmallIntArray => "SMALLINT[]",
                Type::IntArray => "INT[]",
            }
        }
    }

    /// A single column definition: name, type, and the constraints this layer
    /// uses (primary key, not-null, default).
    pub struct Column {
        name: &'static str,
        ty: Type,
        primary_key: bool,
        not_null: bool,
        default: Option<&'static str>,
    }

    impl Column {
        pub const fn new(name: &'static str, ty: Type) -> Self {
            Self {
                name,
                ty,
                primary_key: false,
                not_null: false,
                default: None,
            }
        }

        pub const fn primary_key(mut self) -> Self {
            self.primary_key = true;
            self
        }

        pub const fn not_null(mut self) -> Self {
            self.not_null = true;
            self
        }

        /// Sets the column default. `value` is a raw SQL literal or expression
        /// (e.g. `"0"`, `"'{}'"`, `"now()"`).
        pub const fn default(mut self, value: &'static str) -> Self {
            self.default = Some(value);
            self
        }

        /// Renders `<name> <type> [PRIMARY KEY] [NOT NULL] [DEFAULT <x>]`.
        fn definition(&self) -> String {
            let mut s = format!("{} {}", self.name, self.ty.sql());
            if self.primary_key {
                s.push_str(" PRIMARY KEY");
            }
            if self.not_null {
                s.push_str(" NOT NULL");
            }
            if let Some(default) = self.default {
                s.push_str(" DEFAULT ");
                s.push_str(default);
            }
            s
        }
    }

    /// `CREATE TABLE IF NOT EXISTS <table> ( <columns> );`
    pub fn create_table(table: &str, columns: &[Column]) -> String {
        let cols = columns
            .iter()
            .map(Column::definition)
            .collect::<Vec<_>>()
            .join(",\n    ");
        format!("CREATE TABLE IF NOT EXISTS {table} (\n    {cols}\n);\n")
    }

    /// One `ALTER TABLE <table> ADD COLUMN IF NOT EXISTS <col>;` per column, so
    /// an existing table converges to the declared schema.
    pub fn add_columns(table: &str, columns: &[Column]) -> String {
        columns
            .iter()
            .map(|c| {
                format!(
                    "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS {};\n",
                    c.definition()
                )
            })
            .collect()
    }

    /// `DROP TABLE IF EXISTS <table>;`
    pub fn drop_table(table: &str) -> String {
        format!("DROP TABLE IF EXISTS {table};\n")
    }
}

/// Processes incoming database requests in a loop until the channel closes
/// or a database error occurs.
///
/// Dispatches each [`DbRequest`] variant to the appropriate handler
/// ([`authenticate`], [`save_profile`], or [`load_profile`]) and sends
/// the corresponding [`DbResponse`] back to the engine.
///
/// On save failure, falls back to writing a local `.sav` file.
///
/// # Arguments
/// * `client` - An active database connection.
/// * `pepper` - The password pepper secret.
/// * `request_rx` - Channel receiver for incoming requests.
/// * `response_tx` - Channel sender for responses.
///
/// # Returns
/// `Ok(())` when the channel closes, or a database [`Error`] that caused
/// the connection to be considered lost.
///
/// # Call Stack
/// **Called by:** [`db_client_task`]
/// **Calls:** [`authenticate`], [`save_profile`], [`load_profile`]
async fn run_requests(
    client: &mut Client,
    pepper: &str,
    request_rx: &mut UnboundedReceiver<DbRequest>,
    response_tx: &UnboundedSender<DbResponse>,
) -> Result<(), Error> {
    while let Some(req) = request_rx.recv().await {
        match req {
            DbRequest::Authenticate { user37, password } => {
                match authenticate(client, pepper, user37, &password).await {
                    Ok(success) => {
                        let _ = response_tx.send(DbResponse::AuthResponse { user37, success });
                    }
                    Err(e) => {
                        error!("DB auth failed for {}: {}", user37, e);
                        let _ = response_tx.send(DbResponse::AuthResponse {
                            user37,
                            success: false,
                        });
                        return Err(e);
                    }
                }
            }
            DbRequest::Save {
                user37,
                username,
                profile,
                binary,
            } => {
                let result = save_profile(client, user37, &profile).await;
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
            DbRequest::Load { user37 } => match load_profile(client, user37).await {
                Ok(profile) => {
                    let _ = response_tx.send(DbResponse::LoadResponse { user37, profile });
                }
                Err(e) => {
                    error!("DB load failed for {}: {}", user37, e);
                    let _ = response_tx.send(DbResponse::LoadResponse {
                        user37,
                        profile: None,
                    });
                    return Err(e);
                }
            },
        }
    }
    Ok(())
}

/// Authenticates a player against the database using Argon2 password hashing.
///
/// If the player already exists, verifies the password against the stored
/// Argon2 hash. If the player does not exist, creates a new database row
/// with a freshly hashed password and optionally imports a local save file.
///
/// # Arguments
/// * `client` - An active database connection.
/// * `pepper` - The server-side secret pepper.
/// * `user37` - The base-37 encoded username hash.
/// * `password` - The player's plaintext password.
///
/// # Returns
/// `Ok(true)` if authentication succeeds (or a new account is created),
/// `Ok(false)` if the password is wrong or hashing fails, or a database
/// [`Error`].
///
/// # Side Effects
/// * May insert a new row into `player_saves`.
/// * May import and delete a local `.sav` file for new accounts.
///
/// # Call Stack
/// **Called by:** [`run_requests`]
/// **Calls:** [`new_argon2_with_secret`], [`save_profile`]
async fn authenticate(
    client: &mut Client,
    pepper: &str,
    user37: u64,
    password: &str,
) -> Result<bool, Error> {
    let hash = user37 as i64;

    let row = client
        .query_opt(
            "SELECT password_hash FROM player_saves WHERE user_hash = $1",
            &[&hash],
        )
        .await?;

    match row {
        Some(row) => {
            let stored: String = row.get(0);
            let parsed = match PasswordHash::new(&stored) {
                Ok(h) => h,
                Err(_) => return Ok(false),
            };
            let Some(argon2) = new_argon2_with_secret(pepper, user37) else {
                return Ok(false);
            };
            Ok(argon2.verify_password(password.as_bytes(), &parsed).is_ok())
        }
        None => {
            let Some(argon2) = new_argon2_with_secret(pepper, user37) else {
                return Ok(false);
            };
            let Ok(hashed) = argon2.hash_password(password.as_bytes()) else {
                error!("Failed to hash password for user37={}", user37);
                return Ok(false);
            };
            client
                .execute(
                    "INSERT INTO player_saves (user_hash, password_hash) VALUES ($1, $2)",
                    &[&hash, &hashed.to_string()],
                )
                .await?;

            let username = to_raw_username(user37);
            if let Some(data) = load_from_file(&username)
                && let Ok(profile) = load_binary(&data)
                && save_profile(client, user37, &profile).await?
            {
                delete_save_file(&username);
                info!("Imported local save for '{}' into new DB row", username);
            }

            Ok(true)
        }
    }
}

fn new_argon2_with_secret(pepper: &str, user37: u64) -> Option<Argon2<'_>> {
    match Argon2::new_with_secret(
        pepper.as_bytes(),
        Algorithm::default(),
        Version::default(),
        Params::default(),
    ) {
        Ok(argon2) => Some(argon2),
        Err(e) => {
            error!("Failed to hash password for user37={user37}: {e}");
            None
        }
    }
}

/// `player_saves` columns that map to a [`PlayerProfile`] field. Drives the
/// load `SELECT`; the matching `row.get` reads use the same `col::` names.
const PROFILE_COLUMNS: &[&str] = &[
    col::X,
    col::Z,
    col::Y,
    col::BODY,
    col::COLORS,
    col::GENDER,
    col::RUNENERGY,
    col::PLAYTIME,
    col::STATS,
    col::LEVELS,
    col::AFK_ZONES,
    col::LAST_AFK_ZONE,
    col::PUBLIC_CHAT,
    col::PRIVATE_CHAT,
    col::TRADE_CHAT,
    col::LAST_DATE,
    col::VARP_IDS,
    col::VARP_VALUES,
    col::INV_TYPES,
    col::INV_SLOTS,
    col::INV_OBJS,
    col::INV_COUNTS,
    col::STAFF_MOD_LEVEL,
];

/// Builds `UPDATE player_saves SET <col>=$2, ..., updated_at=now() WHERE user_hash=$1`
/// from an ordered list of column names. Parameter positions start at `$2`;
/// `$1` is reserved for the `user_hash` key.
fn update_player_saves_sql<'a>(columns: impl IntoIterator<Item = &'a str>) -> String {
    let assignments = columns
        .into_iter()
        .enumerate()
        .map(|(i, name)| format!("{name}=${}", i + 2))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "UPDATE player_saves SET {assignments}, {}=now() WHERE {}=$1",
        col::UPDATED_AT,
        col::USER_HASH,
    )
}

/// Builds `SELECT <cols> FROM player_saves WHERE user_hash=$1`.
fn select_player_saves_sql(columns: &[&str]) -> String {
    format!(
        "SELECT {} FROM player_saves WHERE {}=$1",
        columns.join(", "),
        col::USER_HASH,
    )
}

/// Persists a player profile to the database in a single `UPDATE`.
///
/// All persistent state -- including varps and inventories -- lives on the
/// player's one `player_saves` row. Varps are written as the index-aligned
/// `varp_ids`/`varp_values` arrays; inventory items are flattened across the
/// index-aligned `inv_types`/`inv_slots`/`inv_objs`/`inv_counts` arrays
/// (grouped by inventory type as emitted by `extract_profile`).
///
/// The `UPDATE` is generated from `(column, value)` pairs so the bound
/// parameters and their `$n` positions cannot drift; column names come from
/// `col` rather than being written inline.
///
/// # Arguments
/// * `client` - An active database connection.
/// * `user37` - The base-37 encoded username hash.
/// * `profile` - The player profile data to save.
///
/// # Returns
/// `Ok(true)` if the row was updated, `Ok(false)` if no matching row exists
/// (the `UPDATE` affected 0 rows), or a database [`Error`].
///
/// # Call Stack
/// **Called by:** [`run_requests`], [`authenticate`], [`sync_local_saves`]
async fn save_profile(
    client: &mut Client,
    user37: u64,
    profile: &PlayerProfile,
) -> Result<bool, Error> {
    let hash = user37 as i64;

    let x = profile.x as i16;
    let z = profile.z as i16;
    let y = profile.y as i16;
    let body: Vec<i16> = profile.body.iter().map(|&v| v as i16).collect();
    let colors: Vec<i16> = profile.colors.iter().map(|&v| v as i16).collect();
    let gender = profile.gender as i16;
    let runenergy = profile.runenergy as i32;
    let playtime = profile.playtime;
    let stats: Vec<i32> = profile.stats.to_vec();
    let levels: Vec<i16> = profile.levels.iter().map(|&v| v as i16).collect();
    let afk_zones: Vec<i32> = profile.afk_zones.iter().map(|&v| v as i32).collect();
    let last_afk_zone = profile.last_afk_zone as i16;
    let public_chat = profile.public_chat as i16;
    let private_chat = profile.private_chat as i16;
    let trade_chat = profile.trade_chat as i16;
    let last_date = profile.last_date;
    let staff_mod_level = profile.staff_mod_level as i16;

    let varp_ids: Vec<i32> = profile.varps.iter().map(|&(id, _)| id as i32).collect();
    let varp_values: Vec<i32> = profile.varps.iter().map(|&(_, v)| v).collect();

    let mut inv_types: Vec<i32> = Vec::new();
    let mut inv_slots: Vec<i16> = Vec::new();
    let mut inv_objs: Vec<i32> = Vec::new();
    let mut inv_counts: Vec<i32> = Vec::new();
    for inv in &profile.invs {
        for &(slot, obj_id, count) in &inv.items {
            inv_types.push(inv.inv_type as i32);
            inv_slots.push(slot as i16);
            inv_objs.push(obj_id as i32);
            inv_counts.push(count as i32);
        }
    }

    // Each column is paired with its value, so the generated `$n` positions and
    // the bound parameters are guaranteed to line up.
    let updates: [(&str, &(dyn ToSql + Sync)); PROFILE_COLUMNS.len()] = [
        (col::X, &x),
        (col::Z, &z),
        (col::Y, &y),
        (col::BODY, &body),
        (col::COLORS, &colors),
        (col::GENDER, &gender),
        (col::RUNENERGY, &runenergy),
        (col::PLAYTIME, &playtime),
        (col::STATS, &stats),
        (col::LEVELS, &levels),
        (col::AFK_ZONES, &afk_zones),
        (col::LAST_AFK_ZONE, &last_afk_zone),
        (col::PUBLIC_CHAT, &public_chat),
        (col::PRIVATE_CHAT, &private_chat),
        (col::TRADE_CHAT, &trade_chat),
        (col::LAST_DATE, &last_date),
        (col::VARP_IDS, &varp_ids),
        (col::VARP_VALUES, &varp_values),
        (col::INV_TYPES, &inv_types),
        (col::INV_SLOTS, &inv_slots),
        (col::INV_OBJS, &inv_objs),
        (col::INV_COUNTS, &inv_counts),
        (col::STAFF_MOD_LEVEL, &staff_mod_level),
    ];

    let sql = update_player_saves_sql(updates.iter().map(|(name, _)| *name));

    let mut params: Vec<&(dyn ToSql + Sync)> = Vec::with_capacity(updates.len() + 1);
    params.push(&hash);
    params.extend(updates.iter().map(|(_, value)| *value));

    let rows = client.execute(&sql, &params).await?;
    Ok(rows == 1)
}

/// Loads a player profile from the database.
///
/// Reads the player's single `player_saves` row -- including the inline varp
/// and inventory arrays -- and assembles a [`PlayerProfile`]. Returns `None`
/// if no row exists for the given user hash.
///
/// # Arguments
/// * `client` - An active database connection.
/// * `user37` - The base-37 encoded username hash.
///
/// # Returns
/// `Ok(Some(profile))` if the player exists, `Ok(None)` if not, or a
/// database [`Error`].
///
/// # Call Stack
/// **Called by:** [`run_requests`]
async fn load_profile(client: &Client, user37: u64) -> Result<Option<PlayerProfile>, Error> {
    let hash = user37 as i64;

    let row = client
        .query_opt(&select_player_saves_sql(PROFILE_COLUMNS), &[&hash])
        .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    // Columns are read by name (via `col`), so the SELECT order and these reads
    // can't fall out of sync.
    let x: i16 = row.get(col::X);
    let z: i16 = row.get(col::Z);
    let y: i16 = row.get(col::Y);
    let body_vec: Vec<i16> = row.get(col::BODY);
    let colors_vec: Vec<i16> = row.get(col::COLORS);
    let gender: i16 = row.get(col::GENDER);
    let runenergy: i32 = row.get(col::RUNENERGY);
    let playtime: i32 = row.get(col::PLAYTIME);
    let stats_vec: Vec<i32> = row.get(col::STATS);
    let levels_vec: Vec<i16> = row.get(col::LEVELS);
    let afk_zones_vec: Vec<i32> = row.get(col::AFK_ZONES);
    let last_afk_zone: i16 = row.get(col::LAST_AFK_ZONE);
    let public_chat: i16 = row.get(col::PUBLIC_CHAT);
    let private_chat: i16 = row.get(col::PRIVATE_CHAT);
    let trade_chat: i16 = row.get(col::TRADE_CHAT);
    let last_date: i64 = row.get(col::LAST_DATE);

    let mut body = [0; 7];
    for (i, &v) in body_vec.iter().enumerate().take(7) {
        body[i] = v as i32;
    }
    let mut colors = [0; 5];
    for (i, &v) in colors_vec.iter().enumerate().take(5) {
        colors[i] = v as u8;
    }
    let mut stats = [0; 21];
    for (i, &v) in stats_vec.iter().enumerate().take(21) {
        stats[i] = v;
    }
    let mut levels = [1; 21];
    for (i, &v) in levels_vec.iter().enumerate().take(21) {
        levels[i] = v as u8;
    }
    let mut afk_zones = [0; 2];
    for (i, &v) in afk_zones_vec.iter().enumerate().take(2) {
        afk_zones[i] = v as u32;
    }

    let varp_ids: Vec<i32> = row.get(col::VARP_IDS);
    let varp_values: Vec<i32> = row.get(col::VARP_VALUES);
    let varps: Vec<(u16, i32)> = varp_ids
        .iter()
        .zip(varp_values.iter())
        .map(|(&id, &val)| (id as u16, val))
        .collect();

    // Inventory items are stored flattened across four index-aligned arrays
    // (grouped by inv_type at save time); regroup them back into per-inventory
    // lists. The find() tolerates any ordering of the flattened entries.
    let inv_types: Vec<i32> = row.get(col::INV_TYPES);
    let inv_slots: Vec<i16> = row.get(col::INV_SLOTS);
    let inv_objs: Vec<i32> = row.get(col::INV_OBJS);
    let inv_counts: Vec<i32> = row.get(col::INV_COUNTS);
    let staff_mod_level: i16 = row.get(col::STAFF_MOD_LEVEL);

    let mut invs: Vec<PlayerProfileInv> = Vec::new();
    for (i, inv_type) in inv_types.iter().map(|x| *x as u16).enumerate() {
        let item = (
            inv_slots.get(i).copied().unwrap_or(0) as u16,
            inv_objs.get(i).copied().unwrap_or(0) as u16,
            inv_counts.get(i).copied().unwrap_or(0) as u32,
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

    Ok(Some(PlayerProfile {
        x: x as u16,
        z: z as u16,
        y: y as u8,
        body,
        colors,
        gender: gender as u8,
        runenergy: runenergy as u16,
        playtime,
        stats,
        levels,
        varps,
        invs,
        afk_zones,
        last_afk_zone: last_afk_zone as u16,
        public_chat: public_chat as u8,
        private_chat: private_chat as u8,
        trade_chat: trade_chat as u8,
        last_date,
        staff_mod_level: staff_mod_level as u8,
    }))
}

#[cfg(test)]
mod ddl_tests {
    use super::*;

    #[test]
    fn create_table_renders_declared_columns() {
        let sql = ddl::create_table("player_saves", PLAYER_SAVES);
        assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS player_saves ("));
        assert!(sql.contains("user_hash BIGINT PRIMARY KEY"));
        assert!(sql.contains("password_hash TEXT NOT NULL"));
        assert!(sql.contains("x SMALLINT NOT NULL DEFAULT 3094"));
        assert!(sql.contains("body SMALLINT[] NOT NULL DEFAULT '{0,10,18,26,33,36,42}'"));
        assert!(sql.contains("varp_ids INT[] NOT NULL DEFAULT '{}'"));
        assert!(sql.contains("updated_at TIMESTAMPTZ NOT NULL DEFAULT now()"));
        assert!(sql.trim_end().ends_with(");"));
    }

    #[test]
    fn add_columns_emits_one_idempotent_alter_per_column() {
        let sql = ddl::add_columns("player_saves", PLAYER_SAVES);
        assert_eq!(
            sql.matches("ADD COLUMN IF NOT EXISTS").count(),
            PLAYER_SAVES.len()
        );
        assert!(sql.contains(
            "ALTER TABLE player_saves ADD COLUMN IF NOT EXISTS inv_counts INT[] NOT NULL DEFAULT '{}';"
        ));
    }

    #[test]
    fn drop_table_renders() {
        assert_eq!(
            ddl::drop_table("player_varps"),
            "DROP TABLE IF EXISTS player_varps;\n"
        );
    }

    #[test]
    fn constraints_are_optional() {
        // A bare column renders just `name type`, no NOT NULL / DEFAULT.
        let sql = ddl::create_table("t", &[ddl::Column::new("a", Type::Int)]);
        assert!(sql.contains("a INT"));
        assert!(!sql.contains("NOT NULL"));
        assert!(!sql.contains("DEFAULT"));
    }
}

#[cfg(test)]
mod dml_tests {
    use super::*;

    #[test]
    fn update_sql_numbers_params_from_two() {
        let sql = update_player_saves_sql(["a", "b", "c"]);
        assert_eq!(
            sql,
            "UPDATE player_saves SET a=$2, b=$3, c=$4, updated_at=now() WHERE user_hash=$1"
        );
    }

    #[test]
    fn select_sql_lists_columns() {
        let sql = select_player_saves_sql(&["a", "b"]);
        assert_eq!(sql, "SELECT a, b FROM player_saves WHERE user_hash=$1");
    }

    #[test]
    fn save_and_load_cover_the_same_columns() {
        // Every profile column appears in both the UPDATE and the SELECT.
        let update_sql = update_player_saves_sql(PROFILE_COLUMNS.iter().copied());
        let select_sql = select_player_saves_sql(PROFILE_COLUMNS);
        for &c in PROFILE_COLUMNS {
            assert!(update_sql.contains(&format!("{c}=$")), "UPDATE missing {c}");
            assert!(select_sql.contains(c), "SELECT missing {c}");
        }
    }
}
