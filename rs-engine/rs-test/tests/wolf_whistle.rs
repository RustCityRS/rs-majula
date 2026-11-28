//! Wolf Whistle at 2011: the 2008 quest, as cache 786 holds it (quest config 52, done at
//! `wolf_quest` = 110), the way into Summoning.
//!
//! Lines, messages and counts are the dated recordings' (`target/refs/wolf_whistle/.claude/647-wolf-whistle-states.md`,
//! 17 recordings from 2009-09 to 2011-10) and the 2011 wiki's (Wolf Whistle, 23 May 2011). The
//! stage values between 10 and 110 are content's own (`quests/quest_wolf`).

#![cfg(since_647)]

use std::sync::Mutex;

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_pack::cache::varp::VarPlayerScope;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::names::{BACKPACK, com, iface};
use rs_test::runner;
use rs_test::server::{self, Arg};
use rs_test::talk::{Conversation, converse, talk_to};
use rs_ver::TypeId;

static TURN: Mutex<()> = Mutex::new(());

const ANIMALLORE: usize = 23;
/// Pikkupstix's ladder (loc 28572), his trapdoor (multiloc 4039), the cellar's obelisk (28716)
/// and Stikklebrix's body (28586): cache 786's maps l45_53, l34_83 and l44_54.
const LADDER: (u16, u16) = (2925, 3442);
const TRAPDOOR: (u16, u16) = (2927, 3444);
const OBELISK: (u16, u16) = (2209, 5344);
const BODY: (u16, u16) = (2853, 3473);
/// A tile of the cellar, south of the obelisk.
const CELLAR: (u16, u16) = (2209, 5347);

fn id(engine: &Engine, kind: &str, name: &str) -> TypeId {
    let id = match kind {
        "obj" => engine.cache.objs.get_by_debugname(name).map(|t| t.id),
        "npc" => engine.cache.npcs.get_by_debugname(name).map(|t| t.id),
        "loc" => engine.cache.locs.get_by_debugname(name).map(|t| t.id),
        "varp" => engine.cache.varps.get_by_debugname(name).map(|t| t.id),
        _ => unreachable!(),
    };
    id.unwrap_or_else(|| panic!("no {kind} {name}"))
}

fn varp(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let v = id(engine, "varp", name);
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(v.g2())
        .as_int()
}

/// Bits `from..=to` of varp 1178, `wolf_var`: `wolf_quest` 1-10, `wolf_trapdoor` 11-12,
/// `wolf_items` 13, `wolf_bones_taken` 14-15 (cache 786's varbits 4302-4306).
fn wolf_bits(engine: &Engine, client: &HarnessClient, from: u32, to: u32) -> i32 {
    (varp(engine, client, "wolf_var") >> from) & ((1 << (to - from + 1)) - 1)
}

fn stage(engine: &Engine, client: &HarnessClient) -> i32 {
    wolf_bits(engine, client, 1, 10)
}

fn trapdoor(engine: &Engine, client: &HarnessClient) -> i32 {
    wolf_bits(engine, client, 11, 12)
}

fn give(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str, count: u32) {
    let o = id(engine, "obj", name);
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap()
        .set(slot, o, count);
}

fn held(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let o = id(engine, "obj", name);
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap()
        .total(o) as i32
}

fn settle(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) {
    for _ in 0..ticks {
        engine.cycle();
    }
    client.packets();
}

fn at(engine: &Engine, client: &HarnessClient) -> (u16, u8, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.y(), c.z())
}

fn is_open(engine: &Engine, client: &HarnessClient, iface: u16) -> bool {
    engine
        .harness_player(client.pid)
        .unwrap()
        .is_interface_open(iface)
}

fn no_script_errors(engine: &mut Engine, what: &str) {
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "{what}: {errors:?}");
}

/// A walkable tile beside (x, z) on `level`.
fn beside(x: u16, level: u8, z: u16) -> (u16, u16) {
    [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
        .find(|&(x, z)| !rsmod::is_flagged(x, z, level, 0x100 | 0x200000))
        .unwrap_or_else(|| panic!("a walkable tile beside {x}, {z}"))
}

/// Logs in with the quest at `stage`, standing in Pikkupstix's house.
fn login(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    set_stage(engine, &client, stage);
    let (x, z) = beside(LADDER.0, 0, LADDER.1);
    engine.harness_telejump(client.pid, x, 0, z);
    settle(engine, &mut client, 3);
    client
}

fn set_stage(engine: &mut Engine, client: &HarnessClient, stage: i32) {
    let v = id(engine, "varp", "wolf_var");
    let old = varp(engine, client, "wolf_var");
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(v.g2(), VarValue::Int((old & !(0x3FF << 1)) | (stage << 1)));
}

/// The npc of that type nearest the player: (nid, x, level, z).
fn npc(engine: &Engine, client: &HarnessClient, name: &str) -> Option<(u16, u16, u8, u16)> {
    let want = id(engine, "npc", name);
    let (px, _, pz) = at(engine, client);
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == want)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.y(), c.z())
        })
        .min_by_key(|&(_, x, _, z)| x.abs_diff(px) as u32 + z.abs_diff(pz) as u32)
}

/// Talks to Pikkupstix (npc 6988 while the quest runs), taking `choices`.
fn talk(engine: &mut Engine, client: &mut HarnessClient, choices: &[&str]) -> Conversation {
    let (nid, ..) = npc(engine, client, "npc_6988").expect("Pikkupstix is spawned");
    talk_to(client, nid);
    let talk = converse(engine, client, choices, 400);
    assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
    talk
}

