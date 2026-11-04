//! CRC gate — the assembled rev-647 cache against OpenRS2 cache 786, group by
//! group.
//!
//! Ground truth is the archive's API, not a copy in git. Each `(255, a)`
//! reference table is fetched from
//! `https://archive.openrs2.org/caches/runescape/786/archives/255/groups/<a>.dat`
//! and memoised under `CARGO_TARGET_TMPDIR`, so only the first run needs the
//! network. The API omits the 2-byte version trailer: the CRC of what it
//! serves is the login CRC the client computes.
//!
//! Every archive, three checks:
//!
//! 1. **Groups.** Each group's CRC and version is compared, and the set of
//!    groups that differ must *equal* the archive's entry in [`KNOWN`]. A
//!    group that starts to match fails the gate as surely as one that stops,
//!    so the list cannot rot into a blanket skip.
//! 2. **Container.** With every group accounted for, the table container's
//!    CRC must match too, unless [`KNOWN`] says the table alone is encoded
//!    differently around identical groups.
//! 3. **Keys.** Every committed XTEA key decrypts its landscape group *in our
//!    cache* -- including the squares rebuilt from donor caches, whose keys
//!    OpenRS2 does not have.
//!
//! An unexplained group is fetched from the API and both copies are written
//! to `target/crc_gate/<archive>/<group>.{ours,openrs2}.dat` for diffing.
//!
//! Runs the full pack pipeline, so it needs the `content/647` tree and is
//! `#[ignore]`d by default:
//! `REV=647 cargo test -p rs-pack --release --test crc_gate -- --ignored`
#![cfg(rev = "647")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rs_pack::openrs2;

const NAMES: [&str; 35] = [
    "anims",
    "bases",
    "config",
    "interfaces",
    "synths",
    "maps",
    "music",
    "models",
    "sprites",
    "textures",
    "binary",
    "jingles",
    "clientscripts",
    "fontmetrics",
    "vorbis",
    "instruments",
    "config_loc",
    "config_enum",
    "config_npc",
    "config_obj",
    "config_seq",
    "config_spot",
    "config_varbit",
    "worldmap",
    "quickchat",
    "quickchat_global",
    "materials",
    "config_particle",
    "defaults",
    "config_billboard",
    "dlls",
    "shaders",
    "loading_sprites",
    "loading_screens",
    "worldmap_locs",
];

/// A known, explained divergence of one archive from OpenRS2 786.
struct Known {
    archive: u8,
    /// Groups whose CRC, version or presence differs. Must be exact, sorted.
    groups: &'static [u32],
    /// The table container differs even with every group accounted for.
    table_only: bool,
    #[allow(dead_code)] // read by people, not code
    why: &'static str,
}

