use rs_admin_api::{ClaimCheck, PlayerSheet, Verdict};
use rs_db::validate_name;

/// How far off a skill level can be and still be close.
const CLOSE_LEVELS: i32 = 3;
/// The most things of each kind checked.
const MAX_CLAIMS: usize = 20;
/// The shortest item name worth searching for.
const MIN_ITEM_CHARS: usize = 3;

/// Skill names as players shorten them, to the names the sheet gives.
const ALIASES: [(&str, &str); 24] = [
    ("att", "attack"),
    ("atk", "attack"),
    ("str", "strength"),
    ("def", "defence"),
    ("defense", "defence"),
    ("hp", "hitpoints"),
    ("hits", "hitpoints"),
    ("range", "ranged"),
    ("ranging", "ranged"),
    ("mage", "magic"),
    ("pray", "prayer"),
    ("wc", "woodcutting"),
    ("fm", "firemaking"),
    ("fletch", "fletching"),
    ("cook", "cooking"),
    ("fish", "fishing"),
    ("mine", "mining"),
    ("smith", "smithing"),
    ("craft", "crafting"),
    ("herb", "herblore"),
    ("thieve", "thieving"),
    ("thief", "thieving"),
    ("agil", "agility"),
    ("rc", "runecraft"),
];

/// The things in a list a player typed: `a, b and c` is three.
pub fn listed(text: &str) -> Vec<String> {
    text.split([',', ';', '\n', '/'])
        .flat_map(|part| part.split(" and "))
        .map(|part| part.trim().trim_matches(['.', '-']).trim().to_owned())
        .filter(|part| !part.is_empty())
        .take(MAX_CLAIMS)
        .collect()
}

fn check(said: &str, verdict: Verdict, detail: String) -> ClaimCheck {
    ClaimCheck {
        said: said.to_owned(),
        verdict,
        detail,
    }
}

/// Each skill and level they gave (`Woodcutting 60`, `combat 45`, `total
/// 500`) against the account's base levels.
pub fn check_skills(text: &str, sheet: &PlayerSheet) -> Vec<ClaimCheck> {
    listed(text)
        .iter()
        .map(|said| {
            let lower = said.to_lowercase();
            let level: Option<i32> = lower
                .split(|c: char| !c.is_ascii_digit())
                .find(|n| !n.is_empty())
                .and_then(|n| n.parse().ok());
            let words: String = lower
                .chars()
                .filter(|c| c.is_ascii_alphabetic() || *c == ' ')
                .collect();
            let word = words
                .split_whitespace()
                .filter(|w| !matches!(*w, "lvl" | "level" | "levels" | "is" | "my" | "about"))
                .collect::<Vec<_>>()
                .join(" ");
            let found = match word.as_str() {
                "combat" | "cb" | "cmb" | "combat level" => {
                    Some(("Combat".to_owned(), i32::from(sheet.level)))
                }
                "total" | "total level" => Some((
                    "Total level".to_owned(),
                    sheet.stats.iter().map(|s| i32::from(s.base)).sum(),
                )),
                _ => {
                    let skill = ALIASES
                        .iter()
                        .find(|(short, _)| *short == word)
                        .map_or(word.as_str(), |(_, full)| full);
                    sheet
                        .stats
                        .iter()
                        .find(|s| {
                            let name = s.name.to_lowercase();
                            name == skill || (skill.len() >= 3 && name.starts_with(skill))
                        })
                        .map(|s| (s.name.clone(), i32::from(s.base)))
                }
            };
            match (found, level) {
                (Some((name, actual)), Some(level)) => {
                    let off = (actual - level).abs();
                    let verdict = match off {
                        0 => Verdict::Match,
                        off if off <= CLOSE_LEVELS => Verdict::Close,
                        _ => Verdict::No,
                    };
                    check(said, verdict, format!("{name} is {actual}"))
                }
                _ => check(said, Verdict::Unknown, "Not a skill and level".to_owned()),
            }
        })
        .collect()
}

