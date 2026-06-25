use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::time::{Duration, sleep};
use tracing::{info, warn};

/// Opcodes for messages sent from this game world to the ether sidecar.
#[repr(u8)]
pub enum EtherOutboundOp {
    WorldRegister = 0,
    PlayerLogin = 1,
    PlayerLogout = 2,
    FriendAdd = 3,
    FriendDel = 4,
    IgnoreAdd = 5,
    IgnoreDel = 6,
    PrivateMessage = 7,
    RequestLists = 8,
    ChatModeUpdate = 9,
    PlayerResync = 10,
    LoginCheck = 11,
    RefreshAll = 12,
    LoginAbort = 13,
}

#[cfg_attr(rev = "225", allow(unused_variables))]
pub fn max_friends_cap(is_member: bool) -> u16 {
    #[cfg(since_244)]
    if is_member {
        return 200;
    }

    100
}

/// Opcodes for messages received from the ether sidecar.
#[repr(u8)]
pub enum EtherInboundOp {
    UpdateFriendList = 128,
    UpdateIgnoreList = 129,
    MessagePrivate = 130,
    FriendListComplete = 131,
    LoginCheckResponse = 132,
    WorldReady = 133,
}

/// Strongly-typed outbound messages to the ether sidecar.
///
/// Each variant corresponds to an [`EtherOutboundOp`] and carries its
/// payload fields.
#[derive(Debug)]
pub enum EtherOutbound {
    WorldRegister {
        node_id: u8,
    },
    PlayerLogin {
        user37: u64,
        pid: u16,
        max_friends: u16,
        ip: String,
    },
    PlayerLogout {
        user37: u64,
    },
    FriendAdd {
        owner37: u64,
        friend37: u64,
    },
    FriendDel {
        owner37: u64,
        friend37: u64,
    },
    IgnoreAdd {
        owner37: u64,
        ignore37: u64,
    },
    IgnoreDel {
        owner37: u64,
        ignore37: u64,
    },
    PrivateMessage {
        sender37: u64,
        target37: u64,
        level: u8,
        bytes: Vec<u8>,
    },
    RequestLists {
        user37: u64,
    },
    ChatModeUpdate {
        user37: u64,
        private_mode: u8,
    },
    PlayerResync {
        user37: u64,
        pid: u16,
        private_mode: u8,
        max_friends: u16,
        ip: String,
    },
    LoginCheck {
        user37: u64,
        max_per_ip: u8,
        ip: String,
    },
    LoginAbort {
        user37: u64,
        ip: String,
    },
    RefreshAll,
}

/// Strongly-typed inbound messages received from the ether sidecar.
///
/// Each variant corresponds to an [`EtherInboundOp`] and carries its
/// decoded payload. The `EtherReconnected` variant is synthetic -- it is
/// emitted locally when the connection is re-established.
#[derive(Debug)]
pub enum EtherInbound {
    UpdateFriendList {
        target37: u64,
        friend37: u64,
        node: u8,
    },
    UpdateIgnoreList {
        target37: u64,
        users37: Vec<u64>,
    },
    MessagePrivate {
        recipient37: u64,
        sender37: u64,
        msg_id: i32,
        level: u8,
        bytes: Vec<u8>,
    },
    FriendListComplete {
        target37: u64,
    },
    LoginCheckResponse {
        user37: u64,
        allowed: bool,
        ip_limited: bool,
    },
    WorldReady,
    EtherReconnected,
    EtherDisconnected,
}

