extern crate alloc;
extern crate core;

mod active_npc;
mod active_player;
mod build;
mod clients;
pub mod controls;
pub mod counters;
pub mod edit;
mod engine;
mod game_map;
mod handlers;
mod info;
mod phases;
mod player_save;
pub mod support;
pub mod watch;

pub use clients::client_db::{
    DbClientConfig, DbRequest, DbResponse, LoadResult, ModOutcome, SaveOutcome, SyncOutcome,
    TAKEOVER_AFTER_SECS, db_client_task, profile_from_save,
};
pub use clients::client_ether::{EtherInbound, EtherOutbound, ether_client_task};
pub use clients::client_game::{ClientHandle, ClientIO, create_io};
pub use clients::client_replay::replay_task;
pub use engine::{CycleResult, Engine, LoginRequest, TickStats, with_engine};
pub use player_save::PlayerProfile;
pub use rs_db::DbConfig;
pub use rs_entity::build::{MAX_NPCS, MAX_PLAYERS};
