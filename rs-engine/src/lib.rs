extern crate alloc;

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
#[cfg(all(feature = "harness", since_647))]
pub mod harness;
mod info;
mod map;
mod pathfinding;
mod phases;
#[cfg(since_647)]
mod region;
mod save;
mod script;
pub mod support;
pub mod watch;

pub use self::core::{CycleResult, Engine, LoginRequest, TickStats, init_collision, with_engine};
pub use clients::client_db::{
    DbClientConfig, DbRequest, DbResponse, LoadResult, ModOutcome, SaveOutcome, SyncOutcome,
    TAKEOVER_AFTER_SECS, db_client_task, profile_from_save,
};
pub use clients::client_ether::{EtherInbound, EtherOutbound, ether_client_task};
pub use clients::client_game::{ClientHandle, ClientIO, MAX_UNSENT, create_io};
pub use clients::client_replay::replay_task;
pub use pathfinding::PathStats;
pub use rs_db::DbConfig;
pub use rs_entity::build::{MAX_NPCS, MAX_PLAYERS};
pub use save::PlayerProfile;
