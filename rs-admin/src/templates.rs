use crate::store::JsonFile;
use crate::users::{name_key, now_ms};
use rs_admin_api::ModAction;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{ErrorKind, Write};
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use tracing::{error, warn};

/// The longest mute or ban: 10 years, as in game.
const MAX_HOURS: i32 = 87_600;
const MAX_TEMPLATES: usize = 50;
const MAX_STEPS: usize = 10;
const MAX_NAME_CHARS: usize = 60;
const MAX_DESCRIPTION_CHARS: usize = 300;
/// How many offences are kept at least; the oldest go first.
const MAX_OFFENCES: usize = 20_000;
/// How many more are kept before the oldest go, all at once: the log is
/// only rewritten then, and appended to otherwise.
const SPARE_OFFENCES: usize = MAX_OFFENCES >> 1;

/// Offences older than this stop counting, unless a template says otherwise.
const DEFAULT_EXPIRE_DAYS: u32 = 365;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepAction {
    Mute,
    Ban,
}

impl StepAction {
    pub fn mod_action(self) -> ModAction {
        match self {
            Self::Mute => ModAction::Mute,
            Self::Ban => ModAction::Ban,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub action: StepAction,
    pub hours: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The first offence gets the first step, and so on; past the last,
    /// the last again.
    pub steps: Vec<Step>,
    /// Earlier offences older than this many days don't count.
    pub expire_days: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemplateFile {
    pub templates: Vec<Template>,
}

const fn mute(hours: i32) -> Step {
    Step {
        action: StepAction::Mute,
        hours,
    }
}

const fn ban(hours: i32) -> Step {
    Step {
        action: StepAction::Ban,
        hours,
    }
}

const DAY: i32 = 24;
const WEEK: i32 = 168;
const PERMANENT: i32 = MAX_HOURS;

impl Default for TemplateFile {
    /// What a console starts with, until an admin changes it.
    fn default() -> Self {
        let t = |id: &str, name: &str, description: &str, steps: &[Step]| Template {
            id: id.to_owned(),
            name: name.to_owned(),
            description: description.to_owned(),
            steps: steps.to_vec(),
            expire_days: DEFAULT_EXPIRE_DAYS,
        };
        Self {
            templates: vec![
                t(
                    "offensive-language",
                    "Offensive language",
                    "Abuse, slurs or threats in chat.",
                    &[mute(2 * DAY), mute(WEEK), mute(30 * DAY), ban(30 * DAY)],
                ),
                t(
                    "spam",
                    "Spamming",
                    "Flooding chat or advertising.",
                    &[mute(DAY), mute(2 * DAY), mute(WEEK)],
                ),
                t(
                    "macroing",
                    "Macroing",
                    "Bots, autoclickers or other third-party automation.",
                    &[ban(2 * DAY), ban(2 * WEEK), ban(PERMANENT)],
                ),
                t(
                    "scamming",
                    "Scamming",
                    "Item or password scamming.",
                    &[ban(2 * DAY), ban(2 * WEEK), ban(PERMANENT)],
                ),
                t(
                    "bug-abuse",
                    "Bug abuse",
                    "Knowingly using a bug for gain.",
                    &[ban(WEEK), ban(30 * DAY), ban(PERMANENT)],
                ),
                t(
                    "real-world-trading",
                    "Real-world trading",
                    "Buying or selling game items or accounts for real money.",
                    &[ban(2 * WEEK), ban(PERMANENT)],
                ),
                t(
                    "staff-impersonation",
                    "Staff impersonation",
                    "Pretending to be staff.",
                    &[mute(2 * DAY), ban(WEEK), ban(PERMANENT)],
                ),
            ],
        }
    }
}

/// One time a template was applied.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Offence {
    pub at_ms: u64,
    pub player: String,
    pub template: String,
    /// Which step it got (0 is the first).
    pub step: usize,
    pub by: String,
}

/// Where a player stands on a template's ladder.
#[derive(Clone, Debug, Serialize)]
pub struct Standing {
    pub template: String,
    /// Earlier offences that still count.
    pub offences: Vec<Offence>,
    /// The step the next one gets.
    pub next: usize,
    pub step: Step,
}

/// Checks templates an admin saved, filling in ids for new ones.
pub fn checked(templates: Vec<Template>) -> Result<Vec<Template>, String> {
    if templates.len() > MAX_TEMPLATES {
        return Err(format!("At most {MAX_TEMPLATES} templates."));
    }
    let mut out: Vec<Template> = Vec::with_capacity(templates.len());
    for mut t in templates {
        t.name = t.name.trim().to_owned();
        t.description = t.description.trim().to_owned();
        if t.name.is_empty() || t.name.chars().count() > MAX_NAME_CHARS {
            return Err(format!(
                "Template names are 1 to {MAX_NAME_CHARS} characters."
            ));
        }
        if t.description.chars().count() > MAX_DESCRIPTION_CHARS {
            return Err(format!(
                "\"{}\": descriptions are at most {MAX_DESCRIPTION_CHARS} characters.",
                t.name
            ));
        }
        if t.steps.is_empty() || t.steps.len() > MAX_STEPS {
            return Err(format!("\"{}\" needs 1 to {MAX_STEPS} steps.", t.name));
        }
        if let Some(step) = t.steps.iter().find(|s| !(1..=MAX_HOURS).contains(&s.hours)) {
            return Err(format!(
                "\"{}\": a step of {} hours; lengths are 1 to {MAX_HOURS} hours.",
                t.name, step.hours
            ));
        }
        if !(1..=3650).contains(&t.expire_days) {
            return Err(format!(
                "\"{}\": offences count for 1 to 3650 days.",
                t.name
            ));
        }
        // Ids stay put once given, so offences keep pointing at their
        // template through renames.
        t.id = t.id.trim().to_owned();
        if t.id.is_empty() {
            t.id = slug(&t.name);
        }
        let base = t.id.clone();
        let mut n = 2;
        while out.iter().any(|o| o.id == t.id) {
            t.id = format!("{base}-{n}");
            n += 1;
        }
        if out.iter().any(|o| o.name.eq_ignore_ascii_case(&t.name)) {
            return Err(format!("Two templates are named \"{}\".", t.name));
        }
        out.push(t);
    }
    Ok(out)
}

pub fn slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-').to_owned();
    if slug.is_empty() {
        "template".to_owned()
    } else {
        slug
    }
}

/// The offences given, as a log of a JSON line each.
struct Offences {
    path: PathBuf,
    inner: Mutex<OffenceLog>,
}

struct OffenceLog {
    /// What the log holds, oldest first.
    all: Vec<Offence>,
    /// It couldn't be read, so it isn't added to until it's fixed.
    broken: bool,
}

impl Offences {
    /// Opens the log at `path`, starting it from the whole-file JSON the
    /// console kept before (beside it, `.json`) if there's none yet.
    fn open(path: PathBuf) -> Self {
        let (all, broken) = match fs::read_to_string(&path) {
            Ok(text) => (lines(&path, &text), false),
            Err(e) if e.kind() == ErrorKind::NotFound => match from_json(&path) {
                Ok(all) => (all, false),
                Err(e) => {
                    error!("{e}; offences are refused until it's fixed");
                    (Vec::new(), true)
                }
            },
            Err(e) => {
                error!(
                    "Couldn't read {}, so offences are refused until it's fixed: {e}",
                    path.display()
                );
                (Vec::new(), true)
            }
        };
        Self {
            path,
            inner: Mutex::new(OffenceLog { all, broken }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, OffenceLog> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Adds `offences` to the log in one write, so they land together or
    /// not at all.
    fn record(&self, offences: Vec<Offence>) -> Result<(), String> {
        let mut log = self.lock();
        if log.broken {
            return Err(format!(
                "{} is unreadable; fix or remove it first.",
                self.path.display()
            ));
        }
        let saved = if log.all.len() + offences.len() > MAX_OFFENCES + SPARE_OFFENCES {
            let mut all = log.all.clone();
            all.extend(offences);
            all.drain(..all.len() - MAX_OFFENCES);
            rewrite(&self.path, &all).map(|()| all)
        } else {
            append(&self.path, &offences).map(|()| {
                let mut all = mem::take(&mut log.all);
                all.extend(offences);
                all
            })
        };
        match saved {
            Ok(all) => {
                log.all = all;
                Ok(())
            }
            Err(e) => {
                warn!("Couldn't write {}: {e}", self.path.display());
                Err("The console couldn't save that.".to_owned())
            }
        }
    }
}

/// The offences in log `text`, skipping lines that don't read.
fn lines(path: &Path, text: &str) -> Vec<Offence> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| match serde_json::from_str(line) {
            Ok(offence) => Some(offence),
            Err(e) => {
                warn!("Skipping an unreadable line of {}: {e}", path.display());
                None
            }
        })
        .collect()
}

/// The offences kept the old way beside log `path`, written into it.
fn from_json(path: &Path) -> Result<Vec<Offence>, String> {
    let old = path.with_extension("json");
    let all: Vec<Offence> = match fs::read_to_string(&old) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| format!("{} is unreadable: {e}", old.display()))?,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Couldn't read {}: {e}", old.display())),
    };
    rewrite(path, &all).map_err(|e| format!("Couldn't write {}: {e}", path.display()))?;
    Ok(all)
}