/// Answers every dialogue box for up to `rounds` x 20 ticks, until `done`: a cutscene's boxes
/// come with waits between them. A harness client sends nothing on its own, and the engine
/// drops a client that is silent for 100 ticks.
fn play(
    engine: &mut Engine,
    client: &mut HarnessClient,
    rounds: usize,
    done: impl Fn(&Engine, &HarnessClient, &Conversation) -> bool,
) -> Conversation {
    let mut all = Conversation::default();
    for _ in 0..rounds {
        client.send(ClientProt::NoTimeout, &[]);
        let t = converse(engine, client, &[], 20);
        all.texts.extend(t.texts);
        all.messages.extend(t.messages);
        all.packets.extend(t.packets);
        if done(engine, client, &all) {
            break;
        }
    }
    all
}

fn says(t: &Conversation, lines: &[&str]) {
    for line in lines {
        assert!(
            t.said(line),
            "2011 shows {line:?}; {:?} (chat {:?})",
            t.texts,
            t.messages
        );
    }
}

fn messaged(messages: &[String], lines: &[&str]) {
    for line in lines {
        assert!(
            messages.iter().any(|m| m == line),
            "2011 shows {line:?}; {messages:?}"
        );
    }
}

/// Clicks `op` on a loc and returns the game messages of the next `ticks`.
fn op_loc(
    engine: &mut Engine,
    client: &mut HarnessClient,
    op: u8,
    at: (u16, u16),
    loc: &str,
    ticks: usize,
) -> Vec<String> {
    let loc = id(engine, "loc", loc);
    client.op_loc(op, at.0, at.1, loc);
    let mut out = Vec::new();
    for _ in 0..ticks {
        engine.cycle();
        out.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    out
}

/// Clicks a component and returns the packets of the next 6 ticks.
fn click(
    engine: &mut Engine,
    client: &mut HarnessClient,
    op: u8,
    com: i32,
    slot: u16,
) -> Vec<(u16, Vec<u8>)> {
    click_obj(engine, client, op, com, slot, TypeId::NULL)
}

/// Clicks a component slot that draws `obj`, as the client sends it, and returns the packets of
/// the next 6 ticks.
fn click_obj(
    engine: &mut Engine,
    client: &mut HarnessClient,
    op: u8,
    com: i32,
    slot: u16,
    obj: TypeId,
) -> Vec<(u16, Vec<u8>)> {
    client.if_button(op, com, slot, obj);
    let mut out = Vec::new();
    for _ in 0..6 {
        engine.cycle();
        out.extend(client.packets());
    }
    out
}

fn messages(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    HarnessClient::game_messages(packets)
        .into_iter()
        .map(|(_, t)| t)
        .collect()
}

/// Stands in the cellar with the quest at `stage` and the trapdoor open.
fn in_the_cellar(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    let mut client = login(engine, name, stage);
    engine.harness_telejump(client.pid, CELLAR.0, 0, CELLAR.1);
    settle(engine, &mut client, 3);
    client
}

/// The quest's kit as Pikkupstix hands it over, with two wolf bones.
fn give_kit(engine: &mut Engine, client: &HarnessClient) {
    give(engine, client, 0, "wolf_druid_pouch", 1);
    give(engine, client, 1, "wolf_druid_pouch", 1);
    give(engine, client, 2, "wolf_creature_charm", 1);
    give(engine, client, 3, "wolf_creature_charm", 1);
    give(engine, client, 4, "wolf_key", 1);
    give(engine, client, 5, "wolf_spirit_shard", 14);
    give(engine, client, 6, "wolf_bones", 1);
    give(engine, client, 7, "wolf_bones", 1);
}

/// Stands in the attic at stage 50 (pouch and scrolls made, Pikkupstix spoken to) with one
/// spirit wolf pouch and ten Howl scrolls, the wolpertinger out as cutscene 2 leaves it.
fn in_the_attic(engine: &mut Engine, name: &str) -> HarnessClient {
    let mut client = login(engine, name, 50);
    give(engine, &client, 0, "wolf_wolf_pouch", 1);
    give(engine, &client, 1, "lore_howl", 10);
    let msgs = op_loc(engine, &mut client, 1, LADDER, "wolf_druid_quest_ladder", 3);
    assert!(msgs.is_empty(), "{msgs:?}");
    let t = play(engine, &mut client, 12, |_, _, t| {
        t.said("I'll use the spirit wolf pouch. I hope this works.")
    });
    says(
        &t,
        &[
            // 2011's: QlQ_p-MBxJ8 2011-02-09 @269, 2EfU5Mrh8Kc 2011-05-24 @217.
            "Mustn't look round, bunny will eat me.",
            "Okay, I have the spirit wolf pouch, so it can't hurt me...",
            "...I hope.",
            "Oh, good. It's too busy gnawing to notice me.",
            "I spoke too soon!",
            "I'll use the spirit wolf pouch. I hope this works.",
        ],
    );
    settle(engine, &mut client, 3);
    assert_eq!(
        at(engine, &client).1,
        1,
        "cutscene 2 leaves the player upstairs"
    );
    assert_eq!(stage(engine, &client), 50);
    client
}

/// Summon on the pouch in backpack slot 0 (iop4 is the backpack's button 7).
fn summon(engine: &mut Engine, client: &mut HarnessClient) -> Vec<(u16, Vec<u8>)> {
    let pouch = id(engine, "obj", "wolf_wolf_pouch");
    client.if_button(7, BACKPACK, 0, pouch);
    let mut out = Vec::new();
    for _ in 0..4 {
        engine.cycle();
        out.extend(client.packets());
    }
    out
}

/// The quest's progress and its trapdoor are bits of varp 1178 (`wolf_var`), the client's own,
/// whose scope the cache does not carry: a save keeps only perm varps.
#[test]
fn the_quests_varp_is_saved() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let varp = engine
            .cache
            .varps
            .get_by_debugname("wolf_var")
            .expect("wolf_var is registered");
        assert_eq!(varp.scope, VarPlayerScope::Perm);
    });
}

