//! Nature Spirit at 647 (`quests/quest_druidspirit`): 289's quest. Cache 786's map has every
//! loc it operates on 289's tiles; the swamp's decay hurts in 2011's life points. The rewards
//! are the 2011 wiki's and Sal's Realm's as 289's.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{Conversation, converse, settexts, talk_to};
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const BACKPACK: i32 = 149 << 16;
const DEFENCE: usize = 1;
const HITPOINTS: usize = 3;
const CRAFTING: usize = 12;
/// The worn slot of an amulet.
const NECK: u16 = 2;
/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;

/// `^druidspirit_started` .. `^druidspirit_complete`.
const STARTED: i32 = 5;
const ENTERED_SWAMP: i32 = 10;
const BLESSED: i32 = 40;
const CASTED_SPELL: i32 = 45;
const PICKED_FUNGI: i32 = 50;
const SPOKEN_FILLIMAN2: i32 = 55;
const PERFORMED_RITUAL: i32 = 60;
const ENTERED_GROTTO: i32 = 65;
const KILLED_GHAST3: i32 = 105;
const COMPLETE: i32 = 110;

/// Cache 786's map (289's tiles).
const GATE: (u16, u16) = (3444, 3458);
const GROTTO_TREE: (u16, u16) = (3439, 3338);
const GROTTO_DOOR: (u16, u16) = (3440, 3337);
const IN_THE_GROTTO: (u16, u16) = (3442, 9734);
const WAY_OUT: (u16, u16) = (3442, 9733);
const ALTAR: (u16, u16) = (3441, 9740);
const LOG: (u16, u16) = (3406, 3373);
const NATURE_STONE: (u16, u16) = (3439, 3336);
const SPIRIT_STONE: (u16, u16) = (3441, 3336);

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

fn login(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    set_var(engine, &client, "druidspirit", stage);
    client
}

fn no_errors(engine: &mut Engine) {
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "no script errors: {errors:?}");
}

fn walkable(x: u16, level: u8, z: u16) -> bool {
    !rsmod::is_flagged(x, z, level, 0x100 | 0x200000)
}

fn tele(engine: &mut Engine, client: &mut HarnessClient, (x, z): (u16, u16), level: u8) {
    assert!(
        walkable(x, level, z),
        "({x}, {z}) on level {level} is walkable"
    );
    engine.harness_telejump(client.pid, x, level, z);
    settle(engine, client, 3);
}

/// Every spawned npc of that name: its nid, tile and level.
fn npcs(engine: &Engine, name: &str) -> Vec<(u16, u16, u16, u8)> {
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
        .collect()
}

/// Talks to an npc that wanders or stands in a cell: from each walkable tile near it in turn,
/// until one reaches it.
fn talk_near(
    engine: &mut Engine,
    client: &mut HarnessClient,
    npc: &str,
    choices: &[&str],
) -> Conversation {
    let mut last = Conversation::default();
    for attempt in 0..12usize {
        let (nid, x, z, level) = npcs(engine, npc)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("{npc} is spawned"));
        let tiles: Vec<(u16, u16)> = (1i16..=2)
            .flat_map(|r| {
                [
                    (r, 0),
                    (-r, 0),
                    (0, r),
                    (0, -r),
                    (r, r),
                    (-r, -r),
                    (r, -r),
                    (-r, r),
                ]
            })
            .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
            .filter(|&(ax, az)| walkable(ax, level, az))
            .collect();
        assert!(!tiles.is_empty(), "a tile near {npc}");
        let at = tiles[attempt % tiles.len()];
        engine.harness_telejump(client.pid, at.0, level, at.1);
        settle(engine, client, 2);
        talk_to(client, nid);
        // A short look first: a talk that never starts must not idle the client out. The
        // first box's lines are kept for what the talk said.
        let mut first: Vec<String> = Vec::new();
        let mut chat: Vec<String> = Vec::new();
        for _ in 0..10 {
            engine.cycle();
            let packets = client.packets();
            first.extend(settexts(&packets).into_iter().map(|(_, t)| t));
            chat.extend(
                HarnessClient::game_messages(&packets)
                    .into_iter()
                    .map(|(_, m)| m),
            );
            if !first.is_empty() {
                break;
            }
        }
        last = Conversation::default();
        last.messages = chat;
        if !first.is_empty() {
            let more = converse(engine, client, choices, 300);
            assert!(more.unanswered.is_none(), "{:?}", more.unanswered);
            first.extend(more.texts);
            last.texts = first;
            last.messages.extend(more.messages);
            return last;
        }
    }
    panic!("{npc} was never reached: {:?}", last.messages);
}

