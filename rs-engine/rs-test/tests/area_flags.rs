//! 647's free-to-play zones and multi-combat area.
//!
//! The engine flags each zone from `content/647/maps/free2play.csv` when it
//! loads; 647's cache loader once left it empty, so a free world loaded no
//! square at all. Multi-combat is the `multiway` area painted in the maps'
//! `a<mx>_<mz>.jm2` files.

#![cfg(since_647)]

use rs_grid::CoordGrid;
use rs_test::runner;
use rsmod::rsmod::flag::zone_flag::ZoneFlag;

fn flagged(x: u16, z: u16, flag: ZoneFlag) -> bool {
    rsmod::is_zone_flagged(x, z, 0, flag as u8)
}

#[test]
fn free_zones_and_multi_area_load() {
    runner::on_loaded_engine(|engine| {
        // Free to play: Lumbridge, Varrock; members: Taverley, Catherby.
        assert!(flagged(3222, 3218, ZoneFlag::Free), "Lumbridge is free");
        assert!(flagged(3212, 3428, ZoneFlag::Free), "Varrock is free");
        assert!(!flagged(2894, 3456, ZoneFlag::Free), "Taverley is members'");
        assert!(!flagged(2809, 3436, ZoneFlag::Free), "Catherby is members'");

        // Multi-combat: Barbarian Village, Falador, Al Kharid's palace; not
        // Lumbridge.
        let multi = |x, z| engine.multiway(CoordGrid::new(x, 0, z));
        assert!(multi(3082, 3420), "Barbarian Village");
        assert!(multi(2965, 3380), "Falador");
        assert!(multi(3290, 3170), "Al Kharid's palace");
        assert!(!multi(3222, 3218), "Lumbridge is single");
    });
}
