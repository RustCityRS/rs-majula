use crate::active_player::ActivePlayer;
use crate::clients::client_ether::EtherOutbound;
use crate::core::engine;
use crate::handlers::ClientGameHandler;
use rs_entity::{ChatSettingsPrivate, ChatSettingsPublic, ChatSettingsTradeDuel};
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
/// player's typed chat goes from now on, public chat or their clan chat
/// (`CHAT_MODE_*`). The client sends it from CS2 op 5006 when the chatbox
/// switches, so the next `MessagePublic` is routed by it.
///
/// The privacy filters this packet carried before 647 have their own packet,
/// [`SetChatfiltersettings`].
#[cfg(since_647)]
impl ClientGameHandler for ChatSetMode {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        active.player.chat_mode = self.mode;
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
/// trade/duel chat. Each setting is decoded from its numeric value into the
/// corresponding enum variant. If a value is unrecognized, the handler returns
/// early without modifying any setting.
///
/// After updating the player's settings, sends a chat filter update packet
/// back to the client and, if an ether (cross-world) connection is active,
/// broadcasts the private chat mode change so other worlds can update friend
/// list online status.
///
/// # Side Effects
///
/// * Updates `player.public`, `player.private`, and `player.trade` settings.
/// * Sends a `chat_filter_settings` packet to the client.
/// * Sends an `EtherOutbound::ChatModeUpdate` message to the ether service.
///
/// # Call Stack
///
/// **Calls:** `ActivePlayer::chat_filter_settings`, `EtherOutbound::ChatModeUpdate`
fn set_chat_filters(
    active: &mut ActivePlayer,
    public_id: u8,
    private_id: u8,
    trade_id: u8,
) -> Result<(), ScriptError> {
    let public = match public_id {
        0 => ChatSettingsPublic::On,
        1 => ChatSettingsPublic::Friends,
        2 => ChatSettingsPublic::Off,
        3 => ChatSettingsPublic::Hide,
        _ => return Ok(()),
    };
    let private = match private_id {
        0 => ChatSettingsPrivate::On,
        1 => ChatSettingsPrivate::Friends,
        2 => ChatSettingsPrivate::Off,
        _ => return Ok(()),
    };
    let trade = match trade_id {
        0 => ChatSettingsTradeDuel::On,
        1 => ChatSettingsTradeDuel::Friends,
        2 => ChatSettingsTradeDuel::Off,
        _ => return Ok(()),
    };

    active.player.public = public;
    active.player.private = private;
    active.player.trade = trade;
    active.chat_filter_settings(public_id, private_id, trade_id);

    let Some(tx) = &engine().ether_tx else {
        return Ok(());
    };

    let _ = tx.send(EtherOutbound::ChatModeUpdate {
        user37: active.uid().username37(),
        private_mode: private_id,
    });

    Ok(())
}
