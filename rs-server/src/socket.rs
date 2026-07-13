use crate::{ConnectionGuard, ConnectionPermit, REVISION, ServerIO, Socket};
use anyhow::bail;
use mpsc::{Sender, UnboundedReceiver, UnboundedSender};
use num_enum::TryFromPrimitive;
use rand::RngExt;
use rs_crypto::isaac::IsaacPair;
use rs_engine::LoginRequest;
use rs_engine::{ClientIO, create_io};
use rs_io::Packet;
use rs_io::packet::RsaFrame;
use rs_protocol::LoginResponse;
#[cfg(since_244)]
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{info, warn};

#[repr(u8)]
#[derive(TryFromPrimitive)]
pub enum HandshakeType {
    New = 14,
    Js5 = 15,
    Login = 16,
    Reconnect = 18,
}

pub async fn serve(
    host: String,
    port: u16,
    server_state: ServerIO,
    guard: ConnectionGuard,
) -> anyhow::Result<()> {
    let bind_addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    let listener = TcpListener::bind(bind_addr).await?;

    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!("TCP accept error: {e}");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        if stream.set_nodelay(true).is_err() {
            continue; // peer already gone
        }
        info!("TCP connection from {}", addr);
        let server_state = server_state.clone();
        let guard = guard.clone();
        tokio::spawn(async move {
            let connection = Socket::from_tcp(stream, addr, server_state, guard);
            if let Err(e) = handshake(connection).await {
                info!("TCP connection {} closed: {}", addr, e);
            }
        });
    }
}

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_STALL_TIMEOUT: Duration = Duration::from_secs(60);

fn make_seed() -> Packet {
    let mut seed = Packet::new(8);
    seed.p4(rand::rng().random_range(0..0x00ffffff_u32) as i32);
    seed.p4(rand::rng().random_range(0..0xffffffff_u32) as i32);
    seed
}

async fn acquire_permit(client: &mut Socket) -> anyhow::Result<ConnectionPermit> {
    match client.guard.try_acquire(client.addr.ip()) {
        Some(permit) => Ok(permit),
        None => {
            let _ = client
                .write(&[LoginResponse::TooManyConnections as u8])
                .await;
            bail!("{} connection limit reached", client.addr);
        }
    }
}

#[cfg(rev = "225")]
pub async fn handshake(mut client: Socket) -> anyhow::Result<()> {
    client.write(&make_seed().data).await?;
    let permit = acquire_permit(&mut client).await?;
    // A connection that never speaks must not hold its permit forever.
    let Ok(result) = timeout(HANDSHAKE_TIMEOUT, client.read()).await else {
        bail!("{}: handshake timeout", client.addr)
    };
    let Some(bytes) = result? else {
        bail!("no bytes")
    };
    if bytes.is_empty() {
        bail!("empty bytes")
    }
    let mut buf = Packet::from(bytes);
    match HandshakeType::try_from(buf.g1())? {
        h @ (HandshakeType::Login | HandshakeType::Reconnect) => {
            process_login(client, buf, matches!(h, HandshakeType::Reconnect)).await?;
        }
        other => bail!("{}: unexpected handshake type {}", client.addr, other as u8),
    }
    drop(permit);
    Ok(())
}

#[cfg(since_244)]
pub async fn handshake(mut client: Socket) -> anyhow::Result<()> {
    let permit = acquire_permit(&mut client).await?;

    let mut key_exchanges = 0;
    let (handshake, buf) = loop {
        // A connection that never speaks must not hold its permit forever.
        let Ok(result) = timeout(HANDSHAKE_TIMEOUT, client.read()).await else {
            bail!("{}: handshake timeout", client.addr)
        };
        let Some(bytes) = result? else {
            bail!("no bytes")
        };
        if bytes.is_empty() {
            bail!("empty bytes")
        }
        let mut buf = Packet::from(bytes);
        match HandshakeType::try_from(buf.g1())? {
            HandshakeType::New => {
                // The client requests one seed per login attempt; a stream of
                // them is someone keeping the loop (and its permit) spinning.
                key_exchanges += 1;
                if key_exchanges > 3 {
                    bail!("{}: repeated session-key exchanges", client.addr);
                }
                let _name_hash = buf.g1();
                let mut resp = vec![0u8; 9]; // 8 ignored bytes + status 0 (proceed)
                resp.extend_from_slice(&make_seed().data);
                client.write(&resp).await?;
                continue;
            }
            other => break (other, buf),
        }
    };

    match handshake {
        HandshakeType::Js5 => {
            client.write(&[0; 8]).await?;
            js5_loop(client).await?;
        }
        h @ (HandshakeType::Login | HandshakeType::Reconnect) => {
            process_login(client, buf, matches!(h, HandshakeType::Reconnect)).await?;
        }
        HandshakeType::New => bail!("session-key exchange continues the loop"),
    }
    drop(permit);
    Ok(())
}

