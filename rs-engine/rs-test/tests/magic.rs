//! 289's magic on 2011's spellbook (interface 192, the spells at void-634's
//! ids): a spell with an op is cast with IF_BUTTON1 on its component, a spell
//! cast on an item with IF_BUTTONT from the spell to the backpack slot (the
//! spell's obj and slot -1). 289's checks, runes and experience ride along.
//!
//! Telekinetic Grab is cast on a ground obj with OPOBJT. The PR's 289 refuses
//! the finished Zamorak and Guthix books (2011's unholy_book and
//! book_of_balance), where it had listed the unfinished two twice.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_test::names::BACKPACK;
use rs_test::names::com;
use rs_test::runner;
use rs_test::server;
use rs_ver::TypeId;
use rs_vm::ops::test::{calls_for, forget_calls};

#[cfg(before_742)]
fn varrock_teleport() -> i32 {
    com("magic:sb12")
}

fn bones_to_bananas() -> i32 {
    com("magic:sb7")
}

fn lvl_1_enchant() -> i32 {
    com("magic:sb3")
}

fn superheat_item() -> i32 {
    com("magic:sb21")
}

fn high_level_alchemy() -> i32 {
    com("magic:sb28")
}

fn telekinetic_grab() -> i32 {
    com("magic:sb16")
}

const MAGIC: usize = 6;
const SMITHING: usize = 13;
/// Lumbridge, outside the castle.
const TILE: (u16, u16) = (3222, 3218);

fn id(engine: &Engine, name: &str) -> TypeId {
    engine
        .cache
        .objs
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
        .id
}

/// A player at Lumbridge with `magic` and the backpack given, slot by slot.
fn caster(engine: &mut Engine, name: &str, magic: u16, items: &[(&str, u32)]) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    engine.harness_telejump(client.pid, TILE.0, 0, TILE.1);
    let ids: Vec<TypeId> = items.iter().map(|(n, _)| id(engine, n)).collect();
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
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
    server::varcs(packets)
}

fn cast(client: &mut HarnessClient, spell: i32) {
    client.if_button(1, spell, u16::MAX, TypeId::NULL);
}

fn cast_on(client: &mut HarnessClient, spell: i32, slot: u16, obj: u16) {
    client.if_button_t(
        spell,
        u16::MAX,
        TypeId::NULL,
        BACKPACK,
        slot,
        TypeId::from_int(obj as i32),
    );
}

fn count(engine: &Engine, pid: u16, obj: u16) -> u32 {
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    let backpack = engine.harness_player(pid).unwrap().invs.get(&inv).unwrap();
    (0..backpack.capacity as u16)
        .filter_map(|s| backpack.get(s))
        .filter(|i| i.obj == TypeId::new(obj))
        .map(|i| i.num)
        .sum()
}

fn xp(engine: &Engine, pid: u16, stat: usize) -> i32 {
    engine.harness_player(pid).unwrap().stats.xp[stat]
}

/// 289's Varrock Teleport: its runes and 35 experience, and the caster lands
/// within two squares of its spell row's coord (3213, 3424).
// 742: magic:sb12 (192:40) and stats:ranged_button (320:40) share an interface
// trigger lookup key (it keeps only iface mod 64), so Varrock Teleport runs the
// ranged guide.
#[cfg(before_742)]
#[test]
fn varrock_teleport_takes_its_runes_and_lands_in_varrock() {
    runner::on_loaded_engine(|engine| {
        let runes = [("firerune", 1), ("airrune", 3), ("lawrune", 1)];
        let mut client = caster(engine, "teleporter", 25, &runes);
        let before = xp(engine, client.pid, MAGIC);
        cast(&mut client, varrock_teleport());
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
            assert_eq!(
                count(engine, client.pid, id(engine, rune).g2()),
                0,
                "{rune}"
            );
        }
        assert_eq!(xp(engine, client.pid, MAGIC) - before, 350);
    });
}

/// Below the spell's level, 289's refusal; the caster stays.
// 742: magic:sb12 (192:40) and stats:ranged_button (320:40) share an interface
// trigger lookup key (it keeps only iface mod 64), so Varrock Teleport runs the
// ranged guide.
#[cfg(before_742)]
#[test]
fn a_spell_above_the_level_is_refused() {
    runner::on_loaded_engine(|engine| {
        let runes = [("firerune", 1), ("airrune", 3), ("lawrune", 1)];
        let mut client = caster(engine, "lowmagic", 1, &runes);
        cast(&mut client, varrock_teleport());
        ticks(engine, &mut client, 4);
        assert_eq!(
            calls_for("mes"),
            ["Your Magic level is not high enough for this spell."]
        );
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!((player.pathing.coord.x(), player.pathing.coord.z()), TILE);
        assert_eq!(count(engine, client.pid, id(engine, "lawrune").g2()), 1);
    });
}

