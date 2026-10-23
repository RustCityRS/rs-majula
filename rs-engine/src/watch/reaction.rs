use serde_json::{Value, json};
use std::collections::VecDeque;

/// Only reactions to something that happened while the player had done
/// nothing for this long count: a player clicking away all the while would
/// seem to react at once by chance.
const QUIET_MS: u32 = 1_000;
/// A longer wait than this isn't a reaction.
const MAX_MS: u32 = 10_000;
/// Faster than people react, network and all.
const FAST_MS: u32 = 180;
/// Reactions kept.
const KEPT: usize = 200;
/// Too few to judge by.
const MIN_REACTIONS: usize = 20;

#[derive(Default)]
pub struct Reactions {
    gone_ms: Option<u32>,
    last_action_ms: Option<u32>,
    times: VecDeque<u32>,
}

impl Reactions {
    /// What the player was acting on went away, `ms` into the session.
    pub fn gone(&mut self, ms: u32) {
        self.gone_ms = Some(ms);
    }

    /// An action, `ms` into the session; with the time the obj it's on
    /// appeared, if it's on one that did lately.
    pub fn action(&mut self, ms: u32, appeared_ms: Option<u32>) {
        let event = appeared_ms
            .into_iter()
            .chain(self.gone_ms.take())
            .filter(|&e| e <= ms)
            .max();
        if let Some(event) = event
            && self.last_action_ms.is_none_or(|a| a + QUIET_MS <= event)
            && ms - event <= MAX_MS
        {
            if self.times.len() == KEPT {
                self.times.pop_front();
            }
            self.times.push_back(ms - event);
        }
        self.last_action_ms = Some(self.last_action_ms.map_or(ms, |a| a.max(ms)));
    }

    /// The share of reactions faster than people react.
    fn fast_share(&self) -> f64 {
        let fast = self.times.iter().filter(|&&t| t < FAST_MS).count();
        fast as f64 / self.times.len().max(1) as f64
    }

    /// How even the reactions are: their standard deviation over their
    /// mean.
    fn spread(&self) -> Option<f64> {
        super::anticheat::cv(self.times.iter().map(|&t| f64::from(t)))
    }

    pub fn report(&self, flags: &mut Vec<(&'static str, i16)>) -> ReactionStats {
        let enough = self.times.len() >= MIN_REACTIONS;
        let fast_share = self.fast_share();
        if enough && fast_share >= 0.5 {
            flags.push(("instant_reactions", 25));
        }
        let spread = self.spread();
        if enough && spread.is_some_and(|cv| cv < 0.15) {
            flags.push(("even_reactions", 20));
        }
        let mut kept = [0; KEPT];
        let times = &mut kept[..self.times.len()];
        for (to, &t) in times.iter_mut().zip(&self.times) {
            *to = t;
        }
        let mid = times.len() >> 1;
        let median_ms = (!times.is_empty()).then(|| *times.select_nth_unstable(mid).1);
        ReactionStats {
            count: self.times.len(),
            median_ms,
            fast_share,
            spread,
        }
    }
}

/// What [`Reactions::report`] found, for the report's stats.
pub struct ReactionStats {
    count: usize,
    median_ms: Option<u32>,
    fast_share: f64,
    spread: Option<f64>,
}

impl ReactionStats {
    pub fn json(&self) -> Value {
        json!({
            "count": self.count,
            "medianMs": self.median_ms,
            "fastShare": (self.fast_share * 100.0).round() / 100.0,
            "spread": self.spread.map(|cv| (cv * 100.0).round() / 100.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(r: &Reactions) -> Vec<&'static str> {
        let mut flags = Vec::new();
        r.report(&mut flags);
        flags.into_iter().map(|(f, _)| f).collect()
    }

    #[test]
    fn people_react_slowly_and_unevenly() {
        let mut r = Reactions::default();
        let mut ms = 0;
        for i in 0..30u32 {
            ms += 5_000;
            r.gone(ms);
            r.action(ms + 300 + (i * 337) % 900, None);
            ms += 2_000;
        }
        assert!(flags(&r).is_empty(), "{:?}", flags(&r));
    }

    #[test]
    fn bots_react_at_once_or_like_clockwork() {
        let mut instant = Reactions::default();
        let mut even = Reactions::default();
        for i in 0..30u32 {
            let ms = i * 10_000 + 5_000;
            instant.gone(ms);
            instant.action(ms + 20 + (i * 37) % 150, None);
            // An obj appearing, then taken.
            even.action(ms - 3_000, None);
            even.action(ms + 650 + i % 20, Some(ms));
        }
        assert_eq!(flags(&instant), ["instant_reactions"]);
        assert_eq!(flags(&even), ["even_reactions"]);
    }

    #[test]
    fn clicking_away_all_the_while_is_no_reaction() {
        let mut r = Reactions::default();
        let mut ms = 0;
        for _ in 0..40 {
            // Clicking every 300ms, the rock going between two clicks.
            r.action(ms, None);
            r.gone(ms + 200);
            r.action(ms + 300, None);
            ms += 600;
        }
        assert_eq!(r.times.len(), 0);
    }
}
