use std::str;

/// A tick begins: `varint ms` since the session started. Each chunk's first
/// tick, so a chunk reads on its own.
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
/// A later tick in the same chunk: `varint ms` since the tick before.
pub const TICK_DELTA: u8 = 7;
/// Something an anti-cheat check caught, or a check first flagging the
/// session: `varint ms` when (an incident can be judged a while after it),
/// then `varint len` and UTF-8: the flag, a tab, and what happened (none
/// for a flag being raised).
pub const FLAG: u8 = 8;
/// A script that ran on the player was aborted: `varint len`, UTF-8: the
/// script, a tab, and the error.
pub const ERROR: u8 = 9;

/// One recorded entry.
#[derive(Debug, PartialEq, Eq)]
pub enum Entry<'a> {
    Tick {
        ms: u32,
    },
    Server(&'a [u8]),
    Client {
        opcode: u8,
        data: &'a [u8],
    },
    Login(&'a [u8]),
    Reconnect(&'a [u8]),
    Note(&'a str),
    Keyframe(&'a [u8]),
    Flag {
        ms: u32,
        flag: &'a str,
        detail: &'a str,
    },
    Error {
        script: &'a str,
        message: &'a str,
    },
}

/// Writes a [`FLAG`] entry.
pub fn put_flag(out: &mut Vec<u8>, ms: u32, flag: &str, detail: &str) {
    out.push(FLAG);
    put_varint(out, ms);
    put_varint(out, (flag.len() + 1 + detail.len()) as u32);
    out.extend_from_slice(flag.as_bytes());
    out.push(b'\t');
    out.extend_from_slice(detail.as_bytes());
}

/// Writes an [`ERROR`] entry.
pub fn put_error(out: &mut Vec<u8>, script: &str, message: &str) {
    out.push(ERROR);
    put_varint(out, (script.len() + 1 + message.len()) as u32);
    out.extend_from_slice(script.as_bytes());
    out.push(b'\t');
    out.extend_from_slice(message.as_bytes());
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
    ms: u32,
    read: usize,
    broken: bool,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self::resume(buf, 0)
    }

    /// Reads on from where another reader of the same chunk stopped, with
    /// its [`Self::ms`].
    pub fn resume(buf: &'a [u8], ms: u32) -> Self {
        Self {
            buf,
            pos: 0,
            ms,
            read: 0,
            broken: false,
        }
    }

    /// When the last tick read began.
    pub fn ms(&self) -> u32 {
        self.ms
    }

    /// How much of the buffer the entries read so far take up.
    pub fn read(&self) -> usize {
        self.read
    }

    pub fn broken(&self) -> bool {
        self.broken
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
        self.broken = true;
        None
    }

    fn bytes(&mut self) -> Option<&'a [u8]> {
        let len = self.varint()? as usize;
        let bytes = self.buf.get(self.pos..self.pos.checked_add(len)?)?;
        self.pos += len;
        Some(bytes)
    }

    fn text(&mut self) -> Option<&'a str> {
        let text = str::from_utf8(self.bytes()?).ok();
        self.broken |= text.is_none();
        text
    }

    fn entry(&mut self) -> Option<Entry<'a>> {
        let tag = *self.buf.get(self.pos)?;
        self.pos += 1;
        Some(match tag {
            TICK => {
                self.ms = self.varint()?;
                Entry::Tick { ms: self.ms }
            }
            TICK_DELTA => {
                self.ms = self.ms.wrapping_add(self.varint()?);
                Entry::Tick { ms: self.ms }
            }
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
            NOTE => Entry::Note(self.text()?),
            KEYFRAME => Entry::Keyframe(self.bytes()?),
            FLAG => {
                let ms = self.varint()?;
                let text = self.text()?;
                let (flag, detail) = text.split_once('\t').unwrap_or((text, ""));
                Entry::Flag { ms, flag, detail }
            }
            ERROR => {
                let text = self.text()?;
                let (script, message) = text.split_once('\t').unwrap_or((text, ""));
                Entry::Error { script, message }
            }
            _ => {
                self.broken = true;
                return None;
            }
        })
    }
}

impl<'a> Iterator for Reader<'a> {
    type Item = Entry<'a>;

    /// The next entry, `None` at the end or where the stream is cut short.
    fn next(&mut self) -> Option<Entry<'a>> {
        let entry = self.entry();
        if entry.is_some() {
            self.read = self.pos;
        }
        entry
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
        put_flag(&mut out, 299_000, "clicked_unseen", "an NPC");
        put_error(&mut out, "[opnpc1,man]", "Stack underflow");
        out.push(TICK_DELTA);
        put_varint(&mut out, 600);
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
                Entry::Flag {
                    ms: 299_000,
                    flag: "clicked_unseen",
                    detail: "an NPC"
                },
                Entry::Error {
                    script: "[opnpc1,man]",
                    message: "Stack underflow"
                },
                Entry::Tick { ms: 300_600 },
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