#[test]
fn pikkupstix_starts_the_quest_in_2011s_words() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wolf_starter", 0);
        let t = talk(engine, &mut client, &["Talk about Wolf Whistle quest."]);
        says(
            &t,
            &[
                // 2011's: QlQ_p-MBxJ8 2011-02-09 @64-91, ROB_P-ogto4 2011-03-01 @29-155.
                "Talk about Pikkenmix.",
                "You there! What are you doing here, as if I didn't have enough troubles?",
                "I'm just a passing adventurer. Who are you?",
                "Who am I? My dear boy, I just so happen to be one of the most important druids in this village!",
                "Wow, I don't meet that many celebrities.",
                "What do you do here?",
                "Well, firstly, I-",
                "There is a loud crash from upstairs, followed by the tinkling of broken glass.",
                "Confound you, lapine menace!",
                "A resounding crash comes from upstairs.",
                "Tell me, have you ever heard of Summoning?",
                "Gah! That better not be my wardrobe!",
                "I sent my assistant, Stikklebrix, out to bring me what I needed, but he has not returned.",
                "Like a rabbit, because it is-",
                "Oh, don't worry, I'll deal with this myself. One rabbit stew coming right up.",
                "Very well, young fool. You go see how well you do against it.",
            ],
        );
        assert_eq!(stage(engine, &client), 10, "Wolf Whistle started");
        no_script_errors(engine, "the first talk");
    });
}

#[test]
fn the_first_climb_shows_the_wolpertinger_and_ends_downstairs() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wolf_climber", 10);
        op_loc(engine, &mut client, 1, LADDER, "wolf_druid_quest_ladder", 3);
        let t = play(engine, &mut client, 15, |e, c, _| stage(e, c) == 20);
        says(
            &t,
            &[
                // 2011's: JQZnlZk0KqE 2011-05-24 @122-142, 2EfU5Mrh8Kc 2011-05-24 @70-93.
                "Come on then little, fluffy...",
                "Gigantic...",
                "scary-looking...",
                "razor-fanged...",
                "bunny...",
                "What is that thing?",
                "Maybe, if I leave quietly, it won't notice me.",
                "A chilling, unnatural fear envelops you!",
                "It's coming to get me!",
            ],
        );
        assert_eq!(stage(engine, &client), 20, "the wolpertinger seen");
        settle(engine, &mut client, 8);
        let (x, level, z) = at(engine, &client);
        assert_eq!(level, 0, "back downstairs");
        assert!(
            x.abs_diff(LADDER.0) <= 3 && z.abs_diff(LADDER.1) <= 3,
            "beside the ladder: {x}, {z}"
        );
        no_script_errors(engine, "cutscene 1");
    });
}

#[test]
fn pikkupstix_asks_for_two_lots_of_wolf_bones() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wolf_volunteer", 20);
        let t = talk(
            engine,
            &mut client,
            &["Talk about Wolf Whistle quest.", "I'll get right on it."],
        );
        says(
            &t,
            &[
                // 2011's: QlQ_p-MBxJ8 2011-02-09 @130-150, ROB_P-ogto4 2011-03-01 @214-270.
                "The teeth!",
                "So, you finally saw what you're up against, eh? Not as harmless as you assumed, I take it?",
                "A wolpertinger, and a pretty big one at that.",
                "They are spirits that tend to be on the energetic and destructive side when they manifest here, but are a little less violent on the spirit plane.",
                "through, as I said before.",
                "You need to bring two lots of wolf bones. I can provide the other items you will need.",
                "Wait. Why do you need those things?",
                "I'll get right on it.",
            ],
        );
        assert!(
            !t.said("spiritual plane"),
            "2009's b07; 2011 says \"spirit plane\": {:?}",
            t.texts
        );
        assert_eq!(stage(engine, &client), 30, "volunteered");

        // The other answer: lGcCWqJlpSo 2010-01-01 @112-127, the only recording that takes it.
        let mut client = login(engine, "wolf_volunteer", 20);
        let t = talk(
            engine,
            &mut client,
            &[
                "Talk about Wolf Whistle quest.",
                "Wait. Why do you need those things?",
            ],
        );
        says(
            &t,
            &[
                "Well, I can tell you in depth later, but, since you ask, those are the ingredients for a spirit wolf pouch.",
                "The scrolls allow a spirit wolf to perform a special move, allowing it Howl at the giant wolpertinger, sending it home.",
                "Sorry for asking.",
            ],
        );
        assert_eq!(stage(engine, &client), 30, "volunteered, having asked why");
        no_script_errors(engine, "the second talk");
    });
}

#[test]
fn the_journal_tells_the_player_to_get_two_lots_of_wolf_bones() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wolf_reader", 30);
        // View on the quest list's row 131 (struct_642, param_847).
        client.if_button(1, com("questlist_v4:quests"), 131, TypeId::NULL);
        let mut texts = Vec::new();
        for _ in 0..3 {
            engine.cycle();
            texts.extend(server::settexts(&client.packets()));
        }
        let plain: String = texts
            .iter()
            .map(|(_, t)| {
                let mut out = String::new();
                let mut tag = false;
                for ch in t.chars() {
                    match ch {
                        '<' => tag = true,
                        '>' => tag = false,
                        _ if !tag => out.push(ch),
                        _ => {}
                    }
                }
                out
            })
            .collect::<Vec<_>>()
            .join(" ");
        // 2011's: HemCMqn3Y3E 2009-11-20 @91-94 (EwNMHpeFCRk 2010-03-27 @96-99 the same).
        for line in [
            "Wolf Whistle",
            "It appears that I have underestimated the rabbit in this case; it is some huge rabbit-wolf-monster-bird-thing. I think I should speak to Pikkupstix to find out what is going on.",
            "I have spoken to Pikkupstix, who has promised to teach me the secrets of Summoning if I can help dismiss the giant wolpertinger. To do this, I need to bring him 2 lots of wolf bones.",
            "I need to get 2 lots of wolf bones.",
        ] {
            let squashed = plain.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(squashed.contains(line), "2011 shows {line:?}; {squashed:?}");
        }
        no_script_errors(engine, "the journal");
    });
}

