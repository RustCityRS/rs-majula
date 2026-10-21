use super::input::{Input, InputDecoder, Timed};
use Field::*;
use rs_db::ReplayReport;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_applet_focus::EventAppletFocus;
#[cfg(any(rev = "225", since_254))]
use rs_protocol::network::game::client::event_camera_position::EventCameraPosition;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_mouse_click::EventMouseClick;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_mouse_move::EventMouseMove;
#[cfg(before_274)]
use rs_protocol::network::game::client::event_tracking::EventTracking;
use rs_protocol::network::game::client_prot::ClientProt;
use serde_json::json;
use std::collections::VecDeque;

/// A payload field: a fixed value, or (`Any*`) a random one.
#[derive(Clone, Copy)]
enum Field {
    B(u8),
    S(u16),
    T(u32),
    I(u32),
    AnyB,
    AnyS,
}

/// A field the client sends only half the time (at random).
#[derive(Clone, Copy)]
struct Part(Field, bool);

const fn f(field: Field) -> Part {
    Part(field, false)
}

const fn opt(field: Field) -> Part {
    Part(field, true)
}

/// Each `ANTICHEAT_CYCLELOGIC1..` payload, and whether it comes from the
/// game loop (sent while the client runs at all) rather than drawing code
/// (which a background tab skips).
#[cfg(before_254)]
const HEARTBEATS: [(&[Part], bool); 6] = [
    (&[f(B(43))], false),
    (
        &[
            f(S(29711)),
            f(B(70)),
            f(AnyB),
            f(B(242)),
            f(B(186)),
            f(B(39)),
            f(B(61)),
            opt(B(13)),
            opt(S(57856)),
            f(AnyS),
        ],
        false,
    ),
    (&[f(T(4991788))], true),
    (&[f(I(0))], true),
    (&[], false),
    (
        &[
            f(B(162)),
            f(B(22)),
            opt(B(84)),
            f(S(31824)),
            f(S(13490)),
            opt(B(123)),
            opt(B(134)),
            f(B(100)),
            f(B(94)),
            f(S(35521)),
        ],
        false,
    ),
];

#[cfg(since_254)]
const HEARTBEATS: [(&[Part], bool); 7] = [
    (
        &[
            opt(S(11499)),
            f(S(10548)),
            opt(B(139)),
            opt(B(94)),
            f(S(51693)),
            f(B(16)),
            f(S(15036)),
            opt(B(65)),
            f(AnyB),
            f(S(22990)),
        ],
        false,
    ),
    (
        &[
            opt(S(16791)),
            f(B(254)),
            f(AnyS),
            f(S(16128)),
            f(S(52610)),
            f(AnyS),
            f(S(55420)),
            opt(S(35025)),
            f(S(46628)),
            f(AnyB),
        ],
        false,
    ),
    (&[f(B(50))], false),
    (&[f(B(232))], true),
    (&[], false),
    (&[f(B(62))], false),
    (&[], true),
];

/// Each `ANTICHEAT_OPLOGIC1..9` payload.
#[cfg(before_254)]
const OP_CHECKS: [&[Part]; 9] = [
    &[f(I(0))],
    &[f(I(0))],
    &[f(T(14953816))],
    &[f(S(39596))],
    &[],
    &[f(I(0))],
    &[f(I(0))],
    &[f(S(31114))],
    &[f(B(177))],
];

#[cfg(since_254)]
const OP_CHECKS: [&[Part]; 9] = [
    &[f(I(0))],
    &[f(S(37954))],
    &[f(I(0))],
    &[f(B(131))],
    &[f(B(154))],
    &[f(S(6118))],
    &[f(I(0))],
    &[f(B(19))],
    &[f(T(13018169))],
];

