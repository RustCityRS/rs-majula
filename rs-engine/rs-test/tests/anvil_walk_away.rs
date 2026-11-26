//! Smithing several items at an anvil, and walking away from it mid-batch.
//!
//! "The Smithing skill has also received the Make X feature, you will also be
//! free to walk away from the anvil at any point if you make a mistake or change
//! your mind." (Update:Assist System, 5 November 2007; the 2011 wiki's page,
//! revision of 18 October 2010.) 530 (1 February 2009), 647 (17 May 2011) and
//! 742 (7 November 2012) come after it, so a click away from the anvil ends
//! the batch there: no more items, no more bars used. The items keep the pace
//! each revision had (289's p_delays; no source changes it): the first one two
//! ticks after the click, then one every [`EVERY`] ticks. Running out of bars
//! still says so at the next strike. 225 to 289 (2004-2005) come before the
//! update and keep holding the player for the batch; rs-test's harness starts
//! at 530, so this suite does not build for them.
//!
//! The anvil is one of Varrock's west anvils (loc 2783, `anvil`, at
//! 3188,3421), which 530's, 647's and 742's maps all have. A bronze bar used on
//! it (OPLOCT) opens the anvil interface (smithing_new); a slot's Make All
//! (IF_BUTTON1), or Make X (IF_BUTTON1, then RESUME_P_COUNTDIALOG), smiths
//! from that bar: a bronze dagger is a bar.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_test::names::BACKPACK;
use rs_test::names::{com, iface};
use rs_test::runner;
use rs_ver::TypeId;
use std::sync::Mutex;

/// Varrock's west anvil (loc 2783 at 3188,3421).
const ANVIL: (u16, u16, u16) = (2783, 3188, 3421);

/// The cycle that makes the first item, counted from the one that takes the
/// click (cycle 1), and the cycles between items after it: the pace 289's
/// p_delays gave, which no source changes for these revisions.
const FIRST: usize = 3;
const EVERY: usize = 5;

const NONE: u16 = 0xFFFF;

const MAKE_ALL: &str = "smithing_new:smith_dagger_all";
const MAKE_X: &str = "smithing_new:smith_dagger_x";

/// Every engine in this binary shares rs-pathfinder's process-static
/// collision map: the cases take turns.
static TURN: Mutex<()> = Mutex::new(());

fn obj(engine: &Engine, name: &str) -> TypeId {
    engine
        .cache
        .objs
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
        .id
}

/// How many `name` the player carries.
fn carried(engine: &Engine, pid: u16, name: &str) -> u32 {
    let id = obj(engine, name);
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    let player = engine.harness_player(pid).unwrap();
    player.invs.get(&inv).unwrap().total(id)
}

/// Logs a smith in beside the anvil with a hammer and `bars` bronze bars,
/// uses a bar on the anvil and clicks the dagger's Make All, or with `count`
/// its Make X and then answers how many. The next cycle takes the click (or
/// the answer).
fn make_daggers(engine: &mut Engine, name: &str, bars: u16, count: Option<i32>) -> HarnessClient {
    let hammer = obj(engine, "hammer");
    let bronze = obj(engine, "bronze_bar");
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;

    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    let (sx, sz) = [(1, 0), (-1, 0), (0, -1), (0, 1)]
        .into_iter()
        .map(|(dx, dz)| {
            (
                ANVIL.1.wrapping_add_signed(dx),
                ANVIL.2.wrapping_add_signed(dz),
            )
        })
        .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
        .expect("a walkable tile beside the anvil");
    engine.harness_telejump(client.pid, sx, 0, sz);
    let player = engine.harness_player_mut(client.pid).unwrap();
    let backpack = player.invs.get_mut(&inv).unwrap();
    backpack.set(0, hammer, 1);
    for slot in 0..bars {
        backpack.set(slot + 1, bronze, 1);
    }
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();

    let (loc, x, z) = ANVIL;
    client.op_loc_t(x, z, TypeId::from_int(loc as i32), BACKPACK, 1, bronze);
    let smithing = iface("smithing_new");
    for _ in 0..10 {
        engine.cycle();
        if engine
            .harness_player(client.pid)
            .unwrap()
            .is_interface_open(smithing)
        {
            break;
        }
    }
    assert!(
        engine
            .harness_player(client.pid)
            .unwrap()
            .is_interface_open(smithing),
        "the anvil interface opens"
    );
    client.packets();
    let Some(count) = count else {
        client.if_button(1, com(MAKE_ALL), NONE, TypeId::NULL);
        return client;
    };
    client.if_button(1, com(MAKE_X), NONE, TypeId::NULL);
    engine.cycle();
    client.packets();
    client.resume_p_countdialog(count);
    client
}

