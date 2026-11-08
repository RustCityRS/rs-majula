//! Pirate's Treasure at 647 (`quests/quest_hunt`): 289's quest with 2011's changes from the
//! 2011-04-26 recording (YouTube GTgKzVezVDo; runescape/target/refs/hunt/transcript_0426.md) and
//! the 2011 wiki: Redbeard Frank's "Arr, Matey!" and new lines, Luthas's, Wydin's job at the door,
//! "You unlock the chest.", the gardener's shout and refusal, and Hector's casket as the reward.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{Conversation, converse, talk_to};

static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

const BACKPACK: i32 = 149 << 16;
/// Locs in cache 786's map (maps/l*.jm2): Luthas's crate, Wydin's back-room door and crate, and
/// Hector's chest upstairs in the Blue Moon Inn.
const BANANA_CRATE: (u16, u16, u16) = (2072, 2943, 3151);
const WYDIN_DOOR: (u16, u16, u16) = (2069, 3012, 3204);
const GROCERY_CRATE: (u16, u16, u16) = (2071, 3009, 3207);
const HECTOR_CHEST: (u16, u16, u16) = (2079, 3219, 3396);
/// The X under the Saradomin statue in Falador park (general_use/scripts/spade.rs2).
const DIG: (u16, u16) = (2999, 3383);

