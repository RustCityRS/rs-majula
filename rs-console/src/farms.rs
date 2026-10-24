use rs_admin_api::AccountActivity;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// What a pair must weigh to be linked.
const LINK_WEIGHT: u32 = 3;
/// Accounts registered this close together.
const TOGETHER_MS: i64 = 15 * 60 * 1000;
/// Click rhythms this close (see the anti-cheat checks), when both are this
/// even: people's are far apart and well above it.
const RHYTHM_DELTA: f64 = 0.02;
const RHYTHM_EVEN: f64 = 0.25;
/// Saved this close together, in tiles.
const SPOT_TILES: i32 = 5;
/// The share of their playing hours two accounts have in common.
const HOURS_OVERLAP: f64 = 0.6;
/// An address shared by more accounts than this is a network's (a mobile
/// carrier, a VPN) and links nobody.
const MAX_ADDRESS_SHARE: usize = 25;

/// Why two accounts are linked.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reason {
    Registered { ip: String },
    LoggedIn { ip: String },
    Together,
    Names,
    Rhythm,
    Spot,
    Hours,
}

impl Reason {
    fn weight(&self) -> u32 {
        match self {
            Self::Registered { .. } => 3,
            Self::LoggedIn { .. } | Self::Rhythm => 2,
            Self::Together | Self::Names | Self::Spot | Self::Hours => 1,
        }
    }

    /// Whether it's what the accounts do rather than where they're from.
    fn behaviour(&self) -> bool {
        !matches!(self, Self::Registered { .. } | Self::LoggedIn { .. })
    }
}

/// One shared trait of a group, and how many of its accounts share it.
#[derive(Clone, Debug, Serialize)]
pub struct Shared {
    #[serde(flatten)]
    pub reason: Reason,
    pub accounts: usize,
}

#[derive(Debug, Serialize)]
pub struct Farm {
    /// Indexes into the accounts grouped, most active first.
    pub members: Vec<usize>,
    pub shared: Vec<Shared>,
    /// How suspicious the group looks: bigger, more alike, flagged.
    pub score: u32,
}

