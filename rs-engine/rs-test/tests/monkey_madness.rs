//! Monkey Madness at 647 (`quests/quest_mm`): 289's quest on 289's tiles. Its varbits are procs on
//! cache 786's varps (cache 786 has no varbit on them), its chapter screens are interface 225
//! (`mm_message`) and its sliding puzzle is interface 287 (`reinitialisation_puzzle`), whose tiles
//! are components with op 1, Slide.

#![cfg(all(since_647, before_742))]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_grid::CoordGrid;
use rs_pack::cache::VarValue;
use rs_pack::cache::dbrow::DbRowValue;
use rs_pack::types::NpcStat;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::names::BACKPACK;
use rs_test::runner;
use rs_test::server;
use rs_test::talk::{Conversation, converse, settexts, talk_to};
use rs_test::world;
use rs_ver::TypeId;
use rs_vm::ops::test::{calls_for, forget_calls, set_recording};
use rs_vm::subject::ScriptSubject;
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;
/// The quest list's rows, and Monkey Madness's (`^questlist_mm`).
const QUEST_LIST: i32 = 190 << 16 | 18;
const ROW: i32 = 73;
/// `^grandtree_complete` and `^tree_complete`: Narnode only speaks of the 10th squad after both.
const GRANDTREE_COMPLETE: i32 = 160;
const TREE_COMPLETE: i32 = 9;
/// `^mm_started`.
const MM_STARTED: i32 = 1;
/// `^daero_started_reinit`, in bits 7-10 of `%mm_gnomes` (289's `mm_daero` varbit).
const DAERO_STARTED_REINIT: i32 = 5 << 7;
/// Cache 786's reinitialisation puzzle: interface 287, its bottom right tile column5_row5.
const PUZZLE: u16 = 287;
const LAST_TILE: i32 = 287 << 16 | 29;
/// The client var the puzzle's clientscript (mm_reinit_refresh) picks its inv by.
const MM_REINIT_ACTIVE: u16 = 1032;

/// Cache 786's spawns and locs (289's tiles).
const KING: (u16, u16) = (2466, 3497);
/// bunker_controlpanal at 0_37_154_26_27.
const CONTROL_PANEL: (u16, u16) = (2394, 9883);
/// Ardougne Zoo's monkey pen (in 289's greegree zone 0_40_51_32_1..0_41_51_15_23: on Ape Atoll
/// itself the guards jail a human), and Ardougne, outside every greegree zone.
const ZOO_MONKEYS: (u16, u16) = (2631, 3275);
const ARDOUGNE: (u16, u16) = (2662, 3305);

fn id(engine: &Engine, kind: &str, name: &str) -> TypeId {
    let map = match kind {
        "obj" => &engine.cache.objs.debugnames,
        "npc" => &engine.cache.npcs.debugnames,
        "loc" => &engine.cache.locs.debugnames,
        "varp" => &engine.cache.varps.debugnames,
        "inv" => &engine.cache.invs.debugnames,
        _ => unreachable!(),
    };
    *map.get(name).unwrap_or_else(|| panic!("no {kind} {name}"))
}

fn var(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let v = id(engine, "varp", name);
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(v.g2())
        .as_int()
}

fn set_var(engine: &mut Engine, client: &HarnessClient, name: &str, value: i32) {
    let v = id(engine, "varp", name);
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(v.g2(), VarValue::Int(value));
}

fn login(engine: &mut Engine, name: &str) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    set_var(engine, &client, "grandtree", GRANDTREE_COMPLETE);
    set_var(engine, &client, "treequest", TREE_COMPLETE);
    seasoned(engine, &client);
    client
}

/// Level 99 in every skill (its xp, in tenths) and full life.
fn seasoned(engine: &mut Engine, client: &HarnessClient) {
    let player = engine.harness_player_mut(client.pid).unwrap();
    for skill in 0..player.stats.levels.len() {
        player.stats.levels[skill] = 99;
        player.stats.base_levels[skill] = 99;
        player.stats.xp[skill] = 130_344_310;
    }
    let bit0 = player.vars.get(LIFE).as_int() & 1;
    player.vars.set(LIFE, VarValue::Int((990 << 1) | bit0));
}

fn no_errors(engine: &mut Engine) {
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "no script errors: {errors:?}");
}

fn settle(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) {
    for _ in 0..ticks {
        engine.cycle();
    }
    client.packets();
}

fn walkable(x: u16, level: u8, z: u16) -> bool {
    !rsmod::is_flagged(x, z, level, 0x100 | 0x200000)
}

/// The nearest walkable tile to `by` (the tile itself first). When `beside`, a tile beside `by`
/// that `by` itself can step to (`world::beside`): a tile walkable by its flags can be cut off
/// from it, as (2483, 3485) is from Daero on (2484, 3486) up the Grand Tree, and an op from there
/// ends in "I can't reach that!".
fn walkable_near((x, z): (u16, u16), level: u8, beside: bool) -> (u16, u16) {
    if beside {
        return world::beside(level, x, z).unwrap_or_else(|| panic!("a tile beside ({x}, {z})"));
    }
    for r in 0i16..=4 {
        for dx in -r..=r {
            for dz in -r..=r {
                if dx.abs().max(dz.abs()) != r {
                    continue;
                }
                let (ax, az) = (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz));
                if walkable(ax, level, az) {
                    return (ax, az);
                }
            }
        }
    }
    panic!("a walkable tile near ({x}, {z})");
}

