use crate::app::{App, AppError};
use crate::worlds::World;
use axum::Json;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use minijinja::context;
use minijinja::value::Serde;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

/// Flag image (`/img/gamewin/<flag>flag.gif`) for each region name.
const REGION_FLAGS: &[(&str, &str)] = &[
    ("United States", "us"),
    ("Germany", "ger"),
    ("Australia", "au"),
    ("Japan", "jp"),
    ("Russia", "rus"),
    ("Local Development", "uk"),
];

pub async fn title(State(app): State<Arc<App>>) -> Result<Response, AppError> {
    let news = app.db.news_page(None, 0, 5).await.unwrap_or_else(|e| {
        warn!("Loading news for the title page failed: {e}");
        Vec::new()
    });
    let player_count: u32 = app.worlds_now().iter().map(|w| w.players).sum();
    let html = app.templates.render(
        "views/title.html",
        context! { news => Serde(news), player_count },
    )?;
    Ok(html.into_response())
}

#[derive(Serialize)]
struct Region<'a> {
    name: &'a str,
    flag: &'a str,
    worlds: Vec<&'a World>,
}

/// Groups worlds by region in first-seen order.
fn regions(worlds: &[World], members: bool) -> Vec<Region<'_>> {
    let mut out: Vec<Region> = Vec::new();
    for world in worlds.iter().filter(|w| w.members == members) {
        match out.iter_mut().find(|r| r.name == world.region) {
            Some(region) => region.worlds.push(world),
            None => out.push(Region {
                name: &world.region,
                flag: REGION_FLAGS
                    .iter()
                    .find(|(name, _)| *name == world.region)
                    .map_or("uk", |(_, flag)| flag),
                worlds: vec![world],
            }),
        }
    }
    out
}

/// The world list, reached from the detail-select form: `lores.x`/`hires.x`
/// are its image buttons and `method` is how to play.
pub async fn serverlist(
    State(app): State<Arc<App>>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Response, AppError> {
    let present = |key: &str| q.get(key).is_some_and(|v| !v.is_empty());
    let Some(method) = q.get("method").filter(|m| !m.is_empty()) else {
        return Ok(Redirect::to("/detail").into_response());
    };
    if !present("lores.x") && !present("hires.x") {
        return Ok(Redirect::to("/detail").into_response());
    }
    if method == "4" {
        return Ok(Redirect::to("/downloads").into_response());
    }

    let detail = if q.contains_key("hires.x") {
        "high"
    } else {
        "low"
    };
    let worlds = app.worlds_now();
    let html = app.templates.render(
        "views/serverlist.html",
        context! {
            detail,
            method,
            free_regions => Serde(regions(&worlds, false)),
            members_regions => Serde(regions(&worlds, true)),
        },
    )?;
    Ok(html.into_response())
}

#[derive(Deserialize)]
pub struct ClientQuery {
    world: Option<u16>,
    detail: Option<String>,
    method: Option<u8>,
}

/// Frames the chosen world's web client.
pub async fn client(
    State(app): State<Arc<App>>,
    Query(q): Query<ClientQuery>,
) -> Result<Response, AppError> {
    let (Some(id), Some(detail), Some(method)) = (q.world, q.detail, q.method) else {
        return Ok(Redirect::to("/detail").into_response());
    };
    let Some(world) = app.worlds.iter().find(|w| w.id == id) else {
        return Ok(Redirect::to("/detail").into_response());
    };
    let lowmem = u8::from(detail == "low");
    let html = app.templates.render(
        "views/client.html",
        context! { world => Serde(world), method, detail => lowmem },
    )?;
    Ok(html.into_response())
}

/// Configuration for the desktop launcher.
pub async fn jav_config(State(app): State<Arc<App>>) -> String {
    let base = app.base_url();
    let mut lines = vec![
        "viewerversion=101".to_string(),
        format!("serverlist={base}/serverlist.ws"),
        "loader_jar=loader.jar".to_string(),
        "window_preferredwidth=774".to_string(),
        "window_preferredheight=552".to_string(),
        "applet_minwidth=789".to_string(),
        "applet_minheight=532".to_string(),
        "applet_maxwidth=5670".to_string(),
        "applet_maxheight=2160".to_string(),
    ];

    let messages = [
        "lang0=English",
        "ok=OK",
        "cancel=Cancel",
        "options=Options",
        "changes_on_restart=Your changes will take effect when you next start this program.",
        "copy_paste_url=Please copy and paste the following URL into your web browser",
        "err_create_advertising=Unable to create advertising",
        "err_create_target=Unable to create target applet",
        "err_downloading=Error downloading",
        "err_get_file=Error getting file",
        "err_save_file=Error saving file",
        "err_load_bc=Unable to load browsercontrol",
        "err_verify_bc=Unable to verify browsercontrol",
        "err_verify_bc64=Unable to verify browsercontrol64",
        "information=Information",
        "language=Language",
        "switchserver=Switch World",
        "loading_app=Loading application",
        "loading_app_resources=Loading application resources",
        "message=Message",
        "new_version=Launcher update available! Please visit the website to download the latest version.",
        "new_version_linktext=Open Homepage",
        // `\` is the launcher's line break.
        "tandc=This game is copyright (c) 1999 - 2004 Jagex Ltd.\\Recreated and preserved 2023 - 2026.",
    ];
    lines.extend(messages.iter().map(|m| format!("msg={m}")));
    lines.push(format!("msg=new_version_link={base}/downloads"));

    for world in &app.worlds {
        let kind = if world.members { "Members" } else { "Free" };
        // Legacy Java runtimes cope badly with https, and the client finds the
        // game port from portoff, so the codebase is plain http on the host.
        let host = world
            .address
            .split("://")
            .nth(1)
            .unwrap_or(&world.address)
            .split(':')
            .next()
            .unwrap_or_default();
        for (detail, lowmem) in [("high", 0), ("low", 1)] {
            let detail_name = if lowmem == 0 { "High" } else { "Low" };
            lines.extend([
                format!("[{}.{detail}]", world.id),
                format!("title=RuneScape - World {} ({kind})", world.id),
                format!("codebase=http://{host}"),
                format!(
                    "servername=World {} ({kind}), {detail_name} Detail",
                    world.id
                ),
                format!("param=nodeid={}", world.node_id()),
                format!("param=portoff={}", world.port_offset),
                format!("param=free={}", u8::from(!world.members)),
                format!("param=lowmem={lowmem}"),
            ]);
        }
    }
    lines.join("\n")
}

/// The launcher's world list.
pub async fn serverlist_ws(State(app): State<Arc<App>>) -> String {
    app.worlds
        .iter()
        .flat_map(|w| [format!("{}.high,true", w.id), format!("{}.low,true", w.id)])
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn worldlist(State(app): State<Arc<App>>) -> Json<Vec<World>> {
    Json(app.worlds_now())
}
