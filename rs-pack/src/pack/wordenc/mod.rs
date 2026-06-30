use std::path::Path;

use rs_io::Packet;
use rs_io::jag::{JagCompression, JagFile};
use tracing::debug;

fn read_lines(path: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .map(|l| l.trim_end().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

pub fn pack_wordenc(content_dir: &Path) -> Vec<u8> {
    let wordenc_dir = content_dir.join("wordenc");
    if !wordenc_dir.exists() {
        return Vec::new();
    }

    let order_path = wordenc_dir.join("meta").join("wordenc.order");
    let order: Vec<String> = std::fs::read_to_string(&order_path)
        .unwrap_or_else(|e| panic!("Missing wordenc order file {}: {e}", order_path.display()))
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    let mut jag = JagFile::new();
    for name in &order {
        let data = match name.as_str() {
            "badenc.txt" => encode_badenc(&wordenc_dir),
            "fragmentsenc.txt" => encode_fragmentsenc(&wordenc_dir),
            "tldlist.txt" => encode_tldlist(&wordenc_dir),
            "domainenc.txt" => encode_domainenc(&wordenc_dir),
            _ => panic!("Unexpected wordenc file in wordenc.order: {name}"),
        };
        jag.write(name, data);
    }

    debug!("Packed wordenc into Jag ({} entries)", order.len());
    jag.build(JagCompression::PerFile)
}

fn encode_badenc(wordenc_dir: &Path) -> Vec<u8> {
    let lines = read_lines(&wordenc_dir.join("badenc.txt"));
    let mut out = Packet::new(lines.len() * 32 + 16);
    out.p4(lines.len() as i32);
    for line in &lines {
        let parts: Vec<&str> = line.split(' ').collect();
        let word = parts[0];
        let combinations = &parts[1..];

        out.p1(word.len() as u8);
        for ch in word.bytes() {
            out.p1(ch);
        }

        out.p1(combinations.len() as u8);
        for combo in combinations {
            let ab: Vec<&str> = combo.split(':').collect();
            if ab.len() == 2 {
                out.p1(ab[0].parse::<u8>().unwrap_or(0));
                out.p1(ab[1].parse::<u8>().unwrap_or(0));
            }
        }
    }
    out.data[..out.pos].to_vec()
}

fn encode_fragmentsenc(wordenc_dir: &Path) -> Vec<u8> {
    let lines = read_lines(&wordenc_dir.join("fragmentsenc.txt"));
    let mut out = Packet::new(lines.len() * 4 + 16);
    out.p4(lines.len() as i32);
    for line in &lines {
        let fragment: u16 = line.parse().unwrap_or(0);
        out.p2(fragment);
    }
    out.data[..out.pos].to_vec()
}

fn encode_tldlist(wordenc_dir: &Path) -> Vec<u8> {
    let lines = read_lines(&wordenc_dir.join("tldlist.txt"));
    let mut out = Packet::new(lines.len() * 32 + 16);
    out.p4(lines.len() as i32);
    for line in &lines {
        let parts: Vec<&str> = line.split(' ').collect();
        if parts.len() >= 2 {
            let tld = parts[0];
            let tld_type: u8 = parts[1].parse().unwrap_or(0);

            out.p1(tld_type);
            out.p1(tld.len() as u8);
            for ch in tld.bytes() {
                out.p1(ch);
            }
        }
    }
    out.data[..out.pos].to_vec()
}

fn encode_domainenc(wordenc_dir: &Path) -> Vec<u8> {
    let lines = read_lines(&wordenc_dir.join("domainenc.txt"));
    let mut out = Packet::new(lines.len() * 64 + 16);
    out.p4(lines.len() as i32);
    for line in &lines {
        out.p1(line.len() as u8);
        for ch in line.bytes() {
            out.p1(ch);
        }
    }
    out.data[..out.pos].to_vec()
}
