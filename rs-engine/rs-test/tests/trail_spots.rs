//! Every Treasure Trail search spot 289 names has a searchable loc in 2011's
//! map.
//!
//! 289's search steps check a clue's `trail_coord` against the loc searched
//! (`general_use` chests, crates and drawers), and its riddles name a few
//! spots in those scripts. 2011 rebuilt places and remade scenery, so a spot
//! 289 placed on a crate may hold another loc at 2011, or none. The audit
//! reads each spot's locs from the loaded 647 map and needs one whose
//! category 289's hooks answer.

#![cfg(since_647)]

use rs_engine::Engine;
use rs_pack::types::ParamValue;
use rs_test::runner;

/// The categories 289's search hooks answer (`general_use` chests, crates,
/// drawers), with 2011's open drawers and chests whose ops moved
/// (`general_use/configs/searchables_2011.loc`).
const HOOKED: [&str; 11] = [
    "chest_closed",
    "chest_open",
    "chest_open_2011",
    "chest_open_2011b",
    "chest_open_2011c",
    "empty_crate",
    "empty_crates",
    "empty_boxes",
    "drawer",
    "empty_drawer",
    "empty_drawer_2011",
];

/// The spots 289's hooks name outright for riddles, as `level_mx_mz_lx_lz`
/// (its `~is_quest_chest` spots are quest chests, not trail spots), East
/// Ardougne's two moved to 2011's drawers.
const NAMED: [&str; 7] = [
    "0_42_54_21_22",
    "0_45_55_41_57",
    "0_50_54_56_31",
    "1_40_48_33_36",
    "1_40_51_15_62",
    "1_40_51_50_60",
    "1_43_49_57_29",
];

/// Spots with no searchable loc of the clue's kind near 289's in 2011's map,
/// which the 2011 wiki still names (Treasure Trails/Guide/Cryptics): where
/// 2011 put them is not yet known.
const OPEN: [(&str, &str); 2] = [
    (
        "trail_clue_easy_vague004",
        "the crate on the ground floor of East Ardougne's church ('next to the ladder', 2011 wiki): \
         none within 20 tiles of 289's",
    ),
    (
        "trail_clue_easy_vague014",
        "drawers upstairs in a house in eastern Falador: none within 8 tiles of 289's",
    ),
];

fn pack(kind: &str) -> Vec<(i32, String)> {
    let path = format!(
        "{}/../../content/647/pack/{kind}.pack",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path)
        .expect(&path)
        .lines()
        .filter_map(|l| l.split_once('='))
        .filter(|(id, _)| !id.contains(':'))
        .filter_map(|(id, n)| Some((id.parse().ok()?, n.to_string())))
        .collect()
}

fn id_of(entries: &[(i32, String)], name: &str) -> i32 {
    entries
        .iter()
        .find(|(_, n)| n == name)
        .map(|&(i, _)| i)
        .unwrap_or_else(|| panic!("no {name}"))
}

/// `level_mx_mz_lx_lz` as (level, x, z).
fn coord(s: &str) -> (u8, u16, u16) {
    let p: Vec<u16> = s.split('_').map(|n| n.parse().unwrap()).collect();
    (p[0] as u8, p[1] * 64 + p[3], p[2] * 64 + p[4])
}

/// Each loc whose origin is the spot: id, name, category, ops.
fn locs_at(engine: &Engine, (y, x, z): (u8, u16, u16), cats: &[(i32, String)]) -> Vec<String> {
    locs_near(engine, (y, x, z), 0, cats)
}

/// Each loc whose origin is within `r` tiles of the spot, as `locs_at`, with
/// its offset when it is not on the spot.
fn locs_near(
    engine: &Engine,
    (y, x, z): (u8, u16, u16),
    r: u16,
    cats: &[(i32, String)],
) -> Vec<String> {
    let locs = pack("loc");
    let mut out = Vec::new();
    for dx in 0..=2 * r {
        for dz in 0..=2 * r {
            let (tx, tz) = (x + dx - r, z + dz - r);
            out.extend(
                locs_on(engine, &locs, (y, tx, tz), cats)
                    .into_iter()
                    .map(|l| {
                        if (tx, tz) == (x, z) {
                            l
                        } else {
                            format!("{l} @{:+},{:+}", tx as i32 - x as i32, tz as i32 - z as i32)
                        }
                    }),
            );
        }
    }
    out
}

