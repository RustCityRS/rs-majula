use crate::socket::handshake;
use crate::{ConnectionGuard, REVISION, ServerIO, Socket};
use rs_pack::cache::CacheStore;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_tungstenite::accept_hdr_async_with_config;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tracing::{debug, info, warn};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_TIMEOUT: Duration = Duration::from_secs(30);

#[allow(clippy::field_reassign_with_default)]
fn ws_config() -> WebSocketConfig {
    let mut config = WebSocketConfig::default();
    config.read_buffer_size = 4 * 1024;
    config.max_message_size = Some(64 * 1024);
    config.max_frame_size = Some(64 * 1024);
    config
}

pub(crate) enum Body {
    Empty,
    Owned(Vec<u8>),
    Shared(Arc<[u8]>),
}

impl Body {
    fn len(&self) -> usize {
        match self {
            Body::Empty => 0,
            Body::Owned(v) => v.len(),
            Body::Shared(a) => a.len(),
        }
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        match self {
            Body::Empty => &[],
            Body::Owned(v) => v,
            Body::Shared(a) => a,
        }
    }
}

/// This revision's web client page. Its one dynamic line is
/// `new Client({{nodeid}}, {{lowmem}}, {{members}});`, filled in by
/// [`render_client`].
#[cfg(rev = "225")]
const CLIENT_HTML: &str = include_str!("../../public/225/client.html");
#[cfg(rev = "244")]
const CLIENT_HTML: &str = include_str!("../../public/244/client.html");
#[cfg(rev = "245.2")]
const CLIENT_HTML: &str = include_str!("../../public/245.2/client.html");
#[cfg(rev = "254")]
const CLIENT_HTML: &str = include_str!("../../public/254/client.html");
#[cfg(rev = "274")]
const CLIENT_HTML: &str = include_str!("../../public/274/client.html");
#[cfg(rev = "289")]
const CLIENT_HTML: &str = include_str!("../../public/289/client.html");

