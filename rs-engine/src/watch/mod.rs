pub mod anticheat;
pub mod format;
pub mod input;
mod keyframe;
pub mod menu;
pub mod playback;
mod state;

pub use anticheat::{AntiCheat, MapBase, Op};

/// User agent words only automation puts there: headless Chrome (Puppeteer,
/// Playwright, Selenium) and PhantomJS.
const AUTOMATION: [&str; 2] = ["HeadlessChrome", "PhantomJS"];

/// Whether a login's device says it's automated: a web client with no user
/// agent, or one that names a headless browser.
pub fn automated(device: &rs_db::LoginDevice) -> bool {
    device.web
        && device
            .user_agent
            .as_deref()
            .is_none_or(|ua| AUTOMATION.iter().any(|word| ua.contains(word)))
}

use crate::counters::{self, COUNTERS};
use format::{
    CLIENT, KEYFRAME, LOGIN, NOTE, RECONNECT, SERVER, TICK, TICK_DELTA, put_bytes, put_varint,
};
use rs_db::ReplayReport;
use rs_protocol::network::game::client_prot::ClientProt;
use state::ClientState;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::Sender;

/// A chunk is handed over after this long...
pub const CHUNK_MS: u32 = 30_000;
/// ...or this long while staff watch it live, so they're seconds behind.
pub const LIVE_CHUNK_MS: u32 = 2_000;
/// ...or once it's this big, whichever comes first.
pub const CHUNK_BYTES: usize = 64 * 1024;
/// How many requests the recording task can fall behind by before chunks
/// are lost.
pub const QUEUE: usize = 4096;
/// How often a recording takes a keyframe: a replay seeking anywhere plays
/// at most this much to get there.
pub const KEYFRAME_MS: u32 = 60_000;
/// Before 274, how often (in ticks) the client is asked for its input
/// report, which it otherwise holds until it has 500 bytes.
#[cfg(before_274)]
pub const TRACKING_TICKS: u32 = 17;
/// `ENABLE_TRACKING`'s opcode: the client's input report starts a new run.
#[cfg(before_274)]
pub const TRACKING_ENABLED: u8 =
    rs_protocol::network::game::server_prot::ServerProt::EnableTracking as u8;

/// What a "use" or spell action was done with.
#[derive(Clone, Copy, Debug)]
pub enum Using {
    Item(u16),
    Spell(u16),
}

/// What a "use" or spell action was done on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Npc(u16),
    Loc { id: u16, x: u16, z: u16 },
    Obj { id: u16, x: u16, z: u16 },
    Player(u16),
    Item(u16),
}

/// What a world asks its recording task to do.
pub enum ReplayRequest {
    Start {
        id: i64,
        user37: u64,
    },
    Store {
        id: i64,
        chunk: Option<RawChunk>,
        duration_ms: u32,
        report: ReplayReport,
        ended: bool,
        complete: bool,
    },
}

/// A chunk of a recording, not yet compressed.
pub struct RawChunk {
    pub seq: i32,
    pub start_ms: u32,
    pub end_ms: u32,
    pub data: Vec<u8>,
}

struct Recording {
    id: i64,
    tx: Sender<ReplayRequest>,
    buf: Vec<u8>,
    seq: i32,
    chunk_start_ms: u32,
    tick: Option<u32>,
    /// When the last tick entry began.
    tick_ms: u32,
    lost: bool,
    state: ClientState,
    last_keyframe_ms: u32,
}

/// A player's session as moderators see it.
pub struct Watch {
    pub anticheat: AntiCheat,
    started: Instant,
    recording: Option<Recording>,
    /// Until when staff watch it live (see [`Self::watch_live`]).
    live_until: Option<Instant>,
}

impl Default for Watch {
    fn default() -> Self {
        Self {
            anticheat: AntiCheat::default(),
            started: Instant::now(),
            recording: None,
            live_until: None,
        }
    }
}

/// A new recording's id: when it started (Unix ms), the world, and a count
/// to tell apart logins in the same millisecond. Ids sort by start.
fn new_id(node_id: u8) -> i64 {
    static COUNT: AtomicU8 = AtomicU8::new(0);
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    (ms << 16) | (i64::from(node_id) << 8) | i64::from(COUNT.fetch_add(1, Ordering::Relaxed))
}

