//! 289's shops on 2011's shop interface.
//!
//! Trade (OPNPC3, `p1_alt3(ctrl) p2_alt2(npc)`) on Bob in Lumbridge opens
//! 2011's shop (`shop_template`, 620) and its backpack panel
//! (`shop_template_side`, 621). The client draws the stock itself
//! (clientscript 2716) from the inventory varp 118 names -- Bob's is 2011's
//! inventory 1, 289's `axeshop` -- and prices each slot from its base stock
//! (varcs 946-985). The stock's ops reach the server as IF_BUTTON on 620:25,
//! the slot being the first of six components the client draws it with; the
//! backpack's as IF_BUTTON on 621:0.
//!
//! - Opening sends the shop's varp, its stock and base stocks, its title and
//!   the backpack grid (clientscript 150).
//! - A buy costs 2011's price: the item's value at the shop's base stock, and
//!   130% of it with none left (clientscript 2720).
//! - A sale pays 289's: Bob's 60% less 2% for each one he holds over his
//!   base.
//! - Closed, the shop stops going to the client; reopened, it goes whole.

#![cfg(all(since_530, before_742))]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::server;
use rs_ver::TypeId;

const SHOP: u16 = 620;
#[cfg(since_647)]
const SIDE: u16 = 621;
const STOCK: i32 = 620 << 16 | 25;
const BACKPACK: i32 = 621 << 16;
#[cfg(since_647)]
const TITLE: i32 = 620 << 16 | 20;
#[cfg(since_647)]
const SHOP_VARP: u16 = 118;
/// The steel axe's name: 2011 renamed it.
#[cfg(all(since_530, before_647))]
const STEEL_AXE: &str = "Steel axe";
#[cfg(since_647)]
const STEEL_AXE: &str = "Steel hatchet";
#[cfg(since_647)]
const BASE_VARC: u16 = 946;
#[cfg(since_647)]
const SLOT_COMPONENTS: i32 = 6;

fn id(engine: &Engine, kind: &str, name: &str) -> TypeId {
    let id = match kind {
        "obj" => engine.cache.objs.get_by_debugname(name).map(|t| t.id),
        "npc" => engine.cache.npcs.get_by_debugname(name).map(|t| t.id),
        _ => engine.cache.invs.get_by_debugname(name).map(|t| t.id),
    };
    id.unwrap_or_else(|| panic!("{kind} {name}"))
}

fn count(engine: &Engine, pid: u16, inv: &str, obj: &str) -> u32 {
    let inv = id(engine, "inv", inv);
    let obj = id(engine, "obj", obj);
    let player = engine.harness_player(pid).unwrap();
    player
        .invs
        .get(&inv)
        .or_else(|| engine.invs.get(&inv))
        .map_or(0, |i| i.total(obj))
}

fn cost(engine: &Engine, obj: &str) -> u32 {
    let obj = id(engine, "obj", obj);
    engine.cache.objs.get_by_id(obj).unwrap().cost as u32
}

/// A player beside Bob holding `items`, his shop not yet open.
fn customer(engine: &mut Engine, name: &str, items: &[(&str, u32)]) -> (HarnessClient, u16) {
    customer_of(engine, "bob", name, items)
}

/// A player beside `keeper` holding `items`.
fn customer_of(
    engine: &mut Engine,
    keeper: &str,
    name: &str,
    items: &[(&str, u32)],
) -> (HarnessClient, u16) {
    let keeper_id = id(engine, "npc", keeper);
    let (nid, x, z) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .map(|n| (n.npc.uid.nid(), n.npc.uid.id(), n.npc.pathing.coord))
        .find(|(_, id, _)| *id == keeper_id)
        .map(|(nid, _, c)| (nid, c.x(), c.z()))
        .unwrap_or_else(|| panic!("{keeper} spawned"));
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    engine.harness_telejump(client.pid, x + 1, 0, z);
    let inv = id(engine, "inv", "inv");
    let objs: Vec<(TypeId, u32)> = items
        .iter()
        .map(|(o, n)| (id(engine, "obj", o), *n))
        .collect();
    let backpack = engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap();
    for (slot, (obj, n)) in objs.into_iter().enumerate() {
        backpack.set(slot as u16, obj, n);
    }
    engine.cycle();
    client.packets();
    (client, nid)
}

