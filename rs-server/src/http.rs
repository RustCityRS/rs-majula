use crate::socket::handshake;
use crate::{ConnectionGuard, REVISION, ServerIO, Socket};
use rs_db::LoginDevice;
use rs_pack::cache::CacheStore;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_tungstenite::accept_hdr_async_with_config;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
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
    site_origin: Option<&'static str>,
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
                site_origin,
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
    site_origin: Option<&'static str>,
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
                if !same_origin(req) {
                    info!(
                        "WebSocket from {} refused: origin {:?} isn't this host",
                        addr,
                        req.headers().get("Origin")
                    );
                    let mut refused = ErrorResponse::new(None);
                    *refused.status_mut() = StatusCode::FORBIDDEN;
                    return Err(refused);
                }
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
                let mut connection = Socket::from_ws(ws, addr, server_state, guard);
                connection.device = login_device(&raw);
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
            site_origin,
        )
        .await;

        // Check if client wants to close
        let connection_close = raw.to_ascii_lowercase().contains("connection: close");

        let header = format!(
            "HTTP/1.1 {}\r\nConnection: {}\r\n{SECURITY_HEADERS}{}\r\n",
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
    header
        .and_then(|name| header_value(head, name))
        // x-forwarded-for style lists put the client first.
        .and_then(|value| value.split(',').next())
        .and_then(|value| value.trim().parse::<IpAddr>().ok())
        .map_or(peer, |ip| SocketAddr::new(ip, peer.port()))
}

/// Field `name` of the request head, if a line the peek read in full has it.
fn header_value<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    let complete = &head[..head.rfind("\r\n").unwrap_or(0)];
    complete
        .lines()
        .skip(1)
        .take_while(|line| !line.is_empty())
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case(name)
                .then_some(value.trim())
        })
}

/// What the browser says about the device, from the WebSocket request's
/// head, for staff to see who logged in with what. Browsers can claim
/// anything: it's a description, never proof.
fn login_device(head: &str) -> LoginDevice {
    // Only printable ASCII, and never more than `max` of it.
    let clean = |value: &str, max: usize| {
        let value: String = value
            .chars()
            .filter(|c| c.is_ascii_graphic() || *c == ' ')
            .take(max)
            .collect();
        (!value.is_empty()).then_some(value)
    };
    LoginDevice {
        web: true,
        user_agent: header_value(head, "User-Agent").and_then(|ua| clean(ua, 300)),
        platform: header_value(head, "Sec-CH-UA-Platform")
            .and_then(|p| clean(p.trim_matches('"'), 32)),
        language: header_value(head, "Accept-Language")
            .and_then(|l| clean(l.split([',', ';']).next().unwrap_or(""), 35)),
        touch: touch_device(head),
    }
}

/// Whether the browser says it's a phone or tablet. The web client plays
/// those by touch, which the anti-cheat checks allow for. Browsers can claim
/// anything, so this only ever excuses what touch explains. iPads and
/// touchscreen laptops pass as desktops.
fn touch_device(head: &str) -> bool {
    if header_value(head, "Sec-CH-UA-Mobile") == Some("?1") {
        return true;
    }
    header_value(head, "User-Agent").is_some_and(|ua| {
        [
            "Android",
            "iPhone",
            "iPad",
            "iPod",
            "Mobile",
            "webOS",
            "BlackBerry",
        ]
        .iter()
        .any(|word| ua.contains(word))
    })
}

/// Sent on every HTTP response: browsers take each file as the type it's
/// served as, and other sites learn no more than this origin from links.
const SECURITY_HEADERS: &str =
    "X-Content-Type-Options: nosniff\r\nReferrer-Policy: strict-origin-when-cross-origin\r\n";

