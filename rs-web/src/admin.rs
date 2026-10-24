use crate::app::App;
use crate::news_render;
use crate::routes::messages::{message_text, subject_text};
use axum::body::{Body, Bytes};
use axum::extract::{Path, Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rs_admin_api::{
    Appeal, AppealDecide, AppealLink, AppealsWaiting, MAX_MEDIA_BYTES, MediaSaved, MessageGet,
    MessageList, MessageSend, MessageSent, NewsCategory, NewsDelete, NewsItem, NewsList, NewsPage,
    NewsRender, NewsRendered, NewsSave, Outcome, RecoveriesWaiting, Recovery, RecoveryDecide,
    RecoveryLink, StaffMessage, Verifier, media, path,
};
use rs_db::{NewsAdminPost, RecoveryDecided, StaffMessageRecord, validate_name};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};

/// News posts are long-ish text, so a request is allowed a long one.
const MAX_BODY: usize = 256 << 10;

/// The longest title, in characters.
const MAX_TITLE_CHARS: usize = 120;

/// The most posts a page lists.
const MAX_PAGE: u32 = 100;

/// The most appeals handed over at once.
const MAX_APPEALS: u32 = 100;

/// The longest note a player is shown with a decision, in characters.
const MAX_DECISION_NOTE: usize = 2000;

/// How long an approved recovery request's password reset can be redeemed.
const RECOVERY_RESET_HOURS: i32 = 72;

/// The most messages a page lists.
const MAX_MESSAGES: u32 = 100;

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
        .route(path::APPEALS_WAITING, post(appeals_waiting))
        .route(path::APPEAL_LINK, post(appeal_link))
        .route(path::APPEAL_DECIDE, post(appeal_decide))
        .route(path::RECOVERIES_WAITING, post(recoveries_waiting))
        .route(path::RECOVERY_LINK, post(recovery_link))
        .route(path::RECOVERY_DECIDE, post(recovery_decide))
        .route(path::MESSAGES, post(messages))
        .route(path::MESSAGE_GET, post(message_get))
        .route(path::MESSAGE_SEND, post(message_send))
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
    state.app.latest_news.clear();
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
    state.app.latest_news.clear();
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

/// Appeals players sent that no case has taken in yet, oldest first.
async fn appeals_waiting(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<AppealsWaiting>,
) -> Result<Json<Vec<Appeal>>, StatusCode> {
    let rows = state
        .app
        .db
        .waiting_appeals(i64::from(req.limit.clamp(1, MAX_APPEALS)))
        .await
        .map_err(db_failed)?;
    Ok(Json(
        rows.into_iter()
            .map(|a| Appeal {
                id: a.id,
                player: a.player,
                sanction: a.sanction,
                sanction_until_ms: a.sanction_until_ms,
                issued_by: a.issued_by,
                reason: a.reason,
                statement: a.statement,
                filed_ms: a.created_ms,
            })
            .collect(),
    ))
}

async fn appeal_link(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<AppealLink>,
) -> Result<Json<Outcome>, StatusCode> {
    let case = i64::try_from(req.case_id).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let linked = state
        .app
        .db
        .link_appeal(req.id, case)
        .await
        .map_err(db_failed)?;
    Ok(Json(if linked {
        Outcome::ok("Linked.")
    } else {
        Outcome::refused("There's no such appeal.")
    }))
}

/// Records a decision for the player to see on the site.
async fn appeal_decide(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<AppealDecide>,
) -> Result<Json<Outcome>, StatusCode> {
    let note = req.note.trim();
    if !matches!(req.decision.as_str(), "upheld" | "overturned")
        || note.chars().count() > MAX_DECISION_NOTE
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let decided = state
        .app
        .db
        .decide_appeal(req.id, &req.decision, note, &req.actor.name)
        .await
        .map_err(db_failed)?;
    if !decided {
        return Ok(Json(Outcome::refused("There's no such appeal.")));
    }
    info!(
        "{} decided appeal {} from the admin console: {}",
        req.actor.name, req.id, req.decision
    );
    Ok(Json(Outcome::ok("Decided.")))
}

