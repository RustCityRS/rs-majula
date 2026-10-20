//! The website's private admin API, which the staff console (`rs-admin`)
//! calls to post, edit and delete news, and to upload the images and videos
//! posts embed. Requests are signed with the same shared secret the worlds
//! use (see [`rs_admin_api`]).
//!
//! Bind it to a private interface only (`--admin-host`, loopback by default).

use crate::app::App;
use crate::{media, news_render};
use axum::body::{Body, Bytes};
use axum::extract::{Path, Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rs_admin_api::{
    MAX_MEDIA_BYTES, MediaSaved, NewsCategory, NewsDelete, NewsItem, NewsList, NewsPage,
    NewsRender, NewsRendered, NewsSave, Outcome, Verifier, path,
};
use rs_db::NewsAdminPost;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};

/// News posts are long-ish text, so a request is allowed a long one.
const MAX_BODY: usize = 256 * 1024;

/// The longest title, in characters.
const MAX_TITLE_CHARS: usize = 120;

/// The most posts a page lists.
const MAX_PAGE: u32 = 100;

struct AdminState {
    app: Arc<App>,
    verifier: Verifier,
}

pub async fn serve(host: String, port: u16, secret: String, app: Arc<App>) {
    let listener = match TcpListener::bind((host.as_str(), port)).await {
        Ok(l) => l,
        Err(e) => {
            warn!("Failed to bind the admin API on: {}:{} - {}", host, port, e);
            return;
        }
    };
    info!("Admin API available at: http://{}:{}", host, port);
    let state = Arc::new(AdminState {
        app,
        verifier: Verifier::new(&secret),
    });
    let router = Router::new()
        .route(path::NEWS, post(list))
        .route(path::NEWS_SAVE, post(save))
        .route(path::NEWS_DELETE, post(delete))
        .route(path::NEWS_RENDER, post(render))
        .route(path::NEWS_MEDIA, post(upload))
        .route(&format!("{}/{{name}}", path::NEWS_MEDIA), get(media_file))
        .layer(middleware::from_fn_with_state(state.clone(), authenticate))
        .with_state(state);
    if let Err(e) = axum::serve(listener, router).await {
        warn!("Admin API stopped: {}", e);
    }
}

/// Refuses any request that isn't signed with the shared secret, is stale,
/// or replays a nonce already used.
async fn authenticate(State(state): State<Arc<AdminState>>, req: Request, next: Next) -> Response {
    let (parts, body) = req.into_parts();
    let limit = if parts.uri.path() == path::NEWS_MEDIA {
        MAX_MEDIA_BYTES
    } else {
        MAX_BODY
    };
    let Ok(body) = axum::body::to_bytes(body, limit).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    let path = parts
        .uri
        .path_and_query()
        .map_or(parts.uri.path(), |p| p.as_str());
    let header = |name: &str| parts.headers.get(name).and_then(|v| v.to_str().ok());
    if let Err(reason) = state
        .verifier
        .check(header, parts.method.as_str(), path, &body)
    {
        warn!("Admin API refused {} {}: {}", parts.method, path, reason);
        return StatusCode::UNAUTHORIZED.into_response();
    }
    next.run(Request::from_parts(parts, Body::from(body))).await
}

fn db_failed(e: rs_db::Error) -> StatusCode {
    warn!("Admin API database error: {}", e);
    StatusCode::BAD_GATEWAY
}

fn item(post: NewsAdminPost) -> NewsItem {
    NewsItem {
        id: post.id,
        category_id: post.category_id,
        title: post.title,
        markdown: post.markdown.unwrap_or(post.content),
        created_ms: post.created_ms,
    }
}

async fn list(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<NewsList>,
) -> Result<Json<NewsPage>, StatusCode> {
    let db = &state.app.db;
    let categories = db.news_categories().await.map_err(db_failed)?;
    let posts = db
        .news_admin_page(req.before, i64::from(req.limit.clamp(1, MAX_PAGE)))
        .await
        .map_err(db_failed)?;
    Ok(Json(NewsPage {
        categories: categories
            .into_iter()
            .map(|c| NewsCategory {
                id: c.id,
                name: c.name,
            })
            .collect(),
        posts: posts.into_iter().map(item).collect(),
    }))
}

/// Answers 422 for a post or category that doesn't exist.
async fn save(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<NewsSave>,
) -> Result<Json<NewsItem>, StatusCode> {
    let title = req.title.trim();
    if title.is_empty()
        || title.chars().count() > MAX_TITLE_CHARS
        || title.chars().any(char::is_control)
        || req.markdown.trim().is_empty()
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let html = news_render::render(&req.markdown);
    let db = &state.app.db;
    let saved = match req.id {
        Some(id) => {
            db.update_news_post(id, req.category_id, title, &html, &req.markdown)
                .await
        }
        None => {
            db.create_news_post(req.category_id, title, &html, &req.markdown)
                .await
        }
    }
    .map_err(db_failed)?
    .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    info!(
        "{} {} news post {} from the admin console",
        req.actor.name,
        if req.id.is_some() { "edited" } else { "posted" },
        saved.id
    );
    Ok(Json(item(saved)))
}

async fn delete(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<NewsDelete>,
) -> Result<Json<Outcome>, StatusCode> {
    if !state
        .app
        .db
        .delete_news_post(req.id)
        .await
        .map_err(db_failed)?
    {
        return Ok(Json(Outcome::refused("That post is already gone.")));
    }
    info!(
        "{} deleted news post {} from the admin console",
        req.actor.name, req.id
    );
    Ok(Json(Outcome::ok("Deleted.")))
}

/// The HTML a post's Markdown shows as, for the console's preview.
async fn render(Json(req): Json<NewsRender>) -> Json<NewsRendered> {
    Json(NewsRendered {
        html: news_render::render(&req.markdown),
    })
}

/// Stores an image or video sent as the raw body. Answers 415 for anything
/// that isn't one, and 503 when there's nowhere to keep it (`--media-dir`).
async fn upload(
    State(state): State<Arc<AdminState>>,
    body: Bytes,
) -> Result<Json<MediaSaved>, StatusCode> {
    let dir = state
        .app
        .media_dir
        .clone()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let found = media::sniff(&body).ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;
    let kind = found.kind;
    let size = body.len();
    let name = tokio::task::spawn_blocking(move || media::store(&dir, &body, &found))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|e| {
            warn!("Storing an upload failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    info!("Stored news {} {} ({} bytes)", kind, name, size);
    Ok(Json(MediaSaved {
        url: format!("/news/media/{name}"),
        kind: kind.to_owned(),
    }))
}

/// An upload, for the console's preview (the console can't reach the
/// public site's address).
async fn media_file(
    State(state): State<Arc<AdminState>>,
    Path(name): Path<String>,
) -> Result<Response, StatusCode> {
    let dir = state.app.media_dir.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    if !media::valid_name(&name) {
        return Err(StatusCode::NOT_FOUND);
    }
    let bytes = tokio::fs::read(dir.join(&name))
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let kind = media::sniff(&bytes).ok_or(StatusCode::NOT_FOUND)?;
    let content_type = match kind.extension {
        "png" => "image/png",
        "jpg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        _ => "video/webm",
    };
    Ok(([(header::CONTENT_TYPE, content_type)], bytes).into_response())
}
