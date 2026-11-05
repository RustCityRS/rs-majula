//! 289's npc hunting at 647: a Wilderness green dragon attacks a player
//! beside it.
//!
//! The green dragon's hunt is 289's `cowardly` (huntrange 4): a player hunt
//! that does not check whether the player is too strong in the Wilderness.
//! The mode is checked by name: hunt.pack's ids are the server's own (289's
//! `cowardly` was 404 after the 2011 cache's 397 hunts, and is 18 since
//! c7b4f69 dropped those). 2011's other green dragons (4677-4680) take 289's
//! green dragon config and its fight.
//!
//! 289's duckling hunts its duck (`duck_hunt`, general/configs/duckling.hunt):
//! every 11 ticks it looks for a duck_female in sight and takes it as an
//! opnpc3 target.

#![cfg(since_647)]

use rs_engine::Engine;
use rs_entity::InteractionTarget;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

/// `%npc_action_delay` (varn 5): the dragon's attack sets it.
const NPC_ACTION_DELAY: u16 = 5;

/// The npc of type `id` nearest `(x, z)`, within 12 tiles: its slot.
fn nearest(engine: &Engine, id: u16, x: u16, z: u16) -> Option<usize> {
    engine
        .npc_list
        .npcs
        .iter()
        .enumerate()
        .filter_map(|(nid, slot)| Some((nid, slot.as_ref()?)))
        .filter(|(_, a)| a.npc.uid.id() == id && a.npc.active)
        .map(|(nid, a)| {
            let c = a.npc.pathing.coord;
            (
                nid,
                (c.x() as i32 - x as i32)
                    .abs()
                    .max((c.z() as i32 - z as i32).abs()),
            )
        })
        .filter(|&(_, d)| d <= 12)
        .min_by_key(|&(_, d)| d)
        .map(|(nid, _)| nid)
}

/// Logs a player in two tiles south-west of the map's npc of type `id` near
/// `(x, z)` and cycles until that npc targets the player, or 40 ticks pass.
/// Returns whether it targeted the player, and whether its attack ran.
fn stand_beside(engine: &mut Engine, id: u16, x: u16, z: u16) -> (bool, bool) {
    engine.harness_reset();
    let mut client = engine.harness_login_returning("hunted").expect("login");
    // The harness restarts the clock near 0. The hunt and the dragon's script
    // both skip a player in combat within the last 8 ticks (%lastcombat,
    // %lastcombat_pvp), which a fresh clock reads as true: let it pass 8.
    for _ in 0..10 {
        engine.cycle();
    }
    let nid = nearest(engine, id, x, z).unwrap_or_else(|| panic!("no npc {id} near {x},{z}"));
    let size = engine.cache.npcs.get_by_id(id).map_or(1, |t| t.size.max(1)) as i32;
    let (mut targeted, mut attacked) = (false, false);
    for tick in 0..120 {
        // The client's keep-alive: 100 silent ticks would log the player out.
        if tick % 20 == 19 {
            client.send(ClientProt::NoTimeout, &[]);
        }
        // Follow the npc while it wanders, onto a walkable tile against its
        // west, south, east or north edge, until it attacks: a wandering npc
        // can stand where a fixed offset from it is a rock or a wall, and
        // then hunts a player it cannot reach.
        let c = engine.npc_list.npcs[nid]
            .as_ref()
            .unwrap()
            .npc
            .pathing
            .coord;
        if tick % 5 == 0 && !attacked {
            let (cx, cz) = (c.x() as i32, c.z() as i32);
            let edge = (0..size)
                .flat_map(|i| {
                    [
                        (cx - 1, cz + i),
                        (cx + i, cz - 1),
                        (cx + size, cz + i),
                        (cx + i, cz + size),
                    ]
                })
                .map(|(x, z)| (x as u16, z as u16))
                .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
                .expect("a walkable tile beside the npc");
            engine.harness_telejump(client.pid, edge.0, 0, edge.1);
        }
        engine.cycle();
        let npc = &engine.npc_list.npcs[nid].as_ref().unwrap().npc;
        if matches!(npc.interaction.target, Some(InteractionTarget::Player { pid }) if pid == client.pid)
        {
            targeted = true;
        }
        if npc.vars.get(NPC_ACTION_DELAY).as_int() > 0 {
            attacked = true;
        }
        if targeted && attacked {
            break;
        }
    }
    (targeted, attacked)
}

