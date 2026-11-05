//! 289's Bob at 647 (areas/area_lumbridge/scripts/bob.rs2): Talk-to offers
//! 289's three options, and asking for a quest gets 289's answer.

#![cfg(since_647)]

use rs_test::runner;
use rs_test::talk::{converse, talk_to};

const BOB: u16 = 519;

#[test]
fn bob_answers_289_s_quest_question() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("axe_buyer").expect("login");
        for _ in 0..10 {
            engine.cycle();
        }
        let (nid, x, z) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .find(|n| n.npc.uid.id() == BOB)
            .map(|n| {
                (
                    n.npc.uid.nid(),
                    n.npc.pathing.coord.x(),
                    n.npc.pathing.coord.z(),
                )
            })
            .expect("Bob is spawned");
        let (sx, sz) = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1)]
            .into_iter()
            .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
            .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
            .expect("a walkable tile beside Bob");
        engine.harness_telejump(client.pid, sx, 0, sz);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();
        talk_to(&mut client, nid);
        let talk = converse(engine, &mut client, &["Give me a quest!"], 40);
        assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
        assert!(talk.said("Hello. How can I help you?"), "{:?}", talk.texts);
        assert!(
            talk.said("Get yer own!"),
            "289's answer; said {:?}",
            talk.texts
        );
    });
}
