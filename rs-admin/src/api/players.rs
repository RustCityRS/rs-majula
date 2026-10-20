use crate::audit::Event;
use crate::cases::Change;
use crate::http::{ApiError, App, Staff};
use crate::templates::Offence;
use crate::users::{name_key, now_ms};
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
    /// What to do, unless a template decides.
    #[serde(default)]
    action: Option<ModAction>,
    #[serde(default)]
    hours: Option<i32>,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    case: Option<u64>,
    /// A sanction template: the action and length are the next step on its
    /// ladder for this player, and the offence is recorded.
    #[serde(default)]
    template: Option<String>,
    /// Linked accounts to give the same mute or ban.
    #[serde(default)]
    also: Vec<String>,
}

#[derive(Serialize)]
pub struct Moderated {
    name: String,
    ok: bool,
    message: String,
}

#[derive(Serialize)]
pub struct ModerateResult {
    ok: bool,
    message: String,
    /// How it went on each linked account.
    also: Vec<Moderated>,
}

/// The most linked accounts one command reaches.
const MAX_ALSO: usize = 25;

pub(super) fn action_name(action: ModAction) -> &'static str {
    match action {
        ModAction::Mute => "mute",
        ModAction::Unmute => "unmute",
        ModAction::Ban => "ban",
        ModAction::Unban => "unban",
    }
}

/// Runs one moderation command through the first world that can reach the
/// game database, as the in-game command would. A ban also kicks them from
/// wherever they are. Returns whether it took, and what to tell staff.
pub(super) async fn moderate_one(
    app: &App,
    staff: &Staff,
    name: &str,
    action: ModAction,
    hours: Option<i32>,
    reason: &str,
) -> (bool, String) {
    let body = Moderate {
        actor: staff.actor(),
        name: name.to_owned(),
        action,
        hours,
        reason: Some(reason.to_owned()).filter(|r| !r.is_empty()),
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

    let what = match (action, hours) {
        (ModAction::Mute, Some(h)) => format!("Muted {name} for {}.", duration(h)),
        (ModAction::Ban, Some(h)) => format!("Banned {name} for {}.", duration(h)),
        (ModAction::Unmute, _) => format!("Unmuted {name}."),
        (ModAction::Unban, _) => format!("Unbanned {name}."),
        _ => String::new(),
    };
    match outcome {
        Some(ModOutcome::Done) => {
            if action == ModAction::Ban {
                kick_everywhere(app, staff, name).await;
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
    }
}

/// `POST /api/players/moderate`: mutes, unmutes, bans or unbans a player,
/// by hand or by a sanction template, and any linked accounts with them.
pub async fn moderate(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<ModerateRequest>,
) -> Result<Json<ModerateResult>, ApiError> {
    let name = checked_name(&req.name)?;
    check_case(&app, req.case)?;
    let mut also: Vec<String> = Vec::new();
    for other in &req.also {
        let other = checked_name(other)?;
        if name_key(&other) != name_key(&name)
            && !also.iter().any(|a| name_key(a) == name_key(&other))
        {
            also.push(other);
        }
    }
    if also.len() > MAX_ALSO {
        return Err(ApiError::bad_request(format!(
            "At most {MAX_ALSO} linked accounts at once."
        )));
    }

    let typed = req.reason.trim().chars().take(300).collect::<String>();
    let (action, hours, template, reason) = match &req.template {
        Some(id) => {
            let template = app
                .templates
                .get(id)
                .ok_or_else(|| ApiError::bad_request("That template is gone; reload."))?;
            let standing = app.templates.standing(&name, &template);
            let label = format!(
                "{} (offence {})",
                template.name,
                standing.offences.len() + 1
            );
            let reason = if typed.is_empty() {
                label
            } else {
                format!("{label}: {typed}")
            };
            let step = standing.step;
            (
                step.action.mod_action(),
                Some(step.hours),
                Some((template, standing.next)),
                reason,
            )
        }
        None => {
            let action = req
                .action
                .ok_or_else(|| ApiError::bad_request("Choose what to do."))?;
            let hours = if matches!(action, ModAction::Mute | ModAction::Ban) {
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
            (action, hours, None, typed)
        }
    };
    if !also.is_empty() && !matches!(action, ModAction::Mute | ModAction::Ban) {
        return Err(ApiError::bad_request(
            "Only mutes and bans reach linked accounts.",
        ));
    }

    let mut results: Vec<Moderated> = Vec::with_capacity(1 + also.len());
    for (i, target) in std::iter::once(&name).chain(&also).enumerate() {
        let reason = if i == 0 {
            reason.clone()
        } else {
            format!("Linked to {name}: {reason}")
        };
        let (ok, message) = moderate_one(&app, &staff, target, action, hours, &reason).await;
        if ok && let Some((template, step)) = &template {
            let recorded = app.templates.record(Offence {
                at_ms: now_ms(),
                player: target.clone(),
                template: template.id.clone(),
                step: *step,
                by: staff.name().to_owned(),
            });
            if let Err(e) = recorded {
                warn!("Couldn't record {}'s offence: {}", target, e);
            }
        }
        app.audit.record(
            Event::new(Some(staff.name()), staff.client.ip, action_name(action))
                .target(target.as_str())
                .detail(reason_detail(&reason, &message))
                .ok(ok),
        );
        note_on_case(
            &app,
            &staff,
            req.case,
            action_name(action),
            target,
            hours,
            ok,
            &message,
            &reason,
        );
        results.push(Moderated {
            name: target.clone(),
            ok,
            message,
        });
    }
    let first = results.remove(0);
    Ok(Json(ModerateResult {
        ok: first.ok,
        message: first.message,
        also: results,
    }))
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
