use serde_json::{Map, Value, json};

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 24 * HOUR_MS;
/// How far back the account's play is looked at.
pub const WEEK_MS: i64 = 7 * DAY_MS;
/// A day played this long.
const LONG_DAY_MS: i64 = 16 * HOUR_MS;
/// Long days in a week that flag.
const LONG_DAYS: usize = 3;
/// The longest break is looked for over this long, once the account has
/// played that far back...
const BREAK_WINDOW_MS: i64 = 3 * DAY_MS;
/// ...and is flagged under this.
const SHORT_BREAK_MS: i64 = 4 * HOUR_MS;
/// Back this fast after a session was ended for it.
pub const QUICK_RETURN_MS: u32 = 15_000;
/// Too few returns to judge by.
const MIN_RETURNS: usize = 5;

/// A session the account played before this one, in Unix ms, and how long
/// after the one before it was ended for it (a kick, reboot or lost
/// connection) it began.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Played {
    pub start_ms: i64,
    pub end_ms: i64,
    pub return_ms: Option<u32>,
}

/// How a session ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Logout,
    Idle,
    Disconnect,
    Kick,
    Reboot,
}

impl End {
    pub fn name(self) -> &'static str {
        match self {
            End::Logout => "logout",
            End::Idle => "idle",
            End::Disconnect => "disconnect",
            End::Kick => "kick",
            End::Reboot => "reboot",
        }
    }

    pub fn from_name(name: &str) -> Option<End> {
        [
            End::Logout,
            End::Idle,
            End::Disconnect,
            End::Kick,
            End::Reboot,
        ]
        .into_iter()
        .find(|e| e.name() == name)
    }

    /// Whether the player didn't choose it, so how fast they're back says
    /// something: a logout is often a world hop, back in seconds.
    pub fn forced(self) -> bool {
        self != End::Logout
    }
}

/// How long after an ended session a login came: from when it ended, or
/// from when the world came back (`world_up_ms`) if it was a reboot. All in
/// Unix ms.
pub fn return_ms(ended_ms: i64, end: End, login_ms: i64, world_up_ms: i64) -> Option<u32> {
    if !end.forced() {
        return None;
    }
    let from = if end == End::Reboot {
        ended_ms.max(world_up_ms)
    } else {
        ended_ms
    };
    u32::try_from(login_ms - from).ok()
}

#[derive(Default)]
pub struct Schedule {
    /// The week's sessions before this one, oldest first.
    pub history: Vec<Played>,
    /// When this session began (Unix ms).
    pub login_ms: i64,
    /// How long after the last session was ended for it this one began.
    pub return_ms: Option<u32>,
}

impl Schedule {
    /// The week's play up to `now` (Unix ms), this session included.
    fn sessions(&self, now: i64) -> Vec<(i64, i64)> {
        let mut all: Vec<(i64, i64)> = self
            .history
            .iter()
            .filter(|p| p.end_ms > now - WEEK_MS)
            .map(|p| (p.start_ms, p.end_ms.min(self.login_ms)))
            .collect();
        if self.login_ms > 0 {
            all.push((self.login_ms, now));
        }
        all.sort_unstable();
        all
    }

    /// Time played on each of the last 7 days (UTC), oldest first.
    fn days(sessions: &[(i64, i64)], now: i64) -> [i64; 7] {
        let today = now.div_euclid(DAY_MS);
        let mut days = [0; 7];
        for (i, played) in days.iter_mut().enumerate() {
            let day = today - 6 + i as i64;
            let (from, to) = (day * DAY_MS, (day + 1) * DAY_MS);
            *played = sessions
                .iter()
                .map(|&(s, e)| (e.min(to) - s.max(from)).max(0))
                .sum();
        }
        days
    }

    /// The longest time not playing over [`BREAK_WINDOW_MS`] up to `now`, if
    /// the account played that far back.
    fn longest_break(sessions: &[(i64, i64)], now: i64) -> Option<i64> {
        let from = now - BREAK_WINDOW_MS;
        if sessions.first().is_none_or(|&(s, _)| s > from) {
            return None;
        }
        let mut longest = 0;
        let mut until = from;
        for &(s, e) in sessions {
            longest = longest.max(s - until);
            until = until.max(e);
        }
        Some(longest.max(now - until))
    }