fn tele(engine: &mut Engine, client: &mut HarnessClient, at: (u16, u16), beside: bool) {
    let (x, z) = walkable_near(at, 0, beside);
    engine.harness_telejump(client.pid, x, 0, z);
    settle(engine, client, 3);
}

fn give(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str) {
    let o = id(engine, "obj", name);
    let inv = id(engine, "inv", "inv");
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap()
        .set(slot, o, 1);
}

fn held(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let o = id(engine, "obj", name);
    let inv = id(engine, "inv", "inv");
    engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap()
        .total(o) as i32
}

fn says(t: &Conversation, lines: &[&str]) {
    for line in lines {
        assert!(
            t.said(line),
            "289 has {line:?}; {:?} (chat {:?})",
            t.texts,
            t.messages
        );
    }
}

/// What the chat and the boxes are told over `ticks`.
fn watch(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) -> Conversation {
    let mut t = Conversation::default();
    for _ in 0..ticks {
        engine.cycle();
        let packets = client.packets();
        t.texts
            .extend(settexts(&packets).into_iter().map(|(_, text)| text));
        t.messages.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, m)| m),
        );
    }
    t
}

/// King Narnode, after The Grand Tree and Tree Gnome Village, tells of the missing 10th squad and
/// starts the quest with the royal seal; the journal then reads the seal's stage back (289's
/// `mm_narnode` varbit, a proc pair on `%mm_gnomes` here).
#[test]
fn king_narnode_starts_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mmstart");
        let want = id(engine, "npc", "grandtree_narnode");
        let (nid, nx, nz) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .filter(|n| n.npc.uid.id() == want)
            .map(|n| {
                let c = n.npc.pathing.coord;
                (n.npc.uid.nid(), c.x(), c.z())
            })
            .min_by_key(|n| n.1.abs_diff(KING.0) + n.2.abs_diff(KING.1))
            .expect("King Narnode is spawned");
        tele(engine, &mut client, (nx, nz), true);
        talk_to(&mut client, nid);
        let t = converse(engine, &mut client, &["Yes"], 400);
        says(
            &t,
            &[
                "King, you look worried. Is anything the matter?",
                "Start Monkey Madness?",
            ],
        );
        assert_eq!(var(engine, &client, "mm_main"), MM_STARTED);
        assert_eq!(held(engine, &client, "mm_gnome_royal_seal"), 1);

        let mut p = ClientPacket::new();
        p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(ROW);
        client.send(ClientProt::IfButton1, &p.finish());
        let t = watch(engine, &mut client, 3);
        assert!(
            t.texts
                .iter()
                .any(|l| l.contains("Narnode has given me a copy of the Royal Seal")),
            "289's journal at the seal's stage: {:?}",
            t.texts
        );
        no_errors(engine);
    });
}

/// A greegree works only in its zones (Ape Atoll, Ardougne Zoo's monkeys): there the player
/// becomes its monkey (cache 786's mm_transmogrification npc), elsewhere nothing happens.
#[test]
fn a_greegree_turns_the_player_into_a_monkey_in_its_zones() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mmgreegree");
        let greegree = id(engine, "obj", "mm_monkey_greegree_for_small_ninja_monkey");
        let monkey = id(engine, "npc", "mm_transmogrification_small_ninja_monkey");

        tele(engine, &mut client, ARDOUGNE, false);
        give(
            engine,
            &client,
            0,
            "mm_monkey_greegree_for_small_ninja_monkey",
        );
        client.if_button(2, BACKPACK, 0, greegree);
        let t = watch(engine, &mut client, 3);
        assert!(
            t.messages
                .iter()
                .any(|m| m == "You attempt to use the Monkey Greegree but nothing happens."),
            "{:?}",
            t.messages
        );
        assert_eq!(engine.harness_player(client.pid).unwrap().transmog, None);

        // The zoo's brown bears share the zone and wander up to the player: a bear's swing makes
        // the player strike back, which as a monkey opens "You cannot attack as a monkey.", and
        // with that box open the greegree's timer cannot take it off.
        world::calm(engine, ZOO_MONKEYS.0, ZOO_MONKEYS.1, 12);
        tele(engine, &mut client, ZOO_MONKEYS, false);
        // Out of combat: the clock is back at 0 after the harness reset, below a saved %lastcombat.
        set_var(engine, &client, "lastcombat", -1);
        client.if_button(2, BACKPACK, 0, greegree);
        settle(engine, &mut client, 3);
        assert_eq!(
            held(engine, &client, "mm_monkey_greegree_for_small_ninja_monkey"),
            0
        );
        assert_eq!(
            engine.harness_player(client.pid).unwrap().transmog,
            Some(monkey.g2()),
            "a small ninja monkey"
        );

        // Out of the zone, the greegree's timer takes it off.
        tele(engine, &mut client, ARDOUGNE, false);
        settle(engine, &mut client, 3);
        assert_eq!(engine.harness_player(client.pid).unwrap().transmog, None);
        no_errors(engine);
    });
}

