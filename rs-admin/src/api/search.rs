use super::AuditFilter;
use super::access::AuditView;
use super::replays::{ReplayView, view};
use crate::audit::Entry;
use crate::cases::Case;
use crate::http::{App, Staff};
use crate::users::name_key;
use axum::Json;
use axum::extract::{Query, State};
use futures_util::future::join_all;
use rs_admin_api::{PlayerLookup, PlayerRecord, ReplayGet, ReplayLookup, ReplaySummary, path};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::sync::Arc;

/// The most of each kind a search shows.
const SHOWN: usize = 6;

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    q: String,
}

#[derive(Serialize)]
pub struct FoundPlayer {
    name: String,
    online_world: Option<u16>,
    registered: bool,
    staff_level: u8,
}

#[derive(Serialize, Default)]
pub struct Found {
    players: Vec<FoundPlayer>,
    cases: Vec<Case>,
    replays: Vec<ReplayView>,
    audit: Vec<Entry>,
}

/// Whether `q` could be a game name.
fn name_like(q: &str) -> bool {
    (1..=12).contains(&q.chars().count())
        && q.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '_')
}

/// `GET /api/search?q=`: players, cases, recordings (by id, or the best
/// matching player's newest) and recent audit entries that match, a few of
/// each, for the command palette. A part the worlds can't answer is left
/// empty.
pub async fn search(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<SearchQuery>,
) -> Json<Found> {
    let q = query.q.trim();
    if q.is_empty() || q.chars().count() > 100 {
        return Json(Found::default());
    }
    let needle = q.to_lowercase();
    let key = name_key(q);
    let case_id = q.strip_prefix('#').unwrap_or(q).parse::<u64>().ok();
    let replay_id = (q.len() >= 6 && q.len() <= 20 && q.bytes().all(|b| b.is_ascii_digit()))
        .then(|| q.to_owned());
    let name = name_like(q).then(|| q.to_owned());

    let mut cases = app.cases.all(|c| {
        Some(c.id) == case_id
            || c.title.to_lowercase().contains(&needle)
            || name_key(&c.player).contains(&key)
    });
    // The case asked for by number, then open before closed, newest change first.
    cases.sort_by_key(|c| {
        (
            Some(c.id) != case_id,
            !c.is_open(),
            Reverse(c.closed_ms.unwrap_or(c.updated_ms)),
        )
    });
    cases.truncate(SHOWN);
    let filter = AuditFilter::new(None, Some(q));
    let audit_view = AuditView::new(&app, &staff);
    let audit = app
        .audit
        .recent(SHOWN, |e| audit_view.shows(e) && filter.keeps(e))
        .into_iter()
        .map(|e| audit_view.redact(e))
        .collect();

    let worlds = app.worlds.online();
    let online = async {
        if name.is_none() {
            return Vec::new();
        }
        let lists = join_all(worlds.iter().map(|w| app.worlds.players(w))).await;
        worlds
            .iter()
            .zip(lists)
            .filter_map(|(w, list)| list.ok().map(|list| (w.id, list)))
            .flat_map(|(world, list)| {
                list.into_iter()
                    .filter(|p| name_key(&p.name).contains(&key))
                    .map(move |p| (world, p))
            })
            .collect::<Vec<_>>()
    };
    let registered = async {
        match &name {
            Some(name) => app
                .worlds
                .ask::<_, Option<PlayerRecord>>(path::PLAYER, &PlayerLookup { name: name.clone() })
                .await
                .ok()
                .flatten(),
            None => None,
        }
    };
    let by_id = async {
        match &replay_id {
            Some(id) => app
                .worlds
                .ask::<_, Option<ReplaySummary>>(path::REPLAY, &ReplayGet { id: id.clone() })
                .await
                .ok()
                .flatten(),
            None => None,
        }
    };
    let (online, registered, by_id) = tokio::join!(online, registered, by_id);

    let mut players: Vec<FoundPlayer> = Vec::new();
    if let Some(record) = registered {
        players.push(FoundPlayer {
            online_world: online
                .iter()
                .find(|(_, p)| name_key(&p.name) == name_key(&record.name))
                .map(|(w, _)| *w),
            name: record.name,
            registered: true,
            staff_level: record.staff_level,
        });
    }
    let mut others: Vec<FoundPlayer> = online
        .into_iter()
        .filter(|(_, p)| {
            !players
                .iter()
                .any(|f| name_key(&f.name) == name_key(&p.name))
        })
        .map(|(world, p)| FoundPlayer {
            online_world: Some(world),
            registered: name_key(&p.name) == key,
            staff_level: p.staff_level,
            name: p.name,
        })
        .collect();
    others.sort_by_key(|p| (!name_key(&p.name).starts_with(&key), name_key(&p.name)));
    players.extend(others);
    players.truncate(SHOWN);

    // A recording asked for by id, else the sessions of whoever fits best.
    let replays: Vec<ReplaySummary> = match (by_id, players.first()) {
        (Some(found), _) => {
            if super::access::check_player(&app, &staff, &found.player)
                .await
                .is_ok()
            {
                vec![found]
            } else {
                Vec::new()
            }
        }
        (None, Some(best)) => app
            .worlds
            .ask(
                path::REPLAYS,
                &ReplayLookup {
                    name: Some(best.name.clone()),
                    flagged: false,
                    before: None,
                    max_staff_level: super::access::ceiling(&staff),
                    limit: SHOWN as u32,
                },
            )
            .await
            .unwrap_or_default(),
        (None, None) => Vec::new(),
    };

    Json(Found {
        players,
        cases,
        replays: replays.into_iter().map(|r| view(&app, r)).collect(),
        audit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_game_names_are_looked_up_as_players() {
        assert!(name_like("Mod Ash"));
        assert!(!name_like("ban appeal #4"));
        assert!(!name_like("thirteen_char"));
    }
}