pub async fn serve(
    host: String,
    port: u16,
    nodeid: String,
    members: bool,
    server_state: ServerIO,
    guard: ConnectionGuard,
    http_guard: ConnectionGuard,
    jaggrab_only: bool,
    real_ip_header: Option<&'static str>,
) {
    let listener = match TcpListener::bind((host, port)).await {
        Ok(l) => l,
        Err(e) => {
            warn!("Failed to bind HTTP server on: {} - {}", port, e);
            return;
        }
    };

    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                debug!("HTTP accept error: {}", e);
                continue;
            }
        };

        let nodeid = nodeid.clone();
        let server_state = server_state.clone();
        let guard = guard.clone();
        let http_guard = http_guard.clone();

        tokio::spawn(async move {
            handle_connection(
                stream,
                nodeid,
                members,
                server_state,
                addr,
                guard,
                http_guard,
                jaggrab_only,
                real_ip_header,
            )
            .await;
        });
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_connection(
    mut stream: TcpStream,
    nodeid: String,
    members: bool,
    server_state: ServerIO,
    peer: SocketAddr,
    guard: ConnectionGuard,
    http_guard: ConnectionGuard,
    jaggrab_only: bool,
    real_ip_header: Option<&'static str>,
) {
    stream.set_nodelay(true).ok();

    // Without a proxy the peer is the client, so it is limited before any
    // bytes are read. Behind one, the limit waits for the forwarded address.
    let permit = match real_ip_header {
        Some(_) => None,
        None => {
            let Some(permit) = http_guard.try_acquire(peer.ip()) else {
                debug!("HTTP {} connection limit reached", peer);
                return;
            };
            Some(permit)
        }
    };

    // Peek first to check if it's a WebSocket upgrade
    let mut buf = [0; 4096];
    let n = match timeout(HANDSHAKE_TIMEOUT, stream.peek(&mut buf)).await {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => {
            debug!("peek error: {}", e);
            return;
        }
        Err(_) => {
            debug!("HTTP {} handshake timeout", peer);
            return;
        }
    };

    let raw = String::from_utf8_lossy(&buf[..n]);
    let addr = client_addr(&raw, real_ip_header, peer);
    let Some(_permit) = permit.or_else(|| http_guard.try_acquire(addr.ip())) else {
        debug!("HTTP {} connection limit reached", addr);
        return;
    };
    info!("HTTP {:?} connected", addr);

    if raw.to_ascii_lowercase().contains("upgrade: websocket") {
        #[allow(clippy::result_large_err)]
        let upgrade = accept_hdr_async_with_config(
            stream,
            |req: &Request, mut res: Response| {
                if let Some(protocol) = req.headers().get("Sec-WebSocket-Protocol") {
                    res.headers_mut()
                        .insert("Sec-WebSocket-Protocol", protocol.clone());
                }
                Ok(res)
            },
            Some(ws_config()),
        );
        match timeout(HANDSHAKE_TIMEOUT, upgrade).await {
            Ok(Ok(ws)) => {
                info!("WebSocket handshake complete for {}", addr);
                let connection = Socket::from_ws(ws, addr, server_state, guard);
                if let Err(e) = handshake(connection).await {
                    info!("Connection {} closed: {}", addr, e);
                }
            }
            Ok(Err(e)) => {
                info!("WebSocket handshake failed: {}", e);
            }
            Err(_) => {
                info!("WebSocket handshake timed out for {}", addr);
            }
        }
        return;
    }

    loop {
        let mut buf = [0; 1024];

        // Disconnect if idle for 30 seconds
        let n = match timeout(Duration::from_secs(30), stream.read(&mut buf)).await {
            Ok(Ok(0)) | Err(_) | Ok(Err(_)) => {
                info!("HTTP {:?} closed", addr);
                break;
            } // connection closed
            Ok(Ok(n)) => n,
        };

        let raw = String::from_utf8_lossy(&buf[..n]);
        let first_line = raw.lines().next().unwrap_or("");

        let mut parts = first_line.split_whitespace();
        let method = parts.next().unwrap_or("");
        let full_path = parts.next().unwrap_or("/");

        let (path, query) = match full_path.find('?') {
            Some(i) => (&full_path[..i], &full_path[i + 1..]),
            None => (full_path, ""),
        };

        if method != "GET" {
            let _ = stream
                .write_all(
                    b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
                )
                .await;
            break;
        }

        let (status, headers, body) = route(
            path,
            query,
            &nodeid,
            members,
            server_state.cache,
            jaggrab_only,
        )
        .await;

        // Check if client wants to close
        let connection_close = raw.to_ascii_lowercase().contains("connection: close");

        let header = format!(
            "HTTP/1.1 {}\r\nConnection: {}\r\n{}\r\n",
            status,
            if connection_close {
                "close"
            } else {
                "keep-alive"
            },
            headers
        );
        let header_bytes = header.as_bytes();
        let body_bytes = body.as_bytes();
        let write = async {
            stream.write_all(header_bytes).await?;
            if !body_bytes.is_empty() {
                stream.write_all(body_bytes).await?;
            }
            stream.flush().await
        };
        match timeout(WRITE_TIMEOUT, write).await {
            Ok(Ok(())) => {}
            Ok(Err(_)) | Err(_) => {
                info!("HTTP {:?} write failed or stalled", addr);
                break;
            }
        }

        if connection_close {
            info!("HTTP {:?} closed", addr);
            break;
        }
    }
}

/// The client's address: the `header` field of the request head when it
/// holds a valid IP, otherwise the socket peer. Only lines the peek read in
/// full are searched, so a cut-off value is never parsed.
fn client_addr(head: &str, header: Option<&str>, peer: SocketAddr) -> SocketAddr {
    let complete = &head[..head.rfind("\r\n").unwrap_or(0)];
    header
        .and_then(|name| {
            complete
                .lines()
                .skip(1)
                .take_while(|line| !line.is_empty())
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.trim().eq_ignore_ascii_case(name).then_some(value)
                })
        })
        // x-forwarded-for style lists put the client first.
        .and_then(|value| value.split(',').next())
        .and_then(|value| value.trim().parse::<IpAddr>().ok())
        .map_or(peer, |ip| SocketAddr::new(ip, peer.port()))
}

