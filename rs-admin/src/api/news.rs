use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use rs_admin_api::{
    MediaSaved, NewsDelete, NewsItem, NewsList, NewsPage, NewsRender, NewsRendered, NewsSave,
    Outcome, path,
};
use serde::Deserialize;
use std::sync::Arc;

/// Posts per page.
const PAGE: u32 = 20;
/// The longest title, in characters.
const MAX_TITLE_CHARS: usize = 120;
/// The longest post, in bytes.
const MAX_CONTENT: usize = 200 * 1024;

fn unavailable(error: String) -> ApiError {
    ApiError::new(StatusCode::BAD_GATEWAY, format!("{error}."))
}

#[derive(Deserialize)]
pub struct PageQuery {
    before: Option<i32>,
}

/// `GET /api/news?before=`.
pub async fn list(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<PageQuery>,
) -> Result<Json<NewsPage>, ApiError> {
    staff.require_developer()?;
    app.worlds
        .web(
            path::NEWS,
            &NewsList {
                before: query.before,
                limit: PAGE,
            },
        )
        .await
        .map(Json)
        .map_err(unavailable)
}

#[derive(Deserialize)]
pub struct SaveRequest {
    id: Option<i32>,
    category_id: i16,
    title: String,
    markdown: String,
}

/// `POST /api/news/save`: a new post, or an edit of post `id`.
pub async fn save(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<SaveRequest>,
) -> Result<Json<NewsItem>, ApiError> {
    staff.require_developer()?;
    let title = req.title.trim().to_owned();
    if title.is_empty() || title.chars().count() > MAX_TITLE_CHARS {
        return Err(ApiError::bad_request("Titles are 1 to 120 characters."));
    }
    if req.markdown.trim().is_empty() || req.markdown.len() > MAX_CONTENT {
        return Err(ApiError::bad_request("Posts need some text, up to 200 KB."));
    }
    let action = if req.id.is_some() {
        "news_edited"
    } else {
        "news_posted"
    };
    let saved: Result<NewsItem, String> = app
        .worlds
        .web(
            path::NEWS_SAVE,
            &NewsSave {
                actor: staff.actor(),
                id: req.id,
                category_id: req.category_id,
                title: title.clone(),
                markdown: req.markdown,
            },
        )
        .await;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, action)
            .target(&title)
            .detail(match &saved {
                Ok(post) => format!("post {}", post.id),
                Err(e) => e.clone(),
            })
            .ok(saved.is_ok()),
    );
    saved.map(Json).map_err(unavailable)
}

#[derive(Deserialize)]
pub struct DeleteRequest {
    id: i32,
    /// For the audit log.
    title: String,
}

/// `POST /api/news/delete`.
pub async fn delete(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<DeleteRequest>,
) -> Result<Json<Outcome>, ApiError> {
    staff.require_developer()?;
    let outcome: Outcome = app
        .worlds
        .web(
            path::NEWS_DELETE,
            &NewsDelete {
                actor: staff.actor(),
                id: req.id,
            },
        )
        .await
        .map_err(unavailable)?;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "news_deleted")
            .target(req.title.chars().take(MAX_TITLE_CHARS).collect::<String>())
            .detail(format!("post {}: {}", req.id, outcome.message))
            .ok(outcome.ok),
    );
    Ok(Json(outcome))
}

#[derive(Deserialize)]
pub struct PreviewRequest {
    markdown: String,
}

/// `POST /api/news/preview`: the HTML a post's Markdown shows as on the site.
pub async fn preview(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<PreviewRequest>,
) -> Result<Json<NewsRendered>, ApiError> {
    staff.require_developer()?;
    if req.markdown.len() > MAX_CONTENT {
        return Err(ApiError::bad_request("Posts are up to 200 KB."));
    }
    app.worlds
        .web(
            path::NEWS_RENDER,
            &NewsRender {
                markdown: req.markdown,
            },
        )
        .await
        .map(Json)
        .map_err(unavailable)
}

/// `POST /api/news/media`: an image or video for a post, as the raw body.
/// The website checks what it is and keeps it.
pub async fn upload(
    State(app): State<Arc<App>>,
    staff: Staff,
    body: Bytes,
) -> Result<Json<MediaSaved>, ApiError> {
    staff.require_developer()?;
    if body.is_empty() {
        return Err(ApiError::bad_request("That file is empty."));
    }
    let size = body.len();
    let saved = app.worlds.web_upload(body.to_vec()).await;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "news_media_uploaded")
            .target(saved.as_ref().map_or("", |m| m.url.as_str()))
            .detail(match &saved {
                Ok(m) => format!("{} of {} KB", m.kind, size.div_ceil(1024)),
                Err(e) => e.message.clone(),
            })
            .ok(saved.is_ok()),
    );
    saved.map(Json).map_err(|e| {
        if e.refused {
            ApiError::bad_request(format!("{}.", e.message))
        } else {
            unavailable(e.message)
        }
    })
}

/// `GET /api/news/media/{name}`: an upload, for the preview, which can't
/// load it from the website's public address.
pub async fn media(
    State(app): State<Arc<App>>,
    staff: Staff,
    Path(name): Path<String>,
) -> Result<Response, ApiError> {
    staff.require_developer()?;
    if name.len() > 64 || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.') {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "No such file."));
    }
    let (content_type, bytes) = app.worlds.web_media(&name).await.map_err(unavailable)?;
    // Named by their content, so a name always means the same bytes.
    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (
                header::CACHE_CONTROL,
                "private, max-age=31536000, immutable".to_owned(),
            ),
        ],
        bytes,
    )
        .into_response())
}
