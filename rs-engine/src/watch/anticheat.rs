use super::Target;
use super::afk::AntiAfk;
use super::input::{Input, InputDecoder, Timed};
#[cfg(since_254)]
use super::paths::{MousePaths, PathStats};
use super::reaction::{ReactionStats, Reactions};
use super::repetition::{Repetition, RepetitionStats};
use super::schedule::{End, Schedule, ScheduleStats};
use super::slots::{Slot, SlotClicks, SlotStats};
use Field::*;
use rs_db::{LoginDevice, ReplayReport};
use rs_entity::{BuildArea, MAX_NPCS, MAX_PLAYERS};
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
use rs_protocol::network::game::client::move_minimapclick::MinimapTrailer;
use rs_protocol::network::game::client::unpack_coord;
use rs_protocol::network::game::client_prot::ClientProt;
use serde_json::{Map, Value, json};
use std::collections::VecDeque;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Heartbeats the game loop sends every so many cycles, in pairs: one, one
/// sent more rarely, and how many of the first come to one of the second
/// (`CYCLELOGIC4` every 111 cycles and `3` every 128; on 244-245.2 also `5`
/// every 86; from 254 `7` every 63 and `4` every 193). Whatever speed the
/// loop runs at, a real client keeps to them.
#[cfg(rev = "225")]
const LOOP_BEATS: [(usize, usize, f64); 1] = [(3, 2, 128.0 / 111.0)];
#[cfg(all(since_244, before_254))]
const LOOP_BEATS: [(usize, usize, f64); 2] = [(3, 2, 128.0 / 111.0), (4, 2, 128.0 / 86.0)];
#[cfg(since_254)]
const LOOP_BEATS: [(usize, usize, f64); 1] = [(6, 3, 193.0 / 63.0)];
/// Requests turned down that flag (and a tenth of the actions).
const MIN_REJECTED: u32 = 10;
/// Too few of the rarer heartbeat to judge their ratio by.
const MIN_LOOP_BEATS: u32 = 15;
/// From 254 `CYCLELOGIC1` comes from drawing the game (every 1175 frames):
/// a client this long into playing (in `CYCLELOGIC4`s, every 193 cycles)
/// that never sent it never drew a frame.
#[cfg(since_254)]
const DRAWN_BY: u32 = 30;

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
        com: u16,
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
/// The most steps the client sends in a path: a longer one stops short of
/// the click.
const MAX_STEPS: usize = 25;
/// Too few minimap clicks to judge where they drew the player by.
const MIN_MINIMAP: u32 = 5;
/// A minimap click that drew the player this many tiles from where the
/// world has them. The client trails the world by a tile or two.
const MINIMAP_OFF_TILES: i32 = 8;
/// A click that focuses the window is logged just before it gains focus.
#[cfg(before_254)]
const FOCUS_CLICK_MS: u32 = 100;
/// From 254 the desktop client can send that click's action up to a tick or
/// two ahead of the focus.
#[cfg(since_254)]
const FOCUS_TICKS: u32 = 2;
/// How many maps back a minimap click may have been drawn on.
const BASES: usize = 4;
/// Actions while the window is unfocused: a real client gets focus back
/// before its clicks go anywhere, but the odd one can race it.
const MIN_UNFOCUSED: u32 = 3;
/// How many ticks after the world stops showing the client something (an
/// NPC leaving view, the map moving on) its clicks on it still count as
/// real: they were made before the update reached it.
const SHOWN_GRACE: u32 = 10;
/// Actions on things the client was never sent.
const MIN_UNSEEN: u32 = 3;
/// Actions on options that need the menu (no right press to open it).
const MIN_UNPRESSED: u32 = 5;
/// How long a menu may stay open before its option is picked.
const MENU_MS: u32 = 30_000;
/// How far after an action its right press may seem to come (before 254
/// the input reports time presses a little off).
const PRESS_SLACK_MS: u32 = 3_000;
/// How long an action waits for its press to be reported before it counts
/// as having none (before 254 they're reported every 10 seconds or so).
const PRESS_WAIT_MS: u32 = 30_000;
/// Right presses kept waiting for their action.
const MAX_PRESSES: usize = 8;
/// A map's width in zones.
const MAP_ZONES: i32 = 13;
/// A session this long, with this many actions, that never turned the
/// camera. (Whether it went idle says nothing: the client's idle report
/// comes after 90 seconds without input, and logs the player out.)
const UNTURNED_MS: u32 = 3 * 3_600_000;
const UNTURNED_OPS: u32 = 300;

/// The zones of the client's map the world has sent it: one bit for each of
/// its 13 by 13.
#[derive(Clone, Copy, Debug)]
struct Zones {
    base: MapBase,
    sent: [u64; 3],
}

impl Zones {
    fn new(base: MapBase) -> Zones {
        Zones { base, sent: [0; 3] }
    }

    /// Which of the map's zones the tile is in, if it's in the map.
    fn index(&self, x: u16, z: u16) -> Option<usize> {
        let dx = offset(x, self.base.x) >> 3;
        let dz = offset(z, self.base.z) >> 3;
        ((0..MAP_ZONES).contains(&dx) && (0..MAP_ZONES).contains(&dz))
            .then(|| (dx * MAP_ZONES + dz) as usize)
    }

    fn in_map(&self, x: u16, z: u16) -> bool {
        self.index(x, z).is_some()
    }

    fn was_sent(&self, x: u16, z: u16) -> bool {
        self.index(x, z)
            .is_some_and(|i| self.sent[i >> 6] >> (i & 63) & 1 != 0)
    }

    fn send(&mut self, x: u16, z: u16) {
        if let Some(i) = self.index(x, z) {
            self.sent[i >> 6] |= 1 << (i & 63);
        }
    }
}

/// Something a check caught, for the replay's timeline: which flag it
/// counts towards, and what happened.
#[derive(Debug, PartialEq, Eq)]
pub struct Incident {
    pub flag: &'static str,
    pub detail: String,
    /// When it happened (ms into the session), if not just now: some are
    /// only judged a while after.
    pub ms: Option<u32>,
}

