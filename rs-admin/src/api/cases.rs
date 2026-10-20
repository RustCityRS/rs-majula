use super::lookup::unavailable;
use super::players::{action_name, checked_name, moderate_one};
use super::replays::checked_id;
use crate::audit::Event;
use crate::cases::{
    Case, Change, Decision, Entry, Evidence, MAX_DECISION_NOTE, MAX_TEXT, MAX_TITLE, Priority,
    Resolution, Status, Triage,
};
use crate::clips;
use crate::http::{ApiError, App, Staff};
use crate::markdown::{self, MEDIA_PATH};
use crate::users::{Role, name_key};
use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use rs_admin_api::{
    AppealDecide, ChatLookup, ChatRecord, MediaSaved, ModAction, Outcome, ReplayGet, ReplaySummary,
    ReportLookup, ReportRecord, media, path,
};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::sync::Arc;

/// The most cases a list returns.
const MAX_CASES: usize = 500;
/// The most pieces of evidence added at once, as a new case starts or from
/// the triage queue.
const MAX_AT_ONCE: usize = 100;
/// How many are looked up at a time.
const RESOLVING_AT_ONCE: usize = 8;
/// The shortest clip.
const MIN_CLIP_MS: u32 = 1000;
const MAX_CLIP_LABEL: usize = 200;

/// A change the store refused: it's out of date, or doesn't apply.
fn refused(message: String) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, message)
}

/// Staff-written text, trimmed, with control characters dropped (newlines
/// kept where `multiline`). Refused if it's over `max`, or empty and
/// `required`.
fn text(value: &str, max: usize, multiline: bool, what: &str) -> Result<String, ApiError> {
    let value: String = value
        .trim()
        .chars()
        .filter(|&c| !c.is_control() || (multiline && c == '\n'))
        .collect();
    if value.chars().count() > max {
        return Err(ApiError::bad_request(format!(
            "{what} can be at most {max} characters."
        )));
    }
    Ok(value)
}

fn required(value: &str, max: usize, multiline: bool, what: &str) -> Result<String, ApiError> {
    let value = text(value, max, multiline, what)?;
    if value.is_empty() {
        return Err(ApiError::bad_request(format!("{what} can't be empty.")));
    }
    Ok(value)
}

/// Evidence as the page names it; the console looks it up itself, so a
/// case only ever holds what the game recorded.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvidenceRef {
    Report {
        id: i64,
    },
    Replay {
        id: String,
    },
    Chat {
        id: i64,
    },
    Clip {
        replay: String,
        from_ms: u32,
        to_ms: u32,
        #[serde(default)]
        label: String,
    },
}

fn gone(what: &str) -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, format!("That {what} is gone."))
}

