use crate::appeals::OPENED_BY;
use crate::audit;
use crate::cases::{Case, Change, Decision, Dismissal};
use crate::users::{Role, name_key};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

/// How many of someone's overturned appeals are listed.
const OVERTURNED_LISTED: usize = 10;

/// What one staff member did over a stretch of time.
#[derive(Clone, Debug, Default, Serialize)]
pub struct StaffActivity {
    pub name: String,
    pub role: Option<Role>,
    pub kicks: u32,
    pub mutes: u32,
    pub bans: u32,
    pub lifts: u32,
    pub cases_opened: u32,
    pub cases_closed: u32,
    pub comments: u32,
    pub dismissed: u32,
    pub appeals_decided: u32,
    pub appealed: u32,
    pub overturned: u32,
    pub overturned_cases: Vec<u64>,
    pub pickup_ms: Option<u64>,
    pub pickups: u32,
    pub close_ms: Option<u64>,
    pub decide_ms: Option<u64>,
    pub last_active_ms: Option<u64>,
}

#[derive(Default)]
struct Tally {
    activity: StaffActivity,
    pickups: Vec<u64>,
    closes: Vec<u64>,
    decides: Vec<u64>,
    overturned: Vec<(u64, u64)>,
}

impl Tally {
    fn active(&mut self, at_ms: u64) {
        let last = &mut self.activity.last_active_ms;
        *last = Some(last.map_or(at_ms, |l| l.max(at_ms)));
    }
}

/// `name`'s tally, started if it isn't yet.
fn tally<'a>(tallies: &'a mut BTreeMap<String, Tally>, name: &str) -> &'a mut Tally {
    let t = tallies.entry(name_key(name)).or_default();
    if t.activity.name.is_empty() {
        t.activity.name = name.to_owned();
    }
    t
}

fn median(mut values: Vec<u64>) -> Option<u64> {
    values.sort_unstable();
    let n = values.len();
    match n {
        0 => None,
        _ if n % 2 == 1 => Some(values[n / 2]),
        _ => Some((values[n / 2 - 1] + values[n / 2]) / 2),
    }
}