impl EtherOutbound {
    /// Serializes this outbound message into the given byte buffer.
    ///
    /// Writes the opcode byte followed by the payload fields in
    /// big-endian format.
    ///
    /// # Arguments
    /// * `buf` - The output buffer to append encoded bytes to.
    pub fn encode(&self, buf: &mut Vec<u8>) {
        match self {
            Self::WorldRegister { node_id } => {
                buf.push(EtherOutboundOp::WorldRegister as u8);
                buf.push(*node_id);
            }
            Self::PlayerLogin {
                user37,
                pid,
                max_friends,
                ip,
            } => {
                buf.push(EtherOutboundOp::PlayerLogin as u8);
                buf.extend_from_slice(&user37.to_be_bytes());
                buf.extend_from_slice(&pid.to_be_bytes());
                buf.extend_from_slice(&max_friends.to_be_bytes());
                buf.extend_from_slice(ip.as_bytes());
            }
            Self::PlayerLogout { user37 } => {
                buf.push(EtherOutboundOp::PlayerLogout as u8);
                buf.extend_from_slice(&user37.to_be_bytes());
            }
            Self::FriendAdd { owner37, friend37 } => {
                buf.push(EtherOutboundOp::FriendAdd as u8);
                buf.extend_from_slice(&owner37.to_be_bytes());
                buf.extend_from_slice(&friend37.to_be_bytes());
            }
            Self::FriendDel { owner37, friend37 } => {
                buf.push(EtherOutboundOp::FriendDel as u8);
                buf.extend_from_slice(&owner37.to_be_bytes());
                buf.extend_from_slice(&friend37.to_be_bytes());
            }
            Self::IgnoreAdd { owner37, ignore37 } => {
                buf.push(EtherOutboundOp::IgnoreAdd as u8);
                buf.extend_from_slice(&owner37.to_be_bytes());
                buf.extend_from_slice(&ignore37.to_be_bytes());
            }
            Self::IgnoreDel { owner37, ignore37 } => {
                buf.push(EtherOutboundOp::IgnoreDel as u8);
                buf.extend_from_slice(&owner37.to_be_bytes());
                buf.extend_from_slice(&ignore37.to_be_bytes());
            }
            Self::PrivateMessage {
                sender37,
                target37,
                level,
                bytes,
            } => {
                buf.push(EtherOutboundOp::PrivateMessage as u8);
                buf.extend_from_slice(&sender37.to_be_bytes());
                buf.extend_from_slice(&target37.to_be_bytes());
                buf.push(*level);
                buf.extend_from_slice(bytes);
            }
            Self::RequestLists { user37 } => {
                buf.push(EtherOutboundOp::RequestLists as u8);
                buf.extend_from_slice(&user37.to_be_bytes());
            }
            Self::ChatModeUpdate {
                user37,
                private_mode,
            } => {
                buf.push(EtherOutboundOp::ChatModeUpdate as u8);
                buf.extend_from_slice(&user37.to_be_bytes());
                buf.push(*private_mode);
            }
            Self::PlayerResync {
                user37,
                pid,
                private_mode,
                max_friends,
                ip,
            } => {
                buf.push(EtherOutboundOp::PlayerResync as u8);
                buf.extend_from_slice(&user37.to_be_bytes());
                buf.extend_from_slice(&pid.to_be_bytes());
                buf.push(*private_mode);
                buf.extend_from_slice(&max_friends.to_be_bytes());
                buf.extend_from_slice(ip.as_bytes());
            }
            Self::LoginCheck {
                user37,
                max_per_ip,
                ip,
            } => {
                buf.push(EtherOutboundOp::LoginCheck as u8);
                buf.extend_from_slice(&user37.to_be_bytes());
                buf.push(*max_per_ip);
                buf.extend_from_slice(ip.as_bytes());
            }
            Self::LoginAbort { user37, ip } => {
                buf.push(EtherOutboundOp::LoginAbort as u8);
                buf.extend_from_slice(&user37.to_be_bytes());
                buf.extend_from_slice(ip.as_bytes());
            }
            Self::RefreshAll => {
                buf.push(EtherOutboundOp::RefreshAll as u8);
            }
        }
    }
}

