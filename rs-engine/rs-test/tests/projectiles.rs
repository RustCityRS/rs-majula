//! A projectile flies to the npc or the player its uid names.
//!
//! `projanim_npc` and `projanim_pl` take the target as a uid. The engine these scripts were
//! written for looks that uid up and never reads the active npc or player (LostCityRS/Engine-TS,
//! `src/engine/script/handlers/ServerOps.ts`, `PROJANIM_NPC` and `PROJANIM_PL`, the same on its
//! branches 225, 244, 245.2, 254, 274 and 289), so a script whose active npc is the one casting
//! can aim at another:
//!
//! - `~npc_to_npc_projectile(.npc_uid, ...)`: the active npc casts at the second npc, as
//!   Zooknock does at the jungle demon in Monkey Madness's last fight (`mm_demon.rs2`).
//! - `~player_projectile(coord, .coord, .uid, ...)`: the active player shoots the second
//!   player, as every ranged and magic attack on another player does (`pvp_ranged.rs2`,
//!   `pvp_magic.rs2`).
//!
//! Both failed with "Invalid uid: <target>, expected: <caster>". Each case calls the shared proc
//! (`skill_combat/scripts/projectile.rs2`) as those scripts do and reads the MAP_PROJANIM the
//! watching player is sent.
//!
//! From rev 530: the headless harness starts there. The two commands have one handler for every
//! revision.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_grid::CoordGrid;
use rs_protocol::network::game::zone_prot::ZoneProt;
use rs_test::runner;
use rs_ver::TypeId;
use rs_vm::state::ScriptArgument;
use rs_vm::subject::ScriptSubject;
use std::collections::HashMap;
use std::sync::Mutex;

/// The cases share one engine, so they take turns.
static TURN: Mutex<()> = Mutex::new(());

/// What a MAP_PROJANIM carries after its source tile.
struct Flight {
    dx: i8,
    dz: i8,
    /// An npc's index + 1; a player's -index - 1.
    target: i16,
    spotanim: u16,
    src_height: u8,
    dst_height: u8,
    start: u16,
    end: u16,
    peak: u8,
}

/// Whether the packets hold that MAP_PROJANIM. On the wire it is the zone prot's byte, the
/// source tile, then dx, dz, target, spotanim, the two heights, the two delays and the peak
/// (`MapProjAnim::encode`), inside the tick's zone update.
fn flew(packets: &[(u16, Vec<u8>)], flight: &Flight) -> bool {
    let mut tail = vec![flight.dx as u8, flight.dz as u8];
    tail.extend(flight.target.to_be_bytes());
    tail.extend(flight.spotanim.to_be_bytes());
    tail.extend([flight.src_height, flight.dst_height]);
    tail.extend(flight.start.to_be_bytes());
    tail.extend(flight.end.to_be_bytes());
    tail.push(flight.peak);
    packets.iter().any(|(_, body)| {
        (0..body.len()).any(|i| {
            body[i] == ZoneProt::MapProjAnim as u8
                && body.get(i + 2..i + 2 + tail.len()) == Some(&tail[..])
        })
    })
}

/// A spotanim this revision's cache has.
fn any_spotanim(engine: &Engine) -> u16 {
    (1..u16::MAX)
        .find(|&id| engine.cache.spotanims.get_by_id(TypeId::new(id)).is_some())
        .expect("a spotanim")
}

/// Runs `run` and gives the script errors it raised, each with its script and line.
///
/// The engine gathers script errors only while it has a database to send them to
/// (`ScriptEngine::script_error`), and a test engine has none: it is lent a sender for the call.
fn script_errors(engine: &mut Engine, run: impl FnOnce(&mut Engine)) -> Vec<String> {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let _ = engine.script_errors.take();
    engine.db_tx = Some(tx);
    run(engine);
    engine.db_tx = None;
    engine
        .script_errors
        .take()
        .into_iter()
        .map(|e| format!("{} at {} {}:{}", e.message, e.script, e.file, e.line))
        .collect()
}

fn ints(values: &[i32]) -> Option<Vec<ScriptArgument>> {
    Some(values.iter().map(|&v| ScriptArgument::Int(v)).collect())
}

fn delta(from: CoordGrid, to: CoordGrid) -> (i8, i8) {
    (
        (to.x() as i32 - from.x() as i32) as i8,
        (to.z() as i32 - from.z() as i32) as i8,
    )
}

/// A player in the game, standing on (x, z).
fn watcher(engine: &mut Engine, name: &str, x: u16, z: u16) -> HarnessClient {
    let client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    engine.harness_telejump(client.pid, x, 0, z);
    client
}