fn id(engine: &Engine, kind: &str, name: &str) -> u16 {
    let map = match kind {
        "obj" => &engine.cache.objs.debugnames,
        "npc" => &engine.cache.npcs.debugnames,
        "loc" => &engine.cache.locs.debugnames,
        "varp" => &engine.cache.varps.debugnames,
        _ => unreachable!(),
    };
    *map.get(name).unwrap_or_else(|| panic!("no {kind} {name}"))
}

fn give(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str) {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
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

fn tells(t: &Conversation, line: &str) {
    assert!(
        t.messages.iter().any(|m| m == line),
        "289 has {line:?}; chat {:?} (texts {:?})",
        t.messages,
        t.texts
    );
}

fn coord(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

/// `OPLOC1` (`p2_alt1(z) p2_alt1(loc) p1(ctrl) p2_alt2(x)`) or `OPLOC2` (`p2_alt1(loc)
/// p1_alt2(ctrl) p2_alt2(z) p2_alt3(x)`) on a loc, taking `choices`.
fn op_loc(
    engine: &mut Engine,
    client: &mut HarnessClient,
    op: u8,
    loc: &str,
    (x, z): (u16, u16),
    choices: &[&str],
) -> Conversation {
    let loc = id(engine, "loc", loc);
    let mut p = ClientPacket::new();
    if op == 1 {
        p.p2_alt1(z as i32)
            .p2_alt1(loc as i32)
            .p1(0)
            .p2_alt2(x as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
    } else {
        p.p2_alt1(loc as i32)
            .p1_alt2(0)
            .p2_alt2(z as i32)
            .p2_alt3(x as i32);
        client.send(ClientProt::OpLoc2, &p.finish());
    }
    shown(engine, client, choices)
}

/// What an op shows: a short look first (an op that only writes to the chat must not idle the
/// client out), then the boxes, taking `choices`.
fn shown(engine: &mut Engine, client: &mut HarnessClient, choices: &[&str]) -> Conversation {
    let mut first: Vec<String> = Vec::new();
    let mut chat: Vec<String> = Vec::new();
    for _ in 0..12 {
        engine.cycle();
        let packets = client.packets();
        first.extend(settexts(&packets).into_iter().map(|(_, t)| t));
        chat.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, m)| m),
        );
        if !first.is_empty() {
            break;
        }
    }
    let mut t = Conversation {
        messages: chat,
        ..Default::default()
    };
    if !first.is_empty() {
        let more = converse(engine, client, choices, 400);
        assert!(more.unanswered.is_none(), "{:?}", more.unanswered);
        first.extend(more.texts);
        t.messages.extend(more.messages);
    }
    t.texts = first;
    t
}

fn slot_of(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let player = engine.harness_player(client.pid).unwrap();
    let inv = player.invs.get(&inv).unwrap();
    (0..28)
        .find(|&s| inv.has_at(s, o))
        .unwrap_or_else(|| panic!("{name} is held")) as i32
}

/// What the chat is told over `ticks` (an op that opens no box).
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

/// `OPLOC1` on a loc, and what the chat is told over `ticks`.
fn search(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    (x, z): (u16, u16),
    ticks: usize,
) -> Conversation {
    let loc = id(engine, "loc", loc);
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
    watch(engine, client, ticks)
}

/// Uses the backpack's `obj` on a loc (`OPLOCT`: `p2_alt2(obj) p2_alt2(slot) p2(z)
/// p4_alt2(component) p2_alt2(x) p1_alt1(ctrl) p2_alt1(loc)`), and what the chat is told over
/// `ticks`.
fn use_on(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    loc: &str,
    (x, z): (u16, u16),
    ticks: usize,
) -> Conversation {
    let slot = slot_of(engine, client, obj);
    let (obj, loc) = (id(engine, "obj", obj), id(engine, "loc", loc));
    let mut p = ClientPacket::new();
    p.p2_alt2(obj as i32)
        .p2_alt2(slot)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt2(x as i32)
        .p1_alt1(0)
        .p2_alt1(loc as i32);
    client.send(ClientProt::OpLocT, &p.finish());
    watch(engine, client, ticks)
}

/// Level 99 in every skill (its xp, in tenths) and full life: no level-up box follows a reward,
/// and a fall is survived.
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

fn xp(engine: &Engine, client: &HarnessClient, skill: usize) -> i32 {
    engine.harness_player(client.pid).unwrap().stats.xp[skill]
}

fn stage(engine: &Engine, client: &HarnessClient) -> i32 {
    var(engine, client, "druidspirit")
}

fn wear(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str) {
    let o = id(engine, "obj", name);
    let worn = *engine.cache.invs.debugnames.get("worn").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&worn)
        .unwrap()
        .set(slot, o, 1);
}

