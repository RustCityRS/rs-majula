use crate::{ConnectionGuard, ConnectionPermit, ServerIO, Socket};
#[cfg(since_530)]
use crate::{REVISION, SocketType};
#[cfg(since_530)]
use anyhow::anyhow;
use anyhow::bail;
use mpsc::{Sender, UnboundedReceiver, UnboundedSender};
use num_enum::TryFromPrimitive;
#[cfg(before_647)]
use rand::RngExt;
use rs_crypto::isaac::IsaacPair;
#[cfg(before_647)]
use rs_db::is_typeable_password;
use rs_engine::{ClientIO, LoginRequest, MAX_UNSENT, create_io};
use rs_io::Packet;
#[cfg(all(since_647, before_742))]
use rs_protocol::CheckEmailResponse;
#[cfg(all(since_530, before_742))]
use rs_protocol::CreateAccountResponse;
use rs_protocol::LoginResponse;
#[cfg(since_244)]
use rs_protocol::js5::{Js5Group, Js5Response};
#[cfg(all(since_647, before_742))]
use rs_protocol::login::{CHECK_NAME_MAX_PAYLOAD, CREATE_ACCOUNT_MAX_PAYLOAD};
#[cfg(all(since_530, before_742))]
use rs_protocol::login::{CheckNameRequest, CreateAccountRequest};
#[cfg(all(since_530, before_647))]
use rs_protocol::login::{
    Country, World, WorldList, WorldListRequest, WorldSelectRequest, world_select_reply,
};
use rs_protocol::login::{LOGIN_MAX_PAYLOAD, LoginBlock};
#[cfg(since_244)]
use std::collections::VecDeque;
#[cfg(since_244)]
use std::future::ready;
use std::mem::take;
#[cfg(since_244)]
use std::net::SocketAddr;
#[cfg(since_530)]
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
    InitGameConnection = 14,
    #[cfg(since_244)]
    InitJs5RemoteConnection = 15,
    GameLogin = 16,
    #[cfg(before_742)]
    Reconnect = 18,
    #[cfg(since_647)]
    LobbyLogin = 19,
    #[cfg(all(since_530, before_647))]
    CreateAccount = 36,
    #[cfg(all(since_647, before_742))]
    CreateAccount = 22,
    #[cfg(all(since_530, before_647))]
    CheckName = 186,
    #[cfg(all(since_647, before_742))]
    CheckName = 28,
    #[cfg(all(since_530, before_647))]
    Unknown147 = 147,
    #[cfg(all(since_530, before_647))]
    WorldSelect = 210,
    #[cfg(all(since_530, before_647))]
    WorldList = 255,
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
#[cfg(all(since_244, before_530))]
const fn key_exchange_reply(status: u8) -> [u8; 9] {
    let mut reply = [0; 9];
    reply[8] = status;
    reply
}

/// Go on with the login.
#[cfg(all(since_244, before_647))]
const PROCEED: u8 = 0;

/// A login turned away for its address's limit.
#[cfg(all(since_244, before_530))]
const LIMIT_REACHED: [u8; 9] = key_exchange_reply(LoginResponse::IpLimit as u8);

