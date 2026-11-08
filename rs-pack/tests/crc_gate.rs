//! Full offline comparison against the original OpenRS2 274 reference tables.
//! Run with `REV=647 cargo test -p rs-pack --test crc_gate -- --ignored`.
#![cfg(rev = "647")]

use rs_crypto::whirlpool;
use rs_io::js5::compression::compress_best;
use rs_io::js5::compute_group_crc;
use rs_io::js5::reference::encode_reference_table;
use rs_pack::archive::{self, ARCHIVE_IDS, EXPECTED_CRCS};
use rs_pack::cache::js5::PackedCache;
use serde_json::Value;
#[path = "support/reference.rs"]
mod reference;
use std::collections::BTreeMap;
use std::path::Path;
use std::{env, fs};

const NAMES: [&str; 36] = [
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
    "loading_sprites_raw",
    "worldmap_locs",
];

/// A known, explained divergence of one archive from OpenRS2 274.
struct Known {
    archive: u8,
    /// Groups whose CRC, version or presence differs. Must be exact, sorted.
    groups: &'static [u32],
    #[allow(dead_code)] // read by people, not code
    why: &'static str,
}

/// Exact divergences from OpenRS2 274.
const KNOWN: &[Known] = &[
    Known {
        archive: 5,
        // The 29 landscape squares whose 274 keys nobody has (OpenRS2 lists
        // none), rebuilt from donor caches and re-encrypted with the committed
        // key, plus the terrain of 4 of them: m51_53, m33_76, m48_50, m42_58.
        groups: &[
            80, 81, 198, 199, 616, 617, 978, 979, 1493, 1575, 1907, 1909, 1943, 1953, 2145, 3444,
            3592, 3812, 3816, 3818, 3822, 3826, 3835, 3865, 3890, 4005, 4037, 4041, 4045, 4074,
            5040, 5158, 5206,
        ],
        why: "donor-restored landscapes: 274's copies are unreadable without keys",
    },
    Known {
        archive: 7,
        // All 197 are groups 274 does not have at all; no 274 model is
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
        why: "models added for server overlay configs, absent from 274",
    },
    Known {
        archive: 16,
        groups: &[3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 17, 18, 19, 20, 21, 35],
        why: "loc config groups restored from donor caches, plus the woodcutting tree overlays",
    },
];

