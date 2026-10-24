use super::AuditFilter;
use super::access::{AuditView, CaseView};
use super::live;
use super::replays::{ReplayView, view};
use crate::audit::Entry;
use crate::cases::Case;
use crate::http::{App, Staff};
use crate::users::name_key;
use axum::Json;
use axum::extract::{Query, State};
use rs_admin_api::{PlayerRecord, PlayerSearch, ReplayGet, ReplayLookup, ReplaySummary, path};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::sync::Arc;

/// The most of each kind a search shows.
const SHOWN: usize = 6;
/// The most names suggested for a name being typed.
const SUGGESTED: usize = 10;

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

    let case_view = CaseView::new(&app, &staff);
    let mut cases = app.cases.all(|c| {
        case_view.sees(c)
            && (Some(c.id) == case_id
                || c.title.to_lowercase().contains(&needle)
                || name_key(&c.player).contains(&key))
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

    let players = async {
        match &name {
            Some(name) => find_players(&app, name, SHOWN).await,
            None => Vec::new(),
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
    let (players, by_id) = tokio::join!(players, by_id);

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

/// Up to `limit` players for the name (or part of one) `name`: registered
/// names starting with it and anyone online whose name holds it anywhere;
/// the name itself first, then those starting with it, online first,
/// shortest first. What the worlds can't answer is left out.
pub(super) async fn find_players(app: &App, name: &str, limit: usize) -> Vec<FoundPlayer> {
    let key = name_key(name);
    let online = async {
        live::online(app)
            .await
            .iter()
            .filter_map(|l| l.players.as_ref().map(|list| (l.world, list)))
            .flat_map(|(world, list)| {
                list.iter()
                    .filter(|p| name_key(&p.name).contains(&key))
                    .map(move |p| (world, p.clone()))
            })
            .collect::<Vec<_>>()
    };
    let registered = async {
        app.worlds
            .ask::<_, Vec<PlayerRecord>>(
                path::PLAYER_SEARCH,
                &PlayerSearch {
                    prefix: name.to_owned(),
                    limit: limit as u32,
                },
            )
            .await
            .unwrap_or_default()
    };
    let (online, registered) = tokio::join!(online, registered);

    // Registered names starting with the query, and anyone online whose
    // name holds it anywhere.
    let mut players: Vec<FoundPlayer> = registered
        .into_iter()
        .map(|record| FoundPlayer {
            online_world: online
                .iter()
                .find(|(_, p)| name_key(&p.name) == name_key(&record.name))
                .map(|(w, _)| *w),
            name: record.name,
            registered: true,
            staff_level: record.staff_level,
        })
        .collect();
    let others: Vec<FoundPlayer> = online
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
    players.extend(others);
    // The name itself, then those starting with it, online first, shortest first.
    players.sort_by_cached_key(|p| {
        let name = name_key(&p.name);
        (
            name != key,
            !name.starts_with(&key),
            p.online_world.is_none(),
            name.len(),
            name,
        )
    });
    players.truncate(limit);
    players
}

/// `GET /api/players/names?q=`: players to suggest for a name being typed.
pub async fn names(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Query(query): Query<SearchQuery>,
) -> Json<Vec<FoundPlayer>> {
    let q = query.q.trim();
    if !name_like(q) {
        return Json(Vec::new());
    }
    Json(find_players(&app, q, SUGGESTED).await)
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
