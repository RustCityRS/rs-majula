use crate::{ConnectionPermit, REVISION, Socket};
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
use tokio::sync::mpsc;

#[repr(u8)]
#[derive(TryFromPrimitive)]
pub enum HandshakeType {
    New = 14,
    Js5 = 15,
    Login = 16,
    Reconnect = 18,
}

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
    let Some(bytes) = client.read().await? else {
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

    let (handshake, buf) = loop {
        let Some(bytes) = client.read().await? else {
            bail!("no bytes")
        };
        if bytes.is_empty() {
            bail!("empty bytes")
        }
        let mut buf = Packet::from(bytes);
        match HandshakeType::try_from(buf.g1())? {
            HandshakeType::New => {
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
    let version = buf.g1(); // the client revision.
    if version.to_string() != REVISION {
        let _ = client.write(&[LoginResponse::RuneScapeUpdated as u8]).await;
        bail!("{}: Invalid version: {}", client.addr, version)
    }
    let info = buf.g1();
    let low_memory = (info & 0x1) != 0;
    let crcs: Vec<i32> = (0..9).map(|_| buf.g4s()).collect();
    if crcs
        .iter()
        .any(|x| !client.server_io.cache.crctable.contains(x))
    {
        let _ = client.write(&[LoginResponse::RuneScapeUpdated as u8]).await;
        bail!(
            "{}: Invalid crctable: {:?} : {:?}",
            client.addr,
            crcs,
            client.server_io.cache.crctable
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
async fn js5_loop(mut client: Socket) -> anyhow::Result<()> {
    loop {
        let Some(bytes) = client.read().await? else {
            return Ok(()); // connection closed
        };
        let mut buf = Packet::from(bytes);
        while buf.len().saturating_sub(buf.pos) >= 4 {
            let archive = buf.g1();
            let file = buf.g2();
            let priority = buf.g1();
            if archive > 3 || priority > 2 {
                bail!(
                    "{}: invalid ondemand request (archive={}, priority={})",
                    client.addr,
                    archive,
                    priority
                );
            }
            send_js5_file(&mut client, archive, file).await?;
        }
    }
}

#[cfg(since_244)]
async fn send_js5_file(client: &mut Socket, archive: u8, file: u16) -> anyhow::Result<()> {
    let cache = client.server_io.cache;
    let blob: &[u8] = cache
        .ondemand
        .get(archive as usize)
        .and_then(|files| files.get(file as usize))
        .map(|b| b.as_ref())
        .unwrap_or(&[]);
    let len = blob.len();

    if len == 0 {
        let mut pkt = Vec::with_capacity(6);
        pkt.push(archive);
        pkt.extend_from_slice(&file.to_be_bytes());
        pkt.extend_from_slice(&0u16.to_be_bytes());
        pkt.push(0);
        client.write(&pkt).await?;
        return Ok(());
    }

    let mut pos = 0;
    let mut part = 0;
    while pos < len {
        let chunk = (len - pos).min(500);
        let mut pkt = Vec::with_capacity(6 + chunk);
        pkt.push(archive);
        pkt.extend_from_slice(&file.to_be_bytes());
        pkt.extend_from_slice(&(len as u16).to_be_bytes());
        pkt.push(part);
        pkt.extend_from_slice(&blob[pos..pos + chunk]);
        client.write(&pkt).await?;
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
                    // Return the drained buffer to the engine for reuse (TCP
                    // only; WebSocket consumes it). A send failure just means
                    // the engine dropped the client, so the buffer is freed.
                    if let Some(returned) = client.write_owned(bytes).await? {
                        let _ = recycle_tx.send(returned);
                    }
                }
                None => {
                    bail!("{addr} engine closed write channel");
                }
            },
        }
    }
}