/// The control panel opens cache 786's puzzle screen (interface 287) on the real puzzle, and its
/// tile's Slide moves it: the last tile into place finishes the puzzle.
#[test]
fn the_reinitialisation_puzzle_slides_on_2011_s_screen() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mmpuzzle");
        set_var(engine, &client, "mm_main", 5);
        set_var(engine, &client, "mm_gnomes", DAERO_STARTED_REINIT);
        tele(engine, &mut client, CONTROL_PANEL, true);

        let panel = id(engine, "loc", "bunker_controlpanal");
        let mut p = ClientPacket::new();
        p.p2_alt1(CONTROL_PANEL.1 as i32)
            .p2_alt1(panel.int())
            .p1(0)
            .p2_alt2(CONTROL_PANEL.0 as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        let mut sent = Vec::new();
        for _ in 0..8 {
            engine.cycle();
            sent.extend(client.packets());
        }
        assert_eq!(
            engine.harness_player(client.pid).unwrap().modal_main,
            Some(PUZZLE),
            "the puzzle screen is open"
        );
        assert!(
            server::varcs(&sent).contains(&(MM_REINIT_ACTIVE, 1)),
            "the screen draws the real puzzle: {:?}",
            server::varcs(&sent)
        );

        // One move from done: tiles 1-23 in place, the gap at slot 23, tile 24 in slot 24.
        let inv = id(engine, "inv", "reinitialisation_inv");
        let tiles: Vec<TypeId> = (1..=24)
            .map(|n| id(engine, "obj", &format!("reinitialisation_{n:02}")))
            .collect();
        {
            let puzzle = engine
                .harness_player_mut(client.pid)
                .unwrap()
                .invs
                .get_mut(&inv)
                .expect("the puzzle's inv");
            puzzle.clear();
            for (slot, tile) in tiles.iter().enumerate().take(23) {
                puzzle.set(slot as u16, *tile, 1);
            }
            puzzle.set(24, tiles[23], 1);
        }
        let mut p = ClientPacket::new();
        p.p4(LAST_TILE).p2_alt3(-1).p2_alt1(-1);
        client.send(ClientProt::IfButton1, &p.finish());
        let t = watch(engine, &mut client, 3);
        assert!(
            t.messages
                .iter()
                .any(|m| m.starts_with("As you slide the final piece into place")),
            "{:?}",
            t.messages
        );
        // 289's mm_hangar_puzzle_complete, bit 4 of %mm_flags.
        assert_eq!(var(engine, &client, "mm_flags") >> 4 & 1, 1);
        no_errors(engine);
    });
}

fn life_points(engine: &Engine, client: &HarnessClient) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(LIFE)
        .as_int()
        >> 1
}

fn set_life_points(engine: &mut Engine, client: &HarnessClient, points: i32) {
    let player = engine.harness_player_mut(client.pid).unwrap();
    let bit0 = player.vars.get(LIFE).as_int() & 1;
    player.vars.set(LIFE, VarValue::Int((points << 1) | bit0));
}

/// The Ape Atoll food heals the 2011 wiki's life points (Monkey bar 90, not 289's 50), and the
/// banana stew leaves its bowl.
#[test]
fn the_ape_atoll_food_heals_2011_s_life_points() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mmfood");
        for (food, heal) in [
            ("mm_monkey_nuts", 40),
            ("mm_monkey_bar", 90),
            ("mm_banana_stew", 110),
        ] {
            set_life_points(engine, &client, 500);
            give(engine, &client, 0, food);
            let obj = id(engine, "obj", food);
            client.if_button(1, BACKPACK, 0, obj);
            let t = watch(engine, &mut client, 4);
            // Natural regeneration may add a life point over the ticks.
            let healed = life_points(engine, &client) - 500;
            assert!(
                (heal..=heal + 1).contains(&healed),
                "{food} heals {heal}: {healed}"
            );
            assert_eq!(
                held(engine, &client, food),
                0,
                "{food} is eaten: {:?}",
                t.messages
            );
        }
        assert_eq!(held(engine, &client, "bowl_empty"), 1, "the stew's bowl");
        no_errors(engine);
    });
}

/// The spawned `name` nearest `near` (any, without it): its nid, tile and level.
fn spawned(engine: &Engine, name: &str, near: Option<(u16, u16)>) -> Option<(u16, u16, u16, u8)> {
    let want = id(engine, "npc", name);
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == want)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.z(), c.y())
        })
        .min_by_key(|n| near.map_or(0, |(x, z)| n.1.abs_diff(x) + n.2.abs_diff(z)))
}

/// Beside the spawned `name` nearest `near`, on its level; its nid.
fn stand_beside(
    engine: &mut Engine,
    client: &mut HarnessClient,
    name: &str,
    near: (u16, u16),
) -> u16 {
    let (nid, nx, nz, level) =
        spawned(engine, name, Some(near)).unwrap_or_else(|| panic!("{name} is spawned"));
    let (x, z) = walkable_near((nx, nz), level, true);
    engine.harness_telejump(client.pid, x, level, z);
    settle(engine, client, 3);
    nid
}