/// An item as players write it: no amount (`10k coins`, `2x lobster`), and
/// lower case.
fn item_name(said: &str) -> String {
    said.to_lowercase()
        .split_whitespace()
        .filter(|w| {
            let w = w.trim_start_matches('x').trim_end_matches(['x', 'k', 'm']);
            w.is_empty() || !w.chars().all(|c| c.is_ascii_digit() || c == '.')
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Each item they named against what the account holds, anywhere.
pub fn check_items(text: &str, sheet: &PlayerSheet) -> Vec<ClaimCheck> {
    let held: Vec<(String, &str, &str)> = sheet
        .invs
        .iter()
        .flat_map(|inv| {
            inv.items.iter().map(move |item| {
                (
                    item.name.to_lowercase(),
                    item.name.as_str(),
                    inv.name.as_str(),
                )
            })
        })
        .collect();
    listed(text)
        .iter()
        .map(|said| {
            let wanted = item_name(said);
            if wanted.chars().count() < MIN_ITEM_CHARS {
                return check(said, Verdict::Unknown, String::new());
            }
            let singular = wanted.strip_suffix('s').unwrap_or(&wanted);
            if let Some((_, name, inv)) = held
                .iter()
                .find(|(lower, ..)| *lower == wanted || *lower == singular)
            {
                return check(said, Verdict::Match, format!("{name} in {inv}"));
            }
            match held
                .iter()
                .find(|(lower, ..)| lower.contains(singular) || wanted.contains(lower.as_str()))
            {
                Some((_, name, inv)) => check(said, Verdict::Close, format!("{name} in {inv}")),
                None => check(said, Verdict::No, "Not held now".to_owned()),
            }
        })
        .collect()
}

/// Each player they named against `known` (name hashes).
pub fn check_names(text: &str, known: &[u64]) -> Vec<ClaimCheck> {
    listed(text)
        .iter()
        .map(|said| match validate_name(said) {
            Ok(name) if known.contains(&name.hash) => check(said, Verdict::Match, String::new()),
            Ok(_) => check(said, Verdict::No, String::new()),
            Err(_) => check(said, Verdict::Unknown, "Not a name".to_owned()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_admin_api::{InvSheet, ItemLine, StatLine};

    fn sheet() -> PlayerSheet {
        let stat = |id, name: &str, base| StatLine {
            id,
            name: name.to_owned(),
            level: base,
            base,
            xp: 0,
        };
        let item = |slot, name: &str| ItemLine {
            slot,
            id: slot,
            name: name.to_owned(),
            count: 1,
        };
        PlayerSheet {
            name: "Test".into(),
            online: false,
            x: 3222,
            z: 3218,
            level: 45,
            stats: vec![
                stat(0, "Attack", 40),
                stat(8, "Woodcutting", 60),
                stat(20, "Runecraft", 12),
            ],
            invs: vec![InvSheet {
                name: "bank".into(),
                size: 496,
                items: vec![
                    item(0, "Rune scimitar"),
                    item(1, "Lobster"),
                    item(2, "Coins"),
                ],
            }],
        }
    }

    fn verdicts(checks: &[ClaimCheck]) -> Vec<Verdict> {
        checks.iter().map(|c| c.verdict).collect()
    }

    #[test]
    fn lists_split_however_typed() {
        assert_eq!(listed("a, b and c;d\n\ne."), ["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn skills_match_by_name_or_shorthand() {
        let checks = check_skills(
            "Woodcutting 60, att lvl 42, rc 30, combat 45, total 112, cooking 10, lots",
            &sheet(),
        );
        use Verdict::*;
        assert_eq!(
            verdicts(&checks),
            [Match, Close, No, Match, Match, Unknown, Unknown]
        );
        assert_eq!(checks[2].detail, "Runecraft is 12");
    }

    #[test]
    fn items_match_without_amounts_or_plurals() {
        let checks = check_items(
            "Rune scimitar, 20 lobsters, 10k coins, scim, dragon med",
            &sheet(),
        );
        use Verdict::*;
        assert_eq!(verdicts(&checks), [Match, Match, Match, Close, No]);
        assert_eq!(checks[1].detail, "Lobster in bank");
    }

    #[test]
    fn names_match_by_hash() {
        let zezima = validate_name("Zezima").unwrap().hash;
        let checks = check_names("zezima, Durial321, not a real name!", &[zezima]);
        assert_eq!(
            verdicts(&checks),
            [Verdict::Match, Verdict::No, Verdict::Unknown]
        );
    }
}
