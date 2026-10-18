use serde::{Deserialize, Serialize};
use std::path::Path;

/// A game world, as listed in `worlds.json`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct World {
    pub id: u16,
    pub region: String,
    /// Base URL of the world's own HTTP server (it hosts the web client),
    /// e.g. `http://localhost:8080` for node 10.
    pub address: String,
    pub members: bool,
    /// Added to the game port by the desktop client.
    #[serde(default)]
    pub port_offset: u16,
    /// Not tracked yet; always 0.
    #[serde(default, skip_deserializing)]
    pub players: u32,
}

pub fn load(path: &Path) -> anyhow::Result<Vec<World>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}
