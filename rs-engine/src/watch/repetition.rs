use rustc_hash::FxHasher;
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};

/// The actions kept.
const KEPT: usize = 400;
/// How long a run must be to count as a repeat.
const RUN: usize = 4;
/// Too few to judge by...
const MIN_ACTIONS: usize = 150;
/// ...and a sequence with fewer walks than this share isn't laps: standing
/// at a rock or a range, people repeat themselves too.
const MIN_WALK_SHARE: f64 = 0.2;
/// The share of runs seen before that flags.
const REPEATED: f64 = 0.85;

#[derive(Default)]
pub struct Repetition {
    actions: VecDeque<(u64, bool)>,
}

impl Repetition {
    /// An action, as anything that tells it apart (what it's on, where).
    pub fn action(&mut self, what: impl Hash) {
        self.push(what, false);
    }

    /// A walk to `(x, z)`.
    pub fn walk(&mut self, x: u16, z: u16) {
        self.push((x, z), true);
    }

    fn push(&mut self, what: impl Hash, walk: bool) {
        let mut hasher = FxHasher::default();
        (walk, what).hash(&mut hasher);
        if self.actions.len() == KEPT {
            self.actions.pop_front();
        }
        self.actions.push_back((hasher.finish(), walk));
    }

    /// The share of runs of [`RUN`] actions seen earlier in the sequence:
    /// each run hashed to one number, and those sorted in place, a repeat
    /// being every run past the first of its kind.
    fn repeated(&self) -> f64 {
        let n = (self.actions.len() + 1).saturating_sub(RUN);
        let mut kept = [0u64; KEPT];
        let runs = &mut kept[..n];
        for (i, run) in runs.iter_mut().enumerate() {
            let mut hasher = FxHasher::default();
            for k in 0..RUN {
                hasher.write_u64(self.actions[i + k].0);
            }
            *run = hasher.finish();
        }
        runs.sort_unstable();
        let repeats = runs.windows(2).filter(|w| w[0] == w[1]).count();
        repeats as f64 / n.max(1) as f64
    }

    fn walk_share(&self) -> f64 {
        let walks = self.actions.iter().filter(|&&(_, w)| w).count();
        walks as f64 / self.actions.len().max(1) as f64
    }

    pub fn report(&self, flags: &mut Vec<(&'static str, i16)>) -> RepetitionStats {
        let repeated = self.repeated();
        let walk_share = self.walk_share();
        if self.actions.len() >= MIN_ACTIONS && walk_share >= MIN_WALK_SHARE && repeated >= REPEATED
        {
            flags.push(("repeated_laps", 25));
        }
        RepetitionStats {
            actions: self.actions.len(),
            repeated,
            walk_share,
        }
    }
}

/// What [`Repetition::report`] found, for the report's stats.
pub struct RepetitionStats {
    actions: usize,
    repeated: f64,
    walk_share: f64,
}

impl RepetitionStats {
    pub fn json(&self) -> Value {
        json!({
            "actions": self.actions,
            "repeated": (self.repeated * 100.0).round() / 100.0,
            "walkShare": (self.walk_share * 100.0).round() / 100.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(r: &Repetition) -> Vec<&'static str> {
        let mut flags = Vec::new();
        r.report(&mut flags);
        flags.into_iter().map(|(f, _)| f).collect()
    }

    /// Laps of: walk to the bank, booth, deposit, walk to the rocks, mine
    /// three times. Each walk lands `drift` tiles about.
    fn laps(drift: u32) -> Repetition {
        let mut r = Repetition::default();
        let mut seed = 1u32;
        let mut off = || {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            ((seed >> 16) % (2 * drift + 1)) as u16
        };
        for _ in 0..40 {
            r.walk(3250 + off(), 3420 + off());
            r.action(("booth", 3256, 3420));
            r.action(("deposit", 5));
            r.walk(3285 + off(), 3365 + off());
            for rock in [1, 2, 1] {
                r.action(("rock", rock));
            }
        }
        r
    }

    #[test]
    fn exact_laps_are_flagged() {
        assert_eq!(flags(&laps(0)), ["repeated_laps"]);
    }

    #[test]
    fn drifting_laps_pass() {
        let r = laps(3);
        assert!(flags(&r).is_empty(), "{}", r.repeated());
    }

    #[test]
    fn standing_still_passes() {
        let mut r = Repetition::default();
        for _ in 0..100 {
            for rock in [1, 2, 3] {
                r.action(("rock", rock));
            }
        }
        assert!(flags(&r).is_empty());
    }
}
