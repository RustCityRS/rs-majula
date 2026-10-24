use crate::store::JsonFile;
use crate::templates::slug;
use crate::users::name_key;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

/// The most players one pins.
pub const MAX_PINNED: usize = 20;
/// How many recently viewed players are kept.
pub const MAX_RECENT: usize = 12;
/// The most filters one saves.
pub const MAX_FILTERS: usize = 50;
const MAX_FILTER_NAME: usize = 60;
const MAX_FILTER_PARAMS: usize = 12;
const MAX_PARAM_CHARS: usize = 200;

/// A page whose filters can be saved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterPage {
    Timeline,
    Chat,
    Audit,
}

impl FilterPage {
    /// The settings a saved filter of it may hold.
    fn params(self) -> &'static [&'static str] {
        match self {
            Self::Timeline => &["kinds"],
            Self::Chat => &["name", "node"],
            Self::Audit => &["q", "actor", "mine"],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedFilter {
    pub id: String,
    pub name: String,
    pub page: FilterPage,
    pub params: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recent {
    pub name: String,
    pub at_ms: u64,
}

/// One staff member's settings, which follow them from browser to browser.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StaffPrefs {
    #[serde(default)]
    pub pinned: Vec<String>,
    #[serde(default)]
    pub recent: Vec<Recent>,
    #[serde(default)]
    pub filters: Vec<SavedFilter>,
    #[serde(default)]
    pub utc: bool,
    #[serde(default)]
    pub seen: BTreeMap<u64, u64>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct PrefsFile {
    #[serde(default)]
    staff: BTreeMap<String, StaffPrefs>,
}

/// A change to someone's settings.
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PrefsOp {
    Pin { name: String },
    Unpin { name: String },
    Viewed { name: String },
    ForgetRecent,
    SaveFilter { filter: SavedFilter },
    RemoveFilter { id: String },
    Utc { utc: bool },
}

/// Applies `op` to `prefs` at `now_ms`. Player names are checked already.
fn apply(prefs: &mut StaffPrefs, op: PrefsOp, now_ms: u64) -> Result<(), String> {
    let same = |a: &str, b: &str| name_key(a) == name_key(b);
    match op {
        PrefsOp::Pin { name } => {
            if prefs.pinned.iter().any(|p| same(p, &name)) {
                return Ok(());
            }
            if prefs.pinned.len() >= MAX_PINNED {
                return Err(format!(
                    "At most {MAX_PINNED} pinned players; unpin one first."
                ));
            }
            prefs.pinned.push(name);
        }
        PrefsOp::Unpin { name } => prefs.pinned.retain(|p| !same(p, &name)),
        PrefsOp::Viewed { name } => {
            prefs.recent.retain(|r| !same(&r.name, &name));
            prefs.recent.insert(
                0,
                Recent {
                    name,
                    at_ms: now_ms,
                },
            );
            prefs.recent.truncate(MAX_RECENT);
        }
        PrefsOp::ForgetRecent => prefs.recent.clear(),
        PrefsOp::SaveFilter { mut filter } => {
            filter.name = filter
                .name
                .trim()
                .chars()
                .filter(|c| !c.is_control())
                .collect();
            if filter.name.is_empty() || filter.name.chars().count() > MAX_FILTER_NAME {
                return Err(format!("Name it in 1 to {MAX_FILTER_NAME} characters."));
            }
            let allowed = filter.page.params();
            filter
                .params
                .retain(|k, v| allowed.contains(&k.as_str()) && !v.trim().is_empty());
            if filter.params.len() > MAX_FILTER_PARAMS
                || filter
                    .params
                    .values()
                    .any(|v| v.chars().count() > MAX_PARAM_CHARS)
            {
                return Err("That filter is too long to save.".into());
            }
            if filter.params.is_empty() {
                return Err("Set a filter first: there's nothing to save.".into());
            }
            // Saving under a name in use replaces it.
            if let Some(old) = prefs
                .filters
                .iter_mut()
                .find(|f| f.page == filter.page && f.name.eq_ignore_ascii_case(&filter.name))
            {
                filter.id = old.id.clone();
                *old = filter;
                return Ok(());
            }
            if prefs.filters.len() >= MAX_FILTERS {
                return Err(format!("At most {MAX_FILTERS} saved filters."));
            }
            let base = slug(&filter.name);
            filter.id = base.clone();
            let mut n = 2;
            while prefs.filters.iter().any(|f| f.id == filter.id) {
                filter.id = format!("{base}-{n}");
                n += 1;
            }
            prefs.filters.push(filter);
        }
        PrefsOp::RemoveFilter { id } => prefs.filters.retain(|f| f.id != id),
        PrefsOp::Utc { utc } => prefs.utc = utc,
    }
    Ok(())
}

pub struct Prefs {
    file: JsonFile<PrefsFile>,
}

impl Prefs {
    pub fn open(path: PathBuf) -> Self {
        Self {
            file: JsonFile::open(path),
        }
    }

    pub fn of(&self, name: &str) -> StaffPrefs {
        let key = name_key(name);
        self.file
            .read(|file| file.staff.get(&key).cloned().unwrap_or_default())
    }

    /// Runs `f` on when `name` last saw each case (see [`StaffPrefs::seen`]).
    pub fn seen<R>(&self, name: &str, f: impl FnOnce(&BTreeMap<u64, u64>) -> R) -> R {
        let key = name_key(name);
        self.file.read(|file| match file.staff.get(&key) {
            Some(prefs) => f(&prefs.seen),
            None => f(&BTreeMap::new()),
        })
    }

    pub fn change(&self, name: &str, op: PrefsOp) -> Result<StaffPrefs, String> {
        self.file.update(|file| {
            let prefs = file.staff.entry(name_key(name)).or_default();
            apply(prefs, op, crate::users::now_ms())?;
            Ok(prefs.clone())
        })
    }

    /// Notes that `name` saw case `id` as it was at `updated_ms`. Only
    /// written when it's news; `settled` is asked only then.
    pub fn saw(
        &self,
        name: &str,
        id: u64,
        updated_ms: u64,
        settled: impl FnOnce() -> HashSet<u64>,
    ) {
        let known = self.seen(name, |seen| seen.get(&id).copied());
        if known.is_some_and(|ms| ms >= updated_ms) {
            return;
        }
        self.saw_all(name, [(id, updated_ms)], settled());
    }

    /// [`saw`](Self::saw) for many cases. Those `settled`, long closed
    /// without them, are forgotten, so the file doesn't grow for ever: if
    /// one is reopened, it isn't unread for them.
    pub fn saw_all(
        &self,
        name: &str,
        cases: impl IntoIterator<Item = (u64, u64)>,
        settled: HashSet<u64>,
    ) {
        let result = self.file.update(|file| {
            let seen = &mut file.staff.entry(name_key(name)).or_default().seen;
            seen.retain(|id, _| !settled.contains(id));
            for (id, updated_ms) in cases {
                let at = seen.entry(id).or_default();
                *at = (*at).max(updated_ms);
            }
            Ok(())
        });
        if let Err(e) = result {
            tracing::warn!("Couldn't note the cases {} saw: {}", name, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(name: &str, q: &str) -> SavedFilter {
        SavedFilter {
            id: String::new(),
            name: name.into(),
            page: FilterPage::Audit,
            params: BTreeMap::from([("q".into(), q.into()), ("junk".into(), "x".into())]),
        }
    }

    #[test]
    fn recent_players_are_newest_first_and_once_each() {
        let mut prefs = StaffPrefs::default();
        for (i, name) in ["Bob", "Alice", "bob"].into_iter().enumerate() {
            apply(&mut prefs, PrefsOp::Viewed { name: name.into() }, i as u64).unwrap();
        }
        let names: Vec<&str> = prefs.recent.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["bob", "Alice"]);
        for i in 0..20 {
            apply(
                &mut prefs,
                PrefsOp::Viewed {
                    name: format!("p{i}"),
                },
                0,
            )
            .unwrap();
        }
        assert_eq!(prefs.recent.len(), MAX_RECENT);
    }

    #[test]
    fn pins_are_kept_once_and_capped() {
        let mut prefs = StaffPrefs::default();
        apply(
            &mut prefs,
            PrefsOp::Pin {
                name: "Mod_Ash".into(),
            },
            0,
        )
        .unwrap();
        apply(
            &mut prefs,
            PrefsOp::Pin {
                name: "mod ash".into(),
            },
            0,
        )
        .unwrap();
        assert_eq!(prefs.pinned, ["Mod_Ash"]);
        apply(
            &mut prefs,
            PrefsOp::Unpin {
                name: "MOD ASH".into(),
            },
            0,
        )
        .unwrap();
        assert!(prefs.pinned.is_empty());
        for i in 0..MAX_PINNED {
            apply(
                &mut prefs,
                PrefsOp::Pin {
                    name: format!("p{i}"),
                },
                0,
            )
            .unwrap();
        }
        assert!(
            apply(
                &mut prefs,
                PrefsOp::Pin {
                    name: "one more".into()
                },
                0
            )
            .is_err()
        );
    }

    #[test]
    fn filters_keep_only_their_page_settings_and_replace_by_name() {
        let mut prefs = StaffPrefs::default();
        apply(
            &mut prefs,
            PrefsOp::SaveFilter {
                filter: filter(" Bans ", "ban"),
            },
            0,
        )
        .unwrap();
        assert_eq!(prefs.filters[0].id, "bans");
        assert_eq!(prefs.filters[0].name, "Bans");
        assert_eq!(prefs.filters[0].params.len(), 1);
        apply(
            &mut prefs,
            PrefsOp::SaveFilter {
                filter: filter("bans", "unban"),
            },
            0,
        )
        .unwrap();
        assert_eq!(prefs.filters.len(), 1);
        assert_eq!(prefs.filters[0].params["q"], "unban");
        assert!(
            apply(
                &mut prefs,
                PrefsOp::SaveFilter {
                    filter: filter("Empty", "  ")
                },
                0
            )
            .is_err()
        );
        apply(&mut prefs, PrefsOp::RemoveFilter { id: "bans".into() }, 0).unwrap();
        assert!(prefs.filters.is_empty());
    }
}
