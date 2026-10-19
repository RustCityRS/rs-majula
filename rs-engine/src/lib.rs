extern crate alloc;
extern crate core;

mod active_npc;
mod active_player;
mod build;
mod clients;
pub mod counters;
mod engine;
mod game_map;
mod handlers;
mod info;
mod phases;
mod player_save;

pub use clients::client_db::{DbClientConfig, DbRequest, DbResponse, db_client_task};
pub use clients::client_ether::{EtherInbound, EtherOutbound, ether_client_task};
pub use clients::client_game::{ClientHandle, ClientIO, create_io};
pub use engine::{CycleResult, Engine, LoginRequest, TickStats, with_engine};
pub use rs_db::DbConfig;
pub use rs_entity::build::{MAX_NPCS, MAX_PLAYERS};
