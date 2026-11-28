use rs_crypto::isaac::{Isaac, IsaacPair};
use rs_io::{Packet, PacketFrame};
use rs_protocol::network::game::MAX_PACKET_SIZE;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::client_prot_frame::ClientProtFrame;
use rs_protocol::network::game::server::ServerProtMessage;
use std::collections::VecDeque;
use std::mem;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedReceiver, UnboundedSender};

/// Maximum number of inbound packet messages that can be buffered before
/// back-pressure is applied by the channel.
const INBOX_CAPACITY: usize = 128;

/// Channel capacity for the disconnect signal (only one signal is ever sent).
const DISCONNECT_CAPACITY: usize = 1;

/// The most bytes a client can leave unwritten before it's disconnected: one
/// that reads slower than the game writes would otherwise queue without end.
/// About a minute of output in a crowd.
pub const MAX_UNSENT: usize = 1 << 20;

/// Most recycled output buffers a client keeps for reuse; extras are freed.
const OUTPUT_POOL_CAP: usize = 8;

/// The engine-side handle for a single game client connection.
///
/// Holds the decoded packet inbox, the outbound byte sender, ISAAC cipher
/// state for opcode encryption/decryption, and an intermediate read queue
/// for reassembling fragmented messages.
pub struct ClientHandle {
    pub inbox: Receiver<(Instant, Vec<u8>)>,
    outbox: UnboundedSender<Vec<u8>>,
    unsent: Arc<AtomicUsize>,
    recycle_rx: UnboundedReceiver<Vec<u8>>,
    buffer_pool: Vec<Vec<u8>>,
    gathered: Vec<u8>,
    pub write_queue: Packet,
    pub read_queue: VecDeque<u8>,
    pub read_buf: Vec<u8>,
    pub pending_msg: Option<(Instant, Vec<u8>)>,
    pub arrivals: VecDeque<(u64, Instant)>,
    pub queued: u64,
    pub isaac_encode: Isaac,
    pub isaac_decode: Isaac,
    pub opcode: Option<u8>,
    pub disconnect_rx: Receiver<()>,
}

/// Bundles the engine-side [`ClientHandle`] together with the network-side
/// channel endpoints needed by the I/O task to relay packets and detect
/// disconnection.
pub struct ClientIO {
    pub handle: ClientHandle,
    pub packet_tx: Sender<(Instant, Vec<u8>)>,
    pub bytes_rx: UnboundedReceiver<Vec<u8>>,
    pub recycle_tx: UnboundedSender<Vec<u8>>,
    pub disconnect_tx: Sender<()>,
    pub unsent: Arc<AtomicUsize>,
}

/// Creates the paired I/O channels for a new game client connection.
///
/// Returns a [`ClientIO`] containing:
/// * A [`ClientHandle`] (engine side) with the packet inbox receiver, byte
///   outbox sender, a [`MAX_PACKET_SIZE`] write queue, and the ISAAC cipher pair.
/// * The corresponding network-side senders/receivers (`packet_tx`,
///   `bytes_rx`, `disconnect_tx`) that the socket task uses.
///
/// # Arguments
/// * `isaac` - The ISAAC cipher pair negotiated during the login handshake,
///   used for opcode encoding and decoding.
///
/// # Returns
/// A fully wired [`ClientIO`] ready to be split between the engine and the
/// network I/O task.
pub fn create_io(isaac: IsaacPair) -> ClientIO {
    let (isaac_encode, isaac_decode) = (isaac.encode, isaac.decode);
    let (packet_tx, packet_rx) = mpsc::channel(INBOX_CAPACITY);
    let (bytes_tx, bytes_rx) = mpsc::unbounded_channel();
    let (recycle_tx, recycle_rx) = mpsc::unbounded_channel();
    let (disconnect_tx, disconnect_rx) = mpsc::channel(DISCONNECT_CAPACITY);
    let unsent = Arc::new(AtomicUsize::new(0));

    ClientIO {
        handle: ClientHandle {
            inbox: packet_rx,
            outbox: bytes_tx,
            unsent: unsent.clone(),
            recycle_rx,
            buffer_pool: Vec::new(),
            gathered: Vec::new(),
            write_queue: Packet::new(MAX_PACKET_SIZE),
            read_queue: VecDeque::new(),
            read_buf: Vec::new(),
            pending_msg: None,
            arrivals: VecDeque::new(),
            queued: 0,
            isaac_encode,
            isaac_decode,
            opcode: None,
            disconnect_rx,
        },
        packet_tx,
        bytes_rx,
        recycle_tx,
        disconnect_tx,
        unsent,
    }
}

