//! The recording format: a stream of tagged entries, grouped by tick.
//!
//! Each tick that has anything to record starts with a [`TICK`] entry giving
//! how far into the session it is. Packets are stored as they cross the wire
//! minus the ISAAC cipher, so a recording plays back in that revision's
//! client.

/// A tick begins: `varint ms` since the session started.
pub const TICK: u8 = 0;
/// Packets the world sent: `varint len`, then opcode, frame and body.
pub const SERVER: u8 = 1;
/// A packet the client sent: opcode, `varint len`, body.
pub const CLIENT: u8 = 2;
/// The login reply: `varint len`, bytes.
pub const LOGIN: u8 = 3;
/// The client reconnected and was sent this reply: `varint len`, bytes.
pub const RECONNECT: u8 = 4;
/// A description of something the player did: `varint len`, UTF-8.
pub const NOTE: u8 = 5;
/// A keyframe: what a client that has just logged in needs to show what the
/// player's did at this point (see [`crate::watch`]). `varint len`, then
/// packets, each a `u16` length and the packet.
pub const KEYFRAME: u8 = 6;

/// One recorded entry.
#[derive(Debug, PartialEq, Eq)]
pub enum Entry<'a> {
    Tick { ms: u32 },
    Server(&'a [u8]),
    Client { opcode: u8, data: &'a [u8] },
    Login(&'a [u8]),
    Reconnect(&'a [u8]),
    Note(&'a str),
    Keyframe(&'a [u8]),
}

pub fn put_varint(out: &mut Vec<u8>, mut value: u32) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

pub fn put_bytes(out: &mut Vec<u8>, tag: u8, bytes: &[u8]) {
    out.push(tag);
    put_varint(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

/// Reads entries back from a recording.
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn varint(&mut self) -> Option<u32> {
        let mut value = 0u32;
        for shift in (0..35).step_by(7) {
            let byte = *self.buf.get(self.pos)?;
            self.pos += 1;
            value |= u32::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }

    fn bytes(&mut self) -> Option<&'a [u8]> {
        let len = self.varint()? as usize;
        let bytes = self.buf.get(self.pos..self.pos.checked_add(len)?)?;
        self.pos += len;
        Some(bytes)
    }
}

impl<'a> Iterator for Reader<'a> {
    type Item = Entry<'a>;

    /// The next entry, `None` at the end or where the stream is cut short.
    fn next(&mut self) -> Option<Entry<'a>> {
        let tag = *self.buf.get(self.pos)?;
        self.pos += 1;
        Some(match tag {
            TICK => Entry::Tick { ms: self.varint()? },
            SERVER => Entry::Server(self.bytes()?),
            CLIENT => {
                let opcode = *self.buf.get(self.pos)?;
                self.pos += 1;
                Entry::Client {
                    opcode,
                    data: self.bytes()?,
                }
            }
            LOGIN => Entry::Login(self.bytes()?),
            RECONNECT => Entry::Reconnect(self.bytes()?),
            NOTE => Entry::Note(std::str::from_utf8(self.bytes()?).ok()?),
            KEYFRAME => Entry::Keyframe(self.bytes()?),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let mut out = Vec::new();
        out.push(TICK);
        put_varint(&mut out, 300_000);
        put_bytes(&mut out, LOGIN, &[2, 0, 1]);
        put_bytes(&mut out, SERVER, &[7; 200]);
        out.push(CLIENT);
        out.push(42);
        put_varint(&mut out, 2);
        out.extend_from_slice(&[1, 2]);
        put_bytes(&mut out, NOTE, "Walk here".as_bytes());
        let entries: Vec<Entry> = Reader::new(&out).collect();
        assert_eq!(
            entries,
            [
                Entry::Tick { ms: 300_000 },
                Entry::Login(&[2, 0, 1]),
                Entry::Server(&[7; 200]),
                Entry::Client {
                    opcode: 42,
                    data: &[1, 2]
                },
                Entry::Note("Walk here"),
            ]
        );
    }

    #[test]
    fn stops_at_a_cut() {
        let mut out = Vec::new();
        put_bytes(&mut out, SERVER, &[1, 2, 3]);
        out.truncate(3);
        assert_eq!(Reader::new(&out).count(), 0);
    }
}