impl EtherInbound {
    /// Decodes an inbound message from raw frame bytes.
    ///
    /// The first byte is the opcode; the remainder is the payload. Returns
    /// `None` if the data is empty, the opcode is unknown, or the payload
    /// is too short for the given opcode.
    ///
    /// # Arguments
    /// * `data` - The raw frame data (opcode + payload).
    ///
    /// # Returns
    /// `Some(message)` on success, or `None` if decoding fails.
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        let op = data[0];
        let payload = &data[1..];
        match op {
            op if op == EtherInboundOp::UpdateFriendList as u8 => {
                if payload.len() < 17 {
                    return None;
                }
                let target37 = u64::from_be_bytes(payload[0..8].try_into().ok()?);
                let friend37 = u64::from_be_bytes(payload[8..16].try_into().ok()?);
                let node = payload[16];
                Some(Self::UpdateFriendList {
                    target37,
                    friend37,
                    node,
                })
            }
            op if op == EtherInboundOp::UpdateIgnoreList as u8 => {
                if payload.len() < 10 {
                    return None;
                }
                let target37 = u64::from_be_bytes(payload[0..8].try_into().ok()?);
                let count = u16::from_be_bytes(payload[8..10].try_into().ok()?);
                let mut users37 = Vec::with_capacity(count as usize);
                let mut offset = 10;
                for _ in 0..count {
                    if offset + 8 > payload.len() {
                        break;
                    }
                    users37.push(u64::from_be_bytes(
                        payload[offset..offset + 8].try_into().ok()?,
                    ));
                    offset += 8;
                }
                Some(Self::UpdateIgnoreList { target37, users37 })
            }
            op if op == EtherInboundOp::MessagePrivate as u8 => {
                if payload.len() < 22 {
                    return None;
                }
                let recipient37 = u64::from_be_bytes(payload[0..8].try_into().ok()?);
                let sender37 = u64::from_be_bytes(payload[8..16].try_into().ok()?);
                let msg_id = i32::from_be_bytes(payload[16..20].try_into().ok()?);
                let level = payload[20];
                let bytes = payload[21..].to_vec();
                Some(Self::MessagePrivate {
                    recipient37,
                    sender37,
                    msg_id,
                    level,
                    bytes,
                })
            }
            op if op == EtherInboundOp::FriendListComplete as u8 => {
                if payload.len() < 8 {
                    return None;
                }
                let target37 = u64::from_be_bytes(payload[0..8].try_into().ok()?);
                Some(Self::FriendListComplete { target37 })
            }
            op if op == EtherInboundOp::LoginCheckResponse as u8 => {
                if payload.len() < 10 {
                    return None;
                }
                let user37 = u64::from_be_bytes(payload[0..8].try_into().ok()?);
                let allowed = payload[8] != 0;
                let ip_limited = payload[9] == 2;
                Some(Self::LoginCheckResponse {
                    user37,
                    allowed,
                    ip_limited,
                })
            }
            op if op == EtherInboundOp::WorldReady as u8 => Some(Self::WorldReady),
            _ => {
                warn!("Unknown ether inbound opcode: 0x{:02X}", op);
                None
            }
        }
    }
}

/// Long-running async task that maintains a TCP connection to the ether
/// (cross-server communication) sidecar.
///
/// Connects to `127.0.0.1:{port}` with exponential backoff. On each
/// successful connection, performs a handshake (sends `WorldRegister`,
/// waits for `WorldReady`), then enters a bidirectional message relay loop.
///
/// On the first successful handshake, signals `ready_tx` so the engine
/// knows the ether link is available. On subsequent reconnects, sends an
/// `EtherReconnected` event.
///
/// # Arguments
/// * `port` - The TCP port of the ether sidecar.
/// * `node_id` - This world's unique node identifier.
/// * `outbound_rx` - Channel receiver for messages to send to the sidecar.
/// * `inbound_tx` - Channel sender for messages received from the sidecar.
/// * `ready_tx` - One-shot sender signalled when the first handshake completes.
///
/// # Call Stack
/// **Calls:** [`run_connection`]
pub async fn ether_client_task(
    port: u16,
    node_id: u8,
    mut outbound_rx: UnboundedReceiver<EtherOutbound>,
    inbound_tx: UnboundedSender<EtherInbound>,
    ready_tx: oneshot::Sender<()>,
) {
    let addr = format!("127.0.0.1:{}", port);
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);
    let mut ready_tx = Some(ready_tx);

    loop {
        info!("Ether client connecting to {}...", addr);
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                info!("Ether client connected to {}", addr);
                backoff = Duration::from_secs(1);
                match run_connection(
                    stream,
                    node_id,
                    &mut outbound_rx,
                    &inbound_tx,
                    &mut ready_tx,
                )
                .await
                {
                    Ok(()) => {
                        info!("Ether outbound channel closed -- stopping ether client task");
                        return;
                    }
                    Err(e) => {
                        warn!("Ether connection lost: {}", e);
                        let _ = inbound_tx.send(EtherInbound::EtherDisconnected);
                    }
                }
            }
            Err(e) => {
                warn!("Ether connect failed: {} (retry in {:?})", e, backoff);
            }
        }
        sleep(backoff).await;
        backoff = (backoff * 2).min(max_backoff);
    }
}

