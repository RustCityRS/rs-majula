//! Rev-647 abuse reports reach the 289 handler.
//!
//! 647 reports by name (SEND_SNAPSHOT, CS2 command 5002:
//! `pjstr(offender) p1(rule - 1) p1(mute) pjstr(text)`) where 289 sent the
//! name as a base-37 hash. Under its old name, `EventTracking3`, the packet
//! fell through to the unhandled catch-all and the report was never
//! acknowledged.

#![cfg(since_647)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const RECEIVED: &str = "Thank-you, your abuse report has been received";

#[test]
fn an_abuse_report_is_acknowledged() {
    runner::on_loaded_engine(|engine| {
        assert_eq!(report(engine, 3), [(0, RECEIVED.to_string())]);
        // 289 drops a rule past its last (11) without a word.
        assert_eq!(report(engine, 12), []);
    });
}

/// The game messages a returning player gets back for one report under `rule`.
fn report(engine: &mut Engine, rule: u8) -> Vec<(i32, String)> {
    engine.harness_reset();
    let mut client = engine.harness_login_returning("reporter").expect("login");
    engine.cycle();
    client.packets();

    let mut frame = b"offender\0".to_vec();
    frame.extend([rule, 0]);
    frame.extend(b"\0");
    client.send(ClientProt::SendSnapshot, &frame);
    engine.cycle();
    HarnessClient::game_messages(&client.packets())
}
