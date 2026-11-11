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

/// The run orb (topstat_run 750:1, op 1): turns run on with full energy.
fn run_orb(client: &mut HarnessClient) {
    let mut p = ClientPacket::new();
    p.p4(750 << 16 | 1).p2_alt3(NONE).p2_alt1(NONE);
    client.send(ClientProt::IfButton1, &p.finish());
}

/// A click on the floor near the player: the first walkable tile of a few nearby.
fn walk_near(engine: &Engine, client: &mut HarnessClient) {
    let here = engine.harness_player(client.pid).unwrap().pathing.coord;
    let (x, z) = [(2i32, 0i32), (-2, 0), (0, 2), (0, -2), (1, 1), (-1, -1)]
        .into_iter()
        .map(|(a, b)| {
            (
                (i32::from(here.x()) + a) as u16,
                (i32::from(here.z()) + b) as u16,
            )
        })
        .find(|&(tx, tz)| !rsmod::is_flagged(tx, tz, 0, 0x100 | 0x200000))
        .expect("a walkable tile nearby");
    let mut p = ClientPacket::new();
    p.p2_alt1(i32::from(x)).p1_alt1(0).p2(i32::from(z));
    client.send(ClientProt::MoveGameClick, &p.finish());
}

fn varp(engine: &Engine, name: &str) -> u16 {
    *engine.cache.varps.debugnames.get(name).unwrap()
}