/// Measured 2026-10-31 against OpenRS2 786. Thirty archives match outright.
const KNOWN: &[Known] = &[
    Known {
        archive: 5,
        // The 29 landscape squares whose 786 keys nobody has (OpenRS2 lists
        // none), rebuilt from donor caches and re-encrypted with the committed
        // key, plus the terrain of 4 of them: m51_53, m33_76, m48_50, m42_58.
        groups: &[
            80, 81, 198, 199, 616, 617, 978, 979, 1493, 1575, 1907, 1909, 1943, 1953, 2145, 3444,
            3592, 3812, 3816, 3818, 3822, 3826, 3835, 3865, 3890, 4005, 4037, 4041, 4045, 4074,
            5040, 5158, 5206,
        ],
        table_only: false,
        why: "donor-restored landscapes: 786's copies are unreadable without keys",
    },
    Known {
        archive: 7,
        // All 197 are groups 786 does not have at all; no 786 model is
        // altered. Models the server overlay configs reference that Jagex did
        // not ship in this cache.
        groups: &[
            526, 535, 597, 838, 854, 856, 858, 861, 864, 871, 872, 873, 899, 902, 909, 927, 929,
            930, 932, 933, 938, 945, 946, 971, 1018, 1019, 1020, 1021, 1028, 1082, 1094, 1102,
            1109, 1143, 1145, 1378, 1764, 1771, 1847, 1848, 1989, 1990, 1991, 2007, 2009, 2025,
            2026, 2031, 2038, 2041, 2060, 2067, 2078, 2079, 2083, 2101, 2104, 2117, 2168, 2169,
            2170, 2179, 2181, 2198, 2220, 2222, 2236, 2238, 2253, 2254, 2256, 2263, 2264, 2265,
            2266, 2289, 2290, 2294, 2324, 2325, 2327, 2328, 2332, 2359, 2361, 2371, 2374, 2380,
            2392, 2396, 2461, 2471, 2497, 2546, 2611, 2618, 48609, 48610, 48611, 48612, 48613,
            48615, 48616, 48617, 48618, 48619, 48621, 48626, 48627, 48628, 48630, 48634, 48637,
            48638, 48639, 48641, 48642, 48643, 48644, 48646, 48650, 48651, 48652, 48654, 48655,
            48657, 48663, 48666, 48668, 48669, 48670, 48671, 48672, 48673, 48674, 48675, 48676,
            48677, 48678, 48679, 48682, 48684, 48685, 48686, 48687, 48689, 48690, 48691, 48692,
            48693, 48694, 48695, 48696, 48697, 48698, 48699, 48702, 48703, 48705, 48706, 48707,
            48708, 48709, 48711, 48712, 48713, 48714, 48716, 48717, 48718, 48720, 48721, 51104,
            51105, 51106, 51107, 51108, 51109, 51111, 51112, 51113, 51114, 51115, 51116, 51117,
            51118, 51119, 51120, 51121, 51122, 51123, 51124, 51125, 51126, 51127, 51128, 51129,
        ],
        table_only: false,
        why: "models added for server overlay configs, absent from 786",
    },
    Known {
        archive: 12,
        groups: &[],
        table_only: true,
        why: "every clientscript group matches; only the table container's compression differs",
    },
    Known {
        archive: 16,
        // 179 left this list on 2026-10-31: its eight `border_guard` locs had
        // been dropped from the content for no recorded reason, and restoring
        // them from 786 made the group match again.
        groups: &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 17, 18, 19, 20, 21, 35, 36, 58, 65, 67,
            74, 100, 105, 137, 138, 142, 146, 147, 150, 151, 163, 165, 167, 168, 172, 183, 184,
            191, 205, 223,
        ],
        table_only: false,
        why: "loc config groups restored from donor caches, plus the woodcutting tree overlays",
    },
];

fn api_dir() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join("openrs2-api")
}

