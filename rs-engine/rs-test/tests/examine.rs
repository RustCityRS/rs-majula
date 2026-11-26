//! From rev 530 examine is answered by the server.
//!
//! 225/289 clients examined from the cache's `desc`; the 530 and 647 clients
//! send the type id (OPNPC6/OPLOC6/OPOBJ6, 647 `Statics1.do_action`: `p2(type
//! id)`, 530 `p2_alt3` for locs and objs) and print whatever game message
//! comes back. An npc must be in the player's build area; 742 sends the tile
//! of a loc or obj, which that tile's zone must hold. Each case sends the
//! frame as the client does and reads the reply the way the client reads
//! MESSAGE_GAME.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client::oploc6::OpLoc6;
use rs_protocol::network::game::client::opnpc6::OpNpc6;
use rs_protocol::network::game::client::opobj6::OpObj6;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_ver::TypeId;

const LUMBRIDGE: (u16, u16) = (3222, 3218);

/// What a tree near Lumbridge examines as.
#[cfg(before_742)]
const TREE: &str = "A commonly found tree.";
#[cfg(since_742)]
const TREE: &str = "One of the most common trees in RuneScape.";

#[test]
fn examine_answers_with_the_types_desc() {
    runner::on_loaded_engine(|engine| {
        let hans = engine.cache.npcs.get_by_debugname("hans").expect("hans").id;
        let tinderbox = engine
            .cache
            .objs
            .get_by_debugname("tinderbox")
            .expect("tinderbox")
            .id;
        let bare = (0..engine.cache.objs.count() as u16)
            .map(TypeId::new)
            .find(|&id| {
                engine
                    .cache
                    .objs
                    .get_by_id(id)
                    .is_some_and(|t| t.desc.is_none())
            })
            .expect("an obj with no desc");

        engine.harness_reset();
        let (x, z) = LUMBRIDGE;
        let mut client = engine
            .harness_login_returning_at("examiner", x, z)
            .expect("login");
        engine.harness_add_obj(x, 0, z, tinderbox, 1);
        engine.harness_add_obj(x, 0, z, bare, 1);
        settle(engine, &mut client);
        // Locs load from the JS5 cache, which has no names.
        let (tree_x, tree_z, tree) = loc_near(engine, TREE, x, z);

        let npc = |id| (ClientProt::OpNpc6, 0, 0, id);
        let loc = |id| (ClientProt::OpLoc6, tree_x, tree_z, id);
        let obj = |id| (ClientProt::OpObj6, x, z, id);

        assert_eq!(
            examine(engine, &mut client, npc(hans)),
            [(0, "Servant of the Duke of Lumbridge.".to_string())]
        );
        assert_eq!(
            examine(engine, &mut client, loc(tree)),
            [(0, TREE.to_string())]
        );
        assert_eq!(
            examine(engine, &mut client, obj(tinderbox)),
            [(0, "Useful for lighting a fire.".to_string())]
        );
        assert_eq!(
            examine(engine, &mut client, obj(bare)),
            [(0, "This examine is missing!".to_string())]
        );

        // None of them is on the level above.
        engine.harness_telejump(client.pid, x, 3, z);
        settle(engine, &mut client);
        assert_eq!(examine(engine, &mut client, npc(hans)), []);
        #[cfg(since_742)]
        {
            assert_eq!(examine(engine, &mut client, loc(tree)), []);
            assert_eq!(examine(engine, &mut client, obj(tinderbox)), []);
        }
    });
}

/// Past what the first ticks say (530 unlocks the spawn's track).
fn settle(engine: &mut Engine, client: &mut HarnessClient) {
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
}

/// The tile and type of a visible loc within 20 tiles of `x`, `z` on level 0
/// whose type's `desc` is `desc`.
fn loc_near(engine: &Engine, desc: &str, x: u16, z: u16) -> (u16, u16, TypeId) {
    (x - 20..=x + 20)
        .flat_map(|lx| (z - 20..=z + 20).map(move |lz| (lx, lz)))
        .find_map(|(lx, lz)| {
            let zone = engine.zones.zone(lx, 0, lz)?;
            let loc = zone.locs.iter().find(|loc| {
                loc.visible()
                    && loc.is_at(lx, lz)
                    && engine
                        .cache
                        .locs
                        .get_by_id(loc.id())
                        .is_some_and(|t| t.desc.as_deref() == Some(desc))
            })?;
            Some((lx, lz, loc.id()))
        })
        .expect("a loc nearby")
}

/// The game messages the player gets back for one examine of `id`, which
/// 742 sends with the tile `x`, `z` for a loc or obj.
fn examine(
    engine: &mut Engine,
    client: &mut HarnessClient,
    (prot, x, z, id): (ClientProt, u16, u16, TypeId),
) -> Vec<(i32, String)> {
    #[cfg(before_742)]
    let _ = (x, z);
    match prot {
        ClientProt::OpLoc6 => client.send_msg(&OpLoc6 {
            #[cfg(since_742)]
            x,
            #[cfg(since_742)]
            z,
            loc: id,
            #[cfg(since_742)]
            ctrl: false,
        }),
        ClientProt::OpNpc6 => client.send_msg(&OpNpc6 {
            npc: id,
            #[cfg(since_742)]
            ctrl: false,
        }),
        ClientProt::OpObj6 => client.send_msg(&OpObj6 {
            #[cfg(since_742)]
            x,
            #[cfg(since_742)]
            z,
            obj: id,
            #[cfg(since_742)]
            ctrl: false,
        }),
        _ => panic!("{prot:?} is not an examine"),
    }
    engine.cycle();
    HarnessClient::game_messages(&client.packets())
}
