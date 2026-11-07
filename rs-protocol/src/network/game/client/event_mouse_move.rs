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

#[cfg(before_647)]
const FRAME_WIDTH: u32 = 765;
#[cfg(before_647)]
const OUTSIDE: u32 = 0x7ffff;
#[cfg(since_254)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    #[cfg(before_647)]
    At {
        x: i16,
        y: i16,
    },
    #[cfg(since_647)]
    At {
        x: u16,
        y: u16,
    },
    Outside,
}

#[cfg(before_647)]
impl Cursor {
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

#[cfg(since_647)]
impl Cursor {
    pub const fn unpack(pos: u32) -> Self {
        if pos == 0x80000000 {
            return Cursor::Outside;
        }
        Cursor::At {
            x: pos as u16,
            y: (pos >> 16) as u16,
        }
    }
}
#[cfg(since_254)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseMoved {
    By { dx: i8, dy: i8 },
    To(Cursor),
}
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

#[cfg(before_647)]
impl ClientProtMessage for EventMouseMove {
    fn decode(buf: &mut Packet, len: usize) -> Self {
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

#[cfg(since_647)]
impl ClientProtMessage for EventMouseMove {
    fn validate(data: &[u8]) -> bool {
        let mut pos = 0;
        while let Some(&first) = data.get(pos) {
            pos += match first {
                0..=127 => 2,
                128..=191 => 3,
                192..=223 => 5,
                _ => 6,
            };
        }
        pos == data.len()
    }

    fn decode(buf: &mut Packet, len: usize) -> Self {
        let mut samples = Vec::with_capacity(len >> 1);
        while buf.pos < len {
            let first = buf.data[buf.pos];
            let sample = match first {
                0..=127 => {
                    let val = buf.g2();
                    MouseSample {
                        still: val >> 12,
                        moved: MouseMoved::By {
                            dx: ((val >> 6) & 0x3f) as i8 - 32,
                            dy: (val & 0x3f) as i8 - 32,
                        },
                    }
                }
                128..=191 => MouseSample {
                    still: (buf.g1() & 0x1f) as u16,
                    moved: MouseMoved::By {
                        dx: buf.g1().wrapping_sub(128) as i8,
                        dy: buf.g1().wrapping_sub(128) as i8,
                    },
                },
                _ => {
                    let still = if first < 224 {
                        (buf.g1() & 0x1f) as u16
                    } else {
                        buf.g2() & 0x1fff
                    };
                    MouseSample {
                        still,
                        moved: MouseMoved::To(Cursor::unpack(buf.g4s() as u32)),
                    }
                }
            };
            samples.push(sample);
        }
        EventMouseMove { samples }
    }
}
