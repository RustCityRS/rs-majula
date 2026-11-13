//! 289's random events at 2011: the Mysterious Old Man's Maze and Mime (the
//! only two of 289's that 2011 kept), and the Random event gift that 2011
//! gives for solving one (2011 wiki, Random events).
//!
//! The events are summoned with the dev cheat `~macro_event <event>`
//! (`_test/scripts/cheats/cheat_macro_events.rs2`), as the event timer's roll
//! would bring them. Inventory op 1 is `IF_BUTTON1` on `inventory:inv`:
//! `p4(com) p2_alt3(obj) p2_alt1(slot)`.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::player::StaffModLevel;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server::npc_info::add_direction;
use rs_test::runner;
use rs_vm::ops::test::forget_calls;

/// `inventory:inv`, the backpack component `[login]` binds `inv` to.
const BACKPACK: i32 = 149 << 16;
/// `ame_rewards:com_15`, the layer the gift's choices are drawn on.
const GIFT_CHOICES: i32 = (202 << 16) | 15;
/// Lumbridge, east of the castle.
const HOME: (u16, u16) = (3222, 3218);

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

fn varp(engine: &Engine, pid: u16, name: &str) -> i32 {
    let id = *engine
        .cache
        .varps
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no varp {name}"));
    engine.harness_player(pid).unwrap().vars.get(id).as_int()
}

fn coord(engine: &Engine, pid: u16) -> (u16, u8, u16) {
    let c = engine.harness_player(pid).unwrap().pathing.coord;
    (c.x(), c.y(), c.z())
}

/// `(obj, count)` in each slot of the player's `inv`.
fn inv(engine: &Engine, pid: u16, name: &str) -> Vec<Option<(u16, u32)>> {
    let id = *engine.cache.invs.debugnames.get(name).unwrap();
    let inv = engine
        .harness_player(pid)
        .unwrap()
        .invs
        .get(&id)
        .expect(name);
    (0..inv.capacity)
        .map(|slot| inv.get(slot as u16).map(|i| (i.obj, i.num)))
        .collect()
}

fn developer(engine: &mut Engine, name: &str) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .staff_mod_level = StaffModLevel::Maintainer;
    engine.cycle();
    engine.harness_telejump(client.pid, HOME.0, 0, HOME.1);
    // Past 289's "during combat" window (`%lastcombat` + 8 ticks), which a
    // fresh world's clock is still inside.
    for _ in 0..12 {
        engine.cycle();
    }
    client.packets();
    forget_calls();
    client
}

fn cheat(client: &mut HarnessClient, input: &str) {
    let mut p = ClientPacket::new();
    p.p1(0).p1(0).pjstr(input);
    client.send(ClientProt::ClientCheat, &p.finish());
}

fn button1(client: &mut HarnessClient, com: i32, slot: i32, obj: i32) {
    let mut p = ClientPacket::new();
    p.p4(com).p2_alt3(obj).p2_alt1(slot);
    client.send(ClientProt::IfButton1, &p.finish());
}

/// Cycles `n` ticks; the game messages said meanwhile.
fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..n {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    said
}

fn mapsquare(engine: &Engine, pid: u16) -> (u16, u16) {
    let (x, _, z) = coord(engine, pid);
    (x / 64, z / 64)
}

