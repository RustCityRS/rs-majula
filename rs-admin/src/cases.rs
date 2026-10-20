use crate::users::{name_key, now_ms};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::warn;

/// The longest case title.
pub const MAX_TITLE: usize = 100;
/// The longest comment, or note on closing.
pub const MAX_TEXT: usize = 4000;
/// Someone looking at a case shows as there for this long after.
const VIEWING_FOR: Duration = Duration::from_secs(20);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Open,
    Investigating,
    Waiting,
    Closed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

/// How a closed case ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    Actioned,
    NoAction,
    Duplicate,
}

/// An appeal a player sent from the website against a mute or ban, as its
/// case keeps it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Appeal {
    pub id: i64,
    pub sanction: String,
    pub until_ms: i64,
    pub issued_by: Option<String>,
    pub reason: Option<String>,
    pub statement: String,
    pub filed_ms: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Upheld,
    Overturned,
}

impl Decision {
    pub fn name(self) -> &'static str {
        match self {
            Self::Upheld => "upheld",
            Self::Overturned => "overturned",
        }
    }
}

/// An appeal, and how it was decided.
#[derive(Clone, Debug, Serialize)]
pub struct AppealState {
    #[serde(flatten)]
    pub appeal: Appeal,
    pub decision: Option<Decision>,
    pub decided_by: Option<String>,
    pub note: Option<String>,
}

/// Something a case is about, copied in when it's added: recordings are
/// pruned after a while, and the case keeps what it was.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Evidence {
    Report {
        id: i64,
        reporter: String,
        offender: String,
        reason: u8,
        node: u8,
        x: u16,
        z: u16,
        level: u8,
        at_ms: i64,
    },
    Replay {
        id: String,
        player: String,
        node: u8,
        revision: String,
        started_ms: i64,
        duration_ms: u32,
        score: i16,
        flags: Vec<String>,
    },
    Chat {
        id: i64,
        from: String,
        to: Option<String>,
        node: u8,
        message: String,
        at_ms: i64,
    },
}

impl Evidence {
    /// What it's known by: `report:12`, `replay:345`, `chat:678`.
    pub fn key(&self) -> String {
        match self {
            Self::Report { id, .. } => format!("report:{id}"),
            Self::Replay { id, .. } => format!("replay:{id}"),
            Self::Chat { id, .. } => format!("chat:{id}"),
        }
    }
}

/// One change, as the log records it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Change {
    Opened {
        title: String,
        player: String,
        priority: Priority,
    },
    Comment {
        text: String,
    },
    Assigned {
        name: String,
    },
    Unassigned {
        name: String,
    },
    Status {
        status: Status,
    },
    Priority {
        priority: Priority,
    },
    Retitled {
        title: String,
    },
    EvidenceAdded {
        evidence: Evidence,
    },
    EvidenceRemoved {
        key: String,
    },
    Action {
        action: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hours: Option<i32>,
        ok: bool,
        message: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        reason: String,
    },
    Closed {
        resolution: Resolution,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        note: String,
    },
    Reopened,
    Dismissed {
        key: String,
    },
    Restored {
        key: String,
    },
    AppealFiled {
        appeal: Appeal,
    },
    AppealDecided {
        decision: Decision,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        note: String,
    },
}

/// One line of the log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub at_ms: u64,
    pub by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case: Option<u64>,
    #[serde(flatten)]
    pub change: Change,
}

#[derive(Clone, Debug, Serialize)]
pub struct Case {
    pub id: u64,
    pub title: String,
    pub player: String,
    pub status: Status,
    pub priority: Priority,
    pub resolution: Option<Resolution>,
    pub created_by: String,
    pub created_ms: u64,
    pub updated_ms: u64,
    pub closed_ms: Option<u64>,
    pub closed_by: Option<String>,
    pub assignees: Vec<String>,
    pub evidence: Vec<Evidence>,
    pub comments: u32,
    pub actions: u32,
    pub appeal: Option<AppealState>,
    #[serde(skip)]
    pub timeline: Vec<Entry>,
}