async fn resolve(app: &App, item: &EvidenceRef) -> Result<Evidence, ApiError> {
    Ok(match item {
        EvidenceRef::Report { id } => {
            // The report itself is the newest one before the next.
            let rows: Vec<ReportRecord> = app
                .worlds
                .ask(
                    path::REPORTS,
                    &ReportLookup {
                        offender: None,
                        before: id.checked_add(1),
                        limit: 1,
                    },
                )
                .await
                .map_err(unavailable)?;
            let r = rows
                .into_iter()
                .find(|r| r.id == *id)
                .ok_or_else(|| gone("report"))?;
            Evidence::Report {
                id: r.id,
                reporter: r.reporter,
                offender: r.offender,
                reason: r.reason,
                node: r.node,
                x: r.x,
                z: r.z,
                level: r.level,
                at_ms: r.at_ms,
            }
        }
        EvidenceRef::Replay { id } => {
            let id = checked_id(id)?;
            let r: Option<ReplaySummary> = app
                .worlds
                .ask(path::REPLAY, &ReplayGet { id })
                .await
                .map_err(unavailable)?;
            let r = r.ok_or_else(|| gone("recording"))?;
            Evidence::Replay {
                id: r.id,
                player: r.player,
                node: r.node,
                revision: r.revision,
                started_ms: r.started_ms,
                duration_ms: r.duration_ms,
                score: r.score,
                flags: r.flags,
            }
        }
        EvidenceRef::Chat { id } => {
            let rows: Vec<ChatRecord> = app
                .worlds
                .ask(
                    path::CHAT,
                    &ChatLookup {
                        name: None,
                        node: None,
                        before: id.checked_add(1),
                        after: None,
                        limit: 1,
                    },
                )
                .await
                .map_err(unavailable)?;
            let c = rows
                .into_iter()
                .find(|c| c.id == *id)
                .ok_or_else(|| gone("message"))?;
            Evidence::Chat {
                id: c.id,
                from: c.from,
                to: c.to,
                node: c.node,
                message: c.message,
                at_ms: c.at_ms,
            }
        }
        EvidenceRef::Clip {
            replay,
            from_ms,
            to_ms,
            label,
        } => {
            let (from_ms, to_ms) = (*from_ms, *to_ms);
            let id = checked_id(replay)?;
            let label = text(label, MAX_CLIP_LABEL, false, "A clip's label")?;
            if to_ms < from_ms.saturating_add(MIN_CLIP_MS) {
                return Err(ApiError::bad_request(
                    "A clip ends at least a second after it starts.",
                ));
            }
            let (summary, file) = kept_recording(app, &id).await?;
            if clips::file_duration(&file).unwrap_or(0) < u64::from(to_ms) {
                return Err(ApiError::bad_request(
                    "That part of the session isn't stored yet: worlds store it every 30 seconds. Try again shortly.",
                ));
            }
            Evidence::Clip {
                replay: summary.id,
                from_ms,
                to_ms,
                label,
                player: summary.player,
                node: summary.node,
                revision: summary.revision,
                started_ms: summary.started_ms,
                score: summary.score,
                flags: summary.flags,
            }
        }
    })
}

/// Recording `id` and its file, kept by the console for a clip: fetched
/// from the worlds while they have it, else the copy kept earlier.
async fn kept_recording(app: &App, id: &str) -> Result<(ReplaySummary, Vec<u8>), ApiError> {
    let found: Result<Option<ReplaySummary>, String> = app
        .worlds
        .ask(path::REPLAY, &ReplayGet { id: id.to_owned() })
        .await;
    let summary = match found {
        Ok(Some(summary)) => summary,
        other => {
            if let (Some(kept), Some(file)) = (app.clips.summary(id), app.clips.file(id)) {
                return Ok((kept, file));
            }
            return Err(match other {
                Err(e) => unavailable(e),
                Ok(_) => gone("recording"),
            });
        }
    };
    let file = app
        .worlds
        .replay_file(id, &summary.revision)
        .await
        .map_err(unavailable)?;
    let clips = app.clips.clone();
    let (summary, file) =
        tokio::task::spawn_blocking(move || clips.keep(&summary, &file).map(|()| (summary, file)))
            .await
            .map_err(ApiError::internal)?
            .map_err(ApiError::internal)?;
    Ok((summary, file))
}

/// Looks up each of `items`, a few at a time, each once. Refused whole if
/// any is.
async fn resolve_all(app: &App, items: &[EvidenceRef]) -> Result<Vec<Evidence>, ApiError> {
    if items.len() > MAX_AT_ONCE {
        return Err(ApiError::bad_request(format!(
            "At most {MAX_AT_ONCE} pieces of evidence at once."
        )));
    }
    let mut found: Vec<Result<Evidence, ApiError>> = Vec::with_capacity(items.len());
    for batch in items.chunks(RESOLVING_AT_ONCE) {
        found.extend(
            futures_util::future::join_all(batch.iter().map(|item| resolve(app, item))).await,
        );
    }
    let mut out: Vec<Evidence> = Vec::with_capacity(found.len());
    for evidence in found {
        let evidence = evidence?;
        if !out.iter().any(|e| e.key() == evidence.key()) {
            out.push(evidence);
        }
    }
    Ok(out)
}

/// Who may be put on a case: any console account that can sign in, by the
/// name it has there.
fn staff_named(app: &App, name: &str) -> Result<String, ApiError> {
    app.users
        .find(name)
        .map_err(ApiError::internal)?
        .filter(|u| !u.disabled)
        .map(|u| u.name)
        .ok_or_else(|| ApiError::bad_request(format!("{name} has no console account.")))
}