#[derive(Default)]
pub struct AntiCheat {
    pub inputs: InputDecoder,
    pub touch: bool,
    pub automated: bool,
    pub device: Arc<LoginDevice>,
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
    unfocused: bool,
    unfocused_actions: u32,
    #[cfg(since_254)]
    unfocused_at: Option<(u32, u32)>,
    #[cfg(since_254)]
    focus_clock: u32,
    #[cfg(before_254)]
    unfocused_press: Option<u32>,
    idle_reports: u32,
    camera_reports: u32,
    minimap_clicks: u32,
    forged_minimap: u32,
    minimap_off: u32,
    base: Option<MapBase>,
    bases: VecDeque<MapBase>,
    lost: VecDeque<(Target, u32)>,
    zones: Option<Zones>,
    prev_zones: Option<(Zones, u32)>,
    unseen: u32,
    unseen_since: Option<u32>,
    menu_ops: u32,
    menu_unpressed: u32,
    waiting_ops: VecDeque<u32>,
    right_presses: VecDeque<u32>,
    decoded: Vec<Timed>,
    pub schedule: Schedule,
    reactions: Reactions,
    repetition: Repetition,
    slots: SlotClicks,
    #[cfg(since_254)]
    paths: MousePaths,
    afk: AntiAfk,
    pub acting_on: Option<Target>,
    end: Option<(End, u32)>,
    pub auto_banned: Option<&'static str>,
    malformed: u32,
    rejected: u32,
    /// Caught since the watch last marked them.
    pub incidents: Vec<Incident>,
    /// Scripts aborted while running on the player.
    pub script_errors: u32,
    /// The session isn't recorded, so nothing would read the checks: what
    /// it does isn't fed to them.
    pub off: bool,
}

/// The checks' verdict, worked out on the tick. The recording task turns
/// it into what's stored ([`Report::stored`]), off the tick.
pub struct Report {
    pub score: i16,
    pub flags: Vec<&'static str>,
    stats: Box<Stats>,
}

/// The counts behind a [`Report`], as they were when it was made.
struct Stats {
    touch: bool,
    automated: bool,
    device: Arc<LoginDevice>,
    ops: u32,
    ops_unclicked: u32,
    clicks: u32,
    jumps: u32,
    rhythm: Option<f64>,
    input_reports: u32,
    focus_lost: u32,
    unfocused_actions: u32,
    unseen: u32,
    menu_ops: u32,
    menu_unpressed: u32,
    idle_reports: u32,
    camera_reports: u32,
    minimap_clicks: u32,
    forged_minimap: u32,
    minimap_off: u32,
    heartbeats: [u32; 7],
    beat_ratio: Option<f64>,
    bad_heartbeats: [u32; 7],
    op_expected: [u32; 9],
    op_received: [u32; 9],
    bad_op_checks: [u32; 9],
    malformed: u32,
    rejected: u32,
    script_errors: u32,
    reactions: ReactionStats,
    schedule: ScheduleStats,
    repetition: RepetitionStats,
    slots: Option<SlotStats>,
    #[cfg(since_254)]
    paths: Option<PathStats>,
    awake_ms: u32,
    auto_banned: Option<&'static str>,
    end: Option<(&'static str, u32)>,
}

impl Report {
    /// The report as stored with the recording, its stats as JSON.
    pub fn stored(self) -> ReplayReport {
        let s = &*self.stats;
        let mut more = Map::new();
        more.insert("reactions".into(), s.reactions.json());
        s.schedule.json(&mut more);
        more.insert("repetition".into(), s.repetition.json());
        if let Some(slots) = &s.slots {
            more.insert("slotClicks".into(), slots.json());
        }
        #[cfg(since_254)]
        if let Some(paths) = &s.paths {
            more.insert("mousePaths".into(), paths.json());
        }
        more.insert("keptAwakeMinutes".into(), json!(s.awake_ms / 60_000));
        if let Some(trigger) = s.auto_banned {
            more.insert("autoBan".into(), json!(trigger));
        }
        if let Some((end, at)) = s.end {
            more.insert("endReason".into(), json!(end));
            more.insert("endMs".into(), json!(at));
        }
        let mut stats = json!({
            "touch": s.touch,
            "automated": s.automated,
            "web": s.device.web,
            "userAgent": s.device.user_agent,
            "platform": s.device.platform,
            "language": s.device.language,
            "engine": s.device.engine,
            "jsPlatform": s.device.js_platform,
            "gpu": s.device.gpu,
            "tampering": s.device.tampering(),
            "ops": s.ops,
            "opsUnclicked": s.ops_unclicked,
            "clicks": s.clicks,
            "cursorJumps": s.jumps,
            "clickRhythm": s.rhythm.map(|cv| (cv * 1000.0).round() / 1000.0),
            "inputReports": s.input_reports,
            "focusLost": s.focus_lost,
            "unfocusedActions": s.unfocused_actions,
            "unseenTargets": s.unseen,
            "menuOps": s.menu_ops,
            "menuUnpressed": s.menu_unpressed,
            "idleReports": s.idle_reports,
            "cameraReports": s.camera_reports,
            "minimapClicks": s.minimap_clicks,
            "forgedMinimapClicks": s.forged_minimap,
            "minimapOff": s.minimap_off,
            "heartbeats": &s.heartbeats[..HEARTBEATS.len()],
            "heartbeatRatio": s.beat_ratio.map(|r| (r * 100.0).round() / 100.0),
            "badHeartbeats": &s.bad_heartbeats[..HEARTBEATS.len()],
            "opChecksExpected": s.op_expected,
            "opChecksReceived": s.op_received,
            "badOpChecks": s.bad_op_checks,
            "malformedPackets": s.malformed,
            "rejectedRequests": s.rejected,
            "scriptErrors": s.script_errors,
        });
        if let Value::Object(stats) = &mut stats {
            stats.extend(more);
        }
        ReplayReport {
            score: self.score,
            flags: self.flags.iter().map(|&f| f.to_owned()).collect(),
            stats: stats.to_string(),
        }
    }
}

impl AntiCheat {
    /// A packet from the client. (Input reports come in decoded, by their
    /// handlers.)
    pub fn client(&mut self, prot: &ClientProt, data: &[u8]) {
        if self.off {
            return;
        }
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
            ClientProt::IdleTimer => {
                self.idle_reports += 1;
                self.afk.idle();
            }
            _ => {}
        }
    }

    /// An `EVENT_MOUSE_MOVE`, `ms` into the session.
    #[cfg(since_254)]
    pub fn mouse_move(&mut self, ms: u32, msg: &EventMouseMove) {
        if self.off {
            return;
        }
        self.input_reports += 1;
        self.inputs_from(|inputs, out| inputs.mouse_move(ms, msg, out));
    }

    /// An `EVENT_MOUSE_CLICK`, `ms` into the session on tick `clock`.
    #[cfg(since_254)]
    pub fn mouse_click(&mut self, ms: u32, clock: u32, msg: &EventMouseClick) {
        if self.off {
            return;
        }
        self.last_click_clock = Some(clock);
        self.inputs_from(|inputs, out| inputs.mouse_click(ms, msg, out));
    }

