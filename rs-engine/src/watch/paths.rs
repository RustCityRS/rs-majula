use serde_json::{Value, json};
use std::collections::VecDeque;

/// A pause this long (ms) between samples ends a move.
const PAUSE_MS: u32 = 200;
/// Moves shorter than this (pixels), or with fewer samples, aren't judged.
const MIN_DISTANCE: f64 = 40.0;
const MIN_SAMPLES: usize = 5;
/// The most samples a move keeps (10 seconds).
const MAX_SAMPLES: usize = 200;
/// A move whose path is no longer than this over its straight line.
const STRAIGHT: f64 = 1.002;
/// A move whose 50ms steps vary less than this (their spread over their
/// mean) went at an even pace. People's steps vary about as much as their
/// mean, as they speed up, slow down and correct: in recorded sessions 9 in
/// 10 of their moves were over 0.6, and 9 in 10 scripted moves under 0.35. How
/// much faster the middle of a move is than its ends doesn't tell them
/// apart: a third of people's moves aren't faster midway at 50ms samples.
const STEADY: f64 = 0.45;
/// A move that strays further than this from its straight line (over its
/// length) is a curve...
const CURVED: f64 = 0.05;
/// ...and is the same as an earlier one if it never strays from it by more.
const SAME_CURVE: f64 = 0.02;
/// Curves kept to compare new ones to.
const CURVES: usize = 64;
/// Too few to judge by.
const MIN_MOVES: u32 = 30;
const MIN_CURVES: u32 = 20;

/// Where a curve is (off its straight line, over its length) at each eighth
/// of the way along, and how many samples it took.
type Shape = ([f64; 7], usize);

#[derive(Default)]
pub struct MousePaths {
    samples: Vec<(u32, f64, f64)>,
    moves: u32,
    straight: u32,
    steady: u32,
    curves: u32,
    repeated: u32,
    shapes: VecDeque<Shape>,
}

impl MousePaths {
    /// A cursor sample, `ms` into the session.
    pub fn moved(&mut self, ms: u32, x: i16, y: i16) {
        if self
            .samples
            .last()
            .is_some_and(|&(last, ..)| ms.saturating_sub(last) > PAUSE_MS)
        {
            // It stood still: the move is over, and the next starts here.
            let rest = self.samples.last().copied();
            self.end();
            self.samples.extend(rest);
        }
        if self.samples.len() == MAX_SAMPLES {
            self.end();
        }
        self.samples.push((ms, f64::from(x), f64::from(y)));
    }

    /// The cursor left the window.
    pub fn outside(&mut self) {
        self.samples.clear();
    }

    /// A click: the move to it is over.
    pub fn click(&mut self) {
        let rest = self.samples.last().copied();
        self.end();
        self.samples.extend(rest);
    }

    fn end(&mut self) {
        self.judge();
        self.samples.clear();
    }

    /// Judges the move the samples make, in one pass over them.
    fn judge(&mut self) {
        let samples = &self.samples;
        let Some(&(_, x0, y0)) = samples.first() else {
            return;
        };
        let &(_, x1, y1) = samples.last().unwrap();
        let (dx, dy) = (x1 - x0, y1 - y0);
        let distance = dx.hypot(dy);
        if samples.len() < MIN_SAMPLES || distance < MIN_DISTANCE {
            return;
        }
        self.moves += 1;
        // How far off the straight line (over its length) a sample is.
        let off = |i: usize| {
            let (_, x, y) = samples[i];
            ((x - x0) * dy - (y - y0) * dx) / (distance * distance)
        };
        // The first sample is on the line, so each step's end covers the rest.
        let (mut path, mut curved) = (0.0, false);
        let steps = samples.windows(2).enumerate().map(|(i, w)| {
            curved |= off(i + 1).abs() > CURVED;
            (w[1].1 - w[0].1).hypot(w[1].2 - w[0].2)
        });
        let cv = super::anticheat::cv(steps.inspect(|step| path += step));
        if path <= distance * STRAIGHT {
            self.straight += 1;
        }
        if cv.is_some_and(|cv| cv < STEADY) {
            self.steady += 1;
        }
        if !curved {
            return;
        }
        self.curves += 1;
        let last = samples.len() - 1;
        let at = |f: f64| {
            let i = f * last as f64;
            let (lo, t) = (i.floor() as usize, i.fract());
            off(lo) + (off((lo + 1).min(last)) - off(lo)) * t
        };
        let shape: Shape = (
            std::array::from_fn(|i| at((i + 1) as f64 / 8.0)),
            samples.len(),
        );
        let same = self.shapes.iter().any(|(points, len)| {
            *len == shape.1
                && points
                    .iter()
                    .zip(&shape.0)
                    .all(|(a, b)| (a - b).abs() <= SAME_CURVE)
        });
        if same {
            self.repeated += 1;
        }
        if self.shapes.len() == CURVES {
            self.shapes.pop_front();
        }
        self.shapes.push_back(shape);
    }

