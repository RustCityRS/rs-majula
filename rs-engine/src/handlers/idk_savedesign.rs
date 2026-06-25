use crate::active_player::ActivePlayer;
use crate::engine::cache;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::idk_savedesign::IdkSaveDesign;
use rs_vm::ScriptError;

/// Valid body-colour palettes for each of the 5 design slots
/// (hair, torso, legs, feet, skin). The client transmits an *index* into these
/// palettes; the server only validates the index is within range. Ported from
/// `Player.DESIGN_BODY_COLORS`.
const DESIGN_BODY_COLORS: [&[u16]; 5] = [
    &[
        6798, 107, 10283, 16, 4797, 7744, 5799, 4634, 33697, 22433, 2983, 54193,
    ],
    &[
        8741, 12, 64030, 43162, 7735, 8404, 1701, 38430, 24094, 10153, 56621, 4783, 1341, 16578,
        35003, 25239,
    ],
    &[
        25238, 8742, 12, 64030, 43162, 7735, 8404, 1701, 38430, 24094, 10153, 56621, 4783, 1341,
        16578, 35003,
    ],
    &[4626, 11146, 6439, 12, 4758, 10270],
    &[4550, 4537, 5681, 5673, 5790, 6806, 8076, 4574],
];

const WOMAN_JAW: u8 = 8;

/// Handles the `IdkSaveDesign` client protocol message.
///
/// Sent when the player confirms a character design (gender, identity kits, and
/// body colors) from the appearance interface. Validates the player is permitted
/// to design (`allow_design`), that the gender is 0 or 1, that every identity kit
/// matches the expected body type for its slot and is not disabled, and that each
/// color index is within its palette. The female jaw slot may be empty (`-1`).
///
/// On success the player's gender, body kits, and colors are updated and the
/// appearance is rebuilt from the worn-equipment inventory.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success, or if any validation check fails (the design is silently
///   rejected, matching the client's expectation of no response).
///
/// # Side Effects
///
/// * On success: updates `gender`, `body`, and `colours`, then calls `buildappearance`.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `ActivePlayer::buildappearance`
impl ClientGameHandler for IdkSaveDesign {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if !active.player.allow_design {
            // normal: player is not on a design interface
            return Ok(());
        }

        if self.gender > 1 {
            // bad client: invalid gender
            return Ok(());
        }

        // Identity-kit values arrive as signed bytes (-1 == empty slot).
        for i in 0..7 {
            let expected = if self.gender == 1 {
                i as u8 + 7
            } else {
                i as u8
            };
            let kit = self.idkit[i] as i8 as i32;

            if expected == WOMAN_JAW && kit == -1 {
                // female jaw is an exception: it is allowed to be empty
                continue;
            }

            let idk = if kit >= 0 {
                cache().idks.get_by_id(kit as u16)
            } else {
                None
            };
            let Some(idk) = idk else {
                // bad client: identity kit does not exist
                return Ok(());
            };
            if idk.disable || idk.body_type as u8 != expected {
                // bad client: kit is disabled or does not match this slot
                return Ok(());
            }
        }

        for (i, colour) in DESIGN_BODY_COLORS.iter().enumerate() {
            if self.colour[i] as usize >= colour.len() {
                // bad client: colour index is out of range
                return Ok(());
            }
        }

        active.player.gender = self.gender;
        for i in 0..7 {
            active.player.body[i] = self.idkit[i] as i8 as i32;
        }
        for i in 0..5 {
            active.player.colours[i] = self.colour[i];
        }

        let worn = cache()
            .invs
            .get_by_debugname("worn")
            .map_or(94, |inv| inv.id);
        active.buildappearance(worn);

        Ok(())
    }
}
