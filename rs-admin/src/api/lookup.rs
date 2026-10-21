use super::access::{ceiling, check_player, hides_self, may_see};
use super::players::checked_name;
use crate::audit::Event;
use crate::cases::{Triage, evasion_key};
use crate::http::{ApiError, App, Staff};
use crate::users::name_key;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use rs_admin_api::{
    ActivityHour, ActivityLookup, ChainLink, ChatLookup, ChatRecord, Evasion, EvasionLookup,
    LinkChainLookup, Links, LinksLookup, ModerationLookup, ModerationRecord, PlayerLookup,
    PlayerRecord, ProgressLookup, ProgressPoint, ReportLookup, ReportRecord, Sanction,
    SanctionsLookup, TimelineCursor, TimelineEntry, TimelineLookup, path,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

#[derive(Serialize)]
pub struct ModerationView {
    #[serde(flatten)]
    record: ModerationRecord,
    world: Option<u16>,
}

#[derive(Deserialize)]
pub struct ModerationQuery {
    name: Option<String>,
    moderator: Option<String>,
    before: Option<i64>,
}

/// `GET /api/lookup/moderation?name=&moderator=&before=`: moderation
/// commands newest first, everyone's or only those on `name` and by
/// `moderator`.
pub async fn moderation(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Query(query): Query<ModerationQuery>,
) -> Result<Json<Vec<ModerationView>>, ApiError> {
    let name = query.name.as_deref().map(checked_name).transpose()?;
    let moderator = query.moderator.as_deref().map(checked_name).transpose()?;
    let records: Vec<ModerationRecord> = app
        .worlds
        .ask(
            path::MODERATION,
            &ModerationLookup {
                name,
                moderator,
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
    /// Only messages newer than this one, oldest first: a live feed.
    after: Option<i64>,
}

#[derive(Serialize)]
pub struct ChatView {
    #[serde(flatten)]
    record: ChatRecord,
    world: Option<u16>,
}

/// `GET /api/lookup/chat?name=&node=&before=&after=`: chat newest first, or
/// with `after` what's come since, oldest first. Each new search (not its
/// "load older" pages, nor a live feed's next messages) is audited, since
/// this includes private messages.
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
    if let Some(name) = &name {
        check_player(&app, &staff, name).await?;
    }
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
                after: query.after,
                max_staff_level: ceiling(&staff),
                limit: PAGE,
            },
        )
        .await
        .map_err(unavailable)?;
    if query.before.is_none() && query.after.is_none() {
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
    staff: Staff,
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
                .filter(|(record, _)| !hides_self(&staff, &record.offender))
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

/// Whether `staff` may see players' addresses; others see [`address_label`]s.
pub(super) fn sees_addresses(staff: &Staff) -> bool {
    staff.require_admin().is_ok()
}

/// A stand-in for an address: the same address gets the same label all
/// through this console's run, so moderators can match addresses up without
/// seeing them.
pub(super) fn address_label(app: &App, ip: &str) -> String {
    let digest = Sha256::digest(format!("{}:{ip}", app.address_salt).as_bytes());
    let hex: String = digest[..3].iter().map(|b| format!("{b:02x}")).collect();
    format!("addr-{hex}")
}

#[derive(Deserialize)]
pub struct TimelineQuery {
    name: String,
    /// Comma-separated; every kind when missing.
    kinds: Option<String>,
    before_ms: Option<i64>,
    before_kind: Option<String>,
    before_id: Option<i64>,
}

#[derive(Serialize)]
pub struct TimelineView {
    #[serde(flatten)]
    entry: TimelineEntry,
    world: Option<u16>,
}

/// Rows per timeline page.
const TIMELINE_PAGE: u32 = 100;

/// `GET /api/lookup/timeline?name=&kinds=&before_ms=&before_kind=&before_id=`:
/// everything on record about a player, newest first. The first page is
/// audited, since it includes private messages and (for admins) addresses.
pub async fn timeline(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<Vec<TimelineView>>, ApiError> {
    let name = checked_name(&query.name)?;
    check_player(&app, &staff, &name).await?;
    let kinds: Vec<String> = match &query.kinds {
        Some(kinds) => kinds
            .split(',')
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(str::to_owned)
            .collect(),
        None => [
            "login",
            "session",
            "wealth",
            "chat",
            "moderation",
            "reported",
            "report",
            "replay",
        ]
        .map(str::to_owned)
        .to_vec(),
    };
    let before = match (query.before_ms, query.before_kind, query.before_id) {
        (Some(at_ms), Some(kind), Some(id)) => Some(TimelineCursor { at_ms, kind, id }),
        _ => None,
    };
    let first = before.is_none();
    let entries: Vec<TimelineEntry> = app
        .worlds
        .ask(
            path::TIMELINE,
            &TimelineLookup {
                name: name.clone(),
                kinds,
                before,
                limit: TIMELINE_PAGE,
            },
        )
        .await
        .map_err(unavailable)?;
    if first {
        app.audit.record(
            Event::new(Some(staff.name()), staff.client.ip, "timeline_viewed").target(&name),
        );
    }
    let sees = sees_addresses(&staff);
    Ok(Json(
        entries
            .into_iter()
            .map(|mut entry| {
                if entry.kind == "login" && !sees {
                    entry.text = entry.text.map(|ip| address_label(&app, &ip));
                }
                TimelineView {
                    world: entry.node.and_then(|n| world_of(&app, n)),
                    entry,
                }
            })
            .collect(),
    ))
}

#[derive(Serialize)]
pub struct LinksView {
    #[serde(flatten)]
    links: Links,
    /// Whether the addresses are real, or labels standing in for them.
    addresses_shown: bool,
}

/// `GET /api/lookup/links?name=`: the addresses a player logs in from and
/// the accounts that share them. Audited when it shows real addresses.
pub async fn links(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<NameQuery>,
) -> Result<Json<LinksView>, ApiError> {
    let name = checked_name(&query.name)?;
    check_player(&app, &staff, &name).await?;
    let mut links: Links = app
        .worlds
        .ask(
            path::LINKS,
            &LinksLookup {
                name: name.clone(),
                limit: 100,
            },
        )
        .await
        .map_err(unavailable)?;
    let sees = sees_addresses(&staff);
    if sees {
        app.audit.record(
            Event::new(Some(staff.name()), staff.client.ip, "addresses_viewed").target(&name),
        );
    } else {
        for address in &mut links.addresses {
            address.ip = address_label(&app, &address.ip);
        }
        for account in &mut links.accounts {
            for ip in &mut account.shared {
                *ip = address_label(&app, ip);
            }
        }
        // An install's id names a device as surely as an address does.
        for device in &mut links.devices {
            for ip in &mut device.ips {
                *ip = address_label(&app, ip);
            }
            if let Some(uid) = &mut device.uid {
                *uid = address_label(&app, &format!("uid {uid}")).replacen("addr-", "id-", 1);
            }
        }
    }
    Ok(Json(LinksView {
        links,
        addresses_shown: sees,
    }))
}

#[derive(Deserialize)]
pub struct ChainQuery {
    name: String,
    depth: Option<u8>,
}

/// `GET /api/lookup/link-chain?name=&depth=`: every account shared browsers
/// and addresses reach from `name`, `depth` links out at most (2 by
/// default). Staff above the caller are left out, with everyone reached only
/// through them, so the chain shows nothing about them.
pub async fn link_chain(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<ChainQuery>,
) -> Result<Json<Vec<ChainLink>>, ApiError> {
    let name = checked_name(&query.name)?;
    check_player(&app, &staff, &name).await?;
    let chain: Vec<ChainLink> = app
        .worlds
        .ask(
            path::LINK_CHAIN,
            &LinkChainLookup {
                name: name.clone(),
                depth: query.depth.unwrap_or(2),
                limit: 200,
            },
        )
        .await
        .map_err(unavailable)?;
    // Nearest first, so whoever an account was reached through comes before it.
    let mut hidden = std::collections::HashSet::new();
    let shown = chain
        .into_iter()
        .filter(|c| {
            let hide = !may_see(&staff, c.staff_level) || hidden.contains(&name_key(&c.via));
            if hide {
                hidden.insert(name_key(&c.name));
            }
            !hide
        })
        .collect();
    Ok(Json(shown))
}

#[derive(Deserialize)]
pub struct EvasionQuery {
    days: Option<u32>,
}

#[derive(Serialize)]
pub struct EvasionRow {
    #[serde(flatten)]
    evasion: Evasion,
    /// The cases it's evidence in.
    cases: Vec<u64>,
}

/// `GET /api/lookup/evasion?days=`: accounts that logged in with a banned
/// account's browser, or from its address, since its ban, over the last
/// `days` (14 by default).
pub async fn evasion(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<EvasionQuery>,
) -> Result<Json<Vec<EvasionRow>>, ApiError> {
    let rows: Vec<Evasion> = app
        .worlds
        .ask(
            path::EVASION,
            &EvasionLookup {
                days: query.days.unwrap_or(14),
                max_staff_level: ceiling(&staff),
                limit: 100,
            },
        )
        .await
        .map_err(unavailable)?;
    let keys: Vec<String> = rows
        .iter()
        .map(|r| evasion_key(&r.name, &r.banned))
        .collect();
    let mut triage = app.cases.triage_of(&keys);
    Ok(Json(
        rows.into_iter()
            .zip(keys)
            .filter(|(evasion, _)| !hides_self(&staff, &evasion.name))
            .map(|(evasion, key)| EvasionRow {
                evasion,
                cases: triage.remove(&key).map(|t| t.cases).unwrap_or_default(),
            })
            .collect(),
    ))
}

/// How many save points the progress charts reach back over.
const PROGRESS_POINTS: u32 = 500;

/// `GET /api/lookup/progress?name=`: a player's XP and wealth at each save
/// point kept, oldest first, ending with their save as it stands.
pub async fn progress(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<NameQuery>,
) -> Result<Json<Vec<ProgressPoint>>, ApiError> {
    let name = checked_name(&query.name)?;
    check_player(&app, &staff, &name).await?;
    app.worlds
        .ask(
            path::PROGRESS,
            &ProgressLookup {
                name,
                limit: PROGRESS_POINTS,
            },
        )
        .await
        .map(Json)
        .map_err(unavailable)
}

#[derive(Deserialize)]
pub struct ActivityQuery {
    name: String,
    days: Option<u32>,
}

/// `GET /api/lookup/activity?name=&days=`: how many events a player's
/// scripts logged in each hour of the last `days` (90 by default).
pub async fn activity(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<ActivityQuery>,
) -> Result<Json<Vec<ActivityHour>>, ApiError> {
    let name = checked_name(&query.name)?;
    check_player(&app, &staff, &name).await?;
    app.worlds
        .ask(
            path::ACTIVITY,
            &ActivityLookup {
                name,
                days: query.days.unwrap_or(90).clamp(1, 365),
            },
        )
        .await
        .map(Json)
        .map_err(unavailable)
}
