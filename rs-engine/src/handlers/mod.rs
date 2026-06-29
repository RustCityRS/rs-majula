pub mod anticheat;
pub mod chat_setmode;
pub mod client_cheat;
pub mod close_modal;
#[cfg(any(rev = "225", since_254))]
pub mod event;
#[cfg(before_274)]
pub mod event_tracking;
pub mod friendlist_add;
pub mod friendlist_del;
pub mod idk_savedesign;
pub mod idle_timer;
pub mod if_button;
pub mod ignorelist_add;
pub mod ignorelist_del;
pub mod inv_button;
pub mod inv_buttond;
pub mod map_build_complete;
pub mod message_private;
pub mod message_public;
pub mod move_click;
pub mod no_timeout;
pub mod opheld;
pub mod opheldt;
pub mod opheldu;
pub mod oploc;
pub mod oploct;
pub mod oplocu;
pub mod opnpc;
pub mod opnpct;
pub mod opnpcu;
pub mod opobj;
pub mod opobjt;
pub mod opobju;
pub mod opplayer;
pub mod opplayert;
pub mod opplayeru;
#[cfg(rev = "225")]
pub mod rebuild_get_maps;
pub mod resume_p_countdialog;
pub mod resume_pause_button;
pub mod send_snapshot;
pub mod tut_clickside;

use crate::active_player::ActivePlayer;
use rs_vm::ScriptError;

/// Trait implemented by every client-to-server game protocol message.
///
/// Each implementor corresponds to a decoded client packet and contains the
/// logic that the engine executes when that packet is received.
///
/// # Implementors
///
/// Every struct under the `rs_protocol::network::game::client` module tree
/// implements this trait (e.g. `OpLoc1`, `MessagePublic`, `ClientCheat`).
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` after the raw packet bytes
/// have been decoded into the concrete message struct.
pub trait ClientGameHandler {
    /// Processes the decoded client protocol message for the given player.
    ///
    /// # Arguments
    ///
    /// * `self` - The decoded client message, consumed by this call.
    /// * `active` - The active player whose client sent this message.
    ///
    /// # Returns
    ///
    /// * `Ok(())` on success.
    /// * `Err(ScriptError)` if validation fails or a script error occurs.
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError>;
}