async fn process_login(mut client: Socket, mut buf: Packet, reconnect: bool) -> anyhow::Result<()> {
    let waiting = buf.g1();
    if waiting != buf.len().saturating_sub(buf.pos) as u8 {
        let _ = client.write(&[LoginResponse::Rejected as u8]).await;
        bail!("{} Not enough bytes to read", client.addr);
    }
    let mut version = u16::from(buf.g1()); // the client revision.
    if version == 255 {
        version = buf.g2();
    }
    if version.to_string() != REVISION.split('.').next().unwrap_or(REVISION) {
        let _ = client.write(&[LoginResponse::RuneScapeUpdated as u8]).await;
        bail!("{}: Invalid version: {}", client.addr, version)
    }
    let info = buf.g1();
    let low_memory = (info & 0x1) != 0;
    let mut crc_block = [0; 9 * 4];
    buf.gdata(&mut crc_block, 0, 9 * 4);
    let client_crc = rs_io::crc::getcrc(&crc_block, 0, 9 * 4);
    if client_crc != client.server_io.cache.crc_buffer32 {
        let _ = client.write(&[LoginResponse::RuneScapeUpdated as u8]).await;
        bail!(
            "{}: CRC mismatch: client={} expected={}",
            client.addr,
            client_crc,
            client.server_io.cache.crc_buffer32
        )
    }
    buf.rsadec(RsaFrame::Byte, client.server_io.rsa);
    let magic = buf.g1();
    if magic != 10 {
        let _ = client.write(&[LoginResponse::Rejected as u8]).await;
        bail!("{}: Invalid magic: {}", client.addr, magic)
    }
    let seed = [buf.g4s(), buf.g4s(), buf.g4s(), buf.g4s()];
    let _ = buf.g4s(); // uid
    let username = buf.gjstr(10);
    if username.is_empty() || username.len() > 12 {
        let _ = client
            .write(&[LoginResponse::InvalidCredentials as u8])
            .await;
        bail!("{}: Invalid username: {}", client.addr, username)
    }
    let password = buf.gjstr(10);
    if password.is_empty() || password.len() > 20 {
        let _ = client
            .write(&[LoginResponse::InvalidCredentials as u8])
            .await;
        bail!("{}: Invalid password", client.addr)
    }

    let IsaacPair { encode, decode } = IsaacPair::from_client_seeds(&seed);
    let io = create_io(IsaacPair { encode, decode });
    let ClientIO {
        handle,
        packet_tx,
        bytes_rx,
        recycle_tx,
        disconnect_tx,
    } = io;
    if client
        .server_io
        .new_player_tx
        .send(LoginRequest {
            handle,
            username: Box::from(username),
            password: Box::from(password),
            low_memory,
            remote_addr: client.addr,
            reconnect,
        })
        .is_err()
    {
        bail!("{}: Invalid login request", client.addr)
    }
    network_loop(client, packet_tx, bytes_rx, recycle_tx, &disconnect_tx).await
}
#[cfg(since_244)]
const MAX_QUEUED_REQUESTS: usize = 8192;
#[cfg(since_244)]
const JS5_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(since_244)]
const JS5_SEND_TIMEOUT: Duration = Duration::from_secs(60);

