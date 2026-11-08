//! Shield of Arrav at 647 (`quests/quest_blackarmgang` and the Varrock npcs): 289's quest with
//! 2011's lines and half-certificates, from the 2011-02-28 recordings (YouTube LhbN1Qi39D4 Phoenix
//! Gang, TSNR-A0LI8s Black Arm Gang) and the 2011-03-02 one (UgONltQDj38); transcripts in
//! runescape/target/refs/arrav.

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
/// Locs in cache 786's map (maps/l*.jm2): the Phoenix chest underground (l50_152 35,33) and the
/// Black Arm cupboard upstairs (l49_52 level 1, 53,57).
const PHOENIX_CHEST: (u16, u16, u16) = (2403, 3235, 9761);
const BLACKARM_CUPBOARD: (u16, u16, u16) = (2400, 3189, 3385);
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

/// King Roald on his throne (3222,3476): the tiles beside him are behind it, so he is spoken to
/// from the carpet in front.
fn talk_to_king(engine: &mut Engine, client: &mut HarnessClient) -> Conversation {
    let (nid, _, _) = npc_near(engine, "king_roald", 3222, 3476);
    tele(engine, client, 3222, 0, 3473);
    talk_to(client, nid);
    converse(engine, client, &[], 300)
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

fn login(engine: &mut Engine, name: &str) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    client
}

fn oploc2(client: &mut HarnessClient, loc: (u16, u16, u16)) {
    let (id, x, z) = loc;
    let mut p = ClientPacket::new();
    p.p2_alt1(id as i32)
        .p1_alt2(0)
        .p2_alt2(z as i32)
        .p2_alt3(x as i32);
    client.send(ClientProt::OpLoc2, &p.finish());
}

/// The backpack slot holding `name`.
fn slot_of(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let pack = engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap();
    (0..28u16)
        .find(|&s| pack.get(s).map(|i| i.obj) == Some(o))
        .unwrap_or_else(|| panic!("{name} in the backpack")) as i32
}

/// Uses the backpack's `obj` on the npc (OPNPCT) and answers what follows.
fn use_on_npc(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    npc: &str,
    x: u16,
    z: u16,
    choices: &[&str],
) -> Conversation {
    let (nid, sx, sz) = npc_near(engine, npc, x, z);
    tele(engine, client, sx, 0, sz);
    let slot = slot_of(engine, client, obj);
    let mut p = ClientPacket::new();
    p.p2_alt2(id(engine, "obj", obj) as i32)
        .p2_alt2(slot)
        .p2_alt1(nid as i32)
        .p4_alt3(BACKPACK)
        .p1_alt2(0);
    client.send(ClientProt::OpNpcT, &p.finish());
    converse(engine, client, choices, 300)
}

/// Uses the backpack's `used` on its `target` (IF_BUTTONT) and answers what follows.
fn use_on_obj(
    engine: &mut Engine,
    client: &mut HarnessClient,
    used: &str,
    target: &str,
    choices: &[&str],
) -> Conversation {
    let (us, ts) = (
        slot_of(engine, client, used),
        slot_of(engine, client, target),
    );
    let mut p = ClientPacket::new();
    p.p4(BACKPACK)
        .p2_alt1(ts)
        .p2_alt1(us)
        .p2_alt2(id(engine, "obj", used) as i32)
        .p4_alt1(BACKPACK)
        .p2(id(engine, "obj", target) as i32);
    client.send(ClientProt::IfButtonT, &p.finish());
    converse(engine, client, choices, 300)
}

