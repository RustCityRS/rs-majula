use serde_json::{Map, Value, json};

/// A stretch without actions this long (ms) is judged.
const MIN_STRETCH_MS: u32 = 15 * 60_000;
/// Inputs this close together are one nudge.
const NUDGE_MS: u32 = 1_000;
/// A nudge that moves the mouse no further than this (pixels) is tiny.
const TINY_PX: i32 = 5;
/// Nudges this even (standard deviation of the gaps over their mean) are a
/// machine's.
const EVEN: f64 = 0.3;
/// Too few nudges in a stretch to judge it by.
const MIN_NUDGES: usize = 5;
/// The most nudges kept for a stretch.
const MAX_NUDGES: usize = 2048;
/// Kept awake this long in a session flags.
const FLAG_MS: u32 = 60 * 60_000;

/// What kept the client awake: the mouse, a key or the camera turning.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Mouse,
    Key(u8),
    Camera,
}

#[derive(Clone, Copy)]
struct Nudge {
    ms: u32,
    kind: Kind,
    from: (i32, i32),
    far: i32,
}

#[derive(Default)]
pub struct AntiAfk {
    since: u32,
    nudges: Vec<Nudge>,
    idle: bool,
    cursor: Option<(i32, i32)>,
    awake_ms: u32,
}

impl AntiAfk {
    /// A real action, `ms` into the session: an action, a click or chat.
    pub fn activity(&mut self, ms: u32) {
        // Inputs are timed on more than one clock, so one can come early.
        if ms < self.since {
            return;
        }
        self.awake_ms += self.kept_awake(ms);
        self.since = ms;
        self.nudges.clear();
        self.idle = false;
    }

    /// The client said it's idle.
    pub fn idle(&mut self) {
        self.idle = true;
    }

    /// A cursor sample.
    pub fn moved(&mut self, ms: u32, x: i16, y: i16) {
        let at = (i32::from(x), i32::from(y));
        let from = self.cursor.replace(at).unwrap_or(at);
        self.nudge(ms, Kind::Mouse, from, at);
    }

    pub fn key(&mut self, ms: u32, key: u8) {
        self.nudge(ms, Kind::Key(key), (0, 0), (0, 0));
    }

    pub fn camera(&mut self, ms: u32) {
        self.nudge(ms, Kind::Camera, (0, 0), (0, 0));
    }

    fn nudge(&mut self, ms: u32, kind: Kind, from: (i32, i32), to: (i32, i32)) {
        if let Some(last) = self.nudges.last_mut()
            && last.kind == kind
            && ms.saturating_sub(last.ms) < NUDGE_MS
        {
            let far = (to.0 - last.from.0).abs().max((to.1 - last.from.1).abs());
            last.far = last.far.max(far);
            return;
        }
        let far = (to.0 - from.0).abs().max((to.1 - from.1).abs());
        // A day nudged a minute apart; past that the oldest go.
        if self.nudges.len() == MAX_NUDGES {
            self.nudges.drain(..MAX_NUDGES >> 1);
        }
        self.nudges.push(Nudge {
            ms,
            kind,
            from,
            far,
        });
    }

    /// How long of the stretch up to `now` something kept the client awake:
    /// all of it, if it's long, the client never said it was idle, and only
    /// tiny moves or one or two keys came, evenly, or nothing at all came
    /// (from 274 keys aren't reported).
    fn kept_awake(&self, now: u32) -> u32 {
        let length = now.saturating_sub(self.since);
        if length < MIN_STRETCH_MS || self.idle {
            return 0;
        }
        if self.nudges.is_empty() {
            return length;
        }
        let tiny = self
            .nudges
            .iter()
            .all(|n| n.kind != Kind::Mouse || n.far <= TINY_PX);
        let mut kinds: Vec<Kind> = Vec::new();
        for n in &self.nudges {
            if !kinds.contains(&n.kind) {
                kinds.push(n.kind);
            }
        }
        let gaps = self
            .nudges
            .windows(2)
            .map(|w| f64::from(w[1].ms.saturating_sub(w[0].ms)));
        let even = super::anticheat::cv(gaps).is_some_and(|cv| cv < EVEN);
        if tiny && kinds.len() <= 2 && self.nudges.len() >= MIN_NUDGES && even {
            length
        } else {
            0
        }
    }

    pub fn report(
        &self,
        now: u32,
        flags: &mut Vec<(&'static str, i16)>,
        stats: &mut Map<String, Value>,
    ) {
        let awake = self.awake_ms + self.kept_awake(now);
        if awake >= FLAG_MS {
            flags.push(("anti_afk", 30));
        }
        stats.insert("keptAwakeMinutes".into(), json!(awake / 60_000));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u32 = 60_000;

    fn flags(a: &AntiAfk, now: u32) -> Vec<&'static str> {
        let mut flags = Vec::new();
        a.report(now, &mut flags, &mut Map::new());
        flags.into_iter().map(|(f, _)| f).collect()
    }

    #[test]
    fn a_jiggled_mouse_is_flagged() {
        let mut a = AntiAfk::default();
        a.activity(0);
        // A pixel right and back each minute, for 70 minutes.
        for m in 1..=70 {
            a.moved(m * MIN, 300, 200);
            a.moved(m * MIN + 50, 301, 200);
        }
        assert_eq!(flags(&a, 70 * MIN + 100), ["anti_afk"]);
    }

    #[test]
    fn people_watching_or_away_pass() {
        let mut a = AntiAfk::default();
        // Watching: the mouse wanders now and then, for an hour each time.
        for hour in 0..3 {
            let start = hour * 61 * MIN;
            a.activity(start);
            for m in [3, 11, 12, 30, 47, 55] {
                a.moved(start + m * MIN, 100, 100);
                a.moved(start + m * MIN + 50, 180, 140);
            }
        }
        // Away: the client says it's idle.
        a.activity(200 * MIN);
        a.idle();
        assert!(flags(&a, 290 * MIN).is_empty());
        // Short stretches between actions don't count.
        let mut busy = AntiAfk::default();
        for m in 0..120 {
            busy.activity(m * 10 * MIN);
            busy.key(m * 10 * MIN + MIN, 65);
        }
        assert!(flags(&busy, 1_200 * MIN).is_empty());
    }
}