/// Puts `name` on `case`; the first one on an open case starts it.
fn assign(case: &Case, name: String) -> Vec<Change> {
    let mut changes = vec![Change::Assigned { name }];
    if case.status == Status::Open {
        changes.push(Change::Status {
            status: Status::Investigating,
        });
    }
    changes
}

#[derive(Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Show {
    /// Not closed.
    #[default]
    Open,
    Waiting,
    Closed,
    All,
}

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    status: Show,
    /// Only those the caller is on.
    #[serde(default)]
    mine: bool,
    /// Only open ones nobody is on.
    #[serde(default)]
    unassigned: bool,
    player: Option<String>,
    q: Option<String>,
}

/// `GET /api/cases?status=&mine=&unassigned=&player=&q=`: open cases most
/// urgent then most recently changed first, closed ones most recently
/// closed first.
pub async fn list(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<ListQuery>,
) -> Json<Vec<Case>> {
    let player = query
        .player
        .as_deref()
        .map(name_key)
        .filter(|p| !p.is_empty());
    let needle = query
        .q
        .as_deref()
        .map(|q| q.trim().to_lowercase())
        .filter(|q| !q.is_empty());
    let mut cases = app.cases.all(|c| {
        let shown = match query.status {
            Show::Open => c.is_open(),
            Show::Waiting => c.status == Status::Waiting,
            Show::Closed => !c.is_open(),
            Show::All => true,
        };
        shown
            && (!query.mine || c.has(staff.name()))
            && (!query.unassigned || (c.is_open() && c.assignees.is_empty()))
            && player.as_ref().is_none_or(|p| name_key(&c.player) == *p)
            && needle.as_ref().is_none_or(|n| {
                c.title.to_lowercase().contains(n)
                    || c.player.to_lowercase().contains(n)
                    || format!("#{}", c.id) == *n
                    || c.id.to_string() == *n
            })
    });
    cases.sort_by_key(|c| {
        (
            !c.is_open(),
            Reverse(if c.is_open() {
                c.priority
            } else {
                Priority::Low
            }),
            Reverse(c.closed_ms.unwrap_or(c.updated_ms)),
        )
    });
    cases.truncate(MAX_CASES);
    Json(cases)
}

#[derive(Serialize)]
pub struct Summary {
    open: usize,
    yours: usize,
    unassigned: usize,
    /// Open at high priority or above.
    urgent: usize,
    waiting: usize,
}

/// `GET /api/cases/summary`: counts for the nav and the center's header.
pub async fn summary(State(app): State<Arc<App>>, staff: Staff) -> Json<Summary> {
    let open = app.cases.all(Case::is_open);
    Json(Summary {
        open: open.len(),
        yours: open.iter().filter(|c| c.has(staff.name())).count(),
        unassigned: open.iter().filter(|c| c.assignees.is_empty()).count(),
        urgent: open.iter().filter(|c| c.priority >= Priority::High).count(),
        waiting: open.iter().filter(|c| c.status == Status::Waiting).count(),
    })
}

#[derive(Serialize)]
pub struct StaffName {
    name: String,
    role: Role,
}

/// `GET /api/cases/staff`: who can be put on a case.
pub async fn staff(
    State(app): State<Arc<App>>,
    _staff: Staff,
) -> Result<Json<Vec<StaffName>>, ApiError> {
    let mut names: Vec<StaffName> = app
        .users
        .all()
        .map_err(ApiError::internal)?
        .into_iter()
        .filter(|u| !u.disabled)
        .map(|u| StaffName {
            name: u.name,
            role: u.role,
        })
        .collect();
    names.sort_by_key(|n| name_key(&n.name));
    Ok(Json(names))
}

#[derive(Deserialize)]
pub struct NewCase {
    player: String,
    title: String,
    #[serde(default)]
    priority: Priority,
    /// The first comment.
    #[serde(default)]
    note: String,
    #[serde(default)]
    evidence: Vec<EvidenceRef>,
    /// Put the one opening it on it.
    #[serde(default)]
    join: bool,
}

