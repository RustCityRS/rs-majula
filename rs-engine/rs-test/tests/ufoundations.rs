//! Unstable Foundations' start (docs/647-unstable-foundations.md, build plan stages 1 and 3).
//!
//! - Character creation's end (`[queue,character_created]`) sets varp 281 to 5, the start value
//!   of quest config 131 (`op3_counted_g2_g4_g4=1,281,5,1000`), not 1001.
//! - It puts the player in their own copy of the cellar, square 57_77, on local (30, 9): the
//!   2011 recordings' minimaps fit there (design note 3.5 #1).
//! - The plank, ground obj 15276, lies on local (35, 15) (3.5 #2).
//! - The Mentor plays (3.5 #6).
//! - Sir Vant (8850) stands on local (32, 16), the north doorway, and his squire on (33, 15), and
//!   neither wanders off (the 2011 minimaps show them there at 8.5 s and 80 s).
//! - In the dragon cutscene the player backs off to (28, 12) by the west wall, the squire says
//!   "Uh-oh!", walks south along x = 33, says "Help!" and goes to the ladder's foot, and in
//!   shot 5 the dragon's fireball burns him: he is gone a tick after it lands, his kit lies on
//!   (32, 13) and the ladder on (32, 12) is the burnt one; in shot 7 the player panics, seq 12270
//!   (refs GAPS 18, timed against 4KO2h2xPtdM).
//! - A region nobody is in is freed after 50 ticks; logging back in mid-tutorial builds a fresh one
//!   at the start tile (no source says where 2011 put them).

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::player::StaffModLevel;
use rs_grid::CoordGrid;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_protocol::network::game::zone_prot::ZoneProt;
use rs_test::runner;
use rs_test::talk::{converse, talk_to};
use rs_vm::subject::ScriptSubject;

/// Square 57_77's south-west tile.
const SQUARE: (u16, u16) = (57 * 64, 77 * 64);
const NONE: i32 = 0xFFFF;
const INVENTORY: u16 = 149;
const OPTIONS: u16 = 261;
const CONTROLS: u16 = 897;
const CUTSCENES: u16 = 899;
/// The cutscene camera's shot (cs2 2765, on interface 899).
const SHOT: u16 = 1007;
const TASK_SIDE: u16 = 1056;
const COMBAT_SIDE: u16 = 884;
/// The task system's pinned Task (cs2 3973 draws it).
const PINNED_TASK: u16 = 8576;
/// The task system popup's Task (cs2 3968).
const POPUP_TASK: u16 = 1425;
/// The side tab the yellow arrow flashes, + 1 (cs2 117).
const FLASH_SIDE: u16 = 3756;

fn varbit(engine: &Engine, client: &HarnessClient, id: u16) -> i32 {
    let bit = engine.cache.varbits.get_by_id(id).unwrap();
    let value = engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(bit.basevar)
        .as_int() as u32;
    let width = u32::from(bit.end_bit - bit.start_bit) + 1;
    ((value >> bit.start_bit) & ((1u64 << width) - 1) as u32) as i32
}

fn is_open(engine: &Engine, client: &HarnessClient, iface: u16) -> bool {
    engine
        .harness_player(client.pid)
        .unwrap()
        .is_interface_open(iface)
}

/// An arrow key on the Controls overlay's listener: IF_BUTTON1 on 897:0.
fn arrow_key(client: &mut HarnessClient) {
    let mut p = ClientPacket::new();
    p.p4(i32::from(CONTROLS) << 16).p2_alt3(NONE).p2_alt1(NONE);
    client.send(ClientProt::IfButton1, &p.finish());
}

fn varp(engine: &Engine, name: &str) -> u16 {
    *engine.cache.varps.debugnames.get(name).unwrap()
}

fn progress(engine: &Engine, client: &HarnessClient) -> i32 {
    let id = varp(engine, "learning_the_ropes");
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(id)
        .as_int()
}

/// The square 57_77 tile the player stands on a copy of, as local (x, z).
fn cellar_tile(engine: &Engine, client: &HarnessClient) -> Option<(u16, u16)> {
    let coord = engine.harness_player(client.pid)?.pathing.coord;
    let source = engine.regions.source(coord)?;
    Some((source.x() - SQUARE.0, source.z() - SQUARE.1))
}

fn shot(engine: &Engine, client: &HarnessClient) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .varcs
        .get(&SHOT)
        .copied()
        .unwrap_or(0)
}

/// The world tile of the first npc of a type, if one is in the world.
fn npc_tile(engine: &Engine, name: &str) -> Option<(u16, u16)> {
    let id = *engine.cache.npcs.debugnames.get(name).unwrap();
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == id)
        .map(|n| (n.npc.pathing.coord.x(), n.npc.pathing.coord.z()))
}

/// The fine coords an npc was last told to face (npc_facesquare): compare with
/// `CoordGrid::fine` of the tile (u16, so they wrap at these world coords).
fn npc_face(engine: &Engine, name: &str) -> Option<(u16, u16)> {
    let id = *engine.cache.npcs.debugnames.get(name).unwrap();
    let npc = &engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == id)?
        .npc;
    Some((npc.info.orientation_x?, npc.info.orientation_z?))
}

/// A world tile of the cellar copy as square 57_77's local (x, z).
fn local(engine: &Engine, (x, z): (u16, u16)) -> (u16, u16) {
    let source = engine
        .regions
        .source(CoordGrid::new(x, 0, z))
        .expect("in the cellar copy");
    (source.x() - SQUARE.0, source.z() - SQUARE.1)
}

/// The world tile of the cellar copy's local (0, 0): Sir Vant stands on local (32, 16) (as 8850,
/// or 8851 while the dragon cutscene changes him).
fn cellar_origin(engine: &Engine) -> (u16, u16) {
    let (x, z) = npc_tile(engine, "sir_vant_8850")
        .or_else(|| npc_tile(engine, "sir_vant_8851"))
        .expect("Sir Vant in the cellar");
    assert_eq!(local(engine, (x, z)), (32, 16));
    (x - 32, z - 16)
}

/// The ids of the locs whose south-west tile is (x, z), on level 0.
fn loc_ids(engine: &Engine, x: u16, z: u16) -> Vec<u16> {
    let Some(zone) = engine.zones.zone(x, 0, z) else {
        return Vec::new();
    };
    zone.locs
        .iter()
        .filter(|loc| {
            let coord = loc.world_coord(zone.coord);
            (coord.x(), coord.z()) == (x, z)
        })
        .map(|loc| loc.id())
        .collect()
}

/// The index of the first tick showing `shot`.
fn shots_start(ticks: &[(i32, Vec<(u16, Vec<u8>)>, Option<(u16, u16)>)], shot: i32) -> usize {
    ticks.iter().position(|(now, _, _)| *now == shot).unwrap()
}

/// Whether a tick's packets carry `text` (an NPC_INFO say is the bare string).
fn sent_text(packets: &[(u16, Vec<u8>)], text: &str) -> bool {
    let text = text.as_bytes();
    packets
        .iter()
        .any(|(_, body)| body.windows(text.len()).any(|w| w == text))
}

/// The voice-over clips a tick's packets play.
fn speech(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    packets
        .iter()
        .filter(|(op, _)| *op == ServerProt::VorbisSpeechSound as u16)
        .map(|(_, body)| u16::from_be_bytes([body[0], body[1]]))
        .collect()
}

fn create_character(engine: &mut Engine, client: &mut HarnessClient) -> Vec<(u16, Vec<u8>)> {
    let uid = engine.harness_player(client.pid).unwrap().uid;
    engine
        .run_script_by_name(
            "[queue,character_created]",
            Some(ScriptSubject::Player(uid)),
            None,
            Some(true),
            Some(true),
            None,
        )
        .expect("character_created");
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets()
}

/// The 647 HINT_ARROW packets among `packets`, as (slot, type, model): type 1 points at an npc,
/// 0 clears the slot; the model (or 0xFFFF) is drawn at the player's feet.
fn hints(packets: &[(u16, Vec<u8>)]) -> Vec<(u8, u8, u16)> {
    packets
        .iter()
        .filter(|(op, _)| *op == ServerProt::HintArrow as u16)
        .map(|(_, b)| (b[0] >> 5, b[0] & 0x1F, u16::from_be_bytes([b[10], b[11]])))
        .collect()
}