/// Recovery requests players sent that no case has taken in yet, oldest
/// first.
async fn recoveries_waiting(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<RecoveriesWaiting>,
) -> Result<Json<Vec<Recovery>>, StatusCode> {
    let rows = state
        .app
        .db
        .waiting_recoveries(i64::from(req.limit.clamp(1, MAX_APPEALS)))
        .await
        .map_err(db_failed)?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Recovery {
                id: r.id,
                player: r.player,
                reason: r.reason,
                details: r.details,
                passwords_given: r.passwords_given.clamp(0, 255) as u8,
                password_matched: r.password_matched,
                created_month: r.answers.created_month,
                last_played: r.answers.last_played,
                device: r.answers.device,
                items: r.answers.items,
                location: r.answers.location,
                friends: r.answers.friends,
                traded_with: r.answers.traded_with,
                other_accounts: r.answers.other_accounts,
                ip: r.ip,
                uids: r.uids,
                user_agent: r.user_agent,
                platform: r.platform,
                language: r.language,
                filed_ms: r.created_ms,
            })
            .collect(),
    ))
}

async fn recovery_link(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<RecoveryLink>,
) -> Result<Json<Outcome>, StatusCode> {
    let case = i64::try_from(req.case_id).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let linked = state
        .app
        .db
        .link_recovery(req.id, case)
        .await
        .map_err(db_failed)?;
    Ok(Json(if linked {
        Outcome::ok("Linked.")
    } else {
        Outcome::refused("There's no such request.")
    }))
}

/// Records a decision for the player to see on the site. Approving issues
/// the password reset their recovery key redeems.
async fn recovery_decide(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<RecoveryDecide>,
) -> Result<Json<Outcome>, StatusCode> {
    let note = req.note.trim();
    if note.chars().count() > MAX_DECISION_NOTE {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let decided = state
        .app
        .db
        .decide_recovery(
            req.id,
            req.approve,
            note,
            &req.actor.name,
            RECOVERY_RESET_HOURS,
        )
        .await
        .map_err(db_failed)?;
    let what = if req.approve { "approved" } else { "denied" };
    Ok(Json(match decided {
        RecoveryDecided::NotFound => Outcome::refused("There's no such request."),
        // Deciding again after a step failed: the reset already stands.
        RecoveryDecided::AlreadyApproved => Outcome::ok("Already approved."),
        RecoveryDecided::Decided => {
            info!(
                "{} {} recovery request {} from the admin console",
                req.actor.name, what, req.id
            );
            Outcome::ok(if req.approve {
                "Approved: their recovery key sets a new password."
            } else {
                "Denied."
            })
        }
    }))
}

fn staff_message(m: StaffMessageRecord) -> StaffMessage {
    StaffMessage {
        id: m.id,
        player: m.player,
        subject: m.subject,
        body: m.body,
        staff_name: m.staff_name,
        sent_ms: m.sent_ms,
        read_ms: m.read_ms,
    }
}

/// Messages staff sent players, newest first.
async fn messages(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<MessageList>,
) -> Result<Json<Vec<StaffMessage>>, StatusCode> {
    let to = match req.player.as_deref().map(validate_name) {
        None => None,
        Some(Ok(name)) => Some(name.hash),
        // Nobody can be called that, so nobody was sent anything.
        Some(Err(_)) => return Ok(Json(Vec::new())),
    };
    let rows = state
        .app
        .db
        .staff_messages(to, req.before, i64::from(req.limit.clamp(1, MAX_MESSAGES)))
        .await
        .map_err(db_failed)?;
    Ok(Json(rows.into_iter().map(staff_message).collect()))
}

async fn message_get(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<MessageGet>,
) -> Result<Json<Option<StaffMessage>>, StatusCode> {
    let found = state
        .app
        .db
        .staff_message(req.id)
        .await
        .map_err(db_failed)?;
    Ok(Json(found.map(staff_message)))
}

/// A message to a player, which they read in the Message Centre.
async fn message_send(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<MessageSend>,
) -> Result<Json<MessageSent>, StatusCode> {
    let refused = |message: String| {
        Json(MessageSent {
            ok: false,
            message,
            id: None,
        })
    };
    let Ok(name) = validate_name(&req.player) else {
        return Ok(refused("There's no such player.".to_owned()));
    };
    let (subject, body) = match (subject_text(&req.subject), message_text(&req.body)) {
        (Ok(subject), Ok(body)) => (subject, body),
        (Err(error), _) | (_, Err(error)) => return Ok(refused(error)),
    };
    let sent = state
        .app
        .db
        .send_staff_message(name.hash, &req.actor.name, &subject, &body)
        .await
        .map_err(db_failed)?;
    let Some(id) = sent else {
        return Ok(refused("There's no such player.".to_owned()));
    };
    info!(
        "{} sent message {} to {} from the admin console",
        req.actor.name, id, name.display
    );
    Ok(Json(MessageSent {
        ok: true,
        message: "Sent.".to_owned(),
        id: Some(id),
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
    Ok((
        [(header::CONTENT_TYPE, media::content_type(kind.extension))],
        bytes,
    )
        .into_response())
}