impl Case {
    pub fn is_open(&self) -> bool {
        self.status != Status::Closed
    }

    pub fn has(&self, name: &str) -> bool {
        let key = name_key(name);
        self.assignees.iter().any(|a| name_key(a) == key)
    }

    fn has_evidence(&self, key: &str) -> bool {
        self.evidence.iter().any(|e| e.key() == key)
    }
}

/// Who set something aside from the triage queue, and when.
#[derive(Clone, Debug, Serialize)]
pub struct Dismissal {
    pub by: String,
    pub at_ms: u64,
}

/// How a report or session stands in the triage queue.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Triage {
    pub cases: Vec<u64>,
    pub dismissed: Option<Dismissal>,
}

#[derive(Default)]
struct Inner {
    cases: BTreeMap<u64, Case>,
    dismissed: HashMap<String, Dismissal>,
    next_id: u64,
}

impl Inner {
    /// Why `entry` can't be applied, if it can't.
    fn check(&self, entry: &Entry) -> Result<(), String> {
        let case = match (&entry.change, entry.case) {
            (Change::Dismissed { key }, None) => {
                return if self.dismissed.contains_key(key) {
                    Err("That's already dismissed.".into())
                } else {
                    Ok(())
                };
            }
            (Change::Restored { key }, None) => {
                return if self.dismissed.contains_key(key) {
                    Ok(())
                } else {
                    Err("That isn't dismissed.".into())
                };
            }
            (Change::Opened { .. }, Some(id)) => {
                return if self.cases.contains_key(&id) {
                    Err(format!("Case #{id} already exists."))
                } else {
                    Ok(())
                };
            }
            (_, Some(id)) => self
                .cases
                .get(&id)
                .ok_or_else(|| format!("There's no case #{id}."))?,
            (_, None) => return Err("That needs a case.".into()),
        };
        let closed = !case.is_open();
        match &entry.change {
            Change::Comment { .. } | Change::Action { .. } => Ok(()),
            Change::AppealFiled { .. } if case.appeal.is_some() || case.timeline.len() > 1 => {
                Err("An appeal is filed only as its case opens.".into())
            }
            Change::AppealDecided { .. } if case.appeal.is_none() => {
                Err("This case isn't an appeal.".into())
            }
            Change::AppealDecided { .. } if closed => {
                Err("The case is closed; reopen it first.".into())
            }
            Change::AppealDecided { .. }
                if case
                    .appeal
                    .as_ref()
                    .and_then(|a| a.appeal.issued_by.as_deref())
                    .is_some_and(|by| name_key(by) == name_key(&entry.by)) =>
            {
                Err("You issued this sanction, so someone else must decide the appeal.".into())
            }
            Change::Closed { .. } if case.appeal.as_ref().is_some_and(|a| a.decision.is_none()) => {
                Err("Decide the appeal to close its case.".into())
            }
            _ if closed && entry.change != Change::Reopened => {
                Err("The case is closed; reopen it first.".into())
            }
            Change::Assigned { name } if case.has(name) => {
                Err(format!("{name} is already on the case."))
            }
            Change::Unassigned { name } if !case.has(name) => {
                Err(format!("{name} isn't on the case."))
            }
            Change::Status { status } if *status == Status::Closed => {
                Err("Close the case to close it.".into())
            }
            Change::Status { status } if *status == case.status => {
                Err("The case already has that status.".into())
            }
            Change::Priority { priority } if *priority == case.priority => {
                Err("The case already has that priority.".into())
            }
            Change::EvidenceAdded { evidence } if case.has_evidence(&evidence.key()) => {
                Err("That's already in the case.".into())
            }
            Change::EvidenceRemoved { key } if !case.has_evidence(key) => {
                Err("That isn't in the case.".into())
            }
            Change::Reopened if !closed => Err("The case is already open.".into()),
            _ => Ok(()),
        }
    }

