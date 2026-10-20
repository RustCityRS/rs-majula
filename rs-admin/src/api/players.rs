use crate::audit::Event;
use crate::cases::Change;
use crate::http::{ApiError, App, Staff};
use crate::users::name_key;
use crate::worlds::{Outcome, Reply};
use axum::Json;
use axum::extract::{Query, State};
use futures_util::future::join_all;
use rs_admin_api::{Kick, ModAction, ModOutcome, Moderate, path};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::warn;

/// The most players a search returns.
const MAX_RESULTS: usize = 500;
/// The longest mute or ban: 10 years, as in game.
const MAX_MOD_HOURS: i32 = 87_600;

/// Whether `name` could be a game name: 1-12 letters, digits, spaces or
/// underscores.
fn valid_name(name: &str) -> bool {
    (1..=12).contains(&name.chars().count())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '_')
}

pub(super) fn checked_name(name: &str) -> Result<String, ApiError> {
    let name = name.trim();
    if valid_name(name) {
        Ok(name.to_owned())
    } else {
        Err(ApiError::bad_request("That isn't a valid player name."))
    }
}

#[derive(Serialize)]
pub struct FoundPlayer {
    #[serde(flatten)]
    player: rs_admin_api::OnlinePlayer,
    world: u16,
    node: u8,
}

#[derive(Serialize)]
pub struct Search {
    players: Vec<FoundPlayer>,
    total: usize,
    unreachable: Vec<u16>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    q: String,
}

/// `GET /api/players?q=`: everyone online on every world whose name contains
/// `q`.
pub async fn search(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Query(query): Query<SearchQuery>,
) -> Json<Search> {
    let needle = name_key(&query.q);
    let worlds = app.worlds.online();
    let lists = join_all(worlds.iter().map(|w| app.worlds.players(w))).await;

    let mut players = Vec::new();
    let mut unreachable = Vec::new();
    for (world, list) in worlds.iter().zip(lists) {
        match list {
            Ok(list) => players.extend(
                list.into_iter()
                    .filter(|p| name_key(&p.name).contains(&needle))
                    .map(|player| FoundPlayer {
                        player,
                        world: world.id,
                        node: world.node,
                    }),
            ),
            Err(_) => unreachable.push(world.id),
        }
    }
    // Exact matches first, then by name.
    players.sort_by_key(|p| {
        let key = name_key(&p.player.name);
        (key != needle, !key.starts_with(&needle), key)
    });
    let total = players.len();
    players.truncate(MAX_RESULTS);
    Json(Search {
        players,
        total,
        unreachable,
    })
}

#[derive(Deserialize)]
pub struct KickRequest {
    name: String,
    #[serde(default)]
    reason: String,
    /// The case it's sent from, which notes it in its timeline.
    #[serde(default)]
    case: Option<u64>,
}

#[derive(Serialize)]
pub struct KickResult {
    /// The worlds they were kicked from (none if they weren't online).
    kicked_from: Vec<u16>,
    message: String,
}

/// Asks every online world to log `name` out.
async fn kick_everywhere(app: &App, staff: &Staff, name: &str) -> Vec<u16> {
    let actor = staff.actor();
    let replies: Vec<Reply<rs_admin_api::Outcome>> = app
        .worlds
        .command(&app.worlds.online(), path::KICK, |_| Kick {
            actor: actor.clone(),
            name: name.to_owned(),
        })
        .await;
    replies
        .into_iter()
        .filter(|r| matches!(&r.result, Outcome::Answered { answer } if answer.ok))
        .map(|r| r.world)
        .collect()
}

/// `POST /api/players/kick`.
pub async fn kick(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<KickRequest>,
) -> Result<Json<KickResult>, ApiError> {
    let name = checked_name(&req.name)?;
    check_case(&app, req.case)?;
    let kicked_from = kick_everywhere(&app, &staff, &name).await;
    let message = match kicked_from.as_slice() {
        [] => format!("{name} isn't online, or is staff of your level or above."),
        [world] => format!("Kicked {name} from world {world}."),
        worlds => format!("Kicked {name} from {} worlds.", worlds.len()),
    };
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "kick")
            .target(&name)
            .detail(reason_detail(&req.reason, &message))
            .ok(!kicked_from.is_empty()),
    );
    note_on_case(
        &app,
        &staff,
        req.case,
        "kick",
        &name,
        None,
        !kicked_from.is_empty(),
        &message,
        &req.reason,
    );
    Ok(Json(KickResult {
        kicked_from,
        message,
    }))
}

/// Refuses a command sent from a case that doesn't exist.
fn check_case(app: &App, case: Option<u64>) -> Result<(), ApiError> {
    match case {
        Some(id) if app.cases.get(id).is_none() => {
            Err(ApiError::bad_request(format!("There's no case #{id}.")))
        }
        _ => Ok(()),
    }
}

