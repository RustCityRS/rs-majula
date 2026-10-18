use crate::app::App;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{info, warn};

/// How often the live world list is re-read from `world_status`.
const REFRESH: Duration = Duration::from_secs(10);
/// A world whose heartbeat is older than this is shown as offline. Worlds
/// report every ~10 seconds, so this tolerates a couple of missed beats.
const MAX_AGE_SECS: i32 = 30;

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
    /// Filled from the world's heartbeat, never from `worlds.json`.
    #[serde(default, skip_deserializing)]
    pub players: u32,
    #[serde(default, skip_deserializing)]
    pub online: bool,
}

impl World {
    /// The game server's `--node-id` for this world (world 1 is node 10).
    pub fn node_id(&self) -> u16 {
        self.id + 9
    }
}

pub fn load(path: &Path) -> anyhow::Result<Vec<World>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

/// Player counts of the worlds with a fresh heartbeat, by node id.
#[derive(Default)]
pub struct LiveWorlds(RwLock<HashMap<u16, u32>>);

impl LiveWorlds {
    /// `worlds` with their live player counts and online state filled in.
    pub fn apply(&self, worlds: &[World]) -> Vec<World> {
        let live = self.0.read().unwrap_or_else(|e| e.into_inner());
        worlds
            .iter()
            .map(|world| {
                let players = live.get(&world.node_id()).copied();
                World {
                    players: players.unwrap_or(0),
                    online: players.is_some(),
                    ..world.clone()
                }
            })
            .collect()
    }

    fn set(&self, live: HashMap<u16, u32>) {
        *self.0.write().unwrap_or_else(|e| e.into_inner()) = live;
    }
}

/// Re-reads the live world list forever. While the database is unreachable
/// every world shows as offline, since none of them can log anyone in.
pub async fn refresh(app: Arc<App>) {
    let mut interval = tokio::time::interval(REFRESH);
    let mut healthy = true;
    loop {
        interval.tick().await;
        match app.db.live_worlds(MAX_AGE_SECS).await {
            Ok(statuses) => {
                if !healthy {
                    info!("World status is readable again");
                    healthy = true;
                }
                app.live_worlds.set(
                    statuses
                        .into_iter()
                        .map(|s| (s.node_id as u16, s.players.max(0) as u32))
                        .collect(),
                );
            }
            Err(e) => {
                if healthy {
                    warn!("Reading world status failed ({e}); listing every world as offline");
                    healthy = false;
                }
                app.live_worlds.set(HashMap::new());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(id: u16) -> World {
        World {
            id,
            region: "Local Development".into(),
            address: "http://localhost:8080".into(),
            members: true,
            port_offset: 0,
            players: 0,
            online: false,
        }
    }

    #[test]
    fn only_reporting_worlds_are_online() {
        let live = LiveWorlds::default();
        live.set(HashMap::from([(10, 4)]));
        let worlds = live.apply(&[world(1), world(2)]);
        assert_eq!((worlds[0].online, worlds[0].players), (true, 4));
        assert_eq!((worlds[1].online, worlds[1].players), (false, 0));
    }
}
