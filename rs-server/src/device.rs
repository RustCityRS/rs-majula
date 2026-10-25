use rs_db::DeviceCheck;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::str;
use std::sync::OnceLock;

/// How many values device.js sends: the fingerprint, the engine, the
/// platform, the graphics card, the user agent's hash, and the seal.
const VALUES: usize = 6;

/// This deploy's device check.
pub struct Setup {
    key: String,
    names: [String; VALUES],
    pub source: String,
}

static SETUP: OnceLock<Setup> = OnceLock::new();

/// Makes this deploy's device check from the world's login key file. Called
/// once at startup, before the client page is served.
pub fn init(login_key: &[u8]) {
    let _ = SETUP.set(Setup::new(login_key));
}

/// This deploy's device check (an unkeyed one if [`init`] wasn't called, as
/// in tests).
pub fn setup() -> &'static Setup {
    SETUP.get_or_init(|| Setup::new(b""))
}

/// device.js's `hash`: 64 bits of `text` (its UTF-16 code units), as 16 hex
/// digits. Not cryptographic, and needn't be: it has to match the page's.
pub fn hash64(text: &str) -> String {
    let mut h1: u32 = 0xdead_beef;
    let mut h2: u32 = 0x41c6_ce57;
    for c in text.encode_utf16() {
        let c = u32::from(c);
        h1 = (h1 ^ c).wrapping_mul(2_654_435_761);
        h2 = (h2 ^ c).wrapping_mul(1_597_334_677);
    }
    h1 = (h1 ^ (h1 >> 16)).wrapping_mul(2_246_822_507)
        ^ (h2 ^ (h2 >> 13)).wrapping_mul(3_266_489_909);
    h2 = (h2 ^ (h2 >> 16)).wrapping_mul(2_246_822_507)
        ^ (h1 ^ (h1 >> 13)).wrapping_mul(3_266_489_909);
    format!("{h2:08x}{h1:08x}")
}

impl Setup {
    fn new(login_key: &[u8]) -> Self {
        // The first 64 bits of SHA-256 over the login key file: the page gets
        // these, never anything that could lead back to the key.
        let mut hasher = Sha256::new();
        hasher.update(
            b"device-check
",
        );
        hasher.update(login_key);
        let key: String = hasher.finalize()[..8]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        // Three letters each, from the key, all different.
        let mut names: Vec<String> = Vec::with_capacity(VALUES);
        let mut n = 0;
        while names.len() < VALUES {
            let name: String = hash64(&format!("{key}|name|{n}"))
                .bytes()
                .take(3)
                .map(|b| (b'a' + (b as char).to_digit(16).unwrap_or(0) as u8) as char)
                .collect();
            n += 1;
            if !names.contains(&name) {
                names.push(name);
            }
        }
        let source = include_str!("device.js")
            .replace("__DEVICE_KEY__", &key)
            .replace("__DEVICE_NAMES__", &names.join(","));
        Self {
            key,
            names: names.try_into().unwrap_or_else(|_| unreachable!()),
            source,
        }
    }
}

/// What a web client's device check said, as the world takes it.
#[derive(Debug, PartialEq)]
pub struct Measured {
    pub check: DeviceCheck,
    /// Only from a sealed check: an edited one's could name anyone's device.
    pub fingerprint: Option<i64>,
    pub engine: Option<String>,
    pub js_platform: Option<String>,
    pub gpu: Option<String>,
    /// The page's `navigator.userAgent` isn't the one the browser sent.
    pub agent_differs: bool,
}

/// Percent-decodes a query value (`+` is a space), `None` if it isn't UTF-8.
fn decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hex = str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                i += 2;
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8(out).ok()
}

/// Printable ASCII only, and never more than `max` of it.
fn printable(value: &str, max: usize) -> Option<String> {
    let value: String = value
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(max)
        .collect();
    (!value.is_empty()).then_some(value)
}

