use crate::totp;
use anyhow::{Context, Result, bail};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

/// The shortest password a console account may have.
pub const MIN_PASSWORD_LEN: usize = 12;
/// The longest mute a trial moderator gives.
pub const TRIAL_MAX_MUTE_HOURS: i32 = 48;

/// Ordered by rank, matching the game's staff levels.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, clap::ValueEnum,
)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// A moderator still learning: kicks and short mutes only (see
    /// [`Role::is_trial`]).
    Trial,
    Moderator,
    Admin,
    /// Was `developer`: accounts files and scripts from before still say so.
    #[serde(alias = "developer")]
    #[value(alias = "developer")]
    Maintainer,
}

impl Role {
    /// The game staff level a world treats them as (0 is a player). Nobody
    /// at or above it can be moderated by them.
    pub fn staff_level(self) -> u8 {
        match self {
            Self::Trial | Self::Moderator => 1,
            Self::Admin => 2,
            Self::Maintainer => 3,
        }
    }

    /// Whether they may reboot worlds and broadcast.
    pub fn can_command(self) -> bool {
        self >= Self::Admin
    }

    /// Whether they're a trial moderator: they kick and mute for up to
    /// [`TRIAL_MAX_MUTE_HOURS`], but don't ban, lift sanctions, sanction
    /// linked accounts, dismiss in bulk or decide appeals.
    pub fn is_trial(self) -> bool {
        self == Self::Trial
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub role: Role,
    pub password_hash: String,
    #[serde(default)]
    pub totp_secret: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub created_ms: u64,
    #[serde(default)]
    pub signed_out_ms: u64,
}

impl User {
    /// Whether a session signed in at `signed_in_ms` may continue.
    pub fn accepts_session(&self, signed_in_ms: u64) -> bool {
        !self.disabled && signed_in_ms >= self.signed_out_ms
    }

    pub fn totp_secret(&self) -> Option<Vec<u8>> {
        self.totp_secret.as_deref().and_then(totp::unbase32)
    }
}

#[derive(Default, Serialize, Deserialize)]
struct UsersFile {
    users: Vec<User>,
}

/// The key two names are the same account under.
pub fn name_key(name: &str) -> String {
    name.trim().to_lowercase().replace('_', " ")
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// When a file was last changed and how long it was then (`None` while
/// there's none): it's read again only when that changes.
type Stamp = Option<(SystemTime, u64)>;

pub struct Users {
    path: PathBuf,
    lock: Mutex<()>,
    /// The accounts as last read or written, and the file's stamp then.
    loaded: Mutex<Option<(Stamp, Vec<User>)>>,
}

impl Users {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
            loaded: Mutex::new(None),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn read(&self) -> Result<UsersFile> {
        match fs::read_to_string(&self.path) {
            Ok(text) => serde_json::from_str(&text)
                .with_context(|| format!("parsing {}", self.path.display())),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(UsersFile::default()),
            Err(e) => Err(e).with_context(|| format!("reading {}", self.path.display())),
        }
    }

    fn stamp(&self) -> Result<Stamp> {
        match fs::metadata(&self.path) {
            Ok(meta) => Ok(Some((meta.modified()?, meta.len()))),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).with_context(|| format!("reading {}", self.path.display())),
        }
    }

    fn loaded(&self) -> MutexGuard<'_, Option<(Stamp, Vec<User>)>> {
        self.loaded.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Runs `f` on the accounts, reading the file again only when it has
    /// changed (the command line changes it too).
    fn with<R>(&self, f: impl FnOnce(&[User]) -> R) -> Result<R> {
        let stamp = self.stamp()?;
        let mut loaded = self.loaded();
        match &*loaded {
            Some((at, users)) if *at == stamp => Ok(f(users)),
            _ => {
                let users = self.read()?.users;
                let result = f(&users);
                *loaded = Some((stamp, users));
                Ok(result)
            }
        }
    }

    /// Reads the file afresh to change it with `f`, then writes it.
    fn change<R>(&self, f: impl FnOnce(&mut Vec<User>) -> Result<R>) -> Result<R> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut file = self.read()?;
        let result = f(&mut file.users)?;
        self.write(&file)?;
        // Two writes can land within one tick of the clock at the same
        // length, so what was written is kept rather than read back.
        *self.loaded() = Some((self.stamp()?, file.users));
        Ok(result)
    }

