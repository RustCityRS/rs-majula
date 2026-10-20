//! Turns a stored recording into what the replay page plays: the packets the
//! client was sent, in timed frames, and the player's input and actions as
//! JSON.
//!
//! The file is `RSRP`, a version byte, a big-endian `u32` header length, the
//! JSON header, then frames: `u32 ms`, a [`Frame`] kind byte, `u32 len`, and
//! the bytes. A server frame's packets are each prefixed with a `u16`
//! length, in plaintext: ciphering the opcodes is the page's job.

use super::format::{Entry, Reader};
use super::input::{Input, InputDecoder, Timed, decode};
use flate2::read::DeflateDecoder;
use rs_db::ReplayChunk;
use rs_protocol::network::game::client::client_cheat::ClientCheat;
use rs_protocol::network::game::client::message_private::MessagePrivate;
use rs_protocol::network::game::client::message_public::MessagePublic;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_util::base37::{to_raw_username, to_screen_name};
use rs_util::wordpack;
use serde_json::{Map, Value, json};
use std::io::Read;

/// How long the replay takes to type each key of a line the player sent.
const KEY_MS: u32 = 70;

/// The prefixes the client reads a chat line's colour and effect from, by
/// number.
const CHAT_COLOURS: [&str; 12] = [
    "", "red:", "green:", "cyan:", "purple:", "white:", "flash1:", "flash2:", "flash3:", "glow1:",
    "glow2:", "glow3:",
];
#[cfg(before_289)]
const CHAT_EFFECTS: [&str; 3] = ["", "wave:", "scroll:"];
#[cfg(since_289)]
const CHAT_EFFECTS: [&str; 6] = ["", "wave:", "wave2:", "shake:", "scroll:", "slide:"];

pub const MAGIC: &[u8; 4] = b"RSRP";
pub const VERSION: u8 = 1;

/// A frame's kind.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frame {
    Server = 0,
    Login = 1,
    Reconnect = 2,
    Keyframe = 3,
}

/// An input as the page reads it: `[ms, code, ...]`.
fn input_json(timed: &Timed) -> Value {
    let ms = timed.ms;
    match timed.input {
        Input::Move { x, y } => json!([ms, 0, x, y]),
        Input::Outside => json!([ms, 1]),
        Input::Press { x, y, right } => json!([ms, 2, x, y, u8::from(right)]),
        Input::Release { right } => json!([ms, 3, u8::from(right)]),
        Input::Click { x, y, right, .. } => json!([ms, 4, x, y, u8::from(right)]),
        Input::KeyDown(key) => json!([ms, 5, key]),
        Input::KeyUp(key) => json!([ms, 6, key]),
        Input::Focus(focus) => json!([ms, 7, u8::from(focus)]),
        Input::Enter => json!([ms, 8]),
        Input::Leave => json!([ms, 9]),
        Input::Camera { pitch, yaw } => json!([ms, 10, pitch, yaw]),
    }
}

/// A line the player typed and sent: what to type for the client to send it
/// again, and how the timeline puts it.
fn typed_line(prot: &ClientProt, data: &[u8]) -> Option<(String, String)> {
    match prot {
        ClientProt::MessagePublic => {
            let msg: MessagePublic = decode(data)?;
            let text = wordpack::unpack(&msg.bytes);
            let colour = CHAT_COLOURS
                .get(usize::from(msg.colour))
                .copied()
                .unwrap_or_default();
            let effect = CHAT_EFFECTS
                .get(usize::from(msg.effect))
                .copied()
                .unwrap_or_default();
            Some((format!("{colour}{effect}{text}"), format!("Say \"{text}\"")))
        }
        ClientProt::MessagePrivate => {
            let msg: MessagePrivate = decode(data)?;
            let text = wordpack::unpack(&msg.bytes);
            let to = to_screen_name(&to_raw_username(msg.user37 as u64));
            Some((text.clone(), format!("Message {to}: \"{text}\"")))
        }
        ClientProt::ClientCheat => {
            let text = decode::<ClientCheat>(data)?.cheat;
            Some((format!("::{text}"), format!("Command ::{text}")))
        }
        _ => None,
    }
}