/// The letters of a name, lowercased: `Goblin_Slayer42` -> `goblinslayer`.
fn stem(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Whether two names' [`stem`]s are alike.
fn stems_alike(a: &str, b: &str) -> bool {
    let common = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
    (a == b && a.len() >= 3) || common >= 5
}

/// The hours of the day an account played in, a bit each.
fn hour_mask(hours: &[i32]) -> u32 {
    hours
        .iter()
        .take(24)
        .enumerate()
        .filter(|&(_, &n)| n > 0)
        .fold(0, |mask, (i, _)| mask | 1 << i)
}

fn hours_alike(a: u32, b: u32) -> bool {
    a.count_ones() >= 2
        && b.count_ones() >= 2
        && f64::from((a & b).count_ones()) / f64::from((a | b).count_ones()) >= HOURS_OVERLAP
}

/// What's compared of each account, worked out once.
struct Traits {
    stem: String,
    hours: u32,
}

/// What `a` and `b` share, besides addresses (found separately).
fn behaviours(
    (a, at): (&AccountActivity, &Traits),
    (b, bt): (&AccountActivity, &Traits),
    out: &mut Vec<Reason>,
) {
    if (a.registered_ms - b.registered_ms).abs() <= TOGETHER_MS {
        out.push(Reason::Together);
    }
    if stems_alike(&at.stem, &bt.stem) {
        out.push(Reason::Names);
    }
    if let (Some(x), Some(y)) = (a.rhythm, b.rhythm)
        && x < RHYTHM_EVEN
        && y < RHYTHM_EVEN
        && (x - y).abs() <= RHYTHM_DELTA
    {
        out.push(Reason::Rhythm);
    }
    if let (Some(ax), Some(az), Some(bx), Some(bz)) = (a.x, a.z, b.x, b.z)
        && a.level == b.level
        && (i32::from(ax) - i32::from(bx)).abs() <= SPOT_TILES
        && (i32::from(az) - i32::from(bz)).abs() <= SPOT_TILES
    {
        out.push(Reason::Spot);
    }
    if hours_alike(at.hours, bt.hours) {
        out.push(Reason::Hours);
    }
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The groups of at least `min_size` linked accounts, most suspicious first.
pub fn group(accounts: &[AccountActivity], min_size: usize) -> Vec<Farm> {
    let n = accounts.len();
    // Who registered or logged in from each address.
    let mut registered: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut logged_in: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, a) in accounts.iter().enumerate() {
        if let Some(ip) = &a.registration_ip {
            registered.entry(ip).or_default().push(i);
        }
        for ip in &a.login_ips {
            logged_in.entry(ip).or_default().push(i);
        }
    }
    let mut reasons: HashMap<(usize, usize), Vec<Reason>> = HashMap::new();
    let address_pairs =
        |by: &HashMap<&str, Vec<usize>>,
         make: fn(String) -> Reason,
         reasons: &mut HashMap<(usize, usize), Vec<Reason>>| {
            for (ip, list) in by {
                if list.len() < 2 || list.len() > MAX_ADDRESS_SHARE {
                    continue;
                }
                for (k, &i) in list.iter().enumerate() {
                    for &j in &list[k + 1..] {
                        reasons
                            .entry((i.min(j), i.max(j)))
                            .or_default()
                            .push(make((*ip).to_owned()));
                    }
                }
            }
        };
    address_pairs(&registered, |ip| Reason::Registered { ip }, &mut reasons);
    address_pairs(&logged_in, |ip| Reason::LoggedIn { ip }, &mut reasons);
    // By the first of each pair, the second in order, to walk alongside
    // every pair rather than look each up.
    let mut by_first: Vec<Vec<(usize, Vec<Reason>)>> = vec![Vec::new(); n];
    for ((i, j), why) in reasons {
        by_first[i].push((j, why));
    }
    for pairs in &mut by_first {
        pairs.sort_unstable_by_key(|&(j, _)| j);
    }

    let traits: Vec<Traits> = accounts
        .iter()
        .map(|a| Traits {
            stem: stem(&a.name),
            hours: hour_mask(&a.hours),
        })
        .collect();
    let mut parent: Vec<usize> = (0..n).collect();
    let mut links: Vec<(usize, usize, Vec<Reason>)> = Vec::new();
    let mut found = Vec::new();
    for i in 0..n {
        let mut shared = std::mem::take(&mut by_first[i]).into_iter().peekable();
        for j in i + 1..n {
            found.clear();
            behaviours(
                (&accounts[i], &traits[i]),
                (&accounts[j], &traits[j]),
                &mut found,
            );
            let by_address = shared
                .next_if(|&(k, _)| k == j)
                .map(|(_, why)| why)
                .unwrap_or_default();
            if found.is_empty() && by_address.is_empty() {
                continue;
            }
            // An address counts once, however many the pair share.
            let address = by_address.iter().map(Reason::weight).max().unwrap_or(0)
                + u32::from(by_address.len() > 1);
            let weight = address + found.iter().map(Reason::weight).sum::<u32>();
            if weight >= LINK_WEIGHT {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                parent[a] = b;
                found.extend(by_address);
                links.push((i, j, found.clone()));
            }
        }
    }

    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        let root = find(&mut parent, i);
        groups.entry(root).or_default().push(i);
    }
    let mut shared_by: HashMap<usize, BTreeMap<Reason, BTreeSet<usize>>> = HashMap::new();
    for (i, j, why) in links {
        let root = find(&mut parent, i);
        let shared = shared_by.entry(root).or_default();
        for reason in why {
            let members = shared.entry(reason).or_default();
            members.insert(i);
            members.insert(j);
        }
    }

    let mut farms: Vec<Farm> = groups
        .into_iter()
        .filter(|(_, members)| members.len() >= min_size.max(2))
        .map(|(root, mut members)| {
            members.sort_by_key(|&i| std::cmp::Reverse(accounts[i].play_ms));
            let mut shared: Vec<Shared> = shared_by
                .remove(&root)
                .unwrap_or_default()
                .into_iter()
                .map(|(reason, who)| Shared {
                    reason,
                    accounts: who.len(),
                })
                .collect();
            shared.sort_by_key(|s| (std::cmp::Reverse(s.accounts), s.reason.clone()));
            let behaviours = shared.iter().filter(|s| s.reason.behaviour()).count();
            let flagged = members.iter().filter(|&&i| accounts[i].flagged > 0).count();
            let cheating = members.iter().any(|&i| accounts[i].max_score >= 100);
            let score = (members.len() as u32 - 1) * 10
                + behaviours as u32 * 10
                + flagged as u32 * 15
                + if cheating { 20 } else { 0 };
            Farm {
                members,
                shared,
                score,
            }
        })
        .collect();
    farms.sort_by_key(|f| {
        (
            std::cmp::Reverse(f.score),
            std::cmp::Reverse(f.members.len()),
        )
    });
    farms
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(name: &str) -> AccountActivity {
        AccountActivity {
            name: name.to_owned(),
            registered_ms: 0,
            registration_ip: None,
            login_ips: vec![],
            sessions: 1,
            play_ms: 1000,
            flagged: 0,
            max_score: 0,
            rhythm: None,
            hours: vec![0; 24],
            x: None,
            z: None,
            level: None,
            online_node: None,
            muted_until_ms: None,
            banned_until_ms: None,
        }
    }

    fn names(accounts: &[AccountActivity], farm: &Farm) -> Vec<String> {
        let mut out: Vec<String> = farm
            .members
            .iter()
            .map(|&i| accounts[i].name.clone())
            .collect();
        out.sort();
        out
    }

    #[test]
    fn a_shared_registration_address_links() {
        let mut a = account("alice");
        let mut b = account("bob");
        a.registration_ip = Some("203.0.113.7".into());
        b.registration_ip = Some("203.0.113.7".into());
        // Years apart, nothing else alike.
        b.registered_ms = 10 * 365 * 86_400_000;
        let accounts = [a, b, account("carol")];
        let farms = group(&accounts, 2);
        assert_eq!(farms.len(), 1);
        assert_eq!(names(&accounts, &farms[0]), ["alice", "bob"]);
        assert_eq!(
            farms[0].shared[0].reason,
            Reason::Registered {
                ip: "203.0.113.7".into()
            }
        );
    }

    #[test]
    fn one_behaviour_alone_links_nobody() {
        let mut a = account("alice");
        let mut b = account("bob");
        b.registered_ms = 86_400_000;
        (a.x, a.z, a.level) = (Some(3222), Some(3218), Some(0));
        (b.x, b.z, b.level) = (Some(3223), Some(3219), Some(0));
        assert!(group(&[a, b], 2).is_empty());
    }

    #[test]
    fn behaviours_together_link_a_farm() {
        let farm: Vec<AccountActivity> = (0..4)
            .map(|i| {
                let mut a = account(&format!("miner{i}"));
                a.registered_ms = i64::from(i) * 60_000;
                a.rhythm = Some(0.05 + f64::from(i) * 0.002);
                (a.x, a.z, a.level) = (Some(3290), Some(3370), Some(0));
                a.hours[3] = 2;
                a.hours[4] = 2;
                a.flagged = 1;
                a
            })
            .collect();
        let mut accounts = farm;
        let mut person = account("someone");
        person.registered_ms = 120_000;
        person.rhythm = Some(0.6);
        accounts.push(person);
        let farms = group(&accounts, 2);
        assert_eq!(farms.len(), 1);
        assert_eq!(farms[0].members.len(), 4);
        let kinds: Vec<&Reason> = farms[0].shared.iter().map(|s| &s.reason).collect();
        assert!(kinds.contains(&&Reason::Rhythm), "{kinds:?}");
        assert!(kinds.contains(&&Reason::Spot), "{kinds:?}");
        // 3 more than one, 5 kinds of behaviour, 4 flagged.
        assert_eq!(farms[0].score, 30 + 50 + 60);
    }

    #[test]
    fn a_network_address_links_nobody() {
        let accounts: Vec<AccountActivity> = (0..30)
            .map(|i| {
                let mut a = account(&format!("p{i}x"));
                a.registered_ms = i64::from(i) * 86_400_000;
                a.login_ips = vec!["198.51.100.1".into()];
                a
            })
            .collect();
        assert!(group(&accounts, 2).is_empty());
    }

    #[test]
    fn names_alike_ignore_digits_and_separators() {
        let alike = |a, b| stems_alike(&stem(a), &stem(b));
        assert!(alike("Goblin_Slayer1", "goblin slayer22"));
        assert!(alike("woodcutter01", "woodcutterpro"));
        assert!(!alike("ab1", "ab2"));
        assert!(!alike("alice", "bob"));
    }

    #[test]
    fn hours_overlap_by_share() {
        let mut a = vec![0; 24];
        let mut b = vec![0; 24];
        a[1..5].fill(1);
        b[2..5].fill(3);
        // 3 of 4.
        assert!(hours_alike(hour_mask(&a), hour_mask(&b)));
        b[10..14].fill(1);
        // 3 of 8.
        assert!(!hours_alike(hour_mask(&a), hour_mask(&b)));
    }
}