/// Trade on Bob; the packets until his shop is open.
fn trade(engine: &mut Engine, client: &mut HarnessClient, nid: u16) -> Vec<(u16, Vec<u8>)> {
    client.op_npc(3, nid);
    let mut packets = Vec::new();
    for _ in 0..10 {
        engine.cycle();
        packets.extend(client.packets());
        if engine
            .harness_player(client.pid)
            .unwrap()
            .is_interface_open(SHOP)
        {
            engine.cycle();
            packets.extend(client.packets());
            break;
        }
    }
    packets
}

fn button(client: &mut HarnessClient, prot: ClientProt, com: i32, slot: i32) {
    let op = match prot {
        ClientProt::IfButton1 => 1,
        ClientProt::IfButton2 => 2,
        _ => unreachable!(),
    };
    client.if_button(op, com, slot as u16, TypeId::NULL);
}

/// A click on the side panel's slot: there the client's slot component carries the obj
/// (interface_inv_draw_slot_big's cc_setobject), so the click names it.
fn side_button(
    engine: &Engine,
    client: &mut HarnessClient,
    prot: ClientProt,
    slot: i32,
    obj: &str,
) {
    let op = match prot {
        ClientProt::IfButton1 => 1,
        ClientProt::IfButton2 => 2,
        _ => unreachable!(),
    };
    client.if_button(op, BACKPACK, slot as u16, id(engine, "obj", obj));
}

/// The game messages over `ticks`.
fn said(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..ticks {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    said
}

/// The inventory ids UPDATE_INV_FULL sent (`p2 inv`).
fn invs_sent(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    server::invs(packets).iter().map(|i| i.inv as u16).collect()
}

#[cfg(since_647)]
#[test]
fn trade_on_bob_opens_his_axes_on_2011_s_shop() {
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = customer(engine, "axebuyer", &[]);
        let packets = trade(engine, &mut client, nid);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(player.is_interface_open(SHOP), "the shop");
        assert!(player.is_interface_open(SIDE), "its backpack panel");
        // 2011's overlay over the tabs is permanent (type 1): the client's
        // close button leaves it to the server, whose close brings the tabs
        // back. The shop itself is a modal (type 0).
        let kinds: Vec<(u16, u8)> = player.subs.open.values().copied().collect();
        assert!(
            kinds.contains(&(SIDE, 1)),
            "the panel opens as type 1: {kinds:?}"
        );
        assert!(
            kinds.contains(&(SHOP, 0)),
            "the shop opens as type 0: {kinds:?}"
        );

        let axeshop = id(engine, "inv", "axeshop");
        assert_eq!(
            axeshop,
            TypeId::new(1),
            "289's axeshop is 2011's inventory 1"
        );
        assert!(
            server::varps(&packets).contains(&(SHOP_VARP, axeshop.int())),
            "varp 118 names the shop's inventory"
        );
        assert!(
            invs_sent(&packets).contains(&axeshop.g2()),
            "the stock goes to the client"
        );
        // Bob's base stock, slot by slot.
        let bases: Vec<(u16, i32)> = server::varcs(&packets)
            .into_iter()
            .filter(|(v, _)| (BASE_VARC..BASE_VARC + 40).contains(v))
            .collect();
        assert_eq!(bases.len(), 40, "every slot's base stock");
        assert_eq!(
            bases[..8].iter().map(|(_, b)| *b).collect::<Vec<_>>(),
            [5, 10, 5, 3, 5, 2, 1, 0],
            "289's stock counts as each slot's base"
        );
        let titled = server::settexts(&packets)
            .iter()
            .any(|(com, text)| *com == TITLE && text.starts_with("Bob's Brilliant Axes."));
        assert!(titled, "289's title on 2011's");
        assert!(
            server::scripts(&packets).iter().any(|(s, _)| *s == 150),
            "the backpack grid (clientscript 150)"
        );
    });
}

#[cfg(since_647)]
#[test]
fn a_buy_costs_2011_s_price_by_the_shop_s_stock() {
    runner::on_loaded_engine(|engine| {
        let coins = 100_000;
        let (mut client, nid) = customer(engine, "axebuyer2", &[("coins", coins)]);
        trade(engine, &mut client, nid);
        // Slot 6: one mithril battleaxe, Bob's base stock of them.
        let slot6 = 6 * SLOT_COMPONENTS;
        let value = cost(engine, "mithril_battleaxe");

        button(&mut client, ClientProt::IfButton1, STOCK, slot6);
        let said_value = said(engine, &mut client, 1);
        assert_eq!(
            said_value,
            [format!("Mithril battleaxe: currently costs {value}gp.")],
            "at its base stock, its value"
        );

        button(&mut client, ClientProt::IfButton2, STOCK, slot6);
        said(engine, &mut client, 1);
        assert_eq!(count(engine, client.pid, "inv", "mithril_battleaxe"), 1);
        assert_eq!(count(engine, client.pid, "inv", "coins"), coins - value);
        assert_eq!(count(engine, client.pid, "axeshop", "mithril_battleaxe"), 0);

        button(&mut client, ClientProt::IfButton1, STOCK, slot6);
        assert_eq!(
            said(engine, &mut client, 1),
            [format!(
                "Mithril battleaxe: currently costs {}gp.",
                value * 130 / 100
            )],
            "with none left, 130%"
        );
        button(&mut client, ClientProt::IfButton2, STOCK, slot6);
        assert_eq!(
            said(engine, &mut client, 1),
            ["The shop has run out of stock."]
        );
    });
}

