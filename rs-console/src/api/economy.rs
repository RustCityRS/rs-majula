use super::access::{ceiling, check_conflict, check_player, may_see};
use super::lookup::unavailable;
use super::players::{checked_name, note_on_case};
use super::support::{open_case, reason, registered, send_once, with_case, world_for};
use crate::audit::Event;
use crate::economy::{self, Alert, DEFAULT_ALERT_PCT, SAMPLE_EVERY_MS, Sample, Tracked};
use crate::http::{ApiError, App, Staff};
use crate::users::{name_key, now_ms};
use axum::Json;
use axum::extract::{Path, Query, State};
use rs_admin_api::{
    HoldersLookup, ItemHolders, ItemTrace, ObjLine, ObjSearch, Outcome, ReceiversLookup, Rollback,
    SavePointLine, SavePointsLookup, TopReceiver, TraceLookup, TradeGraph, TradeGraphLookup, path,
    utc_time,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

const DAY_MS: i64 = 24 * 3600 * 1000;
/// The most counts a chart is sent: a longer stretch is thinned to this.
const MAX_CHART_POINTS: usize = 600;
/// The most accounts an item's holders list.
const HOLDERS_SHOWN: u32 = 200;
/// How far back a trace goes unless asked, and the most.
const TRACE_DEPTH: u8 = 4;
const MAX_TRACE_DEPTH: u8 = 6;
/// Partners shown around a player on the trade graph.
const GRAPH_PARTNERS: u32 = 30;
/// The longest stretch the trade tools look over, in days.
const MAX_DAYS: u32 = 365;
/// The most accounts one ring rollback takes back.
pub const MAX_RING: usize = 25;
/// The alert threshold staff may set, in percent.
const MAX_ALERT_PCT: f64 = 1000.0;

#[derive(Serialize)]
pub struct Dashboard {
    tracked: Vec<Tracked>,
    samples: Vec<Sample>,
    alerts: Vec<Alert>,
    every_ms: u64,
    can_change: bool,
}

#[derive(Deserialize)]
pub struct DaysQuery {
    #[serde(default)]
    days: Option<u32>,
}

/// `GET /api/economy?days=`: the tracked items, their counts over the last
/// `days` (30 unless said) and the alerts.
pub async fn dashboard(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<DaysQuery>,
) -> Json<Dashboard> {
    let days = u64::from(query.days.unwrap_or(30).clamp(1, MAX_DAYS));
    let since = now_ms().saturating_sub(days * DAY_MS as u64);
    let state = app.economy.state();
    Json(Dashboard {
        tracked: state.tracked,
        samples: app.economy.samples_since(since, MAX_CHART_POINTS),
        alerts: state.alerts,
        every_ms: SAMPLE_EVERY_MS,
        can_change: staff.require_admin().is_ok(),
    })
}

#[derive(Serialize)]
pub struct Summary {
    open_alerts: usize,
}

/// `GET /api/economy/summary`: alerts nobody has acknowledged, for the
/// sidebar.
pub async fn summary(State(app): State<Arc<App>>, _staff: Staff) -> Json<Summary> {
    Json(Summary {
        open_alerts: app.economy.open_alerts(),
    })
}

/// `POST /api/economy/sample`: counts the tracked items now.
pub async fn sample(State(app): State<Arc<App>>, staff: Staff) -> Result<Json<Sample>, ApiError> {
    staff.require_admin()?;
    economy::sample_now(&app)
        .await
        .map(|(sample, _)| Json(sample))
        .map_err(unavailable)
}

#[derive(Deserialize)]
pub struct TrackRequest {
    obj: u16,
    #[serde(default)]
    alert_pct: Option<f64>,
}

/// `POST /api/economy/track`: starts following an item's circulation, or
/// changes the rise that alerts.
pub async fn track(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<TrackRequest>,
) -> Result<Json<Tracked>, ApiError> {
    staff.require_admin()?;
    let alert_pct = req.alert_pct.unwrap_or(DEFAULT_ALERT_PCT);
    if !(alert_pct > 0.0 && alert_pct <= MAX_ALERT_PCT) {
        return Err(ApiError::bad_request(format!(
            "Alert on a rise of more than 0% and at most {MAX_ALERT_PCT}%."
        )));
    }
    let found: Vec<ObjLine> = app
        .worlds
        .ask(
            path::OBJ_SEARCH,
            &ObjSearch {
                query: req.obj.to_string(),
            },
        )
        .await
        .map_err(unavailable)?;
    let obj = found
        .into_iter()
        .find(|o| o.id == req.obj)
        .ok_or_else(|| ApiError::bad_request("There's no such item."))?;
    app.economy
        .track(obj.id, &obj.name, alert_pct, staff.name())
        .map_err(ApiError::bad_request)?;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "economy_tracked")
            .target(obj.name.clone())
            .detail(format!("Alerts on a rise of {alert_pct}% between counts.")),
    );
    let tracked = app
        .economy
        .state()
        .tracked
        .into_iter()
        .find(|t| t.obj == obj.id)
        .ok_or_else(|| ApiError::internal("the item went missing"))?;
    Ok(Json(tracked))
}

