use crate::{ConnectionGuard, ConnectionPermit, ServerIO, Socket};
#[cfg(since_647)]
use crate::{REVISION, SocketType};
#[cfg(since_647)]
use anyhow::anyhow;
use anyhow::bail;
use mpsc::{Sender, UnboundedReceiver, UnboundedSender};
use num_enum::TryFromPrimitive;
#[cfg(before_647)]
use rand::RngExt;
use rs_crypto::isaac::IsaacPair;
use rs_engine::{ClientIO, LoginRequest, MAX_UNSENT, create_io};
use rs_io::Packet;
use rs_protocol::LoginResponse;
#[cfg(since_244)]
use rs_protocol::js5::Js5Response;
#[cfg(since_647)]
use rs_protocol::login::{
    CHECK_NAME_MAX_PAYLOAD, CREATE_ACCOUNT_MAX_PAYLOAD, CheckNameRequest, CreateAccountRequest,
};
use rs_protocol::login::{LOGIN_MAX_PAYLOAD, LoginBlock};
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::ClientProt;
#[cfg(since_647)]
use rs_protocol::network::game::client_prot_frame::ClientProtFrame;
#[cfg(since_647)]
use rs_protocol::{CheckEmailResponse, CreateAccountResponse};
#[cfg(since_244)]
use std::collections::VecDeque;
#[cfg(since_244)]
use std::future::ready;
use std::mem::take;
#[cfg(since_244)]
use std::net::SocketAddr;
#[cfg(since_647)]
use std::sync::PoisonError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};
use tracing::{debug, warn};

#[repr(u8)]
#[derive(TryFromPrimitive)]
pub enum HandshakeType {
    #[cfg(since_244)]
    New = 14,
    #[cfg(since_244)]
    Js5 = 15,
    Login = 16,
    Reconnect = 18,
    #[cfg(since_647)]
    Lobby = 19,
    #[cfg(since_647)]
    CreateAccount = 22,
    #[cfg(since_647)]
    CheckName = 28,
}

pub async fn serve(
    host: String,
    port: u16,
    server_state: ServerIO,
    guard: ConnectionGuard,
    js5_guard: ConnectionGuard,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind((host.as_str(), port)).await?;

    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!("TCP accept error: {e}");
                sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        if stream.set_nodelay(true).is_err() {
            continue; // peer already gone
        }
        debug!("TCP connection from {}", addr);
        let server_state = server_state.clone();
        let guard = guard.clone();
        let js5_guard = js5_guard.clone();
        tokio::spawn(async move {
            let connection = Socket::from_tcp(stream, addr, server_state, guard, js5_guard);
            if let Err(e) = handshake(connection).await {
                debug!("TCP connection {} closed: {}", addr, e);
            }
        });
    }
}

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_STALL_TIMEOUT: Duration = Duration::from_secs(60);
#[cfg(before_647)]
fn make_seed() -> Packet {
    let mut seed = Packet::new(8);
    seed.p4(rand::rng().random_range(0..0x00ffffff_u32) as i32);
    seed.p4(rand::rng().random_range(0..0xffffffff_u32) as i32);
    seed
}

/// Holds a place for this connection under its address's limit, logins and
/// cache downloads (`js5`) apart, or sends `refusal` and closes it.
async fn acquire_permit(
    client: &mut Socket,
    js5: bool,
    refusal: &[u8],
) -> anyhow::Result<ConnectionPermit> {
    let guard = if js5 {
        &client.js5_guard
    } else {
        &client.guard
    };
    if let Some(permit) = guard.try_acquire(client.addr.ip()) {
        return Ok(permit);
    }
    if !refusal.is_empty() {
        let _ = client.write(refusal).await;
    }
    bail!("{} connection limit reached", client.addr);
}