#[test]
#[ignore = "runs the full pack pipeline against content/647 (minutes), and the OpenRS2 API"]
fn assembled_cache_matches_openrs2_group_by_group() {
    // Integration tests run with cwd = rs-pack/; the pipeline's paths
    // (CONTENT_DIR, target/pack/<REV>) are workspace-root relative.
    if !Path::new(rs_pack::CONTENT_DIR).exists() {
        std::env::set_current_dir("..").expect("chdir to workspace root");
    }
    let id = openrs2::default_cache_id().unwrap();
    let source = Path::new(rs_pack::CONTENT_DIR);
    let pack = Path::new(rs_pack::PACK_DIR);

    // verify=false: compare here and report every archive, rather than
    // panicking inside the pipeline on the first divergence.
    let result = rs_pack::pack_all(source, pack, false, true).expect("pack_all failed");
    let mut store = rs_dat2::CacheStore::open(&result.cache_dir).expect("open built cache");

    let known: BTreeMap<u8, &Known> = KNOWN.iter().map(|k| (k.archive, k)).collect();
    let mut failures = Vec::new();
    let mut unexplained: Vec<(u8, u32)> = Vec::new();

    println!(
        "\n{:<18} {:>12} {:>12}  groups differing",
        "Archive", "OpenRS2", "Ours"
    );
    println!("{}", "-".repeat(64));
    for (pos, &aid) in rs_pack::startup::MASTER_INDEX_ORDER.iter().enumerate() {
        let theirs_raw = openrs2::fetch_group_cached(&api_dir(), id, 255, aid as u32)
            .unwrap_or_else(|e| panic!("fetching (255, {aid}) from OpenRS2: {e:#}"));
        let ours_raw = store
            .read_raw_group_bytes(255, aid as u32)
            .unwrap()
            .unwrap_or_else(|| panic!("our cache has no reference table for archive {aid}"));
        let theirs_crc = rs_dat2::compute_group_crc(&theirs_raw);
        let ours_crc = rs_dat2::compute_group_crc(&ours_raw);

        let diff = openrs2::diff_reference_tables(
            &openrs2::parse_reference_container(&ours_raw).unwrap(),
            &openrs2::parse_reference_container(&theirs_raw).unwrap(),
        );
        let differing = diff.groups();
        let status = match (differing.is_empty(), ours_crc == theirs_crc) {
            (true, true) => "none".to_string(),
            (true, false) => "none (table only)".to_string(),
            (false, _) => format!("{} ({})", differing.len(), summarise(&diff)),
        };
        println!(
            "{:<18} {:>#12x} {:>#12x}  {status}",
            NAMES[pos], theirs_crc, ours_crc
        );
        if !differing.is_empty() {
            println!("    {differing:?}");
        }

        match judge(&differing, ours_crc == theirs_crc, known.get(&aid).copied()) {
            Verdict::Explained => {}
            Verdict::Groups { new, fixed } => {
                failures.push(format!(
                    "{} (archive {aid}): {} unexplained group(s) {new:?}; {} listed in KNOWN now match {fixed:?}",
                    NAMES[pos],
                    new.len(),
                    fixed.len(),
                ));
                unexplained.extend(new.into_iter().map(|g| (aid, g)));
            }
            Verdict::Container => failures.push(format!(
                "{} (archive {aid}): every group is accounted for but the table container                  differs (ours {ours_crc:#010x}, OpenRS2 {theirs_crc:#010x}){}",
                NAMES[pos],
                if diff.header { " -- the header differs" } else { "" },
            )),
            Verdict::StaleTableOnly => failures.push(format!(
                "{} (archive {aid}): the table container now matches; drop `table_only`",
                NAMES[pos]
            )),
        }
    }

    for (aid, gid) in unexplained.iter().take(50) {
        dump_group(&mut store, id, *aid, *gid);
    }

    assert!(
        failures.is_empty(),
        "{} archive(s) diverge from OpenRS2 {id} beyond KNOWN:\n  {}",
        failures.len(),
        failures.join("\n  "),
    );

    // 3. Every committed key opens its landscape group in our cache.
    let keys = std::fs::read_to_string(source.join("keys.json")).unwrap();
    let keys = openrs2::parse_keys(&keys).unwrap();
    let bad: Vec<_> = keys
        .iter()
        .filter(|k| {
            !matches!(
                store.read_group_xtea(k.archive as u8, k.group, &k.key),
                Ok(Some(ref d)) if !d.is_empty()
            )
        })
        .map(|k| (k.archive, k.group))
        .collect();
    assert!(
        bad.is_empty(),
        "{} of {} committed keys do not open our cache: {bad:?}",
        bad.len(),
        keys.len()
    );
}

/// The gate's ruling on one archive.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Matches OpenRS2, or differs exactly as [`KNOWN`] says.
    Explained,
    /// The differing groups are not the known set: `new` differ unexplained,
    /// `fixed` are listed in [`KNOWN`] but match now.
    Groups { new: Vec<u32>, fixed: Vec<u32> },
    /// Every group agrees, yet the container differs and is not `table_only`.
    Container,
    /// [`KNOWN`] says `table_only`, but the container matches now.
    StaleTableOnly,
}