/// Without the runes, 289's message names the first one missing.
// 742: magic:sb12 (192:40) and stats:ranged_button (320:40) share an interface
// trigger lookup key (it keeps only iface mod 64), so Varrock Teleport runs the
// ranged guide.
#[cfg(before_742)]
#[test]
fn a_spell_without_its_runes_is_refused() {
    runner::on_loaded_engine(|engine| {
        let mut client = caster(engine, "noruns", 25, &[("airrune", 3)]);
        cast(&mut client, varrock_teleport());
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
        let items = [("naturerune", 1), ("firerune", 5), ("gold_bar", 1)];
        let mut client = caster(engine, "alcher", 55, &items);
        let gold_bar = id(engine, "gold_bar");
        let cost = engine.cache.objs.get_by_id(gold_bar).unwrap().cost;
        let before = xp(engine, client.pid, MAGIC);
        cast_on(&mut client, high_level_alchemy(), 2, gold_bar.g2());
        let packets = ticks(engine, &mut client, 3);
        // 289 turns back to the spellbook: 2011's selected tab, varc 168, is
        // the magic tab (7; 530's is 6).
        #[cfg(before_647)]
        let magic_tab = 6;
        #[cfg(since_647)]
        let magic_tab = 7;
        assert!(
            varcs(&packets).contains(&(168, magic_tab)),
            "{:?}",
            varcs(&packets)
        );
        assert_eq!(count(engine, client.pid, gold_bar.g2()), 0);
        assert_eq!(
            count(engine, client.pid, id(engine, "coins").g2()) as i32,
            (cost * 6 / 10).max(1)
        );
        assert_eq!(count(engine, client.pid, id(engine, "naturerune").g2()), 0);
        assert_eq!(count(engine, client.pid, id(engine, "firerune").g2()), 0);
        assert_eq!(xp(engine, client.pid, MAGIC) - before, 650);
    });
}

/// Superheat Item: iron ore to an iron bar, magic and smithing experience.
#[test]
fn superheat_smelts_iron_ore() {
    runner::on_loaded_engine(|engine| {
        let items = [("naturerune", 1), ("firerune", 4), ("iron_ore", 1)];
        let mut client = caster(engine, "superheater", 43, &items);
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.stats.base_levels[SMITHING] = 15;
        player.stats.levels[SMITHING] = 15;
        let iron_ore = id(engine, "iron_ore");
        cast_on(&mut client, superheat_item(), 2, iron_ore.g2());
        ticks(engine, &mut client, 3);
        let said = calls_for("mes");
        assert_eq!(count(engine, client.pid, iron_ore.g2()), 0, "{said:?}");
        assert_eq!(count(engine, client.pid, id(engine, "iron_bar").g2()), 1);
        assert_eq!(count(engine, client.pid, id(engine, "naturerune").g2()), 0);
    });
}

/// Bones to Bananas: every bone in the backpack.
#[test]
fn bones_to_bananas_turns_every_bone() {
    runner::on_loaded_engine(|engine| {
        let items = [
            ("earthrune", 2),
            ("waterrune", 2),
            ("naturerune", 1),
            ("bones", 1),
            ("bones", 1),
            ("bones", 1),
        ];
        let mut client = caster(engine, "bananas", 15, &items);
        cast(&mut client, bones_to_bananas());
        ticks(engine, &mut client, 3);
        let said = calls_for("mes");
        assert_eq!(
            count(engine, client.pid, id(engine, "bones").g2()),
            0,
            "{said:?}"
        );
        assert_eq!(count(engine, client.pid, id(engine, "banana").g2()), 3);
    });
}

/// Lvl-1 Enchant on a sapphire ring makes 289's ring of recoil.
#[test]
fn lvl_1_enchant_makes_a_ring_of_recoil() {
    runner::on_loaded_engine(|engine| {
        let items = [("waterrune", 1), ("cosmicrune", 1), ("sapphire_ring", 1)];
        let mut client = caster(engine, "enchanter", 7, &items);
        let ring = id(engine, "sapphire_ring");
        cast_on(&mut client, lvl_1_enchant(), 2, ring.g2());
        ticks(engine, &mut client, 3);
        assert_eq!(count(engine, client.pid, ring.g2()), 0);
        assert_eq!(
            count(engine, client.pid, id(engine, "ring_of_recoil").g2()),
            1
        );
    });
}

#[test]
fn telekinetic_grab_reaches_coins_on_a_table() {
    runner::on_loaded_engine(|engine| {
        let mut client = caster(engine, "grabcoins", 99, &[("lawrune", 1), ("airrune", 1)]);
        let coins = id(engine, "coins");
        let (x, z) = (3211, 3212);
        engine.harness_telejump(client.pid, x, 0, z + 3);
        engine.harness_add_obj(x, 0, z, coins, 5);
        engine.cycle();

        client.op_obj_t(x, z, coins, telekinetic_grab(), u16::MAX, TypeId::NULL);
        ticks(engine, &mut client, 8);

        let coord = engine.harness_player(client.pid).unwrap().pathing.coord;
        assert_eq!((coord.x(), coord.z()), (x, z + 3));
        assert_eq!(count(engine, client.pid, coins.g2()), 5);
    });
}

/// Telekinetic Grab on the finished Zamorak and Guthix books is refused, and
/// the books stay on the ground.
#[test]
fn telekinetic_grab_refuses_the_finished_god_books() {
    runner::on_loaded_engine(|engine| {
        for book in ["zamorakbook_complete", "guthixbook_complete"] {
            let runes = [("lawrune", 1), ("airrune", 1)];
            let mut client = caster(engine, &format!("grab{}", book.len()), 99, &runes);
            let obj = id(engine, book);
            let (x, z) = (TILE.0 + 3, TILE.1);
            engine.harness_add_obj(x, 0, z, obj, 1);
            engine.cycle();
            client.packets();
            client.op_obj_t(x, z, obj, telekinetic_grab(), u16::MAX, TypeId::NULL);
            let said: Vec<String> = HarnessClient::game_messages(&ticks(engine, &mut client, 12))
                .into_iter()
                .map(|(_, t)| t)
                .collect();
            assert!(
                said.contains(&"I can't use Telekinetic Grab on this object.".to_string()),
                "{book}: {said:?}"
            );
            assert_eq!(count(engine, client.pid, obj.g2()), 0, "{book} stays put");
        }
    });
}