/// `POST /api/cases`: answered with the new case.
pub async fn create(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<NewCase>,
) -> Result<Json<Case>, ApiError> {
    let player = checked_name(&req.player)?;
    let title = required(&req.title, MAX_TITLE, false, "A title")?;
    let note = text(&req.note, MAX_TEXT, true, "A note")?;
    let mut then: Vec<Change> = resolve_all(&app, &req.evidence)
        .await?
        .into_iter()
        .map(|evidence| Change::EvidenceAdded { evidence })
        .collect();
    if !note.is_empty() {
        then.push(Change::Comment { text: note });
    }
    if req.join {
        then.push(Change::Assigned {
            name: staff.name().to_owned(),
        });
        then.push(Change::Status {
            status: Status::Investigating,
        });
    }
    let case = app
        .cases
        .create(staff.name(), title, player, req.priority, then)
        .map_err(refused)?;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "case_opened")
            .target(&case.player)
            .detail(format!("#{}: {}", case.id, case.title)),
    );
    Ok(Json(case))
}

#[derive(Serialize)]
pub struct EntryView {
    #[serde(flatten)]
    entry: Entry,
    /// A comment's or closing note's Markdown, rendered.
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
}

#[derive(Serialize)]
pub struct CaseDetail {
    #[serde(flatten)]
    case: Case,
    timeline: Vec<EntryView>,
    /// Other staff looking at it now.
    viewing: Vec<String>,
}

fn detail(app: &App, staff: &Staff, mut case: Case) -> CaseDetail {
    let timeline = std::mem::take(&mut case.timeline)
        .into_iter()
        .map(|entry| {
            let html = match &entry.change {
                Change::Comment { text } => Some(markdown::render(text)),
                Change::Closed { note, .. } if !note.is_empty() => Some(markdown::render(note)),
                _ => None,
            };
            EntryView { entry, html }
        })
        .collect();
    CaseDetail {
        timeline,
        viewing: app.cases.viewing(case.id, staff.name()),
        case,
    }
}

/// `GET /api/cases/{id}`: the case and its timeline. Looking counts as
/// viewing it, for the others on it.
pub async fn get(
    State(app): State<Arc<App>>,
    staff: Staff,
    Path(id): Path<u64>,
) -> Result<Json<CaseDetail>, ApiError> {
    let case = app
        .cases
        .get(id)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, format!("There's no case #{id}.")))?;
    Ok(Json(detail(&app, &staff, case)))
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    Comment {
        text: String,
    },
    Join,
    Leave,
    Assign {
        name: String,
    },
    Unassign {
        name: String,
    },
    Status {
        status: Status,
    },
    Priority {
        priority: Priority,
    },
    Retitle {
        title: String,
    },
    AddEvidence {
        item: EvidenceRef,
    },
    /// Adds what of `items` isn't in the case already, together.
    AddEvidenceMany {
        items: Vec<EvidenceRef>,
    },
    RemoveEvidence {
        key: String,
    },
    Close {
        resolution: Resolution,
        #[serde(default)]
        note: String,
    },
    Reopen,
    /// Decides the case's appeal, tells the player, and closes the case;
    /// overturning lifts the mute or ban first.
    DecideAppeal {
        decision: Decision,
        #[serde(default)]
        note: String,
    },
}

