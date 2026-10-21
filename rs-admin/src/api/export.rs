use super::AuditFilter;
use super::access::{AuditView, ceiling, check_player};
use super::lookup::{address_label, sees_addresses, unavailable};
use super::players::checked_name;
use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use crate::users::now_ms;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use rs_admin_api::{
    ChatLookup, ChatRecord, TimelineCursor, TimelineEntry, TimelineLookup, iso_time, path,
};
use serde::Deserialize;
use std::sync::Arc;

/// The most rows an export holds.
const MAX_AUDIT_ROWS: usize = 10_000;
const MAX_LOOKUP_ROWS: usize = 5_000;
/// Rows asked of the worlds at a time.
const PAGE: u32 = 200;

/// A CSV file, built a row at a time.
struct Csv(String);

impl Csv {
    fn new(header: &[&str]) -> Self {
        let mut csv = Self(String::new());
        csv.row(header.iter().map(|h| (*h).to_owned()));
        csv
    }

    /// Quotes what needs it, and defuses text a spreadsheet would run as a
    /// formula.
    fn row(&mut self, fields: impl IntoIterator<Item = String>) {
        for (i, field) in fields.into_iter().enumerate() {
            if i > 0 {
                self.0.push(',');
            }
            let field = if field.starts_with(['=', '+', '-', '@', '\t', '\r']) {
                format!("'{field}")
            } else {
                field
            };
            if field.contains([',', '"', '\n', '\r']) {
                self.0.push('"');
                self.0.push_str(&field.replace('"', "\"\""));
                self.0.push('"');
            } else {
                self.0.push_str(&field);
            }
        }
        self.0.push_str("\r\n");
    }

    fn file(self, name: &str) -> Response {
        let date = iso_time(now_ms() as i64);
        (
            [
                (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_owned()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{name}-{}.csv\"", &date[..10]),
                ),
                (header::CACHE_CONTROL, "no-store".to_owned()),
            ],
            self.0,
        )
            .into_response()
    }
}

fn opt<T: ToString>(value: Option<T>) -> String {
    value.map_or_else(String::new, |v| v.to_string())
}

fn world(app: &App, node: Option<u8>) -> String {
    opt(node.and_then(|n| app.worlds.get(n)).map(|w| w.id))
}

fn audited(app: &App, staff: &Staff, what: String, rows: usize, capped: bool) {
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "exported")
            .target(what)
            .detail(format!(
                "{rows} rows{}",
                if capped {
                    " (the most an export holds)"
                } else {
                    ""
                }
            )),
    );
}

#[derive(Deserialize)]
pub struct AuditQuery {
    actor: Option<String>,
    q: Option<String>,
}

/// `GET /api/export/audit?actor=&q=`: the audit entries a search finds, as
/// CSV, newest first.
pub async fn audit(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<AuditQuery>,
) -> Response {
    let filter = AuditFilter::new(query.actor.as_deref(), query.q.as_deref());
    let view = AuditView::new(&app, &staff);
    let entries: Vec<_> = app
        .audit
        .page(None, MAX_AUDIT_ROWS, |e| view.shows(e) && filter.keeps(e))
        .into_iter()
        .map(|e| view.redact(e))
        .collect();
    let mut csv = Csv::new(&[
        "id", "time", "actor", "address", "action", "target", "detail", "ok",
    ]);
    for e in &entries {
        csv.row([
            e.id.to_string(),
            iso_time(e.at_ms as i64),
            e.actor.clone().unwrap_or_default(),
            opt(e.ip),
            e.action.clone(),
            e.target.clone().unwrap_or_default(),
            e.detail.clone(),
            e.ok.to_string(),
        ]);
    }
    audited(
        &app,
        &staff,
        "the audit log".into(),
        entries.len(),
        entries.len() == MAX_AUDIT_ROWS,
    );
    csv.file("audit")
}

#[derive(Deserialize)]
pub struct ChatQuery {
    name: Option<String>,
    node: Option<u8>,
}