impl Watch {
    /// Starts watching a session that just logged in, recording it through
    /// `tx` if given.
    pub fn start(user37: u64, node_id: u8, tx: Option<&Sender<ReplayRequest>>) -> Self {
        let recording = tx.and_then(|tx| {
            let id = new_id(node_id);
            tx.try_send(ReplayRequest::Start { id, user37 }).ok()?;
            Some(Recording {
                id,
                tx: tx.clone(),
                buf: Vec::with_capacity(CHUNK_BYTES / 4),
                seq: 0,
                chunk_start_ms: 0,
                tick: None,
                tick_ms: 0,
                lost: false,
                state: ClientState::default(),
                last_keyframe_ms: 0,
            })
        });
        Self {
            anticheat: AntiCheat::default(),
            started: Instant::now(),
            recording,
            live_until: None,
        }
    }

    /// The recording's id, while it goes on whole.
    pub fn recording_id(&self) -> Option<i64> {
        self.recording.as_ref().filter(|r| !r.lost).map(|r| r.id)
    }

    /// Staff watch recording `id` live for the next `secs` seconds: its
    /// chunks are stored every [`LIVE_CHUNK_MS`] meanwhile. Whether it's
    /// this one (and still whole).
    pub fn watch_live(&mut self, id: i64, secs: u64) -> bool {
        if self.recording_id() != Some(id) {
            return false;
        }
        self.live_until = Some(Instant::now() + std::time::Duration::from_secs(secs));
        true
    }

    /// Milliseconds into the session.
    pub fn ms(&self) -> u32 {
        self.started.elapsed().as_millis() as u32
    }

    /// The recording to append to on tick `clock`, with the tick's entry
    /// written. `None` when not recording.
    fn buf(&mut self, clock: u32) -> Option<&mut Vec<u8>> {
        let recording = self.recording.as_mut().filter(|r| !r.lost)?;
        if recording.tick != Some(clock) {
            let ms = self.started.elapsed().as_millis() as u32;
            if recording.tick.is_none() {
                recording.buf.push(TICK);
                put_varint(&mut recording.buf, ms);
            } else {
                recording.buf.push(TICK_DELTA);
                put_varint(&mut recording.buf, ms.saturating_sub(recording.tick_ms));
            }
            recording.tick = Some(clock);
            recording.tick_ms = ms;
        }
        Some(&mut recording.buf)
    }

    /// Whether the session is being recorded (whole): notes for the replay's
    /// timeline are only worth making then.
    pub fn recording(&self) -> bool {
        self.recording.as_ref().is_some_and(|r| !r.lost)
    }

    /// Packets sent to the client: `opcode` then `rest` (frame and body).
    pub fn server(&mut self, clock: u32, opcode: u8, rest: &[u8]) {
        #[cfg(before_274)]
        if opcode == TRACKING_ENABLED {
            let ms = self.ms();
            self.anticheat.inputs.tracking_restarted(ms);
        }
        if let Some(buf) = self.buf(clock) {
            buf.push(SERVER);
            put_varint(buf, rest.len() as u32 + 1);
            buf.push(opcode);
            buf.extend_from_slice(rest);
        }
    }

    /// A packet queued for the client: `opcode`, then `rest` (a `frame`-byte
    /// length, then the body). Kept if it sets state a keyframe brings back.
    pub fn remember(&mut self, opcode: u8, frame: usize, rest: &[u8]) {
        if let Some(recording) = self.recording.as_mut().filter(|r| !r.lost) {
            recording.state.remember(opcode, frame, rest);
        }
    }

    /// Whether it's time for a keyframe.
    pub fn keyframe_due(&self) -> bool {
        let ms = self.ms();
        self.recording
            .as_ref()
            .is_some_and(|r| !r.lost && ms.saturating_sub(r.last_keyframe_ms) >= KEYFRAME_MS)
    }

    /// Records a keyframe on tick `clock`: `head` (the map and what's on it),
    /// what the client holds on to, then `tail` (inventories, players and
    /// NPCs). Each is a packet: opcode, frame and body.
    pub fn keyframe(&mut self, clock: u32, head: &[Vec<u8>], tail: &[Vec<u8>]) {
        let ms = self.ms();
        let Some(recording) = self.recording.as_mut().filter(|r| !r.lost) else {
            return;
        };
        recording.last_keyframe_ms = ms;
        let mut payload = Vec::with_capacity(16 * 1024);
        let kept = recording.state.packets();
        for packet in head
            .iter()
            .map(Vec::as_slice)
            .chain(kept)
            .chain(tail.iter().map(Vec::as_slice))
        {
            payload.extend_from_slice(&(packet.len() as u16).to_be_bytes());
            payload.extend_from_slice(packet);
        }
        if let Some(buf) = self.buf(clock) {
            put_bytes(buf, KEYFRAME, &payload);
        }
    }

