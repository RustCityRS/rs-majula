use crate::audit::Event;
use crate::http::App;
use crate::store::JsonFile;
use crate::users::now_ms;
use rs_admin_api::{ItemCirculation, ObjLine, ObjSearch, TotalsLookup, path};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{info, warn};

/// How often the items tracked are counted across every save.
pub const SAMPLE_EVERY_MS: u64 = 15 * 60 * 1000;
/// The rise between two counts that raises an alert, unless an item says.
pub const DEFAULT_ALERT_PCT: f64 = 5.0;
pub const MAX_TRACKED: usize = 50;
/// How long counts are kept.
const KEEP_MS: u64 = 400 * 24 * 3600 * 1000;
/// The most alerts kept, newest.
const MAX_ALERTS: usize = 500;

/// What's tracked until staff change it: coins, and the rares no content
/// makes more of, so any rise in them is a dupe or a staff spawn.
const DEFAULT_ITEMS: [&str; 14] = [
    "coins",
    "red_partyhat",
    "yellow_partyhat",
    "blue_partyhat",
    "green_partyhat",
    "purple_partyhat",
    "white_partyhat",
    "christmas_cracker",
    "santa_hat",
    "halloweenmask_green",
    "halloweenmask_blue",
    "halloweenmask_red",
    "pumpkin",
    "easter_egg",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tracked {
    pub obj: u16,
    pub name: String,
    pub alert_pct: f64,
    pub by: String,
    pub at_ms: u64,
}

/// One count of every tracked item.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sample {
    pub at_ms: u64,
    pub items: Vec<SampleItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SampleItem {
    pub obj: u16,
    pub total: i64,
    pub holders: i64,
}

/// A count that rose more than its item's threshold since the one before.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alert {
    pub id: u64,
    pub at_ms: u64,
    pub obj: u16,
    pub name: String,
    pub from: i64,
    pub to: i64,
    pub since_ms: u64,
    pub acknowledged: Option<Acknowledged>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Acknowledged {
    pub by: String,
    pub at_ms: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EconomyState {
    #[serde(default)]
    pub seeded: bool,
    #[serde(default)]
    pub tracked: Vec<Tracked>,
    #[serde(default)]
    pub alerts: Vec<Alert>,
    #[serde(default)]
    pub next_alert: u64,
}

/// The items whose circulation the console follows, their counts over time
/// (`economy-samples.jsonl`, one per line) and the alerts the counts raised
/// (`economy.json`).
pub struct Economy {
    state: JsonFile<EconomyState>,
    samples_path: PathBuf,
    samples: Mutex<Vec<Sample>>,
}

impl Economy {
    pub fn open(dir: &Path) -> Self {
        let samples_path = dir.join("economy-samples.jsonl");
        let (samples, dropped) = load_samples(&samples_path);
        // Counts past their keeping time leave the file too.
        if dropped > 0
            && let Err(e) = rewrite_samples(&samples_path, &samples)
        {
            warn!("Couldn't compact {}: {}", samples_path.display(), e);
        }
        Self {
            state: JsonFile::open(dir.join("economy.json")),
            samples: Mutex::new(samples),
            samples_path,
        }
    }

    pub fn state(&self) -> EconomyState {
        self.state.get()
    }

    /// The counts taken since `since_ms`, oldest first: every `n`-th of
    /// them (the last always), so at most `max`.
    pub fn samples_since(&self, since_ms: u64, max: usize) -> Vec<Sample> {
        let samples = self.lock();
        let from = samples.partition_point(|s| s.at_ms < since_ms);
        let wanted = &samples[from..];
        let step = wanted.len().div_ceil(max.max(1)).max(1);
        let last = wanted.len().saturating_sub(1);
        wanted
            .iter()
            .enumerate()
            .filter(|&(i, _)| i % step == 0 || i == last)
            .map(|(_, s)| s.clone())
            .collect()
    }

    pub fn last_sample_ms(&self) -> Option<u64> {
        self.lock().last().map(|s| s.at_ms)
    }

    /// Starts following `obj` (or changes its threshold).
    pub fn track(&self, obj: u16, name: &str, alert_pct: f64, by: &str) -> Result<(), String> {
        self.state.update(|state| {
            let count = state.tracked.len();
            match state.tracked.iter_mut().find(|t| t.obj == obj) {
                Some(t) => t.alert_pct = alert_pct,
                None if count >= MAX_TRACKED => {
                    return Err(format!("Track at most {MAX_TRACKED} items."));
                }
                None => state.tracked.push(Tracked {
                    obj,
                    name: name.to_owned(),
                    alert_pct,
                    by: by.to_owned(),
                    at_ms: now_ms(),
                }),
            }
            Ok(())
        })
    }

    pub fn untrack(&self, obj: u16) -> Result<Tracked, String> {
        self.state.update(|state| {
            let i = state
                .tracked
                .iter()
                .position(|t| t.obj == obj)
                .ok_or("That item isn't tracked.")?;
            Ok(state.tracked.remove(i))
        })
    }

    /// Puts the default items in, the first time.
    fn seed(&self, items: Vec<Tracked>) -> Result<(), String> {
        self.state.update(|state| {
            if !state.seeded {
                state.seeded = true;
                for item in items {
                    if !state.tracked.iter().any(|t| t.obj == item.obj) {
                        state.tracked.push(item);
                    }
                }
            }
            Ok(())
        })
    }

    /// Keeps a count, and raises an alert for each item that rose more than
    /// its threshold since the count before.
    pub fn record(&self, sample: Sample) -> Result<Vec<Alert>, String> {
        let mut samples = self.lock();
        let previous = samples.last().cloned();
        self.append(&sample)
            .map_err(|e| format!("Couldn't keep the count: {e}"))?;
        samples.push(sample.clone());
        let cutoff = now_ms().saturating_sub(KEEP_MS);
        samples.retain(|s| s.at_ms >= cutoff);
        drop(samples);
        let Some(previous) = previous else {
            return Ok(Vec::new());
        };
        self.state.update(|state| {
            let mut raised = Vec::new();
            for (tracked, from, to) in jumps(&state.tracked, &previous, &sample) {
                state.next_alert += 1;
                raised.push(Alert {
                    id: state.next_alert,
                    at_ms: sample.at_ms,
                    obj: tracked.obj,
                    name: tracked.name.clone(),
                    from,
                    to,
                    since_ms: previous.at_ms,
                    acknowledged: None,
                });
            }
            for alert in &raised {
                state.alerts.insert(0, alert.clone());
            }
            state.alerts.truncate(MAX_ALERTS);
            Ok(raised)
        })
    }

    pub fn acknowledge(&self, id: u64, by: &str) -> Result<Alert, String> {
        self.state.update(|state| {
            let alert = state
                .alerts
                .iter_mut()
                .find(|a| a.id == id)
                .ok_or("There's no such alert.")?;
            if alert.acknowledged.is_none() {
                alert.acknowledged = Some(Acknowledged {
                    by: by.to_owned(),
                    at_ms: now_ms(),
                });
            }
            Ok(alert.clone())
        })
    }

    /// Alerts nobody has acknowledged yet.
    pub fn open_alerts(&self) -> usize {
        self.state.read(|state| {
            state
                .alerts
                .iter()
                .filter(|a| a.acknowledged.is_none())
                .count()
        })
    }

    fn append(&self, sample: &Sample) -> std::io::Result<()> {
        if let Some(dir) = self.samples_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.samples_path)?;
        let mut line = serde_json::to_string(sample)?;
        line.push('\n');
        file.write_all(line.as_bytes())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Sample>> {
        self.samples.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// The counts in the file still kept, and how many older ones were left out.
fn load_samples(path: &Path) -> (Vec<Sample>, usize) {
    let Ok(file) = std::fs::File::open(path) else {
        return (Vec::new(), 0);
    };
    let cutoff = now_ms().saturating_sub(KEEP_MS);
    let mut samples = Vec::new();
    let mut dropped = 0;
    for (n, line) in BufReader::new(file).lines().enumerate() {
        let Ok(line) = line else {
            break;
        };
        match serde_json::from_str::<Sample>(&line) {
            Ok(sample) if sample.at_ms >= cutoff => samples.push(sample),
            Ok(_) => dropped += 1,
            Err(e) => warn!("Skipped line {} of {}: {}", n + 1, path.display(), e),
        }
    }
    (samples, dropped)
}

/// Writes `samples` as the whole file, beside it then renamed over it.
fn rewrite_samples(path: &Path, samples: &[Sample]) -> std::io::Result<()> {
    let mut text = String::new();
    for sample in samples {
        text.push_str(&serde_json::to_string(sample)?);
        text.push('\n');
    }
    let tmp = path.with_extension("jsonl.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

/// The tracked items whose count rose more than their threshold from
/// `before` to `now`: each with its count then and now. An item that had
/// none and now has some has risen past any threshold.
fn jumps<'a>(
    tracked: &'a [Tracked],
    before: &Sample,
    now: &Sample,
) -> Vec<(&'a Tracked, i64, i64)> {
    let count = |sample: &Sample, obj| sample.items.iter().find(|i| i.obj == obj).map(|i| i.total);
    tracked
        .iter()
        .filter_map(|t| {
            let from = count(before, t.obj)?;
            let to = count(now, t.obj)?;
            let rise = to - from;
            (rise > 0 && rise as f64 >= from as f64 * t.alert_pct / 100.0).then_some((t, from, to))
        })
        .collect()
}

/// Adds the default items the first time there's a world to name them.
async fn seed_defaults(app: &App) {
    if app.economy.state().seeded {
        return;
    }
    let mut items = Vec::new();
    for debugname in DEFAULT_ITEMS {
        let found: Result<Vec<ObjLine>, String> = app
            .worlds
            .ask(
                path::OBJ_SEARCH,
                &ObjSearch {
                    query: debugname.to_owned(),
                },
            )
            .await;
        match found {
            Ok(found) => {
                if let Some(obj) = found.into_iter().find(|o| o.debugname == debugname) {
                    items.push(Tracked {
                        obj: obj.id,
                        name: obj.name,
                        alert_pct: DEFAULT_ALERT_PCT,
                        by: "console".to_owned(),
                        at_ms: now_ms(),
                    });
                }
            }
            // Try again next time.
            Err(_) => return,
        }
    }
    if let Err(e) = app.economy.seed(items) {
        warn!("Couldn't put in the default economy items: {}", e);
    }
}

/// Counts every tracked item across every save now, keeps the count and
/// audits any alert it raises.
pub async fn sample_now(app: &App) -> Result<(Sample, Vec<Alert>), String> {
    seed_defaults(app).await;
    let tracked = app.economy.state().tracked;
    if tracked.is_empty() {
        return Err("No items are tracked.".to_owned());
    }
    let totals: Vec<ItemCirculation> = app
        .worlds
        .ask_long(
            path::ITEM_TOTALS,
            &TotalsLookup {
                objs: tracked.iter().map(|t| t.obj).collect(),
            },
        )
        .await?;
    let sample = Sample {
        at_ms: now_ms(),
        items: totals
            .into_iter()
            .map(|c| SampleItem {
                obj: c.item.id,
                total: c.total,
                holders: c.holders,
            })
            .collect(),
    };
    let alerts = app.economy.record(sample.clone())?;
    for alert in &alerts {
        warn!(
            "Economy alert: {} rose from {} to {}",
            alert.name, alert.from, alert.to
        );
        app.audit.record(
            Event::system(None, "economy_alert")
                .target(alert.name.clone())
                .detail(format!(
                    "{} in circulation rose from {} to {} in one count.",
                    alert.name, alert.from, alert.to
                ))
                .ok(false),
        );
    }
    Ok((sample, alerts))
}

/// Counts the tracked items every [`SAMPLE_EVERY_MS`], on the quarter hour.
pub async fn keep_sampling(app: Arc<App>) {
    // Soon after starting if the last count is stale, so a restart doesn't
    // leave a gap.
    let stale = app
        .economy
        .last_sample_ms()
        .is_none_or(|at| now_ms().saturating_sub(at) >= SAMPLE_EVERY_MS);
    let mut wait = if stale {
        30_000
    } else {
        SAMPLE_EVERY_MS - now_ms() % SAMPLE_EVERY_MS
    };
    loop {
        tokio::time::sleep(Duration::from_millis(wait)).await;
        match sample_now(&app).await {
            Ok((sample, _)) => info!("Counted {} tracked items", sample.items.len()),
            Err(e) => warn!("Couldn't count the tracked items: {}", e),
        }
        wait = SAMPLE_EVERY_MS - now_ms() % SAMPLE_EVERY_MS;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracked(obj: u16, alert_pct: f64) -> Tracked {
        Tracked {
            obj,
            name: format!("obj {obj}"),
            alert_pct,
            by: "test".to_owned(),
            at_ms: 0,
        }
    }

    fn sample(at_ms: u64, totals: &[(u16, i64)]) -> Sample {
        Sample {
            at_ms,
            items: totals
                .iter()
                .map(|&(obj, total)| SampleItem {
                    obj,
                    total,
                    holders: 1,
                })
                .collect(),
        }
    }

    #[test]
    fn only_rises_past_the_threshold_alert() {
        let items = [tracked(995, 5.0), tracked(1038, 5.0), tracked(1040, 5.0)];
        let before = sample(0, &[(995, 1000), (1038, 0), (1040, 10)]);
        let now = sample(1, &[(995, 1049), (1038, 1), (1040, 9)]);
        let raised: Vec<_> = jumps(&items, &before, &now)
            .into_iter()
            .map(|(t, from, to)| (t.obj, from, to))
            .collect();
        assert_eq!(raised, [(1038, 0, 1)]);
        let now = sample(1, &[(995, 1050)]);
        assert_eq!(jumps(&items, &before, &now).len(), 1);
    }

    #[test]
    fn counts_are_kept_and_compared() {
        let dir = std::env::temp_dir().join(format!("rs-console-economy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let economy = Economy::open(&dir);
        economy.track(995, "Coins", 10.0, "Ash").unwrap();
        let now = now_ms();
        assert!(
            economy
                .record(sample(now - 2, &[(995, 100)]))
                .unwrap()
                .is_empty()
        );
        let raised = economy.record(sample(now - 1, &[(995, 200)])).unwrap();
        assert_eq!((raised[0].from, raised[0].to), (100, 200));
        assert_eq!(economy.open_alerts(), 1);
        economy.acknowledge(raised[0].id, "Ash").unwrap();
        assert_eq!(economy.open_alerts(), 0);
        // Read back from the file.
        let again = Economy::open(&dir);
        assert_eq!(again.samples_since(0, 600).len(), 2);
        assert_eq!(again.samples_since(now - 1, 600).len(), 1);
        assert_eq!(again.state().tracked[0].alert_pct, 10.0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn long_charts_are_thinned_keeping_the_last() {
        let economy = Economy {
            state: JsonFile::open(std::env::temp_dir().join("rs-console-economy-thin-unused.json")),
            samples_path: PathBuf::new(),
            samples: Mutex::new((0..1000).map(|at| sample(at, &[])).collect()),
        };
        let thinned = economy.samples_since(0, 600);
        assert!(thinned.len() <= 600);
        assert_eq!(thinned.first().unwrap().at_ms, 0);
        assert_eq!(thinned.last().unwrap().at_ms, 999);
        assert_eq!(economy.samples_since(995, 600).len(), 5);
        assert!(economy.samples_since(5000, 600).is_empty());
    }

    #[test]
    fn old_counts_leave_the_file() {
        let dir =
            std::env::temp_dir().join(format!("rs-console-economy-old-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("economy-samples.jsonl");
        let lines: Vec<String> = [1, now_ms()]
            .iter()
            .map(|&at| serde_json::to_string(&sample(at, &[(995, 1)])).unwrap())
            .collect();
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        let economy = Economy::open(&dir);
        assert_eq!(economy.samples_since(0, 600).len(), 1);
        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