/// `^mm_completed_ch3`: the sigil is given, the demon not yet fought.
const MM_COMPLETED_CH3: i32 = 5;

/// The 10th squad's sigil takes the player to the final battle: to Garkor's last words, as the
/// jungle demon spawns. Defence and Magic are 1, so the demon's spells land, and auto retaliate
/// is off, so the player leaves the demon to the 10th squad. Calls are recorded from the sigil's
/// click on.
fn start_the_final_battle(engine: &mut Engine, name: &str) -> (HarnessClient, Conversation) {
    let mut client = login(engine, name);
    {
        let player = engine.harness_player_mut(client.pid).unwrap();
        for skill in [1, 6] {
            player.stats.levels[skill] = 1;
            player.stats.base_levels[skill] = 1;
        }
    }
    tele(engine, &mut client, ARDOUGNE, false);
    set_var(engine, &client, "mm_main", MM_COMPLETED_CH3);
    set_var(engine, &client, "lastcombat", -1);
    // `^player_auto_retaliate_off`: only the 10th squad fights the demon.
    set_var(engine, &client, "option_nodef", 1);
    give(engine, &client, 0, "mm_sigil");
    let sigil = id(engine, "obj", "mm_sigil");
    set_recording(true);
    forget_calls();
    client.if_button(2, BACKPACK, 0, sigil);
    // The sigil's timer, the teleport and the cutscene, to Garkor's "the final battle begins!": the
    // demon spawns as his words close.
    let t = converse(engine, &mut client, &[], 200);
    says(&t, &["Ready yourself, human: the final battle begins!"]);
    (client, t)
}

/// `magic_spell_table`'s spotanim_proj and spotanim_target columns (skill_magic/configs/
/// magic.dbtable, counted from 0): the spell's travel spotanim and its impact, each the field's
/// first value.
const SPOTANIM_PROJ: usize = 18;
const SPOTANIM_TARGET: usize = 20;

/// The config id (a spotanim, a synth) a spell's row gives in `column`: none for a row that
/// leaves it out or has null.
fn row_ref(engine: &Engine, row: &str, column: usize) -> Option<i32> {
    let row = engine
        .cache
        .dbrows
        .get_by_debugname(row)
        .unwrap_or_else(|| panic!("no dbrow {row}"));
    match row.columns.as_ref()?.get(column)?.as_ref()?.first()? {
        DbRowValue::Int(id) if (0..0xFFFF).contains(id) => Some(*id),
        _ => None,
    }
}

/// Runs `run` and gives the script errors it raised.
///
/// The engine gathers script errors only while it has a database to send them to
/// (`ScriptEngine::script_error`), and a test engine has none, so `engine.script_errors` stays
/// empty and an npc's failed script tells no one: the engine is lent a sender for the call.
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

/// Zooknock's blast at the jungle demon in the final battle (`[ai_apnpc2,
/// mm_zooknock_final_battle]`) shows the spell table's graphics and no others. The port gave
/// him fire blast and water blast by raw id (spotanims 130 and 131, 136 and 137), ids no one has
/// checked against 2011. He now takes the travel and the impact from the spell's row of
/// magic_spell_table, as a player's cast does; 647's rows read as null, and a null one is not
/// shown. The rest of the cast stands: his own animation, and the demon's block when the blast
/// would land.
///
/// His cast of a row that has a travel spotanim launches it with `projanim_npc`, which fails in
/// the engine until main's fix for it is merged here (PR #59: "Invalid uid", the command took
/// the caster for the target). That error is the engine's and is let through; any other fails
/// the case, and while both rows are null there is none at all.
#[test]
fn zooknock_s_blast_shows_only_the_spell_table_s_graphics() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, _) = start_the_final_battle(engine, "mmzooknock");
        let mut fighters = None;
        for _ in 0..80 {
            set_life_points(engine, &client, 990);
            engine.cycle();
            client.packets();
            let zooknock = spawned(engine, "mm_zooknock_final_battle", None);
            let demon = spawned(engine, "mm_demon", None);
            if let (Some(zooknock), Some(demon)) = (zooknock, demon) {
                fighters = Some((zooknock.0, demon.0));
                break;
            }
        }
        let (zooknock, demon) = fighters.expect("Zooknock and the jungle demon are there");
        let (caster, from, size) = {
            let npc = &engine.get_npc(zooknock).unwrap().npc;
            (npc.uid, npc.pathing.coord, i32::from(npc.pathing.size))
        };
        let (target, to) = {
            let npc = &engine.get_npc(demon).unwrap().npc;
            (npc.uid, npc.pathing.coord)
        };

        // Sixteen casts, for both spells: he picks one of the two each time.
        const CASTS: usize = 16;
        forget_calls();
        let errors = script_errors(engine, |engine| {
            for _ in 0..CASTS {
                engine
                    .run_script_by_name(
                        "[ai_apnpc2,mm_zooknock_final_battle]",
                        Some(ScriptSubject::Npc(caster)),
                        Some(ScriptSubject::Npc(target)),
                        None,
                        None,
                        None,
                    )
                    .expect("the cast ran");
            }
        });
        let impacts = calls_for("spotanim_npc");
        let anims = calls_for("npc_anim");
        set_recording(false);

        let rows = ["magic_spell_fire_blast", "magic_spell_water_blast"];
        let travels: Vec<i32> = rows
            .iter()
            .filter_map(|row| row_ref(engine, row, SPOTANIM_PROJ))
            .collect();
        let row_impacts: Vec<String> = rows
            .iter()
            .filter_map(|row| row_ref(engine, row, SPOTANIM_TARGET))
            .map(|id| id.to_string())
            .collect();

        // `spotanim_npc` is recorded as "id,height,delay".
        let unsourced: Vec<&String> = impacts
            .iter()
            .filter(|shown| {
                !row_impacts
                    .iter()
                    .any(|id| shown.split(',').next() == Some(id))
            })
            .collect();
        assert!(
            unsourced.is_empty(),
            "an impact the spell table does not give (it gives {row_impacts:?}): {unsourced:?}"
        );

        let own: Vec<&String> = errors
            .iter()
            .filter(|e| !e.contains("projanim_npc error: Runtime error: Invalid uid"))
            .collect();
        assert!(own.is_empty(), "the cast fails: {own:?}");
        if travels.is_empty() {
            assert!(
                errors.is_empty(),
                "the table gives no travel, so none is launched and none fails: {errors:?}"
            );
        }

        // The flight `~npc_to_npc_projectile` gives the cast (25 + 21 + 5 a tile), projectile or
        // not: the demon's block (`demon_block`, seq 65) waits for it. `npc_anim` is recorded
        // as "seq,delay".
        let range = CoordGrid::distance_to(
            i32::from(from.x()),
            i32::from(from.z()),
            size,
            size,
            i32::from(to.x()),
            i32::from(to.z()),
            1,
            1,
        );
        let block = format!("65,{}", (25 + 21 + range * 5) / 30);
        let count = |anim: &str| anims.iter().filter(|a| *a == anim).count();
        assert_eq!(count("1415,0"), CASTS, "his cast's animation: {anims:?}");
        assert_eq!(count(&block), CASTS, "the demon's block {block}: {anims:?}");
    });
}