    /// A packet from the client: `prot`, sent as `opcode`.
    pub fn client(&mut self, clock: u32, prot: &ClientProt, opcode: u8, data: &[u8]) {
        self.anticheat.client(prot, data);
        if let Some(buf) = self.buf(clock) {
            buf.push(CLIENT);
            buf.push(opcode);
            put_varint(buf, data.len() as u32);
            buf.extend_from_slice(data);
        }
    }

    pub fn login(&mut self, clock: u32, reply: &[u8]) {
        if let Some(buf) = self.buf(clock) {
            put_bytes(buf, LOGIN, reply);
        }
    }

    pub fn reconnect(&mut self, clock: u32, reply: &[u8]) {
        if let Some(buf) = self.buf(clock) {
            put_bytes(buf, RECONNECT, reply);
        }
    }

    /// Notes what the player did, for the replay's timeline.
    pub fn note(&mut self, clock: u32, text: &str) {
        if let Some(buf) = self.buf(clock) {
            put_bytes(buf, NOTE, text.as_bytes());
        }
    }

    /// A menu action, for the checks (see [`AntiCheat::op`]).
    pub fn op(&mut self, op: Op, clock: u32, base: MapBase) {
        self.anticheat.op(op, clock, base);
    }

    /// At the end of each tick: hands over the chunk when it's due.
    pub fn end_tick(&mut self) {
        let ms = self.ms();
        let live = self.live_until.is_some_and(|until| Instant::now() < until);
        let due = self.recording.as_ref().is_some_and(|r| {
            let age = ms.saturating_sub(r.chunk_start_ms);
            r.buf.len() >= CHUNK_BYTES
                || age >= CHUNK_MS
                || (live && age >= LIVE_CHUNK_MS && !r.buf.is_empty())
        });
        if due {
            self.flush(false);
        }
    }

    fn flush(&mut self, ended: bool) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        let ms = self.started.elapsed().as_millis() as u32;
        let report = self.anticheat.report();
        let chunk = (!recording.lost && !recording.buf.is_empty()).then(|| RawChunk {
            seq: recording.seq,
            start_ms: recording.chunk_start_ms,
            end_ms: ms,
            data: std::mem::replace(&mut recording.buf, Vec::with_capacity(CHUNK_BYTES / 4)),
        });
        let had_chunk = chunk.is_some();
        let sent = recording.tx.try_send(ReplayRequest::Store {
            id: recording.id,
            chunk,
            duration_ms: ms,
            report,
            ended,
            complete: !recording.lost,
        });
        if sent.is_err() && had_chunk {
            recording.lost = true;
            counters::add(&COUNTERS.replay_chunks_dropped, 1);
        }
        recording.seq += 1;
        recording.chunk_start_ms = ms;
        // A new chunk starts with its tick's entry, in full.
        recording.tick = None;
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.flush(true);
    }
}

#[cfg(test)]
mod tests {
    use super::format::{Entry, Reader, TICK, TICK_DELTA};
    use super::{ReplayRequest, Watch, automated};
    use rs_db::LoginDevice;

    #[test]
    fn each_chunk_starts_with_a_whole_tick() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let mut watch = Watch::start(42, 10, Some(&tx));
        for clock in 0..3 {
            watch.note(clock, "a");
        }
        watch.flush(false);
        for clock in 3..5 {
            watch.note(clock, "b");
        }
        drop(watch);
        let mut chunks = Vec::new();
        while let Ok(req) = rx.try_recv() {
            if let ReplayRequest::Store {
                chunk: Some(chunk), ..
            } = req
            {
                chunks.push(chunk.data);
            }
        }
        assert_eq!(chunks.len(), 2);
        for (chunk, ticks) in chunks.iter().zip([3, 2]) {
            assert_eq!(chunk[0], TICK);
            // The note after the first tick, then the next tick's tag.
            let next = 1 + chunk[1..].iter().position(|&b| b & 0x80 == 0).unwrap() + 1 + 3;
            assert_eq!(chunk[next], TICK_DELTA);
            let read = Reader::new(chunk)
                .filter(|e| matches!(e, Entry::Tick { .. }))
                .count();
            assert_eq!(read, ticks);
        }
    }

    #[test]
    fn headless_browsers_are_automated() {
        let web = |ua: Option<&str>| LoginDevice {
            web: true,
            user_agent: ua.map(str::to_owned),
            ..LoginDevice::default()
        };
        assert!(automated(&web(Some(
            "Mozilla/5.0 (X11; Linux x86_64) HeadlessChrome/141.0.0.0"
        ))));
        assert!(automated(&web(None)));
        assert!(!automated(&web(Some(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/141.0"
        ))));
        // The desktop client never sends one.
        assert!(!automated(&LoginDevice::default()));
    }
}