fn to_lines(offences: &[Offence]) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::new();
    for offence in offences {
        serde_json::to_writer(&mut out, offence)?;
        out.push(b'\n');
    }
    Ok(out)
}

fn append(path: &Path, offences: &[Offence]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(&to_lines(offences)?)
}

/// Writes beside the log and renames over it, so a crash never leaves it
/// half written.
fn rewrite(path: &Path, offences: &[Offence]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("jsonl.tmp");
    fs::write(&tmp, to_lines(offences)?)?;
    fs::rename(&tmp, path)
}

pub struct Templates {
    file: JsonFile<TemplateFile>,
    offences: Offences,
}

impl Templates {
    /// `offences` is the log of offences given (see [`Offences::open`]).
    pub fn open(templates: PathBuf, offences: PathBuf) -> Self {
        Self {
            file: JsonFile::open(templates),
            offences: Offences::open(offences),
        }
    }

    pub fn all(&self) -> Vec<Template> {
        self.file.get().templates
    }

    pub fn get(&self, id: &str) -> Option<Template> {
        self.all().into_iter().find(|t| t.id == id)
    }

    pub fn save(&self, templates: Vec<Template>) -> Result<Vec<Template>, String> {
        let templates = checked(templates)?;
        self.file.update(|file| {
            file.templates = templates.clone();
            Ok(templates)
        })
    }

