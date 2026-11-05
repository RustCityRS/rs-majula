//! No 647 quest scroll carries the 2004 client's line break.
//!
//! 289 breaks its quest scroll message with `\\n` ("You have completed
//! the\\nImp Catcher Quest!"), which the 2004 client's font splits on. The
//! 2011 client's breaks on `<br>` and draws `\n` as it stands, so a quest
//! ported line for line shows the escape on its scroll. Every 289 quest's
//! completion has one, so this checks each `~send_quest_complete` call in
//! `content/647`.
//!
//! ```text
//! cargo test -p rs-pack --test quest_scroll_text_647
//! ```

#![cfg(since_647)]

use std::fs;
use std::path::{Path, PathBuf};

fn rs2_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable content dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            rs2_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs2") {
            out.push(path);
        }
    }
}

#[test]
fn quest_scrolls_break_lines_with_br() {
    let scripts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647/scripts");
    let mut files = Vec::new();
    rs2_files(&scripts, &mut files);
    let mut calls = 0;
    let mut escaped = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).expect("utf-8 script");
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if !code.contains("~send_quest_complete(") {
                continue;
            }
            calls += 1;
            if code.contains("\\n") {
                escaped.push(format!("{}:{}", file.display(), n + 1));
            }
        }
    }
    assert!(calls > 0, "no ~send_quest_complete call found");
    assert!(
        escaped.is_empty(),
        "quest scrolls with 289's \\n (use <br>):\n  {}",
        escaped.join("\n  ")
    );
}
