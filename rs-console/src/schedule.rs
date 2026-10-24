use crate::audit::Event;
use crate::discord::{Card, ORANGE, PURPLE, countdown, escape};
use crate::http::App;
use crate::store::JsonFile;
use crate::users::now_ms;
use crate::worlds::{Outcome, Reply, WorldConfig};
use rs_admin_api::{Actor, Broadcast, Reboot, path};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// How often due jobs are looked for.
const CHECK_INTERVAL: Duration = Duration::from_secs(5);
/// A job this late (the console was down when it was due) is skipped rather
/// than run: a reboot nobody expects is worse than one that didn't happen.
const LATE_LIMIT_MS: u64 = 5 * 60 * 1000;

pub const MAX_JOBS: usize = 100;
/// Repeats are at least this far apart, and at most this far.
pub const MIN_EVERY_MINS: u32 = 10;
pub const MAX_EVERY_MINS: u32 = 31 * 24 * 60;
/// How far ahead a job can be set.
pub const MAX_AHEAD_MS: u64 = 366 * 24 * 3600 * 1000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Task {
    Reboot {
        countdown_secs: u32,
        stagger_secs: u32,
        reason: Option<String>,
    },
    Broadcast {
        message: String,
    },
}

impl Task {
    fn audit_action(&self) -> &'static str {
        match self {
            Self::Reboot { .. } => "scheduled_reboot",
            Self::Broadcast { .. } => "scheduled_broadcast",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    pub task: Task,
    pub nodes: Vec<u8>,
    pub next_ms: u64,
    pub every_mins: Option<u32>,
    pub by: String,
    pub by_level: u8,
    pub created_ms: u64,
    pub last: Option<LastRun>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LastRun {
    pub at_ms: u64,
    pub ok: bool,
    pub summary: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Jobs {
    pub next_id: u64,
    pub jobs: Vec<Job>,
}

pub struct Schedule {
    file: JsonFile<Jobs>,
}

impl Schedule {
    pub fn open(path: PathBuf) -> Self {
        Self {
            file: JsonFile::open(path),
        }
    }

    pub fn jobs(&self) -> Vec<Job> {
        let mut jobs = self.file.get().jobs;
        jobs.sort_by_key(|j| j.next_ms);
        jobs
    }

    /// Adds a job, giving it its id.
    pub fn add(&self, mut job: Job) -> Result<Job, String> {
        self.file.update(|file| {
            if file.jobs.len() >= MAX_JOBS {
                return Err(format!("The schedule holds at most {MAX_JOBS} jobs."));
            }
            file.next_id += 1;
            job.id = file.next_id;
            file.jobs.push(job.clone());
            Ok(job)
        })
    }

    pub fn remove(&self, id: u64) -> Result<Job, String> {
        self.file.update(|file| {
            let i = file
                .jobs
                .iter()
                .position(|j| j.id == id)
                .ok_or("That job isn't scheduled any more.")?;
            Ok(file.jobs.remove(i))
        })
    }

    /// Takes the jobs due at `now`: a repeating one moves on to its next
    /// time, one that runs once is removed. Saved before any runs, so a
    /// crash can't run a job twice. Returns the jobs to run and those
    /// skipped for being too late.
    fn take_due(&self, now: u64) -> (Vec<Job>, Vec<Job>) {
        // Mostly nothing is: no need to write the file again.
        if self
            .file
            .read(|file| file.jobs.iter().all(|j| j.next_ms > now))
        {
            return (Vec::new(), Vec::new());
        }
        let taken = self.file.update(|file| {
            let mut run = Vec::new();
            let mut missed = Vec::new();
            file.jobs.retain_mut(|job| {
                if job.next_ms > now {
                    return true;
                }
                if now - job.next_ms > LATE_LIMIT_MS {
                    missed.push(job.clone());
                } else {
                    run.push(job.clone());
                }
                match job.every_mins {
                    Some(every) => {
                        job.next_ms = next_after(job.next_ms, every, now);
                        true
                    }
                    None => false,
                }
            });
            Ok((run, missed))
        });
        taken.unwrap_or_else(|e| {
            warn!("Couldn't take the scheduled jobs that are due: {}", e);
            (Vec::new(), Vec::new())
        })
    }

    fn record(&self, id: u64, last: LastRun) {
        let _ = self.file.update(|file| {
            if let Some(job) = file.jobs.iter_mut().find(|j| j.id == id) {
                job.last = Some(last);
            }
            Ok(())
        });
    }
}

/// The first time after `now` a job repeating every `every_mins` from `from`
/// is due.
fn next_after(from: u64, every_mins: u32, now: u64) -> u64 {
    let every = u64::from(every_mins.max(1)) * 60_000;
    from + (now.saturating_sub(from) / every + 1) * every
}

/// The worlds a job goes to, as they are now.
fn targets(app: &App, nodes: &[u8]) -> Vec<WorldConfig> {
    if nodes.is_empty() {
        app.worlds.online()
    } else {
        nodes
            .iter()
            .filter_map(|&n| app.worlds.get(n).cloned())
            .collect()
    }
}

async fn run(app: &App, job: &Job) -> LastRun {
    let targets = targets(app, &job.nodes);
    if targets.is_empty() {
        return LastRun {
            at_ms: now_ms(),
            ok: false,
            summary: "No world was online to send it to.".to_owned(),
        };
    }
    let actor = Actor {
        name: job.by.clone(),
        staff_level: job.by_level,
    };
    let replies: Vec<Reply<rs_admin_api::Outcome>> = match &job.task {
        Task::Reboot {
            countdown_secs,
            stagger_secs,
            reason,
        } => {
            app.worlds
                .command(&targets, path::REBOOT, |i| Reboot {
                    actor: actor.clone(),
                    seconds: countdown_secs + stagger_secs * i as u32,
                    reason: reason.clone(),
                    stay_down: false,
                })
                .await
        }
        Task::Broadcast { message } => {
            app.worlds
                .command(&targets, path::BROADCAST, |_| Broadcast {
                    actor: actor.clone(),
                    message: message.clone(),
                })
                .await
        }
    };
    let card = match &job.task {
        Task::Reboot {
            countdown_secs,
            stagger_secs,
            reason,
        } => countdown(
            "Scheduled reboot",
            ORANGE,
            *countdown_secs,
            *stagger_secs,
            targets.len(),
            reason.as_deref(),
        ),
        Task::Broadcast { message } => {
            Card::new("Scheduled broadcast", PURPLE).text(escape(message))
        }
    };
    app.discord.world_action(&job.by, &replies, card);
    let failed: Vec<String> = replies
        .iter()
        .filter_map(|r| match &r.result {
            Outcome::Answered { answer } if answer.ok => None,
            Outcome::Answered { answer } => Some(format!("world {}: {}", r.world, answer.message)),
            Outcome::Failed { error } => Some(format!("world {}: {}", r.world, error)),
        })
        .collect();
    let ok = replies.len() - failed.len();
    let mut summary = format!("{ok}/{} ok", replies.len());
    if !failed.is_empty() {
        summary.push_str(&format!(" ({})", failed.join("; ")));
    }
    let nodes: Vec<u8> = targets.iter().map(|w| w.node).collect();
    app.worlds.refresh(Some(&nodes)).await;
    LastRun {
        at_ms: now_ms(),
        ok: failed.is_empty(),
        summary,
    }
}

/// What a job is, for the audit log.
pub fn describe(job: &Job) -> String {
    let worlds = if job.nodes.is_empty() {
        "every world".to_owned()
    } else {
        format!("{} world(s)", job.nodes.len())
    };
    let what = match &job.task {
        Task::Reboot {
            countdown_secs,
            stagger_secs,
            reason,
        } => format!(
            "reboot of {worlds}, {countdown_secs}s countdown, stagger {stagger_secs}s{}",
            reason
                .as_ref()
                .map(|r| format!(" (\"{r}\")"))
                .unwrap_or_default()
        ),
        Task::Broadcast { message } => format!("broadcast to {worlds}: \"{message}\""),
    };
    match job.every_mins {
        Some(every) => format!("{what}, every {every} min"),
        None => what,
    }
}

/// Runs scheduled jobs when they're due.
pub async fn keep_running(app: Arc<App>) {
    let mut interval = tokio::time::interval(CHECK_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        let (due, missed) = app.schedule.take_due(now_ms());
        for job in missed {
            warn!(
                "Skipped scheduled job #{}: it was due while the console was down",
                job.id
            );
            app.audit.record(
                Event::system(Some(&job.by), "scheduled_missed")
                    .target(format!("job #{}", job.id))
                    .detail(format!(
                        "{}: skipped, it was due while the console was down",
                        describe(&job)
                    ))
                    .ok(false),
            );
            app.schedule.record(
                job.id,
                LastRun {
                    at_ms: now_ms(),
                    ok: false,
                    summary: "Skipped: it was due while the console was down.".to_owned(),
                },
            );
        }
        for job in due {
            let last = run(&app, &job).await;
            info!("Ran scheduled job #{}: {}", job.id, last.summary);
            app.audit.record(
                Event::system(Some(&job.by), job.task.audit_action())
                    .target(format!("job #{}", job.id))
                    .detail(format!("{}: {}", describe(&job), last.summary))
                    .ok(last.ok),
            );
            app.schedule.record(job.id, last);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeats_land_on_their_grid() {
        let min = 60_000;
        // Due now: the next is one period on.
        assert_eq!(next_after(0, 10, 0), 10 * min);
        // Overdue by a bit: still the grid, not "now + period".
        assert_eq!(next_after(0, 10, 3 * min), 10 * min);
        // Many periods missed: the first one in the future.
        assert_eq!(next_after(0, 10, 95 * min), 100 * min);
        assert_eq!(next_after(0, 10, 100 * min), 110 * min);
    }

    #[test]
    fn nothing_due_writes_nothing() {
        let path = std::env::temp_dir().join(format!("rs-console-schedule-{}.json", now_ms()));
        let schedule = Schedule::open(path.clone());
        let job = |next_ms| Job {
            id: 0,
            task: Task::Broadcast {
                message: "Hi".into(),
            },
            nodes: Vec::new(),
            next_ms,
            every_mins: None,
            by: "Ash".into(),
            by_level: 2,
            created_ms: 0,
            last: None,
        };
        schedule.add(job(1000)).unwrap();
        std::fs::remove_file(&path).unwrap();
        let (due, missed) = schedule.take_due(999);
        assert!(due.is_empty() && missed.is_empty());
        assert!(!path.exists());
        assert_eq!(schedule.take_due(1000).0.len(), 1);
        assert!(path.exists());
        let _ = std::fs::remove_file(path);
    }
}