#[derive(Deserialize)]
pub struct ObjRequest {
    obj: u16,
}

/// `POST /api/economy/untrack`.
pub async fn untrack(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<ObjRequest>,
) -> Result<Json<Tracked>, ApiError> {
    staff.require_admin()?;
    let tracked = app
        .economy
        .untrack(req.obj)
        .map_err(ApiError::bad_request)?;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "economy_untracked")
            .target(tracked.name.clone()),
    );
    Ok(Json(tracked))
}

/// `POST /api/economy/alerts/{id}/acknowledge`.
pub async fn acknowledge(
    State(app): State<Arc<App>>,
    staff: Staff,
    Path(id): Path<u64>,
) -> Result<Json<Alert>, ApiError> {
    let alert = app
        .economy
        .acknowledge(id, staff.name())
        .map_err(ApiError::bad_request)?;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "economy_alert_seen")
            .target(alert.name.clone())
            .detail(format!(
                "Alert #{id}: {} to {} at {}.",
                alert.from,
                alert.to,
                utc_time(alert.at_ms as i64)
            )),
    );
    Ok(Json(alert))
}

#[derive(Deserialize)]
pub struct HoldersQuery {
    obj: u16,
    #[serde(default)]
    min: Option<u32>,
}

/// `GET /api/economy/holders?obj=&min=`: who holds an item, as of their
/// last save; `null` when there's no such item.
pub async fn holders(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Query(query): Query<HoldersQuery>,
) -> Result<Json<Option<ItemHolders>>, ApiError> {
    app.worlds
        .ask_long(
            path::ITEM_HOLDERS,
            &HoldersLookup {
                obj: query.obj,
                min_count: query.min.unwrap_or(1).max(1),
                limit: HOLDERS_SHOWN,
            },
        )
        .await
        .map(Json)
        .map_err(unavailable)
}

#[derive(Deserialize)]
pub struct TraceQuery {
    name: String,
    obj: u16,
    #[serde(default)]
    depth: Option<u8>,
}

/// `GET /api/economy/trace?name=&obj=&depth=`: where a player's item came
/// from; `null` when there's no such item. Not back through staff above
/// the caller.
pub async fn trace(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<TraceQuery>,
) -> Result<Json<Option<ItemTrace>>, ApiError> {
    let name = checked_name(&query.name)?;
    check_player(&app, &staff, &name).await?;
    app.worlds
        .ask_long(
            path::ITEM_TRACE,
            &TraceLookup {
                name,
                obj: query.obj,
                depth: query.depth.unwrap_or(TRACE_DEPTH).clamp(1, MAX_TRACE_DEPTH),
                max_staff_level: ceiling(&staff),
            },
        )
        .await
        .map(Json)
        .map_err(unavailable)
}

#[derive(Deserialize)]
pub struct TradesQuery {
    name: String,
    #[serde(default)]
    days: Option<u32>,
}

fn since(days: Option<u32>, default: u32) -> i64 {
    now_ms() as i64 - i64::from(days.unwrap_or(default).clamp(1, MAX_DAYS)) * DAY_MS
}