/// Two one-tile npcs on different tiles of one ground-level zone: (nid, nid).
fn two_npcs(engine: &Engine) -> (u16, u16) {
    let mut first: HashMap<(u16, u16), (u16, CoordGrid)> = HashMap::new();
    for npc in engine.npc_list.npcs.iter().flatten() {
        let (coord, nid) = (npc.npc.pathing.coord, npc.npc.uid.nid());
        if coord.y() != 0 || npc.npc.pathing.size != 1 {
            continue;
        }
        match first.get(&(coord.x() >> 3, coord.z() >> 3)) {
            Some(&(other, at)) if at != coord => return (other, nid),
            Some(_) => {}
            None => {
                first.insert((coord.x() >> 3, coord.z() >> 3), (nid, coord));
            }
        }
    }
    panic!("no two npcs share a zone");
}

/// `~npc_to_npc_projectile(.npc_uid, $spell, 25, 50, 25, 16, 21, 64, 5)`, Zooknock's cast: the
/// active npc is the caster and the second npc the target. The projectile leaves the caster's
/// tile for the target's, aimed at the target's index.
#[test]
fn an_npc_s_projectile_flies_to_the_npc_its_uid_names() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let (caster, target) = two_npcs(engine);
        let near = engine.get_npc(caster).unwrap().npc.pathing.coord;
        let mut client = watcher(engine, "projwatcher", near.x(), near.z());
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();

        let spotanim = any_spotanim(engine);
        let (caster_uid, from) = {
            let npc = &engine.get_npc(caster).unwrap().npc;
            (npc.uid, npc.pathing.coord)
        };
        let (target_uid, to) = {
            let npc = &engine.get_npc(target).unwrap().npc;
            (npc.uid, npc.pathing.coord)
        };
        let args = [
            target_uid.packed() as i32,
            i32::from(spotanim),
            25,
            50,
            25,
            16,
            21,
            64,
            5,
        ];
        let errors = script_errors(engine, |engine| {
            engine
                .run_script_by_name(
                    "[proc,npc_to_npc_projectile]",
                    Some(ScriptSubject::Npc(caster_uid)),
                    Some(ScriptSubject::Npc(target_uid)),
                    None,
                    None,
                    ints(&args),
                )
                .expect("the proc ran");
        });
        assert!(errors.is_empty(), "the cast failed: {errors:?}");

        engine.cycle();
        let (dx, dz) = delta(from, to);
        // The proc's flight time: its delay, its length, and its step for each tile.
        let end = 25 + 21 + from.distance(to) as u16 * 5;
        assert!(
            flew(
                &client.packets(),
                &Flight {
                    dx,
                    dz,
                    target: target as i16 + 1,
                    spotanim,
                    src_height: 25,
                    dst_height: 50,
                    start: 25,
                    end,
                    peak: 16,
                }
            ),
            "no projectile from {from:?} to npc {target} at {to:?}"
        );
    });
}

/// `~player_projectile(coord, .coord, .uid, <arrow>, 40, 36, 41, 15, 5, 11, 5)`, an arrow shot
/// at another player (`pvp_ranged.rs2`): the active player is the archer and the second player
/// the target. The projectile leaves the archer's tile for the target's, aimed at the target's
/// index.
#[test]
fn a_player_s_projectile_flies_to_the_player_its_uid_names() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        // Lumbridge castle's courtyard.
        let (from, to) = (CoordGrid::new(3222, 0, 3218), CoordGrid::new(3225, 0, 3220));
        let mut archer = watcher(engine, "projarcher", from.x(), from.z());
        let mut victim = watcher(engine, "projvictim", to.x(), to.z());
        for _ in 0..3 {
            engine.cycle();
        }
        archer.packets();
        victim.packets();

        let spotanim = any_spotanim(engine);
        let archer_uid = engine.harness_player(archer.pid).unwrap().uid;
        let victim_uid = engine.harness_player(victim.pid).unwrap().uid;
        let args = [
            from.packed() as i32,
            to.packed() as i32,
            victim_uid.packed() as i32,
            i32::from(spotanim),
            40,
            36,
            41,
            15,
            5,
            11,
            5,
        ];
        let errors = script_errors(engine, |engine| {
            engine
                .run_script_by_name(
                    "[proc,player_projectile]",
                    Some(ScriptSubject::Player(archer_uid)),
                    Some(ScriptSubject::Player(victim_uid)),
                    None,
                    None,
                    ints(&args),
                )
                .expect("the proc ran");
        });
        assert!(errors.is_empty(), "the shot failed: {errors:?}");

        engine.cycle();
        let (dx, dz) = delta(from, to);
        let end = 41 + 5 + from.distance(to) as u16 * 5;
        assert!(
            flew(
                &archer.packets(),
                &Flight {
                    dx,
                    dz,
                    target: -(victim.pid as i16) - 1,
                    spotanim,
                    src_height: 40,
                    dst_height: 36,
                    start: 41,
                    end,
                    peak: 15,
                }
            ),
            "no projectile from {from:?} to player {} at {to:?}",
            victim.pid
        );
    });
}