#[test]
fn stikklebrix_gives_two_wolf_bones_once() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wolf_searcher", 30);
        // The body is 2x2; stand south of its south-west tile.
        engine.harness_telejump(client.pid, BODY.0, 0, BODY.1 - 1);
        settle(engine, &mut client, 3);
        let body = id(engine, "loc", "dead_body");
        client.op_loc(1, BODY.0, BODY.1, body);
        let t = converse(engine, &mut client, &[], 100);
        says(
            &t,
            &[
                // 2011's: WKPggURQByY 2011-10-30 @175-181, zGpeA1CrlVg 2010-11-28 @42-46.
                "This must be Stikklebrix.",
                "The poor fool didn't stand a chance against the wolves.",
                "Let's see what he has in his pack: three unfinished Strength potions, two burnt lobsters, a guam leaf with a footprint on it...",
                "and some wolf bones!",
                "It looks he got enough wolf bones for Pikkupstix and then died. How tragic!",
                "Well, his sacrifice will not be in vain.",
            ],
        );
        assert_eq!(
            held(engine, &client, "wolf_bones"),
            2,
            "two lots of wolf bones"
        );
        assert_eq!(wolf_bits(engine, &client, 14, 15), 1, "wolf_bones_taken");

        // With the bones held: ROB_P-ogto4 2011-03-01 @289-296.
        let body = id(engine, "loc", "dead_body");
        client.op_loc(1, BODY.0, BODY.1, body);
        let t = converse(engine, &mut client, &[], 100);
        says(
            &t,
            &[
                "This must be Stikklebrix.",
                "The poor fool didn't stand a chance against the wolves.",
                "I don't have time to search him. I have to get these wolf bones back to Pikkupstix so that Stikklebrix's sacrifice is not in vain.",
            ],
        );
        assert!(!t.said("and some wolf bones!"), "{:?}", t.texts);
        assert_eq!(held(engine, &client, "wolf_bones"), 2, "no more bones");
        no_script_errors(engine, "the dead body");
    });
}

#[test]
fn pikkupstix_hands_over_the_kit_for_two_wolf_bones() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wolf_fetcher", 30);
        give(engine, &client, 0, "wolf_bones", 1);
        give(engine, &client, 1, "wolf_bones", 1);
        let t = talk(engine, &mut client, &["Talk about Wolf Whistle quest."]);
        says(
            &t,
            &[
                // 2011's: QlQ_p-MBxJ8 2011-02-09 @194-208, ROB_P-ogto4 2011-03-01 @308-354.
                "I have the wolf bones right here.",
                "Splendid, dear boy.",
                "Here are the pouches, spirit shards and charms you will need to make the spirit wolf pouch and Howl scrolls. This key is to the trapdoor over there, which leads to the obelisk.",
                "perform a fear-inducing Howl special move.",
                "the pouch, mixing with the spirit of a familiar to create a spirit wolf pouch.",
                "It all sounds simple enough.",
            ],
        );
        assert_eq!(stage(engine, &client), 40, "kit handed over");
        // 2011 wiki, Wolf Whistle (23 May 2011); HemCMqn3Y3E 2009-11-20 @162 shows the same.
        assert_eq!(held(engine, &client, "wolf_druid_pouch"), 2);
        assert_eq!(held(engine, &client, "wolf_creature_charm"), 2);
        assert_eq!(held(engine, &client, "wolf_key"), 1);
        assert_eq!(held(engine, &client, "wolf_spirit_shard"), 14);
        // The bones are still held after the talk (HemCMqn3Y3E @162).
        assert_eq!(held(engine, &client, "wolf_bones"), 2);
        assert_eq!(wolf_bits(engine, &client, 13, 13), 1, "wolf_items");
        assert!(
            !is_open(engine, &client, 519) && !is_open(engine, &client, 131),
            "no item box anywhere in the quest"
        );
        no_script_errors(engine, "the hand-over");
    });
}

#[test]
fn the_trapdoor_is_locked_until_the_key_opens_it() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wolf_keyholder", 40);
        let (x, z) = (TRAPDOOR.0, TRAPDOOR.1 - 1);
        engine.harness_telejump(client.pid, x, 0, z);
        settle(engine, &mut client, 3);
        // 2011's: HemCMqn3Y3E 2009-11-20 @43.
        let said = op_loc(engine, &mut client, 1, TRAPDOOR, "wolf_multi_trapdoor", 4);
        messaged(&said, &["This trapdoor appears to be locked."]);
        assert_eq!(trapdoor(engine, &client), 0, "still locked");

        give(engine, &client, 0, "wolf_key", 1);
        // 2011's: 2EfU5Mrh8Kc 2011-05-24 @158, JQZnlZk0KqE 2011-05-24 @310: plain Open.
        let said = op_loc(engine, &mut client, 1, TRAPDOOR, "wolf_multi_trapdoor", 4);
        messaged(&said, &["You unlock the trapdoor."]);
        assert_eq!(
            trapdoor(engine, &client),
            2,
            "open: the multiloc shows 28676"
        );

        // Climb-down, to the obelisk's cave.
        op_loc(engine, &mut client, 1, TRAPDOOR, "wolf_multi_trapdoor", 6);
        let (x, _, z) = at(engine, &client);
        assert!(
            x.abs_diff(OBELISK.0) < 10 && z.abs_diff(OBELISK.1) < 10,
            "in the cellar: {x}, {z}"
        );
        no_script_errors(engine, "the trapdoor");
    });
}

