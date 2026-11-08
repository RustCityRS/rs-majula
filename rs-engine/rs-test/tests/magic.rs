//! 289's magic on 2011's spellbook (interface 192, the spells at void-634's
//! ids): a spell with an op is cast with IF_BUTTON1 on its component
//! (`p4(com) p2_alt3(obj) p2_alt1(slot)`), a spell cast on an item with
//! IF_BUTTONT from the spell to the backpack slot (`p4(to_com) p2_alt1(to_slot)
//! p2_alt1(from_slot) p2_alt2(from_obj) p4_alt1(from_com) p2(to_obj)`, the spell's
//! obj and slot -1). 289's checks, runes and experience ride along.
//!
//! Telekinetic Grab is cast on a ground obj with OPOBJT (`p2_alt1(from_obj)
//! p2_alt1(obj) p2(from_slot) p4_alt2(from_com) p1_alt1(ctrl) p2_alt2(z)
//! p2_alt2(x)`, `op_obj_t.rs`). The PR's 289 refuses the finished Zamorak and
//! Guthix books (2011's unholy_book and book_of_balance), where it had listed
//! the unfinished two twice.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;
use rs_vm::ops::test::{calls_for, forget_calls};

const SPELLBOOK: i32 = 192 << 16;
const VARROCK_TELEPORT: i32 = SPELLBOOK | 40;
const BONES_TO_BANANAS: i32 = SPELLBOOK | 33;
const LVL_1_ENCHANT: i32 = SPELLBOOK | 29;
const SUPERHEAT_ITEM: i32 = SPELLBOOK | 50;
const HIGH_LEVEL_ALCHEMY: i32 = SPELLBOOK | 59;
const TELEKINETIC_GRAB: i32 = SPELLBOOK | 44;
/// `inventory:inv`.
const BACKPACK: i32 = 149 << 16;
const MAGIC: usize = 6;
const SMITHING: usize = 13;
/// Lumbridge, outside the castle.
const TILE: (u16, u16) = (3222, 3218);

fn id(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

/// A player at Lumbridge with `magic` and the backpack given, slot by slot.
fn caster(engine: &mut Engine, name: &str, magic: u16, items: &[(&str, u32)]) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    engine.harness_telejump(client.pid, TILE.0, 0, TILE.1);
    let ids: Vec<u16> = items.iter().map(|(n, _)| id(engine, n)).collect();
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.stats.base_levels[MAGIC] = magic;
    player.stats.levels[MAGIC] = magic;
    let backpack = player.invs.get_mut(&inv).expect("backpack");
    for (slot, (&obj, (_, count))) in ids.iter().zip(items).enumerate() {
        backpack.set(slot as u16, obj, *count);
    }
    engine.cycle();
    client.packets();
    forget_calls();
    client
}

fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<(u16, Vec<u8>)> {
    let mut packets = Vec::new();
    for _ in 0..n {
        engine.cycle();
        packets.extend(client.packets());
    }
    packets
}

/// Each VARC_SMALL set, `(varc, value)`: `p1_alt2 value, p2_alt1 varc`.
fn varcs(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, i32)> {
    let opcode = ServerProt::VarcSmall as u16;
    packets
        .iter()
        .filter(|(op, _)| *op == opcode)
        .map(|(_, p)| {
            (
                u16::from_le_bytes([p[1], p[2]]),
                0u8.wrapping_sub(p[0]) as i8 as i32,
            )
        })
        .collect()
}

fn cast(client: &mut HarnessClient, spell: i32) {
    let mut p = ClientPacket::new();
    p.p4(spell).p2_alt3(-1).p2_alt1(-1);
    client.send(ClientProt::IfButton1, &p.finish());
}

fn cast_on(client: &mut HarnessClient, spell: i32, slot: u16, obj: u16) {
    let mut p = ClientPacket::new();
    p.p4(BACKPACK)
        .p2_alt1(slot as i32)
        .p2_alt1(-1)
        .p2_alt2(-1)
        .p4_alt1(spell)
        .p2(obj as i32);
    client.send(ClientProt::IfButtonT, &p.finish());
}