/// `POST /api/cases/{id}`: one change, answered with the case as it is now.
pub async fn change(
    State(app): State<Arc<App>>,
    staff: Staff,
    Path(id): Path<u64>,
    Json(op): Json<Op>,
) -> Result<Json<CaseDetail>, ApiError> {
    let me = staff.name().to_owned();
    let op = match op {
        Op::DecideAppeal { decision, note } => {
            return decide_appeal(&app, &staff, id, decision, &note).await;
        }
        op => op,
    };
    // What needs the game database or the accounts, before the store's lock.
    let evidence = match &op {
        Op::AddEvidence { item } => Some(resolve(&app, item).await?),
        _ => None,
    };
    let many = match &op {
        Op::AddEvidenceMany { items } => resolve_all(&app, items).await?,
        _ => Vec::new(),
    };
    let assignee = match &op {
        Op::Assign { name } => Some(staff_named(&app, name)?),
        _ => None,
    };
    let (audited, changes): (Option<(&str, String)>, _) = match op {
        Op::Comment { text: t } => {
            let text = required(&t, MAX_TEXT, true, "A comment")?;
            (None, vec![Change::Comment { text }])
        }
        Op::Retitle { title } => {
            let title = required(&title, MAX_TITLE, false, "A title")?;
            (None, vec![Change::Retitled { title }])
        }
        Op::Close { resolution, note } => {
            let note = text(&note, MAX_TEXT, true, "A note")?;
            let label = match resolution {
                Resolution::Actioned => "actioned",
                Resolution::NoAction => "no action",
                Resolution::Duplicate => "duplicate",
            };
            (
                Some(("case_closed", label.to_owned())),
                vec![Change::Closed { resolution, note }],
            )
        }
        Op::Reopen => (
            Some(("case_reopened", String::new())),
            vec![Change::Reopened],
        ),
        Op::Status { status } => (None, vec![Change::Status { status }]),
        Op::Priority { priority } => (None, vec![Change::Priority { priority }]),
        Op::RemoveEvidence { key } => (None, vec![Change::EvidenceRemoved { key }]),
        Op::AddEvidence { .. } => (
            None,
            vec![Change::EvidenceAdded {
                evidence: evidence.expect("resolved above"),
            }],
        ),
        Op::DecideAppeal { .. } => unreachable!("decided above"),
        // These depend on what's in it or who's on it now, so they're worked
        // out under the store's lock.
        Op::AddEvidenceMany { .. }
        | Op::Join
        | Op::Leave
        | Op::Assign { .. }
        | Op::Unassign { .. } => {
            let case = app
                .cases
                .change(&me, id, |case| {
                    Ok(match op {
                        Op::AddEvidenceMany { .. } => {
                            let added: Vec<Change> = many
                                .into_iter()
                                .filter(|e| !case.evidence.iter().any(|c| c.key() == e.key()))
                                .map(|evidence| Change::EvidenceAdded { evidence })
                                .collect();
                            if added.is_empty() {
                                return Err("All of that is in the case already.".to_owned());
                            }
                            added
                        }
                        Op::Join => assign(case, me.clone()),
                        Op::Assign { .. } => assign(case, assignee.expect("checked above")),
                        Op::Leave => vec![Change::Unassigned { name: me.clone() }],
                        Op::Unassign { name } => {
                            let key = name_key(&name);
                            let name = case
                                .assignees
                                .iter()
                                .find(|a| name_key(a) == key)
                                .cloned()
                                .ok_or_else(|| format!("{name} isn't on the case."))?;
                            vec![Change::Unassigned { name }]
                        }
                        _ => unreachable!(),
                    })
                })
                .map_err(refused)?;
            return Ok(Json(detail(&app, &staff, case)));
        }
    };
    let case = app
        .cases
        .change(&me, id, |_| Ok(changes))
        .map_err(refused)?;
    if let Some((action, outcome)) = audited {
        let detail = if outcome.is_empty() {
            format!("#{}: {}", case.id, case.title)
        } else {
            format!("#{}: {} ({outcome})", case.id, case.title)
        };
        app.audit.record(
            Event::new(Some(&me), staff.client.ip, action)
                .target(&case.player)
                .detail(detail),
        );
    }
    Ok(Json(detail(&app, &staff, case)))
}