/// Reads the device check off a handshake's `query` (the game connection's
/// address), for a browser that sent `user_agent` in its headers.
pub fn read(query: Option<&str>, user_agent: Option<&str>) -> Measured {
    let setup = setup();
    let params: HashMap<&str, &str> = query
        .unwrap_or("")
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .collect();
    let missing = Measured {
        check: DeviceCheck::Missing,
        fingerprint: None,
        engine: None,
        js_platform: None,
        gpu: None,
        agent_differs: false,
    };
    let values: Option<Vec<String>> = setup
        .names
        .iter()
        .map(|name| params.get(name.as_str()).and_then(|v| decode(v)))
        .collect();
    let Some(values) = values else {
        // None at all is a missing check; some, a broken one.
        return if setup.names.iter().any(|n| params.contains_key(n.as_str())) {
            Measured {
                check: DeviceCheck::Broken,
                ..missing
            }
        } else {
            missing
        };
    };
    let [fingerprint, engine, platform, gpu, agent, seal] = &values[..] else {
        return missing;
    };
    let sealed = hash64(
        &[&setup.key, fingerprint, engine, platform, gpu, agent]
            .map(String::as_str)
            .join("|"),
    ) == *seal;
    let bits = (fingerprint.len() == 16)
        .then(|| u64::from_str_radix(fingerprint, 16).ok())
        .flatten();
    if !sealed || bits.is_none() {
        return Measured {
            check: DeviceCheck::Broken,
            ..missing
        };
    }
    let sent_agent = hash64(user_agent.unwrap_or(""));
    Measured {
        check: DeviceCheck::Sealed,
        fingerprint: bits.filter(|&b| b != 0).map(|b| b as i64),
        engine: Some(
            match engine.as_str() {
                "blink" | "gecko" | "webkit" => engine.as_str(),
                _ => "other",
            }
            .to_owned(),
        ),
        js_platform: printable(platform, 32),
        gpu: printable(gpu, 96),
        agent_differs: !sent_agent.starts_with(agent.as_str()) || agent.len() != 8,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A handshake query as device.js makes it for this setup.
    pub(crate) fn query(values: [&str; 5]) -> String {
        let setup = setup();
        let mut all: Vec<String> = values.iter().map(|v| v.to_string()).collect();
        let seal = hash64(&format!("{}|{}", setup.key, all.join("|")));
        all.push(seal);
        setup
            .names
            .iter()
            .zip(all)
            .map(|(name, value)| format!("{name}={}", value.replace(' ', "+")))
            .collect::<Vec<_>>()
            .join("&")
    }

    #[test]
    fn hashes_match_the_page() {
        // Worked out by device.js's hash in node.
        assert_eq!(hash64(""), "488bdcb81aee8d83");
        assert_eq!(hash64("RuneScape"), "a3ddaa32ad49ea55");
        assert_eq!(
            hash64("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/141.0"),
            "e6259573b503fb04"
        );
        assert_eq!(hash64("drag\u{1F409}n"), "e74356089d783539");
    }

    #[test]
    fn every_deploy_has_its_own_key_and_names() {
        let a = Setup::new(b"-----BEGIN PRIVATE KEY-----\nAAA\n");
        let b = Setup::new(b"-----BEGIN PRIVATE KEY-----\nBBB\n");
        assert_ne!(a.key, b.key);
        assert_ne!(a.names, b.names);
        for setup in [&a, &b] {
            assert!(setup.names.iter().all(|n| n.len() == 3));
            assert!(!setup.source.contains("__DEVICE_"));
            assert!(setup.source.contains(&setup.key));
            assert!(setup.source.contains(&setup.names.join(",")));
        }
    }

    #[test]
    fn a_sealed_check_is_taken_and_an_edited_one_isnt() {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/141.0";
        let agent = &hash64(ua)[..8];
        let gpu = "ANGLE (NVIDIA, GeForce RTX 3070 Direct3D11)";
        let good = query(["ffffffffffffffd6", "blink", "Win32", gpu, agent]);
        let read_good = read(Some(&good), Some(ua));
        assert_eq!(read_good.check, DeviceCheck::Sealed);
        assert_eq!(read_good.fingerprint, Some(-42));
        assert_eq!(read_good.gpu.as_deref(), Some(gpu));
        assert!(!read_good.agent_differs);
        // Another user agent in the headers than the page saw.
        assert!(read(Some(&good), Some("Mozilla/5.0 (Macintosh) Firefox/131.0")).agent_differs);
        // A value changed after sealing: nothing it says is kept.
        let edited = good.replace("Win32", "MacIntel");
        let broken = read(Some(&edited), Some(ua));
        assert_eq!(
            (broken.check, broken.fingerprint),
            (DeviceCheck::Broken, None)
        );
        // A value left out.
        let name = &setup().names[2];
        let partial: String = good
            .split('&')
            .filter(|p| !p.starts_with(&format!("{name}=")))
            .collect::<Vec<_>>()
            .join("&");
        assert_eq!(read(Some(&partial), Some(ua)).check, DeviceCheck::Broken);
        // None at all.
        assert_eq!(read(None, Some(ua)).check, DeviceCheck::Missing);
        assert_eq!(read(Some("x=1"), Some(ua)).check, DeviceCheck::Missing);
    }

    #[test]
    fn query_values_are_decoded() {
        assert_eq!(
            decode("ANGLE+%28Intel%29").as_deref(),
            Some("ANGLE (Intel)")
        );
        assert_eq!(decode("%E2%9C%93").as_deref(), Some("\u{2713}"));
        assert_eq!(decode("50%"), Some("50%".into()));
        assert_eq!(decode("%FF"), None);
    }
}