/// Whether `data` is a payload `parts` describes.
fn matches(parts: &[Part], data: &[u8]) -> bool {
    let Some((&Part(field, optional), rest)) = parts.split_first() else {
        return data.is_empty();
    };
    let width = match field {
        B(_) | AnyB => 1,
        S(_) | AnyS => 2,
        T(_) => 3,
        I(_) => 4,
    };
    let here = data.len() >= width && {
        let value = data[..width]
            .iter()
            .fold(0u32, |v, &b| (v << 8) | u32::from(b));
        match field {
            B(v) => value == u32::from(v),
            S(v) => value == u32::from(v),
            T(v) | I(v) => value == v,
            AnyB | AnyS => true,
        }
    };
    (here && matches(rest, &data[width..])) || (optional && matches(rest, data))
}

/// A menu action, with what the client's op checks count from it.
#[derive(Clone, Copy, Debug)]
pub enum Op {
    Npc {
        op: u8,
        nid: u16,
    },
    Loc {
        op: u8,
        id: u16,
        x: u16,
        z: u16,
    },
    Obj {
        op: u8,
        id: u16,
        x: u16,
        z: u16,
    },
    Player {
        op: u8,
        pid: u16,
    },
    Held {
        op: u8,
        obj: u16,
        slot: u16,
    },
    InvButton {
        op: u8,
        obj: u16,
        slot: u16,
        com: u16,
    },
    Other,
}

/// Where the client's map starts: `(x, z)` of its south-west tile. The
/// client counts some ops by offsets into it.
#[derive(Clone, Copy, Debug, Default)]
pub struct MapBase {
    pub x: u16,
    pub z: u16,
}

/// How an op check's count grows, and the count it's sent from.
#[derive(Clone, Copy)]
enum Count {
    One,
    Quarter(i32),
    By(i32),
}

/// Too few to judge by.
const MIN_OPS: u32 = 20;
const MIN_CLICKS: u32 = 20;
/// A click this far (in pixels) from the cursor's last known spot.
const JUMP_PX: i32 = 40;
/// Click gaps longer than this are pauses, not rhythm.
const RHYTHM_MAX_MS: u32 = 10_000;
/// How many recent click gaps are kept to judge rhythm by.
const RHYTHM_GAPS: usize = 200;

#[derive(Default)]
pub struct AntiCheat {
    pub inputs: InputDecoder,
    /// Whether the browser said it's a phone or tablet. Taps appear where
    /// they land, so cursor jumps are expected and don't score.
    pub touch: bool,
    /// Whether the web client ran in a browser that says it's automated
    /// (headless), or sent no user agent at all.
    pub automated: bool,
    heartbeats: [u32; 7],
    bad_heartbeats: [u32; 7],
    op_counts: [i64; 9],
    op_expected: [u32; 9],
    op_received: [u32; 9],
    bad_op_checks: [u32; 9],
    ops: u32,
    ops_unclicked: u32,
    last_click_clock: Option<u32>,
    clicks: u32,
    jumps: u32,
    gaps: VecDeque<u32>,
    #[cfg(before_254)]
    last_press_ms: Option<u32>,
    last_cursor: Option<(i32, i32)>,
    input_reports: u32,
    focus_lost: u32,
    idle_reports: u32,
    camera_reports: u32,
    decoded: Vec<Timed>,
}

impl AntiCheat {
    /// A packet from the client. (Input reports come in decoded, by their
    /// handlers.)
    pub fn client(&mut self, prot: &ClientProt, data: &[u8]) {
        match prot {
            ClientProt::AnticheatCycleLogic1 => self.heartbeat(0, data),
            ClientProt::AnticheatCycleLogic2 => self.heartbeat(1, data),
            ClientProt::AnticheatCycleLogic3 => self.heartbeat(2, data),
            ClientProt::AnticheatCycleLogic4 => self.heartbeat(3, data),
            ClientProt::AnticheatCycleLogic5 => self.heartbeat(4, data),
            ClientProt::AnticheatCycleLogic6 => self.heartbeat(5, data),
            #[cfg(since_254)]
            ClientProt::AnticheatCycleLogic7 => self.heartbeat(6, data),
            ClientProt::AnticheatOpLogic1 => self.op_check(0, data),
            ClientProt::AnticheatOpLogic2 => self.op_check(1, data),
            ClientProt::AnticheatOpLogic3 => self.op_check(2, data),
            ClientProt::AnticheatOpLogic4 => self.op_check(3, data),
            ClientProt::AnticheatOpLogic5 => self.op_check(4, data),
            ClientProt::AnticheatOpLogic6 => self.op_check(5, data),
            ClientProt::AnticheatOpLogic7 => self.op_check(6, data),
            ClientProt::AnticheatOpLogic8 => self.op_check(7, data),
            ClientProt::AnticheatOpLogic9 => self.op_check(8, data),
            ClientProt::IdleTimer => self.idle_reports += 1,
            _ => {}
        }
    }