/// Whether a WebSocket handshake comes from a page this host served: its
/// `Origin` names the same host (and port) as its `Host`. The web client
/// always connects back to the world that served it, so this refuses pages on
/// other sites, such as a phishing copy of the client. A handshake with no
/// `Origin` isn't from a browser page, so the check has nothing to stop.
fn same_origin(req: &Request) -> bool {
    let header = |name| req.headers().get(name).and_then(|v| v.to_str().ok());
    let Some(origin) = header("Origin") else {
        return true;
    };
    let authority = origin.split_once("://").map_or(origin, |(_, rest)| rest);
    header("Host").is_some_and(|host| authority.eq_ignore_ascii_case(host))
}

#[allow(clippy::too_many_arguments)]
async fn route(
    path: &str,
    query: &str,
    nodeid: &str,
    members: bool,
    cache: &'static CacheStore,
    jaggrab_only: bool,
    site_origin: Option<&'static str>,
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
                    "Content-Type: text/html; charset=UTF-8\r\nCache-Control: no-cache\r\n{}Content-Length: {len}\r\n",
                    frame_ancestors(site_origin)
                ),
                Body::Owned(bytes),
            )
        }
        "/worldmap" => {
            let bytes = render_worldmap(asset_version(cache, "/mapview.js")).into_bytes();
            let len = bytes.len();
            (
                "200 OK",
                format!(
                    "Content-Type: text/html; charset=UTF-8\r\nCache-Control: no-cache\r\n{}Content-Length: {len}\r\n",
                    frame_ancestors(site_origin)
                ),
                Body::Owned(bytes),
            )
        }
        "/uid" => {
            let bytes = render_uid(site_origin).into_bytes();
            let len = bytes.len();
            (
                "200 OK",
                format!(
                    "Content-Type: text/html; charset=UTF-8\r\nCache-Control: no-store\r\n{}Content-Length: {len}\r\n",
                    frame_ancestors(site_origin)
                ),
                Body::Owned(bytes),
            )
        }
        "/livemap" => {
            let bytes = render_livemap(asset_version(cache, "/mapview.js")).into_bytes();
            let len = bytes.len();
            (
                "200 OK",
                format!(
                    "Content-Type: text/html; charset=UTF-8\r\nCache-Control: no-cache\r\nContent-Length: {len}\r\n"
                ),
                Body::Owned(bytes),
            )
        }
        "/livemap.js" => (
            "200 OK",
            format!(
                "Content-Type: application/javascript\r\nCache-Control: no-cache\r\nContent-Length: {}\r\n",
                LIVEMAP_JS.len()
            ),
            Body::Owned(LIVEMAP_JS.as_bytes().to_vec()),
        ),
        "/replay" => {
            let bytes = render_replay(nodeid, members, client_version(cache)).into_bytes();
            let len = bytes.len();
            (
                "200 OK",
                format!(
                    "Content-Type: text/html; charset=UTF-8\r\nCache-Control: no-cache\r\nContent-Length: {len}\r\n"
                ),
                Body::Owned(bytes),
            )
        }
        "/replay.js" => (
            "200 OK",
            format!(
                "Content-Type: application/javascript\r\nCache-Control: no-cache\r\nContent-Length: {}\r\n",
                REPLAY_JS.len()
            ),
            Body::Owned(REPLAY_JS.as_bytes().to_vec()),
        ),
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
        "/worldmap.jag",
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
        p if p.starts_with("/worldmap.jag") => "worldmap",
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
/// changes (a new release, or `deploy/patch.py` patching in a new login key)
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

/// Plays a recorded session in this revision's client, for the staff console.
const REPLAY_JS: &str = include_str!("replay.js");