#[cfg(rev = "225")]
pub async fn handshake(mut client: Socket) -> anyhow::Result<()> {
    client.write(&make_seed().data).await?;
    let permit = acquire_permit(&mut client, false, &[LoginResponse::IpLimit as u8]).await?;
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
                HandshakeType::InitJs5RemoteConnection => &[],
                #[cfg(before_530)]
                HandshakeType::InitGameConnection => &LIMIT_REACHED,
                _ => &[LoginResponse::IpLimit as u8],
            };
            permit = Some(
                acquire_permit(
                    &mut client,
                    matches!(kind, HandshakeType::InitJs5RemoteConnection),
                    refusal,
                )
                .await?,
            );
        }
        match kind {
            HandshakeType::InitGameConnection => {
                key_exchanges += 1;
                if key_exchanges > 3 {
                    bail!("{}: repeated session-key exchanges", client.addr);
                }
                #[cfg(before_647)]
                {
                    timeout(HANDSHAKE_TIMEOUT, client.read_exact(1)).await??;
                    #[cfg(before_530)]
                    let mut reply = key_exchange_reply(PROCEED).to_vec();
                    #[cfg(since_530)]
                    let mut reply = vec![PROCEED];
                    reply.extend_from_slice(&make_seed().data);
                    client.write(&reply).await?;
                }
                #[cfg(since_647)]
                client.write(&[0]).await?;
            }
            HandshakeType::InitJs5RemoteConnection => {
                if key_exchanges > 0 {
                    bail!(
                        "{}: cache request after a session-key exchange",
                        client.addr
                    );
                }
                #[cfg(before_530)]
                client.write(&[0; 8]).await?;
                #[cfg(since_530)]
                {
                    #[cfg(before_742)]
                    let current = {
                        let bytes = timeout(HANDSHAKE_TIMEOUT, client.read_exact(4)).await??;
                        u32::from_be_bytes(bytes.try_into().unwrap()).to_string() == REVISION
                    };
                    #[cfg(since_742)]
                    let current = {
                        let len = timeout(HANDSHAKE_TIMEOUT, client.read_exact(1)).await??[0];
                        let bytes =
                            timeout(HANDSHAKE_TIMEOUT, client.read_exact(len.into())).await??;
                        let mut buf = Packet::from(bytes);
                        buf.len() >= 8 && buf.g4s().to_string() == REVISION && buf.g4s() == 1
                    };
                    if !current {
                        client
                            .write(&[LoginResponse::OutOfDateClientVersion as u8])
                            .await?;
                        bail!("{}: js5 wrong revision", client.addr);
                    }
                    client.server_io.js5 = client
                        .server_io
                        .js5_latest
                        .read()
                        .unwrap_or_else(PoisonError::into_inner)
                        .clone();
                    #[cfg(before_647)]
                    client.write(&[0]).await?;
                    #[cfg(since_647)]
                    {
                        let prefetches = client.server_io.js5.prefetches();
                        let mut reply = Packet::new(1 + size_of_val(&prefetches));
                        reply.p1(0);
                        for size in prefetches {
                            reply.p4(size);
                        }
                        client.write(&reply.data).await?;
                    }
                }
                break js5_loop(client).await;
            }
            #[cfg(all(since_530, before_742))]
            HandshakeType::CreateAccount => break create_account(&mut client).await,
            #[cfg(all(since_530, before_742))]
            HandshakeType::CheckName => break check_name(&mut client).await,
            #[cfg(all(since_530, before_647))]
            HandshakeType::Unknown147 => break unknown147(&mut client).await,
            #[cfg(all(since_530, before_647))]
            HandshakeType::WorldSelect => break world_select(&mut client).await,
            #[cfg(all(since_530, before_647))]
            HandshakeType::WorldList => break world_list(&mut client).await,
            kind => {
                #[cfg(since_530)]
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
        #[cfg(before_530)]
        let len = client.read_exact(1).await?[0] as usize;
        #[cfg(since_530)]
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
        #[cfg(before_530)]
        client.server_io.cache.read(|cache| cache.crc_buffer32),
        #[cfg(since_647)]
        matches!(kind, HandshakeType::LobbyLogin),
    );
    #[cfg(since_647)]
    let login = login.map_err(|_| LoginResponse::BadCredentials);
    let login = match login {
        Ok(login) => login,
        Err(response) => {
            client.write(&[response as u8]).await?;
            bail!("{}: invalid login block", client.addr);
        }
    };
    #[cfg(before_647)]
    if !is_typeable_password(&login.password) {
        client.write(&[LoginResponse::BadCredentials as u8]).await?;
        bail!("{}: invalid password", client.addr);
    }

    let isaac = IsaacPair::from_client_seeds(&login.seed);
    #[cfg(since_647)]
    let lobby = matches!(kind, HandshakeType::LobbyLogin);
    #[cfg(since_647)]
    let world = world_id(&client).or_else(|e| if lobby { Err(e) } else { Ok(0) })?;

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
            #[cfg(before_742)]
            reconnect: matches!(kind, HandshakeType::Reconnect),
            #[cfg(since_742)]
            reconnect: login.reconnect,
            device: take(&mut client.device),
            #[cfg(since_530)]
            display_mode: login.settings.window_mode,
            #[cfg(since_647)]
            lobby,
            #[cfg(since_647)]
            world,
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

#[cfg(since_530)]
fn world_id(client: &Socket) -> anyhow::Result<u16> {
    let SocketType::Tcp(stream) = &client.socket_type else {
        bail!("a world id needs a TCP connection");
    };
    stream
        .local_addr()?
        .port()
        .checked_sub(40000)
        .filter(|id| (1..=15535).contains(id))
        .ok_or_else(|| anyhow!("invalid world port"))
}

#[cfg(all(since_530, before_647))]
async fn read_service_block(client: &mut Socket) -> anyhow::Result<Packet> {
    timeout(HANDSHAKE_TIMEOUT, async {
        let len = client.read_exact(1).await?[0] as usize;
        client.read_exact(len).await.map(Packet::from)
    })
    .await?
}

#[cfg(all(since_530, before_742))]
async fn create_account(client: &mut Socket) -> anyhow::Result<()> {
    let request = async {
        #[cfg(before_647)]
        let payload = read_service_block(client).await?;
        #[cfg(since_647)]
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

#[cfg(all(since_530, before_742))]
async fn check_name(client: &mut Socket) -> anyhow::Result<()> {
    let request = async {
        #[cfg(before_647)]
        let request = {
            let bytes = timeout(HANDSHAKE_TIMEOUT, client.read_exact(8)).await??;
            CheckNameRequest::decode(Packet::from(bytes))
        };
        #[cfg(since_647)]
        let request = {
            let payload = read_login_block(client, CHECK_NAME_MAX_PAYLOAD).await?;
            CheckNameRequest::decode(payload, client.server_io.rsa)
        };
        request.map_err(|e| anyhow!("{}: check-name decode failed: {e:?}", client.addr))
    }
    .await;
    #[cfg(before_647)]
    let response = if request.is_ok() {
        CreateAccountResponse::ServerOffline
    } else {
        CreateAccountResponse::NameInvalid
    };
    #[cfg(since_647)]
    let response = if request.is_ok() {
        CheckEmailResponse::ErrorContactingServer
    } else {
        CheckEmailResponse::UnexpectedServerResponse
    };
    client.write(&[response as u8]).await?;
    request?;
    Ok(())
}

/// The date of birth and country check before an account is made, refused as
/// the other account services are.
#[cfg(all(since_530, before_647))]
async fn unknown147(client: &mut Socket) -> anyhow::Result<()> {
    timeout(HANDSHAKE_TIMEOUT, client.read_exact(6)).await??;
    client
        .write(&[CreateAccountResponse::ServerOffline as u8])
        .await
}

#[cfg(all(since_530, before_647))]
async fn world_select(client: &mut Socket) -> anyhow::Result<()> {
    let payload = read_service_block(client).await?;
    let request = match WorldSelectRequest::decode(payload, client.server_io.rsa) {
        Ok(request) => request,
        Err(response) => {
            client.write(&[response as u8]).await?;
            bail!("{}: invalid world-select block", client.addr);
        }
    };
    if !is_typeable_password(&request.password) {
        client.write(&[LoginResponse::BadCredentials as u8]).await?;
        bail!("{}: invalid password", client.addr);
    }
    client.write(&world_select_reply(world_id(client)?)).await
}

/// This world alone, at the address the client reached it on: the client
/// logs in to the host and world the list gives.
#[cfg(all(since_530, before_647))]
async fn world_list(client: &mut Socket) -> anyhow::Result<()> {
    let bytes = timeout(HANDSHAKE_TIMEOUT, client.read_exact(4)).await??;
    let request = WorldListRequest::decode(Packet::from(bytes))
        .map_err(|e| anyhow!("{}: world-list decode failed: {e:?}", client.addr))?;
    let SocketType::Tcp(stream) = &client.socket_type else {
        bail!("a world list needs a TCP connection");
    };
    let host = stream.local_addr()?.ip().to_canonical().to_string();
    let id = world_id(client)?;
    let list = WorldList {
        countries: &[Country {
            flag: 0,
            name: "Otherworld",
        }],
        worlds: &[World {
            id,
            country: 0,
            flags: 1,
            activity: "",
            host: &host,
            players: client.server_io.tick_stats.borrow().player_count as u16,
        }],
        checksum: i32::from(id),
    };
    client
        .write(&list.encode(request.checksum != list.checksum))
        .await
}

#[cfg(since_244)]
const MAX_QUEUED_REQUESTS: usize = 8192;
#[cfg(since_244)]
const JS5_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(since_244)]
const JS5_SEND_TIMEOUT: Duration = Duration::from_secs(60);
#[cfg(all(since_244, before_742))]
const REQUEST_LEN: usize = 4;
#[cfg(since_742)]
const REQUEST_LEN: usize = 6;

#[cfg(since_244)]
async fn js5_loop(mut client: Socket) -> anyhow::Result<()> {
    let mut queues: [VecDeque<(u8, Js5Group)>; 3] = Default::default();
    let mut partial = Vec::new();
    #[cfg(since_530)]
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
                #[cfg(since_530)]
                &mut xor,
            )? {
                return Ok(());
            }
            continue;
        }
        if let Some(priority) = queues.iter().position(|q| !q.is_empty()) {
            let (archive, file) = queues[priority].pop_front().unwrap();
            #[cfg(since_530)]
            let urgent = priority == 0;
            let send = send_js5_file(
                &mut client,
                archive,
                file,
                #[cfg(since_530)]
                urgent,
                #[cfg(since_530)]
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
    queues: &mut [VecDeque<(u8, Js5Group)>; 3],
    partial: &mut Vec<u8>,
    bytes: &[u8],
    #[cfg(since_530)] xor: &mut u8,
) -> anyhow::Result<bool> {
    partial.extend_from_slice(bytes);
    let complete = partial.len() - partial.len() % REQUEST_LEN;
    for request in partial[..complete].as_chunks::<REQUEST_LEN>().0 {
        let (a, b) = (request[0], request[1]);
        #[cfg(before_530)]
        let (archive, file, priority) = {
            let (c, d) = (request[2], request[3]);
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
        #[cfg(since_530)]
        let (archive, file, priority) = match a {
            0 | 1 => (
                b,
                Js5Group::from_be_bytes(request[2..].try_into().unwrap()),
                usize::from(a == 0),
            ),
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

#[cfg(since_530)]
async fn send_js5_file(
    client: &mut Socket,
    archive: u8,
    file: Js5Group,
    urgent: bool,
    xor: u8,
) -> anyhow::Result<()> {
    let js5 = &client.server_io.js5;
    let data = js5.get(archive, file).unwrap_or_else(|| {
        if !js5.is_absent(archive, file) {
            warn!("JS5 missing: archive={archive} group={file}");
        }
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

#[cfg(all(since_244, before_530))]
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
        #[cfg(before_530)]
        let bytes = [
            0, 0, 11, 0, 1, 0, 12, 2, 0, 0, 0, 10, 2, 0, 13, 1, 3, 0, 14, 2,
        ];
        #[cfg(all(since_530, before_742))]
        let bytes = [
            0, 0, 0, 11, 1, 1, 0, 12, 2, 0, 0, 0, 4, 42, 0, 0, 1, 3, 0, 14,
        ];
        #[cfg(since_742)]
        let bytes = [
            0, 0, 0, 0, 0, 11, 1, 1, 0, 0, 0, 12, 2, 0, 0, 0, 0, 0, 4, 42, 0, 0, 0, 0, 1, 3, 0, 0,
            0, 14,
        ];
        for size in 1..=bytes.len() {
            let mut queues = Default::default();
            let mut partial = Vec::new();
            #[cfg(since_530)]
            let mut xor = 0;
            for chunk in bytes.chunks(size) {
                assert!(
                    enqueue_requests(
                        addr,
                        &mut queues,
                        &mut partial,
                        chunk,
                        #[cfg(since_530)]
                        &mut xor,
                    )
                    .unwrap()
                );
            }
            assert!(partial.is_empty());
            assert_eq!(queues[0], [(1, 12), (3, 14)]);
            #[cfg(before_530)]
            {
                assert_eq!(queues[1], [(2, 13)]);
                assert_eq!(queues[2], [(0, 11)]);
            }
            #[cfg(since_530)]
            {
                assert_eq!(queues[1], [(0, 11)]);
                assert!(queues[2].is_empty());
                assert_eq!(xor, 42);
                assert!(
                    !enqueue_requests(addr, &mut queues, &mut partial, &[7; REQUEST_LEN], &mut xor)
                        .unwrap()
                );
            }
        }
    }

    #[test]
    fn cache_requests_keep_the_queue_limit_and_reject_invalid_frames() {
        let addr = "127.0.0.1:1".parse().unwrap();
        for bytes in [
            vec![0; (MAX_QUEUED_REQUESTS + 1) * REQUEST_LEN],
            vec![255; REQUEST_LEN],
        ] {
            let mut queues = Default::default();
            let mut partial = Vec::new();
            #[cfg(since_530)]
            let mut xor = 0;
            assert!(
                enqueue_requests(
                    addr,
                    &mut queues,
                    &mut partial,
                    &bytes,
                    #[cfg(since_530)]
                    &mut xor,
                )
                .is_err()
            );
        }
    }
}
