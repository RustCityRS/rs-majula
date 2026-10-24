use crate::cases::{MAX_DECISION_NOTE, MAX_TEXT, Resolution};
use crate::store::JsonFile;
use crate::templates::slug;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_CANNED: usize = 100;
const MAX_NAME_CHARS: usize = 60;

/// Where a canned text goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Use {
    Close,
    Upheld,
    Overturned,
}

impl Use {
    fn max_chars(self) -> usize {
        match self {
            Self::Close => MAX_TEXT,
            Self::Upheld | Self::Overturned => MAX_DECISION_NOTE,
        }
    }
}

/// A saved closing note or appeal decision. `{player}`, `{sanction}` and
/// `{case}` in its text are filled in where it's used.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Canned {
    pub id: String,
    pub name: String,
    #[serde(rename = "use")]
    pub used_for: Use,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<Resolution>,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CannedFile {
    pub canned: Vec<Canned>,
}

impl Default for CannedFile {
    /// What a console starts with, until an admin changes it.
    fn default() -> Self {
        let c = |id: &str, name: &str, used_for, resolution, text: &str| Canned {
            id: id.to_owned(),
            name: name.to_owned(),
            used_for,
            resolution,
            text: text.to_owned(),
        };
        Self {
            canned: vec![
                c(
                    "nothing-found",
                    "Nothing found",
                    Use::Close,
                    Some(Resolution::NoAction),
                    "Went through the evidence and found nothing against the rules. No action taken.",
                ),
                c(
                    "not-enough",
                    "Not enough to go on",
                    Use::Close,
                    Some(Resolution::NoAction),
                    "The evidence doesn't show enough to act on. Reopen this if more comes in.",
                ),
                c(
                    "sanctioned",
                    "Confirmed and sanctioned",
                    Use::Close,
                    Some(Resolution::Actioned),
                    "Confirmed from the evidence; {player} was sanctioned (see the timeline).",
                ),
                c(
                    "duplicate",
                    "Duplicate",
                    Use::Close,
                    Some(Resolution::Duplicate),
                    "Covered by case #",
                ),
                c(
                    "logs-confirm",
                    "Our logs confirm it",
                    Use::Upheld,
                    None,
                    "We reviewed your appeal against our logs, and they confirm the offence. The {sanction} stands.",
                ),
                c(
                    "mistake",
                    "It was a mistake",
                    Use::Overturned,
                    None,
                    "We looked again and agree the {sanction} was a mistake. It has been lifted; sorry for the trouble.",
                ),
                c(
                    "second-chance",
                    "Second chance",
                    Use::Overturned,
                    None,
                    "We've decided to lift the {sanction} early. Please keep to the rules from here on.",
                ),
            ],
        }
    }
}

/// Checks texts an admin saved, filling in ids for new ones.
pub fn checked(canned: Vec<Canned>) -> Result<Vec<Canned>, String> {
    if canned.len() > MAX_CANNED {
        return Err(format!("At most {MAX_CANNED} saved texts."));
    }
    let mut out: Vec<Canned> = Vec::with_capacity(canned.len());
    for mut c in canned {
        c.name = c.name.trim().to_owned();
        c.text = c.text.trim().replace("\r\n", "\n");
        if c.name.is_empty() || c.name.chars().count() > MAX_NAME_CHARS {
            return Err(format!("Names are 1 to {MAX_NAME_CHARS} characters."));
        }
        let max = c.used_for.max_chars();
        if c.text.is_empty() || c.text.chars().count() > max {
            return Err(format!(
                "\"{}\": the text is 1 to {max} characters.",
                c.name
            ));
        }
        if c.used_for != Use::Close {
            c.resolution = None;
        }
        c.id = c.id.trim().to_owned();
        if c.id.is_empty() {
            c.id = slug(&c.name);
        }
        let base = c.id.clone();
        let mut n = 2;
        while out.iter().any(|o| o.id == c.id) {
            c.id = format!("{base}-{n}");
            n += 1;
        }
        if out
            .iter()
            .any(|o| o.used_for == c.used_for && o.name.eq_ignore_ascii_case(&c.name))
        {
            return Err(format!("Two of the same kind are named \"{}\".", c.name));
        }
        out.push(c);
    }
    Ok(out)
}

pub struct CannedTexts {
    file: JsonFile<CannedFile>,
}

impl CannedTexts {
    pub fn open(path: PathBuf) -> Self {
        Self {
            file: JsonFile::open(path),
        }
    }

    pub fn all(&self) -> Vec<Canned> {
        self.file.get().canned
    }

    pub fn save(&self, canned: Vec<Canned>) -> Result<Vec<Canned>, String> {
        let canned = checked(canned)?;
        self.file.update(|file| {
            file.canned = canned.clone();
            Ok(canned)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid() {
        let defaults = CannedFile::default().canned;
        let checked = checked(defaults.clone()).unwrap();
        assert_eq!(checked.len(), defaults.len());
    }

    #[test]
    fn saving_checks_and_gives_ids() {
        let mut c = CannedFile::default().canned[4].clone();
        c.id = String::new();
        c.name = " Logs say so ".into();
        c.resolution = Some(Resolution::Actioned);
        let saved = checked(vec![c.clone()]).unwrap();
        assert_eq!(saved[0].id, "logs-say-so");
        // Only closing notes pick an outcome.
        assert_eq!(saved[0].resolution, None);
        assert!(checked(vec![c.clone(), c.clone()]).is_err());
        // The same name is fine for another kind.
        let mut other = c.clone();
        other.used_for = Use::Close;
        assert_eq!(checked(vec![c.clone(), other]).unwrap().len(), 2);
        let mut long = c;
        long.text = "x".repeat(MAX_DECISION_NOTE + 1);
        assert!(checked(vec![long]).is_err());
    }
}