/// `GET /api/economy/trades?name=&days=`: who a player hands items to and
/// gets them from. What passed only between staff above the caller is left
/// out.
pub async fn trades(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<TradesQuery>,
) -> Result<Json<TradeGraph>, ApiError> {
    let name = checked_name(&query.name)?;
    check_player(&app, &staff, &name).await?;
    let mut graph: TradeGraph = app
        .worlds
        .ask_long(
            path::TRADE_GRAPH,
            &TradeGraphLookup {
                name,
                since_ms: since(query.days, 30),
                partners: GRAPH_PARTNERS,
            },
        )
        .await
        .map_err(unavailable)?;
    hide_staff_trades(&staff, &mut graph);
    Ok(Json(graph))
}

/// Leaves out what passed between two staff above `staff`, and anyone with
/// nothing left to show but the player at the centre.
fn hide_staff_trades(staff: &Staff, graph: &mut TradeGraph) {
    let hidden: HashSet<String> = graph
        .accounts
        .iter()
        .filter(|a| !may_see(staff, a.staff_level))
        .map(|a| a.name.clone())
        .collect();
    graph
        .edges
        .retain(|e| !(hidden.contains(&e.from) && hidden.contains(&e.to)));
    let centre = graph.accounts.first().map(|a| a.name.clone());
    let edges = &graph.edges;
    graph.accounts.retain(|a| {
        Some(&a.name) == centre.as_ref() || edges.iter().any(|e| e.from == a.name || e.to == a.name)
    });
}

#[derive(Deserialize)]
pub struct ReceiversQuery {
    #[serde(default)]
    days: Option<u32>,
    #[serde(default)]
    min_givers: Option<u32>,
}

/// `GET /api/economy/receivers?days=&min_givers=`: accounts many players
/// hand items to, which take in far more than they give: mules.
pub async fn receivers(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<ReceiversQuery>,
) -> Result<Json<Vec<TopReceiver>>, ApiError> {
    let mut rows: Vec<TopReceiver> = app
        .worlds
        .ask_long(
            path::TOP_RECEIVERS,
            &ReceiversLookup {
                since_ms: since(query.days, 7),
                min_givers: query.min_givers.unwrap_or(3).clamp(1, 1000),
                limit: 100,
            },
        )
        .await
        .map_err(unavailable)?;
    rows.retain(|r| may_see(&staff, r.staff_level));
    Ok(Json(rows))
}

/// One account of a ring, and the save it would go back to.
#[derive(Serialize)]
pub struct RingMember {
    name: String,
    online_node: Option<u8>,
    world: Option<u16>,
    point: Option<SavePointLine>,
    /// Why it can't be rolled back, if it can't.
    problem: Option<String>,
}

/// The ring's names, each once, in the order given.
fn ring_names(names: &[String]) -> Result<Vec<String>, ApiError> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for name in names {
        let name = checked_name(name)?;
        if seen.insert(name_key(&name)) {
            out.push(name);
        }
    }
    if out.len() < 2 || out.len() > MAX_RING {
        return Err(ApiError::bad_request(format!(
            "A ring is 2 to {MAX_RING} accounts."
        )));
    }
    Ok(out)
}

/// Each account's newest save from at or before `at_ms`.
async fn ring_members(
    app: &App,
    names: Vec<String>,
    at_ms: i64,
) -> Result<Vec<RingMember>, ApiError> {
    if at_ms >= now_ms() as i64 {
        return Err(ApiError::bad_request("Pick a moment in the past."));
    }
    let mut members = Vec::with_capacity(names.len());
    for name in names {
        let record = match registered(app, &name).await {
            Ok(record) => record,
            Err(e) => {
                members.push(RingMember {
                    name,
                    online_node: None,
                    world: None,
                    point: None,
                    problem: Some(e.1),
                });
                continue;
            }
        };
        let points: Vec<SavePointLine> = app
            .worlds
            .ask(
                path::SAVE_POINTS,
                &SavePointsLookup {
                    name: record.name.clone(),
                    before: None,
                    saved_by_ms: Some(at_ms),
                    limit: 1,
                },
            )
            .await
            .map_err(unavailable)?;
        let point = points.into_iter().next();
        let problem = match &point {
            None => Some("They have no save from before then.".to_owned()),
            // A world on an older release ignores the time asked for.
            Some(p) if p.saved_ms > at_ms => {
                Some("The worlds need updating to pick a save by time.".to_owned())
            }
            Some(_) => None,
        };
        members.push(RingMember {
            world: record
                .online_node
                .and_then(|n| app.worlds.get(n))
                .map(|w| w.id),
            online_node: record.online_node,
            name: record.name,
            point,
            problem,
        });
    }
    Ok(members)
}