    /// An `EVENT_APPLET_FOCUS`, `ms` into the session.
    #[cfg(since_254)]
    pub fn focus(&mut self, ms: u32, clock: u32, msg: &EventAppletFocus) {
        if self.off {
            return;
        }
        self.focus_clock = clock;
        self.inputs_from(|inputs, out| inputs.focus(ms, msg, out));
    }

    /// An `EVENT_CAMERA_POSITION`, `ms` into the session.
    #[cfg(any(rev = "225", since_254))]
    pub fn camera(&mut self, ms: u32, msg: &EventCameraPosition) {
        if self.off {
            return;
        }
        self.inputs_from(|inputs, out| inputs.camera(ms, msg, out));
    }

    /// An `EVENT_TRACKING`, `ms` into the session. Before 254 it's the only
    /// input report.
    #[cfg(before_274)]
    pub fn tracking(&mut self, ms: u32, msg: &EventTracking) {
        if self.off {
            return;
        }
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

    /// Notes what a check caught, just now.
    fn caught(&mut self, flag: &'static str, detail: impl Into<String>) {
        self.incidents.push(Incident {
            flag,
            detail: detail.into(),
            ms: None,
        });
    }

    fn heartbeat(&mut self, index: usize, data: &[u8]) {
        self.heartbeats[index] += 1;
        if !matches(HEARTBEATS[index].0, data) {
            self.bad_heartbeats[index] += 1;
            self.caught(
                "modified_client",
                format!(
                    "A heartbeat (CYCLELOGIC{}) the real client never sends",
                    index + 1
                ),
            );
        }
    }

    fn op_check(&mut self, index: usize, data: &[u8]) {
        self.op_received[index] += 1;
        if !matches(OP_CHECKS[index], data) {
            self.bad_op_checks[index] += 1;
            self.caught(
                "modified_client",
                format!(
                    "An op check (OPLOGIC{}) the real client never sends",
                    index + 1
                ),
            );
        }
    }

    fn input(&mut self, timed: &Timed) {
        match timed.input {
            Input::Move { x, y } => {
                self.last_cursor = Some((x.into(), y.into()));
                self.slots.moved();
                #[cfg(since_254)]
                self.paths.moved(timed.ms, x, y);
                self.afk.moved(timed.ms, x, y);
            }
            Input::Outside => {
                self.last_cursor = None;
                #[cfg(since_254)]
                self.paths.outside();
            }
            Input::Click {
                x,
                y,
                gap_ms,
                right,
                ..
            } => {
                #[cfg(since_254)]
                self.paths.click();
                self.slots.press(timed.ms, x, y, right);
                self.afk.activity(timed.ms);
                if right {
                    self.right_press(timed.ms);
                }
                // The first click's gap counts from before login.
                if self.clicks > 0 {
                    self.gap(gap_ms);
                }
                self.click(x.into(), y.into());
            }
            // Before 254 the input report's presses are the clicks.
            #[cfg(before_254)]
            Input::Press { x, y, right } => {
                self.slots.press(timed.ms, x, y, right);
                self.afk.activity(timed.ms);
                if right {
                    self.right_press(timed.ms);
                }
                if let Some(last) = self.last_press_ms {
                    self.gap(timed.ms.saturating_sub(last));
                }
                self.last_press_ms = Some(timed.ms);
                self.click(x.into(), y.into());
                if self.unfocused {
                    self.unfocused_actions += 1;
                    self.unfocused_press = Some(timed.ms);
                }
            }
            Input::Focus(focus) => {
                self.unfocused = !focus;
                if !focus {
                    self.focus_lost += 1;
                }
                // The actions just before were the click that focused it.
                #[cfg(since_254)]
                if focus
                    && let Some((clock, count)) = self.unfocused_at.take()
                    && self.focus_clock.wrapping_sub(clock) <= FOCUS_TICKS
                {
                    self.unfocused_actions -= count;
                }
                // The press was the click that focused it.
                #[cfg(before_254)]
                if focus
                    && self
                        .unfocused_press
                        .take()
                        .is_some_and(|ms| timed.ms.saturating_sub(ms) <= FOCUS_CLICK_MS)
                {
                    self.unfocused_actions -= 1;
                }
            }
            Input::Camera { .. } => {
                self.camera_reports += 1;
                self.afk.camera(timed.ms);
            }
            Input::KeyDown(key) => self.afk.key(timed.ms, key),
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
        if self.off {
            return;
        }
        self.see_base(base);
        self.ops += 1;
        // The click arrives just before its action; allow for the read
        // limit splitting them across a tick.
        if cfg!(since_254)
            && self
                .last_click_clock
                .is_none_or(|c| clock.wrapping_sub(c) > 2)
        {
            self.ops_unclicked += 1;
            self.caught(
                "actions_without_clicks",
                "An action with no click before it",
            );
        }
        // From 254 focus has a report of its own, sent ahead of the actions
        // in the same cycle; before, presses are judged in the input reports.
        #[cfg(since_254)]
        if self.unfocused {
            self.unfocused_actions += 1;
            let count = match self.unfocused_at {
                Some((last, count)) if clock.wrapping_sub(last) <= FOCUS_TICKS => count + 1,
                _ => 1,
            };
            self.unfocused_at = Some((clock, count));
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

    /// A minimap click: its `path`, and the bytes the client sends after it.
    /// `at` is the tile the world has the player on.
    pub fn minimap(
        &mut self,
        path: &[u32],
        trailer: &MinimapTrailer,
        base: MapBase,
        at: (u16, u16),
    ) {
        if self.off {
            return;
        }
        self.see_base(base);
        self.minimap_clicks += 1;
        let (tile_x, tile_z) = minimap_tile(trailer);
        // The path ends on the tile clicked, or next to it if that can't be
        // reached (allowing a tile for Java's slightly different sine table).
        let reach = 1 + i32::from(trailer.nearest);
        let lands_from = |b: &MapBase| {
            path.last().is_some_and(|&to| {
                let (x, z) = unpack_coord(to);
                (i32::from(x) - i32::from(b.x) - tile_x).abs() <= reach
                    && (i32::from(z) - i32::from(b.z) - tile_z).abs() <= reach
            })
        };
        let matched = std::iter::once(base)
            .chain(self.bases.iter().rev().copied())
            .find(lands_from);
        let lands = path.len() >= MAX_STEPS || matched.is_some();
        if !plausible(trailer) || !lands {
            self.forged_minimap += 1;
            self.caught(
                "forged_minimap_click",
                if lands {
                    "A minimap click with bytes the real client never sends"
                } else {
                    "A minimap click whose path doesn't end where it was clicked"
                },
            );
        }
        let b = matched.unwrap_or(base);
        let drawn_x = (i32::from(trailer.fine_x) >> 7) + i32::from(b.x);
        let drawn_z = (i32::from(trailer.fine_z) >> 7) + i32::from(b.z);
        let off = (drawn_x - i32::from(at.0))
            .abs()
            .max((drawn_z - i32::from(at.1)).abs());
        if off > MINIMAP_OFF_TILES {
            self.minimap_off += 1;
            self.caught(
                "minimap_position_off",
                format!("A minimap click that drew the player {off} tiles from where they are"),
            );
        }
    }

    /// The world stopped showing the client an NPC or player, on tick
    /// `clock`.
    pub fn lost(&mut self, what: Target, clock: u32) {
        if self.off {
            return;
        }
        while self
            .lost
            .front()
            .is_some_and(|&(_, c)| clock.wrapping_sub(c) > SHOWN_GRACE)
        {
            self.lost.pop_front();
        }
        self.lost.push_back((what, clock));
    }

    /// The client was sent a new map, from `base`, on tick `clock`.
    pub fn rebuilt(&mut self, base: MapBase, clock: u32) {
        if self.off {
            return;
        }
        self.see_base(base);
        if let Some(zones) = self.zones.replace(Zones::new(base)) {
            self.prev_zones = Some((zones, clock));
        }
    }

    /// The client was sent the zone with tile `(x, z)`, objs and all.
    pub fn zone_sent(&mut self, x: u16, z: u16) {
        if self.off {
            return;
        }
        if let Some(zones) = &mut self.zones {
            zones.send(x, z);
        }
    }

    /// An action on `on`, on tick `clock`: the real client can only click
    /// what it was shown. `area` is what the world is showing it now.
    pub fn target(&mut self, on: Target, area: &BuildArea, clock: u32) {
        if self.off {
            return;
        }
        let lately = |f: &dyn Fn(&Zones) -> bool| {
            self.zones.as_ref().is_none_or(f)
                || self
                    .prev_zones
                    .as_ref()
                    .is_some_and(|(z, c)| clock.wrapping_sub(*c) <= SHOWN_GRACE && f(z))
        };
        // The id comes from the client: `contains` doesn't check its range.
        let shown = match on {
            Target::Npc(nid) => {
                (nid as usize) < MAX_NPCS && area.npcs.contains(nid) || self.lately_lost(on, clock)
            }
            Target::Player(pid) => {
                (pid as usize) < MAX_PLAYERS && area.players.contains(pid)
                    || self.lately_lost(on, clock)
            }
            // Locs are in the map the client loads; objs only come with the
            // zones it's sent.
            Target::Loc { x, z, .. } => lately(&|zones| zones.in_map(x, z)),
            Target::Obj { x, z, .. } => lately(&|zones| zones.was_sent(x, z)),
            Target::Item(_) => true,
        };
        if !shown
            && self
                .unseen_since
                .is_none_or(|since| clock.wrapping_sub(since) > SHOWN_GRACE)
        {
            self.unseen += 1;
            self.unseen_since = Some(clock);
            let what = match on {
                Target::Npc(_) => "an NPC".to_owned(),
                Target::Player(_) => "a player".to_owned(),
                Target::Loc { id, x, z } => format!("loc {id} at {x}, {z}"),
                Target::Obj { id, x, z } => format!("obj {id} on the floor at {x}, {z}"),
                Target::Item(_) => "an item".to_owned(),
            };
            self.caught(
                "clicked_unseen",
                format!("Acted on {what}, which the client was never sent"),
            );
        }
    }

    /// An action, `ms` into the session, on an option a left click doesn't
    /// pick: the menu was opened for it, by a right press just before.
    pub fn menu_op(&mut self, ms: u32) {
        if self.off {
            return;
        }
        self.menu_ops += 1;
        self.expire_menu_ops(ms);
        let pressed = self
            .right_presses
            .iter()
            .position(|&p| p <= ms + PRESS_SLACK_MS && ms <= p + MENU_MS);
        match pressed {
            Some(i) => {
                self.right_presses.remove(i);
            }
            None => self.waiting_ops.push_back(ms),
        }
    }

    fn right_press(&mut self, ms: u32) {
        let waiting = self
            .waiting_ops
            .iter()
            .position(|&op| ms <= op + PRESS_SLACK_MS && op <= ms + MENU_MS);
        if let Some(i) = waiting {
            self.waiting_ops.remove(i);
            return;
        }
        if self.right_presses.len() == MAX_PRESSES {
            self.right_presses.pop_front();
        }
        self.right_presses.push_back(ms);
        self.expire_menu_ops(ms);
    }

    /// Actions whose press hasn't come by now never had one.
    fn expire_menu_ops(&mut self, now: u32) {
        while self
            .waiting_ops
            .front()
            .is_some_and(|&op| now > op + PRESS_WAIT_MS)
        {
            let ms = self.waiting_ops.pop_front();
            self.menu_unpressed += 1;
            self.incidents.push(Incident {
                flag: "menu_without_right_click",
                detail: "A menu option picked without a right click to open the menu".into(),
                ms,
            });
        }
    }

    /// A packet shorter than any the client sends of its kind.
    pub fn malformed(&mut self) {
        self.malformed += 1;
        self.caught(
            "modified_client",
            "A packet shorter than any the real client sends",
        );
    }

    /// A request the world turned down as one the client wouldn't make.
    pub fn rejected(&mut self) {
        self.rejected += 1;
        self.caught("rejected_requests", "A request the world turned down");
    }

    /// An action that arrived `ms` into the session, for how fast the
    /// player reacts and how long they go without acting. `appeared_ms`:
    /// when the obj it's on appeared, if it's on one that did lately.
    pub fn acted(&mut self, ms: u32, appeared_ms: Option<u32>) {
        if self.off {
            return;
        }
        self.reactions.action(ms, appeared_ms);
        self.afk.activity(ms);
    }

    /// What the player last acted on went away, `ms` into the session.
    pub fn gone(&mut self, ms: u32) {
        if self.off {
            return;
        }
        self.acting_on = None;
        self.reactions.gone(ms);
    }

    /// The player chatted.
    pub fn chatted(&mut self, ms: u32) {
        if self.off {
            return;
        }
        self.afk.activity(ms);
    }

    /// An action, as anything that tells it apart, to see how much the
    /// player repeats themselves.
    pub fn repeat(&mut self, what: impl Hash) {
        if self.off {
            return;
        }
        self.repetition.action(what);
    }

    /// A walk to `(x, z)`.
    pub fn walked(&mut self, x: u16, z: u16) {
        if self.off {
            return;
        }
        self.repetition.walk(x, z);
    }

    /// An action on the obj in an inventory's `slot` (its top left on the
    /// screen). `menu`: an option the menu was opened for.
    pub fn slot_action(&mut self, ms: u32, slot: Slot, menu: bool) {
        if self.off {
            return;
        }
        self.slots.action(ms, slot, menu);
    }

    /// The session is ending, `ms` into it (for a lost connection, when it
    /// was lost). The first reason given stands.
    pub fn ended(&mut self, end: End, ms: u32) {
        self.end.get_or_insert((end, ms));
    }

    /// The client reconnected: the session goes on.
    pub fn resumed(&mut self) {
        self.end = None;
        // The client only reports focus when it changes, and a report into
        // the old connection was lost.
        self.unfocused = false;
    }

    fn lately_lost(&self, what: Target, clock: u32) -> bool {
        self.lost
            .iter()
            .any(|&(t, c)| t == what && clock.wrapping_sub(c) <= SHOWN_GRACE)
    }

    fn see_base(&mut self, base: MapBase) {
        if self.base == Some(base) {
            return;
        }
        if let Some(old) = self.base.replace(base) {
            if self.bases.len() == BASES {
                self.bases.pop_front();
            }
            self.bases.push_back(old);
        }
    }

    /// The checks' verdict so far, `ms` into the session.
    pub fn report(&self, ms: u32) -> Report {
        let mut flags: Vec<(&str, i16)> = Vec::new();
        if self.malformed > 0
            || self
                .bad_heartbeats
                .iter()
                .chain(&self.bad_op_checks)
                .any(|&n| n > 0)
        {
            flags.push(("modified_client", 100));
        }
        // The odd one races the world (an interface closing as it's
        // clicked); a stream of them is something sending its own.
        if self.rejected >= MIN_REJECTED && self.rejected * 10 > self.ops {
            flags.push(("rejected_requests", 30));
        }
        if self.auto_banned.is_some() {
            flags.push(("auto_banned", 100));
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
        let beat_ratio = self.beat_ratio();
        if self.beats_forged() {
            flags.push(("forged_heartbeats", 100));
        }
        #[cfg(since_254)]
        if self.ops >= MIN_OPS && self.heartbeats[3] >= DRAWN_BY && self.heartbeats[0] == 0 {
            flags.push(("never_drew", 40));
        }
        if self.automated {
            flags.push(("automated_browser", 50));
        }
        // A web client whose device check was forged, or whose browser's
        // claims disagree with what its page measured; or one that sent no
        // check at all (its script blocked, or the check stripped).
        let tampering = self.device.tampering();
        if tampering.iter().any(|&t| t != "no_check") {
            flags.push(("spoofed_browser", 50));
        } else if !tampering.is_empty() {
            flags.push(("device_check_missing", 30));
        }
        let missing = self
            .op_expected
            .iter()
            .zip(&self.op_received)
            .any(|(&expected, &received)| expected >= 3 && (received << 1) < expected);
        if missing {
            flags.push(("missing_op_checks", 40));
        }
        if cfg!(since_254) && self.ops >= MIN_OPS && (self.ops_unclicked << 2) > self.ops {
            flags.push(("actions_without_clicks", 40));
        }
        // A phone's keyboard takes focus from the game while it's open.
        if !self.touch && self.unfocused_actions >= MIN_UNFOCUSED {
            flags.push(("actions_while_unfocused", 40));
        }
        if self.ops >= MIN_OPS && self.input_reports == 0 {
            flags.push(("no_mouse_movement", 30));
        }
        // 225's web client reports presses but not moves, so every click
        // looks like a jump; so does every tap.
        if cfg!(since_244)
            && !self.touch
            && self.clicks >= MIN_CLICKS
            && (self.jumps << 1) > self.clicks
        {
            flags.push(("cursor_jumps", 25));
        }
        if self.unseen >= MIN_UNSEEN {
            flags.push(("clicked_unseen", 40));
        }
        // Options still waiting for their press that it's too late for. From
        // 254 the press comes first, so once the session is over that's all
        // of them; before, the last input report may have gone with it.
        let ended = cfg!(since_254) && self.end.is_some();
        let menu_unpressed = self.menu_unpressed
            + self
                .waiting_ops
                .iter()
                .filter(|&&op| ended || ms > op + PRESS_WAIT_MS)
                .count() as u32;
        if !self.touch && menu_unpressed >= MIN_UNPRESSED && (menu_unpressed << 1) > self.menu_ops {
            flags.push(("menu_without_right_click", 40));
        }
        if self.forged_minimap >= 2 {
            flags.push(("forged_minimap_click", 100));
        }
        if self.minimap_clicks >= MIN_MINIMAP && (self.minimap_off << 1) > self.minimap_clicks {
            flags.push(("minimap_position_off", 30));
        }
        let rhythm = self.rhythm();
        if self.gaps.len() >= 30 && rhythm.is_some_and(|cv| cv < 0.12) {
            flags.push(("robotic_click_timing", 20));
        }
        // How the player behaves over time.
        let reactions = self.reactions.report(&mut flags);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        let schedule = self.schedule.report(now, &mut flags);
        let repetition = self.repetition.report(&mut flags);
        // A tap lands wherever the finger covers, and moves no cursor.
        let slots = (!self.touch).then(|| self.slots.report(&mut flags));
        #[cfg(since_254)]
        let paths = (!self.touch).then(|| self.paths.report(&mut flags));
        let awake_ms = self.afk.report(ms, &mut flags);
        // Only 225 and 254+ report the camera.
        if cfg!(any(rev = "225", since_254))
            && ms >= UNTURNED_MS
            && self.ops >= UNTURNED_OPS
            && self.camera_reports == 0
        {
            flags.push(("camera_never_turned", 15));
        }

        Report {
            score: flags.iter().map(|&(_, s)| s).sum(),
            flags: flags.iter().map(|&(f, _)| f).collect(),
            stats: Box::new(Stats {
                touch: self.touch,
                automated: self.automated,
                device: Arc::clone(&self.device),
                ops: self.ops,
                ops_unclicked: self.ops_unclicked,
                clicks: self.clicks,
                jumps: self.jumps,
                rhythm,
                input_reports: self.input_reports,
                focus_lost: self.focus_lost,
                unfocused_actions: self.unfocused_actions,
                unseen: self.unseen,
                menu_ops: self.menu_ops,
                menu_unpressed,
                idle_reports: self.idle_reports,
                camera_reports: self.camera_reports,
                minimap_clicks: self.minimap_clicks,
                forged_minimap: self.forged_minimap,
                minimap_off: self.minimap_off,
                heartbeats: self.heartbeats,
                beat_ratio,
                bad_heartbeats: self.bad_heartbeats,
                op_expected: self.op_expected,
                op_received: self.op_received,
                bad_op_checks: self.bad_op_checks,
                malformed: self.malformed,
                rejected: self.rejected,
                script_errors: self.script_errors,
                reactions,
                schedule,
                repetition,
                slots,
                #[cfg(since_254)]
                paths,
                awake_ms,
                auto_banned: self.auto_banned,
                end: self.end.map(|(end, at)| (end.name(), at)),
            }),
        }
    }

    /// Whether the game loop's heartbeats are off its beat: more than 3 (and
    /// 6%) from what the rarer one's count makes the other's. A real
    /// client's land within a count or two; starting from counters a past
    /// session left part way makes up to one more.
    fn beats_forged(&self) -> bool {
        LOOP_BEATS.iter().any(|&(often, rarely, ratio)| {
            let expected = f64::from(self.heartbeats[rarely]) * ratio;
            self.heartbeats[rarely] >= MIN_LOOP_BEATS
                && (f64::from(self.heartbeats[often]) - expected).abs() > 3.0 + expected * 0.06
        })
    }

    /// How many of the game loop's most frequent heartbeat came to one of
    /// the rarer, once there are enough to tell.
    fn beat_ratio(&self) -> Option<f64> {
        let (often, rarely, _) = LOOP_BEATS[0];
        (self.heartbeats[rarely] >= MIN_LOOP_BEATS)
            .then(|| f64::from(self.heartbeats[often]) / f64::from(self.heartbeats[rarely]))
    }

    /// How even the click gaps are: their standard deviation over their
    /// mean. People land well above 0.3.
    fn rhythm(&self) -> Option<f64> {
        cv(self.gaps.iter().map(|&g| f64::from(g)))
    }
}

/// How spread out `values` are: their standard deviation over their mean.
/// `None` for fewer than two, or a mean of 0. One pass (Welford's), so
/// nothing is collected.
pub(super) fn cv(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut spread = Spread::default();
    for v in values {
        spread.push(v);
    }
    spread.cv()
}

/// Values' count, mean and squared deviations so far (Welford's), for
/// their [`cv`] without keeping them.
#[derive(Clone, Copy, Default)]
pub(super) struct Spread {
    n: f64,
    mean: f64,
    m2: f64,
}

impl Spread {
    pub(super) fn push(&mut self, v: f64) {
        self.n += 1.0;
        let d = v - self.mean;
        self.mean += d / self.n;
        self.m2 += d * (v - self.mean);
    }

    /// See [`cv`].
    pub(super) fn cv(&self) -> Option<f64> {
        if self.n < 2.0 || self.mean <= 0.0 {
            return None;
        }
        Some((self.m2 / self.n).sqrt() / self.mean)
    }
}

/// A tile's offset into the client's map.
/// Whether a minimap click's trailer is one the real client could send: its
/// fixed bytes, and everything else in the range the client keeps it to.
pub fn plausible(t: &MinimapTrailer) -> bool {
    (t.magic1, t.magic2, t.magic3) == (57, 89, 63)
        && (-73..=72).contains(&t.x)
        && (-75..=75).contains(&t.y)
        && t.yaw < 2048
        && (-62..=62).contains(&t.angle)
        && (-21..=11).contains(&t.zoom)
        && t.nearest <= 1
}

/// The tile in the client's map a minimap click lands on, worked out as the
/// client does from what it sent.
fn minimap_tile(t: &MinimapTrailer) -> (i32, i32) {
    let yaw = (i32::from(t.yaw) + i32::from(t.angle)) & 0x7ff;
    let scale = i32::from(t.zoom) + 256;
    let sin = (table(f64::sin, yaw) * scale) >> 8;
    let cos = (table(f64::cos, yaw) * scale) >> 8;
    let (x, y) = (i32::from(t.x), i32::from(t.y));
    let right = (y * sin + x * cos) >> 11;
    let up = (y * cos - x * sin) >> 11;
    (
        (i32::from(t.fine_x) + right) >> 7,
        (i32::from(t.fine_z) - up) >> 7,
    )
}

/// The client's sine or cosine table entry `i` (2048 to a turn, 16.16).
fn table(f: fn(f64) -> f64, i: i32) -> i32 {
    (f(f64::from(i) * std::f64::consts::PI / 1024.0) * 65536.0) as i32
}

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
    use rs_db::DeviceCheck;
    use rs_protocol::network::game::client::pack_coord;

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
        let report = ac.report(0);
        assert!(report.flags.is_empty(), "{:?}", report.flags);
        ac.client(&ClientProt::AnticheatCycleLogic5, &[1]);
        assert_eq!(ac.report(0).flags, ["modified_client"]);
    }

    #[test]
    fn heartbeats_off_the_loop_s_beat_are_forged() {
        let beat = |index: usize| match index {
            0 => ClientProt::AnticheatCycleLogic1,
            2 => ClientProt::AnticheatCycleLogic3,
            3 => ClientProt::AnticheatCycleLogic4,
            4 => ClientProt::AnticheatCycleLogic5,
            _ => {
                #[cfg(since_254)]
                {
                    ClientProt::AnticheatCycleLogic7
                }
                #[cfg(before_254)]
                unreachable!()
            }
        };
        let payload = |index: usize| -> Vec<u8> {
            match (index, cfg!(since_254)) {
                (2, false) => vec![0x4c, 0x2b, 0x2c],
                (3, false) => vec![0; 4],
                (3, true) => vec![232],
                _ => vec![],
            }
        };
        // 40 of the rarest, and the rest at `per` times the real ratio.
        let run = |per: f64| {
            let mut ac = AntiCheat::default();
            let rarely = LOOP_BEATS[0].1;
            for i in 0..40 {
                ac.client(&beat(rarely), &payload(rarely));
                for &(often, _, ratio) in &LOOP_BEATS {
                    let r = ratio * per;
                    let due = ((i + 1) as f64 * r) as usize - (i as f64 * r) as usize;
                    for _ in 0..due {
                        ac.client(&beat(often), &payload(often));
                    }
                }
            }
            ac.report(0).flags
        };
        assert!(run(1.0).is_empty(), "{:?}", run(1.0));
        // A bot sending them all as often.
        assert_eq!(run(1.0 / LOOP_BEATS[0].2), ["forged_heartbeats"]);
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
        assert!(ac.report(0).flags.contains(&"missing_op_checks"));
        let payload: &[u8] = if cfg!(since_254) {
            &[0x94, 0x42]
        } else {
            &[0x79, 0x8a]
        };
        for _ in 0..10 {
            ac.client(&check, payload);
        }
        assert!(!ac.report(0).flags.contains(&"missing_op_checks"));
        assert!(!ac.report(0).flags.contains(&"modified_client"));
    }

    #[test]
    fn clockwork_jumping_clicks_are_flagged() {
        let flags = clockwork_jumps(false);
        assert_eq!(
            flags.contains(&"cursor_jumps"),
            cfg!(since_244),
            "{flags:?}"
        );
        assert!(flags.contains(&"robotic_click_timing"), "{flags:?}");
    }

    #[test]
    fn automated_browsers_are_flagged() {
        let ac = AntiCheat {
            automated: true,
            ..AntiCheat::default()
        };
        let report = ac.report(0);
        assert_eq!(report.flags, ["automated_browser"]);
        assert_eq!(report.score, 50);
    }

    #[test]
    fn spoofed_browsers_are_flagged() {
        let spoofed = |check, agent_differs| AntiCheat {
            device: Arc::new(LoginDevice {
                web: true,
                user_agent: Some("Mozilla/5.0 (Windows NT 10.0) Chrome/141.0".into()),
                engine: Some("blink".into()),
                check: Some(check),
                agent_differs,
                ..LoginDevice::default()
            }),
            ..AntiCheat::default()
        };
        let report = spoofed(DeviceCheck::Sealed, false).report(0);
        assert!(report.flags.is_empty(), "{:?}", report.flags);
        let report = spoofed(DeviceCheck::Sealed, true).report(0);
        assert_eq!((report.flags, report.score), (vec!["spoofed_browser"], 50));
        let report = spoofed(DeviceCheck::Broken, false).report(0);
        assert_eq!(report.flags, ["spoofed_browser"]);
        let report = spoofed(DeviceCheck::Missing, false).report(0);
        assert_eq!(
            (report.flags.clone(), report.score),
            (vec!["device_check_missing"], 30)
        );
        let stats: Value = serde_json::from_str(&report.stored().stats).unwrap();
        assert_eq!(stats["tampering"], json!(["no_check"]));
    }

    #[test]
    fn stored_reports_keep_their_stats() {
        let ac = AntiCheat {
            automated: true,
            ..AntiCheat::default()
        };
        let stored = ac.report(0).stored();
        assert_eq!(stored.flags, ["automated_browser"]);
        let stats: Value = serde_json::from_str(&stored.stats).unwrap();
        assert_eq!(stats["automated"], true);
        for key in [
            "heartbeats",
            "reactions",
            "schedule",
            "returnMs",
            "repetition",
            "slotClicks",
            "keptAwakeMinutes",
        ] {
            assert!(stats.get(key).is_some(), "{key} missing: {stats}");
        }
    }

    #[test]
    fn taps_on_a_touch_screen_may_jump() {
        let flags = clockwork_jumps(true);
        assert!(!flags.contains(&"cursor_jumps"), "{flags:?}");
        // Taps like clockwork still count.
        assert!(flags.contains(&"robotic_click_timing"), "{flags:?}");
    }

    /// A minimap click 10 px right of and 20 px above the centre, camera
    /// facing north, untilted and unzoomed: 10 * 32 and 20 * 32 fine units
    /// from the player drawn mid-tile at (52, 52) in a map from (3152, 3152).
    fn minimap_click() -> (Vec<u32>, MinimapTrailer) {
        let trailer = MinimapTrailer {
            x: 10,
            y: -20,
            yaw: 0,
            magic1: 57,
            angle: 0,
            zoom: 0,
            magic2: 89,
            fine_x: (52 << 7) + 64,
            fine_z: (52 << 7) + 64,
            nearest: 0,
            magic3: 63,
        };
        let path = vec![pack_coord(3205, 3205), pack_coord(3207, 3209)];
        (path, trailer)
    }

    const BASE: MapBase = MapBase { x: 3152, z: 3152 };

    #[test]
    fn real_minimap_clicks_pass() {
        let mut ac = AntiCheat::default();
        let (path, trailer) = minimap_click();
        assert_eq!(minimap_tile(&trailer), (55, 57));
        for _ in 0..10 {
            ac.minimap(&path, &trailer, BASE, (3204, 3204));
        }
        assert!(ac.report(0).flags.is_empty(), "{:?}", ac.report(0).flags);
        // Just after the world moved the map on, the click still lands from
        // the old one.
        ac.see_base(MapBase { x: 3160, z: 3152 });
        ac.minimap(&path, &trailer, MapBase { x: 3160, z: 3152 }, (3204, 3204));
        assert_eq!(ac.forged_minimap, 0);
    }

    #[test]
    fn forged_minimap_clicks_are_flagged() {
        let (path, trailer) = minimap_click();
        let forgeries = [
            MinimapTrailer::default(),
            MinimapTrailer {
                magic3: 64,
                ..trailer
            },
            MinimapTrailer {
                angle: 100,
                ..trailer
            },
            // A click somewhere the path doesn't go.
            MinimapTrailer { x: -40, ..trailer },
        ];
        for forged in forgeries {
            let mut ac = AntiCheat::default();
            ac.minimap(&path, &forged, BASE, (3204, 3204));
            assert!(ac.report(0).flags.is_empty(), "one is a fluke");
            ac.minimap(&path, &forged, BASE, (3204, 3204));
            assert_eq!(ac.report(0).flags, ["forged_minimap_click"], "{forged:?}");
        }
        // Over 25 steps the path stops short, so only the bytes are judged.
        let mut ac = AntiCheat::default();
        let long: Vec<u32> = (0..25).map(|i| pack_coord(3205 + i, 3205)).collect();
        for _ in 0..2 {
            ac.minimap(&long, &trailer, BASE, (3204, 3204));
        }
        assert!(ac.report(0).flags.is_empty());
    }

    #[test]
    fn minimap_clicks_far_from_the_player_are_flagged() {
        let mut ac = AntiCheat::default();
        let (path, trailer) = minimap_click();
        for _ in 0..5 {
            ac.minimap(&path, &trailer, BASE, (3204 + 20, 3204));
        }
        assert_eq!(ac.report(0).flags, ["minimap_position_off"]);
    }

    #[test]
    fn actions_on_what_the_client_was_never_sent_are_flagged() {
        let mut ac = AntiCheat::default();
        let mut area = BuildArea::new();
        area.npcs.insert(5);
        ac.rebuilt(BASE, 0);
        ac.zone_sent(3200, 3200);
        ac.lost(Target::Npc(7), 0);
        // What it was shown: a tracked NPC, one that just left view, a loc
        // in its map and an obj in a zone it was sent.
        let shown = [
            Target::Npc(5),
            Target::Npc(7),
            Target::Loc {
                id: 1,
                x: 3160,
                z: 3250,
            },
            Target::Obj {
                id: 1,
                x: 3201,
                z: 3203,
            },
        ];
        for on in shown {
            ac.target(on, &area, 5);
        }
        // Just after the map moved on, the old one still counts.
        ac.rebuilt(MapBase { x: 3400, z: 3400 }, 6);
        ac.target(shown[2], &area, 10);
        assert_eq!(ac.unseen, 0);

        let unseen = [
            Target::Npc(9),
            // Long gone.
            Target::Npc(7),
            Target::Loc {
                id: 1,
                x: 3160,
                z: 3250,
            },
            // In the map, but its zone was never sent.
            Target::Obj {
                id: 1,
                x: 3410,
                z: 3410,
            },
        ];
        // Each in a stretch of its own: several at once are one late update.
        for (i, on) in unseen.into_iter().enumerate() {
            ac.target(on, &area, 30 + i as u32 * (SHOWN_GRACE + 1));
            ac.target(on, &area, 31 + i as u32 * (SHOWN_GRACE + 1));
        }
        assert_eq!(ac.unseen, 4);
        assert_eq!(ac.report(0).flags, ["clicked_unseen"]);
    }

    #[test]
    fn ids_past_the_end_are_unseen() {
        let mut ac = AntiCheat::default();
        let area = BuildArea::new();
        ac.target(Target::Player(u16::MAX), &area, 0);
        ac.target(Target::Npc(u16::MAX), &area, SHOWN_GRACE + 1);
        assert_eq!(ac.unseen, 2);
    }

    /// A right press `ms` into the session, as the input reports have it.
    fn right_press(ac: &mut AntiCheat, ms: u32) {
        #[cfg(since_254)]
        let input = Input::Click {
            x: 100,
            y: 100,
            right: true,
            gap_ms: 600,
        };
        #[cfg(before_254)]
        let input = Input::Press {
            x: 100,
            y: 100,
            right: true,
        };
        ac.input(&Timed { ms, input });
    }

    #[test]
    fn menu_options_without_a_right_press_are_flagged() {
        let mut ac = AntiCheat::default();
        // The press opens the menu, then the option is picked; before 254
        // the input report with the press only comes after the action.
        for i in 0..10 {
            let ms = i * 5_000;
            right_press(&mut ac, ms);
            ac.menu_op(ms + 800);
        }
        for i in 10..20 {
            let ms = i * 5_000;
            ac.menu_op(ms + 800);
            right_press(&mut ac, ms);
        }
        right_press(&mut ac, 199_000);
        ac.menu_op(200_000);
        assert_eq!((ac.menu_ops, ac.menu_unpressed), (21, 0));
        assert!(ac.report(0).flags.is_empty(), "{:?}", ac.report(0).flags);
        // A bot picking options straight off, with the odd right press.
        for i in 0..30 {
            ac.menu_op(300_000 + i * 2_000);
        }
        right_press(&mut ac, 320_000);
        ac.menu_op(400_000);
        assert_eq!(ac.menu_unpressed, 29);
        assert_eq!(ac.report(0).flags, ["menu_without_right_click"]);
        // The last options count too, once it's too late for their press.
        let mut late = AntiCheat::default();
        for i in 0..6 {
            late.menu_op(i * 1_000);
        }
        assert!(late.report(10_000).flags.is_empty());
        assert_eq!(late.report(40_000).flags, ["menu_without_right_click"]);
    }

    /// A click `ms` into the session and the action it makes. Before 254
    /// the click's press is judged, from 254 the action.
    fn act(ac: &mut AntiCheat, ms: u32) {
        #[cfg(before_254)]
        ac.input(&Timed {
            ms,
            input: Input::Press {
                x: 100,
                y: 100,
                right: false,
            },
        });
        #[cfg(since_254)]
        let _ = ms;
        ac.op(Op::Other, 0, BASE);
    }

    #[test]
    fn actions_while_unfocused_are_flagged() {
        let mut ac = AntiCheat::default();
        let focus = |ac: &mut AntiCheat, ms, focus| {
            ac.input(&Timed {
                ms,
                input: Input::Focus(focus),
            });
        };
        // Clicking back into the window, three times: the press comes just
        // before focus does, the action (from 254) after its report.
        for ms in [0, 10_000, 20_000] {
            focus(&mut ac, ms, false);
            if cfg!(since_254) {
                focus(&mut ac, ms + 5_000, true);
                act(&mut ac, ms + 5_000);
            } else {
                act(&mut ac, ms + 5_000);
                focus(&mut ac, ms + 5_000, true);
            }
        }
        assert_eq!(ac.unfocused_actions, 0);
        // From 254 the desktop client can send the focusing click's action
        // just ahead of the focus.
        #[cfg(since_254)]
        {
            focus(&mut ac, 30_000, false);
            act(&mut ac, 31_000);
            ac.focus_clock = 1;
            focus(&mut ac, 31_020, true);
            assert_eq!(ac.unfocused_actions, 0);
        }
        // Clicks that never focused it.
        focus(&mut ac, 40_000, false);
        for ms in [41_000, 42_000, 43_000] {
            act(&mut ac, ms);
        }
        #[cfg(since_254)]
        {
            ac.focus_clock = 20;
        }
        focus(&mut ac, 50_000, true);
        assert_eq!(ac.unfocused_actions, 3);
        assert_eq!(ac.report(0).flags, ["actions_while_unfocused"]);
    }

    /// The flags for 40 clicks 600 ms apart, each 300 px from the last.
    fn clockwork_jumps(touch: bool) -> Vec<&'static str> {
        let mut ac = AntiCheat {
            touch,
            ..AntiCheat::default()
        };
        for i in 0..40 {
            let x = (100 + (i & 1) * 300) as i16;
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
        ac.report(0).flags
    }
}
