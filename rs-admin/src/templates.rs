use crate::store::JsonFile;
use crate::users::{name_key, now_ms};
use rs_admin_api::ModAction;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The longest mute or ban: 10 years, as in game.
const MAX_HOURS: i32 = 87_600;
const MAX_TEMPLATES: usize = 50;
const MAX_STEPS: usize = 10;
const MAX_NAME_CHARS: usize = 60;
const MAX_DESCRIPTION_CHARS: usize = 300;
/// How many offences are kept; the oldest go first.
const MAX_OFFENCES: usize = 20_000;

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

fn slug(name: &str) -> String {
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

pub struct Templates {
    file: JsonFile<TemplateFile>,
    offences: JsonFile<Vec<Offence>>,
}

impl Templates {
    pub fn open(templates: PathBuf, offences: PathBuf) -> Self {
        Self {
            file: JsonFile::open(templates),
            offences: JsonFile::open(offences),
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
        standing(&self.offences.get(), player, template, now_ms())
    }

    /// Where `player` stands on every template.
    pub fn standings(&self, player: &str) -> Vec<Standing> {
        let offences = self.offences.get();
        let now = now_ms();
        self.all()
            .iter()
            .map(|t| standing(&offences, player, t, now))
            .collect()
    }

    pub fn record(&self, offence: Offence) -> Result<(), String> {
        self.offences.update(|all| {
            all.push(offence);
            let excess = all.len().saturating_sub(MAX_OFFENCES);
            all.drain(..excess);
            Ok(())
        })
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