/// Decides case `id`'s appeal: lifts the sanction if it's overturned, has
/// the website show the player the decision, then records it and closes the
/// case. Whoever issued the sanction can't (the case log refuses them too).
/// Each step that fails stops it there, saying what did happen, and deciding
/// again picks up where it stopped: lifting twice or telling twice is fine.
async fn decide_appeal(
    app: &App,
    staff: &Staff,
    id: u64,
    decision: Decision,
    note: &str,
) -> Result<Json<CaseDetail>, ApiError> {
    let me = staff.name().to_owned();
    let note = text(note, MAX_DECISION_NOTE, true, "The note to the player")?;
    let case = app
        .cases
        .get(id)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, format!("There's no case #{id}.")))?;
    let appeal = case
        .appeal
        .as_ref()
        .map(|a| a.appeal.clone())
        .ok_or_else(|| ApiError::bad_request("This case isn't an appeal."))?;
    if !case.is_open() {
        return Err(ApiError::bad_request(
            "The case is closed; reopen it to decide again.",
        ));
    }
    if appeal
        .issued_by
        .as_deref()
        .is_some_and(|by| name_key(by) == name_key(&me))
    {
        return Err(refused(
            "You issued this sanction, so someone else must decide the appeal.".to_owned(),
        ));
    }

    let mut changes = vec![Change::AppealDecided {
        decision,
        note: note.clone(),
    }];
    let what = if appeal.sanction == "mute" {
        "mute"
    } else {
        "ban"
    };
    if decision == Decision::Overturned {
        let action = if what == "mute" {
            ModAction::Unmute
        } else {
            ModAction::Unban
        };
        let reason = format!("Appeal #{} overturned (case #{id})", appeal.id);
        let (ok, message) = moderate_one(app, staff, &case.player, action, None, &reason).await;
        app.audit.record(
            Event::new(Some(&me), staff.client.ip, action_name(action))
                .target(&case.player)
                .detail(format!("{message} Reason: {reason}"))
                .ok(ok),
        );
        if !ok {
            return Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                format!("Couldn't lift the {what}: {message} Nothing was decided."),
            ));
        }
        changes.push(Change::Action {
            action: action_name(action).to_owned(),
            name: case.player.clone(),
            hours: None,
            ok,
            message,
            reason,
        });
    }

    let told: Result<Outcome, String> = app
        .worlds
        .web(
            path::APPEAL_DECIDE,
            &AppealDecide {
                actor: staff.actor(),
                id: appeal.id,
                decision: decision.name().to_owned(),
                note: note.clone(),
            },
        )
        .await
        .and_then(|o: Outcome| if o.ok { Ok(o) } else { Err(o.message) });
    if let Err(e) = told {
        let lifted = if decision == Decision::Overturned {
            // The lift happened: the case keeps it, so the timeline is true.
            let lift = changes.split_off(1);
            let _ = app.cases.change(&me, id, |_| Ok(lift));
            format!("The {what} was lifted, but the")
        } else {
            "The".to_owned()
        };
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            format!("{lifted} website couldn't record the decision ({e}). Decide again to retry."),
        ));
    }

    changes.push(Change::Closed {
        resolution: match decision {
            Decision::Overturned => Resolution::Actioned,
            Decision::Upheld => Resolution::NoAction,
        },
        note: String::new(),
    });
    let case = app
        .cases
        .change(&me, id, |_| Ok(changes))
        .map_err(refused)?;
    app.audit.record(
        Event::new(Some(&me), staff.client.ip, "appeal_decided")
            .target(&case.player)
            .detail(format!(
                "{what} appeal #{} (case #{id}): {}",
                appeal.id,
                decision.name()
            )),
    );
    Ok(Json(detail(app, staff, case)))
}

#[derive(Deserialize)]
pub struct PreviewRequest {
    markdown: String,
}

#[derive(Serialize)]
pub struct Rendered {
    html: String,
}

/// `POST /api/cases/preview`: the HTML a comment's Markdown shows as.
pub async fn preview(
    _staff: Staff,
    Json(req): Json<PreviewRequest>,
) -> Result<Json<Rendered>, ApiError> {
    let markdown = text(&req.markdown, MAX_TEXT, true, "A comment")?;
    Ok(Json(Rendered {
        html: markdown::render(&markdown),
    }))
}

/// `POST /api/cases/media`: an image or video for a comment, as the raw
/// body, told apart by its content and kept in the console's data directory
/// for good. Answered with the address to embed it by.
pub async fn upload(
    State(app): State<Arc<App>>,
    _staff: Staff,
    body: Bytes,
) -> Result<Json<MediaSaved>, ApiError> {
    let found = media::sniff(&body).ok_or_else(|| {
        ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "That isn't a PNG, JPEG, GIF, WebP, MP4 or WebM file.",
        )
    })?;
    let kind = found.kind;
    let dir = app.media_dir.clone();
    let name = tokio::task::spawn_blocking(move || media::store(&dir, &body, &found))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::internal)?;
    Ok(Json(MediaSaved {
        url: format!("{MEDIA_PATH}{name}"),
        kind: kind.to_owned(),
    }))
}

