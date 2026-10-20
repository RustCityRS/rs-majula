use crate::totp;
use anyhow::{Context, Result, bail};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// The shortest password a console account may have.
pub const MIN_PASSWORD_LEN: usize = 12;

/// Ordered by rank, matching the game's staff levels.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, clap::ValueEnum,
)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Moderator,
    Admin,
    Developer,
}

impl Role {
    /// The game staff level a world treats them as (0 is a player). Nobody
    /// at or above it can be moderated by them.
    pub fn staff_level(self) -> u8 {
        match self {
            Self::Moderator => 1,
            Self::Admin => 2,
            Self::Developer => 3,
        }
    }

    /// Whether they may reboot worlds and broadcast.
    pub fn can_command(self) -> bool {
        self >= Self::Admin
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

pub struct Users {
    path: PathBuf,
    lock: Mutex<()>,
}

impl Users {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn read(&self) -> Result<UsersFile> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => serde_json::from_str(&text)
                .with_context(|| format!("parsing {}", self.path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(UsersFile::default()),
            Err(e) => Err(e).with_context(|| format!("reading {}", self.path.display())),
        }
    }

    /// Writes beside the file and renames over it, so a reader never sees a
    /// half-written file. Owner-only where the platform has modes.
    fn write(&self, file: &UsersFile) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(file)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::rename(&tmp, &self.path)
            .with_context(|| format!("replacing {}", self.path.display()))
    }

    pub fn all(&self) -> Result<Vec<User>> {
        Ok(self.read()?.users)
    }

    pub fn find(&self, name: &str) -> Result<Option<User>> {
        let key = name_key(name);
        Ok(self
            .read()?
            .users
            .into_iter()
            .find(|u| name_key(&u.name) == key))
    }

    /// Changes the account `name` with `f`. Fails if there is none.
    pub fn update(&self, name: &str, f: impl FnOnce(&mut User)) -> Result<User> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut file = self.read()?;
        let key = name_key(name);
        let Some(user) = file.users.iter_mut().find(|u| name_key(&u.name) == key) else {
            bail!("no account named {name}");
        };
        f(user);
        let user = user.clone();
        self.write(&file)?;
        Ok(user)
    }

    pub fn add(&self, user: User) -> Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut file = self.read()?;
        let key = name_key(&user.name);
        if file.users.iter().any(|u| name_key(&u.name) == key) {
            bail!("an account named {} already exists", user.name);
        }
        file.users.push(user);
        self.write(&file)
    }

    pub fn remove(&self, name: &str) -> Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut file = self.read()?;
        let key = name_key(name);
        let before = file.users.len();
        file.users.retain(|u| name_key(&u.name) != key);
        if file.users.len() == before {
            bail!("no account named {name}");
        }
        self.write(&file)
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
        assert_eq!(Role::Moderator.staff_level(), 1);
        assert_eq!(Role::Admin.staff_level(), 2);
        assert_eq!(Role::Developer.staff_level(), 3);
        assert!(!Role::Moderator.can_command());
        assert!(Role::Admin.can_command() && Role::Developer.can_command());
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
        let _ = std::fs::remove_dir_all(dir);
    }
}
