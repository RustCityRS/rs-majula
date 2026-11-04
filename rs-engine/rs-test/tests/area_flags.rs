//! 647's free-to-play and multi-combat zones.
//!
//! The engine flags each zone from `content/647/maps/free2play.csv` and
//! `multiway.csv` when it loads. Before 647 the packer reads them; 647's
//! cache loader left both empty, so a free world loaded no square at all
//! and nowhere was multi-combat.

#![cfg(since_647)]

use rs_test::runner;
use rsmod::rsmod::flag::zone_flag::ZoneFlag;

fn flagged(x: u16, z: u16, flag: ZoneFlag) -> bool {
    rsmod::is_zone_flagged(x, z, 0, flag as u8)
}

#[test]
fn free_and_multi_zones_load() {
    runner::on_loaded_engine(|_engine| {
        // Free to play: Lumbridge, Varrock; members: Taverley, Catherby.
        assert!(flagged(3222, 3218, ZoneFlag::Free), "Lumbridge is free");
        assert!(flagged(3212, 3428, ZoneFlag::Free), "Varrock is free");
        assert!(!flagged(2894, 3456, ZoneFlag::Free), "Taverley is members'");
        assert!(!flagged(2809, 3436, ZoneFlag::Free), "Catherby is members'");

        // Multi-combat: Barbarian Village, Falador, Al Kharid's palace; not
        // Lumbridge.
        assert!(flagged(3082, 3420, ZoneFlag::Multi), "Barbarian Village");
        assert!(flagged(2965, 3380, ZoneFlag::Multi), "Falador");
        assert!(flagged(3290, 3170, ZoneFlag::Multi), "Al Kharid's palace");
        assert!(!flagged(3222, 3218, ZoneFlag::Multi), "Lumbridge is single");
    });
}
