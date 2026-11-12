//! Rev-647 targeted ops reach the trigger their *source* names.
//!
//! 647 sends one packet per target kind (`OpLocT`, `OpNpcT`, `OpObjT`,
//! `OpPlayerT`) carrying whatever the player had selected: an item from an
//! inventory, or a spell. Each case writes the frame as the 647 client's send
//! site does (`Statics1.do_action`), queues it on a harness player, lets the
//! engine read and dispatch it through its own input phase, and checks what
//! the handler set: the item cases `[ap|op]<target>u` with `last_useitem`, the
//! spell case `[ap|op]<target>t` keyed on the whole component hash.
//!
//! Until 647 dispatched these packets with their 647 structs, the engine
//! decoded them with the 225-era layouts -- 8 bytes of a 15-byte frame -- and
//! looked the source up as a pre-647 spell component, so every one of these
//! was dropped.
//!
//! 647's player ops 6-10 (OPPLAYER6-10, past 289's five) approach the other
//! player as ops 1-5 do, for `[applayerN]` and then `[opplayerN]`.
//!
//! A ground target (APCOORDT) aims a component at a tile, which the player
//! approaches for the component's `[apcoordt]`. A component that can't aim at
//! tiles, an item, or a tile the client could not have shown is refused.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::InteractionTarget;
use rs_grid::CoordGrid;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_vm::ops::test::calls_for;
use rs_vm::trigger::ServerTriggerType;

/// `inventory:inv`, the backpack component `[login]` binds `inv` to.
const BACKPACK: i32 = 149 << 16;

/// Open ground west of Falador, a tile from the tree the woodcutting tests
/// chop (`tree_1276` at 2552,3450).
const TILE: (u16, u16) = (2551, 3450);

/// Lumbridge, east of the castle: open ground `~area_allow_loc_add` accepts,
/// so a fire may be lit there.
const FIRE_TILE: (u16, u16) = (3221, 3219);

/// The `object_id` the client sends for a source holding no item.
const NO_OBJ: i32 = 0xFFFF;

/// `xmas10_snowball:throw_operation`, the 2010 snowball's thrower, which
/// aims at a player or a tile (`targetmask=player,coord`).
const SNOWBALL: i32 = (1061 << 16) | 8;

/// Lumbridge's spawn, with open ground east of it (see `pathfinding.rs`).
const SPAWN: (u16, u16) = (3222, 3218);

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

fn inv(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .invs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no inv {name}"))
}

/// A fresh world with one player logged in and standing at `x, z`.
fn player_at(engine: &mut Engine, name: &str, x: u16, z: u16) -> HarnessClient {
    engine.harness_reset();
    let client = engine.harness_login_returning(name).expect("login");
    engine.harness_telejump(client.pid, x, 0, z);
    // One tick moves the player and rebuilds the scene around the new tile,
    // so the scene-bounds and visibility checks see it.
    engine.cycle();
    client
}

/// Puts `obj` in backpack slot `slot`.
fn hold(engine: &mut Engine, pid: u16, slot: u16, obj: u16) {
    let inv = inv(engine, "inv");
    engine
        .harness_player_mut(pid)
        .and_then(|p| p.invs.get_mut(&inv))
        .expect("backpack transmitted at login")
        .set(slot, obj, 1);
}

#[test]
fn a_targeted_op_takes_its_source_from_the_packet() {
    runner::on_loaded_engine(|engine| {
        item_on_ground_obj(engine);
        item_on_loc(engine);
        spell_on_npc(engine);
        item_on_player(engine);
        forged_item_is_dropped(engine);
        a_forged_spell_source_is_dropped(engine);
    });
}

/// A player op, its packet, and how the client writes ctrl and the pid.
type PlayerOp = (u8, ClientProt, fn(&mut ClientPacket, i32));

/// Each OPPLAYER6-10, written as `Statics1.do_action` does, approaches the
/// other player for that op's `[applayerN]`.
#[test]
fn player_ops_6_to_10_approach_the_player() {
    runner::on_loaded_engine(|engine| {
        let sends: [PlayerOp; 5] = [
            (6, ClientProt::OpPlayer6, |p, id| _ = p.p1(0).p2_alt2(id)),
            (7, ClientProt::OpPlayer7, |p, id| _ = p.p1_alt2(0).p2(id)),
            (8, ClientProt::OpPlayer8, |p, id| _ = p.p1(0).p2_alt1(id)),
            (9, ClientProt::OpPlayer9, |p, id| _ = p.p2(id).p1_alt1(0)),
            (10, ClientProt::OpPlayer10, |p, id| _ = p.p1_alt2(0).p2(id)),
        ];
        let (x, z) = TILE;
        let mut client = player_at(engine, "op_player_n", x, z);
        let other = engine
            .harness_login_returning("op_player_target")
            .expect("second login");
        engine.harness_telejump(other.pid, x + 1, 0, z);
        engine.cycle();
        for (op, prot, write) in sends {
            let mut p = ClientPacket::new();
            write(&mut p, other.pid as i32);
            client.send(prot, &p.finish());
            engine.harness_inputs();
            let interaction = &engine.harness_player(client.pid).unwrap().interaction;
            let Some(InteractionTarget::Player { pid }) = interaction.target else {
                panic!("op {op} targets no player");
            };
            assert_eq!(pid, other.pid, "op {op} targets the other player");
            assert_eq!(
                interaction.target_op,
                ServerTriggerType::ap_player(op).map(|t| t as u8),
                "op {op}"
            );
        }
    });
}

