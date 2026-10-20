use crate::http::App;
use crate::store::JsonFile;
use crate::worlds::{Outcome, Reply};
use rs_admin_api::{Actor, ControlsSet, SetControls, WorldControls, path};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// How often worlds are checked against what's wanted.
const SYNC_INTERVAL: Duration = Duration::from_secs(5);
/// How long before sending a world what it refused (or failed to take) again.
const RETRY_AFTER: Duration = Duration::from_secs(60);

/// A kill switch staff can turn on with one click: the scripts behind a
/// feature, by name. Every revision's content names them the same.
#[derive(Serialize)]
pub struct Preset {
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub patterns: &'static [&'static str],
}

pub const PRESETS: &[Preset] = &[
    Preset {
        key: "trading",
        label: "Trading",
        description: "Starting a trade, offering or removing items, and accepting either screen.",
        patterns: &[
            "[opplayer4,_]",
            "[inv_button*,trademain:inv]",
            "[inv_button*,tradeside:inv]",
            "[if_button,trademain:accept]",
            "[if_button,tradeconfirm:accept]",
        ],
    },
    Preset {
        key: "dueling",
        label: "Duel arena",
        description: "Challenging, staking items and accepting a duel.",
        patterns: &[
            "[opplayer1,_]",
            "[inv_button*,duel_*]",
            "[if_button,duel_select_type:accept]",
            "[if_button,duel_confirm:accept]",
        ],
    },
    Preset {
        key: "dropping",
        label: "Dropping items",
        description: "The Drop option on every item.",
        patterns: &["[opheld5,*]"],
    },
    Preset {
        key: "pickup",
        label: "Picking up items",
        description: "Taking items off the ground.",
        patterns: &["[opobj3,*]"],
    },
    Preset {
        key: "shops",
        label: "Shops",
        description: "Buying from and selling to shops.",
        patterns: &["[inv_button*,shop_template*]"],
    },
    Preset {
        key: "banking",
        label: "Banking",
        description: "Depositing and withdrawing, and the bank's buttons.",
        patterns: &["[inv_button*,bank_*]", "[if_button,bank_main:*]"],
    },
];

/// The most patterns a custom switch has, and the longest one.
const MAX_PATTERNS: usize = 16;
const MAX_PATTERN_CHARS: usize = 100;
/// What a world takes in all.
const MAX_ALL_PATTERNS: usize = 64;
const MAX_LABEL_CHARS: usize = 60;
/// The longest lockdown: accounts up to a year old turned away.
pub const MAX_LOCKDOWN_HOURS: u32 = 8760;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Desired {
    pub switches: Vec<ActiveSwitch>,
    pub lockdown: Option<Lockdown>,
}

/// A kill switch that's on.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActiveSwitch {
    /// A preset's key, or `custom-<n>`.
    pub key: String,
    pub label: String,
    pub patterns: Vec<String>,
    pub by: String,
    pub at_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lockdown {
    pub hours: u32,
    pub by: String,
    pub at_ms: u64,
}

impl Desired {
    /// What every world should have.
    pub fn world(&self) -> WorldControls {
        let mut switches: Vec<String> = Vec::new();
        for pattern in self.switches.iter().flat_map(|s| &s.patterns) {
            if !switches.contains(pattern) {
                switches.push(pattern.clone());
            }
        }
        WorldControls {
            switches,
            min_account_age_hours: self.lockdown.as_ref().map_or(0, |l| l.hours),
        }
    }
}

/// Checks a custom switch's patterns, trimmed.
pub fn checked_patterns(patterns: &[String]) -> Result<Vec<String>, String> {
    let patterns: Vec<String> = patterns
        .iter()
        .map(|p| p.trim().to_owned())
        .filter(|p| !p.is_empty())
        .collect();
    if patterns.is_empty() || patterns.len() > MAX_PATTERNS {
        return Err(format!("A switch has 1 to {MAX_PATTERNS} patterns."));
    }
    if let Some(p) = patterns
        .iter()
        .find(|p| p.chars().count() > MAX_PATTERN_CHARS)
    {
        return Err(format!(
            "\"{p}\" is too long: patterns are at most {MAX_PATTERN_CHARS} characters."
        ));
    }
    if let Some(p) = patterns.iter().find(|p| p.chars().all(|c| c == '*')) {
        return Err(format!(
            "\"{p}\" would switch off every script, logins included."
        ));
    }
    Ok(patterns)
}

pub fn checked_label(label: &str) -> Result<String, String> {
    let label = label.trim();
    if label.is_empty() || label.chars().count() > MAX_LABEL_CHARS {
        return Err(format!("A label is 1 to {MAX_LABEL_CHARS} characters."));
    }
    Ok(label.to_owned())
}

/// Refuses a set of switches no world would take.
pub fn check_total(desired: &Desired) -> Result<(), String> {
    if desired.world().switches.len() > MAX_ALL_PATTERNS {
        return Err(format!(
            "The switches on add up to more than the {MAX_ALL_PATTERNS} patterns a world takes."
        ));
    }
    Ok(())
}

/// The last time a world was sent its controls, and how it went.
#[derive(Clone, Debug)]
pub struct Push {
    pub at: Instant,
    pub sent: WorldControls,
    /// How many scripts each pattern matched there, or why it failed.
    pub result: Result<Vec<u32>, String>,
}

