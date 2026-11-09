//! 289's quest varps are saved at 647.
//!
//! A player's save keeps only the varps whose scope is perm
//! (`player_save::extract_profile`). 647's quest varps are the client's own,
//! so their scope came from the cache, which knows none: until 289's `.varp`
//! files came across, every quest's progress was temp and lost at logout.
//! Each varp `rs-pack/tests/quest_varps_647.tsv` lists -- the quest varps 289
//! and the 647 client share -- must load as perm.

#![cfg(since_647)]

use rs_pack::cache::varp::VarPlayerScope;
use rs_test::runner;

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
                .get_by_id(id)
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
            "tbwt_tinsay",
            "tbwt_tamayu",
            "tbwt_lubufu",
            "tbwt_bits",
        ] {
            let id = *engine
                .cache
                .varps
                .debugnames
                .get(name)
                .unwrap_or_else(|| panic!("{name} is not registered"));
            assert!(id < 2181, "{name} ({id}) is one of the cache's varps");
            let varp = engine.cache.varps.get_by_id(id).unwrap();
            assert_eq!(varp.scope, VarPlayerScope::Perm, "{name} is saved");
        }
    });
}