    /// How long after a forced end each of the week's sessions began.
    fn returns(&self) -> Vec<u32> {
        self.history
            .iter()
            .filter_map(|p| p.return_ms)
            .chain(self.return_ms)
            .collect()
    }

    pub fn report(
        &self,
        now: i64,
        flags: &mut Vec<(&'static str, i16)>,
        stats: &mut Map<String, Value>,
    ) {
        let sessions = self.sessions(now);
        let days = Self::days(&sessions, now);
        let long_days = days.iter().filter(|&&d| d >= LONG_DAY_MS).count();
        if long_days >= LONG_DAYS {
            flags.push(("long_days", 25));
        }
        let longest_break = Self::longest_break(&sessions, now);
        if longest_break.is_some_and(|b| b < SHORT_BREAK_MS) {
            flags.push(("no_long_breaks", 25));
        }
        let returns = self.returns();
        let quick = returns.iter().filter(|&&r| r < QUICK_RETURN_MS).count();
        if returns.len() >= MIN_RETURNS && quick * 5 >= (returns.len() << 2) {
            flags.push(("instant_relogs", 25));
        }
        let hours = |ms: i64| (ms as f64 / HOUR_MS as f64 * 10.0).round() / 10.0;
        stats.insert(
            "schedule".into(),
            json!({
                "dayHours": days.map(hours),
                "longestBreakHours": longest_break.map(hours),
                "returnMs": self.return_ms,
                "returns": returns.len(),
                "quickReturns": quick,
            }),
        );
        // Kept where the next login reads it back.
        stats.insert("returnMs".into(), json!(self.return_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 20_000 * DAY_MS + 12 * HOUR_MS;

    fn flags(s: &Schedule) -> Vec<&'static str> {
        let mut flags = Vec::new();
        s.report(NOW, &mut flags, &mut Map::new());
        flags.into_iter().map(|(f, _)| f).collect()
    }

    /// A session each day for a week, from `start` hours past midnight for
    /// `hours`, and this one an hour in.
    fn week(start: i64, hours: i64, return_ms: Option<u32>) -> Schedule {
        let today = NOW / DAY_MS * DAY_MS;
        let history = (1..7)
            .map(|d| Played {
                start_ms: today - d * DAY_MS + start * HOUR_MS,
                end_ms: today - d * DAY_MS + (start + hours) * HOUR_MS,
                return_ms,
            })
            .rev()
            .collect();
        Schedule {
            history,
            login_ms: NOW - HOUR_MS,
            return_ms,
        }
    }

    #[test]
    fn evenings_pass() {
        assert!(flags(&week(17, 5, Some(40_000))).is_empty());
    }

    #[test]
    fn round_the_clock_play_is_flagged() {
        // 20 hours a day, the rest in two-hour bits, and on since midnight.
        let mut s = week(0, 10, None);
        let more = week(12, 10, None);
        s.history.extend(more.history);
        s.history.sort_by_key(|p| p.start_ms);
        s.login_ms = NOW - 12 * HOUR_MS;
        assert_eq!(flags(&s), ["long_days", "no_long_breaks"]);
    }

    #[test]
    fn coming_straight_back_every_time_is_flagged() {
        assert_eq!(flags(&week(17, 5, Some(4_000))), ["instant_relogs"]);
    }

    #[test]
    fn returns_count_from_the_world_coming_back() {
        let ended = 1_000_000;
        assert_eq!(return_ms(ended, End::Logout, ended + 5_000, 0), None);
        assert_eq!(
            return_ms(ended, End::Disconnect, ended + 5_000, 0),
            Some(5_000)
        );
        let up = ended + 120_000;
        assert_eq!(return_ms(ended, End::Reboot, up + 3_000, up), Some(3_000));
    }
}