async fn route(
    path: &str,
    query: &str,
    nodeid: &str,
    members: bool,
    cache: &'static CacheStore,
    jaggrab_only: bool,
) -> (&'static str, String, Body) {
    // Test hook: refusing cache downloads over HTTP makes a stock desktop
    // client toggle its archive loader onto the JAGGRAB fallback.
    if jaggrab_only && matches_cache(path) {
        return not_found();
    }
    match path {
        "/" => (
            "302 Found",
            "Location: /rs2.cgi?lowmem=0&plugin=0\r\nContent-Length: 0\r\n".into(),
            Body::Empty,
        ),
        "/rs2.cgi" => {
            let params = parse_query(query);
            // Only ever echoed as parsed numbers: they end up in a redirect
            // header and in the page's JavaScript.
            let plugin = query_number(&params, "plugin");
            let lowmem = query_number(&params, "lowmem") != 0;

            if !params.contains_key("plugin") || !params.contains_key("lowmem") {
                return (
                    "302 Found",
                    format!(
                        "Location: /rs2.cgi?lowmem={}&plugin={}\r\nContent-Length: 0\r\n",
                        u8::from(lowmem),
                        plugin
                    ),
                    Body::Empty,
                );
            }

            let bytes = render_client(nodeid, lowmem, members, client_version(cache)).into_bytes();
            let len = bytes.len();
            (
                "200 OK",
                // Never reused: it names the client.js to load (see render_client).
                format!(
                    "Content-Type: text/html; charset=UTF-8\r\nCache-Control: no-cache\r\nContent-Length: {len}\r\n"
                ),
                Body::Owned(bytes),
            )
        }
        p if matches_cache(p) => match read_cache(p, cache) {
            Some(body) => {
                let len = body.len();
                (
                    "200 OK",
                    format!("Content-Type: application/octet-stream\r\nContent-Length: {len}\r\n"),
                    body,
                )
            }
            None => bad_request(),
        },
        p if is_asset(p) => match read_asset(p, cache).await {
            Some((content_type, body)) => {
                let len = body.len();
                (
                    "200 OK",
                    format!("Content-Type: {content_type}\r\nContent-Length: {len}\r\n"),
                    body,
                )
            }
            None => not_found(),
        },
        _ => bad_request(),
    }
}

fn matches_cache(path: &str) -> bool {
    [
        "/title",
        "/config",
        "/interface",
        "/media",
        "/models",
        "/textures",
        "/wordenc",
        "/sounds",
        "/crc",
        #[cfg(since_244)]
        "/versionlist",
        #[cfg(all(since_244, before_274))]
        "/ondemand.zip",
        #[cfg(all(since_244, before_274))]
        "/build",
    ]
    .iter()
    .any(|p| path.starts_with(p))
}

pub(crate) fn read_cache(path: &str, cache: &'static CacheStore) -> Option<Body> {
    if path.starts_with("/crc") {
        return Some(Body::Shared(Arc::clone(&cache.crctable_bytes)));
    }

    #[cfg(all(since_244, before_274))]
    if path.starts_with("/ondemand.zip") {
        return Some(Body::Shared(Arc::clone(&cache.ondemand_zip)));
    }

    #[cfg(all(since_244, before_274))]
    if path.starts_with("/build") {
        return Some(Body::Shared(Arc::clone(&cache.build)));
    }

    #[cfg(since_244)]
    if path.starts_with("/versionlist") {
        return cache
            .jags
            .get("versionlist")
            .map(|arc| Body::Shared(Arc::clone(arc)));
    }

    let key = match path {
        p if p.starts_with("/title") => "title",
        p if p.starts_with("/config") => "config",
        p if p.starts_with("/interface") => "interface",
        p if p.starts_with("/media") => "media",
        p if p.starts_with("/models") => "models",
        p if p.starts_with("/textures") => "textures",
        p if p.starts_with("/wordenc") => "wordenc",
        p if p.starts_with("/sounds") => "sounds",
        _ => return None,
    };

    let suffix = &path[1 + key.len()..];
    if let Ok(crc) = suffix.parse::<i32>()
        && cache.crcs.get(key).is_some_and(|&expected| expected != crc)
    {
        return None;
    }

    cache.jags.get(key).map(|arc| Body::Shared(Arc::clone(arc)))
}

fn is_asset(path: &str) -> bool {
    path.ends_with(".js")
        || path.ends_with(".wasm")
        || path.ends_with(".sf2")
        || path.ends_with(".mjs")
        || path.ends_with(".ico")
        || path.ends_with(".mid")
}

async fn read_asset(path: &str, cache: &'static CacheStore) -> Option<(&'static str, Body)> {
    if path.ends_with(".mid") {
        let filename = path.split('/').next_back().unwrap_or(path);
        let stem = filename.strip_suffix(".mid").unwrap_or(filename);
        let (prefix, crc_str) = stem.rsplit_once('_')?;
        let crc: i32 = crc_str.parse().ok()?;
        return cache
            .songs
            .get_by_name(prefix)
            .or(cache.jingles.get_by_name(prefix))
            .filter(|midi| crc == 12345678 || midi.crc == crc)
            .map(|midi| ("audio/midi", Body::Shared(Arc::clone(&midi.data))));
    }

    let content_type: &'static str = if path.ends_with(".js") || path.ends_with(".mjs") {
        "application/javascript"
    } else if path.ends_with(".wasm") {
        "application/wasm"
    } else if path.ends_with(".sf2") {
        "application/octet-stream"
    } else if path.ends_with(".ico") {
        "image/vnd.microsoft.icon"
    } else {
        return None;
    };

    if let Some(data) = cache.static_assets.get(path) {
        return Some((content_type, Body::Shared(Arc::clone(data))));
    }

    None
}

