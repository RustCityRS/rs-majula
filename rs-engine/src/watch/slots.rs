use serde_json::{Value, json};
use std::collections::VecDeque;

/// A slot's top left on the client's screen.
pub type Slot = (i16, i16);

/// How long a menu may stay open before its option is picked, and how far
/// after an action its press may seem to come (before 254 the input reports
/// time presses a little off).
const MENU_MS: u32 = 30_000;
const SLACK_MS: u32 = 3_000;
/// Presses kept for actions still to come, and actions kept waiting for
/// presses still to be reported.
const KEPT: usize = 16;
/// Spots kept.
const SPOTS: usize = 300;
/// Too few to judge by.
const MIN_SAME: usize = 20;
const MIN_EVEN: usize = 50;
/// How far (pixels) clicks spread from their mean at most, from people's.
/// Spread evenly over a slot they're 9.2 apart.
const EVEN_SPREAD: f64 = 8.3;

#[derive(Clone, Copy)]
struct Press {
    ms: u32,
    x: i16,
    y: i16,
    right: bool,
}

#[derive(Default)]
pub struct SlotClicks {
    moved: bool,
    presses: VecDeque<Press>,
    waiting: VecDeque<(u32, Slot, bool)>,
    spots: VecDeque<(u8, u8)>,
}

/// Where `press` landed in `slot`, if it did.
fn spot(press: &Press, slot: Slot) -> Option<(u8, u8)> {
    let dx = press.x - slot.0;
    let dy = press.y - slot.1;
    ((0..32).contains(&dx) && (0..32).contains(&dy)).then_some((dx as u8, dy as u8))
}

/// Whether a press at `press` could have made an action at `op`.
fn timed(press: u32, op: u32) -> bool {
    press <= op + SLACK_MS && op <= press + MENU_MS
}

impl SlotClicks {
    /// The cursor moved.
    pub fn moved(&mut self) {
        self.moved = true;
    }

    /// A press (or from 254, a click) on the screen.
    pub fn press(&mut self, ms: u32, x: i16, y: i16, right: bool) {
        if !std::mem::take(&mut self.moved) {
            return;
        }
        let press = Press { ms, x, y, right };
        let waiting = self.waiting.iter().position(|&(op, slot, menu)| {
            menu == right && timed(ms, op) && spot(&press, slot).is_some()
        });
        if let Some(i) = waiting {
            let (_, slot, _) = self.waiting.remove(i).unwrap();
            self.landed(spot(&press, slot).unwrap());
            return;
        }
        if self.presses.len() == KEPT {
            self.presses.pop_front();
        }
        self.presses.push_back(press);
    }

    /// An action, `ms` into the session, on an obj in `slot`. `menu`: it's an
    /// option the menu was opened for.
    pub fn action(&mut self, ms: u32, slot: Slot, menu: bool) {
        while self
            .waiting
            .front()
            .is_some_and(|&(op, _, _)| ms > op + MENU_MS)
        {
            self.waiting.pop_front();
        }
        let pressed = self
            .presses
            .iter()
            .rposition(|p| p.right == menu && timed(p.ms, ms) && spot(p, slot).is_some());
        match pressed {
            Some(i) => {
                let press = self.presses.remove(i).unwrap();
                self.landed(spot(&press, slot).unwrap());
            }
            None => {
                if self.waiting.len() == KEPT {
                    self.waiting.pop_front();
                }
                self.waiting.push_back((ms, slot, menu));
            }
        }
    }

    fn landed(&mut self, spot: (u8, u8)) {
        if self.spots.len() == SPOTS {
            self.spots.pop_front();
        }
        self.spots.push_back(spot);
    }

    /// The share of clicks on the most clicked pixel, and how far clicks
    /// spread from their mean, across and down, averaged: in one pass, the
    /// pixels (32 by 32) counted in place.
    fn same_and_spread(&self) -> (f64, Option<f64>) {
        let mut counts = [0u16; 32 * 32];
        let mut most = 0;
        let (mut sx, mut sy, mut sxx, mut syy) = (0u32, 0u32, 0u32, 0u32);
        for &(x, y) in &self.spots {
            let count = &mut counts[(usize::from(y) << 5) | usize::from(x)];
            *count += 1;
            most = most.max(*count);
            let (x, y) = (u32::from(x), u32::from(y));
            sx += x;
            sy += y;
            sxx += x * x;
            syy += y * y;
        }
        let n = self.spots.len() as f64;
        let same = f64::from(most) / n.max(1.0);
        // Whole sums: exact, so the variance is too.
        let axis = |s: u32, ss: u32| {
            let mean = f64::from(s) / n;
            (f64::from(ss) / n - mean * mean).max(0.0).sqrt()
        };
        let spread = (n >= 2.0).then(|| (axis(sx, sxx) + axis(sy, syy)) / 2.0);
        (same, spread)
    }

