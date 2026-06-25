pub const REVS: &[u32] = &[225, 244];

pub fn compile_target_revision() {
    assert!(!REVS.is_empty(), "REVS must list at least one revision");
    println!("cargo::rerun-if-env-changed=REV");

    let values = REVS
        .iter()
        .map(|r| format!("\"{r}\""))
        .collect::<Vec<_>>()
        .join(", ");
    println!("cargo::rustc-check-cfg=cfg(rev, values({values}))");
    for r in &REVS[1..] {
        println!("cargo::rustc-check-cfg=cfg(since_{r})");
    }

    let active = match std::env::var("REV") {
        Ok(s) => {
            let n: u32 = s
                .trim()
                .parse()
                .unwrap_or_else(|_| panic!("REV = {s:?} is not a number; supported: {REVS:?}"));
            assert!(
                REVS.contains(&n),
                "REV = {n} is not a supported revision; supported: {REVS:?}"
            );
            n
        }
        Err(_) => *REVS.last().unwrap(),
    };

    println!("cargo::rustc-cfg=rev=\"{active}\"");
    for &r in &REVS[1..] {
        if active >= r {
            println!("cargo::rustc-cfg=since_{r}");
        }
    }
}