/// The Phoenix Gang's side: Reldo, Baraek and Straven in 2011's words, then the chest.
#[test]
fn the_phoenix_gang_as_2011s() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "arrav_phoenix");

        let t = talk(
            engine,
            &mut client,
            "reldo_2661",
            3209,
            3495,
            &["I'm in search of a quest."],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        says(
            &t,
            &[
                "Hmmm. I don't... believe there are any here...",
                "Ah yes. I know. If you look in a book called 'The Shield of Arrav', you'll find a quest in there.",
                "I'm not sure where the book is mind you... but I'm sure it's around here somewhere.",
                "Thank you.",
            ],
        );
        assert_eq!(var(engine, &client, "phoenixgang"), 1);
        let t = talk(engine, &mut client, "reldo_2661", 3209, 3495, &[]);
        says(
            &t,
            &[
                "About that book... where is it again?",
                "I'm not sure where it is exactly... but I'm sure it's around here somewhere.",
            ],
        );

        set_var(engine, &client, "phoenixgang", 2);
        let t = talk(engine, &mut client, "reldo_2661", 3209, 3495, &[]);
        says(
            &t,
            &[
                "Ok. I've read the book. Do you know where I can find the Phoenix Gang?",
                "If I were you I would talk to Baraek, the fur trader in the market place. I've heard he has connections with the Phoenix Gang.",
            ],
        );
        assert_eq!(var(engine, &client, "phoenixgang"), 3);

        give(engine, &client, 0, "coins", 20);
        let t = talk(
            engine,
            &mut client,
            "baraek",
            3217,
            3435,
            &[
                "Can you tell me where I can find the Phoenix Gang?",
                "Okay. Have 20 gold coins.",
            ],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        says(
            &t,
            &[
                "Choose an option:",
                "Sh sh sh, not so loud! You don't want to get me in trouble!",
                "No, I don't like things like bribery.",
                "Gang. They're operating there under the name of the VTAM Corporation. Be careful. The Phoenixes ain't the types to be messed about.",
            ],
        );
        assert!(
            !t.said("Hello. I am in search of a quest."),
            "not offered in 2011"
        );
        assert_eq!(var(engine, &client, "phoenixgang"), 4);
        assert_eq!(held(engine, &client, "coins"), 0);

        let t = talk(
            engine,
            &mut client,
            "straven",
            3246,
            9780,
            &["I know who you are!", "I'd like to offer you my services."],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        says(
            &t,
            &[
                "Really.",
                "No, this is a legitimate business run by legitimate businessmen.",
                "Supposing we were this crime gang however, what would you want with us?",
                "Well, obviously I can't speak for them, but the Phoenix Gang doesn't let people join just like that.",
                "Obviously, I would have no idea about that.",
                "OBVIOUSLY I know NOTHING about the dealings of the Phoenix Gang, but I bet if SOMEBODY were to kill him and bring back his intelligence report, they would be considered loyal enough to join.",
            ],
        );
        assert_eq!(var(engine, &client, "phoenixgang"), 8);

        give(engine, &client, 0, "intel_report", 1);
        let t = talk(engine, &mut client, "straven", 3246, 9780, &[]);
        says(
            &t,
            &[
                "You hand over the report. The man reads the report.",
                "Ok! You can join the Phoenix Gang! I am Straven, one of the gang leaders.",
                "Straven hands you a key.",
            ],
        );
        assert_eq!(var(engine, &client, "phoenixgang"), 9);
        assert_eq!(held(engine, &client, "weapon_store_key"), 1);

        tele(engine, &mut client, 3235, 0, 9762);
        oploc1(&mut client, PHOENIX_CHEST);
        settle(engine, &mut client, 4);
        oploc1(&mut client, (2404, PHOENIX_CHEST.1, PHOENIX_CHEST.2));
        let t = converse(engine, &mut client, &[], 300);
        says(
            &t,
            &[
                "You search the chest.",
                "You find half of a shield, which you take.",
            ],
        );
        assert_eq!(held(engine, &client, "broken_shield"), 1);
    });
}

/// The Black Arm Gang's side: Charlie the Tramp's 2011 menus, Katrine's plan, the crossbows and the
/// cupboard.
#[test]
fn the_black_arm_gang_as_2011s() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "arrav_blackarm");

        let t = talk(
            engine,
            &mut client,
            "charlie_the_tramp",
            3208,
            3391,
            &[
                "Is there anything down this alleyway?",
                "Do you think they would let me join?",
            ],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        says(
            &t,
            &[
                "Who are you?",
                "Funny you should mention that...there is actually.",
                "The ruthless and notorious criminal gang known as the Black Arm Gang have their headquarters down there.",
                "You never know. You'll find a lady down there called Katrine. Speak to her.",
            ],
        );
        assert_eq!(var(engine, &client, "blackarmgang"), 1);

        let t = talk(
            engine,
            &mut client,
            "katrine",
            3186,
            3385,
            &[
                "I've heard you're the Black Arm Gang.",
                "It was Charlie, the tramp outside.",
                "I want to become a member of your gang.",
                "Well, you can give me a try can't you?",
                "Ok, no problem.",
            ],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        says(
            &t,
            &[
                "It's a private business. Can I help you at all?",
                "Is that guy still out there? He's getting to be a nuisance. Remind me to send someone to kill him.",
                "Our rival gang - the Phoenix Gang -  has a weapons stash a little east of here.",
                "Sounds simple enough. Any particular reason you need two of them?",
                "thieves gang in Varrock! Brilliant, eh?",
                "You'd better just keep your mouth shut about this plan, or I'll make sure it stays shut for you. Now, are you going to go get those crossbows or not?",
                "Great! You'll find the Phoenix gang's weapon stash just next to a temple, due east of here.",
            ],
        );
        assert_eq!(var(engine, &client, "blackarmgang"), 2);

        give(engine, &client, 0, "coins", 10);
        let t = talk(
            engine,
            &mut client,
            "charlie_the_tramp",
            3208,
            3391,
            &[
                "Any idea where the Phoenix gang hideout is?",
                "Ok, that sounds fair.",
            ],
        );
        assert_eq!(t.unanswered, None, "{:?}", t.texts);
        says(
            &t,
            &[
                "I think I know where the hideout is, but I can't get inside.",
                "Sure, I know everything there is to know about this city. I'd be happy to tell you for 10 gold.",
                "end of that alley. It's called the VTAM corporation.",
                "V T A M, got it.",
            ],
        );
        assert!(
            !t.said("Who are you?"),
            "not offered once Katrine wants the crossbows"
        );
        assert_eq!(held(engine, &client, "coins"), 0);

        give(engine, &client, 0, "phoenix_crossbow", 1);
        give(engine, &client, 1, "phoenix_crossbow", 1);
        let t = talk(engine, &mut client, "katrine", 3186, 3385, &[]);
        says(
            &t,
            &[
                "You give the crossbows to Katrine.",
                "Ok. You can join our gang now. Feel free to enter any of the rooms of the ganghouse.",
            ],
        );
        assert_eq!(var(engine, &client, "blackarmgang"), 3);

        tele(engine, &mut client, 3188, 1, 3386);
        oploc1(&mut client, BLACKARM_CUPBOARD);
        settle(engine, &mut client, 4);
        oploc2(
            &mut client,
            (2401, BLACKARM_CUPBOARD.1, BLACKARM_CUPBOARD.2),
        );
        let t = converse(engine, &mut client, &[], 300);
        says(&t, &["You find half of a shield, which you take."]);
        assert_eq!(held(engine, &client, "arravshield2"), 1);
    });
}

