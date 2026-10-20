use super::players::checked_name;
use crate::audit::Event;
use crate::cases::Triage;
use crate::http::{ApiError, App, Staff};
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use rs_admin_api::{
    ChatLookup, ChatRecord, ModerationLookup, ModerationRecord, PlayerLookup, PlayerRecord,
    ReportLookup, ReportRecord, Sanction, SanctionsLookup, path,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Rows per page.
const PAGE: u32 = 50;
/// The most mutes and bans listed at once.
const MAX_SANCTIONS: u32 = 200;
/// How many pages a triage list reads through for ones still to triage.
pub(super) const TRIAGE_PAGES: usize = 10;

pub(super) fn unavailable(error: String) -> ApiError {
    ApiError::new(StatusCode::BAD_GATEWAY, format!("{error}."))
}

/// The world id of node `node`, if the console manages it.
fn world_of(app: &App, node: u8) -> Option<u16> {
    app.worlds.get(node).map(|w| w.id)
}

#[derive(Deserialize)]
pub struct NameQuery {
    name: String,
}

#[derive(Serialize)]
pub struct PlayerView {
    #[serde(flatten)]
    record: PlayerRecord,
    online_world: Option<u16>,
    last_world: Option<u16>,
}

/// `GET /api/lookup/player?name=`: `null` when no such player is registered.
pub async fn player(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Query(query): Query<NameQuery>,
) -> Result<Json<Option<PlayerView>>, ApiError> {
    let name = checked_name(&query.name)?;
    let record: Option<PlayerRecord> = app
        .worlds
        .ask(path::PLAYER, &PlayerLookup { name })
        .await
        .map_err(unavailable)?;
    Ok(Json(record.map(|record| PlayerView {
        online_world: record.online_node.and_then(|n| world_of(&app, n)),
        last_world: record.last_node.and_then(|n| world_of(&app, n)),
        record,
    })))
}

#[derive(Deserialize)]
pub struct PageQuery {
    name: Option<String>,
    before: Option<i64>,
}

#[derive(Serialize)]
pub struct ModerationView {
    #[serde(flatten)]
    record: ModerationRecord,
    world: Option<u16>,
}

/// `GET /api/lookup/moderation?name=&before=`: moderation commands newest
/// first, everyone's or those on `name`.
pub async fn moderation(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Query(query): Query<PageQuery>,
) -> Result<Json<Vec<ModerationView>>, ApiError> {
    let name = query.name.as_deref().map(checked_name).transpose()?;
    let records: Vec<ModerationRecord> = app
        .worlds
        .ask(
            path::MODERATION,
            &ModerationLookup {
                name,
                before: query.before,
                limit: PAGE,
            },
        )
        .await
        .map_err(unavailable)?;
    Ok(Json(
        records
            .into_iter()
            .map(|record| ModerationView {
                world: world_of(&app, record.node),
                record,
            })
            .collect(),
    ))
}

/// `GET /api/lookup/sanctions`: everyone muted or banned now.
pub async fn sanctions(
    State(app): State<Arc<App>>,
    _staff: Staff,
) -> Result<Json<Vec<Sanction>>, ApiError> {
    app.worlds
        .ask(
            path::SANCTIONS,
            &SanctionsLookup {
                limit: MAX_SANCTIONS,
            },
        )
        .await
        .map(Json)
        .map_err(unavailable)
}

#[derive(Deserialize)]
pub struct ChatQuery {
    name: Option<String>,
    /// A world's node.
    node: Option<u8>,
    before: Option<i64>,
}

#[derive(Serialize)]
pub struct ChatView {
    #[serde(flatten)]
    record: ChatRecord,
    world: Option<u16>,
}

/// `GET /api/lookup/chat?name=&node=&before=`: chat newest first. Each new
/// search (not its "load older" pages) is audited, since this includes
/// private messages.
pub async fn chat(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<ChatQuery>,
) -> Result<Json<Vec<ChatView>>, ApiError> {
    let name = query
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(checked_name)
        .transpose()?;
    let world = match query.node {
        Some(node) => {
            Some(world_of(&app, node).ok_or_else(|| ApiError::bad_request("No such world."))?)
        }
        None => None,
    };
    let records: Vec<ChatRecord> = app
        .worlds
        .ask(
            path::CHAT,
            &ChatLookup {
                name: name.clone(),
                node: query.node,
                before: query.before,
                limit: PAGE,
            },
        )
        .await
        .map_err(unavailable)?;
    if query.before.is_none() {
        let target = match (&name, world) {
            (Some(name), Some(world)) => format!("{name} on world {world}"),
            (Some(name), None) => name.clone(),
            (None, Some(world)) => format!("world {world}"),
            (None, None) => "every world".to_owned(),
        };
        app.audit
            .record(Event::new(Some(staff.name()), staff.client.ip, "chat_viewed").target(target));
    }
    Ok(Json(
        records
            .into_iter()
            .map(|record| ChatView {
                world: world_of(&app, record.node),
                record,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct ReportQuery {
    name: Option<String>,
    before: Option<i64>,
    /// Only those in no case and not dismissed.
    #[serde(default)]
    untriaged: bool,
}

#[derive(Serialize)]
pub struct ReportView {
    #[serde(flatten)]
    record: ReportRecord,
    world: Option<u16>,
    #[serde(flatten)]
    triage: Triage,
}

/// `GET /api/lookup/reports?name=&before=&untriaged=`: abuse reports newest
/// first, everyone's or those against `name`, with the cases each is in.
pub async fn reports(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Query(query): Query<ReportQuery>,
) -> Result<Json<Vec<ReportView>>, ApiError> {
    let offender = query
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(checked_name)
        .transpose()?;
    let mut views = Vec::new();
    let mut before = query.before;
    for _ in 0..if query.untriaged { TRIAGE_PAGES } else { 1 } {
        let rows: Vec<ReportRecord> = app
            .worlds
            .ask(
                path::REPORTS,
                &ReportLookup {
                    offender: offender.clone(),
                    before,
                    limit: PAGE,
                },
            )
            .await
            .map_err(unavailable)?;
        let full = rows.len() == PAGE as usize;
        before = rows.last().map(|r| r.id);
        let keys: Vec<String> = rows.iter().map(|r| format!("report:{}", r.id)).collect();
        let mut triage = app.cases.triage_of(&keys);
        views.extend(
            rows.into_iter()
                .zip(keys)
                .map(|(record, key)| ReportView {
                    world: world_of(&app, record.node),
                    triage: triage.remove(&key).unwrap_or_default(),
                    record,
                })
                .filter(|v| {
                    !query.untriaged || (v.triage.cases.is_empty() && v.triage.dismissed.is_none())
                }),
        );
        if !full || views.len() >= PAGE as usize {
            break;
        }
    }
    Ok(Json(views))
}