#[test]
fn a_sale_pays_289_s_price_and_bob_takes_only_his_stock() {
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = customer(
            engine,
            "axeseller",
            &[("steel_axe", 1), ("steel_axe", 1), ("bronze_dagger", 1)],
        );
        trade(engine, &mut client, nid);
        let value = cost(engine, "steel_axe");
        let held = count(engine, client.pid, "axeshop", "steel_axe");
        let base = 3;
        assert_eq!(held, base, "Bob holds his base of steel hatchets");

        // 289: Bob buys at 60% of the value, less 2% for each one he holds
        // over his base.
        let price = |over: u32| value * (600 - 20 * over) / 1000;
        side_button(engine, &mut client, ClientProt::IfButton1, 0, "steel_axe");
        assert_eq!(
            said(engine, &mut client, 1),
            [format!("{STEEL_AXE}: shop will buy for {}gp.", price(0))]
        );
        side_button(engine, &mut client, ClientProt::IfButton2, 0, "steel_axe");
        said(engine, &mut client, 1);
        assert_eq!(count(engine, client.pid, "inv", "coins"), price(0));
        assert_eq!(count(engine, client.pid, "axeshop", "steel_axe"), base + 1);
        side_button(engine, &mut client, ClientProt::IfButton1, 1, "steel_axe");
        assert_eq!(
            said(engine, &mut client, 1),
            [format!("{STEEL_AXE}: shop will buy for {}gp.", price(1))]
        );

        side_button(
            engine,
            &mut client,
            ClientProt::IfButton2,
            2,
            "bronze_dagger",
        );
        assert_eq!(
            said(engine, &mut client, 1),
            ["You can't sell this item to this shop."],
            "Bob's is no general store"
        );
        assert_eq!(count(engine, client.pid, "inv", "bronze_dagger"), 1);
    });
}

#[test]
fn a_closed_shop_stops_going_to_the_client_and_reopens_whole() {
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = customer(engine, "axebrowser", &[("coins", 10_000)]);
        trade(engine, &mut client, nid);
        let axeshop = id(engine, "inv", "axeshop");
        // Below his base, Bob restocks (every 100 ticks for bronze pickaxes).
        button(&mut client, ClientProt::IfButton2, STOCK, 0);
        said(engine, &mut client, 1);
        engine.harness_close_modal(client.pid);
        let mut packets = Vec::new();
        for tick in 0..110 {
            // The client's keep-alive, or 100 silent ticks log it out.
            if tick % 20 == 0 {
                client.send(ClientProt::NoTimeout, &[]);
            }
            engine.cycle();
            packets.extend(client.packets());
        }
        assert_eq!(
            count(engine, client.pid, "axeshop", "bronze_pickaxe"),
            5,
            "the shop restocked"
        );
        assert!(
            !invs_sent(&packets).contains(&axeshop.g2()),
            "a closed shop's restock is not sent"
        );

        let packets = trade(engine, &mut client, nid);
        assert!(
            invs_sent(&packets).contains(&axeshop.g2()),
            "reopened, it goes whole"
        );
    });
}

/// 289's Karamja general store (`generalshop7`, 2011's inventory 34) is kept
/// at 2011's Musa Point by Shopkeeper Kofi (npc 11674; no source found for a
/// rebuild of the store): 289's own keepers, 532 and 533, are 2011's
/// gravecreepers.
#[cfg(since_647)]
#[test]
fn kofi_keeps_289_s_karamja_general_store() {
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = customer_of(engine, "vkq2_kofi", "musabuyer", &[]);
        let packets = trade(engine, &mut client, nid);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(SHOP),
            "Kofi's Trade opens the shop"
        );
        let store = id(engine, "inv", "generalshop7");
        assert_eq!(store, TypeId::new(34));
        assert!(
            invs_sent(&packets).contains(&store.g2()),
            "289's Karamja stock"
        );
    });
}