/// Steps `n` cycles and returns the cycles (from 1) on which a dagger came,
/// and what the game said.
fn daggers_by_cycle(
    engine: &mut Engine,
    client: &mut HarnessClient,
    n: usize,
) -> (Vec<usize>, Vec<String>) {
    let mut came = Vec::new();
    let mut said = Vec::new();
    let mut had = carried(engine, client.pid, "bronze_dagger");
    for cycle in 1..=n {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
        let has = carried(engine, client.pid, "bronze_dagger");
        for _ in had..has {
            came.push(cycle);
        }
        had = has;
    }
    (came, said)
}

#[test]
fn make_all_smiths_each_dagger_at_the_anvil_s_pace() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = make_daggers(engine, "anvilpace", 4, None);
        let (came, said) = daggers_by_cycle(engine, &mut client, 30);
        assert_eq!(
            came,
            (0..4).map(|i| FIRST + i * EVERY).collect::<Vec<_>>(),
            "the game said {said:?}"
        );
        assert_eq!(carried(engine, client.pid, "bronze_bar"), 0);
        assert_eq!(
            said, ["You hammer the bronze and make a dagger."; 4],
            "no word of running out"
        );
        assert!(engine.script_errors.take().is_empty());
    });
}

#[test]
fn walking_away_from_the_anvil_ends_the_batch() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = make_daggers(engine, "anvilwalk", 4, None);
        // The first dagger, then a click on the ground three tiles away.
        let (came, said) = daggers_by_cycle(engine, &mut client, FIRST);
        assert_eq!(came, [FIRST], "the game said {said:?}");
        let start = engine.harness_player(client.pid).unwrap().pathing.coord;
        client.move_gameclick(start.x() + 3, start.z());

        let (came, said) = daggers_by_cycle(engine, &mut client, 30);
        assert_eq!(came, [] as [usize; 0], "the game said {said:?}");
        assert_eq!(carried(engine, client.pid, "bronze_dagger"), 1);
        assert_eq!(carried(engine, client.pid, "bronze_bar"), 3, "no bar used");
        assert!(said.is_empty(), "the game said {said:?}");
        let end = engine.harness_player(client.pid).unwrap().pathing.coord;
        assert_ne!(end, start, "the smith walked away");
        assert!(engine.script_errors.take().is_empty());
    });
}

#[test]
fn running_out_of_bars_ends_the_batch_with_a_word() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // Make X 5 with two bars: two daggers, then at the next strike (three
        // cycles after the second) the anvil says the bars have run out. (Make
        // X, not Make 5: at 530 [if_button1,smithing_new:smith_dagger_5] and
        // [if_button1,shop_template:main_stock], 300:23 and 620:23, share a
        // script lookup key, which is not this suite's to fix.)
        let mut client = make_daggers(engine, "anvilshort", 2, Some(5));
        let mut came = Vec::new();
        let mut boxed = None;
        let mut said = Vec::new();
        let mut had = 0;
        for cycle in 1..=30 {
            engine.cycle();
            let packets = client.packets();
            said.extend(
                HarnessClient::game_messages(&packets)
                    .into_iter()
                    .map(|(_, t)| t),
            );
            if boxed.is_none()
                && rs_test::talk::settexts(&packets)
                    .iter()
                    .any(|(_, t)| t.contains("enough bronze bars to make any more"))
            {
                boxed = Some(cycle);
            }
            let has = carried(engine, client.pid, "bronze_dagger");
            for _ in had..has {
                came.push(cycle);
            }
            had = has;
        }
        assert_eq!(came, [FIRST, FIRST + EVERY], "the game said {said:?}");
        assert_eq!(
            boxed,
            Some(FIRST + EVERY + 3),
            "the word comes at the strike"
        );
        assert_eq!(carried(engine, client.pid, "bronze_bar"), 0);
        assert!(engine.script_errors.take().is_empty());
    });
}