fn count(engine: &Engine, pid: u16, obj: u16) -> u32 {
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let backpack = engine.harness_player(pid).unwrap().invs.get(&inv).unwrap();
    (0..backpack.capacity as u16)
        .filter_map(|s| backpack.get(s))
        .filter(|i| i.obj == obj)
        .map(|i| i.num)
        .sum()
}

fn xp(engine: &Engine, pid: u16, stat: usize) -> i32 {
    engine.harness_player(pid).unwrap().stats.xp[stat]
}

/// 289's Varrock Teleport: its runes and 35 experience, and the caster lands
/// within two squares of its spell row's coord (3213, 3424).
#[test]
fn varrock_teleport_takes_its_runes_and_lands_in_varrock() {
    runner::on_loaded_engine(|engine| {
        let runes = [("fire_rune", 1), ("air_rune", 3), ("law_rune", 1)];
        let mut client = caster(engine, "teleporter", 25, &runes);
        let before = xp(engine, client.pid, MAGIC);
        cast(&mut client, VARROCK_TELEPORT);
        ticks(engine, &mut client, 6);
        let player = engine.harness_player(client.pid).unwrap();
        let (x, z) = (
            player.pathing.coord.x() as i32,
            player.pathing.coord.z() as i32,
        );
        assert!(
            (x - 3213).abs() <= 2 && (z - 3424).abs() <= 2,
            "landed at {x},{z}"
        );
        for (rune, _) in runes {
            assert_eq!(count(engine, client.pid, id(engine, rune)), 0, "{rune}");
        }
        assert_eq!(xp(engine, client.pid, MAGIC) - before, 350);
    });
}

/// Below the spell's level, 289's refusal; the caster stays.
#[test]
fn a_spell_above_the_level_is_refused() {
    runner::on_loaded_engine(|engine| {
        let runes = [("fire_rune", 1), ("air_rune", 3), ("law_rune", 1)];
        let mut client = caster(engine, "lowmagic", 1, &runes);
        cast(&mut client, VARROCK_TELEPORT);
        ticks(engine, &mut client, 4);
        assert_eq!(
            calls_for("mes"),
            ["Your Magic level is not high enough for this spell."]
        );
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!((player.pathing.coord.x(), player.pathing.coord.z()), TILE);
        assert_eq!(count(engine, client.pid, id(engine, "law_rune")), 1);
    });
}

/// Without the runes, 289's message names the first one missing.
#[test]
fn a_spell_without_its_runes_is_refused() {
    runner::on_loaded_engine(|engine| {
        let mut client = caster(engine, "noruns", 25, &[("air_rune", 3)]);
        cast(&mut client, VARROCK_TELEPORT);
        ticks(engine, &mut client, 4);
        assert_eq!(
            calls_for("mes"),
            ["You do not have enough Fire Runes to cast this spell."]
        );
    });
}

/// High Level Alchemy: 289's 0.6 of the item's cost in coins, the runes and
/// 65 experience.
#[test]
fn high_alchemy_turns_an_item_to_coins() {
    runner::on_loaded_engine(|engine| {
        let items = [("nature_rune", 1), ("fire_rune", 5), ("gold_bar", 1)];
        let mut client = caster(engine, "alcher", 55, &items);
        let gold_bar = id(engine, "gold_bar");
        let cost = engine.cache.objs.get_by_id(gold_bar).unwrap().cost;
        let before = xp(engine, client.pid, MAGIC);
        cast_on(&mut client, HIGH_LEVEL_ALCHEMY, 2, gold_bar);
        let packets = ticks(engine, &mut client, 3);
        // 289 turns back to the spellbook: 2011's selected tab, varc 168, is
        // the magic tab (7).
        assert!(varcs(&packets).contains(&(168, 7)), "{:?}", varcs(&packets));
        assert_eq!(count(engine, client.pid, gold_bar), 0);
        assert_eq!(
            count(engine, client.pid, id(engine, "coins")) as i32,
            (cost * 6 / 10).max(1)
        );
        assert_eq!(count(engine, client.pid, id(engine, "nature_rune")), 0);
        assert_eq!(count(engine, client.pid, id(engine, "fire_rune")), 0);
        assert_eq!(xp(engine, client.pid, MAGIC) - before, 650);
    });
}