fn id(engine: &Engine, kind: &str, name: &str) -> u16 {
    let map = match kind {
        "obj" => &engine.cache.objs.debugnames,
        "npc" => &engine.cache.npcs.debugnames,
        "varp" => &engine.cache.varps.debugnames,
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
        .get(v)
        .as_int()
}

fn set_var(engine: &mut Engine, client: &HarnessClient, name: &str, value: i32) {
    let v = id(engine, "varp", name);
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(v, VarValue::Int(value));
}

fn give(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str, count: u32) {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
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
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
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

fn tele(engine: &mut Engine, client: &mut HarnessClient, x: u16, level: u8, z: u16) {
    engine.harness_telejump(client.pid, x, level, z);
    settle(engine, client, 3);
}

/// The nid of the nearest npc of that name to (x, z), and a walkable tile beside it.
fn npc_near(engine: &Engine, name: &str, x: u16, z: u16) -> (u16, u16, u16) {
    let want = id(engine, "npc", name);
    let (nid, nx, nz) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == want)
        .map(|n| {
            (
                n.npc.uid.nid(),
                n.npc.pathing.coord.x(),
                n.npc.pathing.coord.z(),
            )
        })
        .min_by_key(|&(_, nx, nz)| nx.abs_diff(x) as u32 + nz.abs_diff(z) as u32)
        .unwrap_or_else(|| panic!("{name} is spawned"));
    let (sx, sz) = [
        (1i16, 0i16),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (-1, -1),
        (1, -1),
        (-1, 1),
    ]
    .into_iter()
    .map(|(dx, dz)| (nx.wrapping_add_signed(dx), nz.wrapping_add_signed(dz)))
    .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
    .unwrap_or_else(|| panic!("a walkable tile beside {name}"));
    (nid, sx, sz)
}

fn talk(
    engine: &mut Engine,
    client: &mut HarnessClient,
    name: &str,
    x: u16,
    z: u16,
    choices: &[&str],
) -> Conversation {
    let (nid, sx, sz) = npc_near(engine, name, x, z);
    tele(engine, client, sx, 0, sz);
    talk_to(client, nid);
    converse(engine, client, choices, 300)
}

fn oploc1(client: &mut HarnessClient, loc: (u16, u16, u16)) {
    let (id, x, z) = loc;
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(id as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

fn use_on_loc(client: &mut HarnessClient, obj: u16, slot: u16, loc: (u16, u16, u16)) {
    let (id, x, z) = loc;
    let mut p = ClientPacket::new();
    p.p2_alt2(obj as i32)
        .p2_alt2(slot as i32)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt2(x as i32)
        .p1_alt1(0)
        .p2_alt1(id as i32);
    client.send(ClientProt::OpLocT, &p.finish());
}

fn op_held1(client: &mut HarnessClient, obj: u16, slot: u16) {
    let mut p = ClientPacket::new();
    p.p4(BACKPACK).p2_alt3(obj as i32).p2_alt1(slot as i32);
    client.send(ClientProt::IfButton1, &p.finish());
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

fn messages(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) -> Vec<String> {
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

fn login(engine: &mut Engine, name: &str) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    client
}

/// Redbeard Frank's start and Luthas's job, in 2011's words, through to the shipped crate.
#[test]
fn frank_and_luthas_as_2011s() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hunt_start");

        let t = talk(
            engine,
            &mut client,
            "redbeard_frank",
            3049,
            3253,
            &[
                "I'm in search of treasure.",
                "Ok, I will bring you some rum.",
            ],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        says(
            &t,
            &[
                "Arr, Matey!",
                "Arr, treasure you be after eh? Well I might be able to tell you where to find some... For a price...",
                "I'd like some rum made on Karamja Island. There's no rum like Karamja Rum!",
                "Yer a saint, although it'll take a miracle to get it off Karamja.",
                "What do you mean?",
                "The Customs office has been clampin' down on the export of spirits. You seem like a resourceful young lad, I'm sure ye'll be able to find a way to slip the stuff past them.",
                "Well I'll give it a shot.",
            ],
        );
        assert!(!t.said("Arrrh Matey!"), "289's greeting is gone");
        assert_eq!(var(engine, &client, "hunt"), 1, "started on \"Ok\"");

        let t = talk(
            engine,
            &mut client,
            "luthas",
            2938,
            3154,
            &["Could you offer me employment on your plantation?"],
        );
        says(
            &t,
            &[
                "Hello I'm Luthas, I run the banana plantation here",
                "Yes, I can sort something out. There's a crate ready to be loaded onto the ship.",
                "You wouldn't believe the demand for bananas from Wydin's shop over in Port Sarim. I think this is the third crate I've shipped him this month..",
                "If you could fill it up with bananas, I'll pay you 30 gold.",
            ],
        );
        assert!(
            !t.said("Hello I'm Luthas, I run the banana plantation here."),
            "no full stop in 2011"
        );
        assert_eq!(
            var(engine, &client, "hunt_store_employed") & 1,
            1,
            "employed on the plantation"
        );

        let t = talk(
            engine,
            &mut client,
            "luthas",
            2938,
            3154,
            &["That customs officer is annoying isn't she?"],
        );
        says(
            &t,
            &[
                "Have you completed your task yet?",
                "Well I know her pretty well. She doesn't cause me any trouble any more.",
                "Really? How interesting. Whereabouts do you send those to?",
                "There is a little shop over in Port Sarim that buys them up by the crate. I believe it is run by a man called Wydin.",
            ],
        );

        // The rum and ten bananas into the crate, then paid.
        let rum = id(engine, "obj", "karamjan_rum");
        let banana = id(engine, "obj", "banana");
        tele(engine, &mut client, 2944, 0, 3151);
        give(engine, &client, 0, "karamjan_rum", 1);
        use_on_loc(&mut client, rum, 0, BANANA_CRATE);
        let t = converse(engine, &mut client, &[], 60);
        says(&t, &["You stash the rum in the crate."]);
        assert_eq!(var(engine, &client, "crate_rum"), 1);
        for _ in 0..10 {
            give(engine, &client, 1, "banana", 1);
            use_on_loc(&mut client, banana, 1, BANANA_CRATE);
            let t = converse(engine, &mut client, &[], 60);
            says(&t, &["You pack a banana into the crate."]);
        }
        assert_eq!(var(engine, &client, "crate_bananas"), 10);
        let t = talk(
            engine,
            &mut client,
            "luthas",
            2938,
            3154,
            &["Thank you, I'll be on my way"],
        );
        says(
            &t,
            &[
                "I've filled a crate with bananas.",
                "Well done, here's your payment.",
            ],
        );
        assert_eq!(
            var(engine, &client, "crate_rum"),
            2,
            "the rum shipped to Wydin's"
        );
        assert_eq!(held(engine, &client, "coins"), 30);
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}

/// Wydin hires at the door in 2011, and the rum is in the back room's crate.
#[test]
fn wydin_hires_at_the_door() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hunt_wydin");
        set_var(engine, &client, "hunt", 1);
        set_var(engine, &client, "crate_rum", 2);
        give(engine, &client, 0, "white_apron", 1);
        // In the store, east of the back room's door (Wydin stands at 3014,3204).
        tele(engine, &mut client, 3013, 0, 3204);
        oploc1(&mut client, WYDIN_DOOR);
        let t = converse(engine, &mut client, &["Well, can I get a job here?"], 300);
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        says(
            &t,
            &[
                "Hey, you can't go in there. Only employees of the grocery store can go in.",
                "Can I get a job here?",
                "Well, you're keen, I'll give you that. Okay, I'll give you a go. Have you got your own white apron?",
                "Yes, I have one right here.",
                "Wow - you are well prepared! You're hired. Go through to the back and tidy up for me, please.",
                "You need to put your white apron on first though.",
            ],
        );
        assert_eq!(
            var(engine, &client, "hunt_store_employed") & 2,
            2,
            "employed at the store"
        );

        tele(engine, &mut client, 3010, 0, 3207);
        oploc1(&mut client, GROCERY_CRATE);
        let mut said = messages(engine, &mut client, 8);
        let t = converse(engine, &mut client, &["No."], 60);
        said.extend(t.messages.iter().cloned());
        for line in [
            "There are a lot of bananas in the crate.",
            "You find your bottle of rum in amongst the bananas.",
        ] {
            assert!(
                said.iter().any(|m| m == line),
                "2011 shows {line:?}; {said:?}"
            );
        }
        assert_eq!(held(engine, &client, "karamjan_rum"), 1, "the rum");
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}

/// The rum to Frank, Hector's chest, the gardener and the dig: 2011's casket.
#[test]
fn the_key_the_chest_and_the_casket() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hunt_end");
        set_var(engine, &client, "hunt", 1);
        give(engine, &client, 0, "karamjan_rum", 1);
        let t = talk(
            engine,
            &mut client,
            "redbeard_frank",
            3049,
            3253,
            &["Ok thanks, I'll go and get it."],
        );
        says(
            &t,
            &[
                "Arr, Matey!",
                "Have ye brought some rum for yer ol' mate Frank?",
                "Frank happily takes the rum... ... and hands you a key.",
                "With any luck his treasure will be in there.",
            ],
        );
        assert_eq!(var(engine, &client, "hunt"), 2);

        let key = id(engine, "obj", "chest_key");
        let slot = (0..28u16)
            .find(|&s| {
                let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
                engine
                    .harness_player(client.pid)
                    .unwrap()
                    .invs
                    .get(&inv)
                    .unwrap()
                    .get(s)
                    .map(|o| o.obj)
                    == Some(key)
            })
            .expect("the key in the backpack");
        tele(engine, &mut client, 3219, 1, 3395);
        use_on_loc(&mut client, key, slot, HECTOR_CHEST);
        let said = messages(engine, &mut client, 12);
        for line in [
            "You unlock the chest.",
            "All that's in the chest is a message...",
            "You take the message from the chest.",
        ] {
            assert!(
                said.iter().any(|m| m == line),
                "2011 shows {line:?}; {said:?}"
            );
        }
        assert_eq!(held(engine, &client, "pirate_message"), 1);
        set_var(engine, &client, "hunt", 3);

        // The gardener: his 2011 shout, then a dig refused while he attacks.
        let spade = id(engine, "obj", "spade");
        give(engine, &client, 0, "spade", 1);
        tele(engine, &mut client, DIG.0, 0, DIG.1);
        op_held1(&mut client, spade, 0);
        settle(engine, &mut client, 3);
        let (gardener, _, _) = npc_near(engine, "gardener", DIG.0, DIG.1);
        op_held1(&mut client, spade, 0);
        let said = messages(engine, &mut client, 4);
        assert!(
            said.iter()
                .any(|m| m == "I can't dig up anything with him attacking me!"),
            "2011 refuses the dig: {said:?}"
        );
        assert_eq!(var(engine, &client, "hunt"), 3, "no treasure yet");

        // With the gardener gone (killed, as in the recording): the casket, and what's in it.
        engine.remove_npc(gardener);
        tele(engine, &mut client, DIG.0, 0, DIG.1);
        op_held1(&mut client, spade, 0);
        let said = messages(engine, &mut client, 12);
        for line in [
            "You dig a hole in the ground...",
            "and find a little chest of treasure.",
        ] {
            assert!(
                said.iter().any(|m| m == line),
                "2011 shows {line:?}; {said:?}"
            );
        }
        assert_eq!(var(engine, &client, "hunt"), 4, "complete");
        assert_eq!(held(engine, &client, "casket_7956"), 1, "Hector's casket");
        let casket = id(engine, "obj", "casket_7956");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let cslot = (0..28u16)
            .find(|&s| {
                engine
                    .harness_player(client.pid)
                    .unwrap()
                    .invs
                    .get(&inv)
                    .unwrap()
                    .get(s)
                    .map(|o| o.obj)
                    == Some(casket)
            })
            .unwrap();
        engine.harness_close_modal(client.pid);
        op_held1(&mut client, casket, cslot);
        let said = messages(engine, &mut client, 4);
        assert!(
            said.iter()
                .any(|m| m == "You open the casket, and find One-Eyed Hector's treasure."),
            "{said:?}"
        );
        assert_eq!(held(engine, &client, "coins"), 450);
        assert_eq!(held(engine, &client, "emerald"), 1);
        assert_eq!(held(engine, &client, "gold_ring"), 1);
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}
