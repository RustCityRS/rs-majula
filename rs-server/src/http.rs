use crate::socket::handshake;
use crate::{ConnectionGuard, ServerIO, Socket};
use rs_pack::cache::CacheStore;
use sailfish::TemplateSimple;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tracing::{debug, info, warn};

enum Body {
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

    fn as_bytes(&self) -> &[u8] {
        match self {
            Body::Empty => &[],
            Body::Owned(v) => v,
            Body::Shared(a) => a,
        }
    }
}

// ---------------------------------------------------------------------------
// Client template structs
// ---------------------------------------------------------------------------
enum ClientTemplate {
    TypeScript(TypeScriptClient),
    Java(JavaClient),
}

#[derive(TemplateSimple)]
#[cfg_attr(rev = "225", template(path = "public/225/client.ejs"))]
#[cfg_attr(rev = "244", template(path = "public/244/client.ejs"))]
#[cfg_attr(rev = "245.2", template(path = "public/245.2/client.ejs"))]
#[cfg_attr(rev = "254", template(path = "public/254/client.ejs"))]
struct TypeScriptClient {
    plugin: String,
    nodeid: String,
    portoff: String,
    lowmem: String,
    members: String,
}

#[derive(TemplateSimple)]
#[cfg_attr(rev = "225", template(path = "public/225/java.ejs"))]
#[cfg_attr(rev = "244", template(path = "public/244/java.ejs"))]
#[cfg_attr(rev = "245.2", template(path = "public/245.2/java.ejs"))]
#[cfg_attr(rev = "254", template(path = "public/254/java.ejs"))]
struct JavaClient {
    plugin: String,
    nodeid: String,
    portoff: String,
    lowmem: bool,
    members: bool,
}

// ---------------------------------------------------------------------------
// Server entry point
// ---------------------------------------------------------------------------

/// Spawn the HTTP server on `port`, serving all game-client routes.
#[allow(clippy::too_many_arguments)]
pub async fn serve(
    host: String,
    port: u16,
    nodeid: String,
    portoff: String,
    members: bool,
    server_state: ServerIO,
    guard: ConnectionGuard,
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
        let portoff = portoff.clone();
        let server_state = server_state.clone();
        let guard = guard.clone();

        tokio::spawn(async move {
            info!("HTTP {:?} connected", addr);
            handle_connection(stream, nodeid, portoff, members, server_state, addr, guard).await;
        });
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_connection(
    mut stream: TcpStream,
    nodeid: String,
    portoff: String,
    members: bool,
    server_state: ServerIO,
    addr: SocketAddr,
    guard: ConnectionGuard,
) {
    stream.set_nodelay(true).ok();

    // Peek first to check if it's a WebSocket upgrade
    let mut buf = [0u8; 1024]; // increase buffer
    let n = match stream.peek(&mut buf).await {
        Ok(n) => n,
        Err(e) => {
            debug!("peek error: {}", e);
            return;
        }
    };

    let raw = String::from_utf8_lossy(&buf[..n]);

    if raw.to_ascii_lowercase().contains("upgrade: websocket") {
        #[allow(clippy::result_large_err)]
        match accept_hdr_async(stream, |req: &Request, mut res: Response| {
            if let Some(protocol) = req.headers().get("Sec-WebSocket-Protocol") {
                res.headers_mut()
                    .insert("Sec-WebSocket-Protocol", protocol.clone());
            }
            Ok(res)
        })
        .await
        {
            Ok(ws) => {
                info!("WebSocket handshake complete for {}", addr);
                let connection = Socket::from_ws(ws, addr, server_state, guard);
                if let Err(e) = handshake(connection).await {
                    info!("Connection {} closed: {}", addr, e);
                }
            }
            Err(e) => {
                info!("WebSocket handshake failed: {}", e);
            }
        }
        return;
    }

    loop {
        let mut buf = [0u8; 1024];

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

        let (status, headers, body) =
            route(path, query, &nodeid, &portoff, members, server_state.cache).await;

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
        let _ = stream.write_all(header_bytes).await;
        if !body_bytes.is_empty() {
            let _ = stream.write_all(body_bytes).await;
        }
        let _ = stream.flush().await;

        if connection_close {
            info!("HTTP {:?} closed", addr);
            break;
        }
    }
}

async fn route(
    path: &str,
    query: &str,
    nodeid: &str,
    portoff: &str,
    members: bool,
    cache: &'static CacheStore,
) -> (&'static str, String, Body) {
    match path {
        "/" => (
            "302 Found",
            "Location: /rs2.cgi?lowmem=0&plugin=0\r\nContent-Length: 0\r\n".into(),
            Body::Empty,
        ),
        "/rs2.cgi" => {
            let params = parse_query(query);
            let has_plugin = params.contains_key("plugin");
            let has_lowmem = params.contains_key("lowmem");

            if !has_plugin || !has_lowmem {
                let plugin = params.get("plugin").map(|s| s.as_str()).unwrap_or("0");
                let lowmem = params.get("lowmem").map(|s| s.as_str()).unwrap_or("0");
                return (
                    "302 Found",
                    format!(
                        "Location: /rs2.cgi?lowmem={}&plugin={}\r\nContent-Length: 0\r\n",
                        lowmem, plugin
                    ),
                    Body::Empty,
                );
            }

            let plugin = params.get("plugin").cloned().unwrap_or("0".into());
            let lowmem = params.get("lowmem").cloned().unwrap_or("0".into());
            match render_client(
                plugin,
                lowmem,
                nodeid.to_string(),
                portoff.to_string(),
                members,
            ) {
                Ok(html) => {
                    let bytes = html.into_bytes();
                    let len = bytes.len();
                    (
                        "200 OK",
                        format!(
                            "Content-Type: text/html; charset=UTF-8\r\nContent-Length: {len}\r\n"
                        ),
                        Body::Owned(bytes),
                    )
                }
                Err(e) => {
                    warn!("Template render error: {}", e);
                    (
                        "500 Internal Server Error",
                        "Content-Length: 0\r\n".into(),
                        Body::Empty,
                    )
                }
            }
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

// ---------------------------------------------------------------------------
// Routing helpers
// ---------------------------------------------------------------------------

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
        #[cfg(since_244)]
        "/ondemand.zip",
        #[cfg(since_244)]
        "/build",
    ]
    .iter()
    .any(|p| path.starts_with(p))
}

fn read_cache(path: &str, cache: &'static CacheStore) -> Option<Body> {
    if path.starts_with("/crc") {
        return Some(Body::Shared(Arc::clone(&cache.crctable_bytes)));
    }

    #[cfg(since_244)]
    if path.starts_with("/ondemand.zip") {
        return Some(Body::Shared(Arc::clone(&cache.ondemand_zip)));
    }

    #[cfg(since_244)]
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

// ---------------------------------------------------------------------------
// Template rendering
// ---------------------------------------------------------------------------

fn render_client(
    plugin: String,
    lowmem: String,
    nodeid: String,
    portoff: String,
    members: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = match plugin.as_str() {
        "3" => ClientTemplate::Java(JavaClient {
            plugin,
            nodeid,
            portoff,
            lowmem: lowmem == "1",
            members,
        }),
        _ => ClientTemplate::TypeScript(TypeScriptClient {
            plugin,
            nodeid,
            portoff,
            lowmem,
            members: members.to_string(),
        }),
    };

    Ok(match client {
        ClientTemplate::TypeScript(c) => c.render_once()?,
        ClientTemplate::Java(c) => c.render_once()?,
    })
}

// ---------------------------------------------------------------------------
// Stock responses
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Query-string parser
// ---------------------------------------------------------------------------

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
