use crate::active_player::ActivePlayer;
use crate::clients::client_db::DbRequest;
use crate::counters::{self, AutoBan, COUNTERS};
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_entity::StaffModLevel;
use rs_protocol::network::game::client::send_snapshot::SendSnapshot;
use rs_util::base37::is_valid_userhash;
use rs_vm::ScriptError;

/// Highest rule index the report screen offers: 0 (offensive language) to
/// 11 (real-world trading).
const MAX_REASON: u8 = 11;

/// Handles the `SendSnapshot` (report abuse) client protocol message.
///
/// A player may report once per tick. An out-of-range reason only comes
/// from a modified client, so the reporter is banned and logged out
/// (`ActivePlayer::auto_ban`). Otherwise the report is recorded via
/// [`DbRequest::Snapshot`], which, if the reporter is a moderator who ticked
/// "Mute player for 48 hours", also mutes the offender (unless they are staff
/// of the moderator's level or above). The mute reaches the offender once the
/// database confirms it (`DbResponse::Muted`), or at the next mute sync if
/// they are on another world.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
impl ClientGameHandler for SendSnapshot {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let engine = engine();

        if active.last_report == Some(engine.clock) {
            return Ok(());
        }

        if self.reason > MAX_REASON {
            active.auto_ban(AutoBan::ReportReason);
            return Ok(());
        }

        if !is_valid_userhash(self.offender as u64) {
            active.auto_ban(AutoBan::ReportOffender);
            return Ok(());
        }

        let level = active.player.staff_mod_level as u8;
        let mute_by = (self.mute && level >= StaffModLevel::PlayerModerator as u8).then_some(level);
        if let Some(tx) = &engine.db_tx {
            let coord = active.player.pathing.coord;
            let _ = tx.send(DbRequest::Snapshot {
                reporter37: active.player.uid.username37(),
                offender37: self.offender as u64,
                reason: self.reason,
                x: coord.x(),
                z: coord.z(),
                y: coord.y(),
                mute_by,
            });
        }

        active.message_game("Thank-you, your abuse report has been received");
        active.last_report = Some(engine.clock);
        counters::add(&COUNTERS.reports[self.reason as usize], 1);
        Ok(())
    }
}
