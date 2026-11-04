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

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::InteractionTarget;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
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
    p.ip2(tinderbox as i32)
        .ip2(logs as i32)
        .p2(0)
        .p4_alt2(BACKPACK)
        .p1_alt3(0)
        .p2_alt1(z as i32)
        .p2_alt1(x as i32);
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
        rs_vm::ops::test::calls_for("mes")
            .iter()
            .any(|m| m == "You attempt to light the logs.")
    });
    assert!(
        lit,
        "[opobju,_category_22] never tried to light the logs; mes calls: {:?}",
        rs_vm::ops::test::calls_for("mes")
    );
}

/// An item used on a loc drives `[aplocu]` / `[oplocu]`.
fn item_on_loc(engine: &mut Engine) {
    let tinderbox = obj(engine, "tinderbox");
    let mut client = player_at(engine, "op_t_loc", TILE.0, TILE.1);
    hold(engine, client.pid, 3, tinderbox);

    let mut p = ClientPacket::new();
    p.p2_alt1(tinderbox as i32)
        .p2_alt1(3)
        .p2(3450)
        .p4_alt2(BACKPACK)
        .p2_alt1(2552)
        .p1_alt3(0)
        .ip2(1276);
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
    p.p2_alt1(NO_OBJ)
        .p2_alt1(NO_OBJ)
        .ip2(nid as i32)
        .p4_alt1(wind_strike)
        .p1_alt1(0);
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
    p.p2_alt1(other.pid as i32)
        .ip2(0)
        .p4(BACKPACK)
        .p1_alt2(0)
        .ip2(tinderbox as i32);
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
    p.p2_alt1(logs as i32)
        .p2_alt1(0)
        .p2(3450)
        .p4_alt2(BACKPACK)
        .p2_alt1(2552)
        .p1_alt3(0)
        .ip2(1276);
    client.send(ClientProt::OpLocT, &p.finish());
    engine.harness_inputs();

    let player = engine.harness_player(client.pid).unwrap();
    assert_eq!(player.interaction.target_op, None, "no interaction");
    assert_eq!(player.last_use_item, None, "nothing recorded as used");
}