    /// An `EVENT_MOUSE_MOVE`, `ms` into the session.
    #[cfg(since_254)]
    pub fn mouse_move(&mut self, ms: u32, msg: &EventMouseMove) {
        self.input_reports += 1;
        self.inputs_from(|inputs, out| inputs.mouse_move(ms, msg, out));
    }

    /// An `EVENT_MOUSE_CLICK`, `ms` into the session on tick `clock`.
    #[cfg(since_254)]
    pub fn mouse_click(&mut self, ms: u32, clock: u32, msg: &EventMouseClick) {
        self.last_click_clock = Some(clock);
        self.inputs_from(|inputs, out| inputs.mouse_click(ms, msg, out));
    }

    /// An `EVENT_APPLET_FOCUS`, `ms` into the session.
    #[cfg(since_254)]
    pub fn focus(&mut self, ms: u32, msg: &EventAppletFocus) {
        self.inputs_from(|inputs, out| inputs.focus(ms, msg, out));
    }

    /// An `EVENT_CAMERA_POSITION`, `ms` into the session.
    #[cfg(any(rev = "225", since_254))]
    pub fn camera(&mut self, ms: u32, msg: &EventCameraPosition) {
        self.inputs_from(|inputs, out| inputs.camera(ms, msg, out));
    }

    /// An `EVENT_TRACKING`, `ms` into the session. Before 254 it's the only
    /// input report.
    #[cfg(before_274)]
    pub fn tracking(&mut self, ms: u32, msg: &EventTracking) {
        #[cfg(before_254)]
        {
            self.input_reports += 1;
        }
        self.inputs_from(|inputs, out| inputs.tracking(ms, msg, out));
    }

    /// Checks the inputs `decode` times from a report.
    fn inputs_from(&mut self, decode: impl FnOnce(&mut InputDecoder, &mut Vec<Timed>)) {
        let mut decoded = std::mem::take(&mut self.decoded);
        decoded.clear();
        decode(&mut self.inputs, &mut decoded);
        for timed in &decoded {
            self.input(timed);
        }
        self.decoded = decoded;
    }

    fn heartbeat(&mut self, index: usize, data: &[u8]) {
        self.heartbeats[index] += 1;
        if !matches(HEARTBEATS[index].0, data) {
            self.bad_heartbeats[index] += 1;
        }
    }

    fn op_check(&mut self, index: usize, data: &[u8]) {
        self.op_received[index] += 1;
        if !matches(OP_CHECKS[index], data) {
            self.bad_op_checks[index] += 1;
        }
    }

    fn input(&mut self, timed: &Timed) {
        match timed.input {
            Input::Move { x, y } => self.last_cursor = Some((x.into(), y.into())),
            Input::Outside => self.last_cursor = None,
            Input::Click { x, y, gap_ms, .. } => {
                // The first click's gap counts from before login.
                if self.clicks > 0 {
                    self.gap(gap_ms);
                }
                self.click(x.into(), y.into());
            }
            // Before 254 the input report's presses are the clicks.
            #[cfg(before_254)]
            Input::Press { x, y, .. } => {
                if let Some(last) = self.last_press_ms {
                    self.gap(timed.ms.saturating_sub(last));
                }
                self.last_press_ms = Some(timed.ms);
                self.click(x.into(), y.into());
            }
            Input::Focus(false) => self.focus_lost += 1,
            Input::Camera { .. } => self.camera_reports += 1,
            _ => {}
        }
    }

