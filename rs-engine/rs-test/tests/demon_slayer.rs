//! Demon Slayer at 647 (`quests/quest_demon`, `areas/area_varrock`,
//! `areas/area_draynor`): 289's quest as 2011's rework. Gypsy Aris starts it
//! for a coin and deals the player's own incantation; Captain Rovin, Traiborn
//! (25 bones) and the palace drain give the three keys; Sir Prysin opens
//! Silverlight's case; Delrith, summoned in the stone circle, is banished with
//! the incantation.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_pack::types::NpcStat;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{converse, explore, talk_to};
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::sync::Mutex;

/// Every engine in this binary shares rs-pathfinder's process-static
/// collision map: the cases take turns.
static TURN: Mutex<()> = Mutex::new(());

/// `inventory:inv`.
const BACKPACK: i32 = 149 << 16;
/// The palace kitchen's drain (loc_31759) and the key's spot in the sewer
/// below (loc_17431), from 647's maps.
const DRAIN: (u16, u16, u16) = (31759, 3225, 3496);
const SEWER_KEY: (u16, u16, u16) = (17431, 3225, 9897);
/// Inside the stone circle, west of the stone table.
const CIRCLE: (u16, u16) = (3224, 3369);

/// Varp 222 (289's demonstart), as cache 786 splits it.
#[derive(Clone, Copy, Default)]
struct Demon {
    stage: i32,
    /// The places of Aber, Camerinthum, Carlem, Gabindo and Purchai.
    words: [i32; 5],
    silverlight: i32,
    drain: i32,
    summoned: i32,
    case: i32,
}

impl Demon {
    fn stage(stage: i32) -> Self {
        Demon {
            stage,
            ..Default::default()
        }
    }

    fn pack(self) -> i32 {
        let mut v = self.stage;
        for (i, w) in self.words.iter().enumerate() {
            v |= w << (5 + 3 * i);
        }
        v | self.silverlight << 20 | self.drain << 21 | self.summoned << 23 | self.case << 27
    }

    fn unpack(v: i32) -> Self {
        let mut words = [0; 5];
        for (i, w) in words.iter_mut().enumerate() {
            *w = (v >> (5 + 3 * i)) & 7;
        }
        Demon {
            stage: v & 0x1F,
            words,
            silverlight: (v >> 20) & 1,
            drain: (v >> 21) & 3,
            summoned: (v >> 23) & 1,
            case: (v >> 27) & 3,
        }
    }
}

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("obj.pack names {name}"))
}

fn varp(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .varps
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("varp.pack names {name}"))
}

fn npc_type(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .npcs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("npc.pack names {name}"))
}

/// A spawn of npc `name`: its nid, and a tile beside it that one step from
/// the npc reaches, so no wall stands between (npcs wander).
fn beside(engine: &Engine, name: &str) -> (u16, u16, u8, u16) {
    let id = npc_type(engine, name);
    let (nid, nx, y, nz) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == id)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.y(), c.z())
        })
        .unwrap_or_else(|| panic!("{name} is spawned"));
    let (sx, sz) = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find(|&(dx, dz)| {
            rsmod::can_travel(y, nx, nz, dx, dz, 1, 0, CollisionType::Normal)
                && !rsmod::is_flagged(
                    nx.wrapping_add_signed(dx as i16),
                    nz.wrapping_add_signed(dz as i16),
                    y,
                    0x100 | 0x200000,
                )
        })
        .map(|(dx, dz)| {
            (
                nx.wrapping_add_signed(dx as i16),
                nz.wrapping_add_signed(dz as i16),
            )
        })
        .unwrap_or_else(|| panic!("a tile beside {name} at {nx},{nz}"));
    (nid, sx, y, sz)
}

fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) {
    for _ in 0..n {
        engine.cycle();
        client.packets();
    }
}