/// Fills in this revision's client page. Every value is a number or a bool,
/// so no request text reaches the page's JavaScript.
///
/// With `version`, the page imports `client.js?v=<version>`: a client.js that
/// changes (a new release, or `rotate-rsa.py` patching in a new login key)
/// gets a new URL, so no browser or Cloudflare cache serves the old one.
fn render_client(nodeid: &str, lowmem: bool, members: bool, version: Option<u32>) -> String {
    let html = CLIENT_HTML
        .replace("{{nodeid}}", nodeid)
        .replace("{{lowmem}}", if lowmem { "1" } else { "0" })
        .replace("{{members}}", if members { "true" } else { "false" });
    match version {
        Some(v) => html.replace("client.js'", &format!("client.js?v={v:08x}'")),
        None => html,
    }
}

/// The CRC of the client.js this world serves, `None` if it has none.
fn client_version(cache: &CacheStore) -> Option<u32> {
    let js = cache
        .static_assets
        .get(format!("/{REVISION}/client.js").as_str())?;
    Some(rs_io::crc::getcrc(js, 0, js.len()) as u32)
}

/// A numeric query parameter; missing or non-numeric reads as 0.
fn query_number(params: &HashMap<String, String>, key: &str) -> u8 {
    params.get(key).and_then(|v| v.parse().ok()).unwrap_or(0)
}

fn bad_request() -> (&'static str, String, Body) {
    (
        "400 Bad Request",
        "Content-Length: 0\r\n".into(),
        Body::Empty,
    )
}

fn not_found() -> (&'static str, String, Body) {
    ("404 Not Found", "Content-Length: 0\r\n".into(), Body::Empty)
}

fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut it = pair.splitn(2, '=');
            let key = it.next()?.to_string();
            let val = it.next().unwrap_or("").to_string();
            if key.is_empty() {
                None
            } else {
                Some((key, val))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_page_fills_every_marker() {
        let html = render_client("10", true, false, None);
        assert!(html.contains("new Client(10, 1, false);"));
        assert!(!html.contains("{{"));
    }

    #[test]
    fn client_page_versions_its_client_js() {
        let html = render_client("10", true, false, Some(0xab12));
        assert_eq!(html.matches("/client.js?v=0000ab12'").count(), 1);
        assert!(!render_client("10", true, false, None).contains("?v="));
    }

    #[test]
    fn query_numbers_never_echo_text() {
        let params = parse_query("plugin=3&lowmem=0);alert(document.cookie);(0");
        assert_eq!(query_number(&params, "plugin"), 3);
        assert_eq!(query_number(&params, "lowmem"), 0);
        assert_eq!(query_number(&params, "missing"), 0);
    }

    const PEER: &str = "127.0.0.1:50000";

    fn client(head: &str, header: Option<&str>) -> String {
        client_addr(head, header, PEER.parse().unwrap()).to_string()
    }

    #[test]
    fn real_ip_header_replaces_the_proxy_peer() {
        let head = "GET / HTTP/1.1\r\nHost: w1\r\nX-Real-IP: 203.0.113.7\r\n\r\n";
        assert_eq!(client(head, Some("x-real-ip")), "203.0.113.7:50000");
        assert_eq!(client(head, None), PEER);
    }

    #[test]
    fn forwarded_for_lists_take_the_client() {
        let head = "GET / HTTP/1.1\r\nx-forwarded-for: 2001:db8::1, 10.0.0.2\r\n\r\n";
        assert_eq!(client(head, Some("X-Forwarded-For")), "[2001:db8::1]:50000");
    }

    #[test]
    fn bad_missing_or_cut_off_values_keep_the_peer() {
        let bad = "GET / HTTP/1.1\r\nX-Real-IP: not-an-ip\r\n\r\n";
        assert_eq!(client(bad, Some("x-real-ip")), PEER);
        let missing = "GET / HTTP/1.1\r\nHost: w1\r\n\r\n";
        assert_eq!(client(missing, Some("x-real-ip")), PEER);
        let cut_off = "GET / HTTP/1.1\r\nHost: w1\r\nX-Real-IP: 2001:db8::1";
        assert_eq!(client(cut_off, Some("x-real-ip")), PEER);
        // A body or a later request can't smuggle the field in.
        let body = "GET / HTTP/1.1\r\n\r\nX-Real-IP: 203.0.113.7\r\n";
        assert_eq!(client(body, Some("x-real-ip")), PEER);
    }
}