/// Types each line in `lines` (`(sent at, text)`) into `inputs`: one key at
/// a time, ending with Enter as it was sent. Typing starts no earlier than
/// the last of `clicks` before it (which may have opened the box it's typed
/// into) or `free`, the end of the line before.
fn type_lines(lines: &[(u32, String)], clicks: &[u32], free: &mut u32, inputs: &mut Vec<Value>) {
    for (sent, text) in lines {
        let keys = text.chars().count() as u32;
        let click = clicks
            .iter()
            .rev()
            .find(|&&c| c < *sent)
            .map_or(0, |c| c + KEY_MS);
        let start = sent
            .saturating_sub(keys * KEY_MS)
            .max(click)
            .max(*free)
            .min(*sent);
        let step = (sent - start) / (keys + 1);
        for (i, ch) in text.chars().enumerate() {
            inputs.push(json!([start + step * i as u32, 11, ch.to_string()]));
        }
        inputs.push(json!([sent, 11, "Enter"]));
        *free = sent + 1;
    }
}

/// Inflates `chunks` (in order) back into one recording. Stops at a chunk
/// that doesn't inflate: nothing after it can play.
pub fn inflate(chunks: &[ReplayChunk]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in chunks {
        let mut raw = Vec::new();
        if DeflateDecoder::new(&chunk.data[..])
            .read_to_end(&mut raw)
            .is_err()
        {
            break;
        }
        out.extend_from_slice(&raw);
    }
    out
}

/// Builds the replay file from a recording, with `extra` (who, where,
/// the verdict) added to its header.
pub fn build(recording: &[u8], extra: Map<String, Value>) -> Vec<u8> {
    let mut builder = Builder::default();
    builder.feed(recording);
    builder.take(extra)
}

/// Builds a replay file a piece at a time, for watching a session live: the
/// first [`Self::take`] is the whole file so far, each later one a file of
/// only what was fed since, for the page to add on.
#[derive(Default)]
pub struct Builder {
    frames: Vec<u8>,
    frame_count: u32,
    inputs: Vec<Value>,
    notes: Vec<Value>,
    decoder: InputDecoder,
    decoded: Vec<Timed>,
    /// Lines sent since the last take, still to type.
    lines: Vec<(u32, String)>,
    /// When every click so far was, for typing lines after them.
    clicks: Vec<u32>,
    /// Where the last line typed ended.
    typed_to: u32,
    ms: u32,
    /// The open server frame: its time, and where its length is (to grow
    /// it).
    open: Option<(u32, usize)>,
}

impl Builder {
    fn push(&mut self, kind: Frame, bytes: &[u8]) -> usize {
        self.frames.extend_from_slice(&self.ms.to_be_bytes());
        self.frames.push(kind as u8);
        let len_at = self.frames.len();
        self.frames
            .extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        self.frames.extend_from_slice(bytes);
        self.frame_count += 1;
        len_at
    }