#[test]
fn one_infuse_makes_both_pouches_and_one_transform_ten_scrolls() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = in_the_cellar(engine, "wolf_infuser", 40);
        give_kit(engine, &client);
        let creation = iface("interface_672");
        let scrolls = iface("interface_666");

        let obelisk = id(engine, "loc", "wolf_obelisk_druid01");
        client.op_loc(1, OBELISK.0, OBELISK.1, obelisk);
        let mut packets = Vec::new();
        for _ in 0..6 {
            engine.cycle();
            packets.extend(client.packets());
        }
        assert!(
            is_open(engine, &client, creation),
            "Summoning Pouch Creation"
        );
        // Clientscript 791 fills 672:16: 8 columns, slots 1-78 of enum 1182, the op "Infuse".
        let grid = com("interface_672:scroll_icons");
        let filled = server::scripts(&packets)
            .into_iter()
            .find(|(script, _)| *script == 791)
            .expect("clientscript 791");
        assert!(
            matches!(filled.1.first(), Some(Arg::Int(c)) if *c == grid),
            "{:?}",
            filled.1
        );
        assert!(
            filled
                .1
                .iter()
                .any(|a| matches!(a, Arg::Str(s) if s.starts_with("Infuse"))),
            "{:?}",
            filled.1
        );

        // The spirit wolf pouch is slot 1 of the grid: its clickable child is 0 * 5 + 2.
        // The client sends the obj the slot draws (clientscript 794's cc_setobject): seen on
        // world1 2026-11-27, menu entry "Infuse Spirit wolf pouch" with slot 2.
        let drawn = id(engine, "obj", "lore_wolf");
        let packets = click_obj(engine, &mut client, 1, grid, 2, drawn);
        // 2011's: 2EfU5Mrh8Kc 2011-05-24 @168, JQZnlZk0KqE 2011-05-24 @322-330.
        messaged(
            &messages(&packets),
            &[
                "You infuse a spirit wolf pouch.",
                "You now need to transform a spirit wolf pouch into some Howl scrolls.",
            ],
        );
        assert_eq!(held(engine, &client, "wolf_wolf_pouch"), 2, "both pouches");
        for gone in [
            "wolf_druid_pouch",
            "wolf_creature_charm",
            "wolf_spirit_shard",
            "wolf_bones",
        ] {
            assert_eq!(held(engine, &client, gone), 0, "{gone} used up");
        }
        assert_eq!(held(engine, &client, "wolf_key"), 1);
        assert!(!is_open(engine, &client, creation), "the window closes");
        assert_eq!(
            engine.harness_player(client.pid).unwrap().stats.xp[ANIMALLORE],
            0,
            "no xp during the quest"
        );

        // Transform Scrolls swaps the window to Scroll Creation, filled by clientscript 796.
        client.op_loc(1, OBELISK.0, OBELISK.1, obelisk);
        settle(engine, &mut client, 6);
        assert!(is_open(engine, &client, creation));
        let packets = click(
            engine,
            &mut client,
            1,
            com("interface_672:tab_back"),
            0xFFFF,
        );
        assert!(is_open(engine, &client, scrolls), "Scroll Creation");
        let scroll_grid = com("interface_666:scroll_icons");
        let filled = server::scripts(&packets)
            .into_iter()
            .find(|(script, _)| *script == 796)
            .expect("clientscript 796");
        assert!(
            matches!(filled.1.first(), Some(Arg::Int(c)) if *c == scroll_grid),
            "{:?}",
            filled.1
        );
        assert!(
            filled
                .1
                .iter()
                .any(|a| matches!(a, Arg::Str(s) if s.starts_with("Transform"))),
            "{:?}",
            filled.1
        );
        // And 666:18 swaps back.
        click(
            engine,
            &mut client,
            1,
            com("interface_666:tab_back"),
            0xFFFF,
        );
        assert!(is_open(engine, &client, creation), "back to pouches");
        click(
            engine,
            &mut client,
            1,
            com("interface_672:tab_back"),
            0xFFFF,
        );

        let drawn = id(engine, "obj", "lore_howl");
        let packets = click_obj(engine, &mut client, 1, scroll_grid, 2, drawn);
        // 2011's: 2EfU5Mrh8Kc 2011-05-24 @178, JQZnlZk0KqE 2011-05-24 @352.
        messaged(
            &messages(&packets),
            &[
                "You transform a spirit wolf pouch into some Howl scrolls.",
                "You should speak to Pikkupstix. He will tell you what to do with the scrolls and the spirit wolf pouch.",
            ],
        );
        assert_eq!(held(engine, &client, "lore_howl"), 10, "ten Howl scrolls");
        assert_eq!(
            held(engine, &client, "wolf_wolf_pouch"),
            1,
            "one pouch left"
        );
        assert!(!is_open(engine, &client, scrolls), "the window closes");
        assert_eq!(stage(engine, &client), 40, "Pikkupstix moves the quest on");
        no_script_errors(engine, "the obelisk");
    });
}

#[test]
fn the_cellar_is_a_summoning_free_area() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = in_the_cellar(engine, "wolf_too_soon", 40);
        give(engine, &client, 0, "wolf_wolf_pouch", 1);
        let packets = summon(engine, &mut client);
        // 2011's: HemCMqn3Y3E 2009-11-20 @168.
        messaged(
            &messages(&packets),
            &[
                "You are standing in a Summoning-free area. You must move out of this area to summon a familiar.",
            ],
        );
        assert_eq!(held(engine, &client, "wolf_wolf_pouch"), 1, "pouch kept");
        assert!(
            npc(engine, &client, "lore_wolf").is_none()
                || varp(engine, &client, "follower_uid") <= 0
        );
        no_script_errors(engine, "summoning in the cellar");
    });
}