#[test]
fn a_wilderness_green_dragon_hunts_a_player_beside_it() {
    let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
    runner::on_loaded_engine(|engine| {
        let mode = engine.cache.npcs.get_by_id(941).and_then(|t| t.huntmode);
        let cowardly = engine.cache.hunts.debugnames.get("cowardly").copied();
        assert!(cowardly.is_some(), "hunt.pack names cowardly");
        assert_eq!(mode, cowardly, "green_dragon is cowardly");
        // The map's green dragon (941) at 2982,3618 (map square 46_56).
        let (targeted, attacked) = stand_beside(engine, 941, 2982, 3618);
        assert!(targeted, "the dragon hunted the player");
        assert!(attacked, "and attacked");
    });
}

/// The varbit `life_points` (7198): varp 1240, bits 1-15 (cache 786).
const LIFE_POINTS_VARP: u16 = 1240;

/// 2011's other green dragons hunt as 289's green dragon does, and fight as
/// it does, breath included: without their own trigger they fell to the
/// default npc attack (`[ai_opplayer2,_]`), melee alone. The player, behind
/// an anti-dragon shield and kept on 990 life points, stands beside the map's
/// green dragon 4679 until it breathes (one attack in four).
#[test]
fn a_remodelled_green_dragon_clone_hunts_and_breathes_fire() {
    let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let client = engine.harness_login_returning("hunted").expect("login");
        for _ in 0..10 {
            engine.cycle();
        }
        let worn = *engine.cache.invs.debugnames.get("worn").unwrap();
        let shield = *engine
            .cache
            .objs
            .debugnames
            .get("anti_dragon_shield")
            .unwrap();
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&worn)
            .expect("worn")
            .set(5, shield, 1);
        // The map's green dragon 4679 at 2973,3620 (map square 46_56).
        let nid = nearest(engine, 4679, 2973, 3620).expect("green_dragon_4679");
        rs_vm::ops::test::forget_calls();
        let (mut targeted, mut breathed) = (false, false);
        for tick in 0..200 {
            let c = engine.npc_list.npcs[nid]
                .as_ref()
                .unwrap()
                .npc
                .pathing
                .coord;
            if tick % 5 == 0 && !targeted {
                engine.harness_telejump(client.pid, c.x() - 2, 0, c.z() - 2);
            }
            let player = engine.harness_player_mut(client.pid).unwrap();
            let bit0 = player.vars.get(LIFE_POINTS_VARP).as_int() & 1;
            player
                .vars
                .set(LIFE_POINTS_VARP, VarValue::Int((990 << 1) | bit0));
            engine.cycle();
            let npc = &engine.npc_list.npcs[nid].as_ref().unwrap().npc;
            if matches!(npc.interaction.target, Some(InteractionTarget::Player { pid }) if pid == client.pid)
            {
                targeted = true;
            }
            if rs_vm::ops::test::calls_for("spotanim_npc")
                .iter()
                .any(|a| a == "2465,0,0")
            {
                breathed = true;
                break;
            }
        }
        assert!(targeted, "green_dragon_4679 hunted the player");
        assert!(breathed, "and breathed the remodelled dragons' fire (2465)");
    });
}

/// The map's duckling (2694) at 3142,3273 and duck (2693, 289's duck_female)
/// at 3143,3275, map square 49_51: within the hunt's rate (11 ticks), with a
/// player nearby to watch, the duckling targets the duck with opnpc3.
#[test]
fn a_duckling_hunts_its_duck() {
    let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let client = engine
            .harness_login_returning("duckwatcher")
            .expect("login");
        for _ in 0..10 {
            engine.cycle();
        }
        let duckling = nearest(engine, 2694, 3142, 3273).expect("duckling 2694");
        let duck = nearest(engine, 2693, 3143, 3275).expect("duck 2693");
        let mut target = None;
        for tick in 0..40 {
            let c = engine.npc_list.npcs[duckling]
                .as_ref()
                .unwrap()
                .npc
                .pathing
                .coord;
            if tick % 5 == 0 {
                engine.harness_telejump(client.pid, c.x() - 2, 0, c.z() - 2);
            }
            engine.cycle();
            let npc = &engine.npc_list.npcs[duckling].as_ref().unwrap().npc;
            if let Some(InteractionTarget::Npc { nid }) = npc.interaction.target {
                target = Some((nid as usize, npc.interaction.target_op));
                break;
            }
        }
        assert_eq!(
            target,
            Some((duck, Some(rs_pack::types::NpcMode::OpNpc3 as u8))),
            "the duckling's target"
        );
    });
}