    fn click(&mut self, x: i32, y: i32) {
        self.clicks += 1;
        let near = self
            .last_cursor
            .is_some_and(|(cx, cy)| (cx - x).abs() <= JUMP_PX && (cy - y).abs() <= JUMP_PX);
        if !near {
            self.jumps += 1;
        }
        // The client moves its cursor to where it clicked.
        self.last_cursor = Some((x, y));
    }

    fn gap(&mut self, ms: u32) {
        if ms == 0 || ms > RHYTHM_MAX_MS {
            return;
        }
        if self.gaps.len() == RHYTHM_GAPS {
            self.gaps.pop_front();
        }
        self.gaps.push_back(ms);
    }

    /// The player chose a menu action, on tick `clock`.
    pub fn op(&mut self, op: Op, clock: u32, base: MapBase) {
        self.ops += 1;
        // The click arrives just before its action; allow for the read
        // limit splitting them across a tick.
        if cfg!(since_254)
            && self
                .last_click_clock
                .is_none_or(|c| clock.wrapping_sub(c) > 2)
        {
            self.ops_unclicked += 1;
        }
        if let Some((index, count, threshold)) = op_count(op, base) {
            let add = match count {
                Count::One => 1,
                Count::Quarter(v) => i64::from(v & 3 == 0),
                Count::By(v) => i64::from(v),
            };
            self.op_counts[index] += add;
            if self.op_counts[index] >= threshold {
                self.op_expected[index] += 1;
            }
        }
    }

    /// The checks' verdict so far.
    pub fn report(&self) -> ReplayReport {
        let mut flags: Vec<(&str, i16)> = Vec::new();
        if self
            .bad_heartbeats
            .iter()
            .chain(&self.bad_op_checks)
            .any(|&n| n > 0)
        {
            flags.push(("modified_client", 100));
        }
        let loop_beats: u32 = HEARTBEATS
            .iter()
            .zip(&self.heartbeats)
            .filter(|((_, from_loop), _)| *from_loop)
            .map(|(_, &n)| n)
            .sum();
        if self.ops >= MIN_OPS && loop_beats == 0 {
            flags.push(("no_heartbeat", 40));
        }
        if self.automated {
            flags.push(("automated_browser", 50));
        }
        let missing = self
            .op_expected
            .iter()
            .zip(&self.op_received)
            .any(|(&expected, &received)| expected >= 3 && received * 2 < expected);
        if missing {
            flags.push(("missing_op_checks", 40));
        }
        if cfg!(since_254) && self.ops >= MIN_OPS && self.ops_unclicked * 4 > self.ops {
            flags.push(("actions_without_clicks", 40));
        }
        if self.ops >= MIN_OPS && self.input_reports == 0 {
            flags.push(("no_mouse_movement", 30));
        }
        // 225's web client reports presses but not moves, so every click
        // looks like a jump; so does every tap.
        if cfg!(since_244)
            && !self.touch
            && self.clicks >= MIN_CLICKS
            && self.jumps * 2 > self.clicks
        {
            flags.push(("cursor_jumps", 25));
        }
        let rhythm = self.rhythm();
        if self.gaps.len() >= 30 && rhythm.is_some_and(|cv| cv < 0.12) {
            flags.push(("robotic_click_timing", 20));
        }

        let score = flags.iter().map(|&(_, s)| s).sum::<i16>();
        let stats = json!({
            "touch": self.touch,
            "automated": self.automated,
            "ops": self.ops,
            "opsUnclicked": self.ops_unclicked,
            "clicks": self.clicks,
            "cursorJumps": self.jumps,
            "clickRhythm": rhythm.map(|cv| (cv * 1000.0).round() / 1000.0),
            "inputReports": self.input_reports,
            "focusLost": self.focus_lost,
            "idleReports": self.idle_reports,
            "cameraReports": self.camera_reports,
            "heartbeats": &self.heartbeats[..HEARTBEATS.len()],
            "badHeartbeats": &self.bad_heartbeats[..HEARTBEATS.len()],
            "opChecksExpected": self.op_expected,
            "opChecksReceived": self.op_received,
            "badOpChecks": self.bad_op_checks,
        });
        ReplayReport {
            score,
            flags: flags.iter().map(|&(f, _)| f.to_owned()).collect(),
            stats: stats.to_string(),
        }
    }