/// Zooknock's blast at the tunnel's wall (`[ai_timer,mm_zooknock]`) takes the same rows: while
/// they are null it launches nothing, shows no impact and fails nothing, and he still plays his
/// cast. Green before this change too (the raw ids it had are in cache 786): it holds the null
/// rows' path.
#[test]
fn zooknock_s_blast_in_the_tunnel_runs_on_the_spell_table_s_rows() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let (nid, ..) = spawned(engine, "mm_zooknock", None).expect("Zooknock is spawned");
        let zooknock = engine.get_npc(nid).unwrap().npc.uid;
        const CASTS: usize = 16;
        set_recording(true);
        forget_calls();
        let errors = script_errors(engine, |engine| {
            for _ in 0..CASTS {
                engine
                    .run_script_by_name(
                        "[ai_timer,mm_zooknock]",
                        Some(ScriptSubject::Npc(zooknock)),
                        None,
                        None,
                        None,
                        None,
                    )
                    .expect("the cast ran");
            }
        });
        let anims = calls_for("npc_anim");
        set_recording(false);
        assert!(errors.is_empty(), "the cast fails: {errors:?}");
        let casts = anims.iter().filter(|a| *a == "1415,0").count();
        assert_eq!(casts, CASTS, "his cast's animation: {anims:?}");
    });
}

/// Keeps the player in the world over a long watch: the client's keepalive, and the player counted
/// active (the engine logs out a player who sends nothing for 100 ticks, or does nothing for 500).
fn stay_on(engine: &mut Engine, client: &mut HarnessClient) {
    client.send(ClientProt::NoTimeout, &[]);
    let clock = engine.clock;
    engine.harness_player_mut(client.pid).unwrap().last_active = clock;
}

/// The 10th squad's sigil takes the player to the final battle, where the jungle demon spawns and
/// attacks: it casts from 3 tiles or more, and casts or swings from nearer. Defence and Magic are
/// 1, so its spells land, and life is put back each tick. Gives each hit painted on the player over the fight's first `ticks`
/// (`damage`'s hitmark and amount), once no script has failed. The calls stay recorded for the
/// case to read (`calls_for`).
fn fight_the_jungle_demon(engine: &mut Engine, name: &str, ticks: usize) -> Vec<(i32, i32)> {
    let (mut client, t) = start_the_final_battle(engine, name);
    let mut messages = t.messages;
    for _ in 0..ticks {
        stay_on(engine, &mut client);
        set_life_points(engine, &client, 990);
        engine.cycle();
        messages.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, m)| m),
        );
    }
    let hits = calls_for("damage")
        .iter()
        .map(|hit| {
            let mut n = hit.split(',').map(|n| n.parse::<i32>().unwrap());
            (n.next().unwrap(), n.next().unwrap())
        })
        .collect();
    set_recording(false);
    assert!(
        spawned(engine, "mm_demon", None).is_some(),
        "the jungle demon is there"
    );
    // A script that fails tells its player so (the harness keeps no error log).
    let errors: Vec<&String> = messages
        .iter()
        .filter(|m| m.starts_with("script error"))
        .collect();
    assert!(errors.is_empty(), "the demon's scripts run: {errors:?}");
    no_errors(engine);
    hits
}

