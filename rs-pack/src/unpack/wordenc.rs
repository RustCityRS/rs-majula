use std::path::Path;

use rs_io::Packet;
use rs_io::jag::JagFile;
use tracing::debug;

const WORDENC_NAMES: &[&str] = &[
    "badenc.txt",
    "fragmentsenc.txt",
    "tldlist.txt",
    "domainenc.txt",
];

pub(crate) fn known_hashes() -> Vec<i32> {
    WORDENC_NAMES.iter().map(|n| JagFile::hash(n)).collect()
}

fn find_wordenc_name(hash: i32) -> Option<&'static str> {
    WORDENC_NAMES
        .iter()
        .copied()
        .find(|n| JagFile::hash(n) == hash)
}

pub fn unpack_wordenc(jag: &JagFile, output_dir: &Path) -> anyhow::Result<()> {
    let wordenc_dir = output_dir.join("wordenc");
    let meta_dir = wordenc_dir.join("meta");
    std::fs::create_dir_all(&meta_dir)?;

    if let Some(data) = jag.read("badenc.txt") {
        let text = decode_badenc(&data.data);
        std::fs::write(wordenc_dir.join("badenc.txt"), text)?;
    }

    if let Some(data) = jag.read("fragmentsenc.txt") {
        let text = decode_fragmentsenc(&data.data);
        std::fs::write(wordenc_dir.join("fragmentsenc.txt"), text)?;
    }

    if let Some(data) = jag.read("tldlist.txt") {
        let text = decode_tldlist(&data.data);
        std::fs::write(wordenc_dir.join("tldlist.txt"), text)?;
    }

    if let Some(data) = jag.read("domainenc.txt") {
        let text = decode_domainenc(&data.data);
        std::fs::write(wordenc_dir.join("domainenc.txt"), text)?;
    }

    let mut order = Vec::new();
    for i in 0..jag.file_count {
        if let Some(name) = find_wordenc_name(jag.file_hash(i)) {
            order.push(name.to_string());
        }
    }
    let order_content = order.join("\n") + "\n";
    std::fs::write(meta_dir.join("wordenc.order"), order_content)?;

    debug!("Unpacked wordenc ({} entries)", order.len());
    Ok(())
}

fn decode_badenc(data: &[u8]) -> String {
    let mut buf = Packet::from(data.to_vec());
    let count = buf.g4s() as usize;
    let mut lines = Vec::with_capacity(count);

    for _ in 0..count {
        let word_len = buf.g1() as usize;
        let mut word = String::with_capacity(word_len);
        for _ in 0..word_len {
            word.push(buf.g1() as char);
        }

        let combo_count = buf.g1() as usize;
        let mut parts = vec![word];
        for _ in 0..combo_count {
            let a = buf.g1();
            let b = buf.g1();
            parts.push(format!("{a}:{b}"));
        }

        lines.push(parts.join(" "));
    }

    lines.join("\n") + "\n"
}

fn decode_fragmentsenc(data: &[u8]) -> String {
    let mut buf = Packet::from(data.to_vec());
    let count = buf.g4s() as usize;
    let mut lines = Vec::with_capacity(count);

    for _ in 0..count {
        let fragment = buf.g2();
        lines.push(fragment.to_string());
    }

    lines.join("\n") + "\n"
}

fn decode_tldlist(data: &[u8]) -> String {
    let mut buf = Packet::from(data.to_vec());
    let count = buf.g4s() as usize;
    let mut lines = Vec::with_capacity(count);

    for _ in 0..count {
        let tld_type = buf.g1();
        let tld_len = buf.g1() as usize;
        let mut tld = String::with_capacity(tld_len);
        for _ in 0..tld_len {
            tld.push(buf.g1() as char);
        }
        lines.push(format!("{tld} {tld_type}"));
    }

    lines.join("\n") + "\n"
}

fn decode_domainenc(data: &[u8]) -> String {
    let mut buf = Packet::from(data.to_vec());
    let count = buf.g4s() as usize;
    let mut lines = Vec::with_capacity(count);

    for _ in 0..count {
        let domain_len = buf.g1() as usize;
        let mut domain = String::with_capacity(domain_len);
        for _ in 0..domain_len {
            domain.push(buf.g1() as char);
        }
        lines.push(domain);
    }

    lines.join("\n") + "\n"
}