#[derive(Deserialize)]
pub struct RingQuery {
    /// Comma-separated.
    names: String,
    at_ms: i64,
}

/// `GET /api/economy/ring?names=&at_ms=`: the save each account of a ring
/// would be rolled back to.
pub async fn ring_preview(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<RingQuery>,
) -> Result<Json<Vec<RingMember>>, ApiError> {
    staff.require_admin()?;
    let names: Vec<String> = query
        .names
        .split(',')
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_owned)
        .collect();
    let names = ring_names(&names)?;
    Ok(Json(ring_members(&app, names, query.at_ms).await?))
}

#[derive(Deserialize)]
pub struct RingRequest {
    names: Vec<String>,
    at_ms: i64,
    reason: String,
    case: Option<u64>,
}

#[derive(Serialize)]
pub struct RingResult {
    name: String,
    point: i64,
    outcome: Outcome,
}

/// `POST /api/economy/ring-rollback`: rolls every account of a dupe ring
/// back to its newest save from before the same moment, as one action.
/// Nothing is rolled back unless every account has such a save.
pub async fn ring_rollback(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<RingRequest>,
) -> Result<Json<Vec<RingResult>>, ApiError> {
    staff.require_admin()?;
    let why = reason(&req.reason, true)?;
    let case = open_case(&app, req.case)?;
    let names = ring_names(&req.names)?;
    // As with one rollback: none of them may be the admin's own accounts.
    for name in &names {
        check_conflict(&app, &staff, name, "roll back this ring").await?;
    }
    let members = ring_members(&app, names, req.at_ms).await?;
    let problems: Vec<String> = members
        .iter()
        .filter_map(|m| Some(format!("{}: {}", m.name, m.problem.as_ref()?)))
        .collect();
    if !problems.is_empty() {
        return Err(ApiError::bad_request(format!(
            "Nothing was rolled back. {}",
            problems.join(" ")
        )));
    }
    let moment = utc_time(req.at_ms);
    let note = format!(
        "{} (ring of {} rolled back to {moment})",
        why.trim_end_matches('.'),
        members.len()
    );
    let mut results = Vec::with_capacity(members.len());
    for member in members {
        let Some(point) = member.point else {
            continue;
        };
        let record = registered(&app, &member.name).await?;
        let outcome = match world_for(&app, &record).await {
            Ok(node) => {
                send_once(
                    &app,
                    node,
                    path::ROLLBACK,
                    &Rollback {
                        actor: staff.actor(),
                        name: record.name.clone(),
                        point: point.id,
                        reason: with_case(Some(case), &note),
                    },
                )
                .await
            }
            Err(e) => Outcome::refused(e.1),
        };
        note_on_case(
            &app,
            &staff,
            Some(case),
            "rollback",
            &record.name,
            None,
            outcome.ok,
            &outcome.message,
            &note,
        );
        results.push(RingResult {
            name: record.name,
            point: point.id,
            outcome,
        });
    }
    let all_ok = results.iter().all(|r| r.outcome.ok);
    let each: Vec<String> = results
        .iter()
        .map(|r| {
            if r.outcome.ok {
                format!("{} to point {}", r.name, r.point)
            } else {
                format!("{} failed ({})", r.name, r.outcome.message)
            }
        })
        .collect();
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "ring_rolled_back")
            .target(
                results
                    .iter()
                    .map(|r| r.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
            .detail(format!(
                "Case #{case}. Back to {moment}: {}. Reason: {why}",
                each.join("; ")
            ))
            .ok(all_ok),
    );
    Ok(Json(results))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_are_named_once_each() {
        let names = |list: &[&str]| list.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        assert_eq!(
            ring_names(&names(&["Ash", "ash", "Kim"])).unwrap(),
            ["Ash", "Kim"]
        );
        assert!(ring_names(&names(&["Ash", "ash"])).is_err());
        let many: Vec<String> = (0..=MAX_RING).map(|i| format!("p{i}")).collect();
        assert!(ring_names(&many).is_err());
    }
}