/// `^hitmark_magic`, and with `^hitmark_crit` (a hit above nine tenths of the most).
const MAGIC_HITS: [i32; 2] = [2, 12];

/// The spells' hits among a fight's: their amounts.
fn spell_hits(hits: &[(i32, i32)]) -> Vec<i32> {
    hits.iter()
        .filter(|(hitmark, _)| MAGIC_HITS.contains(hitmark))
        .map(|&(_, amount)| amount)
        .collect()
}

/// The jungle demon's wave spells cast and hit while 647's spell table has no graphics for them.
/// Its magic_spell_table rows for the wave spells name spotanims 2011 does not have, so each reads
/// as null; the cast failed on its projectile ("projanim_pl error: Spotanim not found: -1", every
/// cast of a fight in the client) and nothing hit. A null travel or impact is now left unshown
/// and the spell still rolls and hits. The graphics wait on the table's verified 2011 ids, which
/// the demon reads as 289 does; this case does not show a resolved row's being launched (the
/// harness has no row to give it).
#[test]
fn the_jungle_demon_s_spells_cast_and_hit() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let hits = fight_the_jungle_demon(engine, "mmdemon", 80);
        assert!(!spell_hits(&hits).is_empty(), "a spell lands: {hits:?}");
    });
}

/// `magic_spell_table`'s sound_success and sound_fail columns (counted as above).
const SOUND_SUCCESS: usize = 21;
const SOUND_FAIL: usize = 22;

/// The sounds the rows give for a spell's landing and its splash: the resolved ones.
fn row_sounds(engine: &Engine, rows: &[&str]) -> Vec<i32> {
    rows.iter()
        .flat_map(|row| [SOUND_SUCCESS, SOUND_FAIL].map(|c| row_ref(engine, row, c)))
        .flatten()
        .collect()
}

/// The jungle demon casts the four blast spells, where 289's casts the four waves: the 2011
/// wiki's Jungle demon page (revid 245860, 2011-05-22), "The Jungle Demon appears to alternate
/// between all four [[Blast spells]] for its magic attacks". Each cast reads its spell's row, as
/// it did for the waves: the row's sound is played on a landing or a splash, as
/// `~npc_spell_success` and `~npc_spell_fail` play it, and a row's null sound plays nothing. So
/// over a fight no wave row's sound is heard, and fire blast's (the one blast sound 647's pack
/// resolves) is.
#[test]
fn the_jungle_demon_casts_the_four_blast_spells() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let hits = fight_the_jungle_demon(engine, "mmblasts", 400);
        let heard: Vec<i32> = calls_for("sound_synth")
            .iter()
            .map(|s| s.split(',').next().unwrap().parse().unwrap())
            .collect();
        assert!(!spell_hits(&hits).is_empty(), "a spell lands: {hits:?}");
        let waves = row_sounds(
            engine,
            &[
                "magic_spell_wind_wave",
                "magic_spell_water_wave",
                "magic_spell_earth_wave",
                "magic_spell_fire_wave",
            ],
        );
        assert_eq!(waves.len(), 8, "the wave rows' sounds resolve: {waves:?}");
        let waved: Vec<&i32> = heard.iter().filter(|s| waves.contains(s)).collect();
        assert!(waved.is_empty(), "a wave's sound is played: {waved:?}");
        let fire_blast = row_ref(engine, "magic_spell_fire_blast", SOUND_SUCCESS)
            .expect("fire blast's row has its sound");
        assert!(
            heard.contains(&fire_blast),
            "fire blast's sound {fire_blast} is played: {heard:?}"
        );
    });
}

/// The most a blast takes.
const DEMON_MAX_HIT: i32 = 320;

/// The demon's spells hit in life points, with 2011's magic hitmark, up to 320: the 2011 wiki's
/// Jungle demon page (revid 245860, 2011-05-22) has "max hit = 320 (approx)" and twice "can hit
/// up to 320" (289's most, 31 hitpoints, would be 310). Over a long fight one hit passes 310,
/// and none passes 320; each crit is a hit above nine tenths of 320 (`~combat_hit`).
#[test]
fn the_jungle_demon_s_spells_hit_up_to_320() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let hits = fight_the_jungle_demon(engine, "mmdemonhit", 1500);
        let spells: Vec<(i32, i32)> = hits
            .iter()
            .copied()
            .filter(|(hitmark, _)| MAGIC_HITS.contains(hitmark))
            .collect();
        let hardest = spells.iter().map(|&(_, d)| d).max().unwrap_or(0);
        assert!(
            (311..=DEMON_MAX_HIT).contains(&hardest),
            "a spell takes up to {DEMON_MAX_HIT}, the hardest {hardest} of {}",
            spells.len()
        );
        let miscrit: Vec<&(i32, i32)> = spells
            .iter()
            .filter(|&&(hitmark, d)| {
                d > 10 && (hitmark == MAGIC_HITS[1]) != (d * 10 > DEMON_MAX_HIT * 9)
            })
            .collect();
        assert!(
            miscrit.is_empty(),
            "a crit is a hit above nine tenths of {DEMON_MAX_HIT}: {miscrit:?}"
        );
    });
}