#[test]
#[ignore = "runs the full pack pipeline against content/647"]
fn assembled_cache_matches_openrs2_group_by_group() {
    // Integration tests run with cwd = rs-pack/; the pipeline's paths
    // (CONTENT_DIR, PACK_DIR) are workspace-root relative.
    if !Path::new(rs_pack::CONTENT_DIR).exists() {
        env::set_current_dir("..").expect("chdir to workspace root");
    }
    let id = 274;
    let source = Path::new(rs_pack::CONTENT_DIR);
    let pack = Path::new(rs_pack::PACK_DIR);

    let result = rs_pack::pack_all(source, pack, None, true, true).expect("pack_all failed");
    let mut store = result.cache;

    let known: BTreeMap<u8, &Known> = KNOWN.iter().map(|k| (k.archive, k)).collect();
    let mut failures = Vec::new();

    println!(
        "\n{:<18} {:>12} {:>12}  groups differing",
        "Archive", "OpenRS2", "Ours"
    );
    println!("{}", "-".repeat(64));
    for (pos, &aid) in ARCHIVE_IDS.iter().enumerate() {
        let theirs_raw = fs::read(format!("rs-pack/tests/fixtures/647/{aid}.dat")).unwrap();
        let ours_raw = store
            .groups
            .get(&(255, aid as u32))
            .expect("missing reference table");
        for group in &store.tables[&aid].groups {
            let raw = &store.groups[&(aid, group.id)];
            assert_eq!(
                compute_group_crc(&raw[..raw.len() - 2]),
                group.crc,
                "archive {aid} group {}",
                group.id
            );
            assert_eq!(
                u16::from_be_bytes(raw[raw.len() - 2..].try_into().unwrap()),
                group.version as u16
            );
            if let Some(digest) = group.whirlpool {
                assert_eq!(whirlpool(&raw[..raw.len() - 2]), digest);
            }
        }
        let theirs_crc = compute_group_crc(&theirs_raw);
        let ours_crc = compute_group_crc(ours_raw);
        assert_eq!(Some(ours_crc as i32), EXPECTED_CRCS[pos], "archive {aid}");

        let diff = reference::diff_reference_tables(
            &reference::parse_reference_container(ours_raw).unwrap(),
            &reference::parse_reference_container(&theirs_raw).unwrap(),
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

        let expected = known.get(&aid).map_or(&[][..], |k| k.groups);
        if differing != expected {
            failures.push(format!(
                "archive {aid}: differing groups {differing:?}, expected {expected:?}"
            ));
        }
        if diff.header || differing.is_empty() && ours_crc != theirs_crc {
            failures.push(format!(
                "archive {aid}: reference table differs from OpenRS2 unexpectedly"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} archive(s) diverge from OpenRS2 {id} beyond KNOWN:\n  {}",
        failures.len(),
        failures.join("\n  "),
    );

    // 3. Every committed key opens its landscape group in our cache.
    let keys = fs::read_to_string(source.join("keys.json")).unwrap();
    let keys: Vec<Value> = serde_json::from_str(&keys).unwrap();
    let bad: Vec<_> = keys
        .iter()
        .filter_map(|entry| {
            let archive: u32 = serde_json::from_value(entry["archive"].clone()).unwrap();
            let group: u32 = serde_json::from_value(entry["group"].clone()).unwrap();
            let key: [i32; 4] = serde_json::from_value(entry["key"].clone()).unwrap();
            (!matches!(
                store.read_group_xtea(archive as u8, group, &key),
                Ok(Some(ref d)) if !d.is_empty()
            ))
            .then_some((archive, group))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "{} of {} committed keys do not open our cache: {bad:?}",
        bad.len(),
        keys.len()
    );

    rejects_corrupt_cache(&mut store);
}

fn rejects_corrupt_cache(store: &mut PackedCache) {
    for &aid in ARCHIVE_IDS {
        let Some(group) = store.tables[&aid].groups.first() else {
            continue;
        };
        let gid = group.id;
        let raw = store.groups.get_mut(&(aid, gid)).unwrap();
        raw[5] ^= 1;
        let error = archive::verify_crcs(store, true).unwrap_err().to_string();
        assert!(
            error.contains(&format!("archive {aid} group {gid}:")),
            "{error}"
        );
        store.groups.get_mut(&(aid, gid)).unwrap()[5] ^= 1;
    }

    // A self-consistent replacement in a formerly skipped donor group must fail too.
    for (aid, gid) in [(5, 80), (7, 526), (16, 3)] {
        let original = store.groups[&(aid, gid)].clone();
        let raw = store.groups.get_mut(&(aid, gid)).unwrap();
        raw[5] ^= 1;
        let crc = compute_group_crc(&raw[..raw.len() - 2]);
        let table = store.tables.get_mut(&aid).unwrap();
        let index = table
            .groups
            .iter()
            .position(|group| group.id == gid)
            .unwrap();
        let old_crc = table.groups[index].crc;
        table.groups[index].crc = crc;
        let encoded = compress_best(&encode_reference_table(table), true, true, None).unwrap();
        let reference = store.groups.insert((255, aid as u32), encoded).unwrap();
        let error = archive::verify_crcs(store, true).unwrap_err().to_string();
        assert!(error.contains(&format!("archive {aid}: got")), "{error}");
        archive::verify_crcs(store, false).expect("custom content remains supported");
        store.tables.get_mut(&aid).unwrap().groups[index].crc = old_crc;
        store.groups.insert((aid, gid), original);
        store.groups.insert((255, aid as u32), reference);
    }

    let key = (32, store.tables[&32].groups[0].id);
    let original = store.groups.remove(&key).unwrap();
    let error = archive::verify_crcs(store, true).unwrap_err().to_string();
    assert!(error.contains("missing payload"), "{error}");
    store.groups.insert(key, original);

    let last = store.groups.get_mut(&key).unwrap().pop().unwrap();
    let error = archive::verify_crcs(store, true).unwrap_err().to_string();
    assert!(error.contains("invalid version trailer"), "{error}");
    store.groups.get_mut(&key).unwrap().push(last);

    store.tables.get_mut(&32).unwrap().groups[0].name_hash ^= 1;
    let error = archive::verify_crcs(store, true).unwrap_err().to_string();
    assert!(error.contains("reference metadata differs"), "{error}");
    store.tables.get_mut(&32).unwrap().groups[0].name_hash ^= 1;

    let reference = store.groups.remove(&(255, 32)).unwrap();
    let error = archive::verify_crcs(store, true).unwrap_err().to_string();
    assert!(error.contains("missing reference container"), "{error}");
    store.groups.insert((255, 32), reference);

    store.groups.insert((35, 0), vec![0; 7]);
    let error = archive::verify_crcs(store, true).unwrap_err().to_string();
    assert!(error.contains("unreferenced containers"), "{error}");
    store.groups.remove(&(35, 0));
    archive::verify_crcs(store, true).unwrap();
}

#[test]
fn known_group_lists_are_sorted_and_unique() {
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

fn summarise(diff: &reference::TableDiff) -> String {
    let mut parts = Vec::new();
    for (label, ids) in [
        ("ours-only", &diff.only_ours),
        ("openrs2-only", &diff.only_theirs),
        ("crc", &diff.crc),
        ("version", &diff.version),
        ("metadata", &diff.metadata),
    ] {
        if !ids.is_empty() {
            parts.push(format!("{label} {}", ids.len()));
        }
    }
    parts.join(", ")
}
