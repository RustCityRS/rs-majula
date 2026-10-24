use super::anticheat::Spread;

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
const MIN_NUDGES: u32 = 5;
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

/// What judging a stretch takes of its nudges, kept as they come rather
/// than each one.
#[derive(Default)]
struct Nudges {
    /// The latest, which inputs still join (see [`NUDGE_MS`]).
    last: Option<Nudge>,
    count: u32,
    /// A mouse nudge before the latest wasn't tiny.
    big: bool,
    /// The first two kinds, and whether more came.
    kinds: [Option<Kind>; 2],
    more_kinds: bool,
    /// The gaps between nudges.
    gaps: Spread,
}

impl Nudges {
    fn push(&mut self, next: Nudge) {
        if let Some(prev) = self.last.replace(next) {
            self.big |= prev.kind == Kind::Mouse && prev.far > TINY_PX;
            self.gaps.push(f64::from(next.ms.saturating_sub(prev.ms)));
        }
        self.count += 1;
        if !self.kinds.contains(&Some(next.kind)) {
            match self.kinds.iter_mut().find(|k| k.is_none()) {
                Some(free) => *free = Some(next.kind),
                None => self.more_kinds = true,
            }
        }
    }
}

#[derive(Default)]
pub struct AntiAfk {
    since: u32,
    nudges: Nudges,
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
        self.nudges = Nudges::default();
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
        if let Some(last) = &mut self.nudges.last
            && last.kind == kind
            && ms.saturating_sub(last.ms) < NUDGE_MS
        {
            let far = (to.0 - last.from.0).abs().max((to.1 - last.from.1).abs());
            last.far = last.far.max(far);
            return;
        }
        let far = (to.0 - from.0).abs().max((to.1 - from.1).abs());
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
        let nudges = &self.nudges;
        let Some(last) = nudges.last else {
            return length;
        };
        let tiny = !nudges.big && (last.kind != Kind::Mouse || last.far <= TINY_PX);
        let even = nudges.gaps.cv().is_some_and(|cv| cv < EVEN);
        if tiny && !nudges.more_kinds && nudges.count >= MIN_NUDGES && even {
            length
        } else {
            0
        }
    }

    /// Flags it, and gives how long the player was kept awake (ms).
    pub fn report(&self, now: u32, flags: &mut Vec<(&'static str, i16)>) -> u32 {
        let awake = self.awake_ms + self.kept_awake(now);
        if awake >= FLAG_MS {
            flags.push(("anti_afk", 30));
        }
        awake
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u32 = 60_000;

    fn flags(a: &AntiAfk, now: u32) -> Vec<&'static str> {
        let mut flags = Vec::new();
        a.report(now, &mut flags);
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

    #[test]
    fn more_than_two_kinds_of_nudge_pass() {
        let mut a = AntiAfk::default();
        a.activity(0);
        for m in 1..=70 {
            a.key(m * MIN, 65 + (m % 3) as u8);
        }
        assert!(flags(&a, 70 * MIN + 100).is_empty());
        let mut two = AntiAfk::default();
        two.activity(0);
        for m in 1..=70 {
            two.key(m * MIN, 65 + (m % 2) as u8);
        }
        assert_eq!(flags(&two, 70 * MIN + 100), ["anti_afk"]);
    }
}
