use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};

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
    fn malformed_hash_never_verifies() {
        assert!(!Passwords::new("pepper").verify("hunter2", "not a phc string"));
    }
}