/// Superheat Item: iron ore to an iron bar, magic and smithing experience.
#[test]
fn superheat_smelts_iron_ore() {
    runner::on_loaded_engine(|engine| {
        let items = [("nature_rune", 1), ("fire_rune", 4), ("iron_ore", 1)];
        let mut client = caster(engine, "superheater", 43, &items);
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.stats.base_levels[SMITHING] = 15;
        player.stats.levels[SMITHING] = 15;
        let iron_ore = id(engine, "iron_ore");
        cast_on(&mut client, SUPERHEAT_ITEM, 2, iron_ore);
        ticks(engine, &mut client, 3);
        let said = calls_for("mes");
        assert_eq!(count(engine, client.pid, iron_ore), 0, "{said:?}");
        assert_eq!(count(engine, client.pid, id(engine, "iron_bar")), 1);
        assert_eq!(count(engine, client.pid, id(engine, "nature_rune")), 0);
    });
}

/// Bones to Bananas: every bone in the backpack.
#[test]
fn bones_to_bananas_turns_every_bone() {
    runner::on_loaded_engine(|engine| {
        let items = [
            ("earth_rune", 2),
            ("water_rune", 2),
            ("nature_rune", 1),
            ("bones", 1),
            ("bones", 1),
            ("bones", 1),
        ];
        let mut client = caster(engine, "bananas", 15, &items);
        cast(&mut client, BONES_TO_BANANAS);
        ticks(engine, &mut client, 3);
        let said = calls_for("mes");
        assert_eq!(
            count(engine, client.pid, id(engine, "bones")),
            0,
            "{said:?}"
        );
        assert_eq!(count(engine, client.pid, id(engine, "banana")), 3);
    });
}

/// Lvl-1 Enchant on a sapphire ring makes 289's ring of recoil.
#[test]
fn lvl_1_enchant_makes_a_ring_of_recoil() {
    runner::on_loaded_engine(|engine| {
        let items = [("water_rune", 1), ("cosmic_rune", 1), ("sapphire_ring", 1)];
        let mut client = caster(engine, "enchanter", 7, &items);
        let ring = id(engine, "sapphire_ring");
        cast_on(&mut client, LVL_1_ENCHANT, 2, ring);
        ticks(engine, &mut client, 3);
        assert_eq!(count(engine, client.pid, ring), 0);
        assert_eq!(count(engine, client.pid, id(engine, "ring_of_recoil")), 1);
    });
}

/// Telekinetic Grab on the finished Zamorak and Guthix books is refused, and
/// the books stay on the ground.
#[test]
fn telekinetic_grab_refuses_the_finished_god_books() {
    runner::on_loaded_engine(|engine| {
        for book in ["unholy_book", "book_of_balance"] {
            let runes = [("law_rune", 1), ("air_rune", 1)];
            let mut client = caster(engine, &format!("grab{}", book.len()), 99, &runes);
            let obj = id(engine, book);
            let (x, z) = (TILE.0 + 3, TILE.1);
            engine.harness_add_obj(x, 0, z, obj, 1);
            engine.cycle();
            client.packets();
            let mut p = ClientPacket::new();
            p.p2_alt1(-1)
                .p2_alt1(obj as i32)
                .p2(-1)
                .p4_alt2(TELEKINETIC_GRAB)
                .p1_alt1(0)
                .p2_alt2(z as i32)
                .p2_alt2(x as i32);
            client.send(ClientProt::OpObjT, &p.finish());
            let said: Vec<String> = HarnessClient::game_messages(&ticks(engine, &mut client, 12))
                .into_iter()
                .map(|(_, t)| t)
                .collect();
            assert!(
                said.contains(&"I can't use Telekinetic Grab on this object.".to_string()),
                "{book}: {said:?}"
            );
            assert_eq!(count(engine, client.pid, obj), 0, "{book} stays put");
        }
    });
}