#[test]
fn the_maze_takes_a_player_in_and_their_run_never_tires() {
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "mazerunner");
        cheat(&mut client, "~macro_event 8");
        let said = ticks(engine, &mut client, 6);
        assert_eq!(
            mapsquare(engine, client.pid),
            (45, 71),
            "in the maze; said {said:?}"
        );
        assert!(
            varp(engine, client.pid, "macro_transmit") >= 99,
            "the reward starts full"
        );
        // The old man's word opens a dialogue; the maze's [mapzone] queues
        // behind it until the player dismisses it.
        engine.harness_close_modal(client.pid);
        ticks(engine, &mut client, 2);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(209),
            "mazetimer shows the reward left"
        );

        // "infinite run energy is given to players who are sent to the maze"
        engine.harness_player_mut(client.pid).unwrap().runenergy = 0;
        ticks(engine, &mut client, 2);
        assert_eq!(engine.harness_player(client.pid).unwrap().runenergy, 10000);

        // The shrine at the centre: 289's reward, and back home.
        let before = inv(engine, client.pid, "inv").iter().flatten().count();
        let mut p = ClientPacket::new();
        let shrine = 3634;
        let (sx, sz) = (45 * 64 + 31, 71 * 64 + 31);
        engine.harness_telejump(client.pid, sx + 1, 0, sz);
        ticks(engine, &mut client, 2);
        p.p2_alt1(sz as i32)
            .p2_alt1(shrine)
            .p1(0)
            .p2_alt2(sx as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        ticks(engine, &mut client, 12);
        let (x, _, z) = coord(engine, client.pid);
        assert!(
            x.abs_diff(HOME.0) <= 2 && z.abs_diff(HOME.1) <= 2,
            "back where they were: at {x},{z}"
        );
        let after = inv(engine, client.pid, "inv").iter().flatten().count();
        assert!(after > before, "the maze's reward");
        engine.harness_player_mut(client.pid).unwrap().runenergy = 0;
        ticks(engine, &mut client, 2);
        assert!(
            engine.harness_player(client.pid).unwrap().runenergy < 10000,
            "out of the maze, run tires again"
        );
    });
}

/// 2011's macro_mime_emotes (188) buttons by the mime's emote (289's
/// %npc_int: cry, think, laugh, dance, climb rope, lean, glass wall, glass box).
const MIME_BUTTONS: [i32; 8] = [3, 2, 4, 5, 6, 7, 9, 8];

#[test]
fn the_mime_takes_a_player_to_his_stage_and_four_right_answers_bring_a_gift() {
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "mimic");
        cheat(&mut client, "~macro_event 7");
        let said = ticks(engine, &mut client, 6);
        assert_eq!(
            mapsquare(engine, client.pid),
            (31, 74),
            "on the stage; said {said:?}"
        );

        let npc_int = *engine.cache.varns.debugnames.get("npc_int").unwrap();
        let mut answered = 0;
        for _ in 0..400 {
            ticks(engine, &mut client, 1);
            if mapsquare(engine, client.pid) != (31, 74) {
                break;
            }
            if !engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(188)
            {
                continue;
            }
            let emote = engine
                .npc_list
                .npcs
                .iter()
                .flatten()
                .find(|n| n.npc.base_type == 1056)
                .map(|n| n.npc.vars.get(npc_int).as_int())
                .expect("the mime");
            button1(
                &mut client,
                (188 << 16) | MIME_BUTTONS[emote as usize],
                -1,
                -1,
            );
            answered += 1;
            ticks(engine, &mut client, 2);
        }
        assert_eq!(answered, 4, "four right answers in a row");
        let (x, _, z) = coord(engine, client.pid);
        assert!(
            x.abs_diff(HOME.0) <= 2 && z.abs_diff(HOME.1) <= 2,
            "back where they were: at {x},{z}"
        );
        let gift = obj(engine, "macro_frog_token");
        assert!(
            inv(engine, client.pid, "inv")
                .iter()
                .flatten()
                .any(|&(o, _)| o == gift),
            "a Random event gift"
        );
        assert_eq!(varp(engine, client.pid, "random_event_gift_event"), 7);
    });
}

fn open_gift(engine: &mut Engine, client: &mut HarnessClient) -> Vec<String> {
    cheat(client, "~macro_event_gift 7");
    ticks(engine, client, 2);
    let gift = obj(engine, "macro_frog_token");
    let slot = inv(engine, client.pid, "inv")
        .iter()
        .position(|s| matches!(s, Some((o, _)) if *o == gift))
        .expect("the gift");
    button1(client, BACKPACK, slot as i32, gift as i32);
    ticks(engine, client, 2)
}