    /// More of the recording: whole entries, carrying on from the last feed.
    pub fn feed(&mut self, recording: &[u8]) {
        for entry in Reader::new(recording) {
            match entry {
                Entry::Tick { ms } => {
                    self.ms = ms;
                    self.open = None;
                }
                Entry::Server(bytes) => {
                    #[cfg(before_274)]
                    if bytes.first() == Some(&super::TRACKING_ENABLED) {
                        self.decoder.tracking_restarted(self.ms);
                    }
                    let len_at = match self.open {
                        Some((at, len_at)) if at == self.ms => len_at,
                        _ => {
                            let len_at = self.push(Frame::Server, &[]);
                            self.open = Some((self.ms, len_at));
                            len_at
                        }
                    };
                    let packet_len = bytes.len().min(usize::from(u16::MAX));
                    let frames = &mut self.frames;
                    frames.extend_from_slice(&(packet_len as u16).to_be_bytes());
                    frames.extend_from_slice(&bytes[..packet_len]);
                    let len = u32::from_be_bytes(
                        frames[len_at..len_at + 4].try_into().unwrap_or_default(),
                    );
                    frames[len_at..len_at + 4]
                        .copy_from_slice(&(len + 2 + packet_len as u32).to_be_bytes());
                }
                Entry::Login(bytes) => {
                    self.push(Frame::Login, bytes);
                    self.open = None;
                    self.notes.push(json!([self.ms, "Logged in"]));
                }
                Entry::Reconnect(bytes) => {
                    self.push(Frame::Reconnect, bytes);
                    self.open = None;
                    self.notes.push(json!([self.ms, "Reconnected"]));
                }
                Entry::Client { opcode, data } => {
                    let Ok(prot) = ClientProt::try_from(opcode) else {
                        continue;
                    };
                    self.decoded.clear();
                    if self.decoder.packet(&prot, self.ms, data, &mut self.decoded) {
                        for timed in &self.decoded {
                            if matches!(timed.input, Input::Press { .. } | Input::Click { .. }) {
                                self.clicks.push(timed.ms);
                            }
                            self.inputs.push(input_json(timed));
                        }
                    } else if let Some((text, note)) = typed_line(&prot, data) {
                        self.lines.push((self.ms, text));
                        self.notes.push(json!([self.ms, note]));
                    }
                }
                Entry::Note(text) => self.notes.push(json!([self.ms, text])),
                Entry::Keyframe(bytes) => {
                    self.push(Frame::Keyframe, bytes);
                    self.open = None;
                }
            }
        }
    }

