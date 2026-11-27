//! 289's quest varps are saved at 647.
//!
//! A player's save keeps only the varps whose scope is perm
//! (`player_save::extract_profile`). 647's quest varps are the client's own,
//! so their scope came from the cache, which knows none: until 289's `.varp`
//! files came across, every quest's progress was temp and lost at logout.
//! Each varp `rs-pack/tests/quest_varps_647.tsv` lists -- the quest varps 289
//! and the 647 client share -- must load as perm.

#![cfg(since_530)]

use rs_pack::cache::varp::VarPlayerScope;
use rs_test::runner;
use rs_ver::TypeId;

#[test]
fn every_shared_quest_varp_is_saved() {
    runner::on_loaded_engine(|engine| {
        let mut temp = Vec::new();
        let mut count = 0;
        for line in include_str!("../../../rs-pack/tests/quest_varps_647.tsv")
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
        {
            let f: Vec<&str> = line.split('\t').collect();
            let (name, id) = (f[0], f[1].parse::<u16>().unwrap());
            let varp = engine
                .cache
                .varps
                .get_by_id(TypeId::new(id))
                .unwrap_or_else(|| panic!("no varp {id} ({name})"));
            if varp.scope != VarPlayerScope::Perm {
                temp.push(format!("{name} ({id})"));
            }
            count += 1;
        }
        assert_eq!(count, 47, "quest_varps_647.tsv changed size");
        assert!(temp.is_empty(), "quest varps a save would drop: {temp:?}");
    });
}

/// Tai Bwo Wannai Trio's sub-varps are on 289's ids, 321-324, and tbwt_bits,
/// which 289 lacks, on a free one: all cache 786's own, and saved.
#[test]
fn tai_bwo_wannai_trios_varps_are_the_caches_and_saved() {
    runner::on_loaded_engine(|engine| {
        for name in [
            "tbwt_tiadeche",
            "tbwt_lubufu",
            "tbwt_tamayu",
            "varplayer_324",
            "tbwt_bits",
        ] {
            let id = engine
                .cache
                .varps
                .get_by_debugname(name)
                .unwrap_or_else(|| panic!("{name} is not registered"))
                .id;
            assert!(
                id < TypeId::new(2181),
                "{name} ({id}) is one of the cache's varps"
            );
            let varp = engine.cache.varps.get_by_id(id).unwrap();
            assert_eq!(varp.scope, VarPlayerScope::Perm, "{name} is saved");
        }
    });
}

/// Every varp 289 saves is saved at 647 too, where 647 has it under 289's
/// name. The cache knows no scope, so each came over as temp: Dragon
/// Slayer's, Goblin Diplomacy's, Heroes' Quest's, Lost City's and the music
/// and sound options, among others, were lost at every logout.
#[test]
fn every_varp_289_saves_is_saved() {
    runner::on_loaded_engine(|engine| {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/289/scripts");
        let mut perm = Vec::new();
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("289 scripts").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "varp") {
                    let mut name = None;
                    for line in std::fs::read_to_string(&path).unwrap().lines() {
                        let line = line.trim();
                        if let Some(n) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                            name = Some(n.to_string());
                        } else if line == "scope=perm"
                            && let Some(n) = &name
                        {
                            perm.push(n.clone());
                        }
                    }
                }
            }
        }
        assert!(perm.len() > 200, "289's saved varps: {}", perm.len());
        let temp: Vec<_> = perm
            .iter()
            .filter(|name| {
                engine
                    .cache
                    .varps
                    .get_by_debugname(name)
                    .is_some_and(|v| v.scope != VarPlayerScope::Perm)
            })
            .collect();
        assert!(
            temp.is_empty(),
            "289 saves these, 647 drops them at logout: {temp:?}"
        );
    });
}

/// Unstable Foundations' progress bits are saved: its step hint (varbit
/// 3756, in varp 1021) and its tutorial3_* bits (in tutorial2_tasks2), which
/// the client reads for the quest's hints.
#[test]
fn unstable_foundations_progress_bits_are_saved() {
    runner::on_loaded_engine(|engine| {
        for name in ["varplayer_1021", "tutorial2_tasks2"] {
            let varp = engine
                .cache
                .varps
                .get_by_debugname(name)
                .unwrap_or_else(|| panic!("{name} is not registered"));
            assert_eq!(varp.scope, VarPlayerScope::Perm, "{name} is saved");
        }
    });
}
