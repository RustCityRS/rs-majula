use super::lookup::unavailable;
use crate::http::{ApiError, App, Staff};
use crate::users::name_key;
use axum::http::StatusCode;
use rs_admin_api::{PlayerLookup, PlayerRecord, ReplayGet, ReplaySummary, path};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a player's staff level is trusted before it's looked up again.
const FRESH_FOR: Duration = Duration::from_secs(60);
/// The most levels kept; past it the cache starts again.
const MAX_KEPT: usize = 10_000;

/// Players' game staff levels, by name key, as last looked up.
#[derive(Default)]
pub struct StaffLevels(Mutex<HashMap<String, (u8, Instant)>>);

impl StaffLevels {
    fn get(&self, name: &str) -> Option<u8> {
        let levels = self.0.lock().unwrap_or_else(|e| e.into_inner());
        levels
            .get(&name_key(name))
            .filter(|(_, at)| at.elapsed() < FRESH_FOR)
            .map(|(level, _)| *level)
    }

    fn set(&self, name: &str, level: u8) {
        let mut levels = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if levels.len() >= MAX_KEPT {
            levels.clear();
        }
        levels.insert(name_key(name), (level, Instant::now()));
    }
}

/// `name`'s game staff level (0 for a player, or a name nobody has).
async fn level_of(app: &App, name: &str) -> Result<u8, ApiError> {
    if let Some(level) = app.staff_levels.get(name) {
        return Ok(level);
    }
    let record: Option<PlayerRecord> = app
        .worlds
        .ask(
            path::PLAYER,
            &PlayerLookup {
                name: name.to_owned(),
            },
        )
        .await
        .map_err(unavailable)?;
    let level = record.map_or(0, |r| r.staff_level);
    app.staff_levels.set(name, level);
    Ok(level)
}

/// Whether `staff` may see the sessions and logs of a player at `level`.
pub(super) fn may_see(staff: &Staff, level: u8) -> bool {
    level <= staff.user.role.staff_level()
}

/// Refuses a look at `name`'s sessions or logs when they're staff above the
/// caller.
pub(super) async fn check_player(app: &App, staff: &Staff, name: &str) -> Result<(), ApiError> {
    if may_see(staff, level_of(app, name).await?) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::FORBIDDEN,
            format!("{name} is staff above your role, so their sessions and logs are hidden."),
        ))
    }
}

/// The highest staff level whose sessions and logs `staff` may see, for the
/// worlds to leave out everyone above it from lists.
pub(super) fn ceiling(staff: &Staff) -> Option<u8> {
    Some(staff.user.role.staff_level())
}

/// Refuses recording `id` if its player is staff above the caller: whoever
/// the worlds say it's of, else the copy kept for a clip.
pub(super) async fn check_replay(app: &App, staff: &Staff, id: &str) -> Result<(), ApiError> {
    let found: Result<Option<ReplaySummary>, String> = app
        .worlds
        .ask(path::REPLAY, &ReplayGet { id: id.to_owned() })
        .await;
    let player = match found {
        Ok(Some(summary)) => summary.player,
        other => match app.clips.summary(id) {
            Some(kept) => kept.player,
            None => {
                return Err(match other {
                    Err(e) => unavailable(e),
                    Ok(_) => ApiError::new(StatusCode::NOT_FOUND, "That recording is gone."),
                });
            }
        },
    };
    check_player(app, staff, &player).await
}