    /// Applies a checked entry.
    fn apply(&mut self, entry: Entry) {
        let Some(id) = entry.case else {
            match &entry.change {
                Change::Dismissed { key } => {
                    self.dismissed.insert(
                        key.clone(),
                        Dismissal {
                            by: entry.by.clone(),
                            at_ms: entry.at_ms,
                        },
                    );
                }
                Change::Restored { key } => {
                    self.dismissed.remove(key);
                }
                _ => {}
            }
            return;
        };
        if let Change::Opened {
            title,
            player,
            priority,
        } = &entry.change
        {
            self.next_id = self.next_id.max(id + 1);
            self.cases.insert(
                id,
                Case {
                    id,
                    title: title.clone(),
                    player: player.clone(),
                    status: Status::Open,
                    priority: *priority,
                    resolution: None,
                    created_by: entry.by.clone(),
                    created_ms: entry.at_ms,
                    updated_ms: entry.at_ms,
                    closed_ms: None,
                    closed_by: None,
                    assignees: Vec::new(),
                    evidence: Vec::new(),
                    comments: 0,
                    actions: 0,
                    appeal: None,
                    timeline: vec![entry],
                },
            );
            return;
        }
        let Some(case) = self.cases.get_mut(&id) else {
            return;
        };
        case.updated_ms = entry.at_ms;
        match &entry.change {
            Change::Comment { .. } => case.comments += 1,
            Change::Assigned { name } => case.assignees.push(name.clone()),
            Change::Unassigned { name } => {
                let key = name_key(name);
                case.assignees.retain(|a| name_key(a) != key);
            }
            Change::Status { status } => case.status = *status,
            Change::Priority { priority } => case.priority = *priority,
            Change::Retitled { title } => case.title = title.clone(),
            Change::EvidenceAdded { evidence } => case.evidence.push(evidence.clone()),
            Change::EvidenceRemoved { key } => case.evidence.retain(|e| e.key() != *key),
            Change::Action { ok, .. } => case.actions += u32::from(*ok),
            Change::Closed { resolution, .. } => {
                case.status = Status::Closed;
                case.resolution = Some(*resolution);
                case.closed_ms = Some(entry.at_ms);
                case.closed_by = Some(entry.by.clone());
            }
            Change::Reopened => {
                case.status = if case.assignees.is_empty() {
                    Status::Open
                } else {
                    Status::Investigating
                };
                case.resolution = None;
                case.closed_ms = None;
                case.closed_by = None;
            }
            Change::AppealFiled { appeal } => {
                case.appeal = Some(AppealState {
                    appeal: appeal.clone(),
                    decision: None,
                    decided_by: None,
                    note: None,
                });
            }
            Change::AppealDecided { decision, note } => {
                if let Some(state) = &mut case.appeal {
                    state.decision = Some(*decision);
                    state.decided_by = Some(entry.by.clone());
                    state.note = Some(note.clone()).filter(|n| !n.is_empty());
                }
            }
            Change::Opened { .. } | Change::Dismissed { .. } | Change::Restored { .. } => {}
        }
        case.timeline.push(entry);
    }
}

/// Who is looking at a case: by name key, their name and when they last
/// looked.
type Viewers = HashMap<String, (String, Instant)>;

pub struct Cases {
    path: PathBuf,
    inner: Mutex<Inner>,
    viewers: Mutex<HashMap<u64, Viewers>>,
}

impl Cases {
    /// Opens the log at `path` and rebuilds the cases from it.
    pub fn open(path: PathBuf) -> Self {
        let mut inner = Inner {
            next_id: 1,
            ..Inner::default()
        };
        if let Ok(text) = std::fs::read_to_string(&path) {
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                match serde_json::from_str::<Entry>(line) {
                    Ok(entry) => match inner.check(&entry) {
                        Ok(()) => inner.apply(entry),
                        Err(e) => warn!("Skipping a case log entry that doesn't apply: {}", e),
                    },
                    Err(e) => warn!("Skipping an unreadable case log entry: {}", e),
                }
            }
        }
        Self {
            path,
            inner: Mutex::new(inner),
            viewers: Mutex::default(),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn append(&self, entries: &[Entry]) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut lines = Vec::new();
        for entry in entries {
            serde_json::to_writer(&mut lines, entry)?;
            lines.push(b'\n');
        }
        // One write, so a change of several entries lands whole or not at all.
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?
            .write_all(&lines)
    }

