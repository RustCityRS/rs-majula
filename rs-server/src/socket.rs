use crate::ConnectionPermit;
#[cfg(before_647)]
use crate::REVISION;
use crate::{ConnectionGuard, ServerIO, Socket};
use anyhow::bail;
use mpsc::{Sender, UnboundedReceiver, UnboundedSender};
#[cfg(before_647)]
use num_enum::TryFromPrimitive;
#[cfg(before_647)]
use rand::RngExt;
use rs_crypto::isaac::IsaacPair;
use rs_engine::LoginRequest;
use rs_engine::{ClientIO, MAX_UNSENT, create_io};
use rs_io::Packet;
#[cfg(before_647)]
use rs_io::packet::RsaFrame;
#[cfg(all(since_244, before_647))]
use rs_pack::cache::CacheStore;
use rs_protocol::LoginResponse;
#[cfg(since_647)]
use rs_protocol::js5::Js5Response;
#[cfg(since_647)]
use rs_protocol::login::{
    CHECK_NAME_MAX_PAYLOAD, CREATE_ACCOUNT_MAX_PAYLOAD, LOGIN_MAX_PAYLOAD, LoginDecoder, LoginType,
};
#[cfg(since_647)]
use rs_protocol::{CheckEmailResponse, CreateAccountResponse, ServiceOpcode};
#[cfg(all(since_244, before_647))]
use std::collections::VecDeque;
#[cfg(all(since_244, before_647))]
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
#[cfg(before_647)]
use tokio::time::timeout;
#[cfg(since_647)]
use tracing::info;
use tracing::{debug, warn};

#[cfg(before_647)]
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
    js5_guard: ConnectionGuard,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind((host.as_str(), port)).await?;

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

#[cfg(before_647)]
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
    // The seed is out, so the client reads this as its login's reply.
    let permit = acquire_permit(
        &mut client,
        false,
        &[LoginResponse::TooManyConnections as u8],
    )
    .await?;
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