/// The reply to a session-key exchange: 8 bytes the client skips, then
/// `status`, 0 to go on with the login or a [`LoginResponse`] code. A code
/// sent any sooner is skipped, and the client shows "Unexpected server
/// response".
#[cfg(all(since_244, before_647))]
const fn key_exchange_reply(status: u8) -> [u8; 9] {
    let mut reply = [0; 9];
    reply[8] = status;
    reply
}

/// Go on with the login.
#[cfg(all(since_244, before_647))]
const PROCEED: u8 = 0;

/// A login turned away for its address's limit.
#[cfg(all(since_244, before_647))]
const LIMIT_REACHED: [u8; 9] = key_exchange_reply(LoginResponse::TooManyConnections as u8);

#[cfg(rev = "225")]
pub async fn handshake(mut client: Socket) -> anyhow::Result<()> {
    client.write(&make_seed().data).await?;
    let permit = acquire_permit(
        &mut client,
        false,
        &[LoginResponse::TooManyConnections as u8],
    )
    .await?;
    let bytes = timeout(HANDSHAKE_TIMEOUT, client.read_exact(1)).await??;
    let result = process_login(client, HandshakeType::try_from(bytes[0])?).await;
    drop(permit);
    result
}

#[cfg(since_244)]
pub async fn handshake(mut client: Socket) -> anyhow::Result<()> {
    let mut permit = None;
    let mut key_exchanges = 0;

    let result = loop {
        let bytes = timeout(HANDSHAKE_TIMEOUT, client.read_exact(1)).await??;
        let kind = HandshakeType::try_from(bytes[0])?;
        if permit.is_none() {
            let refusal: &[u8] = match kind {
                HandshakeType::Js5 => &[],
                #[cfg(before_647)]
                HandshakeType::New => &LIMIT_REACHED,
                _ => &[LoginResponse::TooManyConnections as u8],
            };
            permit = Some(
                acquire_permit(&mut client, matches!(kind, HandshakeType::Js5), refusal).await?,
            );
        }
        match kind {
            HandshakeType::New => {
                key_exchanges += 1;
                if key_exchanges > 3 {
                    bail!("{}: repeated session-key exchanges", client.addr);
                }
                #[cfg(before_647)]
                {
                    timeout(HANDSHAKE_TIMEOUT, client.read_exact(1)).await??;
                    let mut reply = key_exchange_reply(PROCEED).to_vec();
                    reply.extend_from_slice(&make_seed().data);
                    client.write(&reply).await?;
                }
                #[cfg(since_647)]
                client.write(&[0]).await?;
            }
            HandshakeType::Js5 => {
                if key_exchanges > 0 {
                    bail!(
                        "{}: cache request after a session-key exchange",
                        client.addr
                    );
                }
                #[cfg(before_647)]
                client.write(&[0; 8]).await?;
                #[cfg(since_647)]
                {
                    let bytes = timeout(HANDSHAKE_TIMEOUT, client.read_exact(4)).await??;
                    let revision = u32::from_be_bytes(bytes.try_into().unwrap());
                    if revision.to_string() != REVISION {
                        client
                            .write(&[LoginResponse::RuneScapeUpdated as u8])
                            .await?;
                        bail!("{}: js5 wrong revision: {revision}", client.addr);
                    }
                    client.server_io.js5 = client
                        .server_io
                        .js5_latest
                        .read()
                        .unwrap_or_else(PoisonError::into_inner)
                        .clone();
                    let crcs = client.server_io.js5.prefetch_crcs();
                    let mut reply = Packet::new(1 + size_of_val(&crcs));
                    reply.p1(0);
                    for crc in crcs {
                        reply.p4(crc);
                    }
                    client.write(&reply.data).await?;
                }
                break js5_loop(client).await;
            }
            #[cfg(since_647)]
            HandshakeType::CreateAccount => break create_account(&mut client).await,
            #[cfg(since_647)]
            HandshakeType::CheckName => break check_name(&mut client).await,
            kind => {
                #[cfg(since_647)]
                if key_exchanges == 0 {
                    bail!("{}: login before service handshake", client.addr);
                }
                break process_login(client, kind).await;
            }
        }
    };
    drop(permit);
    result
}