/// Aims `com` at tile `x, z`, as `Statics1.do_action` writes APCOORDT for
/// TGT_GROUND. A static component's child id, the slot, is -1.
fn aim(client: &mut HarnessClient, com: i32, obj: i32, x: u16, z: u16) {
    let mut p = ClientPacket::new();
    p.p2_alt1(x as i32)
        .p2(0xFFFF)
        .p2_alt2(obj)
        .p2_alt2(z as i32)
        .p4(com);
    client.send(ClientProt::ApCoordT, &p.finish());
}

/// Opens the snowball's interface, which the next tick's output mounts.
fn open_snowball(engine: &mut Engine, pid: u16) {
    engine.harness_open_main(pid, (SNOWBALL >> 16) as u16);
    engine.cycle();
}

/// Whether the player is aiming at `tile`, for `[apcoordt]` keyed on `com`.
fn aiming(engine: &Engine, pid: u16, com: i32, tile: CoordGrid) -> bool {
    let player = engine.harness_player(pid).unwrap();
    matches!(player.interaction.target, Some(InteractionTarget::Coord { coord }) if coord == tile)
        && player.interaction.target_op == Some(ServerTriggerType::ApCoordT as u8)
        && player.interaction.target_subject_com == Some(com)
        && player.last_coord == Some(tile)
}

/// A ground target approaches its tile. With no `[apcoordt]` for the
/// snowball, the player walks onto it and nothing interesting happens.
#[test]
fn a_ground_target_approaches_its_tile() {
    runner::on_loaded_engine(|engine| {
        let (x, z) = SPAWN;
        let tile = CoordGrid::new(x + 3, 0, z);
        let mut client = player_at(engine, "ap_coord_t", x, z);
        open_snowball(engine, client.pid);
        aim(&mut client, SNOWBALL, NO_OBJ, tile.x(), tile.z());
        engine.harness_inputs();
        assert!(
            aiming(engine, client.pid, SNOWBALL, tile),
            "aims at the tile"
        );

        let mut messages = Vec::new();
        for _ in 0..6 {
            engine.cycle();
            messages.extend(HarnessClient::game_messages(&client.packets()));
        }
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.pathing.coord, tile, "walked onto the tile");
        assert!(
            messages
                .iter()
                .any(|(_, m)| m == "Nothing interesting happens."),
            "{messages:?}"
        );
        assert!(engine.script_errors.take().is_empty());
    });
}

/// A ground target the client could not have sent is refused: a component
/// closed or not aiming at tiles, one holding an obj, a tile off the scene
/// (or past the map, which a packed coord would wrap onto another), a
/// delayed player, and a tile no zone is in, beside a region.
#[test]
fn a_ground_target_is_refused_where_the_client_could_not_aim() {
    runner::on_loaded_engine(|engine| {
        let tinderbox = obj(engine, "tinderbox") as i32;
        // modern:wind_strike aims at npcs and players, not tiles.
        let spell = (192 << 16) | 25;
        let (x, z) = SPAWN;
        let refused = [
            ("its interface is closed", false, SNOWBALL, NO_OBJ, x + 3),
            ("wind strike aims at no tile", true, spell, NO_OBJ, x + 3),
            ("an item aims at no tile", true, SNOWBALL, tinderbox, x + 3),
            ("off the scene", true, SNOWBALL, NO_OBJ, x + 100),
            ("past the map", true, SNOWBALL, NO_OBJ, x + 0x4000),
        ];
        for (case, open, com, obj, tx) in refused {
            let mut client = player_at(engine, "ap_coord_t_refused", x, z);
            if open {
                open_snowball(engine, client.pid);
            }
            aim(&mut client, com, obj, tx, z);
            engine.harness_inputs();
            let player = engine.harness_player(client.pid).unwrap();
            assert_eq!(player.interaction.target_op, None, "{case}");
            assert_eq!(player.last_coord, None, "{case}");
        }

        let mut client = player_at(engine, "ap_coord_t_delayed", x, z);
        open_snowball(engine, client.pid);
        engine.harness_player_mut(client.pid).unwrap().state.delayed = true;
        aim(&mut client, SNOWBALL, NO_OBJ, x + 3, z);
        engine.harness_inputs();
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.interaction.target_op, None, "a delayed player");

        // A region of Lumbridge's spawn zone: its own tile is aimed at, the
        // empty slot east of it is not.
        let mut client = player_at(engine, "ap_coord_t_region", x, z);
        let uid = engine
            .harness_region_create(1, 1, 1)
            .expect("a free region");
        let source = CoordGrid::new(x & !7, 0, z & !7);
        engine
            .harness_region_set(uid, 0, 0, 0, source, 0)
            .expect("the zone copies");
        let inside = engine
            .harness_region_coord(uid, 0, (x & 7) as i32, (z & 7) as i32)
            .unwrap();
        engine.harness_telejump(client.pid, inside.x(), 0, inside.z());
        engine.cycle();
        open_snowball(engine, client.pid);
        let void = CoordGrid::new(inside.x() + 6, 0, inside.z());
        aim(&mut client, SNOWBALL, NO_OBJ, void.x(), void.z());
        engine.harness_inputs();
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.interaction.target_op, None, "a tile in no zone");
        let own = CoordGrid::new(inside.x() + 1, 0, inside.z());
        aim(&mut client, SNOWBALL, NO_OBJ, own.x(), own.z());
        engine.harness_inputs();
        assert!(
            aiming(engine, client.pid, SNOWBALL, own),
            "its own region's tile"
        );
    });
}

