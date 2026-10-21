use super::lookup::unavailable;
use crate::audit::Entry;
use crate::http::{ApiError, App, Staff};
use crate::users::{Role, name_key};
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

/// What `staff` may see of the audit trail: nothing staff above them did,
/// and the address and browser only of their own entries and those of staff
/// below them. A name with no console account (removed, or mistyped at
/// sign-in) is shown to maintainers only.
pub(super) struct AuditView {
    levels: HashMap<String, u8>,
    me: String,
    level: u8,
    maintainer: bool,
}

impl AuditView {
    /// Unreadable accounts leave every staff entry hidden from all but
    /// maintainers.
    pub(super) fn new(app: &App, staff: &Staff) -> Self {
        Self {
            levels: app
                .users
                .all()
                .unwrap_or_default()
                .into_iter()
                .map(|u| (name_key(&u.name), u.role.staff_level()))
                .collect(),
            me: name_key(staff.name()),
            level: staff.user.role.staff_level(),
            maintainer: staff.user.role == Role::Maintainer,
        }
    }

    fn level_of(&self, name: &str) -> Option<u8> {
        self.levels.get(&name_key(name)).copied()
    }

    /// Whether they may see what `name` did.
    pub(super) fn sees(&self, name: &str) -> bool {
        self.maintainer || self.level_of(name).is_some_and(|l| l <= self.level)
    }

    pub(super) fn shows(&self, e: &Entry) -> bool {
        e.owner().is_none_or(|name| self.sees(name))
    }

    /// `e` without what they may not see of it.
    pub(super) fn redact(&self, mut e: Entry) -> Entry {
        let private = e.owner().is_some_and(|name| {
            !self.maintainer
                && name_key(name) != self.me
                && self.level_of(name).is_none_or(|l| l >= self.level)
        });
        if private {
            e.ip = None;
            if e.action == "sign_in" {
                e.detail.clear();
            }
        }
        e
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn view(me: &str, role: Role) -> AuditView {
        AuditView {
            levels: [("trial", 1), ("mod", 1), ("admin", 2), ("boss", 3)]
                .into_iter()
                .map(|(n, l)| (n.to_owned(), l))
                .collect(),
            me: me.to_owned(),
            level: role.staff_level(),
            maintainer: role == Role::Maintainer,
        }
    }

    fn entry(actor: Option<&str>, target: Option<&str>, address: bool) -> Entry {
        Entry {
            id: 1,
            at_ms: 0,
            actor: actor.map(str::to_owned),
            ip: address.then(|| "10.0.0.1".parse().unwrap()),
            action: "sign_in".into(),
            target: target.map(str::to_owned),
            detail: "Firefox".into(),
            ok: true,
        }
    }

    #[test]
    fn staff_above_you_are_left_out_of_the_trail() {
        let moderator = view("mod", Role::Moderator);
        assert!(moderator.shows(&entry(Some("trial"), None, true)));
        assert!(moderator.shows(&entry(Some("mod"), None, true)));
        assert!(!moderator.shows(&entry(Some("admin"), None, true)));
        assert!(!moderator.shows(&entry(Some("boss"), None, true)));
        // A failed sign-in by name, and one for a name with no account.
        assert!(!moderator.shows(&entry(None, Some("boss"), true)));
        assert!(!moderator.shows(&entry(None, Some("nobody"), true)));
        assert!(!moderator.shows(&entry(Some("removed"), None, true)));
        // What the console did itself.
        assert!(moderator.shows(&entry(None, Some("a player"), false)));

        let admin = view("admin", Role::Admin);
        assert!(admin.shows(&entry(Some("admin"), None, true)));
        assert!(!admin.shows(&entry(Some("boss"), None, true)));
        let maintainer = view("boss", Role::Maintainer);
        assert!(maintainer.shows(&entry(Some("removed"), None, true)));
        assert!(maintainer.shows(&entry(None, Some("nobody"), true)));
    }

    #[test]
    fn addresses_show_only_for_yourself_and_those_below() {
        let address = |v: &AuditView, actor| v.redact(entry(Some(actor), None, true)).ip.is_some();
        let moderator = view("mod", Role::Moderator);
        assert!(address(&moderator, "mod"));
        assert!(!address(&moderator, "trial"));
        let admin = view("admin", Role::Admin);
        assert!(address(&admin, "mod") && address(&admin, "admin"));
        let other_admin = view("admin2", Role::Admin);
        assert!(!address(&other_admin, "admin"));
        let maintainer = view("boss2", Role::Maintainer);
        assert!(address(&maintainer, "boss"));

        let hidden = moderator.redact(entry(Some("trial"), None, true));
        assert!(hidden.ip.is_none() && hidden.detail.is_empty());
    }
}
