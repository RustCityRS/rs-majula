use crate::active_player::ActivePlayer;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::anticheat_cyclelogic1::AnticheatCycleLogic1;
use rs_protocol::network::game::client::anticheat_cyclelogic2::AnticheatCycleLogic2;
use rs_protocol::network::game::client::anticheat_cyclelogic3::AnticheatCycleLogic3;
use rs_protocol::network::game::client::anticheat_cyclelogic4::AnticheatCycleLogic4;
use rs_protocol::network::game::client::anticheat_cyclelogic5::AnticheatCycleLogic5;
use rs_protocol::network::game::client::anticheat_cyclelogic6::AnticheatCycleLogic6;
#[cfg(since_254)]
use rs_protocol::network::game::client::anticheat_cyclelogic7::AnticheatCycleLogic7;
use rs_protocol::network::game::client::anticheat_oplogic1::AnticheatOpLogic1;
use rs_protocol::network::game::client::anticheat_oplogic2::AnticheatOpLogic2;
use rs_protocol::network::game::client::anticheat_oplogic3::AnticheatOpLogic3;
use rs_protocol::network::game::client::anticheat_oplogic4::AnticheatOpLogic4;
use rs_protocol::network::game::client::anticheat_oplogic5::AnticheatOpLogic5;
use rs_protocol::network::game::client::anticheat_oplogic6::AnticheatOpLogic6;
use rs_protocol::network::game::client::anticheat_oplogic7::AnticheatOpLogic7;
use rs_protocol::network::game::client::anticheat_oplogic8::AnticheatOpLogic8;
use rs_protocol::network::game::client::anticheat_oplogic9::AnticheatOpLogic9;
use rs_vm::ScriptError;

/// Handles the `AnticheatCycleLogic1` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat cycle logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatCycleLogic1 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatCycleLogic2` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat cycle logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatCycleLogic2 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatCycleLogic3` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat cycle logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatCycleLogic3 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatCycleLogic4` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat cycle logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatCycleLogic4 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatCycleLogic5` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat cycle logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatCycleLogic5 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatCycleLogic6` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat cycle logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatCycleLogic6 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatOpLogic1` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat operation logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatOpLogic1 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatOpLogic2` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat operation logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatOpLogic2 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatOpLogic3` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat operation logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatOpLogic3 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatOpLogic4` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat operation logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatOpLogic4 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatOpLogic5` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat operation logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatOpLogic5 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatOpLogic6` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat operation logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatOpLogic6 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatOpLogic7` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat operation logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatOpLogic7 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatOpLogic8` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat operation logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatOpLogic8 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatOpLogic9` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat operation logic packet.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for AnticheatOpLogic9 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `AnticheatCycleLogic7` client protocol message.
///
/// No-op. The server accepts but ignores this anti-cheat cycle logic packet.
#[cfg(since_254)]
impl ClientGameHandler for AnticheatCycleLogic7 {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Shared no-op handler for all anti-cheat messages.
///
/// All anti-cheat cycle logic and operation logic messages are intentionally
/// ignored by the server. This function simply returns `Ok(())`.
///
/// # Returns
///
/// Always returns `Ok(())`.
fn handle() -> Result<(), ScriptError> {
    Ok(())
}