    /// Where `player` stands on `template` now.
    pub fn standing(&self, player: &str, template: &Template) -> Standing {
        standing(&self.offences.lock().all, player, template, now_ms())
    }

    /// Where `player` stands on every template.
    pub fn standings(&self, player: &str) -> Vec<Standing> {
        let templates = self.all();
        let now = now_ms();
        let log = self.offences.lock();
        templates
            .iter()
            .map(|t| standing(&log.all, player, t, now))
            .collect()
    }

    /// Records `offences`, given together (to linked accounts too).
    pub fn record(&self, offences: Vec<Offence>) -> Result<(), String> {
        if offences.is_empty() {
            return Ok(());
        }
        self.offences.record(offences)
    }
}

fn standing(offences: &[Offence], player: &str, template: &Template, now: u64) -> Standing {
    let key = name_key(player);
    let since = now.saturating_sub(u64::from(template.expire_days) * 24 * 3600 * 1000);
    let offences: Vec<Offence> = offences
        .iter()
        .filter(|o| o.template == template.id && o.at_ms >= since && name_key(&o.player) == key)
        .cloned()
        .collect();
    let next = offences.len().min(template.steps.len() - 1);
    Standing {
        template: template.id.clone(),
        next,
        step: template.steps[next],
        offences,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offence(player: &str, template: &str, at_ms: u64) -> Offence {
        Offence {
            at_ms,
            player: player.to_owned(),
            template: template.to_owned(),
            step: 0,
            by: "mod".to_owned(),
        }
    }

    #[test]
    fn offences_are_appended_and_trimmed_now_and_then() {
        let dir = std::env::temp_dir().join(format!("rs-admin-offences-{}", now_ms()));
        let path = dir.join("offences.jsonl");
        fs::create_dir_all(&dir).unwrap();
        // The whole-file JSON from before is read in.
        let old = vec![offence("Bot", "macroing", 1)];
        fs::write(
            dir.join("offences.json"),
            serde_json::to_string(&old).unwrap(),
        )
        .unwrap();
        let log = Offences::open(path.clone());
        assert_eq!(log.lock().all.len(), 1);
        log.record(vec![
            offence("Alt", "macroing", 2),
            offence("Pal", "macroing", 2),
        ])
        .unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap().lines().count(), 3);
        let reopened = Offences::open(path.clone());
        let players: Vec<String> = reopened
            .lock()
            .all
            .iter()
            .map(|o| o.player.clone())
            .collect();
        assert_eq!(players, ["Bot", "Alt", "Pal"]);

        let many = (0..MAX_OFFENCES + SPARE_OFFENCES - 3)
            .map(|i| offence("Spam", "spam", i as u64))
            .collect();
        reopened.record(many).unwrap();
        assert_eq!(reopened.lock().all.len(), MAX_OFFENCES + SPARE_OFFENCES);
        // One more and the oldest go.
        reopened.record(vec![offence("Last", "spam", 0)]).unwrap();
        assert_eq!(reopened.lock().all.len(), MAX_OFFENCES);
        let read = Offences::open(path);
        let all = &read.lock().all;
        assert_eq!(all.len(), MAX_OFFENCES);
        assert_eq!(all[all.len() - 1].player, "Last");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn the_defaults_are_valid() {
        let defaults = TemplateFile::default().templates;
        let checked = checked(defaults.clone()).unwrap();
        assert_eq!(checked.len(), defaults.len());
        for (a, b) in checked.iter().zip(&defaults) {
            assert_eq!(a.id, b.id);
        }
    }

    #[test]
    fn ladders_climb_and_stop_at_the_top() {
        let t = TemplateFile::default()
            .templates
            .into_iter()
            .find(|t| t.id == "macroing")
            .unwrap();
        let day = 24 * 3600 * 1000;
        let now = 400 * day;
        let mut log = vec![offence("Other", "macroing", now)];
        let s = standing(&log, "Bot_1", &t, now);
        assert_eq!((s.next, s.step), (0, ban(2 * DAY)));
        log.push(offence("bot 1", "macroing", now - day));
        assert_eq!(standing(&log, "Bot_1", &t, now).step, ban(2 * WEEK));
        log.push(offence("BOT_1", "macroing", now - 2 * day));
        log.push(offence("bot_1", "macroing", now - 3 * day));
        assert_eq!(standing(&log, "Bot_1", &t, now).step, ban(PERMANENT));
        // Other templates and expired offences don't count.
        let fresh = vec![
            offence("bot_1", "spam", now),
            offence("bot_1", "macroing", now - 366 * day),
        ];
        assert_eq!(standing(&fresh, "Bot_1", &t, now).next, 0);
    }

    #[test]
    fn saving_checks_and_gives_ids() {
        let mut t = TemplateFile::default().templates[0].clone();
        t.id = String::new();
        t.name = "  Rude & crude!  ".to_owned();
        let mut dup = t.clone();
        dup.name = "Another".to_owned();
        let saved = checked(vec![t.clone(), dup]).unwrap();
        assert_eq!(saved[0].id, "rude-crude");
        assert_eq!(saved[0].name, "Rude & crude!");
        assert_eq!(saved[1].id, "another");

        let mut bad = t.clone();
        bad.steps.clear();
        assert!(checked(vec![bad]).is_err());
        let mut bad = t.clone();
        bad.steps[0].hours = 0;
        assert!(checked(vec![bad]).is_err());
        assert!(checked(vec![t.clone(), t]).is_err());
    }
}