/// Ciphers the opcode that starts `frame` with the next ISAAC values: one
/// byte, or from 647 two when the first is 128 or more.
pub(crate) fn cipher_opcode(isaac: &mut Isaac, frame: &mut [u8]) {
    #[cfg(before_647)]
    let len = 1;
    #[cfg(since_647)]
    let len = if frame[0] < 128 { 1 } else { 2 };
    for byte in &mut frame[..len] {
        *byte = byte.wrapping_add(isaac.take_next_value() as u8);
    }
}

impl ClientHandle {
    pub(crate) fn encode_into<M>(out: &mut Packet, message: M) -> bool
    where
        M: ServerProtMessage,
    {
        let frame = M::FRAME as usize;
        #[cfg(before_647)]
        let opcode_bytes = 1;
        #[cfg(since_647)]
        let opcode_bytes = if (M::PROT as u16) < 128 { 1 } else { 2 };
        let len = opcode_bytes + frame + message.sizeof();
        if len > MAX_PACKET_SIZE
            || (matches!(M::FRAME, PacketFrame::VarByte) && message.sizeof() > u8::MAX as usize)
        {
            return false;
        }
        if out.data.len() < out.pos + len {
            out.data.resize(out.pos + len, 0);
        }
        #[cfg(before_647)]
        out.p1(M::PROT as u8);
        #[cfg(since_647)]
        out.psmart1or2(M::PROT as i32);
        out.pos += frame;
        let start = out.pos;
        message.encode(out);
        match M::FRAME {
            PacketFrame::Fixed => {}
            PacketFrame::VarByte => out.psize1((out.pos - start) as u8),
            PacketFrame::VarShort => out.psize2((out.pos - start) as u16),
        }
        true
    }

    pub fn plaintext<M: ServerProtMessage>(message: M) -> Option<Packet> {
        let mut buf = Packet::new(0);
        if !Self::encode_into(&mut buf, message) {
            return None;
        }
        buf.data.truncate(buf.pos);
        Some(buf)
    }

    /// Encodes `message` into the write queue with its opcode ciphered and
    /// gathers it. `false` if it is too big to send.
    pub fn write<M: ServerProtMessage>(&mut self, message: M) -> bool {
        self.write_queue.pos = 0;
        if !Self::encode_into(&mut self.write_queue, message) {
            return false;
        }
        cipher_opcode(&mut self.isaac_encode, &mut self.write_queue.data);
        self.gather_written(self.write_queue.pos);
        true
    }

    /// Places the handshake reply before the frames prepared by the login script.
    #[cfg(since_647)]
    pub fn finish_lobby_login(&mut self, reply: Vec<u8>, accepted: bool) {
        self.push(reply);
        if accepted {
            self.flush();
        } else {
            self.gathered.clear();
        }
    }

    /// Hands `bytes` to the socket task to write, after anything gathered.
    pub fn send(&mut self, bytes: Vec<u8>) {
        self.flush();
        self.push(bytes);
    }

    /// Adds `bytes` to the output [`flush`](Self::flush) writes in one go.
    pub fn gather(&mut self, bytes: &[u8]) {
        self.reuse_buffer();
        self.gathered.extend_from_slice(bytes);
    }

    /// [`gather`](Self::gather)s the first `len` bytes of the write queue.
    pub fn gather_written(&mut self, len: usize) {
        self.reuse_buffer();
        self.gathered
            .extend_from_slice(&self.write_queue.data[..len]);
    }

    /// Gathers a var-short packet: `opcode` (already ciphered), the body's
    /// length, then the body, `parts` in order. Returns the length and body
    /// as gathered.
    pub fn gather_varshort(&mut self, opcode: u8, parts: [&[u8]; 2]) -> &[u8] {
        self.reuse_buffer();
        let size = parts[0].len() + parts[1].len();
        self.gathered.reserve(3 + size);
        self.gathered.push(opcode);
        let start = self.gathered.len();
        self.gathered
            .extend_from_slice(&(size as u16).to_be_bytes());
        self.gathered.extend_from_slice(parts[0]);
        self.gathered.extend_from_slice(parts[1]);
        &self.gathered[start..]
    }

    /// Hands the gathered output to the socket task as one write.
    pub fn flush(&mut self) {
        if self.outbox.is_closed() {
            self.gathered.clear();
            return;
        }
        if !self.gathered.is_empty() {
            let bytes = mem::take(&mut self.gathered);
            self.push(bytes);
        }
    }

    fn push(&self, bytes: Vec<u8>) {
        self.unsent.fetch_add(bytes.len(), Ordering::Relaxed);
        let _ = self.outbox.send(bytes);
    }

    /// Starts the gathered output in a buffer the socket task gave back.
    fn reuse_buffer(&mut self) {
        if self.gathered.capacity() > 0 {
            return;
        }
        while let Ok(returned) = self.recycle_rx.try_recv() {
            if self.buffer_pool.len() < OUTPUT_POOL_CAP {
                self.buffer_pool.push(returned);
            }
        }
        if let Some(mut buf) = self.buffer_pool.pop() {
            buf.clear();
            self.gathered = buf;
        }
    }

