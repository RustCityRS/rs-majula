//! The server offers every op 2011's client shows on a members world.
//!
//! 2011's cache gives many npc and loc ops as memberops (opcodes 150-154: the
//! client shows them in their slot on a members world, hides them on a free
//! one): 154 npc ops, 84 of them Pickpocket, and 985 loc ops, among them the
//! farming patches' Inspect, Harvest and Clear. The server refuses an op its
//! own type lacks (`handlers/opnpc.rs`, `handlers/oploc.rs`), and it had none
//! of them: the npc packer wrote memberops to the client's config only, and
//! the loc conversion read the plain ops alone. 647's server runs a members
//! world, so each memberop must be the server's op in that slot.

#![cfg(since_647)]

use std::collections::HashMap;

use rs_test::runner;

fn content(path: &str) -> String {
    let path = format!("{}/../../content/647/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).expect(&path)
}

fn pack_ids(kind: &str) -> HashMap<String, u16> {
    content(&format!("pack/{kind}.pack"))
        .lines()
        .filter_map(|l| l.split_once('='))
        .filter_map(|(id, n)| Some((n.to_string(), id.parse().ok()?)))
        .collect()
}

/// (name, slot 1-5, op) for every memberop in a cache-derived config file.
fn memberops(kind: &str) -> Vec<(String, usize, String)> {
    let mut out = Vec::new();
    let mut name = String::new();
    for line in content(&format!("scripts/_unpack/all.{kind}")).lines() {
        if let Some(h) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            name = h.to_string();
        } else if let Some(rest) = line.strip_prefix("memberop")
            && let Some((slot, op)) = rest.split_once('=')
            && let Ok(slot) = slot.parse::<usize>()
        {
            out.push((name.clone(), slot, op.to_string()));
        }
    }
    out
}

#[test]
fn every_members_op_is_the_servers() {
    runner::on_loaded_engine(|engine| {
        let mut missing = Vec::new();
        let npcs = pack_ids("npc");
        let npc_ops = memberops("npc");
        assert!(npc_ops.len() > 100, "read {} npc memberops", npc_ops.len());
        for (name, slot, op) in &npc_ops {
            let Some(&id) = npcs.get(name) else { continue };
            let has = engine
                .cache
                .npcs
                .get_by_id(id)
                .and_then(|t| t.op.as_ref())
                .and_then(|ops| ops.get(slot - 1).cloned().flatten());
            if has.is_none() {
                missing.push(format!("npc {name} ({id}) op{slot} {op}"));
            }
        }
        let locs = pack_ids("loc");
        let loc_ops = memberops("loc");
        assert!(loc_ops.len() > 900, "read {} loc memberops", loc_ops.len());
        for (name, slot, op) in &loc_ops {
            let Some(&id) = locs.get(name) else { continue };
            let has = engine
                .cache
                .locs
                .get_by_id(id)
                .and_then(|t| t.op.as_ref())
                .and_then(|ops| ops.get(slot - 1).cloned().flatten());
            if has.is_none() {
                missing.push(format!("loc {name} ({id}) op{slot} {op}"));
            }
        }
        assert!(
            missing.is_empty(),
            "{} members ops the server lacks:\n  {}",
            missing.len(),
            missing[..missing.len().min(20)].join("\n  ")
        );
    });
}

/// The server knows each 2011 song's and jingle's length (`midi_length`, by
/// `midi` id: a song at its id, a jingle at 10000 + its id), decoded from the
/// cache's Jagex MIDI. The length parser had read only 4 of them: the 647
/// client's rebuilt MIDI writes data bytes under a meta event's running status.
#[test]
fn every_2011_song_and_jingle_has_a_length() {
    runner::on_loaded_engine(|engine| {
        let lengths = &engine.cache.midi_tick_lengths;
        let songs = lengths.iter().take(10000).flatten().count();
        let jingles = lengths.iter().skip(10000).flatten().count();
        assert!(songs > 950, "{songs} song lengths");
        assert!(jingles > 450, "{jingles} jingle lengths");
        assert!(lengths[10152].is_some_and(|t| t > 1), "quest complete 1");
    });
}

/// An npc keeps its 289 hunt mode, by name. Before c7b4f69 the server's hunt
/// modes were numbered after the 2011 cache's 397 hunt ids (`cowardly` was
/// 404) and the packer once wrote the mode in one byte, so no npc hunted; the
/// ids are the server's own now. The green dragon's is `cowardly`, a player
/// hunt.
#[test]
fn an_npc_keeps_its_hunt_mode() {
    runner::on_loaded_engine(|engine| {
        let cowardly = pack_ids("hunt")["cowardly"];
        let dragon = pack_ids("npc")["green_dragon"];
        let mode = engine.cache.npcs.get_by_id(dragon).and_then(|t| t.huntmode);
        assert_eq!(mode, Some(cowardly), "green_dragon's huntmode");
        let hunt = engine
            .cache
            .hunts
            .get_by_id(cowardly)
            .expect("the cowardly hunt");
        assert_eq!(hunt.hunt_type, rs_pack::types::HuntModeType::Player);
    });
}
