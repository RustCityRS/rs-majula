use crate::audit::Event;
use crate::cases::{Case, Cases, Change, Evidence, Priority, Resolution};
use crate::http::App;
use crate::users::{name_key, now_ms};
use rs_admin_api::{
    Evasion, EvasionLookup, LinkBetween, LinkedLookup, ModerationLookup, ModerationRecord,
    ReplayLookup, ReplaySummary, ReportLookup, ReportRecord, path,
};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// How often the worlds are checked for something new.
const CHECK_INTERVAL: Duration = Duration::from_secs(60);
/// Who opens its cases, as the case log and audit trail show it.
pub const OPENED_BY: &str = "Detection";
/// The anti-cheat score from which a session is put in a case. An
/// automated browser (50) or a modified client (100) reaches it alone; the
/// weaker signs need another beside them.
pub const CASE_SCORE: i16 = 50;
/// The score, or the flags, that make a session's case high priority.
const HIGH_SCORE: i16 = 100;
const HIGH_FLAGS: [&str; 5] = [
    "modified_client",
    "auto_banned",
    "forged_heartbeats",
    "forged_minimap_click",
    "automated_browser",
];
/// How far back sessions and reports are looked through. Older ones were
/// looked at already, while the console ran.
const LOOK_BACK_MS: i64 = 24 * 3600 * 1000;
/// How long before [`LOOK_BACK_MS`] a session may have started and still be
/// scored lately.
const LONG_SESSION_MS: i64 = 6 * 24 * 3600 * 1000;
/// How far back logins are looked through for ban evasion.
const EVASION_DAYS: u32 = 3;
/// Reports against one player that open a case: at least this many over
/// [`LOOK_BACK_MS`], from at least [`REPORTERS`] different players.
const REPORTS: usize = 3;
const REPORTERS: usize = 2;
/// After a case on a player is closed with no action, the same kind of sign
/// opens no new case on them for this long. It still waits in triage.
const QUIET_MS: u64 = 7 * 24 * 3600 * 1000;
const PAGE: u32 = 100;
const MAX_PAGES: usize = 5;
/// The most lifted sanctions checked for links in one go; the rest wait for
/// the next check.
const MAX_LINK_CHECKS: usize = 50;

/// Something found about a player that moderators must look at.
#[derive(Clone, Debug)]
pub struct Finding {
    pub player: String,
    pub title: String,
    pub priority: Priority,
    pub note: String,
    pub evidence: Vec<Evidence>,
}

/// What [`file`] did with a finding.
#[derive(Debug, PartialEq)]
pub enum Filed {
    Opened(u64),
    Added(u64),
}

/// Whether `a` and `b` are the same kind of evidence.
fn same_kind(a: &Evidence, b: &Evidence) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

/// Puts what's new in `finding` in front of moderators: into the player's
/// open case (raising its priority to the finding's), or a new case. Nothing
/// already in a case or dismissed from triage is filed again, and no new
/// case is opened on a player whose case of that kind was closed with no
/// action lately.
pub fn file(cases: &Cases, finding: Finding, now_ms: u64) -> Result<Option<Filed>, String> {
    let keys: Vec<String> = finding.evidence.iter().map(Evidence::key).collect();
    let triage = cases.triage_of(&keys);
    let fresh: Vec<Evidence> = finding
        .evidence
        .into_iter()
        .filter(|e| {
            triage
                .get(&e.key())
                .is_none_or(|t| t.cases.is_empty() && t.dismissed.is_none())
        })
        .collect();
    let on_player = cases.of_player(&finding.player);
    if fresh.is_empty() {
        // Already filed, but it got worse since (a session that went on to
        // score more): its open case takes the new priority.
        let worse = on_player
            .into_iter()
            .filter(|c| c.is_open() && finding.priority > c.priority)
            .find(|c| c.evidence.iter().any(|e| keys.contains(&e.key())));
        let Some(open) = worse else {
            return Ok(None);
        };
        let changes = vec![
            Change::Priority {
                priority: finding.priority,
            },
            Change::Comment { text: finding.note },
        ];
        let case = cases.change(OPENED_BY, open.id, |_| Ok(changes))?;
        return Ok(Some(Filed::Added(case.id)));
    }
    let mut changes: Vec<Change> = fresh
        .iter()
        .map(|evidence| Change::EvidenceAdded {
            evidence: evidence.clone(),
        })
        .collect();
    changes.push(Change::Comment { text: finding.note });

    if let Some(open) = on_player
        .iter()
        .filter(|c| c.is_open())
        .max_by_key(|c| c.updated_ms)
    {
        if finding.priority > open.priority {
            changes.push(Change::Priority {
                priority: finding.priority,
            });
        }
        let case = cases.change(OPENED_BY, open.id, |_| Ok(changes))?;
        return Ok(Some(Filed::Added(case.id)));
    }
    let quiet = on_player.iter().any(|c: &Case| {
        c.resolution == Some(Resolution::NoAction)
            && c.closed_ms
                .is_some_and(|at| now_ms.saturating_sub(at) < QUIET_MS)
            && c.evidence
                .iter()
                .any(|e| fresh.iter().any(|f| same_kind(e, f)))
    });
    if quiet {
        return Ok(None);
    }
    let case = cases.create(
        OPENED_BY,
        finding.title,
        finding.player,
        finding.priority,
        changes,
    )?;
    Ok(Some(Filed::Opened(case.id)))
}

