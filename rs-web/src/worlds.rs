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
    /// Closed to new logins for maintenance; also from the heartbeat.
    #[serde(default, skip_deserializing)]
    pub maintenance: bool,
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

/// What the worlds with a fresh heartbeat report, by node id.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Live {
    players: u32,
    maintenance: bool,
}

#[derive(Default)]
pub struct LiveWorlds(RwLock<HashMap<u16, Live>>);

impl LiveWorlds {
    /// `worlds` with their live player counts, online and maintenance state
    /// filled in.
    pub fn apply(&self, worlds: &[World]) -> Vec<World> {
        let live = self.0.read().unwrap_or_else(|e| e.into_inner());
        worlds
            .iter()
            .map(|world| {
                let status = live.get(&world.node_id()).copied();
                World {
                    players: status.map_or(0, |s| s.players),
                    online: status.is_some(),
                    maintenance: status.is_some_and(|s| s.maintenance),
                    ..world.clone()
                }
            })
            .collect()
    }

    fn set(&self, live: HashMap<u16, Live>) {
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
                        .map(|s| {
                            let live = Live {
                                players: s.players.max(0) as u32,
                                maintenance: s.maintenance,
                            };
                            (s.node_id as u16, live)
                        })
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
            maintenance: false,
        }
    }

    #[test]
    fn only_reporting_worlds_are_online() {
        let live = LiveWorlds::default();
        let up = |players, maintenance| Live {
            players,
            maintenance,
        };
        live.set(HashMap::from([(10, up(4, false)), (12, up(1, true))]));
        let worlds = live.apply(&[world(1), world(2), world(3)]);
        let seen: Vec<_> = worlds
            .iter()
            .map(|w| (w.online, w.players, w.maintenance))
            .collect();
        assert_eq!(seen, [(true, 4, false), (false, 0, false), (true, 1, true)]);
    }
}
