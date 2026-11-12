//! 2011's reflection checker: `::reflect` has a maintainer's client read one
//! of its own fields, or a member's modifiers (REFLECTION_CHECKER), and the
//! client's REFLECTION_CHECK_REPLY comes back as a game message.
//!
//! The reply is `p4(id)`, a result per check (`p1(0) p4(int)`, or a negative
//! error code), then a CRC32 of both. One with no check pending, for another
//! id, failing its CRC or with other results comes from a modified client and
//! says nothing; it answers the check it claimed to.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::player::StaffModLevel;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
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

/// The REFLECTION_CHECKER a cheat has the server send: its id and body.
fn request(engine: &mut Engine, client: &mut HarnessClient, cheat: &str) -> (i32, Vec<u8>) {
    let mut p = ClientPacket::new();
    p.p1(0).p1(0).pjstr(cheat);
    client.send(ClientProt::ClientCheat, &p.finish());
    engine.cycle();
    let mut checks: Vec<_> = client
        .packets()
        .into_iter()
        .filter(|(op, _)| *op == ServerProt::ReflectionChecker as u16)
        .map(|(_, body)| body)
        .collect();
    assert_eq!(checks.len(), 1, "{cheat} sends one request");
    let body = checks.remove(0);
    let id =
        (body[1] as i32) << 24 | (body[2] as i32) << 16 | (body[3] as i32) << 8 | body[4] as i32;
    (id, body)
}

/// The CRC32 the client's `Packet.addcrc` appends (`Statics1.crc`).
fn crc32(bytes: &[u8]) -> i32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc as i32
}

/// The game messages a reply for check `id` with `results` brings, its CRC
/// off by one unless `crc_ok`.
fn reply(
    engine: &mut Engine,
    client: &mut HarnessClient,
    id: i32,
    results: &[u8],
    crc_ok: bool,
) -> Vec<String> {
    let mut p = ClientPacket::new();
    p.p4(id);
    let mut payload = p.finish();
    payload.extend_from_slice(results);
    let mut crc = ClientPacket::new();
    crc.p4(crc32(&payload).wrapping_add(!crc_ok as i32));
    payload.extend(crc.finish());
    client.send(ClientProt::ReflectionCheckReply, &payload);
    engine.cycle();
    HarnessClient::game_messages(&client.packets())
        .into_iter()
        .map(|(_, text)| text)
        .filter(|text| text.starts_with("Statics1."))
        .collect()
}

#[test]
fn a_reflection_check_comes_back_as_a_game_message() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = developer(engine, "reflection");
        let field = "reflect field Statics1 staff_modlevel";
        let three = [0, 0, 0, 0, 3];

        let said = reply(engine, &mut client, 0, &three, true);
        assert!(said.is_empty(), "no check pending: {said:?}");

        let (id, body) = request(engine, &mut client, field);
        let mut expected = ClientPacket::new();
        expected
            .p1(1)
            .p4(id)
            .p1(0)
            .pjstr("Statics1")
            .pjstr("staff_modlevel");
        assert_eq!(
            body,
            expected.finish(),
            "one field check, in the case typed"
        );
        let said = reply(engine, &mut client, id.wrapping_add(1), &three, true);
        assert!(said.is_empty(), "another id: {said:?}");

        for (results, crc_ok) in [
            (&three[..], false),
            (&[0, 0, 0][..], true),
            (&[1][..], true),
        ] {
            let (id, _) = request(engine, &mut client, field);
            let said = reply(engine, &mut client, id, results, crc_ok);
            assert!(
                said.is_empty(),
                "{results:?} with crc ok {crc_ok}: {said:?}"
            );
        }

        let (id, _) = request(engine, &mut client, field);
        let said = reply(engine, &mut client, id, &three, true);
        assert_eq!(said, ["Statics1.staff_modlevel: 3"]);

        let (id, body) = request(
            engine,
            &mut client,
            "reflect methodmods Statics1 crc [B int int",
        );
        assert_eq!(body[5], 4, "a method modifiers check");
        let said = reply(engine, &mut client, id, &[-20i8 as u8], true);
        assert_eq!(said, ["Statics1.crc([B, int, int) modifiers: error -20"]);
    });
}