fn locs_on(
    engine: &Engine,
    locs: &[(i32, String)],
    (y, x, z): (u8, u16, u16),
    cats: &[(i32, String)],
) -> Vec<String> {
    let Some(zone) = engine.zones.zone(x, y, z) else {
        return vec![];
    };
    zone.locs
        .iter()
        .filter(|l| l.is_at(x, z) && l.visible())
        .map(|l| {
            let id = l.id();
            let ty = engine.cache.locs.get_by_id(id);
            let name = locs
                .iter()
                .find(|&&(i, _)| i == id as i32)
                .map_or("?", |(_, n)| n.as_str());
            let cat = ty
                .and_then(|t| t.category)
                .and_then(|c| cats.iter().find(|&&(i, _)| i == c as i32))
                .map_or("-", |(_, n)| n.as_str());
            let ops: Vec<&str> = ty
                .and_then(|t| t.op.as_deref())
                .unwrap_or(&[])
                .iter()
                .flatten()
                .map(|o| &**o)
                .collect();
            format!("{id} {name} [{cat}] {ops:?}")
        })
        .collect()
}

#[test]
fn every_trail_search_spot_has_a_searchable_loc_in_2011s_map() {
    runner::on_loaded_engine(|engine| {
        let params = pack("param");
        let (trail_loc, trail_coord) = (id_of(&params, "trail_loc"), id_of(&params, "trail_coord"));
        let cats = pack("category");
        let objs = pack("obj");
        let mut spots: Vec<(String, (u8, u16, u16))> = Vec::new();
        for (id, name) in &objs {
            let Some(params) = engine
                .cache
                .objs
                .get_by_id(*id as u16)
                .and_then(|o| o.params.as_deref())
            else {
                continue;
            };
            let searched = matches!(params.get(&trail_loc), Some(ParamValue::Int(1)));
            if let (true, Some(ParamValue::Int(c))) = (searched, params.get(&trail_coord)) {
                let c = *c as u32;
                let at = (
                    (c >> 28) as u8,
                    ((c >> 14) & 0x3FFF) as u16,
                    (c & 0x3FFF) as u16,
                );
                spots.push((name.clone(), at));
            }
        }
        assert!(
            spots.len() >= 50,
            "found only {} searched clues",
            spots.len()
        );
        for s in NAMED {
            spots.push((format!("riddle {s}"), coord(s)));
        }

        let mut report = Vec::new();
        let mut missing = 0;
        for (clue, at) in &spots {
            let here = locs_at(engine, *at, &cats);
            let ok = here
                .iter()
                .any(|l| HOOKED.iter().any(|h| l.contains(&format!("[{h}]"))));
            let mut near = String::new();
            if !ok && !OPEN.iter().any(|(c, _)| c == clue) {
                missing += 1;
                // What 2011 has around it that opens or searches.
                let around: Vec<String> = locs_near(engine, *at, 8, &cats)
                    .into_iter()
                    .filter(|l| l.contains("\"Open\"") || l.contains("\"Search\""))
                    .collect();
                near = format!("\n       near: {}", around.join(" | "));
            }
            report.push(
                format!(
                    "{} {clue} at {}_{}_{}: {}",
                    if ok { "ok  " } else { "MISS" },
                    at.0,
                    at.1,
                    at.2,
                    if here.is_empty() {
                        "nothing".to_string()
                    } else {
                        here.join(" | ")
                    }
                ) + &near,
            );
        }
        report.sort();
        assert_eq!(
            missing,
            0,
            "{missing} spots unsearchable:\n{}",
            report.join("\n")
        );
    });
}