fn flag_words(flags: &[String]) -> String {
    let words: Vec<String> = flags.iter().map(|f| f.replace('_', " ")).collect();
    words.join(", ")
}

/// Each player's sessions that scored [`CASE_SCORE`] or more.
pub fn session_findings(sessions: Vec<ReplaySummary>) -> Vec<Finding> {
    let mut by_player: BTreeMap<String, Vec<ReplaySummary>> = BTreeMap::new();
    for s in sessions.into_iter().filter(|s| s.score >= CASE_SCORE) {
        by_player.entry(name_key(&s.player)).or_default().push(s);
    }
    by_player
        .into_values()
        .map(|sessions| {
            let top = sessions.iter().map(|s| s.score).max().unwrap_or(0);
            let mut flags: Vec<String> = Vec::new();
            for f in sessions.iter().flat_map(|s| &s.flags) {
                if !flags.contains(f) {
                    flags.push(f.clone());
                }
            }
            let high = top >= HIGH_SCORE || flags.iter().any(|f| HIGH_FLAGS.contains(&f.as_str()));
            let count = if sessions.len() == 1 {
                "a session".to_owned()
            } else {
                format!("{} sessions", sessions.len())
            };
            Finding {
                player: sessions[0].player.clone(),
                title: format!("Anti-cheat: {}", flag_words(&flags)),
                priority: if high { Priority::High } else { Priority::Normal },
                note: format!(
                    "**The anti-cheat checks flagged {count} of theirs**, scoring up to {top}: {}. \
                     Watch the sessions before acting: lag, touch screens and old browsers can trip the weaker checks.",
                    flag_words(&flags)
                ),
                evidence: sessions
                    .into_iter()
                    .map(|r| Evidence::Replay {
                        id: r.id,
                        player: r.player,
                        node: r.node,
                        revision: r.revision,
                        started_ms: r.started_ms,
                        duration_ms: r.duration_ms,
                        score: r.score,
                        flags: r.flags,
                    })
                    .collect(),
            }
        })
        .collect()
}

/// Each player's logins after the ban of an account they're linked to.
pub fn evasion_findings(rows: Vec<Evasion>) -> Vec<Finding> {
    let mut by_player: BTreeMap<String, Vec<Evasion>> = BTreeMap::new();
    for r in rows {
        by_player.entry(name_key(&r.name)).or_default().push(r);
    }
    by_player
        .into_values()
        .map(|rows| {
            let browser = rows.iter().any(|r| r.via == "browser");
            let lines: Vec<String> = rows
                .iter()
                .map(|r| {
                    let how = if r.via == "browser" {
                        "with the browser"
                    } else {
                        "from the address"
                    };
                    let times = if r.logins == 1 {
                        "once".to_owned()
                    } else {
                        format!("{} times", r.logins)
                    };
                    format!("Logged in {how} of banned **{}** {times} since that ban.", r.banned)
                })
                .collect();
            let caution = if browser {
                "A shared browser is a strong sign, but a family computer looks the same."
            } else {
                "An address can be a household or a shared connection."
            };
            Finding {
                player: rows[0].name.clone(),
                title: "Possible ban evasion".to_owned(),
                priority: if browser { Priority::High } else { Priority::Normal },
                note: format!(
                    "**Possible ban evasion.** {} {caution} Check the devices and links on their player page before acting.",
                    lines.join(" ")
                ),
                evidence: rows
                    .into_iter()
                    .map(|r| Evidence::Evasion {
                        player: r.name,
                        banned: r.banned,
                        via: r.via,
                        banned_ms: r.banned_ms,
                        logins: r.logins,
                        first_ms: r.first_ms,
                        last_ms: r.last_ms,
                    })
                    .collect(),
            }
        })
        .collect()
}

