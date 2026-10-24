use super::lookup::unavailable;
use crate::audit::Entry;
use crate::cases::Case;
use crate::http::{ApiError, App, Staff};
use crate::users::{Role, name_key};
use axum::http::StatusCode;
use rs_admin_api::{
    LinkBetween, LinkedLookup, PlayerLookup, PlayerRecord, ReplayGet, ReplaySummary,
    StaffAccountRecord, StaffAccountsLookup, path,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::warn;

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

/// Every staff account's game level by name key, as a world last listed
/// them. Kept when the worlds can't be reached, so who's staff is never
/// forgotten.
#[derive(Default)]
pub struct Roster(Mutex<HashMap<String, u8>>);

impl Roster {
    fn levels(&self) -> HashMap<String, u8> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn set(&self, accounts: Vec<StaffAccountRecord>) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = accounts
            .into_iter()
            .map(|a| (name_key(&a.name), a.staff_level))
            .collect();
    }
}

/// How often the roster is listed again.
const ROSTER_EVERY: Duration = Duration::from_secs(60);

/// Keeps [`Roster`] up to date.
pub async fn keep_roster(app: Arc<App>) {
    let mut interval = tokio::time::interval(ROSTER_EVERY);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut failing = false;
    loop {
        interval.tick().await;
        let listed: Result<Vec<StaffAccountRecord>, String> = app
            .worlds
            .ask(path::STAFF_ACCOUNTS, &StaffAccountsLookup {})
            .await;
        match listed {
            Ok(accounts) => {
                app.roster.set(accounts);
                failing = false;
            }
            Err(e) if !failing => {
                warn!("Couldn't list the staff accounts: {}", e);
                failing = true;
            }
            Err(_) => {}
        }
    }
}

/// Each staff member's level by name key: the game's, or their console
/// role's where that's higher (console accounts are their game names).
fn staff_levels(app: &App) -> HashMap<String, u8> {
    let mut levels = app.roster.levels();
    for user in app.users.all().unwrap_or_default() {
        let level = levels.entry(name_key(&user.name)).or_insert(0);
        *level = (*level).max(user.role.staff_level());
    }
    levels
}

/// Who may see the cases and staff notes on whom. Nobody sees those on
/// themselves, and those on staff are only for staff above them, so a
/// staff member who's up to something can't watch them or close them, nor
/// can their peers. Maintainers see those on everyone else, and whoever
/// opened a case or is on it keeps seeing it.
pub(crate) struct CaseView {
    levels: HashMap<String, u8>,
    me: String,
    level: u8,
    maintainer: bool,
}

impl CaseView {
    pub(crate) fn new(app: &App, staff: &Staff) -> Self {
        Self {
            levels: staff_levels(app),
            me: name_key(staff.name()),
            level: staff.user.role.staff_level(),
            maintainer: staff.user.role == Role::Maintainer,
        }
    }

    /// Whether they see what's noted about `player`.
    pub(crate) fn sees_subject(&self, player: &str) -> bool {
        let key = name_key(player);
        if key == self.me {
            return false;
        }
        let level = self.levels.get(&key).copied().unwrap_or(0);
        level == 0 || self.maintainer || self.level > level
    }

    pub(crate) fn sees(&self, case: &Case) -> bool {
        name_key(&case.player) != self.me
            && (self.sees_subject(&case.player) || case.involves(&self.me))
    }

    /// Whether they see an audit entry about `target` (a case, note or
    /// appeal on them) that `actor` made.
    fn sees_about(&self, actor: Option<&str>, target: &str) -> bool {
        actor.is_some_and(|a| name_key(a) == self.me) || self.sees_subject(target)
    }
}

/// What staff do about a player that the player mustn't see, nor staff at
/// or below them: [`CaseView`] decides who sees these entries.
const ABOUT_ACTIONS: [&str; 11] = [
    "case_opened",
    "case_closed",
    "case_reopened",
    "case_detected",
    "case_detection_added",
    "note_added",
    "note_removed",
    "appeal_received",
    "appeal_decided",
    "recovery_received",
    "recovery_decided",
];