pub struct Controls {
    file: JsonFile<Desired>,
    pushes: Mutex<HashMap<u8, Push>>,
}

impl Controls {
    pub fn open(path: PathBuf) -> Self {
        Self {
            file: JsonFile::open(path),
            pushes: Mutex::default(),
        }
    }

    pub fn desired(&self) -> Desired {
        self.file.get()
    }

    pub fn update<R>(
        &self,
        f: impl FnOnce(&mut Desired) -> Result<R, String>,
    ) -> Result<R, String> {
        self.file.update(|desired| {
            let result = f(desired)?;
            check_total(desired)?;
            Ok(result)
        })
    }

    pub fn push(&self, node: u8) -> Option<Push> {
        self.lock().get(&node).cloned()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<u8, Push>> {
        self.pushes.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Whether a world that has `current` should be sent `wanted` now: it
    /// differs, and the world didn't just fail to take that same `wanted`.
    fn due(&self, node: u8, current: &WorldControls, wanted: &WorldControls) -> bool {
        if current == wanted {
            return false;
        }
        match self.lock().get(&node) {
            Some(push) if push.sent == *wanted => {
                push.result.is_ok() || push.at.elapsed() >= RETRY_AFTER
            }
            _ => true,
        }
    }
}

/// Sends the wanted controls to `nodes` (every online world when `None`)
/// that don't have them yet, all at once, and refreshes their status.
pub async fn sync(app: &App, actor: &str, nodes: Option<&[u8]>) {
    let wanted = app.controls.desired().world();
    let targets: Vec<_> = app
        .worlds
        .views()
        .into_iter()
        .filter(|v| nodes.is_none_or(|n| n.contains(&v.config.node)))
        .filter(|v| {
            // A world without controls in its status predates them.
            v.status
                .as_ref()
                .and_then(|s| s.controls.as_ref())
                .is_some_and(|current| app.controls.due(v.config.node, current, &wanted))
        })
        .map(|v| v.config)
        .collect();
    if targets.is_empty() {
        return;
    }
    let actor = Actor {
        name: actor.to_owned(),
        staff_level: 0,
    };
    let replies: Vec<Reply<Result<ControlsSet, String>>> = app
        .worlds
        .command(&targets, path::CONTROLS, |_| SetControls {
            actor: actor.clone(),
            controls: wanted.clone(),
        })
        .await;
    {
        let mut pushes = app.controls.lock();
        for reply in replies {
            let result = match reply.result {
                Outcome::Answered { answer: Ok(set) } => Ok(set.matched),
                Outcome::Answered {
                    answer: Err(refused),
                } => Err(refused),
                Outcome::Failed { error } => Err(error),
            };
            match &result {
                Ok(_) => info!("Set world {}'s kill switches and lockdown", reply.world),
                Err(e) => warn!(
                    "World {} didn't take its kill switches and lockdown: {}",
                    reply.world, e
                ),
            }
            pushes.insert(
                reply.node,
                Push {
                    at: Instant::now(),
                    sent: wanted.clone(),
                    result,
                },
            );
        }
    }
    let nodes: Vec<u8> = targets.iter().map(|w| w.node).collect();
    app.worlds.refresh(Some(&nodes)).await;
}

/// Keeps every world's controls as wanted.
pub async fn keep_in_sync(app: Arc<App>) {
    let mut interval = tokio::time::interval(SYNC_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        sync(&app, "console", None).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn switch(key: &str, patterns: &[&str]) -> ActiveSwitch {
        ActiveSwitch {
            key: key.to_owned(),
            label: key.to_owned(),
            patterns: patterns.iter().map(|p| p.to_string()).collect(),
            by: "mod".to_owned(),
            at_ms: 0,
        }
    }

    #[test]
    fn worlds_get_each_pattern_once() {
        let desired = Desired {
            switches: vec![
                switch("a", &["[x,*]", "[y,_]"]),
                switch("b", &["[y,_]", "[z,_]"]),
            ],
            lockdown: Some(Lockdown {
                hours: 24,
                by: "mod".to_owned(),
                at_ms: 0,
            }),
        };
        let world = desired.world();
        assert_eq!(world.switches, ["[x,*]", "[y,_]", "[z,_]"]);
        assert_eq!(world.min_account_age_hours, 24);
        assert_eq!(Desired::default().world(), WorldControls::default());
    }

    #[test]
    fn patterns_are_checked() {
        let p = |v: &[&str]| checked_patterns(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(p(&[" [opheld5,*] ", ""]).unwrap(), ["[opheld5,*]"]);
        assert!(p(&[]).is_err());
        assert!(p(&["*"]).is_err());
        assert!(p(&["**"]).is_err());
        assert!(p(&[&"x".repeat(101)]).is_err());
    }

    #[test]
    fn presets_are_valid() {
        for preset in PRESETS {
            let patterns: Vec<String> = preset.patterns.iter().map(|p| p.to_string()).collect();
            assert_eq!(
                checked_patterns(&patterns).unwrap(),
                patterns,
                "{}",
                preset.key
            );
        }
    }
}