/// A tinderbox used on logs on the ground reaches `[opobju,_category_22]`,
/// which tries to light them.
fn item_on_ground_obj(engine: &mut Engine) {
    let tinderbox = obj(engine, "tinderbox");
    let logs = obj(engine, "logs");
    let (x, z) = FIRE_TILE;
    let mut client = player_at(engine, "op_t_obj", x, z);
    hold(engine, client.pid, 0, tinderbox);
    engine.harness_add_obj(x, 0, z, logs, 1);

    let mut p = ClientPacket::new();
    p.p2_alt1(tinderbox as i32)
        .p2_alt1(logs as i32)
        .p2(0)
        .p4_alt2(BACKPACK)
        .p1_alt1(0)
        .p2_alt2(z as i32)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpObjT, &p.finish());
    engine.harness_inputs();

    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.last_use_item, Some(tinderbox), "used item");
    assert_eq!(player.last_use_slot, Some(0), "used slot");
    assert_eq!(
        player.interaction.target_op,
        Some(ServerTriggerType::ApObjU as u8),
        "an item drives the U trigger"
    );
    assert!(
        matches!(player.interaction.target, Some(InteractionTarget::Obj { id, .. }) if id == logs),
        "targets the logs"
    );

    // And the content answers it, through the engine's own interaction.
    let lit = (0..5).any(|_| {
        engine.cycle();
        calls_for("mes")
            .iter()
            .any(|m| m == "You attempt to light the logs.")
    });
    assert!(
        lit,
        "[opobju,_category_22] never tried to light the logs; mes calls: {:?}",
        calls_for("mes")
    );
}

/// An item used on a loc drives `[aplocu]` / `[oplocu]`.
fn item_on_loc(engine: &mut Engine) {
    let tinderbox = obj(engine, "tinderbox");
    let mut client = player_at(engine, "op_t_loc", TILE.0, TILE.1);
    hold(engine, client.pid, 3, tinderbox);

    let mut p = ClientPacket::new();
    p.p2_alt2(tinderbox as i32)
        .p2_alt2(3)
        .p2(3450)
        .p4_alt2(BACKPACK)
        .p2_alt2(2552)
        .p1_alt1(0)
        .p2_alt1(1276);
    client.send(ClientProt::OpLocT, &p.finish());
    engine.harness_inputs();

    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.last_use_item, Some(tinderbox), "used item");
    assert_eq!(player.last_use_slot, Some(3), "used slot");
    assert_eq!(
        player.interaction.target_op,
        Some(ServerTriggerType::ApLocU as u8)
    );
    assert!(
        matches!(
            player.interaction.target,
            Some(InteractionTarget::Loc { id: 1276, .. })
        ),
        "targets the tree"
    );
}

