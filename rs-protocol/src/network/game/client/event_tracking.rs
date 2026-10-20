use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

/// An input event the client tracked. Spots are `(x, y)` on the game frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackedEvent {
    Press { x: i16, y: i16, right: bool },
    Release { right: bool },
    MoveBy { dx: i16, dy: i16 },
    MoveTo { x: i16, y: i16 },
    KeyDown(u8),
    KeyUp(u8),
    Focus(bool),
    Enter,
    Leave,
}

/// A tracked event, `gap` 10ms steps after the one before (capped at 250).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tracked {
    pub gap: u8,
    pub event: TrackedEvent,
}

/// The client's input since its last report, while the server has
/// tracking on.
#[client_prot(VarShort { min: 0 }, ClientEvent)]
pub struct EventTracking {
    pub events: Vec<Tracked>,
}

/// A packed `x | y << 10`.
fn spot(pos: i32) -> (i16, i16) {
    ((pos & 0x3ff) as i16, (pos >> 10) as i16)
}

impl ClientProtMessage for EventTracking {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        // A kind byte, the gap, then 0 to 3 bytes the kind says: a body cut
        // short (or an unknown kind) keeps the events before.
        let mut events = Vec::with_capacity(len / 3);
        while len - buf.pos >= 2 {
            let kind = buf.data[buf.pos];
            let extra = match kind {
                1 | 2 | 7 => 3,
                5 | 8 | 9 => 1,
                6 => 2,
                3 | 4 | 10..=13 => 0,
                _ => break,
            };
            if len - buf.pos < 2 + extra {
                break;
            }
            buf.pos += 1;
            let gap = buf.g1();
            let event = match kind {
                1 | 2 => {
                    let (x, y) = spot(buf.g3());
                    TrackedEvent::Press {
                        x,
                        y,
                        right: kind == 1,
                    }
                }
                3 | 4 => TrackedEvent::Release { right: kind == 3 },
                5 => {
                    let v = buf.g1();
                    TrackedEvent::MoveBy {
                        dx: i16::from(v & 0xf) - 8,
                        dy: i16::from(v >> 4) - 8,
                    }
                }
                6 => TrackedEvent::MoveBy {
                    dx: i16::from(buf.g1()) - 128,
                    dy: i16::from(buf.g1()) - 128,
                },
                7 => {
                    let (x, y) = spot(buf.g3());
                    TrackedEvent::MoveTo { x, y }
                }
                8 => TrackedEvent::KeyDown(buf.g1()),
                9 => TrackedEvent::KeyUp(buf.g1()),
                10 | 11 => TrackedEvent::Focus(kind == 10),
                12 => TrackedEvent::Enter,
                _ => TrackedEvent::Leave,
            };
            events.push(Tracked { gap, event });
        }
        EventTracking { events }
    }
}