/// `GET /api/cases/media/{name}`: an upload, to staff only. Ranges are
/// served, so videos can seek; a name never changes what it is, so the
/// browser may keep it.
pub async fn media(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Path(name): Path<String>,
    req: Request,
) -> Result<Response, ApiError> {
    let missing = || ApiError::new(StatusCode::NOT_FOUND, "No such file.");
    if !media::valid_name(&name) {
        return Err(missing());
    }
    let extension = name.rsplit('.').next().unwrap_or_default();
    let mut response = tower_http::services::ServeFile::new(app.media_dir.join(&name))
        .try_call(req)
        .await
        .map_err(|_| missing())?
        .map(Body::new);
    if response.status() == StatusCode::NOT_FOUND {
        return Err(missing());
    }
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(media::content_type(extension)),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=31536000, immutable"),
    );
    Ok(response.into_response())
}

/// A report or recording's key in the triage queue, if it's one.
fn triage_key(key: &str) -> Option<&str> {
    let (kind, id) = key.split_once(':')?;
    (matches!(kind, "report" | "replay")
        && !id.is_empty()
        && id.len() <= 20
        && id.bytes().all(|b| b.is_ascii_digit()))
    .then_some(key)
}

#[derive(Deserialize)]
pub struct TriageRequest {
    key: String,
    dismissed: bool,
}

/// `POST /api/triage`: sets a report or flagged session aside from the
/// queue without opening a case, or puts it back.
pub async fn triage(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<TriageRequest>,
) -> Result<Json<Triage>, ApiError> {
    let key = triage_key(&req.key)
        .ok_or_else(|| ApiError::bad_request("That can't be triaged."))?
        .to_owned();
    app.cases
        .triage(staff.name(), key.clone(), req.dismissed)
        .map_err(refused)?;
    Ok(Json(
        app.cases
            .triage_of(std::slice::from_ref(&key))
            .remove(&key)
            .unwrap_or_default(),
    ))
}

#[derive(Deserialize)]
pub struct BulkTriageRequest {
    keys: Vec<String>,
    dismissed: bool,
}

#[derive(Serialize)]
pub struct BulkTriaged {
    /// How many changed: those already that way are left as they are.
    changed: usize,
}

/// `POST /api/triage/bulk`: [`triage`] for many reports or sessions at once.
pub async fn triage_bulk(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<BulkTriageRequest>,
) -> Result<Json<BulkTriaged>, ApiError> {
    if req.keys.is_empty() || req.keys.len() > MAX_AT_ONCE {
        return Err(ApiError::bad_request(format!(
            "Pick 1 to {MAX_AT_ONCE} at once."
        )));
    }
    let keys = req
        .keys
        .iter()
        .map(|k| triage_key(k).map(str::to_owned))
        .collect::<Option<Vec<String>>>()
        .ok_or_else(|| ApiError::bad_request("That can't be triaged."))?;
    let changed = app
        .cases
        .triage_many(staff.name(), keys, req.dismissed)
        .map_err(refused)?;
    Ok(Json(BulkTriaged { changed }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_cleaned_and_bounded() {
        assert_eq!(text("  a\u{7}b\nc ", 10, true, "X").unwrap(), "ab\nc");
        assert_eq!(text("a\nb", 10, false, "X").unwrap(), "ab");
        assert!(text("abcdef", 5, false, "X").is_err());
        assert!(required("  ", 5, false, "X").is_err());
    }

    #[test]
    fn only_reports_and_recordings_are_triaged() {
        assert!(triage_key("report:12").is_some());
        assert!(triage_key("replay:9007199254740993").is_some());
        assert!(triage_key("chat:12").is_none());
        assert!(triage_key("report:").is_none());
        assert!(triage_key("report:1a").is_none());
    }
}