/// A plain `OPLOC1` from each tile about a loc in turn, until `done` holds.
fn plain_until(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    tile: (u16, u16),
    ticks: usize,
    done: impl Fn(&Engine, &HarnessClient, &Conversation) -> bool,
) -> Conversation {
    let mut last = Conversation::default();
    for at in about(tile, 0) {
        tele(engine, client, at, 0);
        last = search(engine, client, loc, tile, ticks);
        if done(engine, client, &last) {
            return last;
        }
    }
    panic!("{loc} at {tile:?}: {:?}", last.messages);
}

/// The walkable tiles within two of a tile on `level`, nearest first.
fn about((x, z): (u16, u16), level: u8) -> Vec<(u16, u16)> {
    let mut tiles: Vec<(i16, i16)> = (-2i16..=2)
        .flat_map(|dx| (-2i16..=2).map(move |dz| (dx, dz)))
        .filter(|&(dx, dz)| (dx, dz) != (0, 0))
        .collect();
    tiles.sort_by_key(|&(dx, dz)| dx.abs() + dz.abs());
    tiles
        .into_iter()
        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
        .filter(|&(ax, az)| walkable(ax, level, az))
        .collect()
}

/// An op on a loc from each tile about it in turn, until it is reached; then its boxes.
fn reach(
    engine: &mut Engine,
    client: &mut HarnessClient,
    op: u8,
    loc: &str,
    tile: (u16, u16),
    level: u8,
) -> Conversation {
    for at in about(tile, level) {
        tele(engine, client, at, level);
        let t = op_loc(engine, client, op, loc, tile, &[]);
        if !t.messages.iter().any(|m| m == "I can't reach that!") {
            return t;
        }
    }
    panic!("{loc} at {tile:?} on level {level} was never reached");
}

/// The backpack's `obj` on a loc from each tile about it in turn, until it is reached.
fn use_from(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    loc: &str,
    tile: (u16, u16),
    level: u8,
) -> Conversation {
    for at in about(tile, level) {
        tele(engine, client, at, level);
        use_on(engine, client, obj, loc, tile, 0);
        let t = shown(engine, client, &[]);
        if !t.messages.iter().any(|m| m == "I can't reach that!") {
            return t;
        }
    }
    panic!("{loc} at {tile:?} on level {level} was never reached with {obj}");
}

/// The gate into Mort Myre lets the player in once Drezel has asked, and the strange rock's
/// knot has Filliman's journal.
#[test]
fn the_gate_into_mort_myre_and_the_journal() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ns_gate", 0);
        seasoned(engine, &client);
        let t = reach(
            engine,
            &mut client,
            1,
            "mortmyre_metalgateclosed_l",
            GATE,
            0,
        );
        says(&t, &["There's a message attached to this gate, it reads:-"]);
        assert_eq!(stage(engine, &client), 0);

        set_var(engine, &client, "druidspirit", STARTED);
        let t = plain_until(
            engine,
            &mut client,
            "mortmyre_metalgateclosed_l",
            GATE,
            8,
            |_, _, t| {
                t.messages
                    .iter()
                    .any(|m| m == "You walk into the gloomy atmosphere of Mort Myre.")
            },
        );
        tells(&t, "You walk into the gloomy atmosphere of Mort Myre.");
        assert_eq!(stage(engine, &client), ENTERED_SWAMP);
        assert!(
            coord(engine, &client).1 < GATE.1,
            "through the gate: {:?}",
            coord(engine, &client)
        );

        // Search is the rock's second op.
        let mut found = false;
        for at in about(GROTTO_TREE, 0) {
            tele(engine, &mut client, at, 0);
            let t = op_loc(
                engine,
                &mut client,
                2,
                "grotto_druidicspirit",
                GROTTO_TREE,
                &[],
            );
            if t.said("You search the strange rock.") {
                found = true;
                break;
            }
        }
        assert!(found, "the knot is searched");
        assert_eq!(held(engine, &client, "filliman_journal"), 1);
        no_errors(engine);
    });
}