fn progress(engine: &Engine, client: &HarnessClient) -> i32 {
    let id = varp(engine, "tutorial");
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
        .get(SHOT)
        .as_int()
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

/// An npc's tile and its spawn tile as the cellar's local (x, z), and its mode.
type NpcTrace = ((u16, u16), (u16, u16), Option<u8>);

/// The first npc of a type, traced.
fn npc_trace(engine: &Engine, name: &str) -> Option<NpcTrace> {
    let id = *engine.cache.npcs.debugnames.get(name).unwrap();
    let npc = &engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == id)?
        .npc;
    Some((
        local(engine, (npc.pathing.coord.x(), npc.pathing.coord.z())),
        local(engine, (npc.spawn_coord.x(), npc.spawn_coord.z())),
        npc.interaction.target_op,
    ))
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
/// 8851 while the dragon cutscene changes him, or 8852 once he keeps the dragon's attention).
fn cellar_origin(engine: &Engine) -> (u16, u16) {
    let (x, z) = npc_tile(engine, "tutorial3_knight_waitingforbattle")
        .or_else(|| npc_tile(engine, "tutorial3_knight_battle"))
        .or_else(|| npc_tile(engine, "tutorial3_knight_distracting"))
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

/// One tick of a cutscene: its shot, its packets, and where the squire stands.
type CutsceneTick = (i32, Vec<(u16, Vec<u8>)>, Option<(u16, u16)>);

/// One tick of the collapse: the shot, whether "Wow!" went out, the door open, Roddeck there, s038.
type CollapseTick = (i32, bool, bool, bool, bool);

/// The index of the first tick showing `shot`.
fn shots_start(ticks: &[CutsceneTick], shot: i32) -> usize {
    ticks.iter().position(|(now, _, _)| *now == shot).unwrap()
}

/// Where `packets` set the TASK COMPLETE! popup (varc 1425, by VARC_LARGE `p4_alt1 value,
/// p2_alt3 varc`) to `task`.
fn popup_at(packets: &[(u16, Vec<u8>)], task: i32) -> Option<usize> {
    packets.iter().position(|(op, p)| {
        *op == ServerProt::VarcLarge as u16
            && p.len() >= 6
            && u16::from_le_bytes([p[4].wrapping_sub(128), p[5]]) == POPUP_TASK
            && i32::from_le_bytes([p[0], p[1], p[2], p[3]]) == task
    })
}

/// Where `packets` last put varc 1007 back at 0 (VARC_SMALL `p1_alt2 value, p2_alt1 varc`): a
/// cutscene's release.
fn shot_zero_at(packets: &[(u16, Vec<u8>)]) -> Option<usize> {
    packets.iter().rposition(|(op, p)| {
        *op == ServerProt::VarcSmall as u16
            && p.len() >= 3
            && u16::from_le_bytes([p[1], p[2]]) == SHOT
            && p[0] == 0
    })
}

/// The tick packet `i` went out on, counted in PLAYER_INFOs. A tick's Immediate packets (varps,
/// varcs) are written as its scripts run, then its PLAYER_INFO and NPC_INFO, then its Buffered
/// ones (interfaces, clientscripts, the minimap): `buffered` says which kind packet `i` is.
fn tick_of(packets: &[(u16, Vec<u8>)], i: usize, buffered: bool) -> usize {
    let infos = packets[..i]
        .iter()
        .filter(|(op, _)| *op == ServerProt::PlayerInfo as u16)
        .count();
    if buffered { infos - 1 } else { infos }
}

/// What is wrong with a cellar cutscene's release, if anything: the minimap back and `task`'s
/// TASK COMPLETE! on the tick varc 1007 is back at 0.
fn release_fault(packets: &[(u16, Vec<u8>)], task: i32) -> Option<String> {
    let minimap = packets
        .iter()
        .rposition(|(op, b)| *op == ServerProt::MinimapToggle as u16 && b[0] == 0);
    let zero = shot_zero_at(packets);
    let banner = popup_at(packets, task);
    let (Some(z), Some(m), Some(b)) = (zero, minimap, banner) else {
        return Some(format!(
            "varc 1007 at 0 {zero:?}, minimap {minimap:?}, banner {banner:?}"
        ));
    };
    let (zt, mt, bt) = (
        tick_of(packets, z, false),
        tick_of(packets, m, true),
        tick_of(packets, b, false),
    );
    if zt == mt && zt == bt {
        return None;
    }
    Some(format!(
        "varc 1007 at 0 is packet {z} (tick {zt}), the minimap {m} (tick {mt}), the banner {b} (tick {bt}): {}",
        around(packets, z.min(m).min(b), z.max(m).max(b))
    ))
}

/// The opcodes of packets `a..=b` and two either side, PLAYER_INFO as "PI".
fn around(packets: &[(u16, Vec<u8>)], a: usize, b: usize) -> String {
    (a.saturating_sub(2)..(b + 3).min(packets.len()))
        .map(|i| {
            if packets[i].0 == ServerProt::PlayerInfo as u16 {
                format!("{i}:PI")
            } else {
                format!("{i}:{}", packets[i].0)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether a tick's packets carry `text` (an NPC_INFO say is the bare string).
fn sent_text(packets: &[(u16, Vec<u8>)], text: &str) -> bool {
    let text = text.as_bytes();
    packets
        .iter()
        .any(|(_, body)| body.windows(text.len()).any(|w| w == text))
}

/// Whether a tick's NPC_INFO carries an anim block for `seq`: 647 writes the id once for each of
/// its four channels (info.rs's 0x80 npc block), each the high byte then the low byte plus 128.
fn npc_anim_sent(packets: &[(u16, Vec<u8>)], seq: u16) -> bool {
    let [hi, lo] = seq.to_be_bytes();
    let block = [hi, lo.wrapping_add(128)].repeat(4);
    packets
        .iter()
        .any(|(_, body)| body.windows(block.len()).any(|w| w == block))
}

/// The voice-over clips a tick's packets play.
fn speech(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    packets
        .iter()
        .filter(|(op, _)| *op == ServerProt::VorbisSpeechSound as u16)
        .map(|(_, body)| u16::from_be_bytes([body[0], body[1]]))
        .collect()
}

/// Whether every voice line (VORBIS_SPEECH_SOUND) comes straight after a VORBIS_SPEECH_STOP: the
/// 647 client plays speech sounds side by side, so the last line must be stopped first.
fn each_voice_stops_the_last(packets: &[(u16, Vec<u8>)]) -> bool {
    packets.iter().enumerate().all(|(i, (op, _))| {
        *op != ServerProt::VorbisSpeechSound as u16
            || (i > 0 && packets[i - 1].0 == ServerProt::VorbisSpeechStop as u16)
    })
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

/// Whether `packets` run clientscript 4097 on the pack's hint layer with "Click to equip.".
fn equip_hint_shown(packets: &[(u16, Vec<u8>)]) -> bool {
    packets.iter().any(|(op, b)| {
        *op == ServerProt::RunClientScript as u16
            && b.len() >= 4
            && b[b.len() - 4..] == 4097u32.to_be_bytes()
            && b.windows(15).any(|w| w == b"Click to equip.")
    })
}

/// Whether `packets` hide the pack's hint layer (inventory 149:1).
fn equip_hint_hidden(packets: &[(u16, Vec<u8>)]) -> bool {
    let com: u32 = (149 << 16) | 1;
    packets.iter().any(|(op, b)| {
        *op == ServerProt::IfSetHide as u16
            && b.len() >= 5
            && b[..4]
                == [
                    (com >> 16) as u8,
                    (com >> 24) as u8,
                    com as u8,
                    (com >> 8) as u8,
                ]
            && b[4] != 0
            && b[4] != 128
    })
}

/// The hint ring (cache 786 model 40497), as uf.constant's ^uf_hint_ring.
const HINT_RING: u16 = 40497;

/// The character design screen's grids (playerdesign4, interface 1028): a style or a colour is
/// picked by the slot clicked, which the engine gives `[if_button1,...]` and refuses a plain
/// `[if_button,...]` (rs-vm `allows_last_slot`). Two slots a grid, since the screen opens on a
/// random hair and hair colour.
#[test]
fn character_creation_grids_pick_by_the_slot_clicked() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login("uf_designer").expect("login");
        cycles(engine, &mut client, 3);
        let click = |engine: &mut Engine, client: &mut HarnessClient, com: i32, slot: i32| {
            let mut p = ClientPacket::new();
            p.p4((1028 << 16) | com).p2_alt3(NONE).p2_alt1(slot);
            client.send(ClientProt::IfButton1, &p.finish());
            cycles(engine, client, 2);
        };
        let varc = |engine: &Engine, client: &HarnessClient, id: u16| {
            engine
                .harness_player(client.pid)
                .unwrap()
                .varcs
                .get(id)
                .as_int()
        };
        // The hair page (1028:96), then the styles (1028:107): enum_3304's slots 0 and 1 are
        // struct_746 "Short" (idkit_5) and struct_747 "Close-cropped" (idkit_6).
        click(engine, &mut client, 96, NONE);
        click(engine, &mut client, 107, 0);
        assert_eq!(varc(engine, &client, 1008), 5, "playerdesign3_head");
        click(engine, &mut client, 107, 1);
        assert_eq!(varc(engine, &client, 1008), 6, "playerdesign3_head");
        // The hair page's colours (1028:111): enum_2345's slots 0 and 1 are 20 and 19.
        click(engine, &mut client, 111, 0);
        assert_eq!(varc(engine, &client, 1015), 20, "playerdesign3_haircol");
        click(engine, &mut client, 111, 1);
        assert_eq!(varc(engine, &client, 1015), 19, "playerdesign3_haircol");
        // The skin colours (1028:45): enum_748's slots 0 and 1 are 9 and 8.
        click(engine, &mut client, 45, 0);
        assert_eq!(varc(engine, &client, 1019), 9, "playerdesign3_skincol");
        click(engine, &mut client, 45, 1);
        assert_eq!(varc(engine, &client, 1019), 8, "playerdesign3_skincol");
    });
}

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
        let plank = *engine.cache.objs.debugnames.get("tutorial3_plank").unwrap();
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
            .get(POPUP_TASK)
            .as_int();
        assert_eq!(popup, 3522, "TASK COMPLETE! Rotate Your Camera");
        assert_eq!(
            varbit(engine, &client, PINNED_TASK),
            3500,
            "Talk to Sir Vant"
        );
        assert_eq!(varbit(engine, &client, FLASH_SIDE), 0, "no arrow after");

        // Talk to Sir Vant: "Hello!" and his warning, each with its voiced take (s001, s002).
        let vant = *engine
            .cache
            .npcs
            .debugnames
            .get("tutorial3_knight_waitingforbattle")
            .unwrap();
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
        let mut ticks: Vec<CutsceneTick> = vec![(
            shot(engine, &client),
            Vec::new(),
            npc_tile(engine, "tutorial3_squire"),
        )];
        let mut squire_face = vec![npc_face(engine, "tutorial3_squire")];
        let mut goblin = vec![npc_tile(engine, "tutorial3_goblin")];
        let mut dragon = vec![npc_tile(engine, "tutorial3_dragon_initial")];
        // Shot 10 holds until s003 is continued (1.7 s to 5.5 s in seven 2010-2011 recordings):
        // eight ticks of it here, more than any timer of ours ran, with nothing ending it.
        let mut held = 0;
        for _ in 0..80 {
            assert!(
                is_open(engine, &client, CUTSCENES),
                "interface 899 runs the camera"
            );
            engine.cycle();
            let packets = client.packets();
            let now = shot(engine, &client);
            ticks.push((now, packets, npc_tile(engine, "tutorial3_squire")));
            squire_face.push(npc_face(engine, "tutorial3_squire"));
            goblin.push(npc_tile(engine, "tutorial3_goblin"));
            dragon.push(npc_tile(engine, "tutorial3_dragon_initial"));
            assert_ne!(now, 0, "the cutscene ends only when s003 is continued");
            if now == 10 {
                held += 1;
                if held == 8 {
                    break;
                }
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
        assert_eq!(order, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], "{shots:?}");
        let lengths: Vec<u32> = shots[1..9].iter().map(|(_, n)| *n).collect();
        assert_eq!(lengths, vec![6, 3, 13, 6, 7, 4, 4, 9], "{shots:?}");
        assert_eq!(shots[9], (10, 8), "shot 10 for as long as s003 is unread");

        // The dragon stands through shot 3 and runs to the wall as shot 4 opens, two tiles a
        // tick, to (30, 18): 4KO2h2xPtdM has it at the wall 2.44-2.54 s after the cut @25.96,
        // seven tiles on.
        let four = shots_start(&ticks, 4);
        let walk: Vec<Option<(u16, u16)>> = dragon[four..four + 6]
            .iter()
            .map(|tile| tile.map(|tile| local(engine, tile)))
            .collect();
        assert_eq!(
            walk,
            vec![
                Some((30, 25)),
                Some((30, 23)),
                Some((30, 21)),
                Some((30, 19)),
                Some((30, 18)),
                Some((30, 18)),
            ],
            "the dragon runs to the wall from shot 4's first tick"
        );
        // It keeps that tile through shot 8 and stands on (28, 19) as shot 10 opens (4KO2h2xPtdM
        // @44.1-46.4 against @52.2-52.8, and its minimap after the cutscene @55-104).
        let nine = shots_start(&ticks, 9);
        let ten = shots_start(&ticks, 10);
        assert!(
            dragon[four + 4..=nine]
                .iter()
                .all(|tile| tile.map(|tile| local(engine, tile)) == Some((30, 18))),
            "the dragon over Sir Vant through shot 8"
        );
        assert_eq!(
            dragon[ten - 1].map(|tile| local(engine, tile)),
            Some((28, 19)),
            "the dragon north-west of Sir Vant a tick before shot 10 opens"
        );
        // Shot 9: the goblin's "Ooh, shiny!" with its clip; shot 10: Sir Vant's s003 with his.
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
        // The goblin drops in beside the pillar as the shot opens (seq 9965), and is rummaging in
        // the bag a tick before the shot ends (seq 9963; 4KO2 @46.5-46.9 and @51.5).
        assert!(
            npc_anim_sent(&ticks[nine].1, 9965),
            "the goblin's drop as shot 9 opens"
        );
        let rummage = ticks[nine..nine + 9]
            .iter()
            .position(|(_, packets, _)| npc_anim_sent(packets, 9963));
        assert_eq!(rummage, Some(8), "the goblin's rummage on shot 9's tick 8");
        // Its walk to the bag is asked for on tick 3 and moves it on tick 4, one tick before the
        // overhead (4KO2: it starts to turn and walk @48.73-48.80, the overhead is up @49.47).
        assert!(goblin[nine].is_some(), "the goblin is in as shot 9 opens");
        let walk = goblin[nine..nine + 9]
            .iter()
            .position(|tile| *tile != goblin[nine]);
        assert_eq!(
            walk,
            Some(4),
            "the goblin leaves the pillar on shot 9's tick 4"
        );
        // Its mutters wait for the rummage: "Ooh, shiny!" stays overhead to the shot's end.
        assert!(
            !ticks[nine..nine + 9].iter().any(|(_, packets, _)| {
                ["Heh heh heh.", "Mine, all mine!", "Ooh!", "Me get rich!"]
                    .iter()
                    .any(|line| sent_text(packets, line))
            }),
            "no mutter replaces Ooh, shiny! inside shot 9"
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
        // s003 continued. The tick that takes it has the camera back (varc 1007 = 0, with 899
        // still open for its hook), the minimap back and TASK COMPLETE! Talk to Sir Vant, as 2011
        // has them in the frame the box goes (4KO2h2xPtdM @53.7; 8jKH_7To46o @128.7-129.3);
        // 899 closes a tick later.
        let read = converse(engine, &mut client, &[], 1);
        assert_eq!(
            shot(engine, &client),
            10,
            "shot 10 until the continue is taken"
        );
        engine.cycle();
        let release = client.packets();
        assert_eq!(shot(engine, &client), 0, "the camera released on that tick");
        assert!(
            release
                .iter()
                .any(|(op, b)| *op == ServerProt::MinimapToggle as u16 && b[0] == 0),
            "the minimap back on the release's tick"
        );
        assert!(
            popup_at(&release, 3500).is_some(),
            "TASK COMPLETE! on the release's tick"
        );
        assert!(
            is_open(engine, &client, CUTSCENES),
            "899 still open on the tick its hook runs"
        );
        engine.cycle();
        assert!(
            !is_open(engine, &client, CUTSCENES),
            "899 closed a tick after the release"
        );
        let after: Vec<(u16, Vec<u8>)> = read
            .packets
            .into_iter()
            .chain(release)
            .chain(client.packets())
            .collect();
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
                .get(POPUP_TASK)
                .as_int(),
            3500,
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
        // Each voice line stops the one before it (VORBIS_SPEECH_STOP, then the line's take).
        assert!(each_voice_stops_the_last(&talk.packets));
        assert!(
            ticks
                .iter()
                .all(|(_, packets, _)| each_voice_stops_the_last(packets))
        );
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

        // The dragon's roar, seq 12259, starts on shot 2's last tick: its head thrust (the seq's
        // roar sound, 43 cycles in) comes 0.25-0.5 s after shot 3 opens (4KO2 @24.6-24.8 against
        // the cut @24.3; vGlx cut ~@33.1, thrust @33.35-33.5).
        let three = shot_at(3);
        assert!(
            npc_anim_sent(&ticks[three - 1].1, 12259),
            "the roar starts a tick before shot 3"
        );
        assert!(
            !npc_anim_sent(&ticks[three].1, 12259),
            "and is not restarted as shot 3 opens"
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
        let burnt = *engine
            .cache
            .locs
            .debugnames
            .get("tutorial3_cellar_ladder_breaking")
            .unwrap();
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
            npc_tile(engine, "tutorial3_knight_waitingforbattle").map(|tile| local(engine, tile)),
            Some((32, 16)),
            "Sir Vant keeps his tile"
        );
        assert_eq!(
            npc_tile(engine, "tutorial3_squire"),
            None,
            "the squire is gone"
        );
        assert_eq!(
            npc_tile(engine, "tutorial3_goblin").map(|tile| local(engine, tile)),
            Some((34, 13)),
            "the goblin at Sir Vant's bag"
        );
        assert_eq!(
            npc_tile(engine, "tutorial3_dragon_initial").map(|tile| local(engine, tile)),
            Some((28, 19)),
            "the dragon stays at the wall north-west of Sir Vant"
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
    let packets = cycles(engine, &mut client, 56);
    assert_eq!(progress(engine, &client), stage);
    if (10..45).contains(&stage) {
        assert!(
            npc_anim_sent(&packets, 9963),
            "a cellar built with the goblin in it has it rummaging"
        );
    }
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

        talk_to(
            &mut client,
            npc_nid(engine, "tutorial3_knight_waitingforbattle"),
        );
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
        let mut gathering = Vec::new();
        for (i, obj) in kit.iter().enumerate() {
            take(&mut client, *obj, gx, gz);
            gathering = cycles(engine, &mut client, 12);
            if i < 2 {
                assert_eq!(progress(engine, &client), 15);
                assert!(!is_open(engine, &client, INVENTORY), "no inventory yet");
                assert!(!equip_hint_shown(&gathering), "no hint before the third");
            }
        }
        assert_eq!(progress(engine, &client), 20, "Equip Yourself");
        assert!(is_open(engine, &client, INVENTORY), "the inventory tab");
        assert_eq!(
            varbit(engine, &client, FLASH_SIDE),
            5,
            "the arrow at the inventory tab"
        );
        // The pack's "Click to equip." box with the third piece (4KO2h2xPtdM @80.0).
        assert!(equip_hint_shown(&gathering), "Click to equip.");

        // Not yet armed: Sir Vant stops the attack.
        let goblin = npc_nid(engine, "tutorial3_goblin");
        opnpc1(&mut client, goblin);
        let stopped = converse(engine, &mut client, &[], 60);
        assert!(stopped.said(
            "Wait, you're not properly armed. You should equip all the kit you've picked up."
        ));

        let mut wearing = Vec::new();
        for (i, obj) in kit.iter().enumerate() {
            let slot = slot_of(engine, &client, *obj).expect("in the backpack");
            wear(&mut client, *obj, slot);
            let worn = cycles(engine, &mut client, 2);
            // The box goes with the first piece worn (4KO2h2xPtdM @81.5) and stays away.
            assert_eq!(equip_hint_hidden(&worn), i == 0, "piece {i}");
            assert!(!equip_hint_shown(&worn));
            wearing.extend(worn);
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
        assert_eq!(
            npc_tile(engine, "tutorial3_goblin"),
            None,
            "the goblin is dead"
        );
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

        talk_to(
            &mut client,
            npc_nid(engine, "tutorial3_knight_waitingforbattle"),
        );
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
        // From the plan's last line he keeps the dragon's attention: cache 786's Sir Vant 8852.
        assert_eq!(
            npc_tile(engine, "tutorial3_knight_distracting").map(|t| local(engine, t)),
            Some((32, 16)),
            "Sir Vant distracting the dragon"
        );
        assert_eq!(npc_tile(engine, "tutorial3_knight_waitingforbattle"), None);
        // The camera cuts to the pillars for the second line and to the food bowl for the third,
        // and is the player's again for the fourth (4KO2h2xPtdM @112.9, @114.7, @116.0): each
        // line's voice clip, with the camera packets around it. A shot's two packets follow its
        // line's clip: the engine writes camera moves at the end of the tick (update_map), and
        // cam_reset at once.
        let (moveto, lookat, reset, voice) = (
            ServerProt::CamMoveto as u16,
            ServerProt::CamLookat as u16,
            ServerProt::CamReset as u16,
            ServerProt::VorbisSpeechSound as u16,
        );
        let shots: Vec<u16> = plan
            .packets
            .iter()
            .filter(|(op, _)| [moveto, lookat, reset, voice].contains(op))
            .map(|(op, body)| {
                if *op == voice {
                    u16::from_be_bytes([body[0], body[1]])
                } else {
                    *op
                }
            })
            .collect();
        assert_eq!(
            shots,
            vec![732, 943, moveto, lookat, 806, moveto, lookat, reset, 726],
            "two shots, then the player's camera"
        );
        assert_eq!(progress(engine, &client), 50, "Check your Quest Journal");
        assert!(is_open(engine, &client, 190), "the quest list tab");
        assert_eq!(varbit(engine, &client, FLASH_SIDE), 4);

        // The journal (quest list row 147), the world map (548:179, then its close 755:44) and
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
        // The arrow moves to the quest's row: the client draws it at 55 on an Unstable
        // Foundations account (clientscript 1661; 4KO2h2xPtdM @119.0-120.0).
        assert_eq!(progress(engine, &client), 55, "the quest list opened");
        assert_eq!(varbit(engine, &client, 4895), 3, "tutorial_version");
        button(&mut client, 190, 18, 147);
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
            .get(622)
            .as_int();
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
        let bag = *engine
            .cache
            .locs
            .debugnames
            .get("tutorial3_knights_bag")
            .unwrap();
        let (bx, bz) = at(35, 13);
        oploc1(&mut client, bag, bx, bz);
        let search = converse(engine, &mut client, &[], 60);
        assert!(search.said(
            "The bag contains several parcels of meat, all wrapped up in paper. You take one."
        ));
        let meat = obj_id(engine, "tutorial3_meat");
        assert!(slot_of(engine, &client, meat).is_some(), "the meat");

        // The wall (Climb the Wall at 65: the quest journal, world map and minimap steps between
        // are their own stage).
        let ropes = varp(engine, "tutorial");
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .vars
            .set(ropes, VarValue::Int(65));
        cycles(engine, &mut client, 2);
        let wall = *engine
            .cache
            .locs
            .debugnames
            .get("tutorial3_cellar_barricade_clickarea")
            .unwrap();
        let (wx, wz) = at(35, 16);
        oploc1(&mut client, wall, wx, wz);
        let high = converse(engine, &mut client, &[], 60);
        assert!(high.said("Even this low section of the wall is too high for you to climb over."));
        assert_eq!(progress(engine, &client), 70, "Make a Ramp");

        let plank = obj_id(engine, "tutorial3_plank");
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
        // A login at 80 finds Sir Vant keeping the dragon's attention (8852).
        assert_eq!(
            npc_tile(engine, "tutorial3_knight_distracting").map(|t| local(engine, t)),
            Some((32, 16))
        );
        let meat = obj_id(engine, "tutorial3_meat");
        give(engine, &client, 0, meat, 1);
        engine.harness_telejump(client.pid, origin.0 + 17, 0, origin.1 + 37);
        cycles(engine, &mut client, 3);
        use_on_loc(
            &mut client,
            meat,
            0,
            loc_id(engine, "tutorial3_foodbowl_west"),
            origin.0 + 17,
            origin.1 + 35,
        );
        // The meat shows in the bowl once it is put there (4KO2h2xPtdM @200.2 to @223.65).
        let first = cycles(engine, &mut client, 4);
        assert_eq!(
            varbit_named(engine, &client, "tutorial3_meatplacement"),
            1,
            "the meat in the bowl"
        );
        let (mut texts, mut packets) = play_out(engine, &mut client, &[], 256);
        texts.splice(
            0..0,
            rs_test::talk::settexts(&first).into_iter().map(|(_, t)| t),
        );
        packets.splice(0..0, first);
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
        // Stage 2's cellar is built in 106's black: 2011's "Loading - please wait." @233.
        let at_shot = |n: i32| {
            packets
                .iter()
                .position(|(op, p)| {
                    *op == ServerProt::VarcSmall as u16
                        && p.len() >= 3
                        && u16::from_le_bytes([p[1], p[2]]) == SHOT
                        && 0u8.wrapping_sub(p[0]) as i8 as i32 == n
                })
                .unwrap()
        };
        let (fade, last) = (at_shot(106), at_shot(107));
        assert!(
            packets[fade..last]
                .iter()
                .any(|(op, _)| *op == ServerProt::RebuildRegion as u16),
            "the cellar rebuilt in the fade"
        );
        // Shot 105: the dragon circles and lies down on its nest by seq 12254 alone
        // (4KO2h2xPtdM @227.25-230.8), not by turns and seq 14260.
        let nest = at_shot(105);
        assert!(
            npc_anim_sent(&packets[nest..fade], 12254),
            "seq 12254 in shot 105"
        );
        assert!(!npc_anim_sent(&packets, 14260), "seq 14260 is not played");
        // TASK COMPLETE! Put the Dragon to Sleep (varc 1425 = 3510, by VARC_LARGE
        // `p4_alt1 value, p2_alt3 varc`) comes as the cutscene closes, once s020 is continued:
        // 4KO2h2xPtdM has no banner over shot 107 @236-244.6 and its first frame @244.9, with
        // the camera back on the player.
        let banner = packets
            .iter()
            .position(|(op, p)| {
                *op == ServerProt::VarcLarge as u16
                    && p.len() >= 6
                    && u16::from_le_bytes([p[4].wrapping_sub(128), p[5]]) == POPUP_TASK
                    && i32::from_le_bytes([p[0], p[1], p[2], p[3]]) == 3510
            })
            .expect("TASK COMPLETE! Put the Dragon to Sleep");
        assert!(
            banner > at_shot(0),
            "the banner after the cutscene closes, not over shot 107"
        );
        assert_eq!(
            release_fault(&packets, 3510),
            None,
            "the minimap and the banner with the camera's release (4KO2h2xPtdM @244.9)"
        );
        let clips = speech(&packets);
        for clip in [715, 850, 958] {
            assert!(clips.contains(&clip), "clip {clip} in {clips:?}");
        }
        assert_eq!(progress(engine, &client), 93, "Talk to Sir Vant Again");
        // The dragon eats it: from @227.9, the first frame the bowl shows again past the dragon,
        // it is the bowl with the bone that the cellar starts with.
        assert_eq!(
            varbit_named(engine, &client, "tutorial3_meatplacement"),
            0,
            "the meat eaten, the bone bowl again"
        );
        assert_eq!(
            npc_tile(engine, "tutorial3_dragon_sleeping").map(|t| local(engine, t)),
            Some((18, 33)),
            "the dragon asleep on its nest"
        );
        assert_eq!(
            npc_tile(engine, "tutorial3_knight_waitingforbattle").map(|t| local(engine, t)),
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
        talk_to(
            &mut client,
            npc_nid(engine, "tutorial3_knight_waitingforbattle"),
        );
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
            loc_id(engine, "tutorial3_knights_bag"),
            origin.0 + 27,
            origin.1 + 29,
        );
        let bag = converse(engine, &mut client, &[], 60);
        assert!(
            bag.said("You search Sir Vant's bag and take a selection of supplies."),
            "{:?} stage {} locs {:?} at {:?} msgs {:?}",
            bag.texts,
            progress(engine, &client),
            loc_ids(engine, origin.0 + 27, origin.1 + 29),
            cellar_tile(engine, &client),
            HarnessClient::game_messages(&bag.packets)
        );
        assert_eq!(progress(engine, &client), 96, "the supplies");
        // He stays by the pillars (2011 @244-366): an npc a script moves walks back to where it
        // was added once its mode goes back to its default (in-client run ufrest15, Sir Vant back
        // in the doorway (32, 16) for the sabotage).
        cycles(engine, &mut client, 100);
        let (vx, vz) = npc_tile(engine, "tutorial3_knight_waitingforbattle")
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
        talk_to(
            &mut client,
            npc_nid(engine, "tutorial3_knight_waitingforbattle"),
        );
        let talk = converse(engine, &mut client, &[], 120);
        assert!(
            talk.said("We need to destroy these pillars to collapse the ceiling on the dragon.")
        );
        assert_eq!(progress(engine, &client), 95);

        // The bag by the pillars on (27, 29) (tutorial3_2's Bag of supplies); in stage 2's start room
        // a plain barrel stands where it was (35, 13).
        let bag_id = loc_id(engine, "tutorial3_knights_bag");
        let (sx, sz) = at(35, 13);
        assert!(
            !loc_ids(engine, sx, sz).contains(&bag_id),
            "no bag in the start room"
        );
        assert!(loc_ids(engine, sx, sz).contains(&loc_id(engine, "tutorial3_barrel")));
        engine.harness_telejump(client.pid, origin.0 + 27, 0, origin.1 + 31);
        cycles(engine, &mut client, 3);
        let (bx, bz) = at(27, 29);
        oploc1(&mut client, bag_id, bx, bz);
        let bag = converse(engine, &mut client, &[], 60);
        assert!(bag.said("You search Sir Vant's bag and take a selection of supplies."));
        assert_eq!(progress(engine, &client), 96);
        let names = [
            "tinderbox",
            "tutorial3_bigcloth",
            "tutorial3_fuse_1",
            "tutorial3_fuse_2",
            "tutorial3_oilcan",
        ];
        for name in names {
            assert!(
                slot_of(engine, &client, obj_id(engine, name)).is_some(),
                "{name}"
            );
        }

        // Rip, soak both rags.
        let cloth = obj_id(engine, "tutorial3_bigcloth");
        let c = slot_of(engine, &client, cloth).unwrap();
        held1(&mut client, cloth, c);
        assert!(
            converse(engine, &mut client, &[], 30)
                .said("You rip the cloth and end up with two rags.")
        );
        let rag = obj_id(engine, "tutorial3_rag");
        let oil = obj_id(engine, "tutorial3_oilcan");
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
        let oily = obj_id(engine, "tutorial3_rag_oily");
        let fuses = [
            obj_id(engine, "tutorial3_fuse_1"),
            obj_id(engine, "tutorial3_fuse_2"),
        ];
        for (i, (px, pz)) in [(24u16, 31u16), (24, 39)].iter().enumerate() {
            let (wx, wz) = at(*px, *pz);
            let pillar = if i == 0 {
                "tutorial3_pillar_wooden_left"
            } else {
                "tutorial3_pillar_wooden_right"
            };
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
        assert_eq!(varbit_named(engine, &client, "tutorial3_leftpillar"), 2);
        assert_eq!(varbit_named(engine, &client, "tutorial3_rightpillar"), 2);
        assert_eq!(progress(engine, &client), 110);

        // The tinderbox to Sir Vant, then the far pillar.
        let tinderbox = obj_id(engine, "tinderbox");
        let t = slot_of(engine, &client, tinderbox).unwrap();
        use_on_npc(
            &mut client,
            tinderbox,
            t,
            npc_nid(engine, "tutorial3_knight_waitingforbattle"),
        );
        let given = converse(engine, &mut client, &[], 60);
        assert!(given.said("You offer Sir Vant a tinderbox."));
        assert_eq!(progress(engine, &client), 115);
        let t = slot_of(engine, &client, tinderbox).unwrap();
        let (wx, wz) = at(24, 39);
        use_on_loc(
            &mut client,
            tinderbox,
            t,
            loc_id(engine, "tutorial3_pillar_wooden_right"),
            wx,
            wz,
        );
        // s037, then the cutscene a tick at a time up to s038 (4KO2h2xPtdM 2011-04-30, frame by
        // frame): the black 6 ticks (@405.97-409.64), shot 203 9 ticks (@409.64-415.05) with
        // "Wow!" 5 ticks in (@412.4-412.6), the cupboard door a tick into shot 204 (@415.4-415.6),
        // Roddeck two ticks in (@416.3-416.5), s038 seven ticks in (@419.0-419.2).
        let lit = converse(engine, &mut client, &[], 30);
        assert!(lit.said("It's burning! Run!"), "{:?}", lit.texts);
        assert!(
            matches!(shot(engine, &client), 0 | 201),
            "s037 ends before the black"
        );
        let mut texts = lit.texts;
        let mut packets = lit.packets;
        let door_open = loc_id(engine, "tutorial3_wardrobe_door_open");
        let (dx, dz) = at(37, 30);
        // Each tick: the shot, whether "Wow!" went out, the door open, Roddeck there, s038.
        let mut ticks: Vec<CollapseTick> = Vec::new();
        let mut vant = vec![npc_trace(engine, "tutorial3_knight_waitingforbattle")];
        for _ in 0..80 {
            let sent = cycles(engine, &mut client, 1);
            let lines: Vec<String> = rs_test::talk::settexts(&sent)
                .into_iter()
                .map(|(_, t)| t)
                .collect();
            let s038 = said(&lines, "Ah, here you are, Roddeck.");
            let now = shot(engine, &client);
            ticks.push((
                now,
                sent_text(&sent, "Wow!"),
                loc_ids(engine, dx, dz).contains(&door_open),
                npc_tile(engine, "tutorial3_advisor_walkoverrocks").is_some(),
                s038,
            ));
            vant.push(npc_trace(engine, "tutorial3_knight_waitingforbattle"));
            if now == 203 {
                assert_eq!(
                    cellar_tile(engine, &client),
                    Some((25, 34)),
                    "the player before the rubble"
                );
                assert_eq!(
                    npc_tile(engine, "tutorial3_knight_waitingforbattle").map(|t| local(engine, t)),
                    Some((27, 38)),
                    "Sir Vant north-east of the player: {vant:?}"
                );
                assert_eq!(varbit_named(engine, &client, "tutorial3_leftpillar"), 5);
                assert_eq!(varbit_named(engine, &client, "tutorial3_rightpillar"), 5);
            }
            texts.extend(lines);
            packets.extend(sent);
            if s038 {
                break;
            }
        }
        // The rock fall (4KO2h2xPtdM @404.87-405.97): seven MAP_ANIMs (zone prot 6: the tile in
        // its zone, the spotanim, a height, the wait in client cycles, a zero) in one tick's
        // UPDATE_ZONE_PARTIAL_ENCLOSEDs, 14 ticks into shot 201 and 2 before the black.
        let mut rocks: Vec<(usize, u8, u16, u16)> = Vec::new();
        for (i, (op, body)) in packets.iter().enumerate() {
            if *op != ServerProt::UpdateZonePartialEnclosed as u16 {
                continue;
            }
            for j in 0..body.len().saturating_sub(7) {
                let anim = u16::from_be_bytes([body[j + 2], body[j + 3]]);
                if body[j] == ZoneProt::MapAnim as u8
                    && (anim == 2149 || anim == 2150)
                    && body[j + 4] == 0
                    && body[j + 7] == 0
                {
                    let wait = u16::from_be_bytes([body[j + 5], body[j + 6]]);
                    rocks.push((tick_of(&packets, i, true), body[j + 1], anim, wait));
                }
            }
        }
        let mut fall: Vec<(u8, u16, u16)> = rocks.iter().map(|r| (r.1, r.2, r.3)).collect();
        fall.sort_by_key(|r| r.2);
        assert_eq!(
            fall,
            vec![
                (0x17, 2149, 0),
                (0x05, 2150, 2),
                (0x04, 2149, 10),
                (0x71, 2149, 14),
                (0x50, 2150, 22),
                (0x22, 2150, 33),
                (0x56, 2150, 44),
            ],
            "the rock fall: {rocks:?}"
        );
        let fell = rocks[0].0;
        assert!(rocks.iter().all(|r| r.0 == fell), "one tick: {rocks:?}");
        // 201 and 202 do not fit VARC_SMALL's signed byte: VARC_LARGE, `p4_alt1 value, p2_alt3 varc`.
        let shot_at = |n: i32| {
            let i = packets
                .iter()
                .position(|(op, p)| {
                    *op == ServerProt::VarcLarge as u16
                        && p.len() >= 6
                        && u16::from_le_bytes([p[4].wrapping_sub(128), p[5]]) == SHOT
                        && i32::from_le_bytes([p[0], p[1], p[2], p[3]]) == n
                })
                .unwrap_or_else(|| panic!("no shot {n}"));
            tick_of(&packets, i, false)
        };
        assert_eq!(shot_at(202) - shot_at(201), 16, "shot 201: {ticks:?}");
        assert_eq!(shot_at(202) - fell, 2, "the rocks before the black");
        let first = |what: &str, f: &dyn Fn(&CollapseTick) -> bool| {
            ticks
                .iter()
                .position(f)
                .unwrap_or_else(|| panic!("no {what}: {ticks:?}"))
        };
        let count = |n: i32| ticks.iter().filter(|t| t.0 == n).count();
        let s203 = first("shot 203", &|t| t.0 == 203);
        let s204 = first("shot 204", &|t| t.0 == 204);
        assert_eq!(count(202), 6, "the black: {ticks:?}");
        assert_eq!(count(203), 9, "shot 203: {ticks:?}");
        assert_eq!(first("Wow!", &|t| t.1) - s203, 5, "Wow!: {ticks:?}");
        assert_eq!(first("door", &|t| t.2) - s204, 1, "the door: {ticks:?}");
        assert_eq!(first("Roddeck", &|t| t.3) - s204, 2, "Roddeck: {ticks:?}");
        assert_eq!(first("s038", &|t| t.4) - s204, 7, "s038: {ticks:?}");
        let (more, rest) = play_out(engine, &mut client, &[], 300);
        texts.extend(more);
        packets.extend(rest);
        assert!(said(&texts, "Ah, here you are, Roddeck."), "{texts:?}");
        assert!(said(&texts, "Next time, get a puppy!"), "{texts:?}");
        // The release (4KO2h2xPtdM @427.16-429.06): the minimap and the kit-ask box on the tick
        // the camera is released, the Task panel a tick on, the banner only once the kit-ask
        // box has been continued.
        let zero = shot_zero_at(&packets).expect("varc 1007 back at 0");
        let at = |what: &str, i: Option<usize>| {
            i.unwrap_or_else(|| {
                panic!(
                    "no {what}: {texts:?} {}",
                    around(&packets, zero, packets.len())
                )
            })
        };
        let minimap = at(
            "minimap",
            packets
                .iter()
                .rposition(|(op, b)| *op == ServerProt::MinimapToggle as u16 && b[0] == 0),
        );
        let kit = at(
            "kit-ask",
            packets.iter().position(|(_, b)| {
                let t = b"could I have my squire's kit";
                b.windows(t.len()).any(|w| w == t)
            }),
        );
        let tab = at(
            "Task panel",
            packets.iter().rposition(|(op, b)| {
                *op == ServerProt::RunClientScript as u16
                    && b.len() >= 8
                    && b[b.len() - 8..] == [0, 0, 0, 1, 0, 0, 0, 115]
            }),
        );
        let banner = at("banner", popup_at(&packets, 3512));
        let released = tick_of(&packets, zero, false);
        let ctx = format!(
            "zero {zero} minimap {minimap} kit {kit} tab {tab} banner {banner}: {}",
            around(&packets, zero, tab.max(banner))
        );
        assert_eq!(
            tick_of(&packets, minimap, true),
            released,
            "the minimap: {ctx}"
        );
        assert_eq!(
            tick_of(&packets, kit, true),
            released,
            "the kit-ask box: {ctx}"
        );
        assert_eq!(
            tick_of(&packets, tab, true),
            released + 1,
            "the Task panel: {ctx}"
        );
        assert!(
            tick_of(&packets, banner, false) > released + 1,
            "the banner after the kit-ask box: {ctx}"
        );
        assert!(
            said(&texts, "could I have my squire's kit back, please?"),
            "{texts:?}"
        );
        assert_eq!(progress(engine, &client), 125, "Return the Squire's Gear");
        assert_eq!(
            npc_tile(engine, "tutorial3_dragon_sleeping"),
            None,
            "the dragon is under the rubble"
        );
        // Roddeck walks from his cupboard to the player's side (the minimap @428.5 and @431: his
        // dot a tile south and a tile east of the player's).
        assert_eq!(
            npc_tile(engine, "tutorial3_advisor_walkoverrocks").map(|t| local(engine, t)),
            Some((26, 33)),
            "Roddeck beside the player"
        );
        assert_eq!(cellar_tile(engine, &client), Some((25, 34)));
        // And stays: a walked npc heads back to the tile it was added on once its mode is its
        // default again, here the cupboard.
        cycles(engine, &mut client, 100);
        assert_eq!(
            npc_tile(engine, "tutorial3_advisor_walkoverrocks").map(|t| local(engine, t)),
            Some((26, 33)),
            "Roddeck stays beside the player"
        );
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
        // Running in the cellar does not do On the Run (4KO2h2xPtdM runs @186-198, the Task @680).
        run_orb(&mut client);
        cycles(engine, &mut client, 1);
        walk_near(engine, &mut client);
        cycles(engine, &mut client, 3);
        assert_eq!(
            varbit_named(engine, &client, "varplayerbit_8522"),
            0,
            "not in the tutorial"
        );
        run_orb(&mut client); // off again
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
            "tutorial3_oilcan",
        ];
        for (slot, name) in names.iter().enumerate() {
            give(engine, &client, slot as u16, obj_id(engine, name), 1);
        }
        cycles(engine, &mut client, 1);
        let vant = npc_nid(engine, "tutorial3_knight_waitingforbattle");
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

        talk_to(
            &mut client,
            npc_nid(engine, "tutorial3_advisor_walkoverrocks"),
        );
        let choice = converse(engine, &mut client, &["I'll mine through the rocks."], 160);
        assert!(choice.said("Roddeck gives you a pickaxe."));
        assert_eq!(progress(engine, &client), 135);
        // The Task panel's overview: Discover an Exit, Breaking Through, four empty
        // (4KO2h2xPtdM @532).
        assert_eq!(varbit(engine, &client, PINNED_TASK), 4091, "the overview");
        assert_eq!(varbit(engine, &client, 8587), 3514);
        assert_eq!(varbit(engine, &client, 8588), 3516);
        assert_eq!(varbit(engine, &client, 8589), 4091);

        // The blockage at level 1.
        engine.harness_telejump(client.pid, origin.0 + 15, 0, origin.1 + 35);
        cycles(engine, &mut client, 3);
        let (rx, rz) = at(14, 35);
        oploc1(
            &mut client,
            loc_id(engine, "tutorial3_mine_wall_start"),
            rx,
            rz,
        );
        assert!(
            converse(engine, &mut client, &[], 60)
                .said("You'll need a Mining level of 2 to clear the rocks.")
        );
        // Its continue: the practice rocks live (varbit 6495 = 1) and the stats stone with its
        // arrow, with no word from Roddeck (4KO2h2xPtdM @549.0).
        assert_eq!(progress(engine, &client), 140);
        assert_eq!(varbit(engine, &client, 6495), 1, "the practice rocks");
        assert_eq!(varbit(engine, &client, FLASH_SIDE), 3, "the stats stone");
        assert_eq!(varbit(engine, &client, 8589), 3518, "Skill in Mining");
        engine.harness_telejump(client.pid, origin.0 + 31, 0, origin.1 + 32);
        cycles(engine, &mut client, 3);
        talk_to(
            &mut client,
            npc_nid(engine, "tutorial3_advisor_walkoverrocks"),
        );
        let spot = converse(engine, &mut client, &[], 200);
        assert!(spot.said("You can practice mining on the rocks that fell into my cellar"));
        assert_eq!(progress(engine, &client), 145);

        // Two practice rocks to level 2: the practice rock on (17, 38) (the map's (17, 14), the block
        // moved north by the collapse), live at varbit 6495 = 1.
        engine.harness_telejump(client.pid, origin.0 + 17, 0, origin.1 + 37);
        cycles(engine, &mut client, 3);
        let (px, pz) = at(17, 38);
        let mut practice = Vec::new();
        for _ in 0..2 {
            oploc1(
                &mut client,
                loc_id(engine, "tutorial3_training_rock_1_multi"),
                px,
                pz,
            );
            practice.extend(converse(engine, &mut client, &[], 120).packets);
            practice.extend(cycles(engine, &mut client, 4));
        }
        assert_eq!(progress(engine, &client), 150, "Mining level 2");
        // The level's box, then Roddeck, the panel on Breaking Through's summary
        // (4KO2h2xPtdM @568-571.2).
        let boxes: Vec<String> = rs_test::talk::settexts(&practice)
            .into_iter()
            .map(|(_, t)| t)
            .collect();
        let level = boxes
            .iter()
            .position(|t| t.contains("You have reached level 2 Mining."));
        let roddeck = boxes
            .iter()
            .position(|t| t.contains("Congratulations, you've reached level 2."));
        assert!(level.is_some() && level < roddeck, "{boxes:?}");
        assert_eq!(varbit(engine, &client, PINNED_TASK), 3516);
        // The Task System's How To Train and Level (425) once the boxes are closed, its line alone
        // (a first Task), and no Skill in Mining (3518) popup (4KO2h2xPtdM @569-577).
        let lines: Vec<String> = HarnessClient::game_messages(&practice)
            .into_iter()
            .map(|(_, m)| m)
            .collect();
        assert!(
            lines.contains(&"You have completed the Task 'How To Train and Level'!".to_string()),
            "{lines:?}"
        );
        assert!(!lines.iter().any(|m| m.contains("in total")), "{lines:?}");
        assert_eq!(varbit_named(engine, &client, "task_tut4_how_to_2"), 1);
        assert_eq!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .varcs
                .get(POPUP_TASK)
                .as_int(),
            425,
            "TASK COMPLETE! How To Train and Level"
        );

        engine.harness_telejump(client.pid, origin.0 + 15, 0, origin.1 + 35);
        cycles(engine, &mut client, 3);
        oploc1(
            &mut client,
            loc_id(engine, "tutorial3_mine_wall_start"),
            rx,
            rz,
        );
        let cleared = converse(engine, &mut client, &[], 120);
        assert!(cleared.said("You succeed in clearing the rocks from the hole."));
        assert_eq!(progress(engine, &client), 160);
        oploc1(
            &mut client,
            loc_id(engine, "tutorial3_mine_wall_end"),
            rx,
            rz,
        );
        // The click: TASK COMPLETE! Breaking Through; through the hole Roddeck takes his
        // pickaxe back; TASK COMPLETE! Discover an Exit once that box is closed
        // (4KO2h2xPtdM @582.9-589.7).
        let pickaxe = obj_id(engine, "bronze_pickaxe");
        assert!(slot_of(engine, &client, pickaxe).is_some());
        let hole = cycles(engine, &mut client, 8);
        assert_eq!(progress(engine, &client), 170, "To Freedom!");
        assert!(slot_of(engine, &client, pickaxe).is_none());
        assert_eq!(varbit(engine, &client, PINNED_TASK), 3519);
        assert!(
            rs_test::talk::settexts(&hole)
                .iter()
                .any(|(_, t)| t.contains("Roddeck retrieves his pickaxe from you."))
        );
        let popup = engine
            .harness_player(client.pid)
            .unwrap()
            .varcs
            .get(POPUP_TASK)
            .as_int();
        assert_eq!(popup, 3516, "TASK COMPLETE! Breaking Through");
        converse(engine, &mut client, &[], 20);
        cycles(engine, &mut client, 2);
        let popup = engine
            .harness_player(client.pid)
            .unwrap()
            .varcs
            .get(POPUP_TASK)
            .as_int();
        assert_eq!(popup, 3514, "TASK COMPLETE! Discover an Exit");

        // The ladder up: Explorer Jack's house, the quest's end.
        engine.harness_telejump(client.pid, origin.0 + 6, 0, origin.1 + 44);
        cycles(engine, &mut client, 3);
        let (lx, lz) = at(6, 45);
        oploc1(
            &mut client,
            loc_id(engine, "tutorial3_cellar_ladder_exit"),
            lx,
            lz,
        );
        let up = cycles(engine, &mut client, 8);
        assert_eq!(progress(engine, &client), 1000, "the quest complete");
        // The house is a large copy of Lumbridge: 2011's minimap shows the town all round it,
        // castle and streets, with no edge (4KO2h2xPtdM @606-655). Forty tiles out from the
        // player's (3207, 3240) is still copied Lumbridge.
        let here = engine.harness_player(client.pid).unwrap().pathing.coord;
        let far = CoordGrid::new(here.x() - 40, 0, here.z() + 40);
        assert_eq!(
            engine.regions.source(far).map(|c| (c.x(), c.z())),
            Some((3167, 3280)),
            "Lumbridge round the house"
        );
        let messages: Vec<String> = HarnessClient::game_messages(&up)
            .into_iter()
            .map(|(_, m)| m)
            .collect();
        // Once, then Harmony's unlock (4KO2h2xPtdM @608.5; in-client run 19 printed it twice).
        let complete: Vec<usize> = (0..messages.len())
            .filter(|&i| messages[i] == "Congratulations! Quest complete!")
            .collect();
        assert_eq!(complete.len(), 1, "{messages:?}");
        let harmony = messages.iter().position(|m| m.contains("Harmony"));
        assert!(harmony.is_some_and(|h| h > complete[0]), "{messages:?}");
        // This quest's scroll reads "You gain:" over its rewards (@608.5), 277:9's own text being
        // "You are awarded:".
        assert!(
            rs_test::talk::settexts(&up)
                .iter()
                .any(|(_, t)| t == "You gain:"),
            "the scroll's You gain:"
        );
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.stats.base_levels[14], 4, "Mining 4");

        // The scroll closed, the level-up queued behind it runs: 2011's one line for the two
        // levels and no level-up box (4KO2h2xPtdM @610-636, VwpdbjfigD4 @788-796). TASK COMPLETE!
        // To Freedom! (3519) pops only then, not over the scroll (@611).
        let pid = client.pid;
        let popup = |engine: &Engine| {
            engine
                .harness_player(pid)
                .unwrap()
                .varcs
                .get(POPUP_TASK)
                .as_int()
        };
        assert_ne!(popup(engine), 3519, "no To Freedom! over the scroll");
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
        assert_eq!(popup(engine), 3519, "TASK COMPLETE! To Freedom!");
        // Pinned till Jack is talked to: Introducing...Explorer Jack (@607-640), then the Task
        // after it.
        assert_eq!(
            varbit(engine, &client, PINNED_TASK),
            3520,
            "Introducing...Explorer Jack"
        );
        talk_to(&mut client, npc_nid(engine, "tutorial3_explorer_jack"));
        let jack = converse(engine, &mut client, &[], 60);
        assert!(
            jack.said("Good grief! Where did you three come from?"),
            "{:?}",
            jack.texts
        );
        assert!(jack.said("Look at the Rewards tab for a Task to learn more."));
        assert_eq!(varbit_named(engine, &client, "tutorial3_postquest"), 3);
        // On the Run (363) on the first walk with run on after the tutorial (Qh_-AMJpVd0
        // 2011-05-10 @432.5, in this house).
        run_orb(&mut client);
        cycles(engine, &mut client, 1);
        walk_near(engine, &mut client);
        // The step, the soft timer seeing it, the queue completing the Task.
        let ran = cycles(engine, &mut client, 3);
        assert!(
            HarnessClient::game_messages(&ran)
                .iter()
                .any(|(_, m)| m == "You have completed the Task 'On the Run'!"),
            "{:?}",
            HarnessClient::game_messages(&ran)
        );
        assert_eq!(varbit_named(engine, &client, "varplayerbit_8522"), 1);
        assert_eq!(
            varbit(engine, &client, PINNED_TASK),
            3521,
            "The Journey Begins"
        );
    });
}

/// Roddeck's cellar's bookcase and crate, and the tutorial's eat and drop boxes (4KO2h2xPtdM
/// @479.9-516, @574-576).
#[test]
fn the_books_the_cabbage_and_the_drop_hint() {
    runner::on_loaded_engine(|engine| {
        let mut client = at_stage(engine, "uf_shelf", 135);
        let origin = player_origin(engine, &client);
        let diary = obj_id(engine, "tutorial3_diary");
        let manual = obj_id(engine, "tutorial3_book");
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
        let crate_loc = loc_id(engine, "tutorial3_cellar_crate_cabbage");
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

        // The diary dropped and taken back: "You won't be needing that again." (the 2011 wiki's
        // Roddeck's diary), N9 (clip 1558); it stays where it lies.
        let slot = slot_of(engine, &client, diary).expect("the diary");
        let mut p = ClientPacket::new();
        p.p4(i32::from(INVENTORY) << 16)
            .p2_alt3(i32::from(diary))
            .p2_alt1(i32::from(slot));
        client.send(ClientProt::IfButton8, &p.finish());
        converse(engine, &mut client, &[], 60);
        assert!(slot_of(engine, &client, diary).is_none(), "dropped");
        let here = engine.harness_player(client.pid).unwrap().pathing.coord;
        crate::take(&mut client, diary, here.x(), here.z());
        let back = cycles(engine, &mut client, 3);
        assert!(
            HarnessClient::game_messages(&back)
                .iter()
                .any(|(_, m)| m == "You won't be needing that again."),
            "{:?}",
            HarnessClient::game_messages(&back)
        );
        assert!(speech(&back).contains(&1558));
        assert!(slot_of(engine, &client, diary).is_none(), "not taken");
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
        // Where the collapse leaves the three of them (4KO2h2xPtdM @428.5-431).
        for tile in [(25, 34), (26, 33), (27, 38)] {
            assert!(seen.contains(&tile), "{tile:?} is floor:\n{map}");
        }
    });
}