async fn read_login_block(client: &mut Socket, max: usize) -> anyhow::Result<Packet> {
    timeout(HANDSHAKE_TIMEOUT, async {
        #[cfg(before_647)]
        let len = client.read_exact(1).await?[0] as usize;
        #[cfg(since_647)]
        let len = {
            let bytes = client.read_exact(2).await?;
            u16::from_be_bytes([bytes[0], bytes[1]]) as usize
        };
        if len > max {
            bail!("{}: oversized login payload: {len}", client.addr);
        }
        Ok(Packet::from(client.read_exact(len).await?))
    })
    .await?
}

async fn process_login(mut client: Socket, kind: HandshakeType) -> anyhow::Result<()> {
    let buf = read_login_block(&mut client, LOGIN_MAX_PAYLOAD).await?;
    let login = LoginBlock::decode(
        buf,
        client.server_io.rsa,
        #[cfg(before_647)]
        client.server_io.cache.read(|cache| cache.crc_buffer32),
        #[cfg(since_647)]
        matches!(kind, HandshakeType::Lobby),
    );
    #[cfg(since_647)]
    let login = login.map_err(|_| LoginResponse::InvalidCredentials);
    let login = match login {
        Ok(login) => login,
        Err(response) => {
            client.write(&[response as u8]).await?;
            bail!("{}: invalid login block", client.addr);
        }
    };
    #[cfg(before_647)]
    if !rs_db::is_typeable_password(&login.password) {
        client
            .write(&[LoginResponse::InvalidCredentials as u8])
            .await?;
        bail!("{}: invalid password", client.addr);
    }

    let isaac = IsaacPair::from_client_seeds(&login.seed);
    #[cfg(since_647)]
    if matches!(kind, HandshakeType::Lobby) {
        client.write(&[LoginResponse::SuccessNormal as u8]).await?;
        send_lobby_response(&mut client, &login.username).await?;
        return lobby_loop(client, isaac).await;
    }

    let ClientIO {
        handle,
        packet_tx,
        bytes_rx,
        recycle_tx,
        disconnect_tx,
        unsent,
    } = create_io(isaac);
    if client
        .server_io
        .new_player_tx
        .send(LoginRequest {
            handle,
            username: login.username.into(),
            password: login.password.into(),
            low_memory: login.low_memory,
            remote_addr: client.addr,
            uid: login.uid,
            reconnect: matches!(kind, HandshakeType::Reconnect),
            device: take(&mut client.device),
            #[cfg(since_647)]
            display_mode: login.settings.window_mode,
        })
        .is_err()
    {
        bail!("{}: engine channel closed", client.addr);
    }
    network_loop(
        client,
        packet_tx,
        bytes_rx,
        recycle_tx,
        &disconnect_tx,
        &unsent,
    )
    .await
}

#[cfg(since_647)]
fn world_id(client: &Socket) -> anyhow::Result<u16> {
    let SocketType::Tcp(stream) = &client.socket_type else {
        bail!("647 lobby requires a TCP connection");
    };
    stream
        .local_addr()?
        .port()
        .checked_sub(40000)
        .filter(|id| (1..=15535).contains(id))
        .ok_or_else(|| anyhow!("invalid 647 world port"))
}