#[test]
fn pouch_and_scrolls_send_the_player_upstairs() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wolf_maker", 40);
        give(engine, &client, 0, "wolf_wolf_pouch", 1);
        give(engine, &client, 1, "lore_howl", 10);
        let t = talk(engine, &mut client, &["Talk about Wolf Whistle quest."]);
        says(
            &t,
            &[
                // 2011's: QlQ_p-MBxJ8 2011-02-09 @243-261, ROB_P-ogto4 2011-03-01 @422-466.
                "Here is the pouch and scroll.",
                "Wonderful! Now, all you have to do is go upstairs and summon the spirit wolf, then the Howl effect.",
                "scared that it will retreat to its spirit plane, rather than face the spirit wolf.",
                "To access your follower details, right-click the Summoning icon near your minimap and select 'Follower Details'.",
                "You can also access your familiar's spell and various other functions from this menu.",
                "Bear in mind that the icon will only function so long as you have a Summoning follower present.",
                "Wish me luck!",
            ],
        );
        assert_eq!(stage(engine, &client), 50, "sent upstairs");
        no_script_errors(engine, "the fourth talk");
    });
}

#[test]
fn summoning_in_the_attic_brings_the_spirit_wolf() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = in_the_attic(engine, "wolf_summoner");
        assert!(
            npc(engine, &client, "giant_wolpertinger").is_some_and(|(_, _, level, _)| level == 1),
            "the giant wolpertinger stays in the attic"
        );
        let packets = summon(engine, &mut client);
        // 2011's: 2EfU5Mrh8Kc 2011-05-24 @246, JQZnlZk0KqE 2011-05-24 @415.
        messaged(
            &messages(&packets),
            &[
                "You have summoned the spirit wolf! Now you need to use the spirit wolf's special move - Howl - on the giant wolpertinger to banish it.",
            ],
        );
        assert_eq!(held(engine, &client, "wolf_wolf_pouch"), 0, "pouch used");
        let (nid, _, level, _) = npc(engine, &client, "lore_wolf").expect("the spirit wolf");
        assert_eq!(level, 1);
        let follower = engine.get_npc(nid).unwrap().npc.uid.packed();
        assert_eq!(
            varp(engine, &client, "follower_uid"),
            follower as i32,
            "the wolf is the player's follower"
        );
        // What the client's Follower Details reads (cache 786's interface 662 and its scripts).
        let wolf = id(engine, "npc", "lore_wolf").int();
        assert_eq!(varp(engine, &client, "lore_npc_head"), wolf);
        assert_eq!(varp(engine, &client, "lore_spell_points"), 60, "60/60");
        let timer = varp(engine, &client, "lore_timer_var");
        assert_eq!(timer >> 7, 6, "6.00 left (varbit 4534)");
        assert_eq!((timer >> 6) & 1, 0, "no half minute (varbit 4290)");
        assert_eq!(
            (varp(engine, &client, "lore_npc_stats") >> 23) & 0x1F,
            3,
            "Howl takes 3 special move points (varbit 4288)"
        );
        assert_eq!(
            server::varcstrs(&packets)
                .into_iter()
                .filter(|(v, _)| *v == 204 || *v == 205)
                .collect::<Vec<_>>(),
            [
                (204, "Howl".to_owned()),
                (205, "Makes NPCs flee for a short time".to_owned())
            ],
            "the special move's name and description (HemCMqn3Y3E 2009-11-20 @206)"
        );
        // Follower Details opens in side tab 95 and the client switches to it.
        let details = iface("lore_stats_side");
        assert!(
            server::subs(&packets)
                .iter()
                .any(|&(sub, parent)| sub == details && parent == com("toplevel:side95")),
            "662 in 548:220"
        );
        assert!(is_open(engine, &client, details));
        // The special icon's child 0 takes an npc target, on the panel and on the orb.
        let events = server::events(&packets);
        for icon in ["lore_stats_side:spell_icon", "topstat_lore:spell_icon"] {
            assert!(
                events
                    .iter()
                    .any(|&(c, from, to, e)| c == com(icon) && from == 0 && to == 0 && e != 0),
                "{icon}: {events:?}"
            );
        }
        // Points are untouched until the banishing (HemCMqn3Y3E @206: 1/1).
        assert_eq!(
            engine.harness_player(client.pid).unwrap().stats.levels[ANIMALLORE],
            1
        );

        // 2011's: HemCMqn3Y3E 2009-11-20 @212, Interact on the wolf.
        client.op_npc(1, nid);
        let mut said = Vec::new();
        for _ in 0..4 {
            engine.cycle();
            said.extend(messages(&client.packets()));
        }
        messaged(
            &said,
            &["The wolf is growling at the giant wolpertinger, and will not reply to you."],
        );
        no_script_errors(engine, "the summon");

        // Logging out takes the wolf with it.
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .logout_requested = true;
        settle(engine, &mut client, 4);
        assert!(engine.harness_player(client.pid).is_none(), "logged out");
        assert!(
            engine
                .get_npc(nid)
                .is_none_or(|n| n.npc.uid.packed() != follower)
        );
        no_script_errors(engine, "the logout");
    });
}