/// Whether `packets` run clientscript 115 (switchside) with side 1, the task system.
fn switched_to_task(packets: &[(u16, Vec<u8>)]) -> bool {
    packets.iter().any(|(op, b)| {
        *op == ServerProt::RunClientScript as u16
            && b.len() >= 8
            && b[b.len() - 8..] == [0, 0, 0, 1, 0, 0, 0, 115]
    })
}

/// The hint ring (cache 786 model 40497), as uf.constant's ^uf_hint_ring.
const HINT_RING: u16 = 40497;

#[test]
fn character_creation_starts_unstable_foundations_in_the_cellar() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login("uf_newcomer").expect("login");
        engine.cycle();
        client.packets();
        assert_eq!(
            progress(engine, &client),
            0,
            "a new account designs its character first"
        );
        // While the character is designed, Destiny plays and no region track is played or
        // unlocked (Qh_-AMJpVd0, GAPS 6): a new account's spawn square is not its music.
        for _ in 0..4 {
            engine.cycle();
        }
        let early = client.packets();
        let destiny = *engine.cache.midi_ids.get("destiny").unwrap();
        let song = varp(engine, "currentsong");
        assert_eq!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .vars
                .get(song)
                .as_int(),
            destiny as i32,
            "Destiny plays during character creation"
        );
        assert!(
            !early
                .iter()
                .any(|(op, body)| *op == ServerProt::MessageGame as u16
                    && body.windows(14).any(|w| w == b"You have unloc")),
            "no music unlocked during character creation"
        );

        let created = create_character(engine, &mut client);
        assert_eq!(
            hints(&created),
            vec![(0, 1, HINT_RING)],
            "a hint on Sir Vant, the ring at the player's feet"
        );
        // Creation sets quest config 131's start value, 5; the Controls overlay takes it to 6.
        assert_eq!(progress(engine, &client), 6, "the Controls overlay's stage");
        assert_eq!(
            cellar_tile(engine, &client),
            Some((30, 9)),
            "the start tile"
        );

        let player = engine.harness_player(client.pid).unwrap();
        let here = player.pathing.coord;
        let (x, z) = (here.x() + 5, here.z() + 6);
        let plank = *engine.cache.objs.debugnames.get("plank_15276").unwrap();
        assert!(
            engine
                .zones
                .find_obj(x, 0, z, plank, Some(player.uid.username37()))
                .is_some(),
            "the plank on local (35, 15)"
        );

        let mentor = *engine.cache.midi_ids.get("the mentor").unwrap();
        let song = varp(engine, "currentsong");
        assert_eq!(
            player.vars.get(song).as_int(),
            mentor as i32,
            "The Mentor plays"
        );

        // The side panel: the task system and options only (8jKH_7To46o @56), Rotate Your Camera
        // pinned, the Controls overlay up.
        assert!(is_open(engine, &client, TASK_SIDE), "the task system tab");
        assert!(is_open(engine, &client, OPTIONS), "the options tab");
        assert!(!is_open(engine, &client, INVENTORY), "no inventory tab yet");
        assert!(is_open(engine, &client, CONTROLS), "the Controls overlay");
        assert_eq!(
            varbit(engine, &client, PINNED_TASK),
            3522,
            "Rotate Your Camera"
        );
        assert_eq!(
            varbit(engine, &client, FLASH_SIDE),
            2,
            "the arrow at the task tab"
        );

        // An arrow key turns the camera: Rotate Your Camera is done.
        arrow_key(&mut client);
        let mut turned = Vec::new();
        for _ in 0..2 {
            engine.cycle();
            turned.extend(client.packets());
        }
        assert_eq!(
            hints(&turned),
            vec![(0, 0, 0)],
            "the ring goes with the camera turned"
        );
        assert_eq!(
            progress(engine, &client),
            7,
            "Rotate Your Camera done (cs2 3227)"
        );
        assert!(!is_open(engine, &client, CONTROLS), "the overlay closes");
        let popup = engine
            .harness_player(client.pid)
            .unwrap()
            .varcs
            .get(&POPUP_TASK)
            .copied();
        assert_eq!(popup, Some(3522), "TASK COMPLETE! Rotate Your Camera");
        assert_eq!(
            varbit(engine, &client, PINNED_TASK),
            3500,
            "Talk to Sir Vant"
        );
        assert_eq!(varbit(engine, &client, FLASH_SIDE), 0, "no arrow after");

        // Talk to Sir Vant: "Hello!" and his warning, each with its voiced take (s001, s002).
        let vant = *engine.cache.npcs.debugnames.get("sir_vant_8850").unwrap();
        let nid = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .find(|n| n.npc.uid.id() == vant)
            .map(|n| n.npc.uid.nid())
            .expect("Sir Vant in the cellar");
        talk_to(&mut client, nid);
        let talk = converse(engine, &mut client, &[], 80);
        assert!(talk.said("Hello!"), "{:?}", talk.texts);
        assert!(
            talk.said("Wait a moment - there's a dragon coming!"),
            "{:?}",
            talk.texts
        );
        let clips: Vec<u16> = talk
            .packets
            .iter()
            .filter(|(op, _)| *op == ServerProt::VorbisSpeechSound as u16)
            .map(|(_, body)| u16::from_be_bytes([body[0], body[1]]))
            .collect();
        assert_eq!(
            clips[..2],
            [1850, 770],
            "the male voice's Hello. and Sir Vant's line"
        );
        assert_eq!(
            progress(engine, &client),
            9,
            "talked; the dragon cutscene's stage"
        );

        // The dragon cutscene: the client's camera shots 1-10 in 2011's order and lengths
        // (4KO2h2xPtdM 16.98-53.72, GAPS 17b), then back to play. Shot 1 began during the talk.
        // Each tick from here: the shot after it, what it sent and where the squire stood.
        // The talk's op script is still suspended through the cutscene, so the player stays locked
        // facing Sir Vant (647: the client side-steps and backs away only under that lock).
        let pid = client.pid;
        let face = |engine: &Engine| engine.harness_player(pid).unwrap().info.face_entity;
        assert_eq!(
            face(engine),
            Some(nid),
            "facing Sir Vant as the cutscene opens"
        );
        let mut ticks: Vec<(i32, Vec<(u16, Vec<u8>)>, Option<(u16, u16)>)> = vec![(
            shot(engine, &client),
            Vec::new(),
            npc_tile(engine, "squire_8857"),
        )];
        let mut squire_face = vec![npc_face(engine, "squire_8857")];
        for _ in 0..80 {
            assert!(
                is_open(engine, &client, CUTSCENES),
                "interface 899 runs the camera"
            );
            engine.cycle();
            let packets = client.packets();
            let now = shot(engine, &client);
            ticks.push((now, packets, npc_tile(engine, "squire_8857")));
            squire_face.push(npc_face(engine, "squire_8857"));
            if now == 0 {
                break;
            }
        }
        let mut shots: Vec<(i32, u32)> = Vec::new();
        for (now, _, _) in &ticks {
            match shots.last_mut() {
                Some((last, n)) if last == now => *n += 1,
                _ => shots.push((*now, 1)),
            }
        }
        let order: Vec<i32> = shots.iter().map(|(shot, _)| *shot).collect();
        assert_eq!(order, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0], "{shots:?}");
        let lengths: Vec<u32> = shots[1..10].iter().map(|(_, n)| *n).collect();
        assert_eq!(lengths, vec![6, 3, 13, 6, 7, 4, 4, 9, 3], "{shots:?}");
        // 899 stays a tick after varc 1007 reaches 0, for its hook to reset the camera.
        assert!(
            is_open(engine, &client, CUTSCENES),
            "899 still open on the tick its hook runs"
        );
        engine.cycle();
        client.packets();
        assert!(
            !is_open(engine, &client, CUTSCENES),
            "the camera is the player's again"
        );

        // Shot 9: the goblin's "Ooh, shiny!" with its clip; shot 10: Sir Vant's s003 with his.
        let nine = shots_start(&ticks, 9);
        let shiny = ticks[nine..nine + 9]
            .iter()
            .position(|(_, packets, _)| sent_text(packets, "Ooh, shiny!"))
            .expect("Ooh, shiny! in shot 9");
        assert_eq!(
            shiny, 5,
            "Ooh, shiny! five ticks into shot 9 (4KO2: the cut @46.41, the overhead @49.5)"
        );
        assert_eq!(
            speech(&ticks[nine + shiny].1),
            vec![1215],
            "the goblin's take"
        );
        let ten = shots_start(&ticks, 10);
        assert!(
            sent_text(&ticks[ten].1, "I need some help over here!"),
            "s003 as shot 10 opens"
        );
        assert_eq!(speech(&ticks[ten].1), vec![882], "Sir Vant's s003 take");
        assert_eq!(
            progress(engine, &client),
            9,
            "still the cutscene's stage until s003 is read"
        );
        assert_eq!(face(engine), Some(nid), "still facing Sir Vant under s003");
        let read = converse(engine, &mut client, &[], 20);
        engine.cycle();
        let after: Vec<(u16, Vec<u8>)> = read.packets.into_iter().chain(client.packets()).collect();
        assert_eq!(
            hints(&after),
            vec![(0, 1, HINT_RING)],
            "the hint on Sir Vant again after s003"
        );
        assert_eq!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .varcs
                .get(&POPUP_TASK)
                .copied(),
            Some(3500),
            "TASK COMPLETE! Talk to Sir Vant"
        );
        assert_eq!(face(engine), None, "the lock ends with the talk's script");
        assert_eq!(progress(engine, &client), 10, "Assist Sir Vant");
        assert_eq!(
            varbit(engine, &client, PINNED_TASK),
            3501,
            "Assist Sir Vant pinned"
        );

        assert_eq!(
            cellar_tile(engine, &client),
            Some((28, 12)),
            "the player backed off to the west wall"
        );

        // The squire's part, timed against 4KO2 at 30 fps (GAPS 18): "Uh-oh!" in shot 1, "Help!" a
        // tick into shot 2, the fireball a tick before shot 5, "Aaargh!" a tick into it, then he
        // burns and his kit lies on (32, 13).
        let shot_at = |n: i32| ticks.iter().position(|(shot, _, _)| *shot == n).unwrap();
        let (two, five) = (shot_at(2), shot_at(5));
        let uh_oh = std::iter::once(&talk.packets)
            .chain(ticks[..two].iter().map(|(_, packets, _)| packets))
            .any(|packets| sent_text(packets, "Uh-oh!"));
        assert!(uh_oh, "Uh-oh! in shot 1");
        let early: Vec<u16> = std::iter::once(&talk.packets)
            .chain(ticks[..two].iter().map(|(_, packets, _)| packets))
            .flat_map(|packets| speech(packets))
            .collect();
        assert_eq!(early, vec![1850, 770, 1319], "then his Uh-oh take");
        assert!(
            sent_text(&ticks[two + 1].1, "Help!"),
            "Help! a tick into shot 2"
        );
        assert_eq!(speech(&ticks[two + 1].1), vec![1284], "his Help! take");
        assert_eq!(
            ticks[two].2.map(|tile| local(engine, tile)),
            Some((33, 13)),
            "south along x = 33 by shot 2"
        );
        assert_eq!(
            ticks[five].2.map(|tile| local(engine, tile)),
            Some((32, 13)),
            "at the ladder's foot in shot 5"
        );
        // MAP_PROJANIM (zone prot 14) rides in the tick's UPDATE_ZONE_PARTIAL_ENCLOSED: its spotanim
        // is the u16 after the coord, dx, dz and target.
        let fire = |packets: &[(u16, Vec<u8>)]| {
            packets
                .iter()
                .filter(|(op, _)| *op == ServerProt::UpdateZonePartialEnclosed as u16)
                .any(|(_, body)| {
                    (0..body.len()).any(|i| {
                        body[i] == ZoneProt::MapProjAnim as u8
                            && body.get(i + 6..i + 8) == Some(&2148u16.to_be_bytes()[..])
                    })
                })
        };
        let fired: Vec<usize> = (0..ticks.len()).filter(|&i| fire(&ticks[i].1)).collect();
        assert_eq!(
            fired,
            vec![five - 1],
            "the fireball leaves a tick before shot 5"
        );
        // Shot 5 opens on him facing the fire, the doorway (32, 17), not Sir Vant on (32, 16)
        // behind it (4KO2 @33.9; in-client ufdragon17 he faced away till his "Aaargh!").
        let origin = cellar_origin(engine);
        assert_eq!(
            squire_face[five],
            Some((
                CoordGrid::fine(origin.0 + 32, 1),
                CoordGrid::fine(origin.1 + 17, 1)
            )),
            "the squire faces the fire as shot 5 opens"
        );
        // Seq 12226 (raise the shield and vanish, 0.56 s; NPC_INFO's anim block, as Sir Vant's
        // swing below) with his "Aaargh!" as the fire lands, not as shot 5 opens: there it hid him
        // before the fire came (in-client ufdragon18).
        let shield = [0x2F, 0xC2u8.wrapping_add(128)].repeat(4);
        let hides = |packets: &[(u16, Vec<u8>)]| {
            packets.iter().any(|(op, body)| {
                *op == ServerProt::NpcInfo as u16 && body.windows(8).any(|w| w == shield)
            })
        };
        assert!(
            !hides(&ticks[five].1) && hides(&ticks[five + 1].1),
            "the squire's seq 12226 a tick into shot 5"
        );
        assert!(
            sent_text(&ticks[five + 1].1, "Aaargh!"),
            "Aaargh! a tick into shot 5"
        );
        assert_eq!(speech(&ticks[five + 1].1), vec![1292], "his scream");
        assert!(ticks[five + 2].2.is_some(), "he burns a tick later");
        assert!(ticks[five + 3].2.is_none(), "and is gone the tick after");

        // Shot 7: the player's panic, seq 12270, from the shot's first tick (PLAYER_INFO's anim
        // block carries it four times).
        let panic = 12270u16.to_be_bytes().repeat(4);
        let seven = shot_at(7);
        assert!(
            ticks[seven]
                .1
                .iter()
                .any(|(op, body)| *op == ServerProt::PlayerInfo as u16
                    && body.windows(8).any(|w| w == panic)),
            "the player's panic, seq 12270, as shot 7 opens"
        );

        // Shot 6: Sir Vant's swing, seq 12232, from the shot's first tick (NPC_INFO's anim block:
        // four ids, each big-endian with 128 added to its low byte).
        let swing = [0x2F, 0xC8u8.wrapping_add(128)].repeat(4);
        let six = shot_at(6);
        assert!(
            ticks[six]
                .1
                .iter()
                .any(|(op, body)| *op == ServerProt::NpcInfo as u16
                    && body.windows(8).any(|w| w == swing)),
            "Sir Vant's swing, seq 12232, as shot 6 opens"
        );

        let origin = cellar_origin(engine);
        let at = |x: u16, z: u16| (origin.0 + x, origin.1 + z);
        let receiver = engine.harness_player(client.pid).unwrap().uid.username37();
        for name in ["bronze_platebody", "bronze_sq_shield", "bronze_sword"] {
            let id = *engine.cache.objs.debugnames.get(name).unwrap();
            let (x, z) = at(32, 13);
            assert!(
                engine.zones.find_obj(x, 0, z, id, Some(receiver)).is_some(),
                "{name} on (32, 13)"
            );
        }
        let burnt = *engine.cache.locs.debugnames.get("ladder_45786").unwrap();
        let (x, z) = at(32, 12);
        assert!(
            loc_ids(engine, x, z).contains(&burnt),
            "the ladder burnt: {:?}",
            loc_ids(engine, x, z)
        );

        let mut later = Vec::new();
        for _ in 0..60 {
            engine.cycle();
            later.extend(client.packets());
        }
        // The goblin mutters as it rummages (a 1-in-8 chance a tick: none in 60 ticks is 0.03%).
        assert!(
            ["Heh heh heh.", "Mine, all mine!", "Ooh!", "Me get rich!"]
                .iter()
                .any(|line| sent_text(&later, line)),
            "the goblin talks to itself at the bag"
        );
        assert_eq!(
            npc_tile(engine, "sir_vant_8850").map(|tile| local(engine, tile)),
            Some((32, 16)),
            "Sir Vant keeps his tile"
        );
        assert_eq!(npc_tile(engine, "squire_8857"), None, "the squire is gone");
        assert_eq!(
            npc_tile(engine, "goblin_8862").map(|tile| local(engine, tile)),
            Some((34, 13)),
            "the goblin at Sir Vant's bag"
        );
        assert_eq!(
            npc_tile(engine, "dragon_8858").map(|tile| local(engine, tile)),
            Some((30, 18)),
            "the dragon stays at the wall over Sir Vant"
        );
    });
}

