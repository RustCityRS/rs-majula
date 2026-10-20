//! Times the client's input reports (mouse, keys, focus, camera), as decoded
//! by their protocol messages, for the anti-cheat checks and the replay's
//! cursor.
//!
//! The reports arrive in batches, so each event's time is worked out from
//! when its packet arrived and the gaps the client measured between events.
//! Times are milliseconds into the session.

use rs_io::Packet;
use rs_protocol::network::game::client::ClientProtMessage;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_applet_focus::EventAppletFocus;
#[cfg(any(rev = "225", since_254))]
use rs_protocol::network::game::client::event_camera_position::EventCameraPosition;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_mouse_click::EventMouseClick;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_mouse_move::{Cursor, EventMouseMove, MouseMoved};
#[cfg(before_274)]
use rs_protocol::network::game::client::event_tracking::{EventTracking, TrackedEvent};
use rs_protocol::network::game::client_prot::ClientProt;

/// `EVENT_MOUSE_MOVE` samples the cursor this often.
#[cfg(since_254)]
const SAMPLE_MS: u32 = 50;
/// A move report holds at most this many samples (the client sends at 40).
#[cfg(since_254)]
const MAX_SAMPLES_MS: u32 = 40 * SAMPLE_MS;
/// `EVENT_TRACKING` measures gaps in 10ms steps.
#[cfg(before_274)]
const TRACK_STEP_MS: u32 = 10;
/// A report this soon after the server restarted tracking is the client
/// finishing the old run (a new run takes seconds to fill a report).
#[cfg(before_274)]
const FINISH_WINDOW_MS: u32 = 1500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Move {
        x: i16,
        y: i16,
    },
    Outside,
    Press {
        x: i16,
        y: i16,
        right: bool,
    },
    Release {
        right: bool,
    },
    Click {
        x: i16,
        y: i16,
        right: bool,
        gap_ms: u32,
    },
    KeyDown(u8),
    KeyUp(u8),
    Focus(bool),
    Enter,
    Leave,
    Camera {
        pitch: u16,
        yaw: u16,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timed {
    pub ms: u32,
    pub input: Input,
}

/// Turns a session's input reports into [`Timed`] events. One per session:
/// reports build on earlier ones.
#[derive(Default)]
pub struct InputDecoder {
    #[cfg(since_254)]
    move_ms: Option<u32>,
    #[cfg(since_254)]
    move_x: i32,
    #[cfg(since_254)]
    move_y: i32,
    #[cfg(before_274)]
    track_ms: u32,
    #[cfg(before_274)]
    restarted: Option<u32>,
    #[cfg(before_274)]
    track_x: i32,
    #[cfg(before_274)]
    track_y: i32,
}

/// A recorded message's body, decoded. `None` when it's too short to be one
/// (it's recorded before that's checked).
pub(super) fn decode<M: ClientProtMessage>(data: &[u8]) -> Option<M> {
    (data.len() >= M::FRAME.min()).then(|| M::decode(&mut Packet::from(data.to_vec()), data.len()))
}

#[cfg(since_254)]
fn cursor(at: Cursor) -> Input {
    match at {
        Cursor::At { x, y } => Input::Move { x, y },
        Cursor::Outside => Input::Outside,
    }
}

impl InputDecoder {
    /// A recorded `prot` body that arrived at `at`, if it's an input report.
    /// Returns whether it was. (Live ones come in by their handlers.)
    pub fn packet(
        &mut self,
        prot: &ClientProt,
        at: u32,
        data: &[u8],
        out: &mut Vec<Timed>,
    ) -> bool {
        match prot {
            #[cfg(since_254)]
            ClientProt::EventMouseMove => {
                if let Some(msg) = decode(data) {
                    self.mouse_move(at, &msg, out);
                }
            }
            #[cfg(since_254)]
            ClientProt::EventMouseClick => {
                if let Some(msg) = decode(data) {
                    self.mouse_click(at, &msg, out);
                }
            }
            #[cfg(since_254)]
            ClientProt::EventAppletFocus => {
                if let Some(msg) = decode(data) {
                    self.focus(at, &msg, out);
                }
            }
            #[cfg(any(rev = "225", since_254))]
            ClientProt::EventCameraPosition => {
                if let Some(msg) = decode(data) {
                    self.camera(at, &msg, out);
                }
            }
            #[cfg(before_274)]
            ClientProt::EventTracking => {
                if let Some(msg) = decode(data) {
                    self.tracking(at, &msg, out);
                }
            }
            _ => return false,
        }
        true
    }

    /// An `EVENT_MOUSE_MOVE` that arrived at `at`: cursor samples 50ms
    /// apart, each change after how many samples it stood still.
    #[cfg(since_254)]
    pub fn mouse_move(&mut self, at: u32, msg: &EventMouseMove, out: &mut Vec<Timed>) {
        // A report covers at most two seconds, so a chain that fell behind
        // (the tab was in the background) picks up from there.
        let floor = at.saturating_sub(MAX_SAMPLES_MS);
        let mut ms = self.move_ms.unwrap_or(floor).max(floor);
        for sample in &msg.samples {
            let input = match sample.moved {
                MouseMoved::By { dx, dy } => Input::Move {
                    x: (self.move_x + i32::from(dx)) as i16,
                    y: (self.move_y + i32::from(dy)) as i16,
                },
                MouseMoved::To(at) => cursor(at),
            };
            match input {
                Input::Move { x, y } => {
                    self.move_x = x.into();
                    self.move_y = y.into();
                }
                _ => {
                    self.move_x = -1;
                    self.move_y = -1;
                }
            }
            ms = (ms + (u32::from(sample.still) + 1) * SAMPLE_MS).min(at);
            out.push(Timed { ms, input });
        }
        self.move_ms = Some(ms);
    }

    /// An `EVENT_MOUSE_CLICK` that arrived at `at`. The client sends its
    /// last moves just before it, so it happened when they end.
    #[cfg(since_254)]
    pub fn mouse_click(&mut self, at: u32, msg: &EventMouseClick, out: &mut Vec<Timed>) {
        let ms = self.move_ms.filter(|&m| at - m < 1000).unwrap_or(at);
        let Cursor::At { x, y } = msg.at else {
            return;
        };
        out.push(Timed {
            ms,
            input: Input::Click {
                x,
                y,
                right: msg.right,
                gap_ms: u32::from(msg.gap) * SAMPLE_MS,
            },
        });
    }

    /// The server sent `FINISH_TRACKING` then `ENABLE_TRACKING` at `at`:
    /// the client sends what it has, then starts a new run whose first gap
    /// counts from here.
    #[cfg(before_274)]
    pub fn tracking_restarted(&mut self, at: u32) {
        if let Some(previous) = self.restarted.replace(at) {
            self.track_ms = previous;
        }
    }

    /// An `EVENT_TRACKING` report that arrived at `at`: events with the gap
    /// since the one before, in 10ms steps (capped at 2.5 seconds, so a
    /// longer pause comes out short).
    #[cfg(before_274)]
    pub fn tracking(&mut self, at: u32, msg: &EventTracking, out: &mut Vec<Timed>) {
        // The first report after a restart either finishes the old run
        // (straight away) or begins the new one.
        let restart = self.restarted.take();
        if let Some(restart) = restart.filter(|&r| at.saturating_sub(r) > FINISH_WINDOW_MS) {
            self.track_ms = restart;
        }
        let start = out.len();
        let mut ms = self.track_ms;
        for tracked in &msg.events {
            let input = match tracked.event {
                TrackedEvent::Press { x, y, right } => Input::Press { x, y, right },
                TrackedEvent::Release { right } => Input::Release { right },
                TrackedEvent::MoveBy { dx, dy } => {
                    self.track_x += i32::from(dx);
                    self.track_y += i32::from(dy);
                    self.track_move()
                }
                TrackedEvent::MoveTo { x, y } => {
                    self.track_x = x.into();
                    self.track_y = y.into();
                    self.track_move()
                }
                TrackedEvent::KeyDown(key) => Input::KeyDown(key),
                TrackedEvent::KeyUp(key) => Input::KeyUp(key),
                TrackedEvent::Focus(focus) => Input::Focus(focus),
                TrackedEvent::Enter => Input::Enter,
                TrackedEvent::Leave => Input::Leave,
            };
            ms = (ms + u32::from(tracked.gap) * TRACK_STEP_MS).min(at);
            out.push(Timed { ms, input });
        }
        self.track_ms = ms;
        if let Some(restart) = restart.filter(|&r| at.saturating_sub(r) <= FINISH_WINDOW_MS) {
            self.track_ms = restart;
        }
        // From 254 the mouse and focus have reports of their own, sampled
        // and timed better: only the keys are new here.
        if cfg!(since_254) {
            let mut index = 0;
            out.retain(|t| {
                let keep = index < start || matches!(t.input, Input::KeyDown(_) | Input::KeyUp(_));
                index += 1;
                keep
            });
        }
    }

    #[cfg(before_274)]
    fn track_move(&self) -> Input {
        Input::Move {
            x: self.track_x as i16,
            y: self.track_y as i16,
        }
    }

    /// `EVENT_APPLET_FOCUS`.
    #[cfg(since_254)]
    pub fn focus(&mut self, at: u32, msg: &EventAppletFocus, out: &mut Vec<Timed>) {
        out.push(Timed {
            ms: at,
            input: Input::Focus(msg.focused),
        });
    }

    /// `EVENT_CAMERA_POSITION`: pitch and yaw (225's minimap angle and zoom
    /// aren't kept).
    #[cfg(any(rev = "225", since_254))]
    pub fn camera(&mut self, at: u32, msg: &EventCameraPosition, out: &mut Vec<Timed>) {
        out.push(Timed {
            ms: at,
            input: Input::Camera {
                pitch: msg.pitch,
                yaw: msg.yaw,
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encodes samples as the client's `EVENT_MOUSE_MOVE` does.
    #[cfg(since_254)]
    fn encode_moves(samples: &[(i32, i32)], tracked: &mut (i32, i32)) -> Vec<u8> {
        let mut out = Vec::new();
        let mut delta = 0u32;
        for &(x, y) in samples {
            if (x, y) == *tracked {
                delta += 1;
                continue;
            }
            let (dx, dy) = (x - tracked.0, y - tracked.1);
            *tracked = (x, y);
            let pos = (y * 765 + x) as u32;
            if delta < 8 && (-32..=31).contains(&dx) && (-32..=31).contains(&dy) {
                let v = (delta << 12) + (((dx + 32) as u32) << 6) + (dy + 32) as u32;
                out.extend_from_slice(&(v as u16).to_be_bytes());
            } else if delta < 8 {
                let v = 0x800000 + (delta << 19) + pos;
                out.extend_from_slice(&v.to_be_bytes()[1..]);
            } else {
                let v = 0xc0000000 + (delta << 19) + pos;
                out.extend_from_slice(&v.to_be_bytes());
            }
            delta = 0;
        }
        out
    }

    #[cfg(since_254)]
    #[test]
    fn decodes_mouse_moves() {
        let samples = [(10, 10), (12, 11), (100, 200), (100, 200), (101, 199)];
        let data = encode_moves(&samples, &mut (0, 0));
        let mut decoder = InputDecoder::default();
        let mut out = Vec::new();
        decoder.mouse_move(10_000, &decode(&data).unwrap(), &mut out);
        let moves: Vec<(u32, Input)> = out.iter().map(|t| (t.ms, t.input)).collect();
        assert_eq!(
            moves,
            [
                (8_050, Input::Move { x: 10, y: 10 }),
                (8_100, Input::Move { x: 12, y: 11 }),
                (8_150, Input::Move { x: 100, y: 200 }),
                (8_250, Input::Move { x: 101, y: 199 }),
            ]
        );
    }

    #[cfg(since_254)]
    #[test]
    fn long_stills_and_outside() {
        let mut samples = vec![(5, 5)];
        samples.extend(std::iter::repeat_n((5, 5), 20));
        samples.push((6, 5));
        let data = encode_moves(&samples, &mut (0, 0));
        let mut decoder = InputDecoder::default();
        let mut out = Vec::new();
        decoder.mouse_move(5_000, &decode(&data).unwrap(), &mut out);
        assert_eq!(out[1].input, Input::Move { x: 6, y: 5 });
        assert_eq!(out[1].ms - out[0].ms, 21 * SAMPLE_MS);

        // pos 0x7ffff: the cursor left the frame.
        let v: u32 = 0x800000 + 0x7ffff;
        decoder.mouse_move(6_000, &decode(&v.to_be_bytes()[1..]).unwrap(), &mut out);
        assert_eq!(out[2].input, Input::Outside);
    }

    #[cfg(since_254)]
    #[test]
    fn decodes_clicks() {
        let mut decoder = InputDecoder::default();
        let mut out = Vec::new();
        let v: u32 = (12 << 20) + (1 << 19) + 300 * 765 + 400;
        decoder.mouse_click(3_000, &decode(&v.to_be_bytes()).unwrap(), &mut out);
        assert_eq!(
            out[0].input,
            Input::Click {
                x: 400,
                y: 300,
                right: true,
                gap_ms: 600
            }
        );
    }

    /// A recorded packet goes through its message too, and a short one is
    /// passed over.
    #[test]
    fn decodes_recorded_packets() {
        let mut decoder = InputDecoder::default();
        let mut out = Vec::new();
        assert!(!decoder.packet(&ClientProt::NoTimeout, 1_000, &[], &mut out));
        #[cfg(any(rev = "225", since_254))]
        {
            let data = [0x01, 0x80, 0x02, 0x00, 0, 0];
            let len = <EventCameraPosition as rs_protocol::network::game::client_prot_message::ClientProtMessageInfo>::FRAME.min();
            assert!(decoder.packet(
                &ClientProt::EventCameraPosition,
                1_000,
                &data[..len],
                &mut out
            ));
            assert_eq!(
                out,
                [Timed {
                    ms: 1_000,
                    input: Input::Camera {
                        pitch: 384,
                        yaw: 512
                    }
                }]
            );
            assert!(decoder.packet(
                &ClientProt::EventCameraPosition,
                1_000,
                &data[..len - 1],
                &mut out
            ));
            assert_eq!(out.len(), 1);
        }
    }

    #[cfg(before_274)]
    #[test]
    fn decodes_tracking() {
        let mut decoder = InputDecoder::default();
        decoder.tracking_restarted(1_000);
        let pos = 100 + (50 << 10);
        let data = [
            7,
            10,
            0,
            (pos >> 8) as u8,
            pos as u8, // move to (100, 50)
            5,
            3,
            (2 + 8) | ((8 - 1) << 4), // move by (+2, -1)
            6,
            1,
            128 + 40,
            128 - 20, // move by (+40, -20)
            2,
            5,
            0,
            (pos >> 8) as u8,
            pos as u8, // left press
            4,
            2, // left release
            8,
            0,
            65, // key down
            11,
            250, // focus lost 2.5s later
        ];
        let mut out = Vec::new();
        decoder.tracking(9_000, &decode(&data).unwrap(), &mut out);
        let got: Vec<(u32, Input)> = out.iter().map(|t| (t.ms, t.input)).collect();
        if cfg!(since_254) {
            assert_eq!(got, [(1_210, Input::KeyDown(65))]);
            return;
        }
        assert_eq!(
            got,
            [
                (1_100, Input::Move { x: 100, y: 50 }),
                (1_130, Input::Move { x: 102, y: 49 }),
                (1_140, Input::Move { x: 142, y: 29 }),
                (
                    1_190,
                    Input::Press {
                        x: 100,
                        y: 50,
                        right: false
                    }
                ),
                (1_210, Input::Release { right: false }),
                (1_210, Input::KeyDown(65)),
                (3_710, Input::Focus(false)),
            ]
        );
    }

    #[cfg(before_274)]
    #[test]
    fn restarts_finish_the_old_run_then_start_a_new_one() {
        let mut decoder = InputDecoder::default();
        let mut out = Vec::new();
        decoder.tracking_restarted(0);
        // Keys (every revision reports them here), 1s apart: the run so far.
        decoder.tracking(3_000, &decode(&[8, 100, 65, 9, 100, 65]).unwrap(), &mut out);
        assert_eq!((out[0].ms, out[1].ms), (1_000, 2_000));
        // Restarted at 10s: the report straight after finishes the old run...
        decoder.tracking_restarted(10_000);
        decoder.tracking(10_300, &decode(&[8, 100, 66]).unwrap(), &mut out);
        assert_eq!(out[2].ms, 3_000);
        // ...and the next counts from the restart.
        decoder.tracking(15_000, &decode(&[9, 50, 66]).unwrap(), &mut out);
        assert_eq!(out[3].ms, 10_500);
        // A restart with nothing to finish: the next report is the new run.
        decoder.tracking_restarted(20_000);
        decoder.tracking(25_000, &decode(&[8, 10, 67]).unwrap(), &mut out);
        assert_eq!(out[4].ms, 20_100);
    }
}
