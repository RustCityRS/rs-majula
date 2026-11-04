pub mod anticheat;
pub mod chat_setmode;
pub mod client_cheat;
pub mod close_modal;
#[cfg(any(rev = "225", since_254))]
pub mod event;
#[cfg(before_274)]
pub mod event_tracking;
// Rev-647 examine is the sixth op, answered by the server.
#[cfg(since_647)]
pub mod examine;
pub mod friendlist_add;
pub mod friendlist_del;
pub mod idk_savedesign;
pub mod idle_timer;
pub mod if_button;
// Rev-647 drag: a distinct packet from the pre-647 InvButtonD -- packed
// component hashes and both endpoints -- so it gets its own handler.
#[cfg(since_647)]
pub mod if_buttond;
// Rev-647 item-on-item and spell-on-item: one packet for what were
// `OpHeldU` and `OpHeldT`.
#[cfg(since_647)]
pub mod if_buttont;
// Rev-647 reports its display mode out-of-band; without this the gameframe
// mounts under the wrong root in resizable mode.
pub mod ignorelist_add;
pub mod ignorelist_del;
#[cfg(since_647)]
pub mod window_status;
// Rev-647 merged the OpHeld/InvButton packet families into IfButton1-10
// (handled in `if_button`); the 2004-era handlers write the pre-647
// `Option<u16>` last_com/last_slot/last_item player fields.
#[cfg(before_647)]
pub mod inv_button;
#[cfg(before_647)]
pub mod inv_buttond;
pub mod map_build_complete;
pub mod message_private;
pub mod message_public;
pub mod move_click;
pub mod no_timeout;
pub mod op_common;
#[cfg(before_647)]
pub mod opheld;
#[cfg(before_647)]
pub mod opheldt;
#[cfg(before_647)]
pub mod opheldu;
pub mod oploc;
#[cfg(before_647)]
pub mod oploct;
#[cfg(before_647)]
pub mod oplocu;
pub mod opnpc;
#[cfg(before_647)]
pub mod opnpct;
#[cfg(before_647)]
pub mod opnpcu;
pub mod opobj;
#[cfg(before_647)]
pub mod opobjt;
#[cfg(before_647)]
pub mod opobju;
pub mod opplayer;
#[cfg(before_647)]
pub mod opplayert;
#[cfg(before_647)]
pub mod opplayeru;
// Rev-647 sends one targeted packet per target kind, carrying an item *or* a
// spell as its source; the pre-647 `OpXU`/`OpXT` pairs fold into it.
#[cfg(since_647)]
pub mod op_t;
#[cfg(rev = "225")]
pub mod rebuild_get_maps;
pub mod resume_p_countdialog;
/// Rev-647: typed-string entry (`p_stringdialog` resume).
#[cfg(since_647)]
pub mod resume_p_string_dialog;
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