/// 2011's ending: one half to the King (sent to the curator), the half used on the curator for two
/// half-certificates, a partner's half joined to one, and the certificate to the King.
#[test]
fn half_certificates_and_the_king() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "arrav_ending");
        set_var(engine, &client, "phoenixgang", 9);
        give(engine, &client, 0, "broken_shield", 1);

        let t = talk_to_king(engine, &mut client);
        says(
            &t,
            &[
                "Greetings, your majesty.",
                "Your majesty, I have recovered the Shield Of Arrav; I would like to claim the reward.",
                "The Shield of Arrav, eh? Yes, I do recall my father, King Roald, put a reward out for that.",
                "If you get the authenticity of the shield verified by the curator at the museum and then return here with authentication, I will grant you your reward.",
            ],
        );

        let t = use_on_npc(
            engine,
            &mut client,
            "broken_shield",
            "curator_haig_halen",
            3257,
            3447,
            &[],
        );
        assert!(
            !t.said("Welcome to the museum of Varrock."),
            "a use goes straight to the shield"
        );
        says(
            &t,
            &[
                "Let me have a look at it first.",
                "The curator peers at the shield.",
                "Can I have two certificates please?",
                "The curator writes out two half-certificates.",
                "Of course, you won't actually be able to claim the reward with only half the reward certificate...",
                "Well, if you were to get me the other half of the shield, I could give you the other half of the reward certificate. It's rumoured to be in the possession of the infamous Blackarm Gang, beyond that I can't help you.",
            ],
        );
        assert_eq!(held(engine, &client, "broken_shield"), 0);
        assert_eq!(held(engine, &client, "half_certificate"), 2);

        // The partner's half, then the two halves joined.
        give(engine, &client, 5, "half_certificate_11174", 1);
        let t = use_on_obj(
            engine,
            &mut client,
            "half_certificate",
            "half_certificate_11174",
            &["Yes."],
        );
        says(&t, &["The certificate will become untradable. Continue?"]);
        assert_eq!(held(engine, &client, "certificate"), 1);
        assert_eq!(held(engine, &client, "half_certificate"), 1);
        assert_eq!(held(engine, &client, "half_certificate_11174"), 0);

        let t = talk_to_king(engine, &mut client);
        says(
            &t,
            &[
                "Your majesty, I have come to claim the reward for the return of the Shield Of Arrav.",
                "You show the certificate to the king.",
                "I never thought I would live to see the day when someone came forward to claim this reward!",
                "I heard that you found half the shield, so I will give you half of the bounty. That comes to exactly 600gp!",
                "You hand over a certificate. The king gives you 600gp.",
            ],
        );
        settle(engine, &mut client, 4);
        assert_eq!(var(engine, &client, "phoenixgang"), 10, "complete");
        assert_eq!(held(engine, &client, "coins"), 600);
        assert_eq!(held(engine, &client, "certificate"), 0);
    });
}
