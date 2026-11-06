//! 2011's reworked Imp Catcher, talked through as the client does: Wizard
//! Mizgog (706, the top of the Wizards' Tower, 3103,3163 on level 2) starts it
//! when asked nicely and the quest is accepted (void-634's flow). He ends it
//! for the four beads after the rework's cutscene, with the 2011 wiki's reward
//! (Imp Catcher, 2011-05-16): an amulet of accuracy and 875 Magic experience.

#![cfg(since_647)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_test::runner;
use rs_test::talk::{converse, settexts, talk_to};
use rsmod::rsmod::collision::collision_strategy::CollisionType;

static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

const MIZGOG: u16 = 706;
const MAGIC: usize = 6;

/// A player beside Mizgog, with Imp Catcher at `stage` and `objs` carried.
fn visitor(engine: &mut Engine, stage: i32, objs: &[&str]) -> (HarnessClient, u16) {
    engine.harness_reset();
    let mut client = engine.harness_login_returning("imp_hunter").expect("login");
    for _ in 0..10 {
        engine.cycle();
    }
    let (nid, x, z) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == MIZGOG)
        .map(|n| {
            (
                n.npc.uid.nid(),
                n.npc.pathing.coord.x(),
                n.npc.pathing.coord.z(),
            )
        })
        .expect("Mizgog is spawned");
    // One step from Mizgog, so no wall stands between (he wanders).
    let (sx, sz) = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find(|&(dx, dz)| {
            rsmod::can_travel(2, x, z, dx, dz, 1, 0, CollisionType::Normal)
                && !rsmod::is_flagged(
                    x.wrapping_add_signed(dx as i16),
                    z.wrapping_add_signed(dz as i16),
                    2,
                    0x100 | 0x200000,
                )
        })
        .map(|(dx, dz)| {
            (
                x.wrapping_add_signed(dx as i16),
                z.wrapping_add_signed(dz as i16),
            )
        })
        .expect("a walkable tile beside Mizgog");
    engine.harness_telejump(client.pid, sx, 2, sz);
    let varp = *engine.cache.varps.debugnames.get("imp").unwrap();
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let ids: Vec<u16> = objs
        .iter()
        .map(|o| *engine.cache.objs.debugnames.get(*o).unwrap())
        .collect();
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.vars.set(varp, VarValue::Int(stage));
    let backpack = player.invs.get_mut(&inv).unwrap();
    for (slot, id) in ids.into_iter().enumerate() {
        backpack.set(slot as u16, id, 1);
    }
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    (client, nid)
}

fn stage(engine: &Engine, client: &HarnessClient) -> i32 {
    let varp = *engine.cache.varps.debugnames.get("imp").unwrap();
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(varp)
        .as_int()
}

#[test]
fn mizgog_gives_the_quest_when_asked_nicely() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = visitor(engine, 0, &[]);
        talk_to(&mut client, nid);
        let talk = converse(
            engine,
            &mut client,
            &["Give me a quest please.", "Yes."],
            60,
        );
        assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
        assert!(
            talk.said("These imps have now spread out all over the kingdom."),
            "289's ask; said {:?}",
            talk.texts
        );
        assert_eq!(stage(engine, &client), 1, "Imp Catcher started");
    });
}

#[test]
fn mizgog_takes_the_four_beads_for_an_amulet_of_accuracy() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let beads = ["black_bead", "white_bead", "yellow_bead", "red_bead"];
        let (mut client, nid) = visitor(engine, 1, &beads);
        let before = engine.harness_player(client.pid).unwrap().stats.xp[MAGIC];
        talk_to(&mut client, nid);
        let mut talk = converse(engine, &mut client, &[], 60);
        // The cutscene runs about 13 ticks after its last box.
        for _ in 0..20 {
            engine.cycle();
            let packets = client.packets();
            talk.messages.extend(
                HarnessClient::game_messages(&packets)
                    .into_iter()
                    .map(|(_, t)| t),
            );
            talk.texts
                .extend(settexts(&packets).into_iter().map(|(_, t)| t));
        }
        // 647 breaks interface text with <br>. 289's `\\n` is the 2004
        // client's line break; the 2011 client draws it as it stands.
        assert!(
            talk.texts
                .iter()
                .any(|t| t == "You have completed the<br>Imp Catcher Quest!"),
            "the quest scroll; set {:?}",
            talk.texts
        );
        assert!(
            talk.texts.iter().all(|t| !t.contains('\\')),
            "a 2004 escape reached the client: {:?}",
            talk.texts
        );
        assert!(
            talk.messages
                .iter()
                .any(|m| m == "The wizard hands you an amulet."),
            "the cutscene's end; said {:?}",
            talk.messages
        );
        assert_eq!(
            stage(engine, &client),
            2,
            "Imp Catcher complete; said {:?}",
            talk.texts
        );
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        let id = |o: &str| *engine.cache.objs.debugnames.get(o).unwrap();
        assert_eq!(
            backpack.total(id("amulet_of_accuracy")) as i64,
            1,
            "the amulet"
        );
        for bead in beads {
            assert_eq!(backpack.total(id(bead)) as i64, 0, "{bead} given");
        }
        assert_eq!(
            player.stats.xp[MAGIC] - before,
            8750,
            "875 Magic xp, in tenths"
        );
    });
}
