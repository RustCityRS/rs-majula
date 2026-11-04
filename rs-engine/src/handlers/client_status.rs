use crate::active_player::ActivePlayer;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client_prot::{
    ClickWorldMap, ClientDetailoptionsStatus, EventKeyboard, MapBuildLoading, SoundSongPreloaded,
    TransmitvarVerifyId, VideoEnd, WorldLatency,
};
use rs_vm::ScriptError;

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
    ClickWorldMap,
    ClientDetailoptionsStatus,
    EventKeyboard,
    MapBuildLoading,
    SoundSongPreloaded,
    TransmitvarVerifyId,
    VideoEnd,
    WorldLatency,
);
