use crate::ConnectionGuard;
use crate::http::read_cache;
use rs_pack::cache::CacheStore;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tracing::{debug, info, warn};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn serve(host: String, port: u16, cache: &'static CacheStore, guard: ConnectionGuard) {
    let listener = match TcpListener::bind((host.as_str(), port)).await {
        Ok(l) => l,
        Err(e) => {
            warn!("Failed to bind JAGGRAB server on {}: {}", port, e);
            return;
        }
    };

    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                debug!("JAGGRAB accept error: {}", e);
                continue;
            }
        };
        let Some(permit) = guard.try_acquire(addr.ip()) else {
            debug!("JAGGRAB {} connection limit reached", addr);
            continue;
        };
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(e) = handle(stream, addr, cache).await {
                debug!("JAGGRAB {} closed: {}", addr, e);
            }
        });
    }
}

async fn handle(
    mut stream: TcpStream,
    addr: SocketAddr,
    cache: &'static CacheStore,
) -> anyhow::Result<()> {
    stream.set_nodelay(true)?;

    let line = timeout(REQUEST_TIMEOUT, read_request_line(&mut stream)).await??;

    let Some(file) = line
        .strip_prefix("JAGGRAB /")
        .and_then(|rest| rest.split_whitespace().next())
    else {
        anyhow::bail!("malformed request line: {line:?}");
    };

    let path = format!("/{file}");
    match read_cache(&path, cache) {
        Some(body) => {
            timeout(RESPONSE_TIMEOUT, async {
                stream.write_all(body.as_bytes()).await?;
                stream.flush().await
            })
            .await
            .map_err(|_| anyhow::anyhow!("response write stalled"))??;
            info!(
                "JAGGRAB {} served {} ({} bytes)",
                addr,
                path,
                body.as_bytes().len()
            );
        }
        None => debug!("JAGGRAB: no cache file for {path:?}"),
    }

    Ok(())
}

async fn read_request_line(stream: &mut TcpStream) -> anyhow::Result<String> {
    let mut buf = Vec::with_capacity(64);
    let mut chunk = [0; 64];
    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break; // peer closed before sending a newline
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            buf.truncate(pos);
            break;
        }
        if buf.len() > 256 {
            anyhow::bail!("request line exceeded 256 bytes");
        }
    }
    Ok(String::from_utf8_lossy(&buf).trim_end().to_string())
}