#[test]
fn leaving_the_attic_dismisses_the_wolf() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // Out of the attic by any way but the ladder (a teleport, a death): the attic's timer
        // sends the wolf away, back into its pouch.
        let mut client = in_the_attic(engine, "wolf_leaver");
        summon(engine, &mut client);
        let (nid, ..) = npc(engine, &client, "lore_wolf").expect("the spirit wolf");
        let follower = engine.get_npc(nid).unwrap().npc.uid.packed();
        assert_eq!(held(engine, &client, "wolf_wolf_pouch"), 0);
        engine.harness_telejump(client.pid, 3222, 0, 3218);
        settle(engine, &mut client, 8);
        assert!(
            engine
                .get_npc(nid)
                .is_none_or(|n| n.npc.uid.packed() != follower)
        );
        assert!(varp(engine, &client, "follower_uid") <= 0, "no follower");
        assert_eq!(
            held(engine, &client, "wolf_wolf_pouch"),
            1,
            "the pouch is back"
        );
        no_script_errors(engine, "teleporting out of the attic");

        // And by the ladder.
        let mut client = in_the_attic(engine, "wolf_leaver");
        summon(engine, &mut client);
        let (nid, ..) = npc(engine, &client, "lore_wolf").expect("the spirit wolf");
        let follower = engine.get_npc(nid).unwrap().npc.uid.packed();
        let top = id(engine, "loc", "wolf_quest_druid_laddertop");
        client.op_loc(1, LADDER.0, LADDER.1, top);
        let mut packets = Vec::new();
        for _ in 0..14 {
            engine.cycle();
            packets.extend(client.packets());
        }
        assert_eq!(at(engine, &client).1, 0, "climbed down");
        assert!(
            engine
                .get_npc(nid)
                .is_none_or(|n| n.npc.uid.packed() != follower)
        );
        assert!(varp(engine, &client, "follower_uid") <= 0, "no follower");
        assert_eq!(varp(engine, &client, "lore_npc_head"), -1);
        assert!(!is_open(engine, &client, iface("lore_stats_side")));
        assert!(
            server::closes(&packets).contains(&com("toplevel:side95")),
            "side tab 95 shut"
        );
        assert!(
            server::scripts(&packets).iter().any(|(s, _)| *s == 2471),
            "clientscript 2471 hides the orb's action layer"
        );
        no_script_errors(engine, "leaving the attic");
    });
}

#[test]
fn howl_banishes_the_wolpertinger_and_drains_the_points() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = in_the_attic(engine, "wolf_howler");
        summon(engine, &mut client);
        let (wolf, ..) = npc(engine, &client, "lore_wolf").expect("the spirit wolf");
        let follower = engine.get_npc(wolf).unwrap().npc.uid.packed();
        let (bunny, ..) = npc(engine, &client, "giant_wolpertinger").expect("the wolpertinger");

        // 2011's: HemCMqn3Y3E 2009-11-20 @212-213, the scroll itself used on the wolpertinger.
        let scroll = id(engine, "obj", "lore_howl");
        client.op_npc_t(bunny, BACKPACK, 1, scroll);
        let mut said = Vec::new();
        for _ in 0..12 {
            engine.cycle();
            said.extend(messages(&client.packets()));
        }
        messaged(
            &said,
            &[
                "These scrolls can only be used by clicking the Special Move button in the Follower Details tab.",
            ],
        );
        assert_eq!(held(engine, &client, "lore_howl"), 10);

        // The special move's icon, aimed at the giant wolpertinger.
        client.op_npc_t(bunny, com("lore_stats_side:spell_icon"), 0, TypeId::NULL);
        let t = play(engine, &mut client, 15, |e, c, _| stage(e, c) == 60);
        // 2011's: 2EfU5Mrh8Kc 2011-05-24 @251, JQZnlZk0KqE 2011-05-24 @415.
        messaged(
            &t.messages,
            &["The spirit wolf howls at the giant wolpertinger."],
        );
        says(
            &t,
            &[
                // 2011's: QlQ_p-MBxJ8 2011-02-09 @300-311, 2EfU5Mrh8Kc 2011-05-24 @261-268.
                "Get that thing!",
                "Well, that certainly seems to have got rid of the giant wolpertinger.",
                "Come on, let's go tell Pikkupstix.",
                "They both vanished! I'm sure Pikkupstix will be able to explain this to me.",
            ],
        );
        settle(engine, &mut client, 10);
        assert_eq!(stage(engine, &client), 60, "the wolpertinger banished");
        assert_eq!(held(engine, &client, "lore_howl"), 9, "one scroll used");
        assert_eq!(at(engine, &client).1, 0, "downstairs, beside Pikkupstix");
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.stats.levels[ANIMALLORE], 0, "0 Summoning points");
        assert_eq!(player.stats.base_levels[ANIMALLORE], 1);
        assert!(
            engine
                .get_npc(wolf)
                .is_none_or(|n| n.npc.uid.packed() != follower)
        );
        assert!(varp(engine, &client, "follower_uid") <= 0, "no follower");
        assert!(!is_open(engine, &client, iface("lore_stats_side")));
        assert!(
            npc(engine, &client, "giant_wolpertinger").is_none_or(|(nid, ..)| nid != bunny),
            "the wolpertinger is gone"
        );
        no_script_errors(engine, "cutscene 3");
    });
}