/// Notes a command in the timeline of the case it was sent from.
#[allow(clippy::too_many_arguments)]
fn note_on_case(
    app: &App,
    staff: &Staff,
    case: Option<u64>,
    action: &str,
    name: &str,
    hours: Option<i32>,
    ok: bool,
    message: &str,
    reason: &str,
) {
    let Some(id) = case else {
        return;
    };
    let change = Change::Action {
        action: action.to_owned(),
        name: name.to_owned(),
        hours,
        ok,
        message: message.to_owned(),
        reason: reason.trim().chars().take(300).collect(),
    };
    if let Err(e) = app.cases.change(staff.name(), id, |_| Ok(vec![change])) {
        warn!("Couldn't note {} on case #{}: {}", action, id, e);
    }
}

fn reason_detail(reason: &str, message: &str) -> String {
    let reason = reason.trim();
    if reason.is_empty() {
        message.to_owned()
    } else {
        format!(
            "{message} Reason: {}",
            reason.chars().take(300).collect::<String>()
        )
    }
}

#[derive(Deserialize)]
pub struct ModerateRequest {
    name: String,
    action: ModAction,
    #[serde(default)]
    hours: Option<i32>,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    case: Option<u64>,
}

#[derive(Serialize)]
pub struct ModerateResult {
    ok: bool,
    message: String,
}

fn action_name(action: ModAction) -> &'static str {
    match action {
        ModAction::Mute => "mute",
        ModAction::Unmute => "unmute",
        ModAction::Ban => "ban",
        ModAction::Unban => "unban",
    }
}

/// `POST /api/players/moderate`: mutes, unmutes, bans or unbans through the
/// first world that can reach the game database, as the in-game command
/// would. A ban also kicks them from wherever they are.
pub async fn moderate(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<ModerateRequest>,
) -> Result<Json<ModerateResult>, ApiError> {
    let name = checked_name(&req.name)?;
    check_case(&app, req.case)?;
    let timed = matches!(req.action, ModAction::Mute | ModAction::Ban);
    let hours = if timed {
        match req.hours {
            Some(h @ 1..=MAX_MOD_HOURS) => Some(h),
            _ => {
                return Err(ApiError::bad_request(format!(
                    "Choose a length from 1 hour to {} years.",
                    MAX_MOD_HOURS / 8760
                )));
            }
        }
    } else {
        None
    };

    let body = Moderate {
        actor: staff.actor(),
        name: name.clone(),
        action: req.action,
        hours,
    };
    let mut outcome = None;
    for world in app.worlds.online() {
        let replies: Vec<Reply<ModOutcome>> = app
            .worlds
            .command(std::slice::from_ref(&world), path::MODERATE, |_| {
                body.clone()
            })
            .await;
        if let Some(Reply {
            result: Outcome::Answered { answer },
            ..
        }) = replies.into_iter().next()
            && answer != ModOutcome::Offline
        {
            outcome = Some(answer);
            break;
        }
    }

    let what = match (req.action, hours) {
        (ModAction::Mute, Some(h)) => format!("Muted {name} for {}.", duration(h)),
        (ModAction::Ban, Some(h)) => format!("Banned {name} for {}.", duration(h)),
        (ModAction::Unmute, _) => format!("Unmuted {name}."),
        (ModAction::Unban, _) => format!("Unbanned {name}."),
        _ => String::new(),
    };
    let (ok, message) = match outcome {
        Some(ModOutcome::Done) => {
            if req.action == ModAction::Ban {
                kick_everywhere(&app, &staff, &name).await;
            }
            (true, what)
        }
        Some(ModOutcome::Refused) => (
            false,
            format!("{name} isn't a registered player, or is staff of your level or above."),
        ),
        Some(ModOutcome::Pending) => (
            false,
            "The game database is slow to answer; check again in a minute.".to_owned(),
        ),
        Some(ModOutcome::Offline) | None => (
            false,
            "No world could reach the game database, so nothing was changed.".to_owned(),
        ),
    };
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, action_name(req.action))
            .target(&name)
            .detail(reason_detail(&req.reason, &message))
            .ok(ok),
    );
    note_on_case(
        &app,
        &staff,
        req.case,
        action_name(req.action),
        &name,
        hours,
        ok,
        &message,
        &req.reason,
    );
    Ok(Json(ModerateResult { ok, message }))
}

/// `48` -> `2 days`.
fn duration(hours: i32) -> String {
    let plural = |n: i32, unit: &str| format!("{n} {unit}{}", if n == 1 { "" } else { "s" });
    match hours {
        h if h % 8760 == 0 => plural(h / 8760, "year"),
        h if h % 168 == 0 => plural(h / 168, "week"),
        h if h % 24 == 0 => plural(h / 24, "day"),
        h => plural(h, "hour"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_checked_like_game_names() {
        assert!(valid_name("Mod Ash"));
        assert!(valid_name("a_b_c"));
        assert!(!valid_name(""));
        assert!(!valid_name("thirteen_char"));
        assert!(!valid_name("bad-name"));
    }

    #[test]
    fn durations_read_naturally() {
        assert_eq!(duration(1), "1 hour");
        assert_eq!(duration(48), "2 days");
        assert_eq!(duration(168), "1 week");
        assert_eq!(duration(8760), "1 year");
        assert_eq!(duration(25), "25 hours");
    }
}