    /// A replay file of everything fed since the last take (from the start
    /// on the first), with `extra` added to its header.
    pub fn take(&mut self, extra: Map<String, Value>) -> Vec<u8> {
        // Reports arrive batched, so the clicks may be out of order.
        self.clicks.sort_unstable();
        let lines = std::mem::take(&mut self.lines);
        type_lines(&lines, &self.clicks, &mut self.typed_to, &mut self.inputs);
        let mut inputs = std::mem::take(&mut self.inputs);
        // Reports arrive batched, so put the inputs in time order.
        inputs.sort_by_key(|v| v[0].as_u64().unwrap_or(0));
        let frames = std::mem::take(&mut self.frames);
        // What's fed next starts frames of its own, in the next file.
        self.open = None;

        let mut header = extra;
        header.insert(
            "logout".into(),
            json!(rs_protocol::network::game::server_prot::ServerProt::Logout as u8),
        );
        #[cfg(any(rev = "225", since_254))]
        header.insert(
            "cameraOp".into(),
            json!(ClientProt::EventCameraPosition as u8),
        );
        header.insert("durationMs".into(), json!(self.ms));
        header.insert(
            "frames".into(),
            json!(std::mem::take(&mut self.frame_count)),
        );
        header.insert("inputs".into(), Value::Array(inputs));
        header.insert(
            "notes".into(),
            Value::Array(std::mem::take(&mut self.notes)),
        );
        let header = Value::Object(header).to_string();

        let mut out = Vec::with_capacity(9 + header.len() + frames.len());
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.extend_from_slice(&(header.len() as u32).to_be_bytes());
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(&frames);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watch::format::{CLIENT, LOGIN, NOTE, SERVER, TICK, put_bytes, put_varint};

    #[test]
    fn builds_frames_and_notes() {
        let mut rec = Vec::new();
        rec.push(TICK);
        put_varint(&mut rec, 0);
        put_bytes(&mut rec, LOGIN, &[2]);
        put_bytes(&mut rec, SERVER, &[10, 1]);
        put_bytes(&mut rec, SERVER, &[11, 2, 3]);
        rec.push(TICK);
        put_varint(&mut rec, 600);
        put_bytes(&mut rec, NOTE, b"Walk to 3200, 3200");
        put_bytes(&mut rec, SERVER, &[12]);

        let file = build(&rec, Map::new());
        assert_eq!(&file[..4], MAGIC);
        let header_len = u32::from_be_bytes(file[5..9].try_into().unwrap()) as usize;
        let header: Value = serde_json::from_slice(&file[9..9 + header_len]).unwrap();
        assert_eq!(header["frames"], 3);
        assert_eq!(header["durationMs"], 600);
        assert_eq!(header["notes"][1], json!([600, "Walk to 3200, 3200"]));

        let frames = &file[9 + header_len..];
        // Login frame, then one server frame with both tick-0 packets merged.
        assert_eq!(frames[4], Frame::Login as u8);
        let second = &frames[4 + 1 + 4 + 1..];
        assert_eq!(u32::from_be_bytes(second[..4].try_into().unwrap()), 0);
        assert_eq!(second[4], Frame::Server as u8);
        assert_eq!(u32::from_be_bytes(second[5..9].try_into().unwrap()), 9);
        assert_eq!(&second[9..18], &[0, 2, 10, 1, 0, 3, 11, 2, 3]);
    }

    /// A file's header and frames.
    fn split(file: &[u8]) -> (Value, &[u8]) {
        let header_len = u32::from_be_bytes(file[5..9].try_into().unwrap()) as usize;
        (
            serde_json::from_slice(&file[9..9 + header_len]).unwrap(),
            &file[9 + header_len..],
        )
    }

    #[test]
    fn builds_live_a_piece_at_a_time() {
        let mut first = Vec::new();
        first.push(TICK);
        put_varint(&mut first, 0);
        put_bytes(&mut first, LOGIN, &[2]);
        put_bytes(&mut first, SERVER, &[10, 1]);
        put_bytes(&mut first, NOTE, b"Walk here");
        let mut second = Vec::new();
        second.push(TICK);
        put_varint(&mut second, 600);
        put_bytes(&mut second, SERVER, &[11, 2]);
        put_bytes(&mut second, NOTE, b"Take Logs");

        let whole = build(&[first.clone(), second.clone()].concat(), Map::new());
        let mut live = Builder::default();
        live.feed(&first);
        let start = live.take(Map::new());
        live.feed(&second);
        let next = live.take(Map::new());
        let nothing = live.take(Map::new());

        let (whole_header, whole_frames) = split(&whole);
        let (start_header, start_frames) = split(&start);
        let (next_header, next_frames) = split(&next);
        assert_eq!([start_frames, next_frames].concat(), whole_frames);
        assert_eq!(start_header["frames"], 2);
        assert_eq!(next_header["frames"], 1);
        assert_eq!(next_header["durationMs"], whole_header["durationMs"]);
        assert_eq!(next_header["notes"], json!([[600, "Take Logs"]]));
        let (nothing_header, nothing_frames) = split(&nothing);
        assert!(nothing_frames.is_empty());
        assert_eq!(nothing_header["notes"], json!([]));
    }

    #[test]
    fn types_sent_chat_again() {
        let mut rec = Vec::new();
        rec.push(TICK);
        put_varint(&mut rec, 6000);
        let mut body = vec![1, 1];
        body.extend(wordpack::pack("hi"));
        rec.push(CLIENT);
        rec.push(ClientProt::MessagePublic as u8);
        put_varint(&mut rec, body.len() as u32);
        rec.extend_from_slice(&body);

        let file = build(&rec, Map::new());
        let header_len = u32::from_be_bytes(file[5..9].try_into().unwrap()) as usize;
        let header: Value = serde_json::from_slice(&file[9..9 + header_len]).unwrap();
        let keys: Vec<&str> = header["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i[1] == 11)
            .map(|i| i[2].as_str().unwrap())
            .collect();
        assert_eq!(keys.concat(), "red:wave:HiEnter");
        assert_eq!(
            header["inputs"].as_array().unwrap().last().unwrap()[0],
            6000
        );
        assert_eq!(header["notes"][0], json!([6000, "Say \"Hi\""]));
    }
}