fn judge(differing: &[u32], container_matches: bool, known: Option<&Known>) -> Verdict {
    let expected: &[u32] = known.map_or(&[], |k| k.groups);
    let table_only = known.is_some_and(|k| k.table_only);
    if differing != expected {
        return Verdict::Groups {
            new: differing
                .iter()
                .filter(|g| !expected.contains(g))
                .copied()
                .collect(),
            fixed: expected
                .iter()
                .filter(|g| !differing.contains(g))
                .copied()
                .collect(),
        };
    }
    match (differing.is_empty(), container_matches, table_only) {
        (true, false, false) => Verdict::Container,
        (true, true, true) => Verdict::StaleTableOnly,
        _ => Verdict::Explained,
    }
}

#[test]
fn judge_rules_on_every_shape_of_divergence() {
    let known = |groups: &'static [u32], table_only| Known {
        archive: 0,
        groups,
        table_only,
        why: "",
    };
    // Clean archive, nothing listed.
    assert_eq!(judge(&[], true, None), Verdict::Explained);
    // An unlisted group differs.
    assert_eq!(
        judge(&[4], false, None),
        Verdict::Groups {
            new: vec![4],
            fixed: vec![]
        }
    );
    // Exactly the listed groups differ; the container differs with them.
    assert_eq!(
        judge(&[1, 2], false, Some(&known(&[1, 2], false))),
        Verdict::Explained
    );
    // A listed group started to match: the list must shrink.
    assert_eq!(
        judge(&[1], false, Some(&known(&[1, 2], false))),
        Verdict::Groups {
            new: vec![],
            fixed: vec![2]
        }
    );
    // Groups agree but the container does not, and nothing says why.
    assert_eq!(judge(&[], false, None), Verdict::Container);
    // ... unless the table alone is known to be encoded differently.
    assert_eq!(
        judge(&[], false, Some(&known(&[], true))),
        Verdict::Explained
    );
    // That allowance goes stale once the container matches.
    assert_eq!(
        judge(&[], true, Some(&known(&[], true))),
        Verdict::StaleTableOnly
    );
}

#[test]
fn known_group_lists_are_sorted_and_unique() {
    // `judge` compares against `TableDiff::groups()`, which is sorted.
    for k in KNOWN {
        assert!(
            k.groups.windows(2).all(|w| w[0] < w[1]),
            "KNOWN archive {}: groups must be strictly ascending",
            k.archive
        );
    }
    let mut archives: Vec<u8> = KNOWN.iter().map(|k| k.archive).collect();
    archives.dedup();
    assert_eq!(
        archives.len(),
        KNOWN.len(),
        "an archive is listed twice in KNOWN"
    );
}

fn summarise(diff: &openrs2::TableDiff) -> String {
    let mut parts = Vec::new();
    for (label, ids) in [
        ("ours-only", &diff.only_ours),
        ("openrs2-only", &diff.only_theirs),
        ("crc", &diff.crc),
        ("version", &diff.version),
    ] {
        if !ids.is_empty() {
            parts.push(format!("{label} {}", ids.len()));
        }
    }
    parts.join(", ")
}

/// Write our copy and OpenRS2's copy of one group side by side for diffing.
fn dump_group(store: &mut rs_dat2::CacheStore, id: u32, aid: u8, gid: u32) {
    let dir = Path::new("target").join("crc_gate").join(aid.to_string());
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(Some(ours)) = store.read_raw_group_bytes(aid, gid) {
        let _ = std::fs::write(dir.join(format!("{gid}.ours.dat")), &ours);
    }
    if let Ok(theirs) = openrs2::fetch_group_cached(&api_dir(), id, aid, gid) {
        let _ = std::fs::write(dir.join(format!("{gid}.openrs2.dat")), &theirs);
    }
}