/// The replay page: this revision's client, with replay.js loaded first to
/// stand in for the world.
fn render_replay(nodeid: &str, members: bool, version: Option<u32>) -> String {
    let (width, height) = if cfg!(rev = "225") {
        (789, 532)
    } else {
        (765, 503)
    };
    let query = version.map_or_else(String::new, |v| format!("?v={v:08x}"));
    format!(
        r#"<!DOCTYPE html>
<html data-rev="{REVISION}">
<head>
<meta charset="utf-8">
<title>Replay</title>
<style>
html, body {{ margin: 0; width: 100%; height: 100%; background: #000; overflow: hidden; }}
#game {{ position: relative; width: 100vw; height: 100vh; }}
#canvas {{ display: block; width: 100%; height: 100%; image-rendering: pixelated; }}
</style>
<script src="/replay.js"></script>
</head>
<body>
<div id="game"><canvas id="canvas" width="{width}" height="{height}"></canvas></div>
<script type="module">
import {{ Client }} from './{REVISION}/client.js{query}';
window.__replay.start(new Client({nodeid}, 0, {members}));
</script>
</body>
</html>
"#
    )
}

/// The page the website's account recovery form frames to learn the id this
/// world's web client gave the browser (kept in its localStorage, as the
/// client framed on the website's play page sees it). It tells only the
/// website, trying each of its origins (a browser delivers to the one that
/// framed it), so says nothing without `site_origin`.
fn render_uid(site_origin: Option<&str>) -> String {
    let sites: Vec<&str> = site_origin.map_or_else(Vec::new, |s| s.split_whitespace().collect());
    let sites = serde_json::to_string(&sites)
        .unwrap_or_else(|_| "[]".to_owned())
        .replace('<', "\\u003c");
    format!(
        "<!doctype html><script>(function(){{var sites={sites};if(parent===window)return;\
         try{{var v=localStorage.getItem('rs-uid')|0;if(!v||v===1337)return;\
         sites.forEach(function(site){{try{{parent.postMessage({{rsUid:v}},site)}}catch(e){{}}}})}}catch(e){{}}}})()</script>"
    )
}

/// The world map viewer for this revision's `worldmap.jag`, framed by the
/// website's world map page. `mapview.js` is shared by every revision.
fn render_worldmap(version: Option<u32>) -> String {
    let query = version.map_or_else(String::new, |v| format!("?v={v:08x}"));
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>World Map</title>
<style>html, body {{ margin: 0; background: #000; overflow: hidden; }} #canvas {{ display: block; }}</style>
</head>
<body>
<canvas id="canvas" width="635" height="503"></canvas>
<script type="module">
import {{ MapView }} from '/mapview.js{query}';
new MapView();
</script>
</body>
</html>
"#
    )
}

/// Draws the players the staff console sends over the world map.
const LIVEMAP_JS: &str = include_str!("livemap.js");

/// The world map viewer with livemap.js drawing players over it, for the
/// staff console's live map. Sized to the frame it loads in.
fn render_livemap(version: Option<u32>) -> String {
    let query = version.map_or_else(String::new, |v| format!("?v={v:08x}"));
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Live Map</title>
<style>html, body {{ margin: 0; width: 100%; height: 100%; background: #000; overflow: hidden; }} #map {{ position: relative; width: 100%; height: 100%; }} #canvas {{ display: block; width: 100%; height: 100%; }}</style>
<script src="/livemap.js"></script>
</head>
<body>
<div id="map"><canvas id="canvas" width="635" height="503"></canvas></div>
<script>
// The viewer draws at the canvas's size: the frame's, with a width that
// isn't also the height (livemap.js tells the two apart by it).
const c = document.getElementById('canvas');
c.width = Math.max(320, Math.min(2560, innerWidth));
c.height = Math.max(240, Math.min(1600, innerHeight));
if (c.width === c.height) c.width += 1;
</script>
<script type="module">
import {{ MapView }} from '/mapview.js{query}';
window.__livemap.start(new MapView());
</script>
</body>
</html>
"#
    )
}

/// The client page's framing rule: only this host and the website (which
/// frames it on its play page) may show it, so no other site can wrap the
/// game in its own page. Without a website origin, framing is left open (a
/// local setup frames the world from another port).
fn frame_ancestors(site_origin: Option<&str>) -> String {
    site_origin.map_or_else(String::new, |site| {
        format!("Content-Security-Policy: frame-ancestors 'self' {site}\r\n")
    })
}

/// The CRC of the client.js this world serves, `None` if it has none.
fn client_version(cache: &CacheStore) -> Option<u32> {
    asset_version(cache, &format!("/{REVISION}/client.js"))
}

/// The CRC of a static asset, `None` if this world doesn't have it.
fn asset_version(cache: &CacheStore, path: &str) -> Option<u32> {
    let data = cache.static_assets.get(path)?;
    Some(rs_io::crc::getcrc(data, 0, data.len()) as u32)
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

    fn ws_request(origin: Option<&str>, host: &str) -> Request {
        let mut req = Request::builder().header("Host", host);
        if let Some(origin) = origin {
            req = req.header("Origin", origin);
        }
        req.body(()).unwrap()
    }

    #[test]
    fn websockets_must_come_from_this_host() {
        let same = |origin, host| same_origin(&ws_request(origin, host));
        assert!(same(Some("https://w1.rustcity.org"), "w1.rustcity.org"));
        assert!(same(Some("http://localhost:8080"), "localhost:8080"));
        assert!(same(Some("https://W1.RustCity.org"), "w1.rustcity.org"));
        assert!(!same(Some("https://evil.example"), "w1.rustcity.org"));
        assert!(!same(Some("https://w2.rustcity.org"), "w1.rustcity.org"));
        assert!(!same(Some("http://localhost:9999"), "localhost:8080"));
        assert!(!same(Some("null"), "w1.rustcity.org"));
        // Not a browser page.
        assert!(same(None, "w1.rustcity.org"));
    }

    #[test]
    fn client_page_framing_names_the_website() {
        assert_eq!(
            frame_ancestors(Some("https://rustcity.org")),
            "Content-Security-Policy: frame-ancestors 'self' https://rustcity.org\r\n"
        );
        assert_eq!(frame_ancestors(None), "");
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

    #[test]
    fn login_devices_keep_what_the_browser_said() {
        let head = "GET / HTTP/1.1\r\nUser-Agent: Mozilla/5.0 (Windows NT 10.0) Chrome/141.0\r\nSec-CH-UA-Platform: \"Windows\"\r\nAccept-Language: en-GB,en;q=0.9\r\n\r\n";
        let device = login_device(head);
        assert!(device.web && !device.touch);
        assert_eq!(
            device.user_agent.as_deref(),
            Some("Mozilla/5.0 (Windows NT 10.0) Chrome/141.0")
        );
        assert_eq!(device.platform.as_deref(), Some("Windows"));
        assert_eq!(device.language.as_deref(), Some("en-GB"));
        // Nothing past what's printable, or past the cap.
        let long = format!(
            "GET / HTTP/1.1\r\nUser-Agent: \u{7}{}\r\n\r\n",
            "a".repeat(400)
        );
        let ua = login_device(&long).user_agent.unwrap();
        assert_eq!(ua, "a".repeat(300));
        assert_eq!(login_device("GET / HTTP/1.1\r\n\r\n").user_agent, None);
    }

    #[test]
    fn phones_and_tablets_are_touch_devices() {
        let head = |headers: &str| format!("GET / HTTP/1.1\r\nHost: w1\r\n{headers}\r\n");
        let android = "User-Agent: Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Mobile Safari/537.36\r\n";
        let iphone = "User-Agent: Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15\r\n";
        let desktop =
            "User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/141.0 Safari/537.36\r\n";
        assert!(touch_device(&head(android)));
        assert!(touch_device(&head(iphone)));
        assert!(touch_device(&head(&format!(
            "{desktop}Sec-CH-UA-Mobile: ?1\r\n"
        ))));
        assert!(!touch_device(&head(desktop)));
        assert!(!touch_device(&head(&format!(
            "{desktop}Sec-CH-UA-Mobile: ?0\r\n"
        ))));
        // A head cut off mid-line says nothing.
        assert!(!touch_device(
            "GET / HTTP/1.1\r\nUser-Agent: Mozilla/5.0 (iPh"
        ));
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
