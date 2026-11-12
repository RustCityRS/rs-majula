use crate::active_player::ActivePlayer;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::reflection_check_reply::ReflectionResult;
use rs_protocol::network::game::client_prot::ReflectionCheckReply;
use rs_protocol::network::game::server::reflection_checker::{ReflectionCheck, ReflectionChecker};
use rs_vm::ScriptError;
use rs_vm::engine::ScriptPlayer;
use std::collections::VecDeque;
use tracing::info;

/// The `REFLECTION_CHECKER` requests a client hasn't answered, oldest first
/// (it answers them in order), and the id the next one takes.
#[derive(Default)]
pub struct ReflectionChecks {
    next: i32,
    pending: VecDeque<(i32, Vec<ReflectionCheck>)>,
}

impl ActivePlayer {
    /// Has the client read `checks` about its own classes. Each result comes
    /// back to the player as a game message, and to the log.
    pub(crate) fn reflection_check(&mut self, checks: Vec<ReflectionCheck>) {
        let id = self.reflection.next;
        self.reflection.next = id.wrapping_add(1);
        self.write(ReflectionChecker {
            id,
            checks: &checks,
        });
        self.reflection.pending.push_back((id, checks));
    }
}

/// Handles the `ReflectionCheckReply` client protocol message.
///
/// The answer to the oldest pending `REFLECTION_CHECKER` request: its id, an
/// int or an error code for each check, and a CRC32 of both. Any other reply
/// (with none pending, for another id, failing its CRC or with other results)
/// comes from a modified client.
impl ClientGameHandler for ReflectionCheckReply {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let Some((id, checks)) = active.reflection.pending.pop_front() else {
            return Err(ScriptError::Client(
                "Reflection check reply with no check pending".to_string(),
            ));
        };
        if self.id != id {
            return Err(ScriptError::Client(format!(
                "Reflection check reply {} for check {id}",
                self.id
            )));
        }
        if !self.crc_matches() {
            return Err(ScriptError::Client(format!(
                "Reflection check reply {id} fails its crc"
            )));
        }
        let Some(results) = self.results(checks.len()) else {
            return Err(ScriptError::Client(format!(
                "Malformed reflection check reply {id}"
            )));
        };

        for (check, result) in checks.iter().zip(results) {
            let line = match result {
                ReflectionResult::Value(value) => format!("{}: {value}", describe(check)),
                ReflectionResult::Error(code) => format!("{}: error {code}", describe(check)),
            };
            info!("Reflection check on {}: {line}", active.uid().username());
            active.message_game(&line);
        }
        Ok(())
    }
}

fn describe(check: &ReflectionCheck) -> String {
    match check {
        ReflectionCheck::FieldValue { class, field } => format!("{class}.{field}"),
        ReflectionCheck::FieldModifiers { class, field } => format!("{class}.{field} modifiers"),
        ReflectionCheck::MethodModifiers {
            class,
            method,
            params,
        } => format!("{class}.{method}({}) modifiers", params.join(", ")),
    }
}
