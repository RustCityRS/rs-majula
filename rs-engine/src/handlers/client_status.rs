use crate::active_player::ActivePlayer;
use crate::engine::engine_mut;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client_prot::{
    ClickWorldMap, ClientDetailoptionsStatus, EventKeyboard, MapBuildLoading, SoundSongPreloaded,
    TransmitvarVerifyId, VideoEnd, WorldLatency,
};
use rs_vm::ScriptError;
use rs_vm::state::ScriptArgument;
use rs_vm::subject::ScriptSubject;

macro_rules! client_status {
    ($($packet:ty),* $(,)?) => {$(
        impl ClientGameHandler for $packet {
            fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
                Ok(())
            }
        }
    )*};
}

client_status!(
    ClientDetailoptionsStatus,
    EventKeyboard,
    MapBuildLoading,
    SoundSongPreloaded,
    TransmitvarVerifyId,
    VideoEnd,
    WorldLatency,
);

/// Handles `ClickWorldMap` — rev-647 CLICKWORLDMAP (opcode 60).
///
/// The client sends it on every press on the world map (interface 755's map,
/// clientcode 1400; client.java:436-457), with the tile under the pointer
/// packed as a coord: `level << 28 | x << 14 | z`. What a press means is
/// content's to decide, so the engine only hands the tile on: it runs
/// `[proc,clickworldmap](coord)` by name, as `WindowStatus` runs
/// `[proc,switch_window_mode]`. There is no trigger type for it in
/// `ServerTriggerType`.
///
/// # Side Effects
///
/// * Runs `[proc,clickworldmap]` with the pressed tile, unprotected. Content
///   without that proc ignores the packet, as before.
impl ClientGameHandler for ClickWorldMap {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        match engine_mut().run_script_by_name(
            // Compiled scripts are named by their header.
            "[proc,clickworldmap]",
            Some(ScriptSubject::Player(active.player.uid)),
            None,
            None,
            None,
            Some(vec![ScriptArgument::Int(self.coord as i32)]),
        ) {
            Err(ScriptError::ScriptNotFoundName(_)) => Ok(()),
            other => other,
        }
    }
}