    /// How even the click gaps are: their standard deviation over their
    /// mean. People land well above 0.3.
    fn rhythm(&self) -> Option<f64> {
        if self.gaps.len() < 2 {
            return None;
        }
        let n = self.gaps.len() as f64;
        let mean = self.gaps.iter().map(|&g| f64::from(g)).sum::<f64>() / n;
        let var = self
            .gaps
            .iter()
            .map(|&g| (f64::from(g) - mean).powi(2))
            .sum::<f64>()
            / n;
        Some(var.sqrt() / mean)
    }
}

/// A tile's offset into the client's map.
fn offset(pos: u16, base: u16) -> i32 {
    i32::from(pos) - i32::from(base)
}

/// Which op check (0-based) counts `op`, how, and from what count the client
/// sends it.
#[cfg(before_254)]
fn op_count(op: Op, base: MapBase) -> Option<(usize, Count, i64)> {
    Some(match op {
        Op::Npc { op: 3, nid } => (1, Count::Quarter(nid.into()), 124),
        Op::Npc { op: 5, nid } => (3, Count::Quarter(nid.into()), 85),
        Op::Loc { op: 4, x, .. } => (0, Count::Quarter(offset(x, base.x)), 99),
        Op::Loc { op: 5, .. } => (5, Count::By(base.z.into()), 92),
        Op::Player { op: 2, .. } => (7, Count::One, 90),
        Op::Held { op: 1, obj, .. } => (2, Count::By(obj.into()), 97),
        Op::Held { op: 4, slot, .. } => (4, Count::Quarter(slot.into()), 90),
        Op::InvButton { op: 4, slot, .. } => (8, Count::Quarter(slot.into()), 130),
        Op::InvButton { op: 5, com, .. } => (6, Count::Quarter(com.into()), 55),
        _ => return None,
    })
}