/// 2011's jungle demon life points: its hitpoints key in cache 786's npc section (all.npc), and
/// the 2011 wiki's "LP = 1700".
const DEMON_LIFE_POINTS: u16 = 1700;

/// The 10th squad cannot kill the demon: a hit that would heals it to 10% of its life points
/// instead, as the 2011 wiki's Jungle demon page (revid 245860, 2011-05-22) has it: "You cannot
/// let the 10th squad do all the work, or the demon will heal to 10% of his full health" (289
/// heals 15%). The demon is left 1 life point, and the squad's next hit that lands sends it to
/// 170, less any other hit of the same tick.
#[test]
fn the_10th_squad_s_killing_hit_heals_the_demon_to_a_tenth() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, _) = start_the_final_battle(engine, "mmdemonheal");
        let mut demon = None;
        for _ in 0..80 {
            set_life_points(engine, &client, 990);
            engine.cycle();
            client.packets();
            if let Some((nid, ..)) = spawned(engine, "mm_demon", None) {
                demon = Some(nid);
                break;
            }
        }
        let demon = demon.expect("the jungle demon is there");
        let life = |engine: &Engine| {
            engine.get_npc(demon).unwrap().npc.stats.levels[NpcStat::Hitpoints as usize]
        };
        assert_eq!(
            engine.get_npc(demon).unwrap().npc.stats.base_levels[NpcStat::Hitpoints as usize],
            DEMON_LIFE_POINTS
        );
        engine.get_npc_mut(demon).unwrap().npc.stats.levels[NpcStat::Hitpoints as usize] = 1;
        let mut after = 1;
        for _ in 0..300 {
            stay_on(engine, &mut client);
            set_life_points(engine, &client, 990);
            engine.cycle();
            client.packets();
            after = life(engine);
            if after != 1 {
                break;
            }
        }
        set_recording(false);
        let tenth = DEMON_LIFE_POINTS / 10;
        // Nine of the squad, each hitting at most 10 in a tick.
        assert!(
            (tenth - 90..=tenth).contains(&after),
            "healed to {tenth}, less the tick's other hits: {after}"
        );
    });
}

/// `^mm_reported_narnode`: the king has heard the report and owes the reward.
const MM_REPORTED_NARNODE: i32 = 7;
/// `^mm_complete`: rewarded, Daero's training still to do.
const MM_COMPLETE: i32 = 9;
/// Cache 786's quest complete scroll.
const QUEST_SCROLL: u16 = 277;

/// King Narnode hands over the reward, the scroll opens, and once it is closed he sends the
/// player to Daero's training (289's mm_complete_dialogue, queued behind the scroll).
#[test]
fn king_narnode_speaks_of_the_training_after_the_reward() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mmreward");
        set_var(engine, &client, "mm_main", MM_REPORTED_NARNODE);
        let (nid, nx, nz, _) = spawned(engine, "grandtree_narnode", Some(KING)).expect("the king");
        tele(engine, &mut client, (nx, nz), true);
        talk_to(&mut client, nid);
        let t = converse(engine, &mut client, &[], 400);
        says(
            &t,
            &["King Narnode hands you a huge stack  of gold coins and several diamonds! "],
        );
        assert_eq!(var(engine, &client, "mm_main"), MM_COMPLETE);
        assert_eq!(held(engine, &client, "diamond"), 3);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(QUEST_SCROLL),
            "the reward scroll is open"
        );
        assert!(
            !t.said("So you're officially a member of the 10th squad then?"),
            "nothing is said under the scroll"
        );

        // The scroll's X.
        client.send(ClientProt::CloseModal, &[]);
        let t = converse(engine, &mut client, &[], 60);
        says(
            &t,
            &[
                "So you're officially a member of the 10th squad then?",
                "The High Tree Guardian Daero is in charge of the training programme. You should know where to find him by now.",
            ],
        );
        no_errors(engine);
    });
}

/// Daero's spawn up the Grand Tree (1_38_54_52_30), and `^narnode_received_orders` in bits 0-3 of
/// `%mm_gnomes` (289's `mm_narnode` varbit): before the orders Daero has no time to talk.
const DAERO: (u16, u16) = (2484, 3486);
const NARNODE_RECEIVED_ORDERS: i32 = 7;

/// Daero's training names 2011's skill: Constitution (cache 786's stat names, enum 680), where
/// 289 has Hitpoints.
#[test]
fn daero_s_training_gives_constitution_xp() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        for (choice, lines) in [
            (
                "Focus on increasing strength and stamina...",
                [
                    "You gain 35,000 Strength XP.",
                    "You gain 35,000 Constitution XP.",
                    "You gain 20,000 Attack XP.",
                    "You gain 20,000 Defence XP.",
                ],
            ),
            (
                "Focus on improving attack and defence techniques...",
                [
                    "You gain 20,000 Strength XP.",
                    "You gain 20,000 Constitution XP.",
                    "You gain 35,000 Attack XP.",
                    "You gain 35,000 Defence XP.",
                ],
            ),
        ] {
            let mut client = login(engine, "mmtraining");
            set_var(engine, &client, "mm_main", MM_COMPLETE);
            set_var(engine, &client, "mm_gnomes", NARNODE_RECEIVED_ORDERS);
            let nid = stand_beside(engine, &mut client, "daero", DAERO);
            talk_to(&mut client, nid);
            let mut t = converse(engine, &mut client, &[choice], 400);
            t.messages.extend(watch(engine, &mut client, 40).messages);
            let gains: Vec<&String> = t
                .messages
                .iter()
                .filter(|m| m.starts_with("You gain"))
                .collect();
            assert_eq!(
                gains,
                lines.iter().collect::<Vec<_>>(),
                "after {choice:?}: {:?}",
                t.messages
            );
            no_errors(engine);
        }
    });
}

