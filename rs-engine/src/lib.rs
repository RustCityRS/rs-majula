extern crate alloc;

#[cfg(since_647)]
use rs_db::LobbyWorld;
#[cfg(since_647)]
use rs_protocol::world_list::{WorldEntry, WorldList};

mod active_npc;
mod active_player;
mod build;
mod clients;
pub mod console;
pub mod controls;
mod core;
pub mod counters;
mod handlers;
/// Headless harness for `rs-test`; see [`harness::HarnessClient`].
#[cfg(all(feature = "harness", since_530))]
pub mod harness;
mod info;
mod map;
mod pathfinding;
mod phases;
#[cfg(since_530)]
mod region;
mod save;
mod script;
#[cfg(since_647)]
pub mod social;
pub mod support;
pub mod watch;

pub use self::core::{CycleResult, Engine, LoginRequest, TickStats, init_collision, with_engine};
pub use active_player::mute_deadline;
#[cfg(since_530)]
pub use active_player::script_params;
pub use clients::client_db::{
    AuthResult, DB_FENCE, DbClientConfig, DbRequest, DbResponse, LoadResult, MAX_AUTH_RUNNING,
    MAX_AUTH_WAITING, ModOutcome, SaveOutcome, SyncOutcome, TAKEOVER_AFTER_SECS, authenticate,
    db_client_task, prepare_schema, profile_from_save,
};
pub use clients::client_ether::{
    EtherInbound, EtherOutbound, EtherRegistration, EtherSession, ether_client_task, finish_resync,
    max_friends_cap,
};
pub use clients::client_game::{ClientHandle, ClientIO, MAX_UNSENT, create_io};
pub use clients::client_replay::replay_task;
#[cfg(since_530)]
pub use handlers::message_public::{MAX_CHAT_CHARS, chat_bytes, chat_text};
pub use pathfinding::PathStats;
pub use phases::login::MAX_PLAYING_PER_IP;
pub use rs_db::DbConfig;
pub use rs_entity::build::{MAX_NPCS, MAX_PLAYERS};
#[cfg(since_530)]
pub use rs_entity::subs::SubTable;
pub use save::PlayerProfile;

#[cfg(since_647)]
pub fn world_list(worlds: Vec<LobbyWorld>) -> WorldList {
    let worlds: Vec<_> = worlds
        .into_iter()
        .map(|w| WorldEntry {
            id: w.id as u16,
            address: w.address,
            members: w.members,
            players: w.players as u16,
            location: w.location,
            country: w.country as u16,
            activity: w.activity,
        })
        .collect();
    WorldList::new(&worlds)
}
