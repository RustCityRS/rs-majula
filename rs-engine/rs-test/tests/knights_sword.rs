//! The Knight's Sword at 647, as May 2011 had it (2011-05-04 recording, YouTube lM6IxO8W1kg): 289's
//! quest with 2011's wording (Thurgo's titled menu and "some redberry pie", his blurite "except
//! crossbows", the squire's "try and get"), the player speaking first at the hand-over, and the
//! scroll's "12,725 Smithing XP". %squire is the cache's varp 122, which quest 152 counts to 7.

#![cfg(since_647)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_test::runner;
use rs_test::talk::{Conversation, converse, settexts, talk_to};
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const SMITHING: usize = 13;

fn id(engine: &Engine, kind: &str, name: &str) -> u16 {
    let names = match kind {
        "obj" => &engine.cache.objs.debugnames,
        "npc" => &engine.cache.npcs.debugnames,
        "varp" => &engine.cache.varps.debugnames,
        _ => &engine.cache.invs.debugnames,
    };
    *names
        .get(name)
        .unwrap_or_else(|| panic!("no {kind} {name}"))
}

/// Moves the player beside npc `name`: its nid.
fn to(engine: &mut Engine, client: &HarnessClient, name: &str) -> u16 {
    let npc = id(engine, "npc", name);
    let (nid, nx, y, nz) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == npc)
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
        .unwrap_or_else(|| panic!("a tile beside {name}"));
    engine.harness_telejump(client.pid, sx, y, sz);
    for _ in 0..3 {
        engine.cycle();
    }
    nid
}

fn stage(engine: &Engine, client: &HarnessClient) -> i32 {
    let varp = id(engine, "varp", "squire");
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(varp)
        .as_int()
}

fn give(engine: &mut Engine, client: &HarnessClient, items: &[(&str, u32)]) {
    let inv = id(engine, "inv", "inv");
    let ids: Vec<(u16, u32)> = items
        .iter()
        .map(|&(n, c)| (id(engine, "obj", n), c))
        .collect();
    let pack = engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap();
    for (slot, (o, c)) in ids.into_iter().enumerate() {
        pack.set(slot as u16, o, c);
    }
}

/// Talks to `npc`, answering `answers`, and plays out the rest (the scroll's texts included).
fn talk(
    engine: &mut Engine,
    client: &mut HarnessClient,
    npc: &str,
    answers: &[&str],
) -> Conversation {
    let nid = to(engine, client, npc);
    client.packets();
    talk_to(client, nid);
    let mut talk = converse(engine, client, answers, 400);
    for _ in 0..3 {
        engine.cycle();
        talk.texts
            .extend(settexts(&client.packets()).into_iter().map(|(_, t)| t));
    }
    talk
}

fn shows(talk: &Conversation, lines: &[&str]) {
    for line in lines {
        assert!(talk.said(line), "2011 shows {line:?}; {:?}", talk.texts);
    }
}

#[test]
fn the_knights_sword_as_2011_had_it() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let _ = engine.script_errors.take();
        engine.harness_reset();
        let mut client: HarnessClient = engine
            .harness_login_returning("squire_2011")
            .expect("login");
        for _ in 0..10 {
            engine.cycle();
        }

        // The squire: the quest starts.
        let t = talk(
            engine,
            &mut client,
            "squire",
            &[
                "And how is life as a squire?",
                "I can make a new sword if you like...",
                "So would these dwarves make another one?",
                "Ok, I'll give it a go.",
            ],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        shows(
            &t,
            &[
                "Hello. I am the squire to Sir Vyvin.",
                "Thanks for the offer. I'd be surprised if you could though.",
                "Thank you very much! As I say, the best place to start should be with Reldo...",
            ],
        );
        assert_eq!(stage(engine, &client), 1);

        // Reldo: the Imcando dwarves, wrapped without 289's hard breaks.
        let t = talk(
            engine,
            &mut client,
            "reldo_2661",
            &["What do you know about the Imcando dwarves?"],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        shows(
            &t,
            &[
                "I believe a few of them survived, but with the bulk of their population destroyed their numbers have dwindled even further.",
                "redberry pie. They REALLY like redberry pie.",
            ],
        );
        assert_eq!(stage(engine, &client), 2);

        // Thurgo: 2011's titled menu and "some redberry pie".
        give(engine, &client, &[("redberry_pie", 1)]);
        let t = talk(
            engine,
            &mut client,
            "thurgo",
            &["Would you like some redberry pie?"],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        shows(
            &t,
            &[
                "Choose an option:",
                "Would you like some redberry pie?",
                "By Guthix! THAT was good pie! Anyone who makes pie like THAT has got to be alright!",
            ],
        );
        assert_eq!(stage(engine, &client), 3);

        let t = talk(engine, &mut client, "thurgo", &[]);
        shows(
            &t,
            &[
                "All the Faladian knights used to have swords with unique designs according to their position. Could you bring me a picture or something?",
            ],
        );
        assert_eq!(stage(engine, &client), 4);

        // The squire: "try and get".
        let t = talk(engine, &mut client, "squire", &[]);
        shows(
            &t,
            &[
                "Ok, I'll try and get that then.",
                "Please don't let him catch you! He MUSTN'T know what happened!",
            ],
        );
        assert_eq!(stage(engine, &client), 5);

        // Thurgo with the portrait: blurite "except crossbows".
        give(engine, &client, &[("portrait", 1)]);
        let t = talk(engine, &mut client, "thurgo", &[]);
        shows(
            &t,
            &[
                "I'll need two iron bars to make the sword to start with. I'll also need an ore called blurite. It's useless for making actual weapons for fighting with except crossbows, but I'll need some as decoration for the hilt.",
                "The only place I know where to get it is under this cliff here...",
            ],
        );
        assert!(!t.said("It is fairly rare sort of ore..."), "2011 drops it");
        assert_eq!(stage(engine, &client), 6);

        // Thurgo with the materials: the sword.
        give(
            engine,
            &client,
            &[("blurite_ore", 1), ("iron_bar", 1), ("iron_bar", 1)],
        );
        let t = talk(engine, &mut client, "thurgo", &[]);
        shows(
            &t,
            &[
                "You give the blurite ore and two iron bars to Thurgo. Thurgo starts to make the sword. Thurgo hands you a sword.",
            ],
        );

        // The squire: the player first, then the scroll.
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[SMITHING];
        let t = talk(engine, &mut client, "squire", &[]);
        shows(
            &t,
            &[
                "I have retrieved your sword for you.",
                "Thank you, thank you, thank you! I was seriously worried I would have to own up to Sir Vyvin!",
                "You give the sword to the squire.",
                "You have completed the Knight's Sword Quest!",
                "12,725 Smithing XP",
            ],
        );
        assert!(
            !t.said("So how are you doing getting a sword?"),
            "2011 asks nothing first"
        );
        for _ in 0..3 {
            engine.cycle();
        }
        assert_eq!(stage(engine, &client), 7, "complete");
        assert_eq!(
            engine.harness_player(client.pid).unwrap().stats.xp[SMITHING] - xp,
            127250,
            "12,725 Smithing experience"
        );
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}