/// Players reported [`REPORTS`] times or more, by [`REPORTERS`] or more
/// players, with every report against them.
pub fn report_findings(reports: Vec<ReportRecord>) -> Vec<Finding> {
    let mut by_player: BTreeMap<String, Vec<ReportRecord>> = BTreeMap::new();
    for r in reports {
        by_player.entry(name_key(&r.offender)).or_default().push(r);
    }
    by_player
        .into_values()
        .filter(|rows| {
            let reporters: HashSet<String> = rows.iter().map(|r| name_key(&r.reporter)).collect();
            rows.len() >= REPORTS && reporters.len() >= REPORTERS
        })
        .map(|rows| {
            let reporters: HashSet<String> = rows.iter().map(|r| name_key(&r.reporter)).collect();
            Finding {
                player: rows[0].offender.clone(),
                title: "Reported by several players".to_owned(),
                priority: Priority::Normal,
                note: format!(
                    "**Reported {} times by {} players** in the last day. Watch their session around each report.",
                    rows.len(),
                    reporters.len()
                ),
                evidence: rows
                    .into_iter()
                    .map(|r| Evidence::Report {
                        id: r.id,
                        reporter: r.reporter,
                        offender: r.offender,
                        reason: r.reason,
                        node: r.node,
                        x: r.x,
                        z: r.z,
                        level: r.level,
                        at_ms: r.at_ms,
                    })
                    .collect(),
            }
        })
        .collect()
}

/// A sanction a staff member lifted on an account linked to theirs (`via`):
/// their own alt, or someone they play beside.
pub fn lift_finding(lift: ModerationRecord, via: String) -> Finding {
    let what = if lift.action == "unmute" {
        "mute"
    } else {
        "ban"
    };
    let how = if via == "browser" {
        "logged in with the same browser as"
    } else {
        "shares an address with"
    };
    Finding {
        player: lift.moderator.clone(),
        title: format!("Lifted a {what} on a linked account"),
        priority: Priority::High,
        note: format!(
            "**{} lifted {}'s {what}**, an account that {how} theirs. Check who {} is to them, and whether the {what} should stand.",
            lift.moderator, lift.target, lift.target
        ),
        evidence: vec![Evidence::Moderation {
            id: lift.id,
            moderator: lift.moderator,
            target: lift.target,
            action: lift.action,
            via,
            at_ms: lift.at_ms,
            reason: lift.reason.unwrap_or_default(),
        }],
    }
}

/// Mutes and bans lifted over the last day, newest first.
async fn recent_lifts(app: &App, since_ms: i64) -> Result<Vec<ModerationRecord>, String> {
    let mut found = Vec::new();
    let mut before = None;
    for _ in 0..MAX_PAGES {
        let page: Vec<ModerationRecord> = app
            .worlds
            .ask(
                path::MODERATION,
                &ModerationLookup {
                    name: None,
                    moderator: None,
                    before,
                    limit: PAGE,
                },
            )
            .await?;
        let full = page.len() == PAGE as usize;
        before = page.last().map(|r| r.id);
        let older = page.iter().any(|r| r.at_ms < since_ms);
        found
            .extend(page.into_iter().filter(|r| {
                r.at_ms >= since_ms && matches!(r.action.as_str(), "unban" | "unmute")
            }));
        if !full || older {
            break;
        }
    }
    Ok(found)
}

