pub const REVS: &[&str] = &["225", "244", "245.2", "254"];

pub fn compile_target_revision() {
    assert!(!REVS.is_empty(), "REVS must list at least one revision");
    println!("cargo::rerun-if-env-changed=REV");

    let values = REVS
        .iter()
        .map(|r| format!("\"{r}\""))
        .collect::<Vec<_>>()
        .join(", ");
    println!("cargo::rustc-check-cfg=cfg(rev, values({values}))");
    for &r in &REVS[1..] {
        let id = since_ident(r);
        println!("cargo::rustc-check-cfg=cfg(since_{id})");
        println!("cargo::rustc-check-cfg=cfg(before_{id})");
    }

    let active = match std::env::var("REV") {
        Ok(s) => {
            let s = s.trim().to_string();
            assert!(
                REVS.contains(&s.as_str()),
                "REV = {s:?} is not a supported revision; supported: {REVS:?}"
            );
            s
        }
        Err(_) => REVS.last().unwrap().to_string(),
    };

    let active_idx = REVS
        .iter()
        .position(|&r| r == active.as_str())
        .expect("active revision is listed in REVS");

    println!("cargo::rustc-cfg=rev=\"{active}\"");
    for (i, &r) in REVS.iter().enumerate().skip(1) {
        if active_idx >= i {
            println!("cargo::rustc-cfg=since_{}", since_ident(r));
        } else {
            println!("cargo::rustc-cfg=before_{}", since_ident(r));
        }
    }
}

/// Maps a revision string to a valid cfg-identifier suffix (`"245.2"` -> `245_2`).
fn since_ident(rev: &str) -> String {
    rev.replace('.', "_")
}