    /// Checks each entry against the state the ones before it leave, writes
    /// them all, then applies them. Nothing changes if any is refused.
    fn commit(&self, inner: &mut Inner, entries: Vec<Entry>) -> Result<(), String> {
        let mut trial = Inner {
            cases: entries
                .iter()
                .filter_map(|e| e.case)
                .filter_map(|id| inner.cases.get(&id).map(|c| (id, c.clone())))
                .collect(),
            dismissed: inner.dismissed.clone(),
            next_id: inner.next_id,
        };
        for entry in &entries {
            trial.check(entry)?;
            trial.apply(entry.clone());
        }
        self.append(&entries).map_err(|e| {
            warn!(
                "Failed to write the case log {}: {}",
                self.path.display(),
                e
            );
            "The console couldn't save that. Try again.".to_owned()
        })?;
        for entry in entries {
            inner.apply(entry);
        }
        Ok(())
    }

    /// Opens a case on `player`, with `then` (evidence, a first comment, ...)
    /// as its first changes. Returns it.
    pub fn create(
        &self,
        by: &str,
        title: String,
        player: String,
        priority: Priority,
        then: Vec<Change>,
    ) -> Result<Case, String> {
        let mut inner = self.lock();
        let id = inner.next_id;
        let at_ms = now_ms();
        let entry = |change| Entry {
            at_ms,
            by: by.to_owned(),
            case: Some(id),
            change,
        };
        let mut entries = vec![entry(Change::Opened {
            title,
            player,
            priority,
        })];
        entries.extend(then.into_iter().map(entry));
        self.commit(&mut inner, entries)?;
        Ok(inner.cases[&id].clone())
    }

    /// Makes the changes `f` decides on, from the case as it is, to case
    /// `id` together, as `by`. Returns the case.
    pub fn change(
        &self,
        by: &str,
        id: u64,
        f: impl FnOnce(&Case) -> Result<Vec<Change>, String>,
    ) -> Result<Case, String> {
        let mut inner = self.lock();
        let case = inner
            .cases
            .get(&id)
            .ok_or_else(|| format!("There's no case #{id}."))?;
        let changes = f(case)?;
        let at_ms = now_ms();
        let entries = changes
            .into_iter()
            .map(|change| Entry {
                at_ms,
                by: by.to_owned(),
                case: Some(id),
                change,
            })
            .collect();
        self.commit(&mut inner, entries)?;
        Ok(inner.cases[&id].clone())
    }

    /// Sets a report or session aside from the triage queue, or puts it back.
    pub fn triage(&self, by: &str, key: String, dismissed: bool) -> Result<(), String> {
        let mut inner = self.lock();
        let change = if dismissed {
            Change::Dismissed { key }
        } else {
            Change::Restored { key }
        };
        let entry = Entry {
            at_ms: now_ms(),
            by: by.to_owned(),
            case: None,
            change,
        };
        self.commit(&mut inner, vec![entry])
    }

    pub fn get(&self, id: u64) -> Option<Case> {
        self.lock().cases.get(&id).cloned()
    }

    /// The case opened for website appeal `id`, if there is one.
    pub fn of_appeal(&self, id: i64) -> Option<u64> {
        self.lock()
            .cases
            .values()
            .find(|c| c.appeal.as_ref().is_some_and(|a| a.appeal.id == id))
            .map(|c| c.id)
    }

    /// Every case that passes `keep`.
    pub fn all(&self, keep: impl Fn(&Case) -> bool) -> Vec<Case> {
        self.lock()
            .cases
            .values()
            .filter(|c| keep(c))
            .map(|c| Case {
                timeline: Vec::new(),
                ..c.clone()
            })
            .collect()
    }