/// Logs in at (x, level, z) with Demon Slayer at `demon`, %demon_slayer_bones
/// at `bones` and `items` in the backpack.
fn login(
    engine: &mut Engine,
    name: &str,
    (x, level, z): (u16, u8, u16),
    demon: Demon,
    bones: i32,
    items: &[(&str, i32)],
) -> HarnessClient {
    let (quest, owed) = (
        varp(engine, "demonstart"),
        varp(engine, "demon_slayer_bones"),
    );
    let backpack = *engine.cache.invs.debugnames.get("inv").unwrap();
    let items: Vec<(u16, i32)> = items.iter().map(|&(n, c)| (obj(engine, n), c)).collect();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    for _ in 0..10 {
        engine.cycle();
    }
    engine.harness_telejump(client.pid, x, level, z);
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.vars.set(quest, VarValue::Int(demon.pack()));
    player.vars.set(owed, VarValue::Int(bones));
    let pack = player.invs.get_mut(&backpack).unwrap();
    for (slot, &(o, n)) in items.iter().enumerate() {
        pack.set(slot as u16, o, n as u32);
    }
    ticks(engine, &mut client, 3);
    client
}

/// Logs in beside `npc` and talks to it.
fn visit(
    engine: &mut Engine,
    npc: &str,
    demon: Demon,
    bones: i32,
    items: &[(&str, i32)],
) -> HarnessClient {
    let (nid, x, y, z) = beside(engine, npc);
    let mut client = login(engine, "demon_visitor", (x, y, z), demon, bones, items);
    talk_to(&mut client, nid);
    client
}

fn demon(engine: &Engine, client: &HarnessClient) -> Demon {
    let quest = varp(engine, "demonstart");
    Demon::unpack(
        engine
            .harness_player(client.pid)
            .unwrap()
            .vars
            .get(quest)
            .as_int(),
    )
}

fn total(engine: &Engine, client: &HarnessClient, inv: &str, name: &str) -> i32 {
    let inv = *engine.cache.invs.debugnames.get(inv).unwrap();
    let o = obj(engine, name);
    engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .map_or(0, |i| i.total(o) as i32)
}

fn no_script_errors(engine: &mut Engine, what: &str) {
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "{what}: {errors:?}");
}

/// An npc, the quest's state, %demon_slayer_bones and the backpack.
type Case = (&'static str, Demon, i32, &'static [(&'static str, i32)]);

/// Each npc at each step of the quest says something down every branch, and
/// fails no script.
#[test]
fn every_demon_slayer_npc_answers_every_branch() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let with_words = |stage| Demon {
            stage,
            words: [1, 2, 3, 4, 5],
            ..Default::default()
        };
        let cases: &[Case] = &[
            ("gypsy_aris_9362", Demon::stage(0), 0, &[("coins", 10)]),
            ("gypsy_aris_9362", Demon::stage(0), 0, &[]),
            ("gypsy_aris_9362", with_words(1), 0, &[]),
            ("gypsy_aris_9362", with_words(3), 0, &[]),
            // 2011's palace Sir Prysin is the multi-npc 9359.
            ("npc_9359", Demon::stage(0), 0, &[]),
            ("npc_9359", Demon::stage(1), 0, &[]),
            ("npc_9359", Demon::stage(2), 0, &[]),
            (
                "npc_9359",
                Demon::stage(2),
                0,
                &[("silverlight_key_captain_rovin", 1)],
            ),
            (
                "npc_9359",
                Demon {
                    silverlight: 1,
                    ..Demon::stage(2)
                },
                0,
                &[("silverlight", 1)],
            ),
            (
                "npc_9359",
                Demon {
                    silverlight: 1,
                    ..Demon::stage(2)
                },
                0,
                &[],
            ),
            ("npc_9359", Demon::stage(3), 0, &[]),
            ("captain_rovin", Demon::stage(0), 0, &[]),
            ("captain_rovin", Demon::stage(2), 0, &[]),
            (
                "captain_rovin",
                Demon::stage(2),
                0,
                &[("silverlight_key_captain_rovin", 1)],
            ),
            ("captain_rovin", Demon::stage(3), 0, &[]),
            ("traiborn", Demon::stage(0), 0, &[]),
            ("traiborn", Demon::stage(2), 0, &[]),
            ("traiborn", Demon::stage(2), 25, &[]),
            ("traiborn", Demon::stage(2), 25, &[("bones", 3)]),
            ("traiborn", Demon::stage(2), -1, &[]),
            (
                "traiborn",
                Demon::stage(2),
                -1,
                &[("silverlight_key_wizard_traiborn", 1)],
            ),
        ];
        for &(npc, at, bones, items) in cases {
            let _ = engine.script_errors.take();
            let branches = explore(engine, |engine| visit(engine, npc, at, bones, items), 40);
            no_script_errors(engine, &format!("{npc} at stage {}", at.stage));
            assert!(!branches.is_empty(), "{npc}");
            for branch in &branches {
                assert!(
                    !branch.conversation.texts.is_empty(),
                    "{npc} at stage {}, path {:?}: nothing said ({:?})",
                    at.stage,
                    branch.path,
                    branch.conversation.messages
                );
            }
        }
    });
}

