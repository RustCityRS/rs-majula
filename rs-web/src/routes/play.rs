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
    let news = app
        .latest_news
        .get(async || app.db.news_page(None, 0, 5).await)
        .await
        .unwrap_or_else(|e| {
            warn!("Loading news for the title page failed: {e}");
            Default::default()
        });
    let player_count: u32 = app.worlds_now().iter().map(|w| w.players).sum();
    let html = app.templates.render(
        "views/title.html",
        context! { news => Serde(news.as_slice()), player_count },
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

pub async fn worldlist(State(app): State<Arc<App>>) -> Json<Vec<World>> {
    Json(app.worlds_now())
}