    /// Writes beside the file and renames over it, so a reader never sees a
    /// half-written file. Owner-only where the platform has modes.
    fn write(&self, file: &UsersFile) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(file)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
        }
        fs::rename(&tmp, &self.path).with_context(|| format!("replacing {}", self.path.display()))
    }

    pub fn all(&self) -> Result<Vec<User>> {
        self.with(<[User]>::to_vec)
    }

    pub fn find(&self, name: &str) -> Result<Option<User>> {
        let key = name_key(name);
        self.with(|users| users.iter().find(|u| name_key(&u.name) == key).cloned())
    }

    /// Changes the account `name` with `f`. Fails if there is none.
    pub fn update(&self, name: &str, f: impl FnOnce(&mut User)) -> Result<User> {
        let key = name_key(name);
        self.change(|users| {
            let Some(user) = users.iter_mut().find(|u| name_key(&u.name) == key) else {
                bail!("no account named {name}");
            };
            f(user);
            Ok(user.clone())
        })
    }

    pub fn add(&self, user: User) -> Result<()> {
        let key = name_key(&user.name);
        self.change(|users| {
            if users.iter().any(|u| name_key(&u.name) == key) {
                bail!("an account named {} already exists", user.name);
            }
            users.push(user);
            Ok(())
        })
    }

    pub fn remove(&self, name: &str) -> Result<()> {
        let key = name_key(name);
        self.change(|users| {
            let before = users.len();
            users.retain(|u| name_key(&u.name) != key);
            if users.len() == before {
                bail!("no account named {name}");
            }
            Ok(())
        })
    }
}

/// Hashes a password into a PHC string (argon2id, random salt).
pub fn hash_password(password: &str) -> Result<String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("hashing failed: {e}"))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

/// Why a new password isn't acceptable, if it isn't.
pub fn password_problem(password: &str, name: &str) -> Option<String> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Some(format!(
            "Passwords must be at least {MIN_PASSWORD_LEN} characters."
        ));
    }
    if password.chars().count() > 256 {
        return Some("Passwords must be at most 256 characters.".into());
    }
    if name_key(password).contains(&name_key(name)) {
        return Some("Passwords can't contain the account name.".into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_like_game_names() {
        assert_eq!(name_key(" Mod_Ash "), name_key("mod ash"));
        assert_ne!(name_key("mod ash"), name_key("modash"));
    }

    #[test]
    fn roles_are_game_staff_levels() {
        assert_eq!(Role::Trial.staff_level(), 1);
        assert_eq!(Role::Moderator.staff_level(), 1);
        assert!(Role::Trial < Role::Moderator && Role::Trial.is_trial());
        assert_eq!(Role::Admin.staff_level(), 2);
        assert_eq!(Role::Maintainer.staff_level(), 3);
        assert!(!Role::Moderator.can_command());
        assert!(Role::Admin.can_command() && Role::Maintainer.can_command());
    }

    #[test]
    fn developers_from_before_are_maintainers() {
        let role: Role = serde_json::from_str("\"developer\"").unwrap();
        assert_eq!(role, Role::Maintainer);
        assert_eq!(serde_json::to_string(&role).unwrap(), "\"maintainer\"");
        assert_eq!(
            <Role as clap::ValueEnum>::from_str("developer", false),
            Ok(Role::Maintainer)
        );
    }

    #[test]
    fn passwords_verify_against_their_hash() {
        let hash = hash_password("correct horse battery").unwrap();
        assert!(verify_password("correct horse battery", &hash));
        assert!(!verify_password("correct horse battery!", &hash));
        assert!(!verify_password("anything", "not a hash"));
    }

    #[test]
    fn weak_passwords_are_refused() {
        assert!(password_problem("short", "ash").is_some());
        assert!(password_problem("i am mod ash really", "Mod_Ash").is_some());
        assert!(password_problem("a much longer passphrase", "ash2").is_none());
    }

    #[test]
    fn file_changes_round_trip() {
        let dir = std::env::temp_dir().join(format!("rs-admin-users-{}", now_ms()));
        let users = Users::new(dir.join("users.json"));
        users
            .add(User {
                name: "Mod Ash".into(),
                role: Role::Admin,
                password_hash: "x".into(),
                totp_secret: None,
                disabled: false,
                created_ms: 0,
                signed_out_ms: 0,
            })
            .unwrap();
        assert!(users.add(users.find("mod_ash").unwrap().unwrap()).is_err());
        users.update("MOD ASH", |u| u.disabled = true).unwrap();
        assert!(users.find("mod ash").unwrap().unwrap().disabled);
        users.remove("mod_ash").unwrap();
        assert!(users.find("mod ash").unwrap().is_none());
        // Changed from the command line: read again.
        std::fs::write(
            users.path(),
            r#"{"users":[{"name":"Mod Kat","role":"moderator","password_hash":"y"}]}"#,
        )
        .unwrap();
        assert_eq!(users.all().unwrap()[0].name, "Mod Kat");
        std::fs::remove_file(users.path()).unwrap();
        assert!(users.all().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