/// Runs a single ether connection lifecycle: handshake then bidirectional
/// message relay until the connection drops.
///
/// Sends a `WorldRegister` frame, waits for a `WorldReady` response
/// during the handshake phase, then enters a `tokio::select!` loop that
/// forwards outbound messages to the sidecar and decodes inbound frames.
///
/// # Arguments
/// * `stream` - The connected TCP stream to the sidecar.
/// * `node_id` - This world's unique node identifier.
/// * `outbound_rx` - Channel receiver for messages to send.
/// * `inbound_tx` - Channel sender for received messages.
/// * `ready_tx` - One-shot sender consumed on first successful handshake.
///
/// # Returns
/// `Ok(())` when the outbound channel closes, or an I/O error when the
/// connection is lost.
///
/// # Call Stack
/// **Called by:** [`ether_client_task`]
/// **Calls:** [`write_frame`], [`try_read_frame`],
/// [`EtherOutbound::encode`], [`EtherInbound::decode`]
async fn run_connection(
    mut stream: TcpStream,
    node_id: u8,
    outbound_rx: &mut UnboundedReceiver<EtherOutbound>,
    inbound_tx: &UnboundedSender<EtherInbound>,
    ready_tx: &mut Option<oneshot::Sender<()>>,
) -> std::io::Result<()> {
    stream.set_nodelay(true)?;

    let mut frame_buf = Vec::with_capacity(256);
    frame_buf.clear();
    EtherOutbound::WorldRegister { node_id }.encode(&mut frame_buf);
    write_frame(&mut stream, &frame_buf).await?;

    let mut read_buf = vec![0u8; 4096];
    let mut pending = Vec::new();

    // Wait for WorldReady before signaling the engine and entering the main loop.
    loop {
        let n = stream.read(&mut read_buf).await?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "ether connection closed during handshake",
            ));
        }
        pending.extend_from_slice(&read_buf[..n]);
        let mut ready = false;
        while let Some(frame) = try_read_frame(&mut pending) {
            if let Some(msg) = EtherInbound::decode(&frame) {
                match msg {
                    EtherInbound::WorldReady => {
                        info!("Ether sidecar ready (handshake complete)");
                        ready = true;
                    }
                    other => {
                        let _ = inbound_tx.send(other);
                    }
                }
            }
        }
        if ready {
            break;
        }
    }

    let mut dropped = 0;
    while outbound_rx.try_recv().is_ok() {
        dropped += 1;
    }
    if dropped > 0 {
        info!("Ether reconnect: dropped {dropped} stale outbound messages");
    }

    if let Some(tx) = ready_tx.take() {
        let _ = tx.send(());
    }
    let _ = inbound_tx.send(EtherInbound::EtherReconnected);

    loop {
        tokio::select! {
            msg = outbound_rx.recv() => {
                let Some(msg) = msg else {
                    return Ok(());
                };
                frame_buf.clear();
                msg.encode(&mut frame_buf);
                write_frame(&mut stream, &frame_buf).await?;
            }
            result = stream.read(&mut read_buf) => {
                let n = result?;
                if n == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::ConnectionReset,
                        "ether connection closed",
                    ));
                }
                pending.extend_from_slice(&read_buf[..n]);
                while let Some(frame) = try_read_frame(&mut pending) {
                    if let Some(msg) = EtherInbound::decode(&frame) {
                        let _ = inbound_tx.send(msg);
                    }
                }
            }
        }
    }
}

/// Writes a length-prefixed frame to the TCP stream.
///
/// The frame format is a 2-byte big-endian length followed by the payload
/// bytes.
///
/// # Arguments
/// * `stream` - The TCP stream to write to.
/// * `payload` - The frame payload bytes.
///
/// # Returns
/// `Ok(())` on success, or an I/O error.
async fn write_frame(stream: &mut TcpStream, payload: &[u8]) -> std::io::Result<()> {
    let len = payload.len() as u16;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(payload).await?;
    Ok(())
}

/// Attempts to extract a complete length-prefixed frame from the pending
/// read buffer.
///
/// If at least 2 bytes are available, reads the big-endian length prefix.
/// If the buffer contains enough bytes for the full frame, extracts and
/// returns the payload, draining the consumed bytes from `buf`.
///
/// # Arguments
/// * `buf` - The pending read buffer (accumulated TCP reads).
///
/// # Returns
/// `Some(frame)` if a complete frame was available, `None` otherwise.
fn try_read_frame(buf: &mut Vec<u8>) -> Option<Vec<u8>> {
    if buf.len() < 2 {
        return None;
    }
    let len = u16::from_be_bytes([buf[0], buf[1]]) as usize;
    if buf.len() < 2 + len {
        return None;
    }
    let frame = buf[2..2 + len].to_vec();
    buf.drain(..2 + len);
    Some(frame)
}
