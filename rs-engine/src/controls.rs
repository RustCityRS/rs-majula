use rs_entity::StaffModLevel;
use rs_pack::cache::script::ScriptProvider;
use rustc_hash::FxHashSet;
use std::time::Duration;

/// What a player is told when they try something switched off.
pub const DISABLED_MESSAGE: &str = "That's switched off for now. Please try again later.";

/// The most kill switch patterns a world takes.
pub const MAX_SWITCHES: usize = 64;

#[derive(Default)]
pub struct Controls {
    patterns: Vec<String>,
    disabled: FxHashSet<i32>,
    pub min_account_age_hours: u32,
}

impl Controls {
    /// Switches off the scripts matching `patterns` (and only those).
    /// Returns how many scripts each pattern matches.
    pub fn set_switches(&mut self, patterns: Vec<String>, scripts: &ScriptProvider) -> Vec<usize> {
        self.patterns = patterns;
        self.refresh(scripts)
    }

    /// Matches the patterns against `scripts` again, after a reload.
    pub fn refresh(&mut self, scripts: &ScriptProvider) -> Vec<usize> {
        self.disabled.clear();
        self.patterns
            .iter()
            .map(|pattern| {
                let mut matched = 0;
                for (name, &id) in &scripts.names {
                    if glob(pattern, name) {
                        matched += 1;
                        self.disabled.insert(id);
                    }
                }
                matched
            })
            .collect()
    }

    /// How many of `scripts` `pattern` matches.
    pub fn matches(pattern: &str, scripts: &ScriptProvider) -> usize {
        scripts
            .names
            .keys()
            .filter(|name| glob(pattern, name))
            .count()
    }

    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }

    /// Whether players are kept from starting the script `id`.
    #[inline]
    pub fn is_disabled(&self, id: i32) -> bool {
        !self.disabled.is_empty() && self.disabled.contains(&id)
    }

    /// Whether the lockdown lets in an account of `staff` level registered
    /// `age` ago.
    pub fn admits(&self, staff: StaffModLevel, age: Duration) -> bool {
        self.min_account_age_hours == 0
            || staff != StaffModLevel::Normal
            || age >= Duration::from_secs(u64::from(self.min_account_age_hours) * 3600)
    }
}

/// Whether `name` matches `pattern`, where `*` stands for any run of
/// characters (none included) and everything else for itself.
pub fn glob(pattern: &str, name: &str) -> bool {
    let (p, n) = (pattern.as_bytes(), name.as_bytes());
    let (mut pi, mut ni) = (0, 0);
    // Where the last `*` was, and how much of the name it has taken.
    let mut star: Option<(usize, usize)> = None;
    while ni < n.len() {
        if pi < p.len() && p[pi] == b'*' {
            star = Some((pi, ni));
            pi += 1;
        } else if pi < p.len() && p[pi] == n[ni] {
            pi += 1;
            ni += 1;
        } else if let Some((sp, sn)) = star {
            pi = sp + 1;
            ni = sn + 1;
            star = Some((sp, sn + 1));
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == b'*')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob("[opplayer4,_]", "[opplayer4,_]"));
        assert!(!glob("[opplayer4,_]", "[opplayer4,_x]"));
        assert!(glob("[opheld5,*]", "[opheld5,_]"));
        assert!(glob("[opheld5,*]", "[opheld5,saradomin_cape]"));
        assert!(!glob("[opheld5,*]", "[opheld4,_]"));
        assert!(glob("[inv_button*,trade*]", "[inv_button3,trademain:inv]"));
        assert!(glob("*", "anything"));
        assert!(glob("a*b*c", "abc"));
        assert!(glob("a*b*c", "aXXbYYc"));
        assert!(!glob("a*b*c", "aXXbYY"));
        assert!(!glob("", "a"));
    }

    #[test]
    fn lockdown_spares_staff_and_old_accounts() {
        let hour = Duration::from_secs(3600);
        let mut controls = Controls::default();
        assert!(controls.admits(StaffModLevel::Normal, Duration::ZERO));
        controls.min_account_age_hours = 24;
        assert!(!controls.admits(StaffModLevel::Normal, hour * 23));
        assert!(controls.admits(StaffModLevel::Normal, hour * 24));
        assert!(controls.admits(StaffModLevel::PlayerModerator, Duration::ZERO));
    }
}