/// For a coin the gypsy tells the fortune and Wally's story, which deals the
/// player's incantation (the five words, each in its own place) and starts the
/// quest; she then recites the same incantation.
#[test]
fn gypsy_aris_deals_the_incantation_and_starts_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let _ = engine.script_errors.take();
        let mut client = visit(
            engine,
            "gypsy_aris_9362",
            Demon::stage(0),
            0,
            &[("coins", 10)],
        );
        let talk = converse(
            engine,
            &mut client,
            &[
                "Okay, here you go.",
                "How am I meant to fight",
                "So, how did Wally kill Delrith?",
                "What is the magical incantation?",
                "Ok thanks.",
            ],
            600,
        );
        assert_eq!(talk.unanswered, None, "{:?}", talk.texts);
        no_script_errors(engine, "the gypsy");
        let at = demon(engine, &client);
        assert_eq!(at.stage, 1, "the quest is started");
        assert_eq!(
            total(engine, &client, "inv", "coins"),
            9,
            "one coin for the fortune"
        );
        let mut places = at.words;
        places.sort();
        assert_eq!(places, [1, 2, 3, 4, 5], "each word has its own place");
        let names = ["Aber", "Camerinthum", "Carlem", "Gabindo", "Purchai"];
        let mut order = [""; 5];
        for (i, &p) in at.words.iter().enumerate() {
            order[p as usize - 1] = names[i];
        }
        let recited = format!(
            "it goes... {}... {}... {}... {}... {}.",
            order[0], order[1], order[2], order[3], order[4]
        );
        assert!(talk.said(&recited), "{recited} in {:?}", talk.texts);
    });
}

/// Rovin gives his key once told Sir Prysin's key went down a drain.
#[test]
fn captain_rovin_gives_his_key() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let _ = engine.script_errors.take();
        let mut client = visit(engine, "captain_rovin", Demon::stage(2), 0, &[]);
        let talk = converse(
            engine,
            &mut client,
            &[
                "Yes, I know, but this is important.",
                "There's a demon",
                "Yes, very.",
                "They're not going to fight",
                "Sir Prysin said",
                "Why did he give you",
            ],
            400,
        );
        assert_eq!(talk.unanswered, None, "{:?}", talk.texts);
        no_script_errors(engine, "Rovin");
        assert_eq!(
            total(engine, &client, "inv", "silverlight_key_captain_rovin"),
            1
        );
    });
}

/// Traiborn wants 25 sets of bones, takes what the player has and opens his
/// closet once he has them all.
#[test]
fn traiborn_takes_25_bones_for_his_key() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let owed = varp(engine, "demon_slayer_bones");
        let _ = engine.script_errors.take();
        let mut client = visit(engine, "traiborn", Demon::stage(2), 0, &[]);
        let talk = converse(
            engine,
            &mut client,
            &[
                "I need to get a key",
                "Well, have you got any keys",
                "I'll help get the bones",
            ],
            400,
        );
        assert_eq!(talk.unanswered, None, "{:?}", talk.texts);
        let bones = |engine: &Engine, client: &HarnessClient| {
            engine
                .harness_player(client.pid)
                .unwrap()
                .vars
                .get(owed)
                .as_int()
        };
        assert_eq!(bones(engine, &client), 25, "he wants 25");

        let mut client = visit(engine, "traiborn", Demon::stage(2), 25, &[("bones", 10)]);
        let talk = converse(engine, &mut client, &[], 400);
        assert!(
            talk.said("You give Traiborn 10 sets of bones."),
            "{:?}",
            talk.texts
        );
        assert!(talk.said("I still need 15 more."), "{:?}", talk.texts);
        assert_eq!(bones(engine, &client), 15);
        assert_eq!(total(engine, &client, "inv", "bones"), 0);

        let mut client = visit(engine, "traiborn", Demon::stage(2), 15, &[("bones", 20)]);
        let talk = converse(engine, &mut client, &[], 400);
        no_script_errors(engine, "Traiborn");
        assert!(
            talk.said("Hurrah! That's all 25 sets of bones."),
            "{:?}",
            talk.texts
        );
        assert_eq!(
            total(engine, &client, "inv", "bones"),
            5,
            "he takes only 15"
        );
        assert_eq!(
            total(engine, &client, "inv", "silverlight_key_wizard_traiborn"),
            1
        );
        assert_eq!(bones(engine, &client), -1, "all given");
    });
}