/// Blessed, the bloom spell grows fungi on the swamp's rotting logs; a fungus goes on the
/// nature stone and the used spell on the spirit stone.
#[test]
fn the_bloom_spell_and_the_stones() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ns_bloom", BLESSED);
        seasoned(engine, &client);
        give(engine, &client, 0, "bloom_spell");
        let cast = |engine: &mut Engine, client: &mut HarnessClient| {
            let slot = slot_of(engine, client, "bloom_spell");
            let spell = id(engine, "obj", "bloom_spell");
            let mut p = ClientPacket::new();
            p.p4(BACKPACK).p2_alt3(spell as i32).p2_alt1(slot);
            client.send(ClientProt::IfButton1, &p.finish());
            watch(engine, client, 6)
        };
        // Beside a log (the spell blooms the eight tiles about the caster).
        let at = about(LOG, 0).into_iter().next().expect("a tile by the log");
        tele(engine, &mut client, at, 0);
        let t = cast(engine, &mut client);
        tells(&t, "You cast the spell in the swamp.");
        assert_eq!(stage(engine, &client), CASTED_SPELL);

        let log2 = id(engine, "loc", "log_druidicspirit2");
        let mut p = ClientPacket::new();
        p.p2_alt1(log2 as i32)
            .p1_alt2(0)
            .p2_alt2(LOG.1 as i32)
            .p2_alt3(LOG.0 as i32);
        client.send(ClientProt::OpLoc2, &p.finish());
        let t = watch(engine, &mut client, 6);
        tells(&t, "You pick a mushroom from the log.");
        assert_eq!(held(engine, &client, "mortmyremushroom"), 1);
        assert_eq!(stage(engine, &client), PICKED_FUNGI);

        set_var(engine, &client, "druidspirit", SPOKEN_FILLIMAN2);
        use_from(
            engine,
            &mut client,
            "mortmyremushroom",
            "stonedisc_ds_nature",
            NATURE_STONE,
            0,
        );
        assert_eq!(
            held(engine, &client, "mortmyremushroom"),
            0,
            "the fungus is laid on the nature stone"
        );
        assert_eq!(var(engine, &client, "druidspirit_bits") & 1, 1);
        let spell = if held(engine, &client, "bloom_spell") > 0 {
            "bloom_spell"
        } else {
            "used_bloom_spell"
        };
        use_from(
            engine,
            &mut client,
            spell,
            "stonedisc_ds_spirit",
            SPIRIT_STONE,
            0,
        );
        assert_eq!(
            var(engine, &client, "druidspirit_bits") & 3,
            3,
            "the spell is laid on the spirit stone"
        );
        no_errors(engine);
    });
}

/// After the ritual the grotto's door leads inside, and the roots lead out again.
#[test]
fn the_grotto_is_entered_after_the_ritual() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ns_grotto", PERFORMED_RITUAL);
        seasoned(engine, &client);
        plain_until(
            engine,
            &mut client,
            "grotto_door_druidicspirit",
            GROTTO_DOOR,
            6,
            |e, c, _| coord(e, c) == IN_THE_GROTTO,
        );
        assert_eq!(stage(engine, &client), ENTERED_GROTTO);
        let t = search(engine, &mut client, "underground_rootwall_door", WAY_OUT, 6);
        tells(&t, "You crawl back out of the grotto.");
        assert_eq!(coord(engine, &client), (GROTTO_DOOR.0, GROTTO_DOOR.1));
        no_errors(engine);
    });
}

/// With three ghasts released, the Nature Spirit at his altar completes the quest: 2 quest
/// points, 3,000 Crafting, 2,000 Defence and 2,000 Constitution experience (2011 wiki and
/// Sal's Realm, as 289).
#[test]
fn the_nature_spirit_completes_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ns_done", KILLED_GHAST3);
        seasoned(engine, &client);
        wear(engine, &client, NECK, "amulet_of_ghostspeak");
        tele(engine, &mut client, IN_THE_GROTTO, 0);
        // The altar calls him.
        for at in about(ALTAR, 0) {
            tele(engine, &mut client, at, 0);
            search(engine, &mut client, "druidic_spirit_grotto", ALTAR, 4);
            if !npcs(engine, "filliman_tarlock_ns").is_empty() {
                break;
            }
        }
        let (crafting, defence, hitpoints) = (
            xp(engine, &client, CRAFTING),
            xp(engine, &client, DEFENCE),
            xp(engine, &client, HITPOINTS),
        );
        let t = talk_near(engine, &mut client, "filliman_tarlock_ns", &[]);
        says(
            &t,
            &["Yes, I've killed all three and their spirits have been released!"],
        );
        settle(engine, &mut client, 12);
        assert_eq!(stage(engine, &client), COMPLETE);
        assert_eq!(
            xp(engine, &client, CRAFTING) - crafting,
            30_000,
            "3,000 Crafting xp"
        );
        assert_eq!(
            xp(engine, &client, DEFENCE) - defence,
            20_000,
            "2,000 Defence xp"
        );
        assert_eq!(
            xp(engine, &client, HITPOINTS) - hitpoints,
            20_000,
            "2,000 Constitution xp"
        );
        no_errors(engine);
    });
}