/// The lifts over the last day by staff linked to whoever they lifted,
/// each looked into once: `unlinked` holds those found not to be, for as
/// long as they're that recent.
async fn lift_findings(
    app: &App,
    since_ms: i64,
    unlinked: &mut HashSet<i64>,
) -> Result<Vec<Finding>, String> {
    let lifts = recent_lifts(app, since_ms).await?;
    unlinked.retain(|id| lifts.iter().any(|l| l.id == *id));
    let keys: Vec<String> = lifts
        .iter()
        .map(|l| format!("moderation:{}", l.id))
        .collect();
    let triage = app.cases.triage_of(&keys);
    let unchecked: Vec<ModerationRecord> = lifts
        .into_iter()
        .zip(keys)
        .filter(|(lift, key)| {
            let seen = triage
                .get(key)
                .is_some_and(|t| !t.cases.is_empty() || t.dismissed.is_some());
            !seen && !lift.moderator.is_empty() && !unlinked.contains(&lift.id)
        })
        .map(|(lift, _)| lift)
        .take(MAX_LINK_CHECKS)
        .collect();
    let mut findings = Vec::new();
    for lift in unchecked {
        let link: LinkBetween = app
            .worlds
            .ask(
                path::LINKED,
                &LinkedLookup {
                    name: lift.moderator.clone(),
                    other: lift.target.clone(),
                },
            )
            .await?;
        match link.via {
            Some(via) => findings.push(lift_finding(lift, via)),
            None => {
                unlinked.insert(lift.id);
            }
        }
    }
    Ok(findings)
}

/// Flagged sessions, staff's too, started over the last day.
async fn recent_sessions(app: &App, since_ms: i64) -> Result<Vec<ReplaySummary>, String> {
    let mut found = Vec::new();
    let mut before = None;
    for _ in 0..MAX_PAGES {
        let page: Vec<ReplaySummary> = app
            .worlds
            .ask(
                path::REPLAYS,
                &ReplayLookup {
                    name: None,
                    flagged: true,
                    before,
                    max_staff_level: None,
                    limit: PAGE,
                },
            )
            .await?;
        let full = page.len() == PAGE as usize;
        before = page.last().map(|r| r.started_ms);
        // A session still going was scored again lately however long ago it
        // started (a bot runs for days); one that ended before is done with.
        let older = page
            .iter()
            .any(|r| r.started_ms < since_ms - LONG_SESSION_MS);
        found.extend(page.into_iter().filter(|r| r.updated_ms >= since_ms));
        if !full || older {
            break;
        }
    }
    Ok(found)
}

/// Abuse reports sent over the last day.
async fn recent_reports(app: &App, since_ms: i64) -> Result<Vec<ReportRecord>, String> {
    let mut found = Vec::new();
    let mut before = None;
    for _ in 0..MAX_PAGES {
        let page: Vec<ReportRecord> = app
            .worlds
            .ask(
                path::REPORTS,
                &ReportLookup {
                    offender: None,
                    before,
                    limit: PAGE,
                },
            )
            .await?;
        let full = page.len() == PAGE as usize;
        before = page.last().map(|r| r.id);
        let older = page.iter().any(|r| r.at_ms < since_ms);
        found.extend(page.into_iter().filter(|r| r.at_ms >= since_ms));
        if !full || older {
            break;
        }
    }
    Ok(found)
}

/// Looks for what's new once, staff included: a case on a staff member is
/// only for staff above them (see `CaseView`). `unlinked` carries the lifts
/// already looked into from one check to the next. Returns how many cases
/// it opened or added to.
pub async fn check(app: &App, unlinked: &mut HashSet<i64>) -> Result<usize, String> {
    let since_ms = now_ms() as i64 - LOOK_BACK_MS;
    let mut findings = session_findings(recent_sessions(app, since_ms).await?);
    let evasion: Vec<Evasion> = app
        .worlds
        .ask(
            path::EVASION,
            &EvasionLookup {
                days: EVASION_DAYS,
                max_staff_level: None,
                limit: PAGE,
            },
        )
        .await?;
    findings.extend(evasion_findings(evasion));
    findings.extend(report_findings(recent_reports(app, since_ms).await?));
    findings.extend(lift_findings(app, since_ms, unlinked).await?);

    let mut filed = 0;
    for finding in findings {
        let (player, title) = (finding.player.clone(), finding.title.clone());
        match file(&app.cases, finding, now_ms()) {
            Ok(None) => {}
            Ok(Some(done)) => {
                filed += 1;
                let (action, id) = match done {
                    Filed::Opened(id) => ("case_detected", id),
                    Filed::Added(id) => ("case_detection_added", id),
                };
                info!("{title}: {player}, case #{id}");
                app.audit.record(
                    Event::system(None, action)
                        .target(&player)
                        .detail(format!("{title}, case #{id}")),
                );
            }
            Err(e) => warn!("Couldn't file \"{title}\" on {player}: {e}"),
        }
    }
    Ok(filed)
}

