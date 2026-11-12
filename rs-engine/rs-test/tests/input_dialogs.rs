//! 2011's name, obj, colour and clan-forum quick find code dialogs: the
//! client's RESUME_P_NAMEDIALOG, _OBJDIALOG, _HSLDIALOG or
//! _CLANFORUMQFCDIALOG resumes the script waiting in `p_namedialog`,
//! `p_objdialog`, `p_hsldialog` or `p_clanforumqfcdialog` (the dev cheats in
//! `_test/scripts/cheats/cheat_dialogs.rs2`, which say what came back).
//!
//! An answer the 2011 client cannot send resumes nothing and leaves the
//! dialog waiting, as does any answer with no dialog waiting:
//! - a name is at most 12 printable characters (`meslayer_mode8`);
//! - an obj is one the stockmarket search lists, so no cert;
//! - a code is at most 13 characters (`clan_qfc_input`). One that spells no
//!   base-36 long still resumes, with no code.
//!
//! The name and code are `jstr`s; the obj and colour `p2`.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::player::StaffModLevel;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

fn developer(engine: &mut Engine, name: &str) -> HarnessClient {
    let mut client = engine.harness_login_returning(name).expect("login");
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .staff_mod_level = StaffModLevel::Maintainer;
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    client
}

/// Opens a dev cheat's dialog.
fn open(engine: &mut Engine, client: &mut HarnessClient, cheat: &str) {
    let mut p = ClientPacket::new();
    p.p1(0).p1(0).pjstr(cheat);
    client.send(ClientProt::ClientCheat, &p.finish());
    engine.cycle();
    client.packets();
}

/// The game messages the answer's cycle brings that start with `prefix`.
fn answer(
    engine: &mut Engine,
    client: &mut HarnessClient,
    prot: ClientProt,
    p: &ClientPacket,
    prefix: &str,
) -> Vec<String> {
    client.send(prot, &p.finish());
    engine.cycle();
    HarnessClient::game_messages(&client.packets())
        .into_iter()
        .map(|(_, text)| text)
        .filter(|text| text.starts_with(prefix))
        .collect()
}

fn text(text: &str) -> ClientPacket {
    let mut p = ClientPacket::new();
    p.pjstr(text);
    p
}

fn word(value: u16) -> ClientPacket {
    let mut p = ClientPacket::new();
    p.p2(value as i32);
    p
}

#[test]
fn a_name_dialog_takes_twelve_printable_characters() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = developer(engine, "namedialog");
        let name = ClientProt::ResumePNameDialog;

        let waiting = answer(engine, &mut client, name, &text("Zezima"), "name:");
        assert!(waiting.is_empty(), "no dialog waits: {waiting:?}");

        open(engine, &mut client, "~namedialog");
        for bad in ["abcdefghijklm", "tab\tname"] {
            let resumed = answer(engine, &mut client, name, &text(bad), "name:");
            assert!(resumed.is_empty(), "{bad:?} resumed {resumed:?}");
        }
        let resumed = answer(engine, &mut client, name, &text("Iron Man 12!"), "name:");
        assert_eq!(resumed, ["name: Iron Man 12!"]);

        let again = answer(engine, &mut client, name, &text("Zezima"), "name:");
        assert!(again.is_empty(), "the dialog was answered: {again:?}");
    });
}

#[test]
fn an_obj_dialog_takes_an_obj_the_stockmarket_search_lists() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let objs = &engine.cache.objs;
        let listed = (0..objs.count() as u16)
            .filter_map(|id| objs.get_by_id(id))
            .find(|obj| {
                obj.stockmarket
                    && obj.certtemplate.is_none()
                    && obj.lenttemplate.is_none()
                    && obj.boughttemplate.is_none()
                    && obj.certlink.is_some()
                    && obj.name.is_some()
            })
            .expect("a stockmarket obj with a cert");
        let (id, cert) = (listed.id, listed.certlink.unwrap());
        let label = format!("obj: {}", listed.name.as_deref().unwrap());
        let unlisted = (0..objs.count() as u16)
            .find(|&id| objs.get_by_id(id).is_some_and(|obj| !obj.stockmarket))
            .expect("an obj off the stockmarket");
        let mut client = developer(engine, "objdialog");
        let obj = ClientProt::ResumePObjDialog;

        open(engine, &mut client, "~objdialog");
        for bad in [cert, unlisted, u16::MAX] {
            let resumed = answer(engine, &mut client, obj, &word(bad), "obj:");
            assert!(resumed.is_empty(), "obj {bad} resumed {resumed:?}");
        }
        let resumed = answer(engine, &mut client, obj, &word(id), "obj:");
        assert_eq!(resumed, [label]);
    });
}

#[test]
fn a_colour_dialog_takes_any_hsl() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = developer(engine, "hsldialog");
        let hsl = ClientProt::ResumePHslDialog;

        let waiting = answer(engine, &mut client, hsl, &word(0), "hsl:");
        assert!(waiting.is_empty(), "no dialog waits: {waiting:?}");

        open(engine, &mut client, "~hsldialog");
        let resumed = answer(engine, &mut client, hsl, &word(0xFFFF), "hsl:");
        assert_eq!(resumed, ["hsl: 65535"]);
    });
}

#[test]
fn a_quick_find_code_dialog_reads_base_36() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = developer(engine, "qfcdialog");
        let qfc = ClientProt::ResumePClanforumqfcDialog;

        open(engine, &mut client, "~qfcdialog");
        let resumed = answer(engine, &mut client, qfc, &text("12345678901234"), "qfc:");
        assert!(resumed.is_empty(), "14 characters resumed {resumed:?}");
        let resumed = answer(engine, &mut client, qfc, &text("00ab12"), "qfc:");
        assert_eq!(resumed, ["qfc: AB12"]);

        for none in ["", "no code", "zzzzzzzzzzzzz"] {
            open(engine, &mut client, "~qfcdialog");
            let resumed = answer(engine, &mut client, qfc, &text(none), "qfc:");
            assert_eq!(resumed, ["qfc: "], "{none:?}");
        }
    });
}
