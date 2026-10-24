use hmac::{Hmac, KeyInit, Mac};
use rs_admin_api::hex;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

/// How long a sign-in token lasts. Each page the Message Centre shows
/// carries a new one, so it's 30 minutes from the last click.
pub const TOKEN_SECS: u64 = 30 * 60;

/// Signed ahead of every token, so nothing else this key might sign could
/// pass for one.
const PURPOSE: &[u8] = b"rs-web message centre\0";

/// The player's row and the expiry, then their HMAC-SHA256.
const TOKEN_BYTES: usize = 16 + 32;

/// Proof of a sign-in that the Message Centre's pages carry from click to
/// click in a hidden form field, as the site sets no cookies: the player's
/// row (never reused, unlike a name) and an expiry, signed with HMAC-SHA256
/// under a random key made at startup, so every token dies with a restart.
/// The signature also covers the player's password hash, which the token
/// doesn't carry, so changing the password ends every token they hold, and
/// deleting the account leaves no row for one to name.
pub struct SignInTokens {
    key: [u8; 32],
}

impl SignInTokens {
    pub fn new() -> Self {
        let mut key = [0u8; 32];
        getrandom::fill(&mut key).expect("the OS has no randomness");
        Self { key }
    }

    /// A token signing in the player with row `player_id` and
    /// `password_hash` for [`TOKEN_SECS`].
    pub fn issue(&self, player_id: i64, password_hash: &str) -> String {
        self.issue_until(player_id, password_hash, now() + TOKEN_SECS)
    }

    /// The row a token names, if it's well formed and unexpired. This is
    /// only which player to look up: it isn't checked until [`Self::check`]
    /// is given their password hash.
    pub fn claimed(&self, token: &str) -> Option<i64> {
        let bytes = unhex(token)?;
        let (player_id, expires) = fields(&bytes);
        (now() < expires).then_some(player_id)
    }

    /// Whether a token is this key's, unexpired, and signed for the player
    /// with row `player_id` while their password hash was `password_hash`.
    pub fn check(&self, token: &str, player_id: i64, password_hash: &str) -> bool {
        self.check_at(token, player_id, password_hash, now())
    }

    fn issue_until(&self, player_id: i64, password_hash: &str, expires: u64) -> String {
        let mut bytes = [0u8; TOKEN_BYTES];
        bytes[..8].copy_from_slice(&player_id.to_be_bytes());
        bytes[8..16].copy_from_slice(&expires.to_be_bytes());
        let mac = self.mac(&bytes[..16], password_hash);
        bytes[16..].copy_from_slice(&mac.finalize().into_bytes());
        hex(&bytes)
    }

    fn check_at(&self, token: &str, player_id: i64, password_hash: &str, now: u64) -> bool {
        let Some(bytes) = unhex(token) else {
            return false;
        };
        let (claimed, expires) = fields(&bytes);
        // In constant time.
        let signed = self
            .mac(&bytes[..16], password_hash)
            .verify_slice(&bytes[16..])
            .is_ok();
        signed && claimed == player_id && now < expires
    }

    /// The MAC over a token's fields and the password hash it was issued
    /// under. The fields are fixed width and the hash goes last, so no two
    /// inputs run together.
    fn mac(&self, fields: &[u8], password_hash: &str) -> Hmac<Sha256> {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.key).expect("HMAC accepts any key length");
        mac.update(PURPOSE);
        mac.update(fields);
        mac.update(password_hash.as_bytes());
        mac
    }
}

/// A token's player row and expiry.
fn fields(bytes: &[u8; TOKEN_BYTES]) -> (i64, u64) {
    let mut player_id = [0u8; 8];
    let mut expires = [0u8; 8];
    player_id.copy_from_slice(&bytes[..8]);
    expires.copy_from_slice(&bytes[8..16]);
    (i64::from_be_bytes(player_id), u64::from_be_bytes(expires))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// A token's bytes, if it's exactly [`TOKEN_BYTES`] of hex.
fn unhex(token: &str) -> Option<[u8; TOKEN_BYTES]> {
    // Digits only: `from_str_radix` would take a sign too.
    if token.len() != TOKEN_BYTES << 1 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = [0u8; TOKEN_BYTES];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(token.get(i << 1..(i << 1) + 2)?, 16).ok()?;
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: i64 = 0x0123_4567_89ab_cdef;
    const HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA";

    #[test]
    fn tokens_round_trip() {
        let tokens = SignInTokens::new();
        let token = tokens.issue(ID, HASH);
        assert_eq!(tokens.claimed(&token), Some(ID));
        assert!(tokens.check(&token, ID, HASH));
        let first = tokens.issue(1, HASH);
        assert!(tokens.check(&first, 1, HASH));
    }

    #[test]
    fn tokens_expire() {
        let tokens = SignInTokens::new();
        let token = tokens.issue_until(ID, HASH, 1_000);
        assert!(tokens.check_at(&token, ID, HASH, 999));
        assert!(!tokens.check_at(&token, ID, HASH, 1_000));
        assert!(!tokens.check_at(&token, ID, HASH, 5_000));
        assert_eq!(tokens.claimed(&token), None);
        let now = now();
        let fresh = tokens.issue(ID, HASH);
        assert!(tokens.check_at(&fresh, ID, HASH, now + TOKEN_SECS - 5));
        assert!(!tokens.check_at(&fresh, ID, HASH, now + TOKEN_SECS + 5));
    }

    #[test]
    fn a_new_password_ends_its_tokens() {
        let tokens = SignInTokens::new();
        let token = tokens.issue(ID, HASH);
        assert!(!tokens.check(&token, ID, "$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$bmV3"));
        assert!(!tokens.check(&token, ID, ""));
    }

    #[test]
    fn tokens_are_for_their_own_player() {
        let tokens = SignInTokens::new();
        let token = tokens.issue(ID, HASH);
        // Another row, even one with the same password hash (a name taken
        // again after its account was deleted is a new row).
        assert!(!tokens.check(&token, ID + 1, HASH));
    }

    #[test]
    fn tampered_tokens_fail() {
        let tokens = SignInTokens::new();
        let token = tokens.issue(ID, HASH);
        // Every character changed in turn: the row, the expiry, the MAC.
        for i in 0..token.len() {
            let mut bytes = token.clone().into_bytes();
            bytes[i] = if bytes[i] == b'0' { b'1' } else { b'0' };
            let changed = String::from_utf8(bytes).unwrap();
            let claimed = tokens.claimed(&changed).unwrap_or(ID);
            assert!(!tokens.check(&changed, claimed, HASH), "changed at {i}");
        }
        // Another key's (another run of the site), and malformed ones.
        assert!(!SignInTokens::new().check(&token, ID, HASH));
        for bad in [
            String::new(),
            token[1..].to_owned(),
            format!("{token}00"),
            "zz".repeat(TOKEN_BYTES),
        ] {
            assert_eq!(tokens.claimed(&bad), None);
            assert!(!tokens.check(&bad, ID, HASH));
        }
        // A later expiry with the old MAC.
        let later = tokens.issue_until(ID, HASH, u64::MAX);
        let forged = format!("{}{}", &later[..32], &token[32..]);
        assert!(!tokens.check(&forged, ID, HASH));
    }
}