fn cheat(client: &mut HarnessClient, input: &str) {
    let mut p = ClientPacket::new();
    p.p1(0).p1(0).pjstr(input);
    client.send(ClientProt::ClientCheat, &p.finish());
}

/// The index of the npc of a type nearest the first harness player (the static world can hold
/// the same type as a copy, Explorer Jack's house).
fn npc_nid(engine: &Engine, name: &str) -> u16 {
    let id = *engine.cache.npcs.debugnames.get(name).unwrap();
    let me = engine
        .player_list
        .players
        .iter()
        .flatten()
        .next()
        .map(|p| p.player.pathing.coord);
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == id)
        .min_by_key(|n| {
            me.map(|c| {
                let d = n.npc.pathing.coord;
                (i32::from(d.x()) - i32::from(c.x())).abs()
                    + (i32::from(d.z()) - i32::from(c.z())).abs()
            })
            .unwrap_or(0)
        })
        .map(|n| n.npc.uid.nid())
        .unwrap_or_else(|| panic!("{name} in the world"))
}

fn obj_id(engine: &Engine, name: &str) -> u16 {
    *engine.cache.objs.debugnames.get(name).unwrap()
}

fn cycles(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<(u16, Vec<u8>)> {
    let mut out = Vec::new();
    for tick in 0..n {
        // A client says it is still there; the harness drops one silent for too long.
        if tick % 10 == 0 {
            client.send(ClientProt::NoTimeout, &[]);
        }
        engine.cycle();
        out.extend(client.packets());
    }
    out
}

/// Take (OPOBJ3) on a ground obj at world (x, z).
fn take(client: &mut HarnessClient, obj: u16, x: u16, z: u16) {
    let mut p = ClientPacket::new();
    p.p2_alt1(i32::from(obj))
        .p2_alt2(i32::from(z))
        .p1(0)
        .p2(i32::from(x));
    client.send(ClientProt::OpObj3, &p.finish());
}

/// Wear / Wield (op 2) on a backpack slot.
fn wear(client: &mut HarnessClient, obj: u16, slot: u16) {
    let mut p = ClientPacket::new();
    p.p4(i32::from(INVENTORY) << 16)
        .p2_alt3(i32::from(obj))
        .p2_alt1(i32::from(slot));
    client.send(ClientProt::IfButton2, &p.finish());
}

/// Op 1 on a loc at world (x, z).
fn oploc1(client: &mut HarnessClient, loc: u16, x: u16, z: u16) {
    let mut p = ClientPacket::new();
    p.p2_alt1(i32::from(z))
        .p2_alt1(i32::from(loc))
        .p1(0)
        .p2_alt2(i32::from(x));
    client.send(ClientProt::OpLoc1, &p.finish());
}

/// Op 1 on an npc (OPNPC1).
fn opnpc1(client: &mut HarnessClient, nid: u16) {
    let mut p = ClientPacket::new();
    p.p1_alt3(0).p2(i32::from(nid));
    client.send(ClientProt::OpNpc1, &p.finish());
}

/// A backpack item used on a loc at world (x, z).
fn use_on_loc(client: &mut HarnessClient, obj: u16, slot: u16, loc: u16, x: u16, z: u16) {
    let mut p = ClientPacket::new();
    p.p2_alt2(i32::from(obj))
        .p2_alt2(i32::from(slot))
        .p2(i32::from(z))
        .p4_alt2(i32::from(INVENTORY) << 16)
        .p2_alt2(i32::from(x))
        .p1_alt1(0)
        .p2_alt1(i32::from(loc));
    client.send(ClientProt::OpLocT, &p.finish());
}

fn slot_of(engine: &Engine, client: &HarnessClient, obj: u16) -> Option<u16> {
    let player = engine.harness_player(client.pid).unwrap();
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let inv = player.invs.get(&inv)?;
    (0..28u16).find(|&slot| inv.get(slot).map(|o| o.obj) == Some(obj))
}

/// A fresh character in the cellar, put at step `stage` of varp 281 (::ufstage rebuilds the
/// cellar as a login at that step would).
fn at_stage(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login(name).expect("login");
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .staff_mod_level = StaffModLevel::Maintainer;
    engine.cycle();
    client.packets();
    create_character(engine, &mut client);
    cheat(&mut client, &format!("~ufstage {stage}"));
    // The creation's cellar, left empty, is freed after 50 ticks: then its Sir Vant and the rest
    // are gone, and npc lookups find this cellar's.
    cycles(engine, &mut client, 56);
    assert_eq!(progress(engine, &client), stage);
    // Back to a player's level: the engine counts isolated (staff) players as they log in, and
    // a level raised after it would be taken off that count at logout.
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .staff_mod_level = StaffModLevel::Normal;
    client
}

/// Build plan stage 4: Sir Vant's s004-s005, the squire's gear, Equip Yourself, the goblin
/// (4KO2h2xPtdM @60-96, refs GAPS 19).
#[test]
fn the_squires_gear_and_the_goblin() {
    runner::on_loaded_engine(|engine| {
        let mut client = at_stage(engine, "uf_gear", 10);
        let origin = cellar_origin(engine);
        let at = |x: u16, z: u16| (origin.0 + x, origin.1 + z);

        talk_to(&mut client, npc_nid(engine, "sir_vant_8850"));
        let talk = converse(engine, &mut client, &[], 200);
        assert!(talk.said(
            "Thank goodness you're here! My squire's dead, and there's a goblin going through my bag!"
        ));
        assert!(talk.said(
            "I can't turn my back on this dragon. I need you to take my squire's equipment and deal with the goblin!"
        ));
        assert_eq!(speech(&talk.packets), vec![887, 807]);
        assert_eq!(progress(engine, &client), 15, "Gather the Squire's Gear");
        let mut later = talk.packets.clone();
        later.extend(cycles(engine, &mut client, 4));
        assert_eq!(
            hints(&later),
            vec![(0, 2, HINT_RING)],
            "the hint on the gear, the ring"
        );

        // The gear: the inventory, its arrow and TASK COMPLETE come with the third piece.
        let (gx, gz) = at(32, 13);
        let kit =
            ["bronze_platebody", "bronze_sq_shield", "bronze_sword"].map(|n| obj_id(engine, n));
        for (i, obj) in kit.iter().enumerate() {
            take(&mut client, *obj, gx, gz);
            cycles(engine, &mut client, 12);
            if i < 2 {
                assert_eq!(progress(engine, &client), 15);
                assert!(!is_open(engine, &client, INVENTORY), "no inventory yet");
            }
        }
        assert_eq!(progress(engine, &client), 20, "Equip Yourself");
        assert!(is_open(engine, &client, INVENTORY), "the inventory tab");
        assert_eq!(
            varbit(engine, &client, FLASH_SIDE),
            5,
            "the arrow at the inventory tab"
        );

        // Not yet armed: Sir Vant stops the attack.
        let goblin = npc_nid(engine, "goblin_8862");
        opnpc1(&mut client, goblin);
        let stopped = converse(engine, &mut client, &[], 60);
        assert!(stopped.said(
            "Wait, you're not properly armed. You should equip all the kit you've picked up."
        ));

        let mut wearing = Vec::new();
        for (i, obj) in kit.iter().enumerate() {
            let slot = slot_of(engine, &client, *obj).expect("in the backpack");
            wear(&mut client, *obj, slot);
            wearing.extend(cycles(engine, &mut client, 2));
            if i < 2 {
                assert_eq!(progress(engine, &client), 25 + 5 * i as i32);
                // Combat comes with the sword in hand (4KO2h2xPtdM @82.00), worn last here.
                assert!(!is_open(engine, &client, COMBAT_SIDE), "no combat tab yet");
            }
        }
        cycles(engine, &mut client, 2);
        assert_eq!(progress(engine, &client), 35, "Kill the Goblin");
        assert!(is_open(engine, &client, COMBAT_SIDE), "the combat tab");
        assert!(is_open(engine, &client, 387), "the worn equipment tab");
        assert_eq!(
            varbit(engine, &client, FLASH_SIDE),
            6,
            "the arrow at worn equipment"
        );
        // The ring moves to the goblin as the third piece goes on (the equip runs in a script
        // with no region selected: ufrest13's "no active_region").
        assert!(
            hints(&wearing).contains(&(0, 1, HINT_RING)),
            "{:?}",
            hints(&wearing)
        );

        // The fight, to the goblin's end and Sir Vant's s007.
        opnpc1(&mut client, goblin);
        let mut packets = Vec::new();
        for tick in 0..900 {
            if tick % 20 == 0 {
                client.send(ClientProt::NoTimeout, &[]);
            }
            engine.cycle();
            packets.extend(client.packets());
            if progress(engine, &client) == 45 {
                break;
            }
        }
        packets.extend(cycles(engine, &mut client, 2));
        assert_eq!(progress(engine, &client), 45, "Find Out Sir Vant's Plan");
        assert_eq!(npc_tile(engine, "goblin_8862"), None, "the goblin is dead");
        assert!(sent_text(
            &packets,
            "Thank you! Now to deal with the dragon."
        ));
        assert!(speech(&packets).contains(&730));
    });
}

/// Build plan stages 5-6: the plan (s009-s012), the quest list tab, the meat, the wall and the
/// ramp.
#[test]
fn sir_vants_plan_the_meat_and_the_ramp() {
    runner::on_loaded_engine(|engine| {
        let mut client = at_stage(engine, "uf_plan", 45);
        let origin = cellar_origin(engine);
        let at = |x: u16, z: u16| (origin.0 + x, origin.1 + z);

        talk_to(&mut client, npc_nid(engine, "sir_vant_8850"));
        let plan = converse(engine, &mut client, &[], 300);
        for line in [
            "This dragon's a tougher customer than I'd expected, but I know a way we can defeat it.",
            "We're going to lure the dragon there and then collapse the roof on top of it.",
            "The dragon will eat it and fall asleep.",
            "Once you've got the drugged meat you'll need to climb over the wall next to me.",
        ] {
            assert!(plan.said(line), "{line}");
        }
        assert_eq!(speech(&plan.packets), vec![732, 943, 806, 726]);
        assert_eq!(progress(engine, &client), 50, "Check your Quest Journal");
        assert!(is_open(engine, &client, 190), "the quest list tab");
        assert_eq!(varbit(engine, &client, FLASH_SIDE), 4);

        // The journal (quest list row 131), the world map (548:179, then its close 755:44) and
        // the compass (548:177 "Face North").
        let button = |client: &mut HarnessClient, iface: i32, com: i32, slot: i32| {
            let mut p = ClientPacket::new();
            p.p4((iface << 16) | com).p2_alt3(NONE).p2_alt1(slot);
            client.send(ClientProt::IfButton1, &p.finish());
        };
        // The quest list tab clicked at its arrow (toplevel:stone3, 548:131): the arrow stops
        // (4KO2h2xPtdM @118-120; worn equipment's @85.5-86.5).
        button(&mut client, 548, 131, NONE);
        cycles(engine, &mut client, 2);
        assert_eq!(varbit(engine, &client, FLASH_SIDE), 0, "the tab clicked");
        button(&mut client, 190, 18, 131);
        let journal = cycles(engine, &mut client, 3);
        assert_eq!(progress(engine, &client), 60, "Check your World Map");
        assert_eq!(
            varbit(engine, &client, FLASH_SIDE),
            19,
            "the arrow at the globe"
        );
        assert!(
            switched_to_task(&journal),
            "a new Task brings the panel to the task system (4KO2h2xPtdM @83.75, @103.75)"
        );
        engine.harness_close_modal(client.pid);
        button(&mut client, 548, 179, NONE);
        let map = cycles(engine, &mut client, 3);
        // The map opens on the cellar the copy was made from (square 57_77, the "Tutorial
        // Cellar" areas), not the copy's own tiles, with the arrow at its close button.
        let here = engine
            .harness_player(client.pid)
            .unwrap()
            .varcs
            .get(&622)
            .copied()
            .unwrap_or(-1);
        let (hx, hz) = ((here >> 14) & 0x3FFF, here & 0x3FFF);
        assert!(
            (57 * 64..58 * 64).contains(&hx) && (77 * 64..78 * 64).contains(&hz),
            "varc 622 on square 57_77, not the copy: {hx},{hz}"
        );
        let closearrow = (755 << 16) | 45;
        assert!(
            map.iter().any(|(op, b)| *op == ServerProt::IfSetHide as u16
                && b.len() >= 4
                && b[..4]
                    == [
                        (closearrow >> 16) as u8,
                        (closearrow >> 24) as u8,
                        closearrow as u8,
                        (closearrow >> 8) as u8
                    ]),
            "the arrow at the world map's close button (755:45)"
        );
        button(&mut client, 755, 44, NONE);
        cycles(engine, &mut client, 3);
        assert_eq!(progress(engine, &client), 62, "Realign your Minimap");
        button(&mut client, 548, 177, NONE);
        cycles(engine, &mut client, 3);
        assert_eq!(progress(engine, &client), 65, "Climb the Wall");

        // The bag: one parcel of meat.
        let bag = *engine.cache.locs.debugnames.get("sir_vant_s_bag").unwrap();
        let (bx, bz) = at(35, 13);
        oploc1(&mut client, bag, bx, bz);
        let search = converse(engine, &mut client, &[], 60);
        assert!(search.said(
            "The bag contains several parcels of meat, all wrapped up in paper. You take one."
        ));
        let meat = obj_id(engine, "drugged_meat");
        assert!(slot_of(engine, &client, meat).is_some(), "the meat");

        // The wall (Climb the Wall at 65: the quest journal, world map and minimap steps between
        // are their own stage).
        let ropes = varp(engine, "learning_the_ropes");
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .vars
            .set(ropes, VarValue::Int(65));
        cycles(engine, &mut client, 2);
        let wall = *engine.cache.locs.debugnames.get("low_wall_45603").unwrap();
        let (wx, wz) = at(35, 16);
        oploc1(&mut client, wall, wx, wz);
        let high = converse(engine, &mut client, &[], 60);
        assert!(high.said("Even this low section of the wall is too high for you to climb over."));
        assert_eq!(progress(engine, &client), 70, "Make a Ramp");

        let plank = obj_id(engine, "plank_15276");
        let (px, pz) = at(35, 15);
        take(&mut client, plank, px, pz);
        cycles(engine, &mut client, 12);
        let slot = slot_of(engine, &client, plank).expect("the plank");
        use_on_loc(&mut client, plank, slot, wall, wx, wz);
        let ramp = converse(engine, &mut client, &[], 60);
        assert!(ramp.said("You use the plank to construct a crude ramp."));
        assert_eq!(progress(engine, &client), 75, "Climb the Wall, the ramp up");
        assert!(slot_of(engine, &client, plank).is_none());
    });
}

/// The cellar copy's local (0, 0), from the player's own tile.
fn player_origin(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let coord = engine.harness_player(client.pid).unwrap().pathing.coord;
    let (lx, lz) = cellar_tile(engine, client).expect("in the cellar");
    (coord.x() - lx, coord.z() - lz)
}

fn give(engine: &mut Engine, client: &HarnessClient, slot: u16, obj: u16, num: u32) {
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap()
        .set(slot, obj, num);
}

fn loc_id(engine: &Engine, name: &str) -> u16 {
    *engine.cache.locs.debugnames.get(name).unwrap()
}

fn varbit_named(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let id = *engine.cache.varbits.debugnames.get(name).unwrap();
    varbit(engine, client, id)
}

/// Op 1 (the held item's first op) on a backpack slot.
fn held1(client: &mut HarnessClient, obj: u16, slot: u16) {
    let mut p = ClientPacket::new();
    p.p4(i32::from(INVENTORY) << 16)
        .p2_alt3(i32::from(obj))
        .p2_alt1(i32::from(slot));
    client.send(ClientProt::IfButton1, &p.finish());
}

/// A backpack item used on another (IF_BUTTONT).
fn use_on_item(client: &mut HarnessClient, used: (u16, u16), target: (u16, u16)) {
    let backpack = i32::from(INVENTORY) << 16;
    let mut p = ClientPacket::new();
    p.p4(backpack)
        .p2_alt1(i32::from(target.1))
        .p2_alt1(i32::from(used.1))
        .p2_alt2(i32::from(used.0))
        .p4_alt1(backpack)
        .p2(i32::from(target.0));
    client.send(ClientProt::IfButtonT, &p.finish());
}

/// A backpack item used on an npc (OPNPCT).
fn use_on_npc(client: &mut HarnessClient, obj: u16, slot: u16, nid: u16) {
    let mut p = ClientPacket::new();
    p.p2_alt2(i32::from(obj))
        .p2_alt2(i32::from(slot))
        .p2_alt1(i32::from(nid))
        .p4_alt3(i32::from(INVENTORY) << 16)
        .p1_alt2(0);
    client.send(ClientProt::OpNpcT, &p.finish());
}

/// Cycles until the conversation's boxes stop, answering them, then waits out a cutscene's
/// silence and answers what follows, up to `max` ticks in all.
fn play_out(
    engine: &mut Engine,
    client: &mut HarnessClient,
    choices: &[&str],
    max: usize,
) -> (Vec<String>, Vec<(u16, Vec<u8>)>) {
    let mut texts = Vec::new();
    let mut packets = Vec::new();
    let mut spent = 0;
    while spent < max {
        match engine.harness_player(client.pid) {
            Some(_) => {}
            None => panic!(
                "player gone after {spent} ticks; texts {texts:?}; messages {:?}",
                HarnessClient::game_messages(&packets)
            ),
        }
        let talk = converse(engine, client, choices, 30);
        let quiet = talk.texts.is_empty();
        texts.extend(talk.texts);
        packets.extend(talk.packets);
        spent += 30;
        if quiet {
            let more = cycles(engine, client, 10);
            texts.extend(rs_test::talk::settexts(&more).into_iter().map(|(_, t)| t));
            packets.extend(more);
            spent += 10;
            if !is_open(engine, client, CUTSCENES) && spent > 60 {
                break;
            }
        }
    }
    (texts, packets)
}

fn said(texts: &[String], needle: &str) -> bool {
    let joined = texts.join(" ").replace("<br>", " ");
    joined.contains(needle)
}

/// Build plan stage 7: the meat in the bowl and the sleep cutscene (4KO2h2xPtdM @190-237).
#[test]
fn the_dragon_is_put_to_sleep() {
    runner::on_loaded_engine(|engine| {
        let mut client = at_stage(engine, "uf_sleep", 80);
        let origin = player_origin(engine, &client);
        let meat = obj_id(engine, "drugged_meat");
        give(engine, &client, 0, meat, 1);
        engine.harness_telejump(client.pid, origin.0 + 17, 0, origin.1 + 37);
        cycles(engine, &mut client, 3);
        use_on_loc(
            &mut client,
            meat,
            0,
            loc_id(engine, "loc_45795"),
            origin.0 + 17,
            origin.1 + 35,
        );
        let (texts, packets) = play_out(engine, &mut client, &[], 260);
        assert!(said(&texts, "It's smelled the meat! Hide!"), "{texts:?}");
        assert!(
            said(
                &texts,
                "Well done. Come out of hiding and I'll tell you what we're doing next."
            ),
            "{texts:?}"
        );
        assert!(sent_text(&packets, "Grub's up! Shoo!"));
        // 2011's shots (4KO2h2xPtdM, cuts @202.67, 206.27, 211.04, 215.35, 216.45): clientscript
        // 2762's 101 to 105, the fade 106, then 107. Varc 1007 by VARC_SMALL,
        // `p1_alt2 value, p2_alt1 varc`.
        let mut sent_shots: Vec<i32> = packets
            .iter()
            .filter(|(op, p)| *op == ServerProt::VarcSmall as u16 && p.len() >= 3)
            .filter(|(_, p)| u16::from_le_bytes([p[1], p[2]]) == SHOT)
            .map(|(_, p)| 0u8.wrapping_sub(p[0]) as i8 as i32)
            .collect();
        sent_shots.dedup();
        assert_eq!(sent_shots, vec![101, 102, 103, 104, 105, 106, 107, 0]);
        let clips = speech(&packets);
        for clip in [715, 850, 958] {
            assert!(clips.contains(&clip), "clip {clip} in {clips:?}");
        }
        assert_eq!(progress(engine, &client), 93, "Talk to Sir Vant Again");
        assert_eq!(
            varbit_named(engine, &client, "varplayerbit_6487"),
            1,
            "the meat in the bowl"
        );
        assert_eq!(
            npc_tile(engine, "dragon_8861").map(|t| local(engine, t)),
            Some((18, 33)),
            "the dragon asleep on its nest"
        );
        assert_eq!(
            npc_tile(engine, "sir_vant_8850").map(|t| local(engine, t)),
            Some((26, 30)),
            "Sir Vant by the pillars"
        );
        assert!(
            !is_open(engine, &client, CUTSCENES),
            "the camera is the player's again"
        );
        // The minimap black for the cutscene (4KO2h2xPtdM @203-244.81), and back after it.
        let toggles: Vec<u8> = packets
            .iter()
            .filter(|(op, _)| *op == ServerProt::MinimapToggle as u16)
            .map(|(_, b)| b[0])
            .collect();
        assert_eq!(toggles.first(), Some(&2), "{toggles:?}");
        assert_eq!(toggles.last(), Some(&0), "{toggles:?}");
        // In the hole, the recess in the wall south of the nest (4KO2h2xPtdM @245.4), from where
        // talking to Sir Vant walks on to the pillars.
        assert_eq!(cellar_tile(engine, &client), Some((20, 29)), "in the hole");
        talk_to(&mut client, npc_nid(engine, "sir_vant_8850"));
        let next = converse(engine, &mut client, &[], 60);
        assert!(
            next.said("We need to destroy these pillars to collapse the ceiling on the dragon."),
            "{:?}",
            next.texts
        );
        // His bag is by the pillars (4KO2h2xPtdM @284): the supplies without going back.
        let origin = player_origin(engine, &client);
        oploc1(
            &mut client,
            loc_id(engine, "sir_vant_s_bag"),
            origin.0 + 28,
            origin.1 + 29,
        );
        let bag = converse(engine, &mut client, &[], 60);
        assert!(
            bag.said("You search Sir Vant's bag and take a selection of supplies."),
            "{:?} stage {} locs {:?} at {:?} msgs {:?}",
            bag.texts,
            progress(engine, &client),
            loc_ids(engine, origin.0 + 28, origin.1 + 29),
            cellar_tile(engine, &client),
            HarnessClient::game_messages(&bag.packets)
        );
        assert_eq!(progress(engine, &client), 96, "the supplies");
        // He stays by the pillars (2011 @244-366): an npc a script moves walks back to where it
        // was added once its mode goes back to its default (in-client run ufrest15, Sir Vant back
        // in the doorway (32, 16) for the sabotage).
        cycles(engine, &mut client, 100);
        let (vx, vz) = npc_tile(engine, "sir_vant_8850")
            .map(|t| local(engine, t))
            .expect("Sir Vant");
        assert!(
            (i32::from(vx) - 26).abs() <= 5 && (i32::from(vz) - 30).abs() <= 5,
            "Sir Vant by the pillars, not ({vx}, {vz})"
        );
    });
}

/// Build plan stage 8: the supplies, the rags and fuses on both pillars, the tinderbox, and the
/// collapse to Roddeck (4KO2h2xPtdM @289-427).
#[test]
fn the_pillars_burn_and_the_roof_comes_down() {
    runner::on_loaded_engine(|engine| {
        let mut client = at_stage(engine, "uf_pillars", 93);
        let origin = player_origin(engine, &client);
        let at = |x: u16, z: u16| (origin.0 + x, origin.1 + z);

        engine.harness_telejump(client.pid, origin.0 + 27, 0, origin.1 + 31);
        cycles(engine, &mut client, 3);
        talk_to(&mut client, npc_nid(engine, "sir_vant_8850"));
        let talk = converse(engine, &mut client, &[], 120);
        assert!(
            talk.said("We need to destroy these pillars to collapse the ceiling on the dragon.")
        );
        assert_eq!(progress(engine, &client), 95);

        // The bag, on the other side of the wall.
        engine.harness_telejump(client.pid, origin.0 + 34, 0, origin.1 + 12);
        cycles(engine, &mut client, 3);
        let (bx, bz) = at(35, 13);
        oploc1(&mut client, loc_id(engine, "sir_vant_s_bag"), bx, bz);
        let bag = converse(engine, &mut client, &[], 60);
        assert!(bag.said("You search Sir Vant's bag and take a selection of supplies."));
        assert_eq!(progress(engine, &client), 96);
        let names = [
            "tinderbox",
            "cloth",
            "fuse_15282",
            "fuse_15283",
            "sir_vant_s_oil_can",
        ];
        for name in names {
            assert!(
                slot_of(engine, &client, obj_id(engine, name)).is_some(),
                "{name}"
            );
        }

        // Rip, soak both rags.
        let cloth = obj_id(engine, "cloth");
        let c = slot_of(engine, &client, cloth).unwrap();
        held1(&mut client, cloth, c);
        assert!(
            converse(engine, &mut client, &[], 30)
                .said("You rip the cloth and end up with two rags.")
        );
        let rag = obj_id(engine, "rag");
        let oil = obj_id(engine, "sir_vant_s_oil_can");
        for _ in 0..2 {
            let r = slot_of(engine, &client, rag).expect("a rag");
            let o = slot_of(engine, &client, oil).unwrap();
            use_on_item(&mut client, (oil, o), (rag, r));
            assert!(converse(engine, &mut client, &[], 30).said("You soak the rag in oil."));
        }
        assert_eq!(progress(engine, &client), 100);

        // Both pillars: the oily rag, then a fuse.
        engine.harness_telejump(client.pid, origin.0 + 25, 0, origin.1 + 35);
        cycles(engine, &mut client, 3);
        let oily = obj_id(engine, "oily_rag");
        let fuses = [obj_id(engine, "fuse_15282"), obj_id(engine, "fuse_15283")];
        for (i, (px, pz)) in [(24u16, 31u16), (24, 39)].iter().enumerate() {
            let (wx, wz) = at(*px, *pz);
            let pillar = if i == 0 { "loc_45564" } else { "loc_45565" };
            let r = slot_of(engine, &client, oily).unwrap();
            use_on_loc(&mut client, oily, r, loc_id(engine, pillar), wx, wz);
            assert!(
                converse(engine, &mut client, &[], 60)
                    .said("You wrap the oily rag around the pillar.")
            );
            let f = slot_of(engine, &client, fuses[i]).unwrap();
            use_on_loc(&mut client, fuses[i], f, loc_id(engine, pillar), wx, wz);
            assert!(
                converse(engine, &mut client, &[], 60).said("You attach the fuse to the pillar.")
            );
        }
        assert_eq!(varbit_named(engine, &client, "varplayerbit_6496"), 2);
        assert_eq!(varbit_named(engine, &client, "varplayerbit_6497"), 2);
        assert_eq!(progress(engine, &client), 110);

        // The tinderbox to Sir Vant, then the far pillar.
        let tinderbox = obj_id(engine, "tinderbox");
        let t = slot_of(engine, &client, tinderbox).unwrap();
        use_on_npc(&mut client, tinderbox, t, npc_nid(engine, "sir_vant_8850"));
        let given = converse(engine, &mut client, &[], 60);
        assert!(given.said("You offer Sir Vant a tinderbox."));
        assert_eq!(progress(engine, &client), 115);
        let t = slot_of(engine, &client, tinderbox).unwrap();
        let (wx, wz) = at(24, 39);
        use_on_loc(
            &mut client,
            tinderbox,
            t,
            loc_id(engine, "loc_45565"),
            wx,
            wz,
        );
        let (texts, packets) = play_out(engine, &mut client, &[], 300);
        assert!(said(&texts, "It's burning! Run!"), "{texts:?}");
        assert!(sent_text(&packets, "Wow!"));
        assert!(said(&texts, "Ah, here you are, Roddeck."), "{texts:?}");
        assert!(said(&texts, "Next time, get a puppy!"), "{texts:?}");
        assert!(
            said(&texts, "could I have my squire's kit back, please?"),
            "{texts:?}"
        );
        assert_eq!(progress(engine, &client), 120, "Return the Squire's Gear");
        assert_eq!(
            npc_tile(engine, "dragon_8861"),
            None,
            "the dragon is under the rubble"
        );
        assert!(npc_tile(engine, "roddeck").is_some(), "Roddeck is out");
    });
}

/// Build plan stages 9-11: the kit back, Roddeck's choice, practice to Mining 2, the rocks, the
/// hole, the ladder, the quest's end and Explorer Jack (4KO2h2xPtdM @427-655).
#[test]
fn the_way_out_to_lumbridge() {
    runner::on_loaded_engine(|engine| {
        let mut client = at_stage(engine, "uf_exit", 120);
        let origin = player_origin(engine, &client);
        let at = |x: u16, z: u16| (origin.0 + x, origin.1 + z);

        engine.harness_telejump(client.pid, origin.0 + 29, 0, origin.1 + 31);
        cycles(engine, &mut client, 3);
        // The kit, the tinderbox and the oil can in the pack, as the collapse leaves a player:
        // talking gets s042 only; each piece used on him is taken, the last with s043 (in-client
        // run 18 talked to him holding it and got no further).
        let names = [
            "bronze_platebody",
            "bronze_sq_shield",
            "bronze_sword",
            "tinderbox",
            "sir_vant_s_oil_can",
        ];
        for (slot, name) in names.iter().enumerate() {
            give(engine, &client, slot as u16, obj_id(engine, name), 1);
        }
        cycles(engine, &mut client, 1);
        let vant = npc_nid(engine, "sir_vant_8850");
        talk_to(&mut client, vant);
        let held = converse(engine, &mut client, &[], 60);
        assert!(held.said("Please hand over my squire's kit."));
        assert_eq!(progress(engine, &client), 120);
        let mut last = None;
        for (slot, name) in names.iter().enumerate() {
            use_on_npc(&mut client, obj_id(engine, name), slot as u16, vant);
            last = Some(converse(
                engine,
                &mut client,
                &["Okay, I'll get on with it."],
                160,
            ));
        }
        let kit = last.unwrap();
        assert!(
            kit.said("Right, I think that's everything."),
            "{:?}",
            kit.texts
        );
        assert_eq!(progress(engine, &client), 128, "Discover an Exit");

        talk_to(&mut client, npc_nid(engine, "roddeck"));
        let choice = converse(engine, &mut client, &["I'll mine through the rocks."], 160);
        assert!(choice.said("Roddeck gives you a pickaxe."));
        assert_eq!(progress(engine, &client), 130);

        // The blockage at level 1.
        engine.harness_telejump(client.pid, origin.0 + 15, 0, origin.1 + 35);
        cycles(engine, &mut client, 3);
        let (rx, rz) = at(14, 35);
        oploc1(&mut client, loc_id(engine, "rock_45595"), rx, rz);
        assert!(
            converse(engine, &mut client, &[], 60)
                .said("You'll need a Mining level of 2 to clear the rocks.")
        );
        assert_eq!(progress(engine, &client), 135);
        engine.harness_telejump(client.pid, origin.0 + 31, 0, origin.1 + 32);
        cycles(engine, &mut client, 3);
        talk_to(&mut client, npc_nid(engine, "roddeck"));
        let spot = converse(engine, &mut client, &[], 200);
        assert!(spot.said("You can practice mining on the rocks that fell into my cellar"));
        assert_eq!(progress(engine, &client), 140);

        // Two practice rocks to level 2: the practice rock on (17, 38) (the map's (17, 14), the block
        // moved north by the collapse), live at varbit 6495 = 1.
        engine.harness_telejump(client.pid, origin.0 + 17, 0, origin.1 + 37);
        cycles(engine, &mut client, 3);
        let (px, pz) = at(17, 38);
        for _ in 0..2 {
            oploc1(&mut client, loc_id(engine, "loc_45571"), px, pz);
            converse(engine, &mut client, &[], 120);
            cycles(engine, &mut client, 4);
        }
        assert_eq!(progress(engine, &client), 150, "Mining level 2");

        engine.harness_telejump(client.pid, origin.0 + 15, 0, origin.1 + 35);
        cycles(engine, &mut client, 3);
        oploc1(&mut client, loc_id(engine, "rock_45595"), rx, rz);
        let cleared = converse(engine, &mut client, &[], 120);
        assert!(cleared.said("You succeed in clearing the rocks from the hole."));
        assert_eq!(progress(engine, &client), 160);
        oploc1(&mut client, loc_id(engine, "hole_45597"), rx, rz);
        cycles(engine, &mut client, 8);
        assert_eq!(progress(engine, &client), 170, "To Freedom!");

        // The ladder up: Explorer Jack's house, the quest's end.
        engine.harness_telejump(client.pid, origin.0 + 6, 0, origin.1 + 44);
        cycles(engine, &mut client, 3);
        let (lx, lz) = at(6, 45);
        oploc1(&mut client, loc_id(engine, "ladder_45784"), lx, lz);
        let up = cycles(engine, &mut client, 8);
        assert_eq!(progress(engine, &client), 1000, "the quest complete");
        let messages: Vec<String> = HarnessClient::game_messages(&up)
            .into_iter()
            .map(|(_, m)| m)
            .collect();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("Congratulations! Quest complete!")),
            "{messages:?}"
        );
        assert!(
            messages.iter().any(|m| m.contains("Harmony")),
            "{messages:?}"
        );
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.stats.base_levels[14], 4, "Mining 4");

        // The scroll closed, the level-up queued behind it runs: 2011's one line for the two
        // levels and no level-up box (4KO2h2xPtdM @610-636, VwpdbjfigD4 @788-796).
        engine.harness_close_modal(client.pid);
        let after = cycles(engine, &mut client, 3);
        let lines: Vec<String> = HarnessClient::game_messages(&after)
            .into_iter()
            .map(|(_, m)| m)
            .collect();
        assert!(
            lines
                .iter()
                .any(|m| m == "You've just advanced 2 Mining levels! You have reached level 4."),
            "{lines:?}"
        );
        assert!(!is_open(engine, &client, 740), "no level-up box");
        talk_to(&mut client, npc_nid(engine, "explorer_jack_13295"));
        let jack = converse(engine, &mut client, &[], 60);
        assert!(
            jack.said("Good grief! Where did you three come from?"),
            "{:?}",
            jack.texts
        );
        assert!(jack.said("Look at the Rewards tab for a Task to learn more."));
        assert_eq!(varbit_named(engine, &client, "varplayerbit_6494"), 3);
    });
}

