use crate::active_player::ActivePlayer;
use crate::core::{engine, engine_mut};
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

/// Handles `ClickWorldMap`, rev-647 CLICKWORLDMAP (opcode 60): a press on the
/// world map (clientcode 1400) with the tile as `level << 28 | x << 14 | z`.
/// The client sends coord 0 for a press off every map area. What a press
/// means is content's to decide: this runs `[proc,clickworldmap](coord)`
/// unprotected, as the map's buttons run, and content without the proc
/// ignores the packet.
impl ClientGameHandler for ClickWorldMap {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let iface = engine().layout.world_map;
        if !active.player.is_interface_open(iface) {
            return Err(ScriptError::Client(format!(
                "Interface is not visible: {iface}"
            )));
        }
        if self.coord >> 30 != 0 {
            return Err(ScriptError::Client(format!(
                "Invalid coord: {}",
                self.coord
            )));
        }
        if self.coord == 0 {
            return Ok(());
        }
        match engine_mut().run_script_by_name(
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