#[cfg(since_647)]
async fn send_lobby_response(client: &mut Socket, display_name: &str) -> anyhow::Result<()> {
    let len = u8::try_from(59 + Packet::sizejstr(display_name))?;
    let mut pkt = Packet::new(1 + len as usize);
    pkt.p1(len);

    pkt.p1(2);
    pkt.p1(0);
    pkt.p1(1);
    pkt.p1(0);
    pkt.p1(0);
    pkt.p4(0);
    pkt.p4(0);
    pkt.p1(0);
    pkt.p4(0);
    pkt.p1(3);
    pkt.p4(0x7f000001u32 as i32);
    pkt.p2(0);
    pkt.p2(0);
    pkt.p2(0);
    pkt.p4(0);
    pkt.p1(3);
    pkt.p2(0xD1DF);
    pkt.p2(0xD1DF);
    pkt.p1(0);
    pkt.p1(0);
    pkt.pjstr(display_name, 0);
    pkt.p1(10);
    pkt.p4(0x0046F489);
    pkt.p1(0);
    pkt.p2(world_id(client)?);
    pkt.p1(0);
    pkt.pjstr("127.0.0.1", 0);

    client.write(&pkt.data[..pkt.pos]).await?;
    client.flush().await?;
    Ok(())
}

#[cfg(since_647)]
async fn lobby_loop(mut client: Socket, mut isaac: IsaacPair) -> anyhow::Result<()> {
    loop {
        let raw = client.read_exact(1).await?[0];
        let opcode = raw.wrapping_sub(isaac.decode.next_int() as u8);

        let size = match ClientProt::try_from(opcode) {
            Ok(prot) => match prot.info().frame {
                ClientProtFrame::VarByte { .. } => client.read_exact(1).await?[0] as usize,
                ClientProtFrame::VarShort { .. } => {
                    let b = client.read_exact(2).await?;
                    ((b[0] as usize) << 8) | b[1] as usize
                }
                ClientProtFrame::Fixed(len) => len as usize,
            },
            Err(_) => bail!("{}: unknown lobby opcode {opcode}", client.addr),
        };
        let _payload = client.read_exact(size).await?;

        match opcode {
            0 => {
                let encoded = 12u8.wrapping_add(isaac.encode.next_int() as u8);
                client.write(&[encoded]).await?;
                client.flush().await?;
            }
            45 => {
                send_world_list(&mut client, &mut isaac).await?;
            }
            _ => {}
        }
    }
}

#[cfg(since_647)]
async fn send_world_list(client: &mut Socket, isaac: &mut IsaacPair) -> anyhow::Result<()> {
    let world_id = world_id(client)? as i32;
    let mut inner = Packet::new(128);

    inner.p1(2);
    inner.p1(1);

    inner.psmart1or2(1);
    inner.psmart1or2(0);
    inner.p1(0);
    inner.pjstr("Otherworld", 0);

    inner.psmart1or2(world_id);
    inner.psmart1or2(world_id);
    inner.psmart1or2(1);

    inner.psmart1or2(0);
    inner.p1(0);
    inner.p4(1);
    inner.p1(0);
    inner.pjstr("", 0);
    inner.p1(0);
    inner.pjstr("127.0.0.1", 0);

    inner.p4(0);

    inner.psmart1or2(0);
    inner.p2(10);

    let mut frame = Packet::new(4 + inner.pos);
    frame.p1(107u8.wrapping_add(isaac.encode.next_int() as u8));
    frame.p2((1 + inner.pos) as u16);
    frame.p1(1);
    frame.pdata(&inner.data, 0, inner.pos);

    client.write(&frame.data[..frame.pos]).await?;
    client.flush().await?;
    Ok(())
}

#[cfg(since_647)]
async fn create_account(client: &mut Socket) -> anyhow::Result<()> {
    let request = async {
        let payload = read_login_block(client, CREATE_ACCOUNT_MAX_PAYLOAD).await?;
        CreateAccountRequest::decode(payload, client.server_io.rsa)
            .map_err(|e| anyhow!("{}: create-account decode failed: {e:?}", client.addr))
    }
    .await;
    client
        .write(&[CreateAccountResponse::ServerOffline as u8])
        .await?;
    request?;
    Ok(())
}