/// Water down the palace drain washes Sir Prysin's key into the sewer, where
/// it is taken; the drain is then empty.
#[test]
fn water_washes_the_drain_key_into_the_sewer() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let _ = engine.script_errors.take();
        let bucket = obj(engine, "bucket_of_water");
        let mut client = login(
            engine,
            "drain_washer",
            (DRAIN.1 + 1, 0, DRAIN.2),
            Demon::stage(2),
            0,
            &[("bucket_of_water", 1)],
        );
        let mut p = ClientPacket::new();
        p.p2_alt2(bucket as i32)
            .p2_alt2(0)
            .p2(DRAIN.2 as i32)
            .p4_alt2(BACKPACK)
            .p2_alt2(DRAIN.1 as i32)
            .p1_alt1(0)
            .p2_alt1(DRAIN.0 as i32);
        client.send(ClientProt::OpLocT, &p.finish());
        let talk = converse(engine, &mut client, &[], 60);
        assert!(
            talk.said("washed the key down into the sewer"),
            "{:?}",
            talk.texts
        );
        assert_eq!(demon(engine, &client).drain, 1, "the key is in the sewer");
        assert_eq!(
            total(engine, &client, "inv", "bucket"),
            1,
            "the bucket is emptied"
        );

        engine.harness_telejump(client.pid, SEWER_KEY.1 + 1, 0, SEWER_KEY.2);
        ticks(engine, &mut client, 3);
        let mut p = ClientPacket::new();
        p.p2_alt1(SEWER_KEY.2 as i32)
            .p2_alt1(SEWER_KEY.0 as i32)
            .p1(0)
            .p2_alt2(SEWER_KEY.1 as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        let talk = converse(engine, &mut client, &[], 60);
        no_script_errors(engine, "the drain");
        assert!(
            talk.said("You pick up an old, rusty key."),
            "{:?}",
            talk.texts
        );
        assert_eq!(
            total(engine, &client, "inv", "silverlight_key_sir_prysin"),
            1
        );
        assert_eq!(demon(engine, &client).drain, 2, "mud where the key lay");
    });
}

/// With the three keys Sir Prysin opens the case and hands Silverlight over.
#[test]
fn sir_prysin_hands_over_silverlight_for_the_three_keys() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let _ = engine.script_errors.take();
        let keys = [
            "silverlight_key_wizard_traiborn",
            "silverlight_key_captain_rovin",
            "silverlight_key_sir_prysin",
        ];
        let mut client = visit(
            engine,
            "npc_9359",
            Demon::stage(2),
            -1,
            &[(keys[0], 1), (keys[1], 1), (keys[2], 1)],
        );
        // The handover runs some 20 ticks between chat boxes: one converse per
        // stretch of talk.
        let mut talk = converse(engine, &mut client, &[], 400);
        talk.texts
            .extend(converse(engine, &mut client, &[], 400).texts);
        no_script_errors(engine, "Sir Prysin");
        assert!(
            talk.said("Sir Prysin hands you a very shiny sword."),
            "{:?}",
            talk.texts
        );
        assert!(
            talk.texts.iter().any(|t| t == "Sir Prysin")
                && !talk.texts.iter().any(|t| t == "npc_9359"),
            "the multi-npc is named as the type it shows: {:?}",
            talk.texts
        );
        assert_eq!(total(engine, &client, "inv", "silverlight"), 1);
        for key in keys {
            assert_eq!(total(engine, &client, "inv", key), 0, "{key} is taken");
        }
        let at = demon(engine, &client);
        assert_eq!(
            (at.silverlight, at.case),
            (1, 2),
            "Silverlight had, its case empty"
        );
    });
}

