use rs_crypto::isaac::{Isaac, IsaacPair};
use rs_io::Packet;
use rs_protocol::network::game::MAX_PACKET_SIZE;
use std::collections::VecDeque;
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

/// The engine-side handle for a single game client connection.
///
/// Holds the decoded packet inbox, the outbound byte sender, ISAAC cipher
/// state for opcode encryption/decryption, and an intermediate read queue
/// for reassembling fragmented messages.
pub struct ClientHandle {
    pub inbox: Receiver<(Instant, Vec<u8>)>,
    outbox: UnboundedSender<Vec<u8>>,
    unsent: Arc<AtomicUsize>,
    pub recycle_rx: UnboundedReceiver<Vec<u8>>,
    pub buffer_pool: Vec<Vec<u8>>,
    pub write_queue: Packet,
    pub read_queue: VecDeque<u8>,
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
            write_queue: Packet::new(MAX_PACKET_SIZE),
            read_queue: VecDeque::new(),
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

impl ClientHandle {
    /// Hands `bytes` to the socket task to write.
    pub fn send(&self, bytes: Vec<u8>) {
        self.unsent.fetch_add(bytes.len(), Ordering::Relaxed);
        let _ = self.outbox.send(bytes);
    }

    /// Whether the socket task is over [`MAX_UNSENT`] bytes behind.
    pub fn backlogged(&self) -> bool {
        !self.outbox.is_closed() && self.unsent.load(Ordering::Relaxed) > MAX_UNSENT
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