/// `GET /api/export/chat?name=&node=`: chat as a search finds it, as CSV,
/// newest first. Private messages included, so it's audited.
pub async fn chat(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<ChatQuery>,
) -> Result<Response, ApiError> {
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
    if query.node.is_some_and(|n| app.worlds.get(n).is_none()) {
        return Err(ApiError::bad_request("No such world."));
    }
    let mut rows: Vec<ChatRecord> = Vec::new();
    let mut before = None;
    while rows.len() < MAX_LOOKUP_ROWS {
        let page: Vec<ChatRecord> = app
            .worlds
            .ask(
                path::CHAT,
                &ChatLookup {
                    name: name.clone(),
                    node: query.node,
                    before,
                    after: None,
                    max_staff_level: ceiling(&staff),
                    limit: PAGE,
                },
            )
            .await
            .map_err(unavailable)?;
        let full = page.len() == PAGE as usize;
        before = page.last().map(|c| c.id);
        rows.extend(page);
        if !full {
            break;
        }
    }
    rows.truncate(MAX_LOOKUP_ROWS);
    let mut csv = Csv::new(&[
        "id", "time", "world", "from", "to", "x", "z", "level", "message",
    ]);
    for c in &rows {
        csv.row([
            c.id.to_string(),
            iso_time(c.at_ms),
            world(&app, Some(c.node)),
            c.from.clone(),
            c.to.clone().unwrap_or_default(),
            c.x.to_string(),
            c.z.to_string(),
            c.level.to_string(),
            c.message.clone(),
        ]);
    }
    let what = match (&name, query.node.and_then(|n| app.worlds.get(n))) {
        (Some(name), Some(w)) => format!("chat of {name} on world {}", w.id),
        (Some(name), None) => format!("chat of {name}"),
        (None, Some(w)) => format!("chat on world {}", w.id),
        (None, None) => "chat on every world".to_owned(),
    };
    audited(
        &app,
        &staff,
        what,
        rows.len(),
        rows.len() == MAX_LOOKUP_ROWS,
    );
    Ok(csv.file("chat"))
}

#[derive(Deserialize)]
pub struct TimelineQuery {
    name: String,
    /// Comma-separated; every kind when missing.
    kinds: Option<String>,
}

const TIMELINE_KINDS: [&str; 8] = [
    "login",
    "session",
    "wealth",
    "chat",
    "moderation",
    "reported",
    "report",
    "replay",
];

/// `GET /api/export/timeline?name=&kinds=`: a player's timeline as CSV,
/// newest first; addresses as the caller may see them.
pub async fn timeline(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<TimelineQuery>,
) -> Result<Response, ApiError> {
    let name = checked_name(&query.name)?;
    check_player(&app, &staff, &name).await?;
    let kinds: Vec<String> = match query.kinds.as_deref().filter(|k| !k.trim().is_empty()) {
        Some(kinds) => kinds
            .split(',')
            .map(str::trim)
            .filter(|k| TIMELINE_KINDS.contains(k))
            .map(str::to_owned)
            .collect(),
        None => TIMELINE_KINDS.map(str::to_owned).to_vec(),
    };
    let mut rows: Vec<TimelineEntry> = Vec::new();
    let mut before = None;
    while rows.len() < MAX_LOOKUP_ROWS {
        let page: Vec<TimelineEntry> = app
            .worlds
            .ask(
                path::TIMELINE,
                &TimelineLookup {
                    name: name.clone(),
                    kinds: kinds.clone(),
                    before,
                    limit: PAGE,
                },
            )
            .await
            .map_err(unavailable)?;
        let full = page.len() == PAGE as usize;
        before = page.last().and_then(|e| {
            Some(TimelineCursor {
                at_ms: e.at_ms,
                kind: e.kind.clone(),
                id: e.id.parse().ok()?,
            })
        });
        rows.extend(page);
        if !full || before.is_none() {
            break;
        }
    }
    rows.truncate(MAX_LOOKUP_ROWS);
    let sees = sees_addresses(&staff);
    let mut csv = Csv::new(&[
        "kind", "id", "time", "world", "x", "z", "level", "type", "text", "other", "count",
        "value", "hours", "detail",
    ]);
    for e in &rows {
        let text = match (&e.text, e.kind.as_str()) {
            (Some(ip), "login") if !sees => address_label(&app, ip),
            (text, _) => text.clone().unwrap_or_default(),
        };
        csv.row([
            e.kind.clone(),
            e.id.clone(),
            iso_time(e.at_ms),
            world(&app, e.node),
            opt(e.x),
            opt(e.z),
            opt(e.level),
            opt(e.event_type),
            text,
            e.other.clone().unwrap_or_default(),
            opt(e.count),
            opt(e.value),
            opt(e.hours),
            e.detail.clone().unwrap_or_default(),
        ]);
    }
    audited(
        &app,
        &staff,
        format!("timeline of {name}"),
        rows.len(),
        rows.len() == MAX_LOOKUP_ROWS,
    );
    Ok(csv.file(&format!("timeline-{}", name.replace(' ', "_"))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_are_quoted_and_formulas_defused() {
        let mut csv = Csv::new(&["a", "b"]);
        csv.row(["say \"hi\", ok".to_owned(), "=1+1".to_owned()]);
        csv.row(["two\nlines".to_owned(), "-5".to_owned()]);
        assert_eq!(
            csv.0,
            "a,b\r\n\"say \"\"hi\"\", ok\",'=1+1\r\n\"two\nlines\",'-5\r\n"
        );
    }
}