/// Walking into the stone circle with Silverlight summons Delrith; beaten, he
/// is weakened; a wrong incantation revives him, and the player's own banishes
/// him and completes the quest.
#[test]
fn delrith_is_banished_with_the_player_s_incantation() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let _ = engine.script_errors.take();
        let worn = *engine.cache.invs.debugnames.get("worn").unwrap();
        let silverlight = obj(engine, "silverlight");
        let (delrith, weakened) = (
            npc_type(engine, "delrith"),
            npc_type(engine, "weakened_delrith"),
        );
        // Aber Camerinthum Carlem Gabindo Purchai in places 2 5 1 4 3: Carlem
        // Aber Purchai Gabindo Camerinthum.
        let at = Demon {
            stage: 2,
            words: [2, 5, 1, 4, 3],
            silverlight: 1,
            ..Default::default()
        };
        let mut client = login(
            engine,
            "demon_slayer",
            (CIRCLE.0, 0, CIRCLE.1 + 20),
            at,
            -1,
            &[],
        );
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&worn).unwrap().set(3, silverlight, 1);
        for skill in [0, 1, 2, 3] {
            player.stats.levels[skill] = 99;
            player.stats.base_levels[skill] = 99;
            // Level 99's xp, in tenths.
            player.stats.xp[skill] = 130_344_310;
        }
        let lp = player.vars.get(1240).as_int();
        player.vars.set(1240, VarValue::Int((lp & 1) | 990 << 1));
        // Past 289's "under attack" window.
        ticks(engine, &mut client, 12);

        let find = |engine: &Engine, ty: u16| {
            engine
                .npc_list
                .npcs
                .iter()
                .flatten()
                .find(|n| n.npc.uid.id() == ty)
                .map(|n| n.npc.uid.nid())
        };
        assert_eq!(find(engine, delrith), None, "no Delrith before the circle");
        engine.harness_telejump(client.pid, CIRCLE.0, 0, CIRCLE.1);
        ticks(engine, &mut client, 6);
        let nid = find(engine, delrith).expect("Delrith rises");
        assert_eq!(demon(engine, &client).summoned, 1, "the table is shattered");

        let fight = |engine: &mut Engine, client: &mut HarnessClient, nid: u16, which: &str| {
            let mut p = ClientPacket::new();
            p.p1_alt2(0).p2_alt3(nid as i32);
            client.send(ClientProt::OpNpc2, &p.finish());
            let mut said = Vec::new();
            for tick in 0..300 {
                // The engine drops a client silent for 100 ticks.
                if tick % 20 == 0 {
                    client.send(ClientProt::NoTimeout, &[]);
                }
                engine.cycle();
                said.extend(HarnessClient::game_messages(&client.packets()));
                if find(engine, weakened).is_some() {
                    return;
                }
            }
            let errors = engine.script_errors.take();
            let hp = engine
                .get_npc(nid)
                .map(|n| n.npc.stats.levels[NpcStat::Hitpoints as usize]);
            let at = engine.harness_player(client.pid).unwrap().pathing.coord;
            panic!(
                "{which}: Delrith is not weakened in 300 ticks (life points {hp:?}, player at {},{}): {said:?} {errors:?}",
                at.x(),
                at.z()
            );
        };
        fight(engine, &mut client, nid, "the first fight");
        no_script_errors(engine, "the fight");

        talk_to(&mut client, find(engine, weakened).unwrap());
        let wrong = ["Aber", "Carlem", "Purchai", "Gabindo", "Camerinthum"];
        let talk = converse(engine, &mut client, &wrong, 200);
        assert_eq!(talk.unanswered, None, "{:?}", talk.texts);
        assert!(
            talk.said("That was the wrong incantation."),
            "{:?}",
            talk.texts
        );
        let nid = find(engine, delrith).expect("Delrith revives");
        assert_eq!(demon(engine, &client).stage, 2);

        fight(
            engine,
            &mut client,
            nid,
            "the fight after the wrong incantation",
        );
        talk_to(&mut client, find(engine, weakened).unwrap());
        let right = ["Carlem", "Aber", "Purchai", "Gabindo", "Camerinthum"];
        let talk = converse(engine, &mut client, &right, 300);
        no_script_errors(engine, "the banishing");
        assert!(
            talk.said("Delrith is sucked into the vortex..."),
            "{:?}",
            talk.texts
        );
        let after = demon(engine, &client);
        assert_eq!(after.stage, 3, "the quest is complete");
        assert_eq!(after.words, [0; 5], "the incantation is forgotten");
        assert_eq!(find(engine, weakened), None, "Delrith is gone");
        assert_eq!(
            total(engine, &client, "worn", "silverlight"),
            1,
            "Silverlight is kept"
        );
    });
}
