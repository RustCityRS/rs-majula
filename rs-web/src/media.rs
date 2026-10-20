//! Images and videos uploaded for news posts. Each is stored once, named by
//! its content's hash, and served under `/news/media/` for good: a name
//! never changes what it points at, so browsers and Cloudflare can cache it
//! forever.

use sha2::{Digest, Sha256};
use std::io;
use std::path::Path;

/// A kind of file posts can embed, told by its first bytes, never by its
/// name or the uploader's say-so.
pub struct Media {
    pub extension: &'static str,
    /// `image` or `video`.
    pub kind: &'static str,
}

/// What `bytes` are, if they're an image or video posts can embed. SVG is
/// left out on purpose: it can carry script.
pub fn sniff(bytes: &[u8]) -> Option<Media> {
    let media = |extension, kind| Some(Media { extension, kind });
    match bytes {
        [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, ..] => media("png", "image"),
        [0xff, 0xd8, 0xff, ..] => media("jpg", "image"),
        [b'G', b'I', b'F', b'8', b'7' | b'9', b'a', ..] => media("gif", "image"),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => media("webp", "image"),
        [_, _, _, _, b'f', b't', b'y', b'p', ..] => media("mp4", "video"),
        [0x1a, 0x45, 0xdf, 0xa3, ..] => media("webm", "video"),
        _ => None,
    }
}

/// Whether `name` could be one of ours (and so is safe to look up).
pub fn valid_name(name: &str) -> bool {
    let Some((hash, extension)) = name.split_once('.') else {
        return false;
    };
    hash.len() == 32
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && ["png", "jpg", "gif", "webp", "mp4", "webm"].contains(&extension)
}

/// Stores `bytes` (already sniffed as `media`) in `dir` and returns its name.
/// Storing the same file twice keeps the one copy.
pub fn store(dir: &Path, bytes: &[u8], media: &Media) -> io::Result<String> {
    let hash = Sha256::digest(bytes);
    let hex: String = hash[..16].iter().map(|b| format!("{b:02x}")).collect();
    let name = format!("{hex}.{}", media.extension);
    let path = dir.join(&name);
    if path.exists() {
        return Ok(name);
    }
    std::fs::create_dir_all(dir)?;
    // Written aside then renamed into place, so the site never serves half
    // a file.
    let partial = dir.join(format!(".{name}.partial"));
    std::fs::write(&partial, bytes)?;
    std::fs::rename(&partial, &path)?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_by_content() {
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n....").unwrap().extension, "png");
        assert_eq!(sniff(b"\xff\xd8\xff\xe0").unwrap().extension, "jpg");
        assert_eq!(sniff(b"GIF89a...").unwrap().extension, "gif");
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 ").unwrap().extension, "webp");
        assert_eq!(sniff(b"\0\0\0\x18ftypmp42").unwrap().kind, "video");
        assert_eq!(sniff(b"\x1a\x45\xdf\xa3....").unwrap().extension, "webm");
        assert!(sniff(b"<svg xmlns=").is_none());
        assert!(sniff(b"<html>").is_none());
        assert!(sniff(b"").is_none());
    }

    #[test]
    fn names_are_checked() {
        assert!(valid_name("0123456789abcdef0123456789abcdef.png"));
        assert!(!valid_name("../../etc/passwd"));
        assert!(!valid_name("0123456789abcdef0123456789abcdef.svg"));
        assert!(!valid_name("0123456789ABCDEF0123456789abcdef.png"));
        assert!(!valid_name("short.png"));
    }

    #[test]
    fn stores_once_by_content() {
        let dir = std::env::temp_dir().join(format!("rs-web-media-test-{}", std::process::id()));
        let bytes = b"GIF89a-test-bytes";
        let media = sniff(bytes).unwrap();
        let a = store(&dir, bytes, &media).unwrap();
        let b = store(&dir, bytes, &media).unwrap();
        assert_eq!(a, b);
        assert!(valid_name(&a));
        assert_eq!(std::fs::read(dir.join(&a)).unwrap(), bytes);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
