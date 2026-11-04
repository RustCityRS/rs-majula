//! 2011's make-X amount is kept across logouts, as void-634 keeps it
//! (`skill_creation_amount`, persist = true): in the server-only, saved
//! `skill_creation_saved`. Varp 1363, which holds the client's copy beside
//! character creation's varbits, is not saved.

#![cfg(since_647)]

use rs_pack::cache::varp::VarPlayerScope;
use rs_test::runner;

#[test]
fn the_make_x_amount_is_saved_and_varp_1363_is_not() {
    runner::on_loaded_engine(|engine| {
        let saved = engine.cache.varps.get_by_id(2235).expect("varp 2235");
        assert_eq!(
            saved.scope,
            VarPlayerScope::Perm,
            "skill_creation_saved is saved"
        );
        // The number panel's buttons write it unprotected, as they write 1363.
        assert!(
            !saved.protect,
            "skill_creation_saved takes the buttons' writes"
        );
        let shared = engine.cache.varps.get_by_id(1363).expect("varp 1363");
        assert_ne!(shared.scope, VarPlayerScope::Perm, "varp 1363 is not saved");
    });
}
