use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use std::fmt;

/// The most characters the game's login screen takes for a password.
pub const MAX_PASSWORD_CHARS: usize = 20;
/// The fewest a password may be set to (RuneScape's rule). Logins take
/// shorter ones, as the login screen does.
pub const MIN_PASSWORD_CHARS: usize = 5;

/// Whether the game's login screen can type `c` in a password: its keyboard
/// is the client's `CHARSET` (the same in every revision), which is
/// printable ASCII without the backtick, plus `£`. The client sends it as
/// cp1252, and the login block decodes it back to the same character.
pub fn is_password_char(c: char) -> bool {
    (c != '`' && (' '..='~').contains(&c)) || c == '£'
}

/// Whether `password` could have come from the login screen: 1 to
/// [`MAX_PASSWORD_CHARS`] characters it can type. No other password can be
/// anyone's, so a login with one is refused before it's checked.
pub fn is_typeable_password(password: &str) -> bool {
    let count = password.chars().count();
    (1..=MAX_PASSWORD_CHARS).contains(&count) && password.chars().all(is_password_char)
}

/// Why a password can't be set, from [`check_new_password`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordError {
    Length,
    Characters,
}

impl fmt::Display for PasswordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PasswordError::Length => "Your password must be between 5 and 20 characters long.",
            PasswordError::Characters => {
                "Your password may only contain letters, numbers, spaces and keyboard symbols (not `)."
            }
        })
    }
}

/// Checks a password a player wants to set (at registration or a reset):
/// [`MIN_PASSWORD_CHARS`] to [`MAX_PASSWORD_CHARS`] characters the login
/// screen can type, so it can always be logged in with.
pub fn check_new_password(password: &str) -> Result<(), PasswordError> {
    let count = password.chars().count();
    if !(MIN_PASSWORD_CHARS..=MAX_PASSWORD_CHARS).contains(&count) {
        return Err(PasswordError::Length);
    }
    if !password.chars().all(is_password_char) {
        return Err(PasswordError::Characters);
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Passwords {
    pepper: Box<[u8]>,
}

impl Passwords {
    pub fn new(pepper: &str) -> Self {
        Self {
            pepper: pepper.as_bytes().into(),
        }
    }

    fn argon2(&self) -> Option<Argon2<'_>> {
        Argon2::new_with_secret(
            &self.pepper,
            Algorithm::default(),
            Version::default(),
            Params::default(),
        )
        .ok()
    }

    /// Hashes `password` into a PHC string (algorithm, params, salt and hash).
    /// Returns `None` only if hashing itself fails.
    pub fn hash(&self, password: &str) -> Option<String> {
        let hash = self.argon2()?.hash_password(password.as_bytes()).ok()?;
        Some(hash.to_string())
    }

    /// Checks `password` against a PHC string produced by [`Passwords::hash`].
    /// A malformed `stored` hash never verifies.
    pub fn verify(&self, password: &str, stored: &str) -> bool {
        let Ok(parsed) = PasswordHash::new(stored) else {
            return false;
        };
        self.argon2()
            .is_some_and(|a| a.verify_password(password.as_bytes(), &parsed).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify() {
        let p = Passwords::new("pepper");
        let hash = p.hash("hunter2").unwrap();
        assert!(p.verify("hunter2", &hash));
        assert!(!p.verify("hunter3", &hash));
    }

    #[test]
    fn pepper_is_part_of_the_hash() {
        let hash = Passwords::new("a").hash("hunter2").unwrap();
        assert!(!Passwords::new("b").verify("hunter2", &hash));
    }

    #[test]
    fn passwords_are_what_the_login_screen_types() {
        // The client's CHARSET, as every revision has it.
        const CHARSET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!\"£$%^&*()-_=+[{]};:'@#~,<.>/?\\| ";
        let typed: Vec<char> = (0..=0x2FFF_u32)
            .filter_map(char::from_u32)
            .filter(|&c| is_password_char(c))
            .collect();
        let mut charset: Vec<char> = CHARSET.chars().collect();
        charset.sort_unstable();
        assert_eq!(typed, charset);

        assert!(check_new_password("hunter2").is_ok());
        assert!(check_new_password("p@ss w0rd £5!").is_ok());
        assert!(check_new_password(&"£".repeat(20)).is_ok());
        assert_eq!(check_new_password("four"), Err(PasswordError::Length));
        assert_eq!(
            check_new_password(&"x".repeat(21)),
            Err(PasswordError::Length)
        );
        assert_eq!(
            check_new_password("back`tick"),
            Err(PasswordError::Characters)
        );
        assert_eq!(
            check_new_password("café au lait"),
            Err(PasswordError::Characters)
        );
        assert_eq!(
            check_new_password("new\nline"),
            Err(PasswordError::Characters)
        );

        assert!(is_typeable_password("a"));
        assert!(is_typeable_password(&"£".repeat(20)));
        assert!(!is_typeable_password(""));
        assert!(!is_typeable_password(&"a".repeat(21)));
        assert!(!is_typeable_password("tab\there"));
    }

    #[test]
    fn malformed_hash_never_verifies() {
        assert!(!Passwords::new("pepper").verify("hunter2", "not a phc string"));
    }
}