/// Roddeck's cellar's bookcase and crate, and the tutorial's eat and drop boxes (4KO2h2xPtdM
/// @479.9-516, @574-576).
#[test]
fn the_books_the_cabbage_and_the_drop_hint() {
    runner::on_loaded_engine(|engine| {
        let mut client = at_stage(engine, "uf_shelf", 130);
        let origin = player_origin(engine, &client);
        let diary = obj_id(engine, "roddeck_s_diary");
        let manual = obj_id(engine, "manual_15671");
        let cabbage = obj_id(engine, "cabbage");

        engine.harness_telejump(client.pid, origin.0 + 48, 0, origin.1 + 36);
        cycles(engine, &mut client, 3);
        let shelf = loc_ids(engine, origin.0 + 49, origin.1 + 36);
        let bookcase = loc_id(engine, "bookcase_49101");
        assert!(
            shelf.contains(&bookcase),
            "the bookcase in the copy: {shelf:?}"
        );
        oploc1(&mut client, bookcase, origin.0 + 49, origin.1 + 36);
        let books = converse(engine, &mut client, &[], 60);
        assert!(
            books.said("You find a couple of books and take them."),
            "{:?} {:?} at {:?}",
            books.texts,
            HarnessClient::game_messages(&books.packets),
            engine.harness_player(client.pid).unwrap().pathing.coord
        );
        assert!(speech(&books.packets).contains(&1345));
        assert!(slot_of(engine, &client, diary).is_some(), "the diary");
        assert!(slot_of(engine, &client, manual).is_some(), "the manual");

        // Read: spread one as 4KO2h2xPtdM @483-491 shows it, the narrator reading it (clip 691).
        let at_slot = slot_of(engine, &client, diary).unwrap();
        held1(&mut client, diary, at_slot);
        let read = cycles(engine, &mut client, 3);
        let shown: Vec<String> = rs_test::talk::settexts(&read)
            .into_iter()
            .map(|(_, t)| t)
            .collect();
        for line in [
            "Roddeck's Diary",
            "Yet again it's raining.",
            "Saradomin rest his soul.",
        ] {
            assert!(shown.iter().any(|t| t == line), "{line} in {shown:?}");
        }
        assert!(speech(&read).contains(&691));
        engine.harness_close_modal(client.pid);
        cycles(engine, &mut client, 2);

        engine.harness_telejump(client.pid, origin.0 + 42, 0, origin.1 + 36);
        cycles(engine, &mut client, 3);
        let crate_loc = loc_id(engine, "crate_49100");
        oploc1(&mut client, crate_loc, origin.0 + 42, origin.1 + 35);
        let take = converse(engine, &mut client, &[], 60);
        assert!(
            take.said("It's full of cabbages. You take one."),
            "{:?}",
            take.texts
        );
        let slot = slot_of(engine, &client, cabbage).expect("a cabbage");
        oploc1(&mut client, crate_loc, origin.0 + 42, origin.1 + 35);
        let again = converse(engine, &mut client, &[], 60);
        assert!(again.said("It's full of cabbages. You've already got one."));

        held1(&mut client, cabbage, slot);
        let eat = converse(engine, &mut client, &[], 60);
        assert!(eat.said("You eat the cabbage."), "{:?}", eat.texts);
        assert!(slot_of(engine, &client, cabbage).is_none(), "eaten");

        // Drop (op 8 on the backpack): the hint, with the dropped book in the box.
        let slot = slot_of(engine, &client, manual).expect("the manual");
        let mut p = ClientPacket::new();
        p.p4(i32::from(INVENTORY) << 16)
            .p2_alt3(i32::from(manual))
            .p2_alt1(i32::from(slot));
        client.send(ClientProt::IfButton8, &p.finish());
        let drop = converse(engine, &mut client, &[], 60);
        assert!(
            drop.said("When you drop items on the ground, they will eventually vanish."),
            "{:?}",
            drop.texts
        );
        assert!(speech(&drop.packets).contains(&1669));
    });
}

