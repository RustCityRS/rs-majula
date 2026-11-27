//! 2011's Task System keeps its progress across logins.
//!
//! A player's save keeps only the varps whose scope is perm
//! (`player_save::extract_profile`). Cache 786 knows no scope, so the varps
//! that hold each task's done bit (`tasks/scripts/task_done.rs2`) and the
//! task tab's state (`task_system.rs2`) loaded as temp: On the Run (varbit
//! 8522, in varp 1956) and every other task came back undone at each login.

#![cfg(all(since_647, before_742))]

use rs_pack::cache::varp::VarPlayerScope;
use rs_test::runner;

#[test]
fn every_task_varp_is_saved() {
    runner::on_loaded_engine(|engine| {
        let mut temp = Vec::new();
        for name in [
            "atfal_task2",
            "atjun_tasks_1",
            "atjun_tasks_2",
            "atjun_tasks_3",
            "atvar_task1",
            "atvar_task2",
            "atvar_task3",
            "tutorial4_tasks",
            "varplayer_1256",
            "varplayer_1257",
            "varplayer_1258",
            "varplayer_1289",
            "varplayer_1358",
            "varplayer_1359",
            "varplayer_1360",
            "varplayer_1372",
            "varplayer_1373",
            "varplayer_1374",
            "varplayer_1562",
            "varplayer_1563",
            "varplayer_1564",
            "varplayer_1565",
            "varplayer_1886",
            "varplayer_1887",
            // Run, Rest and the other Lumbridge/Draynor beginner tasks.
            "varplayer_1956",
            "varplayer_1957",
            // The total done, the set shown, the pin and the tab's six slots.
            "varplayer_1958",
            "varplayer_1960",
            "varplayer_1963",
            "varplayer_1964",
            "varplayer_1965",
        ] {
            let varp = engine
                .cache
                .varps
                .get_by_debugname(name)
                .unwrap_or_else(|| panic!("{name} is not registered"));
            if varp.scope != VarPlayerScope::Perm {
                temp.push(name);
            }
        }
        assert!(temp.is_empty(), "task varps a save would drop: {temp:?}");
    });
}