/// What each of `staff` (and anyone else who shows up) did from `since_ms`
/// on: their commands from the audit trail (`entries`, already from then
/// on), and their part in cases and the triage queue. Most active first.
pub fn of(
    staff: &[(String, Role)],
    cases: &BTreeMap<u64, Case>,
    dismissed: &HashMap<String, Dismissal>,
    entries: &[audit::Entry],
    since_ms: u64,
) -> Vec<StaffActivity> {
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for (name, role) in staff {
        let t = tally(&mut tallies, name);
        t.activity.name = name.clone();
        t.activity.role = Some(*role);
    }

    for e in entries.iter().filter(|e| e.ok && e.at_ms >= since_ms) {
        let Some(actor) = &e.actor else { continue };
        let t = tally(&mut tallies, actor);
        let a = &mut t.activity;
        match e.action.as_str() {
            "kick" => a.kicks += 1,
            "mute" => a.mutes += 1,
            "ban" => a.bans += 1,
            "unmute" | "unban" => a.lifts += 1,
            _ => continue,
        }
        t.active(e.at_ms);
    }

    for d in dismissed.values().filter(|d| d.at_ms >= since_ms) {
        let t = tally(&mut tallies, &d.by);
        t.activity.dismissed += 1;
        t.active(d.at_ms);
    }

    for case in cases.values() {
        let opener = name_key(&case.created_by);
        let mut touched: Vec<String> = vec![opener.clone()];
        for e in &case.timeline {
            let by = name_key(&e.by);
            let recent = e.at_ms >= since_ms;
            // Their first change on a case someone else opened.
            if !touched.contains(&by) {
                touched.push(by.clone());
                if recent {
                    tally(&mut tallies, &e.by)
                        .pickups
                        .push(e.at_ms.saturating_sub(case.created_ms));
                }
            }
            if !recent || e.by == OPENED_BY {
                continue;
            }
            let t = tally(&mut tallies, &e.by);
            t.active(e.at_ms);
            let mut issued = None;
            match &e.change {
                Change::Opened { .. } => t.activity.cases_opened += 1,
                Change::Comment { .. } => t.activity.comments += 1,
                Change::Closed { .. } => {
                    t.activity.cases_closed += 1;
                    t.closes.push(e.at_ms.saturating_sub(case.created_ms));
                }
                Change::AppealDecided { decision, .. } => {
                    t.activity.appeals_decided += 1;
                    if let Some(state) = &case.appeal {
                        let filed = u64::try_from(state.appeal.filed_ms).unwrap_or(0);
                        t.decides.push(e.at_ms.saturating_sub(filed));
                        issued = state.appeal.issued_by.as_deref().map(|by| (by, *decision));
                    }
                }
                _ => {}
            }
            // How the one who issued the sanction fared.
            if let Some((issuer, decision)) = issued {
                let i = tally(&mut tallies, issuer);
                i.activity.appealed += 1;
                if decision == Decision::Overturned {
                    i.activity.overturned += 1;
                    i.overturned.push((e.at_ms, case.id));
                }
            }
        }
    }

    let mut out: Vec<StaffActivity> = tallies
        .into_values()
        .map(|mut t| {
            t.overturned.sort_unstable_by(|a, b| b.cmp(a));
            let a = &mut t.activity;
            a.overturned_cases = t
                .overturned
                .iter()
                .take(OVERTURNED_LISTED)
                .map(|&(_, id)| id)
                .collect();
            a.pickups = t.pickups.len() as u32;
            a.pickup_ms = median(t.pickups);
            a.close_ms = median(t.closes);
            a.decide_ms = median(t.decides);
            t.activity
        })
        // Those with no account and nothing done are nobody.
        .filter(|a| a.role.is_some() || a.last_active_ms.is_some() || a.appealed > 0)
        .collect();
    out.sort_by_key(|a| {
        let done = a.kicks
            + a.mutes
            + a.bans
            + a.lifts
            + a.cases_closed
            + a.comments
            + a.dismissed
            + a.appeals_decided;
        (std::cmp::Reverse(done), name_key(&a.name))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cases::{Appeal, Cases, Priority, Resolution};
    use crate::users::now_ms;
    use std::net::{IpAddr, Ipv4Addr};

    fn entry(actor: &str, action: &str, ok: bool) -> audit::Entry {
        audit::Entry {
            id: 0,
            at_ms: now_ms(),
            actor: Some(actor.into()),
            ip: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            action: action.into(),
            target: None,
            detail: String::new(),
            ok,
        }
    }

    #[test]
    fn activity_counts_commands_cases_and_appeals() {
        let path = std::env::temp_dir().join(format!("rs-admin-activity-{}.jsonl", now_ms()));
        let cases = Cases::open(path.clone());
        let opened = cases
            .create("Kim", "Spam".into(), "Bob".into(), Priority::Normal, vec![])
            .unwrap();
        cases
            .change("Ash", opened.id, |_| {
                Ok(vec![
                    Change::Comment {
                        text: "Looking".into(),
                    },
                    Change::Closed {
                        resolution: Resolution::Actioned,
                        note: String::new(),
                    },
                ])
            })
            .unwrap();
        let appeal = cases
            .create(
                OPENED_BY,
                "Ban appeal #1".into(),
                "Bob".into(),
                Priority::Normal,
                vec![Change::AppealFiled {
                    appeal: Appeal {
                        id: 1,
                        sanction: "ban".into(),
                        until_ms: 0,
                        issued_by: Some("Trainee".into()),
                        reason: None,
                        statement: "sorry".into(),
                        filed_ms: 0,
                    },
                }],
            )
            .unwrap();
        cases
            .change("Ash", appeal.id, |_| {
                Ok(vec![Change::AppealDecided {
                    decision: Decision::Overturned,
                    note: String::new(),
                }])
            })
            .unwrap();
        cases.triage("Kim", "report:3".into(), true).unwrap();

        let audit = [
            entry("Kim", "mute", true),
            entry("Kim", "ban", false),
            entry("Trainee", "ban", true),
        ];
        let staff = [
            ("Ash".to_owned(), Role::Admin),
            ("Kim".to_owned(), Role::Moderator),
        ];
        let found = cases.read(|all, dismissed| of(&staff, all, dismissed, &audit, 0));
        let by = |name: &str| found.iter().find(|a| a.name == name).unwrap();

        let ash = by("Ash");
        assert_eq!(
            (ash.comments, ash.cases_closed, ash.appeals_decided),
            (1, 1, 1)
        );
        assert_eq!(ash.pickups, 2);
        assert!(ash.close_ms.is_some() && ash.decide_ms.is_some());
        let kim = by("Kim");
        // A refused command doesn't count.
        assert_eq!(
            (kim.mutes, kim.bans, kim.cases_opened, kim.dismissed),
            (1, 0, 1, 1)
        );
        assert_eq!(kim.pickups, 0);
        // Someone with no account still shows, for their overturned appeal.
        let trainee = by("Trainee");
        assert_eq!(trainee.role, None);
        assert_eq!(
            (trainee.bans, trainee.appealed, trainee.overturned),
            (1, 1, 1)
        );
        assert_eq!(trainee.overturned_cases, [appeal.id]);
        // Nothing before the stretch counts.
        let later = cases.read(|all, dismissed| of(&staff, all, dismissed, &[], now_ms() + 1000));
        assert!(
            later
                .iter()
                .all(|a| a.last_active_ms.is_none() && a.pickups == 0)
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn medians_take_the_middle() {
        assert_eq!(median(vec![]), None);
        assert_eq!(median(vec![5, 1, 3]), Some(3));
        assert_eq!(median(vec![4, 1, 2, 3]), Some(2));
    }
}