/// Refuses `staff` doing `what` about `name` when it's their own game
/// account, or one sharing a browser or an address with it: someone else
/// must. Maintainers aren't held to it (they run the machines, so the
/// console couldn't hold them); the console notices instead, as with
/// sanctions lifted on linked accounts. Refused too when no world can say.
pub(crate) async fn check_conflict(
    app: &App,
    staff: &Staff,
    name: &str,
    what: &str,
) -> Result<(), ApiError> {
    if staff.user.role == Role::Maintainer {
        return Ok(());
    }
    if name_key(name) == name_key(staff.name()) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            format!("That's your own account, so someone else must {what}."),
        ));
    }
    let link: LinkBetween = app
        .worlds
        .ask(
            path::LINKED,
            &LinkedLookup {
                name: staff.name().to_owned(),
                other: name.to_owned(),
            },
        )
        .await
        .map_err(|e| {
            ApiError::new(
                StatusCode::BAD_GATEWAY,
                format!(
                    "Couldn't check whether {name} is linked to your account ({e}). Try again."
                ),
            )
        })?;
    match link.via.as_deref() {
        None => Ok(()),
        Some(via) => {
            let how = if via == "browser" {
                "logged in with the same browser as"
            } else {
                "shares an address with"
            };
            Err(ApiError::new(
                StatusCode::FORBIDDEN,
                format!("{name} {how} your account, so someone else must {what}."),
            ))
        }
    }
}

/// Whether `staff` may look themselves up: only maintainers, who run the
/// machines anyway. Everyone else has another staff member do it.
pub(super) fn may_look_up_self(staff: &Staff) -> bool {
    staff.user.role == Role::Maintainer
}

/// Whether `name` is `staff`'s own account and they may not look it up, for
/// leaving it out of lists.
pub(super) fn hides_self(staff: &Staff, name: &str) -> bool {
    !may_look_up_self(staff) && name_key(name) == name_key(staff.name())
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
    if hides_self(staff, name) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "That's your own account: another staff member looks into anything about it.",
        ));
    }
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
    /// Who sees entries about cases, notes and appeals on whom.
    about: CaseView,
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
            about: CaseView::new(app, staff),
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
            && (!ABOUT_ACTIONS.contains(&e.action.as_str())
                || e.target
                    .as_deref()
                    .is_none_or(|t| self.about.sees_about(e.actor.as_deref(), t)))
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

    fn levels() -> HashMap<String, u8> {
        [("trial", 1), ("mod", 1), ("admin", 2), ("boss", 3)]
            .into_iter()
            .map(|(n, l)| (n.to_owned(), l))
            .collect()
    }

    fn cases(me: &str, role: Role) -> CaseView {
        CaseView {
            levels: levels(),
            me: me.to_owned(),
            level: role.staff_level(),
            maintainer: role == Role::Maintainer,
        }
    }

    fn view(me: &str, role: Role) -> AuditView {
        AuditView {
            levels: levels(),
            me: me.to_owned(),
            level: role.staff_level(),
            maintainer: role == Role::Maintainer,
            about: cases(me, role),
        }
    }

    #[test]
    fn cases_on_staff_are_for_those_above_them() {
        let moderator = cases("mod", Role::Moderator);
        assert!(moderator.sees_subject("a player"));
        // Not themselves, nor a peer, nor anyone above.
        assert!(!moderator.sees_subject("Mod"));
        assert!(!moderator.sees_subject("trial"));
        assert!(!moderator.sees_subject("admin"));
        let admin = cases("admin", Role::Admin);
        assert!(admin.sees_subject("mod") && !admin.sees_subject("admin"));
        assert!(!admin.sees_subject("boss"));
        // Maintainers see everyone but themselves.
        let maintainer = cases("boss2", Role::Maintainer);
        assert!(maintainer.sees_subject("boss") && maintainer.sees_subject("admin"));
        assert!(!cases("boss", Role::Maintainer).sees_subject("boss"));

        // The case log: a peer's case seen only once they're on it.
        let path = std::env::temp_dir().join(format!(
            "rs-console-caseview-{}.jsonl",
            crate::users::now_ms()
        ));
        let log = crate::cases::Cases::open(path.clone());
        let on_peer = log
            .create(
                "Detection",
                "T".into(),
                "trial".into(),
                crate::cases::Priority::Normal,
                vec![],
            )
            .unwrap();
        assert!(!moderator.sees(&on_peer));
        let on_peer = log
            .change("admin", on_peer.id, |_| {
                Ok(vec![crate::cases::Change::Assigned { name: "mod".into() }])
            })
            .unwrap();
        assert!(moderator.sees(&on_peer));
        assert!(!cases("trial", Role::Trial).sees(&on_peer));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn what_is_done_about_staff_is_left_out_for_them_and_their_peers() {
        let about = |actor: Option<&str>, target: &str| Entry {
            action: "case_detected".into(),
            ..entry(actor, Some(target), false)
        };
        let moderator = view("mod", Role::Moderator);
        assert!(moderator.shows(&about(None, "a player")));
        assert!(!moderator.shows(&about(None, "mod")));
        assert!(!moderator.shows(&about(None, "trial")));
        // What they did themselves, they see.
        let note = Entry {
            action: "note_added".into(),
            ..entry(Some("mod"), Some("trial"), false)
        };
        assert!(moderator.shows(&note));
        assert!(view("admin", Role::Admin).shows(&about(None, "mod")));
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