#[cfg(all(since_244, before_647))]
pub async fn handshake(mut client: Socket) -> anyhow::Result<()> {
    // Taken once the first message says which limit the connection counts
    // against. Until then the handshake timeout bounds it.
    let mut permit = None;
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
        let kind = HandshakeType::try_from(buf.g1())?;
        if permit.is_none() {
            permit = Some(match kind {
                HandshakeType::Js5 => acquire_permit(&mut client, true, &[]).await?,
                HandshakeType::New => acquire_permit(&mut client, false, &LIMIT_REACHED).await?,
                _ => {
                    let refusal = [LoginResponse::TooManyConnections as u8];
                    acquire_permit(&mut client, false, &refusal).await?
                }
            });
        }
        match kind {
            HandshakeType::New => {
                // The client requests one seed per login attempt; a stream of
                // them is someone keeping the loop (and its permit) spinning.
                key_exchanges += 1;
                if key_exchanges > 3 {
                    bail!("{}: repeated session-key exchanges", client.addr);
                }
                // Its second byte, the login server node, isn't used.
                let mut resp = key_exchange_reply(PROCEED).to_vec();
                resp.extend_from_slice(&make_seed().data);
                client.write(&resp).await?;
                continue;
            }
            HandshakeType::Js5 if key_exchanges > 0 => {
                bail!(
                    "{}: cache request after a session-key exchange",
                    client.addr
                )
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

/// What the web client sends as its uid until a deploy patches it (see
/// `deploy/patch.py`): every browser the same, so it identifies nobody.
#[cfg(before_647)]
const UNPATCHED_UID: i32 = 1337;

/// The login block's uid, if it can tell a client install apart: the
/// desktop client's `uid.dat`, or the patched web client's per-browser id.
#[cfg(before_647)]
fn client_uid(uid: i32) -> Option<i32> {
    (uid != 0 && uid != UNPATCHED_UID).then_some(uid)
}
/// Rev-647 service dispatcher: one service byte selects JS5 (15), login (14),
/// create-account (22) or check-name (28). The cache is delivered in-band over
/// JS5 — there is no JAGGRAB or HTTP cache bootstrap in this era.
#[cfg(since_647)]
pub async fn handshake(client: Socket) -> anyhow::Result<()> {
    let mut client = BufferedSocket::new(client);
    let service = client.read_exact(1).await?[0];
    match ServiceOpcode::try_from(service)? {
        // Each held under its address's limit, as before 647. A 647 client
        // reads a refused login's code at once, with no key exchange first.
        ServiceOpcode::Js5 => {
            let permit = acquire_permit(&mut client.socket, true, &[]).await?;
            let result = js5_loop(&mut client).await;
            drop(permit);
            result
        }
        ServiceOpcode::GameLogin => {
            let permit = acquire_permit(
                &mut client.socket,
                false,
                &[LoginResponse::TooManyConnections as u8],
            )
            .await?;
            let result = process_login(client).await;
            drop(permit);
            result
        }
        ServiceOpcode::CreateAccount => create_account(&mut client).await,
        ServiceOpcode::CheckName => check_name(&mut client).await,
    }
}

/// Buffered reads over `Socket` — the rev-647 handshakes read fixed-size
/// headers and length-prefixed payloads, but `Socket::read` returns whatever
/// chunk the transport delivered (a TCP segment or a WebSocket frame).
#[cfg(since_647)]
struct BufferedSocket {
    socket: Socket,
    buf: Vec<u8>,
    pos: usize,
}

#[cfg(since_647)]
impl BufferedSocket {
    fn new(socket: Socket) -> Self {
        Self {
            socket,
            buf: Vec::new(),
            pos: 0,
        }
    }

    /// Read exactly `n` bytes, awaiting more transport chunks as needed.
    async fn read_exact(&mut self, n: usize) -> anyhow::Result<Vec<u8>> {
        while self.buf.len() - self.pos < n {
            if self.pos > 0 {
                self.buf.drain(..self.pos);
                self.pos = 0;
            }
            match self.socket.read().await? {
                Some(bytes) => self.buf.extend_from_slice(&bytes),
                None => bail!("{}: connection closed", self.socket.addr),
            }
        }
        let out = self.buf[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(out)
    }

    /// Any bytes buffered past the last `read_exact` — game packets the
    /// client pipelined behind its login block. Handed to the engine's
    /// packet channel before the byte pump takes over the raw socket.
    fn take_residual(&mut self) -> Vec<u8> {
        let out = self.buf[self.pos..].to_vec();
        self.buf.clear();
        self.pos = 0;
        out
    }
}

/// Serve cache groups over JS5 until the client disconnects.
#[cfg(since_647)]
async fn js5_loop(client: &mut BufferedSocket) -> anyhow::Result<()> {
    let mut rev_pkt = Packet::from(client.read_exact(4).await?);
    rev_pkt.pos = 2; // skip first two bytes
    let client_rev = rev_pkt.g2();
    if client_rev != 647 {
        client.socket.write(&[6]).await?;
        bail!("{}: js5 wrong revision: {}", client.socket.addr, client_rev);
    }
    client.socket.write(&[0]).await?;
    let crcs = client.socket.server_io.js5.prefetch_crcs();
    let mut crc_pkt = Packet::new(crcs.len() * 4);
    for crc in &crcs {
        crc_pkt.p4(*crc);
    }
    client.socket.write(&crc_pkt.data[..crc_pkt.pos]).await?;
    client.socket.flush().await?;

    loop {
        let req = client.read_exact(4).await?;
        match req[0] {
            // 2/3 = status, 4 = xor rekey, 6 = connect notice — no reply
            2 | 3 | 4 | 6 => continue,
            7 => return Ok(()), // disconnect
            _ => {}
        }
        let urgent = req[0] == 1;
        let mut req_pkt = Packet::from(req);
        req_pkt.pos = 1; // skip opcode byte
        let archive = req_pkt.g1();
        let group = req_pkt.g2() as u32;

        let data = match client.socket.server_io.js5.get(archive, group) {
            Some(data) => data,
            None => {
                warn!("JS5 missing: archive={} group={}", archive, group);
                // 5-byte empty container: [comp=0][size=0]
                bytes::Bytes::from_static(&[0u8, 0, 0, 0, 0])
            }
        };
        let resp = Js5Response::new(archive, group, data);
        let mut pkt = Packet::new(resp.encoded_size());
        resp.encode(&mut pkt, urgent);
        client.socket.write(&pkt.data[..pkt.pos]).await?;
        client.socket.flush().await?;
    }
}

/// Rev-647 login: `[0]` handshake OK, then `[type:1][len:2][payload]` holding
/// the RSA + XTEA login block. Lobby logins get a lobby response; game logins
/// are refused until the 647 world runtime lands.
#[cfg(since_647)]
async fn process_login(mut client: BufferedSocket) -> anyhow::Result<()> {
    client.socket.write(&[0]).await?;
    client.socket.flush().await?;

    let mut full = client.read_exact(3).await?;
    let payload_len = u16::from_be_bytes([full[1], full[2]]) as usize;
    if payload_len > LOGIN_MAX_PAYLOAD {
        bail!(
            "{}: oversized login payload: {}",
            client.socket.addr,
            payload_len
        );
    }
    full.extend_from_slice(&client.read_exact(payload_len).await?);

    let decoder = LoginDecoder::new(client.socket.server_io.rsa);
    let login_req = match decoder.decode(&full) {
        Ok(req) => req,
        Err(e) => {
            let _ = client
                .socket
                .write(&[LoginResponse::InvalidCredentials as u8])
                .await;
            bail!("{}: login decode failed: {}", client.socket.addr, e);
        }
    };
    info!(
        "{} login: user={} type={:?}",
        client.socket.addr, login_req.username, login_req.login_type
    );

    match login_req.login_type {
        LoginType::Lobby => {
            client
                .socket
                .write(&[LoginResponse::SuccessNormal as u8])
                .await?;
            send_lobby_response(&mut client.socket, &login_req.username).await?;
            let isaac = IsaacPair::from_client_seeds(&login_req.isaac_seeds);
            lobby_loop(client, isaac).await
        }
        LoginType::New | LoginType::Reconnect => {
            let isaac = IsaacPair::from_client_seeds(&login_req.isaac_seeds);
            let ClientIO {
                handle,
                packet_tx,
                bytes_rx,
                recycle_tx,
                disconnect_tx,
                unsent,
            } = create_io(isaac);

            let (pid_tx, pid_rx) = tokio::sync::oneshot::channel();
            let engine_login = LoginRequest {
                handle,
                username: Box::from(login_req.username.as_str()),
                password: Box::from(login_req.password.as_str()),
                low_memory: false,
                remote_addr: client.socket.addr,
                reconnect: matches!(login_req.login_type, LoginType::Reconnect),
                // The 647 login block carries no install id the server reads,
                // and the desktop client says nothing of its device.
                uid: None,
                device: rs_db::LoginDevice::default(),
                pid_tx,
                display_mode: login_req.settings.window_mode,
            };
            if client
                .socket
                .server_io
                .new_player_tx
                .send(engine_login)
                .is_err()
            {
                bail!("{}: engine channel closed", client.socket.addr);
            }

            let pid = match pid_rx.await {
                Ok(Ok(pid)) => pid,
                Ok(Err(reject_code)) => {
                    client.socket.write(&[reject_code]).await?;
                    client.socket.flush().await?;
                    bail!(
                        "{}: game login rejected (code {reject_code})",
                        client.socket.addr
                    );
                }
                Err(_) => {
                    let _ = client
                        .socket
                        .write(&[LoginResponse::CouldNotComplete as u8])
                        .await;
                    bail!("{}: engine dropped login request", client.socket.addr);
                }
            };

            client
                .socket
                .write(&[LoginResponse::SuccessNormal as u8])
                .await?;
            send_game_login_response(&mut client.socket, &login_req.username, pid).await?;

            // Game packets the client pipelined behind the login block.
            let residual = client.take_residual();
            if !residual.is_empty() {
                let _ = packet_tx.try_send((std::time::Instant::now(), residual));
            }
            network_loop(
                client.socket,
                packet_tx,
                bytes_rx,
                recycle_tx,
                &disconnect_tx,
                &unsent,
            )
            .await
        }
    }
}

/// Game login response payload — `[len][rights][…][pid:u16][…][username]`,
/// written after the `[2]` success byte. NOT ISAAC-encrypted.
#[cfg(since_647)]
async fn send_game_login_response(
    client: &mut Socket,
    username: &str,
    pid: u16,
) -> anyhow::Result<()> {
    let mut pkt = Packet::new(100);

    let len_pos = pkt.pos;
    pkt.p1(0); // length placeholder — back-patched below
    let payload_start = pkt.pos;

    pkt.p1(2); // rights (2 = admin)
    pkt.p1(0);
    pkt.p1(0);
    pkt.p1(0);
    pkt.p1(1);
    pkt.p1(0);
    pkt.p2(pid); // u16 BE: player index
    pkt.p1(1);
    pkt.p3(0); // u24 medium int
    pkt.p1(1);
    pkt.p1(0);
    pkt.pjstr(username, 0);

    let payload_len = pkt.pos - payload_start;
    pkt.data[len_pos] = payload_len as u8;

    client.write(&pkt.data[..pkt.pos]).await?;
    client.flush().await?;
    Ok(())
}

/// Lobby login response payload — the field layout the rev-647 client reads
/// after the `[2]` success byte.
#[cfg(since_647)]
async fn send_lobby_response(client: &mut Socket, display_name: &str) -> anyhow::Result<()> {
    let mut pkt = Packet::new(100);

    let len_pos = pkt.pos;
    pkt.p1(0); // length placeholder — back-patched below
    let payload_start = pkt.pos;

    pkt.p1(2); // rights
    pkt.p1(0); // mod flag
    pkt.p1(1); // member
    pkt.p1(0); // flag
    pkt.p1(0); // flag
    pkt.p4(0); // server time hi (i32)
    pkt.p4(0); // server time lo (i32)
    pkt.p1(0); // time offset hi
    pkt.p4(0); // time offset lo (i32)
    pkt.p1(3); // flags
    pkt.p4(0x7f000001u32 as i32); // client IP
    pkt.p2(0); // recovery question days
    pkt.p2(0);
    pkt.p2(0); // last login days
    pkt.p4(0);
    pkt.p1(3); // email registration status
    pkt.p2(0xD1DF);
    pkt.p2(0xD1DF);
    pkt.p1(0); // flag
    pkt.p1(0);
    pkt.pjstr(display_name, 0); // gjstr2 display name
    pkt.p1(10);
    pkt.p4(0x0046F489);
    // TODO(647): name_required stays 0 until email accounts land with the
    // world-runtime milestone.
    pkt.p1(0);
    pkt.p2(1); // world id
    pkt.p1(0);
    pkt.pjstr("127.0.0.1", 0); // game server address

    let payload_len = pkt.pos - payload_start;
    pkt.data[len_pos] = payload_len as u8;

    client.write(&pkt.data[..pkt.pos]).await?;
    client.flush().await?;
    Ok(())
}

/// Post-login lobby packet loop: ISAAC-framed client packets, answering
/// keepalive pings (0 → PING_REPLY 12) and world-list requests (45 →
/// WORLDLIST 107) so the PLAY button can resolve the game world. Other
/// lobby packets have their payloads consumed and dropped.
/// TODO(647): social ops (friends/ignores/private chat) land with the
/// ether milestone.
#[cfg(since_647)]
async fn lobby_loop(mut client: BufferedSocket, mut isaac: IsaacPair) -> anyhow::Result<()> {
    use rs_protocol::network::game::client_prot::ClientProt;
    use rs_protocol::network::game::client_prot_frame::ClientProtFrame;

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
            Err(_) => {
                warn!("{}: unknown lobby opcode {opcode}", client.socket.addr);
                0
            }
        };
        let _payload = client.read_exact(size).await?;

        match opcode {
            0 => {
                // NoTimeout keepalive → PING_REPLY.
                let encoded = 12u8.wrapping_add(isaac.encode.next_int() as u8);
                client.socket.write(&[encoded]).await?;
                client.socket.flush().await?;
            }
            45 => {
                send_world_list(&mut client.socket, &mut isaac).await?;
            }
            _ => {}
        }
    }
}

/// World-list response (opcode 107, VAR_SHORT): a single world entry
/// pointing the client back at this host, so PLAY connects to the game
/// service on the same address.
#[cfg(since_647)]
async fn send_world_list(client: &mut Socket, isaac: &mut IsaacPair) -> anyhow::Result<()> {
    let mut inner = Packet::new(128);

    inner.p1(2);
    inner.p1(1); // include_defs = true

    inner.psmart1or2(1);
    inner.psmart1or2(0);
    inner.p1(0);
    inner.pjstr("Otherworld", 0); // gjstr2: version_byte=0, string, null

    inner.psmart1or2(1); // min_world
    inner.psmart1or2(1); // max_world
    inner.psmart1or2(1); // total_worlds

    inner.psmart1or2(0); // world_index
    inner.p1(0); // language
    inner.p4(1); // flags
    inner.p1(0);
    inner.pjstr("", 0); // activity (gjstr2)
    inner.p1(0);
    inner.pjstr("127.0.0.1", 0); // address (gjstr2)

    inner.p4(0); // trailing hash

    // Player counts
    inner.psmart1or2(0);
    inner.p2(10);

    let mut frame = Packet::new(4 + inner.pos);
    frame.p1(107u8.wrapping_add(isaac.encode.next_int() as u8));
    frame.p2((1 + inner.pos) as u16);
    frame.p1(1); // full_update
    frame.pdata(&inner.data, 0, inner.pos);

    client.write(&frame.data[..frame.pos]).await?;
    client.flush().await?;
    Ok(())
}

/// Create-account service (22): decode the request, then report the account
/// service as offline until persistence lands.
#[cfg(since_647)]
async fn create_account(client: &mut BufferedSocket) -> anyhow::Result<()> {
    let hdr = client.read_exact(2).await?;
    let payload_len = u16::from_be_bytes([hdr[0], hdr[1]]) as usize;
    if payload_len > CREATE_ACCOUNT_MAX_PAYLOAD {
        client
            .socket
            .write(&[CreateAccountResponse::ServerOffline as u8])
            .await?;
        bail!(
            "{}: oversized create-account payload: {}",
            client.socket.addr,
            payload_len
        );
    }
    let payload = client.read_exact(payload_len).await?;
    let decoder = LoginDecoder::new(client.socket.server_io.rsa);
    if let Err(e) = decoder.decode_create_account(&payload) {
        client
            .socket
            .write(&[CreateAccountResponse::ServerOffline as u8])
            .await?;
        bail!(
            "{}: create-account decode failed: {}",
            client.socket.addr,
            e
        );
    }
    // TODO(647): account persistence lands with the world-runtime milestone.
    client
        .socket
        .write(&[CreateAccountResponse::ServerOffline as u8])
        .await?;
    Ok(())
}

/// Check-name service (28): decode the request, then report the lookup as
/// unavailable until persistence lands.
#[cfg(since_647)]
async fn check_name(client: &mut BufferedSocket) -> anyhow::Result<()> {
    let hdr = client.read_exact(2).await?;
    let payload_len = u16::from_be_bytes([hdr[0], hdr[1]]) as usize;
    if payload_len > CHECK_NAME_MAX_PAYLOAD {
        client
            .socket
            .write(&[CheckEmailResponse::UnexpectedServerResponse as u8])
            .await?;
        bail!(
            "{}: oversized check-name payload: {}",
            client.socket.addr,
            payload_len
        );
    }
    let payload = client.read_exact(payload_len).await?;
    let decoder = LoginDecoder::new(client.socket.server_io.rsa);
    if let Err(e) = decoder.decode_check_name(&payload) {
        client
            .socket
            .write(&[CheckEmailResponse::UnexpectedServerResponse as u8])
            .await?;
        bail!("{}: check-name decode failed: {}", client.socket.addr, e);
    }
    // TODO(647): name-availability lookup lands with the world-runtime
    // milestone.
    client
        .socket
        .write(&[CheckEmailResponse::ErrorContactingServer as u8])
        .await?;
    Ok(())
}

#[cfg(before_647)]
async fn process_login(mut client: Socket, mut buf: Packet, reconnect: bool) -> anyhow::Result<()> {
    // `Packet` reads don't check bounds, so every length is checked before
    // anything is read: past the end is UB, not a panic.
    if buf.pos >= buf.len() || usize::from(buf.g1()) != buf.len() - buf.pos {
        let _ = client.write(&[LoginResponse::Rejected as u8]).await;
        bail!("{} Not enough bytes to read", client.addr);
    }
    // The version (1 byte, or 255 and 2), the info byte, the CRCs and the
    // RSA block's length byte.
    let head = if buf.data.get(buf.pos) == Some(&255) {
        3
    } else {
        1
    } + 1
        + 9 * 4
        + 1;
    if buf.len() - buf.pos < head {
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
    let expected = client.server_io.cache.read(|cache| cache.crc_buffer32);
    if client_crc != expected {
        let _ = client.write(&[LoginResponse::RuneScapeUpdated as u8]).await;
        bail!(
            "{}: CRC mismatch: client={} expected={}",
            client.addr,
            client_crc,
            expected
        )
    }
    if usize::from(buf.data[buf.pos]) > buf.len() - buf.pos - 1 {
        let _ = client.write(&[LoginResponse::Rejected as u8]).await;
        bail!("{} Not enough bytes to read", client.addr);
    }
    // The plaintext is written over the start of the buffer, and it can be
    // as long as the modulus however short the ciphertext was.
    let modulus = client.server_io.rsa.n.bits().div_ceil(8) as usize;
    if buf.data.len() < modulus {
        buf.data.resize(modulus, 0);
    }
    buf.rsadec(RsaFrame::Byte, client.server_io.rsa);
    // Two terminators after the plaintext stop both strings inside the buffer.
    buf.data.extend_from_slice(b"\n\n");
    let magic = buf.g1();
    // Most likely a client from before a deploy changed the login key: it
    // needs reloading.
    if magic != 10 {
        let _ = client.write(&[LoginResponse::RuneScapeUpdated as u8]).await;
        bail!("{}: Invalid magic: {}", client.addr, magic)
    }
    let seed = [buf.g4s(), buf.g4s(), buf.g4s(), buf.g4s()];
    let uid = client_uid(buf.g4s());
    let username = buf.gjstr(10);
    if username.is_empty() || username.len() > 12 {
        let _ = client
            .write(&[LoginResponse::InvalidCredentials as u8])
            .await;
        bail!("{}: Invalid username: {:?}", client.addr, username)
    }
    let password = buf.gjstr(10);
    if !rs_db::is_typeable_password(&password) {
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
        unsent,
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
            uid,
            reconnect,
            device: std::mem::take(&mut client.device),
        })
        .is_err()
    {
        bail!("{}: Invalid login request", client.addr)
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
#[cfg(all(since_244, before_647))]
const MAX_QUEUED_REQUESTS: usize = 8192;
#[cfg(all(since_244, before_647))]
const JS5_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(all(since_244, before_647))]
const JS5_SEND_TIMEOUT: Duration = Duration::from_secs(60);

#[cfg(all(since_244, before_647))]
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

#[cfg(all(since_244, before_647))]
fn enqueue_requests(
    addr: SocketAddr,
    queues: &mut [VecDeque<(u8, u16)>; 3],
    partial: &mut Vec<u8>,
    bytes: &[u8],
) -> anyhow::Result<()> {
    partial.extend_from_slice(bytes);
    let complete = partial.len() - partial.len() % 4;
    for &[archive, file_hi, file_lo, priority] in partial[..complete].as_chunks::<4>().0 {
        let file = u16::from_be_bytes([file_hi, file_lo]);
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

#[cfg(all(since_244, before_647))]
async fn send_js5_file(client: &mut Socket, archive: u8, file: u16) -> anyhow::Result<()> {
    // Built before the first write: a debug reload can replace the cache
    // while one waits.
    let packets = client
        .server_io
        .cache
        .read(|cache| js5_packets(cache, archive, file));
    for pkt in packets {
        client.write(&pkt.data).await?;
    }
    Ok(())
}

/// A cache file in parts of up to 500 bytes, or one empty part if it's
/// missing or too big for the length field.
#[cfg(all(since_244, before_647))]
fn js5_packets(cache: &CacheStore, archive: u8, file: u16) -> Vec<Packet> {
    let blob: &[u8] = cache
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
        return vec![pkt];
    }

    let mut packets = Vec::with_capacity(len.div_ceil(500));
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
        packets.push(pkt);
        pos += chunk;
        part = part.wrapping_add(1);
    }
    packets
}

#[allow(clippy::collapsible_match)]
pub async fn network_loop(
    mut client: Socket,
    packet_tx: Sender<(std::time::Instant, Vec<u8>)>,
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
                    if packet_tx.try_send((std::time::Instant::now(), bytes)).is_err() {
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
                    match tokio::time::timeout(WRITE_STALL_TIMEOUT, client.write_owned(bytes)).await {
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