/// A spell cast on an npc drives `[apnpct]` / `[opnpct]`, keyed on the whole
/// component hash -- not its low 16 bits, which every interface's component
/// 25 shares.
fn spell_on_npc(engine: &mut Engine) {
    // Find any npc and stand a few tiles from it.
    let (nid, x, z) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .map(|n| (n.npc.uid.nid(), n.npc.pathing.coord))
        .find(|(_, c)| c.y() == 0)
        .map(|(nid, c)| (nid, c.x(), c.z()))
        .expect("an npc on the ground floor");
    let mut client = player_at(engine, "op_t_npc", x + 2, z);
    assert!(
        engine
            .harness_player(client.pid)
            .unwrap()
            .build_area
            .npcs
            .contains(nid),
        "the npc is in the player's view"
    );

    // modern:wind_strike, the standard spellbook's first combat spell.
    let wind_strike = (192 << 16) | 25;
    let mut p = ClientPacket::new();
    p.p2_alt2(NO_OBJ)
        .p2_alt2(NO_OBJ)
        .p2_alt1(nid as i32)
        .p4_alt3(wind_strike)
        .p1_alt2(0);
    client.send(ClientProt::OpNpcT, &p.finish());
    engine.harness_inputs();

    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(
        player.interaction.target_op,
        Some(ServerTriggerType::ApNpcT as u8),
        "a spell drives the T trigger"
    );
    assert_eq!(
        player.interaction.target_subject_com,
        Some(wind_strike),
        "keyed on the spell's whole component hash"
    );
    assert_eq!(player.last_use_item, None, "a spell is not a used item");
}

/// An item used on another player drives `[applayeru]` / `[opplayeru]`,
/// keyed on the item, since a player has no type of its own.
fn item_on_player(engine: &mut Engine) {
    let tinderbox = obj(engine, "tinderbox");
    let (x, z) = TILE;
    let mut client = player_at(engine, "op_t_user", x, z);
    let other = engine
        .harness_login_returning("op_t_target")
        .expect("second login");
    engine.harness_telejump(other.pid, x + 1, 0, z);
    engine.cycle();
    hold(engine, client.pid, 0, tinderbox);
    assert!(
        engine
            .harness_player(client.pid)
            .unwrap()
            .build_area
            .players
            .contains(other.pid),
        "the other player is in view"
    );

    let mut p = ClientPacket::new();
    p.p2_alt2(other.pid as i32)
        .p2_alt1(0)
        .p4(BACKPACK)
        .p1_alt3(0)
        .p2_alt1(tinderbox as i32);
    client.send(ClientProt::OpPlayerT, &p.finish());
    engine.harness_inputs();

    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.last_use_item, Some(tinderbox));
    assert_eq!(
        player.interaction.target_op,
        Some(ServerTriggerType::ApPlayerU as u8)
    );
    assert_eq!(
        player.interaction.target_subject_com,
        Some(tinderbox as i32),
        "keyed on the used item"
    );
}

/// A packet naming an item the slot does not hold is dropped, as a stale or
/// forged click should be: no interaction, nothing recorded as used.
fn forged_item_is_dropped(engine: &mut Engine) {
    let tinderbox = obj(engine, "tinderbox");
    let logs = obj(engine, "logs");
    let mut client = player_at(engine, "op_t_forged", TILE.0, TILE.1);
    hold(engine, client.pid, 0, tinderbox);

    let mut p = ClientPacket::new();
    p.p2_alt2(logs as i32)
        .p2_alt2(0)
        .p2(3450)
        .p4_alt2(BACKPACK)
        .p2_alt2(2552)
        .p1_alt1(0)
        .p2_alt1(1276);
    client.send(ClientProt::OpLocT, &p.finish());
    engine.harness_inputs();

    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.interaction.target_op, None, "no interaction");
    assert_eq!(player.last_use_item, None, "nothing recorded as used");
}

/// A spell cannot claim an inventory item or target an unsupported entity.
fn a_forged_spell_source_is_dropped(engine: &mut Engine) {
    let tinderbox = obj(engine, "tinderbox");
    let wind_strike = (192 << 16) | 25;
    let on_tree = |source: i32, obj: i32| {
        let mut p = ClientPacket::new();
        p.p2_alt2(obj)
            .p2_alt2(0)
            .p2(3450)
            .p4_alt2(source)
            .p2_alt2(2552)
            .p1_alt1(0)
            .p2_alt1(1276);
        p.finish()
    };

    let mut client = player_at(engine, "op_t_stray_obj", TILE.0, TILE.1);
    hold(engine, client.pid, 0, tinderbox);
    client.send(ClientProt::OpLocT, &on_tree(wind_strike, tinderbox as i32));
    engine.harness_inputs();
    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(
        player.interaction.target_op, None,
        "a forged spell source does not start an interaction"
    );
    assert_eq!(player.last_use_item, None, "and no item is used");

    let mut client = player_at(engine, "op_t_no_obj", TILE.0, TILE.1);
    hold(engine, client.pid, 0, tinderbox);
    client.send(ClientProt::OpLocT, &on_tree(BACKPACK, NO_OBJ));
    engine.harness_inputs();
    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(
        player.interaction.target_op, None,
        "the backpack with no obj is an item use that holds nothing"
    );
}
