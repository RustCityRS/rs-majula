#[cfg(since_254)]
use crate::network::game::client::ClientProtMessage;
#[cfg(since_254)]
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
#[cfg(since_254)]
use crate::network::game::client_prot_message::ClientProtMessageInfo;
#[cfg(since_254)]
use rs_io::Packet;
#[cfg(since_254)]
use rs_protocol_macros::client_prot;

/// The game frame's width: the client packs a spot as `y * 765 + x`.
#[cfg(since_254)]
const FRAME_WIDTH: u32 = 765;
/// A packed spot meaning the cursor is off the game frame.
#[cfg(since_254)]
const OUTSIDE: u32 = 0x7ffff;

/// Where the cursor is: a spot on the game frame, or off it.
#[cfg(since_254)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    At { x: i16, y: i16 },
    Outside,
}

#[cfg(since_254)]
impl Cursor {
    /// From the client's packed `y * 765 + x`.
    pub const fn unpack(pos: u32) -> Self {
        if pos == OUTSIDE {
            return Cursor::Outside;
        }
        Cursor::At {
            x: (pos % FRAME_WIDTH) as i16,
            y: (pos / FRAME_WIDTH) as i16,
        }
    }
}

/// Where the cursor went at a sample.
#[cfg(since_254)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseMoved {
    By { dx: i8, dy: i8 },
    To(Cursor),
}

/// One cursor sample, after `still` samples where it stood still. The
/// client samples every 50ms and only sends the changes.
#[cfg(since_254)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseSample {
    pub still: u16,
    pub moved: MouseMoved,
}

#[cfg(since_254)]
#[client_prot(VarByte { min: 0 }, ClientEvent)]
pub struct EventMouseMove {
    pub samples: Vec<MouseSample>,
}

#[cfg(since_254)]
impl ClientProtMessage for EventMouseMove {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        // Each sample is 2, 3 or 4 bytes, told by its first: a body cut
        // short keeps the samples before.
        let mut samples = Vec::with_capacity(len >> 1);
        while buf.pos < len {
            let first = buf.data[buf.pos];
            let left = len - buf.pos;
            let sample = if first & 0x80 == 0 {
                if left < 2 {
                    break;
                }
                let v = buf.g2();
                MouseSample {
                    still: v >> 12,
                    moved: MouseMoved::By {
                        dx: ((v >> 6) & 0x3f) as i8 - 32,
                        dy: (v & 0x3f) as i8 - 32,
                    },
                }
            } else if first & 0x40 == 0 {
                if left < 3 {
                    break;
                }
                let v = buf.g3() as u32;
                MouseSample {
                    still: ((v >> 19) & 0x7) as u16,
                    moved: MouseMoved::To(Cursor::unpack(v & OUTSIDE)),
                }
            } else {
                if left < 4 {
                    break;
                }
                let v = buf.g4s() as u32;
                MouseSample {
                    still: ((v >> 19) & 0x7ff) as u16,
                    moved: MouseMoved::To(Cursor::unpack(v & OUTSIDE)),
                }
            };
            samples.push(sample);
        }
        EventMouseMove { samples }
    }
}
