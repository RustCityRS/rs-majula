use super::access::CaseView;
use super::players::checked_name;
use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use rs_admin_api::{
    MAX_MESSAGE_CHARS, MAX_MESSAGE_SUBJECT_CHARS, MessageGet, MessageList, MessageSend,
    MessageSent, StaffMessage, path,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Messages per page.
const PAGE: u32 = 50;

fn unavailable(error: String) -> ApiError {
    ApiError::new(StatusCode::BAD_GATEWAY, format!("{error}."))
}

#[derive(Deserialize)]
pub struct ListQuery {
    player: Option<String>,
    before: Option<i64>,
}

#[derive(Serialize)]
pub struct MessagePage {
    messages: Vec<StaffMessage>,
    next: Option<i64>,
}

/// `GET /api/messages?player=&before=`: the messages staff sent players,
/// newest first, leaving out those to accounts the caller may not look
/// into (see [`CaseView::sees_subject`]).
pub async fn list(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<ListQuery>,
) -> Result<Json<MessagePage>, ApiError> {
    let player = match query.player.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(name) => Some(checked_name(name)?),
    };
    let page: Vec<StaffMessage> = app
        .worlds
        .web(
            path::MESSAGES,
            &MessageList {
                player,
                before: query.before,
                limit: PAGE,
            },
        )
        .await
        .map_err(unavailable)?;
    let next = page
        .last()
        .filter(|_| page.len() == PAGE as usize)
        .map(|m| m.id);
    let view = CaseView::new(&app, &staff);
    Ok(Json(MessagePage {
        messages: page
            .into_iter()
            .filter(|m| view.sees_subject(&m.player))
            .collect(),
        next,
    }))
}

/// `GET /api/messages/{id}`: one message, with who sent it and when it was
/// read.
pub async fn get(
    State(app): State<Arc<App>>,
    staff: Staff,
    Path(id): Path<i64>,
) -> Result<Json<StaffMessage>, ApiError> {
    let found: Option<StaffMessage> = app
        .worlds
        .web(path::MESSAGE_GET, &MessageGet { id })
        .await
        .map_err(unavailable)?;
    let view = CaseView::new(&app, &staff);
    found
        .filter(|m| view.sees_subject(&m.player))
        .map(Json)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "No such message."))
}

#[derive(Deserialize)]
pub struct SendRequest {
    player: String,
    subject: String,
    body: String,
}

/// `POST /api/messages/send`: a message to a player, which they read in the
/// website's Message Centre from "our staff". They can't reply.
pub async fn send(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<SendRequest>,
) -> Result<Json<MessageSent>, ApiError> {
    let player = checked_name(&req.player)?;
    let subject: String = req.subject.chars().filter(|c| !c.is_control()).collect();
    let subject = subject.trim();
    if subject.is_empty() || subject.chars().count() > MAX_MESSAGE_SUBJECT_CHARS {
        return Err(ApiError::bad_request(format!(
            "A subject is 1 to {MAX_MESSAGE_SUBJECT_CHARS} characters."
        )));
    }
    let body: String = req
        .body
        .replace("\r\n", "\n")
        .chars()
        .filter(|&c| c == '\n' || !c.is_control())
        .collect();
    let body = body.trim();
    if body.is_empty() || body.chars().count() > MAX_MESSAGE_CHARS {
        return Err(ApiError::bad_request(format!(
            "A message is 1 to {MAX_MESSAGE_CHARS} characters."
        )));
    }
    if !CaseView::new(&app, &staff).sees_subject(&player) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "Your own account and staff at or above your level can't be messaged from here.",
        ));
    }
    let sent: Result<MessageSent, String> = app
        .worlds
        .web(
            path::MESSAGE_SEND,
            &MessageSend {
                actor: staff.actor(),
                player: player.clone(),
                subject: subject.to_owned(),
                body: body.to_owned(),
            },
        )
        .await;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "message_sent")
            .target(&player)
            .detail(match &sent {
                Ok(MessageSent { id: Some(id), .. }) => format!("message #{id}: {subject}"),
                Ok(s) => format!("{subject}: {}", s.message),
                Err(e) => format!("{subject}: {e}"),
            })
            .ok(sent.as_ref().is_ok_and(|s| s.ok)),
    );
    sent.map(Json).map_err(unavailable)
}