    pub fn report(&self, flags: &mut Vec<(&'static str, i16)>) -> PathStats {
        if self.moves >= MIN_MOVES {
            if self.straight * 10 >= self.moves * 6 {
                flags.push(("straight_mouse_moves", 25));
            }
            if self.steady * 10 >= self.moves * 6 {
                flags.push(("steady_mouse_moves", 25));
            }
        }
        if self.curves >= MIN_CURVES && self.repeated * 10 >= (self.curves << 2) {
            flags.push(("repeated_mouse_curves", 25));
        }
        PathStats {
            moves: self.moves,
            straight: self.straight,
            steady: self.steady,
            curves: self.curves,
            repeated: self.repeated,
        }
    }
}

/// What [`MousePaths::report`] found, for the report's stats.
pub struct PathStats {
    moves: u32,
    straight: u32,
    steady: u32,
    curves: u32,
    repeated: u32,
}

impl PathStats {
    pub fn json(&self) -> Value {
        json!({
            "moves": self.moves,
            "straight": self.straight,
            "steady": self.steady,
            "curves": self.curves,
            "repeatedCurves": self.repeated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(p: &MousePaths) -> Vec<&'static str> {
        let mut flags = Vec::new();
        p.report(&mut flags);
        flags.into_iter().map(|(f, _)| f).collect()
    }

    /// 40 moves back and forth 300 px across, of `samples(i)` samples each,
    /// placed along the way by `along` (0 to 1) and off it by `off`
    /// (pixels), each then a click.
    fn moves(
        samples: impl Fn(u32) -> u32,
        along: impl Fn(u32, f64) -> f64,
        off: impl Fn(u32, f64) -> f64,
    ) -> MousePaths {
        let mut p = MousePaths::default();
        p.moved(0, 100, 100);
        let mut ms = 0;
        for i in 0..40 {
            ms += 1_000;
            let n = samples(i);
            for s in 1..=n {
                let t = f64::from(s) / f64::from(n);
                let a = along(i, t);
                let x = if i & 1 == 0 {
                    100.0 + 300.0 * a
                } else {
                    400.0 - 300.0 * a
                };
                p.moved(ms, x.round() as i16, (100.0 + off(i, t)).round() as i16);
                ms += 50;
            }
            p.click();
        }
        p
    }

    /// Smooth start and stop, as people move.
    fn ease(t: f64) -> f64 {
        (1.0 - (t * std::f64::consts::PI).cos()) / 2.0
    }

    /// A pseudo-random number from 0 to 1 for move `i`.
    fn rand(i: u32) -> f64 {
        f64::from(i.wrapping_mul(2_654_435_761) >> 8) / f64::from(1u32 << 24)
    }

    #[test]
    fn people_curve_and_ease_in_and_out() {
        let p = moves(
            |i| 6 + i * 7 % 9,
            |_, t| ease(t),
            |i, t| {
                let bend = 0.5 + 1.5 * rand(i + 1);
                (std::f64::consts::PI * t.powf(bend)).sin() * (50.0 * rand(i) - 25.0)
            },
        );
        assert_eq!(p.moves, 40);
        assert!(flags(&p).is_empty(), "{:?} {}", flags(&p), p.repeated);
    }

    #[test]
    fn bots_move_straight_and_steady() {
        let p = moves(|_| 9, |_, t| t, |_, _| 0.0);
        assert_eq!(flags(&p), ["straight_mouse_moves", "steady_mouse_moves"]);
    }

    #[test]
    fn bots_repeat_their_curves() {
        let p = moves(
            |_| 9,
            |_, t| ease(t),
            |_, t| (t * std::f64::consts::PI).sin() * 30.0,
        );
        // Eased at an even pace, so steady too.
        assert_eq!(flags(&p), ["steady_mouse_moves", "repeated_mouse_curves"]);
    }
}
