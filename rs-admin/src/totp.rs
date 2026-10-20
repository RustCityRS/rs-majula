use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;
use std::time::{SystemTime, UNIX_EPOCH};

const STEP_SECS: u64 = 30;
const DIGITS: u32 = 6;

/// Bytes of a new secret: 160 bits, as RFC 4226 recommends.
pub const SECRET_LEN: usize = 20;

/// The time step `unix_secs` falls in.
pub fn step_at(unix_secs: u64) -> u64 {
    unix_secs / STEP_SECS
}

pub fn current_step() -> u64 {
    step_at(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    )
}

/// The code for `secret` at time step `step`.
pub fn code(secret: &[u8], step: u64) -> u32 {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(&step.to_be_bytes());
    let hash = mac.finalize().into_bytes();
    let offset = (hash[hash.len() - 1] & 0xf) as usize;
    let bin = u32::from_be_bytes([
        hash[offset] & 0x7f,
        hash[offset + 1],
        hash[offset + 2],
        hash[offset + 3],
    ]);
    bin % 10u32.pow(DIGITS)
}

/// The step `input` is the code of, allowing one step of clock drift either
/// way. `None` if it matches none of them.
pub fn matching_step(secret: &[u8], input: &str, now: u64) -> Option<u64> {
    let input = input.trim().replace(' ', "");
    if input.len() != DIGITS as usize || !input.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let input: u32 = input.parse().ok()?;
    [now.saturating_sub(1), now, now + 1]
        .into_iter()
        .find(|&step| constant_time_eq(code(secret, step), input))
}

fn constant_time_eq(a: u32, b: u32) -> bool {
    (a ^ b) == 0
}

/// The `otpauth://` URI an authenticator app scans to add the account.
pub fn provisioning_uri(issuer: &str, account: &str, secret: &[u8]) -> String {
    let issuer = percent_encode(issuer);
    format!(
        "otpauth://totp/{issuer}:{}?secret={}&issuer={issuer}&algorithm=SHA1&digits={DIGITS}&period={STEP_SECS}",
        percent_encode(account),
        base32(secret)
    )
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// RFC 4648 base32 without padding, as authenticator apps take secrets.
pub fn base32(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(5) * 8);
    let (mut buffer, mut bits) = (0u32, 0);
    for &b in bytes {
        buffer = (buffer << 8) | u32::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(BASE32[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(BASE32[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

pub fn unbase32(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 5 / 8);
    let (mut buffer, mut bits) = (0u32, 0);
    for c in s.bytes().filter(|&c| c != b'=' && c != b' ') {
        let value = BASE32.iter().position(|&d| d == c.to_ascii_uppercase())? as u32;
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238 appendix B's SHA-1 vectors, cut to six digits.
    #[test]
    fn rfc_6238_vectors() {
        let secret = b"12345678901234567890";
        for (time, expected) in [
            (59, 94_287_082),
            (1_111_111_109, 7_081_804),
            (1_111_111_111, 14_050_471),
            (1_234_567_890, 89_005_924),
            (2_000_000_000, 69_279_037),
            (20_000_000_000, 65_353_130),
        ] {
            assert_eq!(
                code(secret, step_at(time)),
                expected % 1_000_000,
                "t={time}"
            );
        }
    }

    #[test]
    fn codes_match_within_one_step() {
        let secret = b"12345678901234567890";
        let step = step_at(59);
        let code = format!("{:06}", code(secret, step));
        assert_eq!(matching_step(secret, &code, step), Some(step));
        assert_eq!(matching_step(secret, &code, step + 1), Some(step));
        assert_eq!(matching_step(secret, &code, step - 1), Some(step));
        assert_eq!(matching_step(secret, &code, step + 2), None);
        assert_eq!(matching_step(secret, "12345", step), None);
        assert_eq!(matching_step(secret, "abcdef", step), None);
    }

    #[test]
    fn base32_round_trips() {
        // RFC 4648 section 10, unpadded.
        for (raw, encoded) in [
            ("", ""),
            ("f", "MY"),
            ("fo", "MZXQ"),
            ("foo", "MZXW6"),
            ("foob", "MZXW6YQ"),
            ("fooba", "MZXW6YTB"),
            ("foobar", "MZXW6YTBOI"),
        ] {
            assert_eq!(base32(raw.as_bytes()), encoded);
            assert_eq!(unbase32(encoded).unwrap(), raw.as_bytes());
        }
        assert_eq!(unbase32("mzxw6ytboi").unwrap(), b"foobar");
        assert_eq!(unbase32("MZXW1"), None);
    }

    #[test]
    fn uri_escapes_names() {
        let uri = provisioning_uri("RS Admin", "Mod Ash", b"foobar");
        assert_eq!(
            uri,
            "otpauth://totp/RS%20Admin:Mod%20Ash?secret=MZXW6YTBOI&issuer=RS%20Admin&algorithm=SHA1&digits=6&period=30"
        );
    }
}