/// Cache 786's quest journal, interface 275.
const JOURNAL: u32 = 275;

/// A finished journal line is struck from its first word to its last: every line of the journal
/// opens with its own tag (a colour, or the strike), and no strike starts part way along a line.
/// 289 marks each line of a struck paragraph where its own journal wrapped (q8_full, 400 wide);
/// 2011's wraps elsewhere (p12_full, 404 wide), which left the wrapped ends of lines unstruck.
#[test]
fn the_journal_strikes_whole_lines() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mmjournal");
        // The seal's stage (`^narnode_given_seal`), as the client showed it; then each stage of
        // the quest with every gnome's and monkey's own stage (289's varbits, by their bits on
        // `%mm_gnomes`, `%mm_monkeys` and `%mm_misc`) at each value from its start to its end.
        let each = |fields: &[(u32, u32)], stage: i32| {
            fields.iter().fold(0, |var, &(low, high)| {
                var | stage.min((1 << (high - low + 1)) - 1) << low
            })
        };
        let gnomes = [
            (0, 3),
            (4, 6),
            (7, 10),
            (11, 12),
            (13, 15),
            (16, 19),
            (20, 23),
            (24, 25),
            (26, 27),
        ];
        let monkeys = [(0, 1), (2, 4), (5, 8), (9, 10)];
        let misc = [(0, 1), (2, 9), (12, 16)];
        let mut stages: Vec<(i32, i32, i32, i32, i32)> = vec![(MM_STARTED, 3, 0, 0, 0)];
        for main in MM_STARTED..=10 {
            // 289's stages run to 10 (quest_mm.constant); 15 is every field at its last.
            for stage in (0..=10).chain([15]) {
                stages.push((
                    main,
                    each(&gnomes, stage),
                    each(&monkeys, stage),
                    if stage == 0 { 0 } else { 0x3FFF },
                    each(&misc, stage),
                ));
            }
        }
        for (main, gnomes, monkeys, flags, misc) in stages {
            set_var(engine, &client, "mm_main", main);
            set_var(engine, &client, "mm_gnomes", gnomes);
            set_var(engine, &client, "mm_monkeys", monkeys);
            set_var(engine, &client, "mm_flags", flags);
            set_var(engine, &client, "mm_misc", misc);
            let mut p = ClientPacket::new();
            p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(ROW);
            client.send(ClientProt::IfButton1, &p.finish());
            let mut lines = Vec::new();
            for _ in 0..3 {
                engine.cycle();
                lines.extend(
                    settexts(&client.packets())
                        .into_iter()
                        .filter(|(com, text)| com >> 16 == JOURNAL && !text.is_empty())
                        .map(|(_, text)| text),
                );
            }
            assert!(lines.len() >= 2, "the journal at stage {main}: {lines:?}");
            for line in &lines {
                assert!(
                    line.starts_with('<') && !line[1..].contains("<str>"),
                    "stage {main} ({gnomes:#x}): {line:?} is struck in part"
                );
            }
            client.send(ClientProt::CloseModal, &[]);
            settle(engine, &mut client, 2);
        }
        no_errors(engine);
    });
}

/// Tutab's spawn in cache 786 (n43_43: 0 5 18).
const TUTAB: (u16, u16) = (2757, 2770);

/// Tutab's Trade opens his shop titled "Tutab's Magical Market", the 2011 wiki's name for it (its
/// page, revid 266223, 2010-12-31; and Tutab's, revid 266222, 2011-04-17: "sells magical items
/// ([[Tutab's Magical Market]])"). 289's npc carried "Ifaba's General Store", the general store's
/// title.
#[test]
fn tutab_s_trade_opens_tutab_s_magical_market() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mmtutab");
        // Worn, the monkey greegree makes the player a monkey to Ape Atoll's shopkeepers.
        let greegree = id(engine, "obj", "mm_monkey_greegree_for_small_ninja_monkey");
        let worn = id(engine, "inv", "worn");
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&worn)
            .unwrap()
            .set(3, greegree, 1);
        let tutab = stand_beside(engine, &mut client, "mm_tutab", TUTAB);
        client.op_npc(3, tutab);
        let t = watch(engine, &mut client, 6);
        assert!(
            engine.harness_player(client.pid).unwrap().is_interface_open(620),
            "the shop opens: {:?}",
            t.messages
        );
        assert!(
            t.texts.iter().any(|text| text == "Tutab's Magical Market"),
            "the shop's title: {:?}",
            t.texts
        );
        no_errors(engine);
    });
}
