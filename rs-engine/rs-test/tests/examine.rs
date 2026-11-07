//! Rev-647 examine is answered by the server.
//!
//! 225/289 clients examined from the cache's `desc`; the 647 client sends the
//! type id (OPNPC6/OPLOC6/OPOBJ6, `Statics1.do_action`: `p2(type id)`) and
//! prints whatever game message comes back. Until these had handlers they
//! fell through to the engine's unhandled-packet catch-all and examine said
//! nothing. Each case sends the frame as the client does and reads the reply
//! the way the client reads MESSAGE_GAME.

#![cfg(since_647)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

#[test]
fn examine_answers_with_the_types_desc() {
    runner::on_loaded_engine(|engine| {
        let npc = *engine.cache.npcs.debugnames.get("hans").expect("hans");
        // Locs load from the JS5 cache, which has no names: `tree2`.
        let loc = 1278;
        let obj = *engine
            .cache
            .objs
            .debugnames
            .get("tinderbox")
            .expect("tinderbox");

        assert_eq!(
            examine(engine, ClientProt::ExamineNpc, npc),
            [(0, "Servant of the Duke of Lumbridge.".to_string())]
        );
        assert_eq!(
            examine(engine, ClientProt::ExamineLoc, loc),
            [(0, "A commonly found tree.".to_string())]
        );
        assert_eq!(
            examine(engine, ClientProt::ExamineObj, obj),
            [(0, "Useful for lighting a fire.".to_string())]
        );

        // A type with no desc gets no answer, as in void-634.
        let bare = (0..engine.cache.objs.count() as u16)
            .find(|&id| {
                engine
                    .cache
                    .objs
                    .get_by_id(id)
                    .is_some_and(|t| t.desc.is_none())
            })
            .expect("an obj with no desc");
        assert_eq!(examine(engine, ClientProt::ExamineObj, bare), []);
    });
}

/// The game messages a returning player gets back for one examine of `id`.
fn examine(engine: &mut Engine, prot: ClientProt, id: u16) -> Vec<(i32, String)> {
    engine.harness_reset();
    let mut client = engine.harness_login_returning("examiner").expect("login");
    engine.cycle();
    client.packets();

    client.send(prot, &id.to_be_bytes());
    engine.cycle();
    HarnessClient::game_messages(&client.packets())
}