    /// How each of `keys` stands in the triage queue.
    pub fn triage_of(&self, keys: &[String]) -> HashMap<String, Triage> {
        let inner = self.lock();
        let mut found: HashMap<String, Triage> = keys
            .iter()
            .map(|k| {
                let triage = Triage {
                    cases: Vec::new(),
                    dismissed: inner.dismissed.get(k).cloned(),
                };
                (k.clone(), triage)
            })
            .collect();
        for case in inner.cases.values() {
            for evidence in &case.evidence {
                if let Some(triage) = found.get_mut(&evidence.key()) {
                    triage.cases.push(case.id);
                }
            }
        }
        found
    }

    /// Notes that `name` is looking at case `id`; returns who else is.
    pub fn viewing(&self, id: u64, name: &str) -> Vec<String> {
        let mut viewers = self.viewers.lock().unwrap_or_else(|e| e.into_inner());
        viewers.retain(|_, people| {
            people.retain(|_, (_, seen)| seen.elapsed() < VIEWING_FOR);
            !people.is_empty()
        });
        let me = name_key(name);
        let people = viewers.entry(id).or_default();
        people.insert(me.clone(), (name.to_owned(), Instant::now()));
        let mut others: Vec<String> = people
            .iter()
            .filter(|(key, _)| **key != me)
            .map(|(_, (name, _))| name.clone())
            .collect();
        others.sort();
        others
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_log(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rs-admin-cases-{tag}-{}.jsonl", now_ms()))
    }

    fn chat(id: i64) -> Evidence {
        Evidence::Chat {
            id,
            from: "Bob".into(),
            to: None,
            node: 10,
            message: "buy gold".into(),
            at_ms: 1,
        }
    }

    #[test]
    fn cases_rebuild_from_their_log() {
        let path = temp_log("rebuild");
        let cases = Cases::open(path.clone());
        let case = cases
            .create(
                "Ash",
                "Gold seller".into(),
                "Bob".into(),
                Priority::High,
                vec![
                    Change::EvidenceAdded { evidence: chat(5) },
                    Change::Assigned { name: "Ash".into() },
                ],
            )
            .unwrap();
        assert_eq!(case.id, 1);
        cases
            .change("Kim", 1, |_| {
                Ok(vec![Change::Comment {
                    text: "Seen him at the GE too".into(),
                }])
            })
            .unwrap();
        cases
            .change("Ash", 1, |_| {
                Ok(vec![Change::Closed {
                    resolution: Resolution::Actioned,
                    note: String::new(),
                }])
            })
            .unwrap();
        cases.triage("Kim", "report:9".into(), true).unwrap();

        let reopened = Cases::open(path.clone());
        let case = reopened.get(1).unwrap();
        assert_eq!(case.status, Status::Closed);
        assert_eq!(case.resolution, Some(Resolution::Actioned));
        assert_eq!(case.closed_by.as_deref(), Some("Ash"));
        assert_eq!(case.assignees, ["Ash"]);
        assert_eq!(case.comments, 1);
        assert_eq!(case.timeline.len(), 5);
        let triage = reopened.triage_of(&["chat:5".into(), "report:9".into()]);
        assert_eq!(triage["chat:5"].cases, [1]);
        assert_eq!(triage["report:9"].dismissed.as_ref().unwrap().by, "Kim");
        // The next case carries on from the log's ids.
        let next = reopened
            .create("Kim", "Next".into(), "Eve".into(), Priority::Low, vec![])
            .unwrap();
        assert_eq!(next.id, 2);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn refused_changes_leave_nothing_behind() {
        let path = temp_log("refused");
        let cases = Cases::open(path.clone());
        cases
            .create("Ash", "T".into(), "Bob".into(), Priority::Normal, vec![])
            .unwrap();
        // The second half of this is refused, so neither half happens.
        let refused = cases.change("Ash", 1, |_| {
            Ok(vec![
                Change::Assigned { name: "Ash".into() },
                Change::Assigned { name: "ash".into() },
            ])
        });
        assert!(refused.is_err());
        assert!(cases.get(1).unwrap().assignees.is_empty());
        assert!(
            cases
                .change("Ash", 2, |_| Ok(vec![Change::Reopened]))
                .is_err()
        );
        let close = |_: &Case| {
            Ok(vec![Change::Status {
                status: Status::Closed,
            }])
        };
        assert!(cases.change("Ash", 1, close).is_err());

        cases
            .change("Ash", 1, |_| {
                Ok(vec![Change::Closed {
                    resolution: Resolution::NoAction,
                    note: "nothing there".into(),
                }])
            })
            .unwrap();
        // Closed: talk about it, or reopen it, but nothing else.
        let add = |_: &Case| Ok(vec![Change::EvidenceAdded { evidence: chat(1) }]);
        assert!(cases.change("Ash", 1, add).is_err());
        let comment = |_: &Case| Ok(vec![Change::Comment { text: "ok".into() }]);
        cases.change("Kim", 1, comment).unwrap();
        let case = cases
            .change("Kim", 1, |_| Ok(vec![Change::Reopened]))
            .unwrap();
        assert_eq!(case.status, Status::Open);
        assert_eq!(case.resolution, None);

        // Only what took is in the log.
        let lines = std::fs::read_to_string(&path).unwrap().lines().count();
        assert_eq!(lines, 4);
        let _ = std::fs::remove_file(path);
    }

    fn appeal(issued_by: Option<&str>) -> Change {
        Change::AppealFiled {
            appeal: Appeal {
                id: 3,
                sanction: "ban".into(),
                until_ms: 1,
                issued_by: issued_by.map(str::to_owned),
                reason: Some("Macroing".into()),
                statement: "It wasn't me".into(),
                filed_ms: 1,
            },
        }
    }

    #[test]
    fn appeals_are_decided_by_someone_else_before_closing() {
        let path = temp_log("appeal");
        let cases = Cases::open(path.clone());
        let case = cases
            .create(
                "Appeals",
                "Ban appeal".into(),
                "Bob".into(),
                Priority::Normal,
                vec![appeal(Some("Mod Ash"))],
            )
            .unwrap();
        assert_eq!(cases.of_appeal(3), Some(case.id));
        assert_eq!(cases.of_appeal(4), None);
        let close = |_: &Case| {
            Ok(vec![Change::Closed {
                resolution: Resolution::NoAction,
                note: String::new(),
            }])
        };
        let decide = |_: &Case| {
            Ok(vec![Change::AppealDecided {
                decision: Decision::Upheld,
                note: "Stands.".into(),
            }])
        };
        // Not closed without a decision, nor decided by whoever banned.
        assert!(cases.change("Kim", case.id, close).is_err());
        let err = cases.change("mod_ash", case.id, decide).unwrap_err();
        assert!(err.contains("someone else"), "{err}");
        // A second appeal can't be stuck on.
        let refile = |_: &Case| Ok(vec![appeal(None)]);
        assert!(cases.change("Kim", case.id, refile).is_err());

        cases.change("Kim", case.id, decide).unwrap();
        cases.change("Kim", case.id, close).unwrap();
        let reopened = Cases::open(path.clone()).get(case.id).unwrap();
        let state = reopened.appeal.unwrap();
        assert_eq!(state.decision, Some(Decision::Upheld));
        assert_eq!(state.decided_by.as_deref(), Some("Kim"));
        assert_eq!(state.note.as_deref(), Some("Stands."));
        assert_eq!(reopened.status, Status::Closed);

        // Not an appeal: nothing to decide.
        let plain = cases
            .create("Kim", "T".into(), "Eve".into(), Priority::Low, vec![])
            .unwrap();
        assert!(cases.change("Kim", plain.id, decide).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn viewers_see_each_other_not_themselves() {
        let cases = Cases::open(temp_log("viewers"));
        assert!(cases.viewing(1, "Ash").is_empty());
        assert_eq!(cases.viewing(1, "Kim"), ["Ash"]);
        assert_eq!(cases.viewing(1, "ash"), ["Kim"]);
        assert!(cases.viewing(2, "Kim").is_empty());
    }
}