#[cfg(since_254)]
fn op_count(op: Op, base: MapBase) -> Option<(usize, Count, i64)> {
    Some(match op {
        Op::Obj { op: 1, x, .. } => (6, Count::Quarter(offset(x, base.x)), 123),
        Op::Obj { op: 4, z, .. } => (7, Count::By(offset(z, base.z)), 75),
        Op::Obj { op: 5, .. } => (2, Count::By(base.z.into()), 118),
        Op::Loc { op: 2, z, .. } => (0, Count::By(offset(z, base.z)), 139),
        Op::Loc { op: 3, .. } => (1, Count::One, 124),
        Op::Player { op: 1, .. } => (3, Count::One, 52),
        Op::Player { op: 4, pid } => (4, Count::By(pid.into()), 66),
        Op::Held { op: 4, .. } => (8, Count::One, 116),
        Op::InvButton { op: 1, obj, .. } => (5, Count::Quarter(obj.into()), 133),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_match_with_optional_fields() {
        let parts = [f(B(1)), opt(S(0x0203)), f(AnyB), opt(B(9))];
        assert!(matches(&parts, &[1, 2, 3, 77]));
        assert!(matches(&parts, &[1, 77]));
        assert!(matches(&parts, &[1, 2, 3, 77, 9]));
        assert!(!matches(&parts, &[2, 77]));
        assert!(!matches(&parts, &[1, 2, 3, 77, 8]));
        assert!(!matches(&parts, &[1]));
    }

    #[test]
    fn real_heartbeats_pass() {
        let mut ac = AntiCheat::default();
        #[cfg(before_254)]
        {
            ac.client(&ClientProt::AnticheatCycleLogic3, &[0x4c, 0x2b, 0x2c]);
            ac.client(&ClientProt::AnticheatCycleLogic4, &[0, 0, 0, 0]);
            // 162, 22, [84], 31824, 13490, [123], [134], 100, 94, 35521
            ac.client(
                &ClientProt::AnticheatCycleLogic6,
                &[162, 22, 0x7c, 0x50, 0x34, 0xb2, 134, 100, 94, 0x8a, 0xc1],
            );
        }
        #[cfg(since_254)]
        {
            ac.client(&ClientProt::AnticheatCycleLogic4, &[232]);
            ac.client(&ClientProt::AnticheatCycleLogic7, &[]);
            // [16791], 254, r, 16128, 52610, r, 55420, [35025], 46628, r
            ac.client(
                &ClientProt::AnticheatCycleLogic2,
                &[
                    254, 1, 2, 0x3f, 0x00, 0xcd, 0x82, 3, 4, 0xd8, 0x7c, 0xb6, 0x24, 5,
                ],
            );
        }
        let report = ac.report();
        assert!(report.flags.is_empty(), "{:?}", report.flags);
        ac.client(&ClientProt::AnticheatCycleLogic5, &[1]);
        assert_eq!(ac.report().flags, ["modified_client"]);
    }

    #[test]
    fn op_checks_are_mirrored() {
        let mut ac = AntiCheat::default();
        #[cfg(before_254)]
        let (op, check, threshold) = (
            Op::Player { op: 2, pid: 1 },
            ClientProt::AnticheatOpLogic8,
            90,
        );
        #[cfg(since_254)]
        let (op, check, threshold) = (
            Op::Loc {
                op: 3,
                id: 1,
                x: 3200,
                z: 3200,
            },
            ClientProt::AnticheatOpLogic2,
            124,
        );
        for _ in 0..threshold + 9 {
            ac.op(op, 0, MapBase { x: 3152, z: 3152 });
        }
        assert_eq!(ac.op_expected.iter().sum::<u32>(), 10);
        assert!(ac.report().flags.contains(&"missing_op_checks".to_owned()));
        let payload: &[u8] = if cfg!(since_254) {
            &[0x94, 0x42]
        } else {
            &[0x79, 0x8a]
        };
        for _ in 0..10 {
            ac.client(&check, payload);
        }
        assert!(!ac.report().flags.contains(&"missing_op_checks".to_owned()));
        assert!(!ac.report().flags.contains(&"modified_client".to_owned()));
    }

    #[test]
    fn clockwork_jumping_clicks_are_flagged() {
        let flags = clockwork_jumps(false);
        assert_eq!(
            flags.contains(&"cursor_jumps".to_owned()),
            cfg!(since_244),
            "{flags:?}"
        );
        assert!(
            flags.contains(&"robotic_click_timing".to_owned()),
            "{flags:?}"
        );
    }

    #[test]
    fn automated_browsers_are_flagged() {
        let ac = AntiCheat {
            automated: true,
            ..AntiCheat::default()
        };
        let report = ac.report();
        assert_eq!(report.flags, ["automated_browser"]);
        assert_eq!(report.score, 50);
    }

    #[test]
    fn taps_on_a_touch_screen_may_jump() {
        let flags = clockwork_jumps(true);
        assert!(!flags.contains(&"cursor_jumps".to_owned()), "{flags:?}");
        // Taps like clockwork still count.
        assert!(
            flags.contains(&"robotic_click_timing".to_owned()),
            "{flags:?}"
        );
    }

    /// The flags for 40 clicks 600 ms apart, each 300 px from the last.
    fn clockwork_jumps(touch: bool) -> Vec<String> {
        let mut ac = AntiCheat {
            touch,
            ..AntiCheat::default()
        };
        for i in 0..40 {
            let x = (100 + (i % 2) * 300) as i16;
            #[cfg(since_254)]
            let input = Input::Click {
                x,
                y: 100,
                right: false,
                gap_ms: 600,
            };
            #[cfg(before_254)]
            let input = Input::Press {
                x,
                y: 100,
                right: false,
            };
            ac.input(&Timed { ms: i * 600, input });
        }
        ac.report().flags
    }
}
