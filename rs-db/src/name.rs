use rs_util::base37::{to_screen_name, to_userhash};
use std::fmt;

/// A player name that passed [`validate_name`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerName {
    pub hash: u64,
    pub display: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameError {
    Length,
    Characters,
    Separators,
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NameError::Length => "Usernames must be between 1 and 12 characters long.",
            NameError::Characters => {
                "Usernames may only contain letters, numbers, spaces and underscores."
            }
            NameError::Separators => {
                "Usernames must start and end with a letter or number, and cannot contain two spaces or underscores in a row."
            }
        })
    }
}

/// Checks a name a player wants to register.
///
/// The game folds case and treats spaces and underscores alike, and base37
/// silently drops anything else, so this only admits names that survive that
/// round trip unchanged in meaning: 1-12 letters, digits, spaces or
/// underscores, starting and ending with a letter or digit, with no two
/// separators in a row (`a__b` would display the same as `a_b`).
pub fn validate_name(input: &str) -> Result<PlayerName, NameError> {
    let name = input.trim();
    if name.is_empty() || name.chars().count() > 12 {
        return Err(NameError::Length);
    }

    let is_separator = |c: char| c == ' ' || c == '_';
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || is_separator(c))
    {
        return Err(NameError::Characters);
    }

    let bytes = name.as_bytes();
    let edges_ok =
        bytes[0].is_ascii_alphanumeric() && bytes[bytes.len() - 1].is_ascii_alphanumeric();
    let doubled = bytes
        .windows(2)
        .any(|w| is_separator(w[0] as char) && is_separator(w[1] as char));
    if !edges_ok || doubled {
        return Err(NameError::Separators);
    }

    Ok(PlayerName {
        hash: to_userhash(name),
        display: to_screen_name(name),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_and_normalises() {
        let name = validate_name("  red_Rooster ").unwrap();
        assert_eq!(name.display, "Red Rooster");
        assert_eq!(name.hash, to_userhash("red rooster"));
        assert_eq!(validate_name("Zezima").unwrap().display, "Zezima");
        assert!(validate_name("abcdefghijk1").is_ok());
    }

    #[test]
    fn spellings_of_one_name_share_a_hash() {
        let a = validate_name("Red Rooster").unwrap();
        let b = validate_name("red_rooster").unwrap();
        assert_eq!(a.hash, b.hash);
    }

    #[test]
    fn rejects_bad_names() {
        assert_eq!(validate_name(""), Err(NameError::Length));
        assert_eq!(validate_name("   "), Err(NameError::Length));
        assert_eq!(validate_name("abcdefghijklm"), Err(NameError::Length));
        assert_eq!(validate_name("bob!"), Err(NameError::Characters));
        assert_eq!(validate_name("b\u{f6}b"), Err(NameError::Characters));
        assert_eq!(validate_name("_bob"), Err(NameError::Separators));
        assert_eq!(validate_name("bob_"), Err(NameError::Separators));
        assert_eq!(validate_name("bo__b"), Err(NameError::Separators));
        assert_eq!(validate_name("bo _b"), Err(NameError::Separators));
    }
}