/// Checks every [`CHECK_INTERVAL`], while a world answers.
pub async fn keep_checking(app: Arc<App>) {
    let mut interval = tokio::time::interval(CHECK_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut failing = false;
    let mut unlinked = HashSet::new();
    loop {
        interval.tick().await;
        match check(&app, &mut unlinked).await {
            Ok(_) => failing = false,
            Err(e) if !failing => {
                warn!("Couldn't check the worlds for detections: {}", e);
                failing = true;
            }
            Err(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_log(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rs-console-detect-{tag}-{}.jsonl", now_ms()))
    }

    fn session(id: &str, player: &str, score: i16, flags: &[&str]) -> ReplaySummary {
        ReplaySummary {
            id: id.into(),
            player: player.into(),
            node: 10,
            revision: "289".into(),
            started_ms: 1,
            ended_ms: None,
            updated_ms: 1,
            duration_ms: 1000,
            bytes: 1,
            complete: false,
            score,
            flags: flags.iter().map(|f| f.to_string()).collect(),
            stats: String::new(),
        }
    }

    fn evasion(name: &str, banned: &str, via: &str) -> Evasion {
        Evasion {
            name: name.into(),
            staff_level: 0,
            banned: banned.into(),
            via: via.into(),
            banned_ms: 1,
            logins: 2,
            first_ms: 2,
            last_ms: 3,
        }
    }

    fn report(id: i64, reporter: &str, offender: &str) -> ReportRecord {
        ReportRecord {
            id,
            reporter: reporter.into(),
            offender: offender.into(),
            reason: 0,
            node: 10,
            x: 3222,
            z: 3218,
            level: 0,
            muted: false,
            at_ms: 1,
        }
    }

    #[test]
    fn only_sessions_scoring_enough_are_found() {
        let found = session_findings(vec![
            session("1", "Bob", 40, &["no_heartbeat"]),
            session("2", "Bob", 50, &["automated_browser"]),
            session("3", "bob", 140, &["modified_client", "no_heartbeat"]),
            session("4", "Eve", 65, &["no_heartbeat", "cursor_jumps"]),
        ]);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].player, "Bob");
        assert_eq!(found[0].evidence.len(), 2);
        assert_eq!(found[0].priority, Priority::High);
        assert!(
            found[0].title.contains("automated browser"),
            "{}",
            found[0].title
        );
        assert_eq!(found[1].priority, Priority::Normal);
    }

    #[test]
    fn a_shared_browser_is_high_priority_an_address_is_not() {
        let found = evasion_findings(vec![
            evasion("Alt", "Tyler", "address"),
            evasion("Alt", "Bob", "browser"),
            evasion("Kid", "Tyler", "address"),
            Evasion {
                staff_level: 1,
                ..evasion("Mod", "Tyler", "browser")
            },
        ]);
        // Staff are looked at too.
        assert_eq!(found.len(), 3);
        assert_eq!(
            (found[2].player.as_str(), found[2].priority),
            ("Mod", Priority::High)
        );
        assert_eq!(
            (found[0].player.as_str(), found[0].priority),
            ("Alt", Priority::High)
        );
        assert_eq!(found[0].evidence.len(), 2);
        assert_eq!(
            (found[1].player.as_str(), found[1].priority),
            ("Kid", Priority::Normal)
        );
        assert!(
            found[1]
                .note
                .contains("from the address of banned **Tyler** 2 times"),
            "{}",
            found[1].note
        );
    }

    #[test]
    fn several_reports_from_several_players_are_found() {
        let reports = vec![
            report(1, "Ann", "Bob"),
            report(2, "Ann", "Bob"),
            report(3, "Ann", "Bob"),
            report(4, "Ann", "Eve"),
            report(5, "Cat", "Eve"),
            report(6, "Dan", "Eve"),
            report(7, "Ann", "Mod"),
            report(8, "Cat", "Mod"),
        ];
        let found = report_findings(reports);
        // Bob: one reporter. Mod: two reports.
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].player, "Eve");
        assert_eq!(found[0].evidence.len(), 3);
    }

    #[test]
    fn a_lift_on_a_linked_account_is_on_whoever_lifted_it() {
        let lift = ModerationRecord {
            id: 41,
            moderator: "Mod".into(),
            target: "Pal".into(),
            action: "unban".into(),
            hours: None,
            node: 10,
            at_ms: 1,
            reason: Some("mistake".into()),
        };
        let found = lift_finding(lift, "browser".into());
        assert_eq!(
            (found.player.as_str(), found.priority),
            ("Mod", Priority::High)
        );
        assert_eq!(found.title, "Lifted a ban on a linked account");
        assert_eq!(found.evidence[0].key(), "moderation:41");
        assert_eq!(found.evidence[0].player(), "Mod");
        assert!(
            found.note.contains("logged in with the same browser as"),
            "{}",
            found.note
        );
    }

    #[test]
    fn findings_open_a_case_once_then_add_to_it() {
        let path = temp_log("file");
        let cases = Cases::open(path.clone());
        let first = evasion_findings(vec![evasion("Alt", "Tyler", "address")]).remove(0);
        let Some(Filed::Opened(id)) = file(&cases, first.clone(), now_ms()).unwrap() else {
            panic!("no case opened");
        };
        let case = cases.get(id).unwrap();
        assert_eq!(case.created_by, OPENED_BY);
        assert_eq!((case.evidence.len(), case.comments), (1, 1));
        // The same again changes nothing.
        assert_eq!(file(&cases, first, now_ms()).unwrap(), None);
        // Something new about them goes in the same case, raising it.
        let more = session_findings(vec![session("9", "alt", 120, &["modified_client"])]).remove(0);
        assert_eq!(
            file(&cases, more, now_ms()).unwrap(),
            Some(Filed::Added(id))
        );
        let case = cases.get(id).unwrap();
        assert_eq!((case.evidence.len(), case.priority), (2, Priority::High));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_session_that_gets_worse_raises_its_case() {
        let path = temp_log("worse");
        let cases = Cases::open(path.clone());
        let early =
            session_findings(vec![session("7", "Botty", 50, &["clicked_unseen"])]).remove(0);
        let Some(Filed::Opened(id)) = file(&cases, early, now_ms()).unwrap() else {
            panic!("no case opened");
        };
        assert_eq!(cases.get(id).unwrap().priority, Priority::Normal);
        let later = session_findings(vec![session(
            "7",
            "Botty",
            150,
            &["clicked_unseen", "forged_heartbeats"],
        )])
        .remove(0);
        assert_eq!(
            file(&cases, later.clone(), now_ms()).unwrap(),
            Some(Filed::Added(id))
        );
        let case = cases.get(id).unwrap();
        assert_eq!((case.evidence.len(), case.priority), (1, Priority::High));
        // Once it's as high as it goes, nothing more.
        assert_eq!(file(&cases, later, now_ms()).unwrap(), None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn closed_with_no_action_keeps_that_kind_quiet_for_a_while() {
        let path = temp_log("quiet");
        let cases = Cases::open(path.clone());
        let flagged = |id: &str| {
            session_findings(vec![session(id, "Tap", 50, &["automated_browser"])]).remove(0)
        };
        let Some(Filed::Opened(id)) = file(&cases, flagged("1"), now_ms()).unwrap() else {
            panic!("no case opened");
        };
        cases
            .change("Ash", id, |_| {
                Ok(vec![Change::Closed {
                    resolution: Resolution::NoAction,
                    note: "Their phone".into(),
                }])
            })
            .unwrap();
        // Another session like it: no new case this week.
        assert_eq!(file(&cases, flagged("2"), now_ms()).unwrap(), None);
        // Something else about them still opens one.
        let other = evasion_findings(vec![evasion("Tap", "Tyler", "browser")]).remove(0);
        assert!(matches!(
            file(&cases, other, now_ms()).unwrap(),
            Some(Filed::Opened(_))
        ));
        // And after the week, sessions do too.
        let path2 = temp_log("quiet2");
        let fresh = Cases::open(path2.clone());
        let Some(Filed::Opened(id)) = file(&fresh, flagged("1"), now_ms()).unwrap() else {
            panic!("no case opened");
        };
        fresh
            .change("Ash", id, |_| {
                Ok(vec![Change::Closed {
                    resolution: Resolution::NoAction,
                    note: String::new(),
                }])
            })
            .unwrap();
        let later = now_ms() + QUIET_MS + 1;
        let r = file(&fresh, flagged("2"), later);
        assert!(matches!(r, Ok(Some(Filed::Opened(_)))), "{r:?}");
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path2);
    }
}