    pub fn report(&self, flags: &mut Vec<(&'static str, i16)>) -> SlotStats {
        let n = self.spots.len();
        let (same, spread) = self.same_and_spread();
        if n >= MIN_SAME && same >= 0.5 {
            flags.push(("same_pixel_clicks", 30));
        }
        if n >= MIN_EVEN && spread.is_some_and(|s| s >= EVEN_SPREAD) {
            flags.push(("even_slot_clicks", 20));
        }
        SlotStats {
            count: n,
            same,
            spread,
        }
    }
}

/// What [`SlotClicks::report`] found, for the report's stats.
pub struct SlotStats {
    count: usize,
    same: f64,
    spread: Option<f64>,
}

impl SlotStats {
    pub fn json(&self) -> Value {
        json!({
            "count": self.count,
            "samePixel": (self.same * 100.0).round() / 100.0,
            "spread": self.spread.map(|s| (s * 10.0).round() / 10.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(s: &SlotClicks) -> Vec<&'static str> {
        let mut flags = Vec::new();
        s.report(&mut flags);
        flags.into_iter().map(|(f, _)| f).collect()
    }

    /// 60 left clicks on slots, each landing at `at(i)` in its slot, the
    /// press reported before the action (from 254) or after it (before).
    fn clicks(at: impl Fn(u32) -> (i16, i16), press_first: bool) -> SlotClicks {
        let mut s = SlotClicks::default();
        for i in 0..60u32 {
            let slot = (563 + (i & 3) as i16 * 42, 213 + ((i >> 2) % 7) as i16 * 36);
            let (dx, dy) = at(i);
            let ms = i * 1_000;
            s.moved();
            if press_first {
                s.press(ms, slot.0 + dx, slot.1 + dy, false);
                s.action(ms + 100, slot, false);
            } else {
                s.action(ms + 100, slot, false);
                s.press(ms, slot.0 + dx, slot.1 + dy, false);
            }
        }
        s
    }

    /// A pseudo-random number below `n` for click `i`.
    fn rand(i: u32, n: u32) -> i16 {
        ((i.wrapping_mul(2_654_435_761) >> 7) % n) as i16
    }

    #[test]
    fn people_aim_at_the_middle() {
        for press_first in [true, false] {
            let s = clicks(|i| (10 + rand(i, 12), 9 + rand(i + 99, 13)), press_first);
            assert_eq!(s.spots.len(), 60);
            assert!(flags(&s).is_empty(), "{:?}", s.same_and_spread().1);
        }
    }

    #[test]
    fn bots_hit_one_pixel_or_anywhere() {
        let same = clicks(|_| (16, 16), true);
        assert_eq!(flags(&same), ["same_pixel_clicks"]);
        let even = clicks(|i| (rand(i, 32), rand(i + 7, 32)), false);
        assert_eq!(flags(&even), ["even_slot_clicks"]);
    }

    #[test]
    fn a_menu_option_is_judged_by_its_right_press() {
        let mut s = SlotClicks::default();
        s.moved();
        s.press(1_000, 580, 230, true);
        // The left press picking the option, over the menu.
        s.moved();
        s.press(1_800, 600, 260, false);
        s.action(1_900, (563, 213), true);
        assert_eq!(s.spots, [(17, 17)]);
    }

    #[test]
    fn clicking_again_without_moving_isnt_aiming() {
        let mut s = SlotClicks::default();
        for i in 0..40 {
            // Withdraw-1, again and again, the cursor left where it was.
            if i == 0 {
                s.moved();
            }
            s.press(i * 600, 580, 230, false);
            s.action(i * 600 + 100, (563, 213), false);
        }
        assert_eq!(s.spots.len(), 1);
    }
}