#[cfg(since_647)]
async fn check_name(client: &mut Socket) -> anyhow::Result<()> {
    let request = async {
        let payload = read_login_block(client, CHECK_NAME_MAX_PAYLOAD).await?;
        CheckNameRequest::decode(payload, client.server_io.rsa)
            .map_err(|e| anyhow!("{}: check-name decode failed: {e:?}", client.addr))
    }
    .await;
    let response = if request.is_ok() {
        CheckEmailResponse::ErrorContactingServer
    } else {
        CheckEmailResponse::UnexpectedServerResponse
    };
    client.write(&[response as u8]).await?;
    request?;
    Ok(())
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
    let mut partial = Vec::new();
    #[cfg(since_647)]
    let mut xor = 0;
    loop {
        let bytes = if queues.iter().all(|q| q.is_empty()) {
            let Ok(result) = timeout(JS5_IDLE_TIMEOUT, client.read()).await else {
                return Ok(());
            };
            Some(result?)
        } else {
            tokio::select! {
                biased;
                result = client.read() => Some(result?),
                _ = ready(()) => None,
            }
        };
        if let Some(bytes) = bytes {
            let Some(bytes) = bytes else {
                return Ok(());
            };
            if !enqueue_requests(
                client.addr,
                &mut queues,
                &mut partial,
                &bytes,
                #[cfg(since_647)]
                &mut xor,
            )? {
                return Ok(());
            }
            continue;
        }
        if let Some(priority) = queues.iter().position(|q| !q.is_empty()) {
            let (archive, file) = queues[priority].pop_front().unwrap();
            #[cfg(since_647)]
            let urgent = priority == 0;
            let send = send_js5_file(
                &mut client,
                archive,
                file,
                #[cfg(since_647)]
                urgent,
                #[cfg(since_647)]
                xor,
            );
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
    #[cfg(since_647)] xor: &mut u8,
) -> anyhow::Result<bool> {
    partial.extend_from_slice(bytes);
    let complete = partial.len() & !3;
    for &[a, b, c, d] in partial[..complete].as_chunks::<4>().0 {
        #[cfg(before_647)]
        let (archive, file, priority) = {
            let priority = match d {
                2 => 0,
                1 => 1,
                0 => 2,
                10 => continue,
                _ => bail!("{addr}: invalid ondemand priority {d}"),
            };
            if a > 3 {
                bail!("{addr}: invalid ondemand archive {a}");
            }
            (a, u16::from_be_bytes([b, c]), priority)
        };
        #[cfg(since_647)]
        let (archive, file, priority) = match a {
            0 | 1 => (b, u16::from_be_bytes([c, d]), usize::from(a == 0)),
            2 | 3 | 6 => continue,
            4 => {
                *xor = b;
                continue;
            }
            7 => return Ok(false),
            _ => bail!("{addr}: invalid js5 opcode {a}"),
        };
        queues[priority].push_back((archive, file));
    }
    partial.drain(..complete);
    if queues.iter().map(|q| q.len()).sum::<usize>() > MAX_QUEUED_REQUESTS {
        bail!("{addr}: ondemand request queue overflow");
    }
    Ok(true)
}

#[cfg(since_647)]
async fn send_js5_file(
    client: &mut Socket,
    archive: u8,
    file: u16,
    urgent: bool,
    xor: u8,
) -> anyhow::Result<()> {
    let data = client
        .server_io
        .js5
        .get(archive, file as u32)
        .unwrap_or_else(|| {
            warn!("JS5 missing: archive={archive} group={file}");
            &[0; 5]
        });
    let reply = Js5Response {
        archive,
        group: file,
        data,
    };
    let mut buf = Packet::new(reply.encoded_size());
    reply.encode(&mut buf, urgent);
    if xor != 0 {
        for byte in &mut buf.data[..buf.pos] {
            *byte ^= xor;
        }
    }
    client.write(&buf.data[..buf.pos]).await
}

#[cfg(all(since_244, before_647))]
async fn send_js5_file(client: &mut Socket, archive: u8, file: u16) -> anyhow::Result<()> {
    let buf = client.server_io.cache.read(|cache| {
        let data = cache
            .ondemand
            .get(archive as usize)
            .and_then(|files| files.get(file as usize))
            .map_or(&[][..], |data| data.as_ref());
        let reply = Js5Response {
            archive,
            group: file,
            data,
        };
        let mut buf = Packet::new(reply.encoded_size());
        reply.encode(&mut buf);
        buf
    });
    client.write(&buf.data).await
}

#[allow(clippy::collapsible_match)]
pub async fn network_loop(
    mut client: Socket,
    packet_tx: Sender<(Instant, Vec<u8>)>,
    mut bytes_rx: UnboundedReceiver<Vec<u8>>,
    recycle_tx: UnboundedSender<Vec<u8>>,
    disconnect_tx: &Sender<()>,
    unsent: &AtomicUsize,
) -> anyhow::Result<()> {
    let addr = client.addr;
    loop {
        tokio::select! {
            result = client.read() => match result {
                Ok(Some(bytes)) if !bytes.is_empty() => {
                    if packet_tx.try_send((Instant::now(), bytes)).is_err() {
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
                    if unsent.load(Ordering::Relaxed) > MAX_UNSENT {
                        disconnect_tx.send(()).await?;
                        bail!("{addr} over {MAX_UNSENT} bytes behind writing");
                    }
                    let len = bytes.len();
                    match timeout(WRITE_STALL_TIMEOUT, client.write_owned(bytes)).await {
                        Ok(result) => {
                            unsent.fetch_sub(len, Ordering::Relaxed);
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

#[cfg(all(test, since_244))]
mod tests {
    use super::*;

    #[test]
    fn cache_requests_keep_priority_and_partial_frames() {
        let addr = "127.0.0.1:1".parse().unwrap();
        #[cfg(before_647)]
        let bytes = [
            0, 0, 11, 0, 1, 0, 12, 2, 0, 0, 0, 10, 2, 0, 13, 1, 3, 0, 14, 2,
        ];
        #[cfg(since_647)]
        let bytes = [
            0, 0, 0, 11, 1, 1, 0, 12, 2, 0, 0, 0, 4, 42, 0, 0, 1, 3, 0, 14,
        ];
        for size in 1..=bytes.len() {
            let mut queues = Default::default();
            let mut partial = Vec::new();
            #[cfg(since_647)]
            let mut xor = 0;
            for chunk in bytes.chunks(size) {
                assert!(
                    enqueue_requests(
                        addr,
                        &mut queues,
                        &mut partial,
                        chunk,
                        #[cfg(since_647)]
                        &mut xor,
                    )
                    .unwrap()
                );
            }
            assert!(partial.is_empty());
            assert_eq!(queues[0], [(1, 12), (3, 14)]);
            #[cfg(before_647)]
            {
                assert_eq!(queues[1], [(2, 13)]);
                assert_eq!(queues[2], [(0, 11)]);
            }
            #[cfg(since_647)]
            {
                assert_eq!(queues[1], [(0, 11)]);
                assert!(queues[2].is_empty());
                assert_eq!(xor, 42);
                assert!(
                    !enqueue_requests(addr, &mut queues, &mut partial, &[7, 0, 0, 0], &mut xor)
                        .unwrap()
                );
            }
        }
    }

    #[test]
    fn cache_requests_keep_the_queue_limit_and_reject_invalid_frames() {
        let addr = "127.0.0.1:1".parse().unwrap();
        for bytes in [vec![0; (MAX_QUEUED_REQUESTS + 1) * 4], vec![255; 4]] {
            let mut queues = Default::default();
            let mut partial = Vec::new();
            #[cfg(since_647)]
            let mut xor = 0;
            assert!(
                enqueue_requests(
                    addr,
                    &mut queues,
                    &mut partial,
                    &bytes,
                    #[cfg(since_647)]
                    &mut xor,
                )
                .is_err()
            );
        }
    }
}