#[cfg(since_244)]
async fn js5_loop(mut client: Socket) -> anyhow::Result<()> {
    let mut queues: [VecDeque<(u8, u16)>; 3] = Default::default();
    let mut partial: Vec<u8> = Vec::new();
    loop {
        if queues.iter().all(|q| q.is_empty()) {
            let Ok(result) = timeout(JS5_IDLE_TIMEOUT, client.read()).await else {
                return Ok(());
            };
            match result? {
                Some(bytes) => enqueue_requests(client.addr, &mut queues, &mut partial, &bytes)?,
                None => return Ok(()), // connection closed
            }
            continue;
        }
        tokio::select! {
            biased;
            result = client.read() => {
                match result? {
                    Some(bytes) => enqueue_requests(client.addr, &mut queues, &mut partial, &bytes)?,
                    None => return Ok(()),
                }
                continue;
            }
            _ = std::future::ready(()) => {}
        }
        if let Some((archive, file)) = queues.iter_mut().find_map(VecDeque::pop_front) {
            let send = send_js5_file(&mut client, archive, file);
            match timeout(JS5_SEND_TIMEOUT, send).await {
                Ok(result) => result?,
                Err(_) => bail!("{}: ondemand send stalled", client.addr),
            }
        }
    }
}

#[cfg(since_244)]
fn enqueue_requests(
    addr: SocketAddr,
    queues: &mut [VecDeque<(u8, u16)>; 3],
    partial: &mut Vec<u8>,
    bytes: &[u8],
) -> anyhow::Result<()> {
    partial.extend_from_slice(bytes);
    let complete = partial.len() - partial.len() % 4;
    for req in partial[..complete].chunks_exact(4) {
        let (archive, priority) = (req[0], req[3]);
        let file = u16::from_be_bytes([req[1], req[2]]);
        let tier = match priority {
            2 => 0,
            1 => 1,
            0 => 2,
            10 => continue,
            _ => bail!("{addr}: invalid ondemand request (archive={archive}, priority={priority})"),
        };
        if archive > 3 {
            bail!("{addr}: invalid ondemand request (archive={archive}, priority={priority})");
        }
        queues[tier].push_back((archive, file));
    }
    partial.drain(..complete);
    if queues.iter().map(|q| q.len()).sum::<usize>() > MAX_QUEUED_REQUESTS {
        bail!("{addr}: ondemand request queue overflow");
    }
    Ok(())
}

#[cfg(since_244)]
async fn send_js5_file(client: &mut Socket, archive: u8, file: u16) -> anyhow::Result<()> {
    let blob: &[u8] = client
        .server_io
        .cache
        .ondemand
        .get(archive as usize)
        .and_then(|files| files.get(file as usize))
        .map(|b| b.as_ref())
        .unwrap_or(&[]);

    let len = blob.len();

    if len == 0 || len > u16::MAX as usize {
        let mut pkt = Packet::new(6);
        pkt.p1(archive);
        pkt.p2(file);
        pkt.p2(0);
        pkt.p1(0);
        client.write(&pkt.data).await?;
        return Ok(());
    }

    let mut pos = 0;
    let mut part = 0;
    while pos < len {
        let chunk = (len - pos).min(500);
        let mut pkt = Packet::new(6 + chunk);
        pkt.p1(archive);
        pkt.p2(file);
        pkt.p2(len as u16);
        pkt.p1(part);
        pkt.pdata(blob, pos, chunk);
        client.write(&pkt.data).await?;
        pos += chunk;
        part = part.wrapping_add(1);
    }
    Ok(())
}

#[allow(clippy::collapsible_match)]
pub async fn network_loop(
    mut client: Socket,
    packet_tx: Sender<Vec<u8>>,
    mut bytes_rx: UnboundedReceiver<Vec<u8>>,
    recycle_tx: UnboundedSender<Vec<u8>>,
    disconnect_tx: &Sender<()>,
) -> anyhow::Result<()> {
    let addr = client.addr;
    loop {
        tokio::select! {
            result = client.read() => match result {
                Ok(Some(bytes)) if !bytes.is_empty() => {
                    if packet_tx.try_send(bytes).is_err() {
                        disconnect_tx.send(()).await?;
                        bail!("{addr} inbox full or closed");
                    }
                }
                Ok(None) | Err(_) => {
                    disconnect_tx.send(()).await?;
                    bail!("{addr} disconnected");
                }
                _ => {}
            },
            msg = bytes_rx.recv() => match msg {
                Some(bytes) => {
                    match tokio::time::timeout(WRITE_STALL_TIMEOUT, client.write_owned(bytes)).await {
                        Ok(result) => {
                            if let Some(returned) = result? {
                                let _ = recycle_tx.send(returned);
                            }
                        }
                        Err(_) => {
                            disconnect_tx.send(()).await?;
                            bail!("{addr} write stalled");
                        }
                    }
                }
                None => {
                    bail!("{addr} engine closed write channel");
                }
            },
        }
    }
}