    /// Whether the socket task is over [`MAX_UNSENT`] bytes behind.
    pub fn backlogged(&self) -> bool {
        !self.outbox.is_closed() && self.unsent.load(Ordering::Relaxed) > MAX_UNSENT
    }

    pub fn pull_input(&mut self) -> bool {
        let mut received = false;
        let mut next = self.pending_msg.take();
        loop {
            let (at, mut bytes) = match next.take() {
                Some(m) => m,
                None => match self.inbox.try_recv() {
                    Ok(m) => {
                        received = true;
                        m
                    }
                    Err(_) => break,
                },
            };
            let available = MAX_PACKET_SIZE - self.read_queue.len();
            if bytes.len() > available {
                if available == 0 {
                    self.pending_msg = Some((at, bytes));
                } else {
                    let rest = bytes.split_off(available);
                    self.queue(at, bytes);
                    self.pending_msg = Some((at, rest));
                }
                break;
            }
            self.queue(at, bytes);
        }

        received
    }

    pub fn next_packet(&mut self) -> Result<Option<(ClientProt, Packet)>, String> {
        if self.opcode.is_none() {
            let Some(byte) = self.read_queue.pop_front() else {
                return Ok(None);
            };
            self.opcode = Some(byte.wrapping_sub(self.isaac_decode.take_next_value() as u8));
        }
        let opcode = self.opcode.unwrap();
        let prot = ClientProt::try_from(opcode)
            .map_err(|_| format!("Unrecognized packet type: {opcode}"))?;
        let queue = &self.read_queue;
        let (head, len) = match prot.info().frame {
            ClientProtFrame::VarByte { .. } if !queue.is_empty() => (1, queue[0] as usize),
            ClientProtFrame::VarShort { .. } if queue.len() >= 2 => {
                (2, u16::from_be_bytes([queue[0], queue[1]]) as usize)
            }
            ClientProtFrame::Fixed(len) => (0, len as usize),
            _ => return Ok(None),
        };
        if head + len > MAX_PACKET_SIZE {
            return Err(format!("{prot:?} of {len} bytes, closing the connection"));
        }
        if queue.len() < head + len {
            return Ok(None);
        }
        self.opcode = None;
        self.read_queue.drain(..head);
        let mut buf = Packet::from(mem::take(&mut self.read_buf));
        buf.data.clear();
        buf.data.extend(self.read_queue.drain(..len));
        Ok(Some((prot, buf)))
    }

    /// Puts a read from the socket in the read queue.
    pub fn queue(&mut self, at: Instant, bytes: Vec<u8>) {
        self.queued += bytes.len() as u64;
        self.arrivals.push_back((self.queued, at));
        self.read_queue.extend(bytes);
    }

    /// Closes the connection: dropping this end of its channels stops the
    /// socket task. Whatever was read and not yet decoded is thrown away.
    pub fn close(&mut self) {
        self.flush();
        self.inbox = mpsc::channel(1).1;
        self.outbox = mpsc::unbounded_channel().0;
        self.read_queue.clear();
        self.arrivals.clear();
        self.pending_msg = None;
        self.opcode = None;
    }

    /// When the last byte taken from the read queue arrived.
    pub fn arrived(&mut self) -> Option<Instant> {
        let taken = self.queued - self.read_queue.len() as u64;
        while self.arrivals.front().is_some_and(|&(end, _)| end < taken) {
            self.arrivals.pop_front();
        }
        self.arrivals.front().map(|&(_, at)| at)
    }
}

impl Drop for ClientHandle {
    fn drop(&mut self) {
        self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parts land as the one var-short packet the write queue would
    /// have built from their join.
    #[test]
    fn gather_varshort_matches_a_joined_packet() {
        let bits: Vec<u8> = (0..300).map(|i| i as u8).collect();
        let updates: Vec<u8> = (0..41).map(|i| (i * 7) as u8).collect();
        for parts in [[&bits[..], &updates[..]], [&bits[..], &[]], [&[], &[]]] {
            let body = parts.concat();
            let mut joined = Packet::new(MAX_PACKET_SIZE);
            joined.p1(0xAB);
            joined.pos += 2;
            joined.pdata(&body, 0, body.len());
            joined.psize2(body.len() as u16);

            let mut handle = create_io(IsaacPair::new(&[0; 4], &[0; 4])).handle;
            let rest = handle.gather_varshort(0xAB, parts).to_vec();
            assert_eq!(handle.gathered, joined.data[..joined.pos]);
            assert_eq!(rest, joined.data[1..joined.pos]);
        }
    }
}
