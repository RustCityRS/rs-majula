//! The backpack keeps its ops after a death.
//!
//! 289's death runs `~initalltabs` again, which at 647 re-opens every tab.
//! Opening a sub where one is open closes it first, and the client's close
//! drops every event the server set on that interface (`Class83.method563`),
//! so a tab's IF_SETEVENTS must follow its IF_OPENSUB. The case keeps the
//! client's event table from the packets the server sends, as the client
//! does, and kills the player.
//!
//! The death plays 289's death jingle, `death2`: 2011's jingle 90 by 647's
//! midi.pack (void-634's `death`).

#![cfg(since_530)]

use std::collections::{HashMap, HashSet};

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
#[cfg(since_647)]
use rs_test::names::BACKPACK;
use rs_test::runner;
use rs_test::server;
use rs_vm::ops::test::{calls_for, forget_calls};
use rs_vm::subject::ScriptSubject;

/// The client's open subs (parent -> interface) and server-set events
/// (component, slot), kept as `MapAreaConfig` keeps them.
#[derive(Default)]
struct ClientInterfaces {
    subs: HashMap<i32, i32>,
    events: HashSet<(i32, i32)>,
}

impl ClientInterfaces {
    fn close(&mut self, iface: i32) {
        self.events
            .retain(|(com, _)| (*com as u32 >> 16) as i32 != iface);
    }

    fn read(&mut self, packets: &[(u16, Vec<u8>)]) {
        for packet in packets {
            let one = std::slice::from_ref(packet);
            for (iface, parent) in server::subs(one) {
                if let Some(old) = self.subs.insert(parent, iface as i32) {
                    self.close(old);
                }
            }
            for parent in server::closes(one) {
                if let Some(old) = self.subs.remove(&parent) {
                    self.close(old);
                }
            }
            for (com, from, to, _) in server::events(one) {
                for slot in from..=to {
                    self.events.insert((com, slot as i32));
                }
            }
        }
    }
}

fn ticks(
    engine: &mut Engine,
    client: &mut HarnessClient,
    ui: &mut ClientInterfaces,
    n: usize,
) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..n {
        engine.cycle();
        let packets = client.packets();
        ui.read(&packets);
        said.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    said
}

#[cfg(since_647)]
#[test]
fn the_backpack_keeps_its_ops_after_a_death() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("mortal").expect("login");
        let mut ui = ClientInterfaces::default();
        ticks(engine, &mut client, &mut ui, 2);
        assert!(
            ui.events.contains(&(BACKPACK, 0)),
            "login gives the backpack its ops"
        );

        let uid = engine.harness_player(client.pid).unwrap().uid;
        engine
            .run_script_by_name(
                "[queue,player_death]",
                Some(ScriptSubject::Player(uid)),
                None,
                Some(true),
                Some(true),
                None,
            )
            .expect("death");
        let said = ticks(engine, &mut client, &mut ui, 8);
        assert!(
            said.iter().any(|m| m == "Oh dear, you are dead!"),
            "the player died; the game said {said:?}"
        );
        assert!(
            (0..28).all(|slot| ui.events.contains(&(BACKPACK, slot))),
            "every backpack slot keeps its ops after the death"
        );
    });
}

#[test]
fn a_death_plays_the_death_jingle() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("mortal2").expect("login");
        let mut ui = ClientInterfaces::default();
        ticks(engine, &mut client, &mut ui, 2);
        forget_calls();
        let uid = engine.harness_player(client.pid).unwrap().uid;
        engine
            .run_script_by_name(
                "[queue,player_death]",
                Some(ScriptSubject::Player(uid)),
                None,
                Some(true),
                Some(true),
                None,
            )
            .expect("death");
        let said = ticks(engine, &mut client, &mut ui, 8);
        assert!(
            said.iter().any(|m| m == "Oh dear, you are dead!"),
            "the player died; the game said {said:?}"
        );
        let jingles = calls_for("midi_jingle");
        assert!(
            jingles.iter().any(|a| a == "90"),
            "jingle 90 played; midi_jingle calls {jingles:?}"
        );
    });
}