#[test]
fn a_gift_offers_2011_s_choices_and_gives_the_one_chosen() {
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "giftee");
        let said = open_gift(engine, &mut client);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(202),
            "ame_rewards opens; said {said:?}"
        );
        let choices = inv(engine, client.pid, "macro_certer");
        assert!(
            choices[26].is_some() && choices[27].is_some(),
            "the emote and the costume point"
        );
        let rewards = choices[..26].iter().flatten().count();
        assert!((2..=11).contains(&rewards), "{rewards} rewards");

        // The first reward offered: its choice is its slot (the category).
        let slot = choices[..26].iter().position(Option::is_some).unwrap();
        let (reward, count) = choices[slot].unwrap();
        button1(&mut client, GIFT_CHOICES, slot as i32 * 7, -1);
        let said = ticks(engine, &mut client, 2);
        assert!(
            said.iter().any(|m| m == "Enjoy your gift."),
            "said {said:?}"
        );
        let gift = obj(engine, "macro_frog_token");
        let held = inv(engine, client.pid, "inv");
        assert!(
            !held.iter().flatten().any(|&(o, _)| o == gift),
            "the gift is used up"
        );
        let got = held
            .iter()
            .flatten()
            .find(|&&(o, _)| {
                o == reward
                    || engine.cache.objs.get_by_id(o).and_then(|t| t.certlink) == Some(reward)
            })
            .copied();
        assert_eq!(
            got.map(|(_, c)| c),
            Some(count),
            "the reward chosen, {count} of it"
        );
        assert!(
            !engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(202)
        );
    });
}

#[test]
fn a_gift_s_emote_unlocks_one_of_the_mime_s_emotes() {
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "emoter");
        let before = varp(engine, client.pid, "emote_access") & 0xF;
        open_gift(engine, &mut client);
        let choices = inv(engine, client.pid, "macro_certer");
        assert!(choices[26].is_some(), "the emote is offered");
        button1(&mut client, GIFT_CHOICES, 26 * 7, -1);
        ticks(engine, &mut client, 2);
        let after = varp(engine, client.pid, "emote_access") & 0xF;
        assert_eq!(
            (after & !before).count_ones(),
            1,
            "one more of bits 0-3: {before:04b} -> {after:04b}"
        );
    });
}

/// 2011's audience faces the stage (user-reported: "sitting backwards"). The
/// watchers cannot turn, so the client keeps the facing of NPC_INFO's add
/// block, which the engine sent as 0 for every npc: angle 8192, their backs to
/// the Mime. Seen in-client, angle 0 (direction 4, the client's own default)
/// faces them at the stage.
#[test]
fn the_mime_s_audience_faces_the_stage() {
    runner::on_loaded_engine(|engine| {
        // A player among the seats: the zone's npcs run their AI.
        engine.harness_reset();
        let client = engine
            .harness_login_returning("theatregoer")
            .expect("login");
        engine.harness_telejump(client.pid, 31 * 64 + 29, 0, 74 * 64 + 21);
        for _ in 0..20 {
            engine.cycle();
        }
        let audience: Vec<_> = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .filter(|n| (1057..=1059).contains(&n.npc.base_type))
            .map(|n| {
                // As NPC_INFO's add block reports it (info.rs).
                let c = n.npc.pathing.coord;
                let facing = n.npc.facing().map(|(fx, fz)| {
                    (
                        (c.x() * 2 + 1) as i32 - fx as i32,
                        (c.z() * 2 + 1) as i32 - fz as i32,
                    )
                });
                let dir = add_direction(facing);
                (n.npc.base_type, dir)
            })
            .collect();
        assert_eq!(audience.len(), 3, "three strange watchers: {audience:?}");
        assert!(
            audience.iter().all(|&(_, dir)| dir == 4),
            "each comes into view facing the stage: {audience:?}"
        );
    });
}