/// The tiles a player can walk to from `start` inside the cellar copy (64 x 64 from `origin`).
fn walkable_from(origin: (u16, u16), start: (u16, u16)) -> std::collections::HashSet<(u16, u16)> {
    use rsmod::rsmod::collision::collision_strategy::CollisionType;
    let mut seen = std::collections::HashSet::new();
    let mut todo = vec![(origin.0 + start.0, origin.1 + start.1)];
    while let Some((x, z)) = todo.pop() {
        if !seen.insert((x - origin.0, z - origin.1)) {
            continue;
        }
        for (dx, dz) in [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)] {
            let (nx, nz) = (
                x.wrapping_add_signed(dx as i16),
                z.wrapping_add_signed(dz as i16),
            );
            if nx < origin.0 || nz < origin.1 || nx >= origin.0 + 64 || nz >= origin.1 + 64 {
                continue;
            }
            if rsmod::can_travel(0, x, z, dx, dz, 1, 0, CollisionType::Normal)
                && !seen.contains(&(nx - origin.0, nz - origin.1))
            {
                todo.push((nx, nz));
            }
        }
    }
    seen
}

/// After the collapse the way out is on foot: 4KO2h2xPtdM (2011-04-30) @526-542, from Roddeck to
/// the rocks in the damaged wall in ~8 s with no climb; then through the hole and up the ladder.
#[test]
fn the_way_out_is_walkable_from_roddeck() {
    runner::on_loaded_engine(|engine| {
        let client = at_stage(engine, "uf_route", 120);
        let origin = player_origin(engine, &client);
        let seen = walkable_from(origin, (31, 32));
        let near = |x: u16, z: u16| {
            [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|&(dx, dz)| seen.contains(&((x as i16 + dx) as u16, (z as i16 + dz) as u16)))
        };
        let mut map = String::new();
        for z in (0..48u16).rev() {
            for x in 0..64u16 {
                map.push(if seen.contains(&(x, z)) { '.' } else { '#' });
            }
            map.push_str(&format!(" {z}\n"));
        }
        assert!(
            near(14, 35) || near(14, 38),
            "the rocks or roots from Roddeck:\n{map}"
        );
    });
}
