use crate::active_player::ActivePlayer;
use crate::clients::client_ether::EtherOutbound;
use crate::core::engine;
use crate::handlers::ClientGameHandler;
#[cfg(since_647)]
use rs_entity::player::CHAT_MODE_GUEST_CLAN;
use rs_protocol::network::game::client_prot::ChatSetMode;
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::SetChatfiltersettings;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptPlayer;

/// Handles the `ChatSetMode` client protocol message.
///
/// Before rev 647 this packet carries the public, private and trade chat
/// filters; see [`set_chat_filters`].
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`set_chat_filters`]
#[cfg(before_647)]
impl ClientGameHandler for ChatSetMode {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        set_chat_filters(active, self.public, self.private, self.trade)
    }
}

/// Handles the rev-647 `ChatSetMode` client protocol message: where the
/// player's typed chat goes from now on (`CHAT_MODE_*`). The client sends it
/// from CS2 op 5006 when the chatbox switches, so the next `MessagePublic` is
/// routed by it; a mode past the guest clan chat comes from a modified client.
///
/// The privacy filters this packet carried before 647 have their own packet,
/// [`SetChatfiltersettings`].
#[cfg(since_647)]
impl ClientGameHandler for ChatSetMode {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if self.mode <= CHAT_MODE_GUEST_CLAN {
            active.player.chat_mode = self.mode;
        }
        Ok(())
    }
}

/// Handles the rev-647 `SetChatfiltersettings` client protocol message: the
/// public, private and trade filters, which before 647 rode in
/// [`ChatSetMode`]. The client sends it from CS2 op 5001.
#[cfg(since_647)]
impl ClientGameHandler for SetChatfiltersettings {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        set_chat_filters(active, self.public, self.private, self.trade)
    }
}

/// Updates the player's chat filter settings for public, private, and
/// trade/duel chat. A value out of range drops the packet.
///
/// After updating the player's settings, sends a chat filter update packet
/// back to the client and, if an ether (cross-world) connection is active,
/// broadcasts the private chat mode change so other worlds can update friend
/// list online status.
///
/// # Side Effects
///
/// * Updates `player.chat_filters`.
/// * Sends a `chat_filter_settings` packet to the client.
/// * Sends an `EtherOutbound::ChatModeUpdate` message to the ether service.
///
/// # Call Stack
///
/// **Calls:** `ActivePlayer::chat_filter_settings`, `EtherOutbound::ChatModeUpdate`
fn set_chat_filters(
    active: &mut ActivePlayer,
    public: u8,
    private: u8,
    trade: u8,
) -> Result<(), ScriptError> {
    if !active.player.set_chat_filters(public, private, trade) {
        return Ok(());
    }
    active.chat_filter_settings();
    engine().send_ether(EtherOutbound::ChatModeUpdate {
        user37: active.uid().username37(),
        private_mode: private,
    });
    Ok(())
}