#[test]
fn the_obelisk_renews_the_points_pikkupstix_sent_the_player_for() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wolf_dizzy", 60);
        engine.harness_player_mut(client.pid).unwrap().stats.levels[ANIMALLORE] = 0;
        let t = talk(engine, &mut client, &["Talk about Wolf Whistle quest."]);
        says(
            &t,
            &[
                // 2011's: QlQ_p-MBxJ8 2011-02-09 @325-335, ROB_P-ogto4 2011-03-01 @561-594.
                "So, how did it go?",
                "The spirit wolf scared it away!",
                "Woah, I feel a little dizzy.",
                "Go downstairs to the obelisk, as it will renew your energy.",
                "Wooooah...",
                "You really need to renew your Summoning skill points at the obelisk, Wolf Dizzy.",
            ],
        );
        assert_eq!(stage(engine, &client), 70, "sent to the obelisk");

        engine.harness_telejump(client.pid, CELLAR.0, 0, CELLAR.1);
        settle(engine, &mut client, 3);
        // 2011's: 2EfU5Mrh8Kc 2011-05-24 @306: Infuse-pouch is not for now.
        let said = op_loc(engine, &mut client, 1, OBELISK, "wolf_obelisk_druid01", 5);
        messaged(
            &said,
            &["You do not have the required Summoning knowledge to use this obelisk yet."],
        );
        assert!(!is_open(engine, &client, iface("interface_672")));
        // 2011's: 2EfU5Mrh8Kc 2011-05-24 @310, JQZnlZk0KqE 2011-05-24 @468.
        let said = op_loc(engine, &mut client, 2, OBELISK, "wolf_obelisk_druid01", 6);
        messaged(
            &said,
            &[
                "You have recharged your Summoning skill points.",
                "You should speak to Pikkupstix to get your reward.",
            ],
        );
        assert_eq!(
            engine.harness_player(client.pid).unwrap().stats.levels[ANIMALLORE],
            1,
            "points renewed"
        );
        assert_eq!(stage(engine, &client), 80, "points renewed");
        // 2011's: HemCMqn3Y3E 2009-11-20 @168, ldSYrbNZbI8 2010-07-31 @327.
        let said = op_loc(engine, &mut client, 2, OBELISK, "wolf_obelisk_druid01", 6);
        messaged(&said, &["You already have full Summoning points."]);
        no_script_errors(engine, "the renewal");
    });
}

#[test]
fn pikkupstix_rewards_the_player_with_summoning() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wolf_finisher", 80);
        give(engine, &client, 0, "lore_howl", 9);
        give(engine, &client, 1, "wolf_key", 1);
        let qp = varp(engine, &client, "qp");
        let mut t = talk(engine, &mut client, &["Talk about Wolf Whistle quest."]);
        for _ in 0..10 {
            engine.cycle();
            let packets = client.packets();
            t.texts
                .extend(server::settexts(&packets).into_iter().map(|(_, t)| t));
            t.messages.extend(messages(&packets));
        }
        says(
            &t,
            &[
                // 2011's: QlQ_p-MBxJ8 2011-02-09 @361-382, ROB_P-ogto4 2011-03-01 @623-682.
                "Feeling refreshed?",
                "Yeah, that does feel a lot better!",
                "Think of your Summoning skill points as Prayer Points, that need to be recharged from time-to-time.",
                "One is guarded by the ogres in Gu'Tanoth, and the other is on the coast near Brimhaven. There will be others, however.",
                "Here is a little something to help you start your Summoning training.",
                "Thanks!",
                // The scroll: 2EfU5Mrh8Kc 2011-05-24 @336-341, JQZnlZk0KqE 2011-05-24 @500-504.
                "You have completed Wolf Whistle!",
                "1 Quest Point.",
                "276 Summoning XP.",
                "Access to the Summoning skill.",
                "275 gold charms.",
            ],
        );
        messaged(&t.messages, &["Congratulations! Quest complete!"]);
        assert_eq!(stage(engine, &client), 110, "Wolf Whistle complete");
        assert_eq!(varp(engine, &client, "qp") - qp, 1, "1 quest point");
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(
            player.stats.xp[ANIMALLORE], 2760,
            "276 Summoning xp (tenths)"
        );
        assert_eq!(player.stats.base_levels[ANIMALLORE], 4, "level 4");
        assert_eq!(held(engine, &client, "lore_creature_charm"), 275);
        assert_eq!(held(engine, &client, "lore_howl"), 9, "the 9 scrolls kept");
        assert_eq!(held(engine, &client, "wolf_key"), 0, "leftover kit removed");
        no_script_errors(engine, "the reward");
    });
}

#[test]
fn the_trapdoor_key_says_where_to_get_another() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wolf_destroyer", 40);
        give(engine, &client, 0, "wolf_key", 1);
        // Destroy is iop5, the backpack's button 8.
        let key = id(engine, "obj", "wolf_key");
        client.if_button(8, BACKPACK, 0, key);
        let mut texts = Vec::new();
        for _ in 0..3 {
            engine.cycle();
            texts.extend(server::settexts(&client.packets()));
        }
        // 2011 wiki, Trapdoor key (8 Mar 2011).
        let line = "The trapdoor key will be destroyed if you drop it; you will have to speak to Pikkupstix to get another.";
        assert!(
            texts
                .iter()
                .any(|(c, t)| *c == com("confirm_destroy:destroy_message") && t == line),
            "{texts:?}"
        );
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);

        // And he does give another, to a player he has given the kit (`wolf_items`).
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let wolf_var = id(engine, "varp", "wolf_var");
        let old = varp(engine, &client, "wolf_var");
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.vars.set(wolf_var.g2(), VarValue::Int(old | 1 << 13));
        player.invs.get_mut(&inv).unwrap().delete_slot(0);
        assert_eq!(held(engine, &client, "wolf_key"), 0);
        let (nid, ..) = npc(engine, &client, "npc_6988").expect("Pikkupstix is spawned");
        talk_to(&mut client, nid);
        converse(
            engine,
            &mut client,
            &["Talk about Wolf Whistle quest."],
            100,
        );
        assert_eq!(held(engine, &client, "wolf_key"), 1, "another key");
        assert_eq!(stage(engine, &client), 40);
        no_script_errors(engine, "the key");
    });
}
