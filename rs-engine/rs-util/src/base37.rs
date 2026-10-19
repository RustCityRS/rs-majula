/// Lookup table for decoding a base37 digit back to its character representation.
///
/// Index 0 maps to `'_'` (underscore / separator), indices 1-26 map to `'a'`-`'z'`,
/// and indices 27-36 map to `'0'`-`'9'`. This table is the inverse of the encoding
/// logic in [`to_userhash`] and is used by [`to_raw_username`] to reconstruct a
/// username string from its numeric hash.
const USERHASH_CHAR: [char; 37] = [
    '_', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
    's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
];

/// Encodes a player username string into a base37 hash.
///
/// Each character is mapped to a base37 digit: `A`-`Z` and `a`-`z` map to 1-26
/// (case-insensitive), `0`-`9` map to 27-36, and all other characters (including
/// underscore) map to 0. Only the first 12 characters are considered; any remainder
/// is silently ignored. After encoding, trailing zero-digits (underscores / special
/// characters at the end) are stripped by dividing out factors of 37.
///
/// # Arguments
///
/// * `string` - The username to encode. Leading and trailing whitespace is trimmed
///   before processing. Maximum effective length is 12 characters.
///
/// # Returns
///
/// A `u64` base37 hash of the username. Returns `0` for empty or whitespace-only strings.
///
/// # Call Stack
///
/// **Called by:** [`to_safe_name`], [`to_screen_name`] (indirectly), player name handling
/// throughout rs-engine (friend lists, ignore lists, chat messages).
/// **Calls:** Nothing.
pub fn to_userhash(string: &str) -> u64 {
    let string = string.trim();
    let mut l: u64 = 0;

    for (i, c) in string.chars().enumerate() {
        if i >= 12 {
            break;
        }
        let c = c as u32;
        l *= 37;

        if (0x41..=0x5a).contains(&c) {
            l += (c + 1 - 0x41) as u64;
        } else if (0x61..=0x7a).contains(&c) {
            l += (c + 1 - 0x61) as u64;
        } else if (0x30..=0x39).contains(&c) {
            l += (c + 27 - 0x30) as u64;
        }
    }

    while l.is_multiple_of(37) && l != 0 {
        l /= 37;
    }

    l
}

/// Whether `value` is a base37 hash [`to_raw_username`] can decode: in range,
/// and not ending in an underscore (divisible by 37).
pub const fn is_valid_userhash(value: u64) -> bool {
    value < 6582952005840035281 && !value.is_multiple_of(37)
}

/// Decodes a base37 hash back into a lowercase username string.
///
/// Repeatedly divides the hash by 37, using the remainder as an index into
/// [`USERHASH_CHAR`] to recover each character from least-significant to
/// most-significant digit. The result is a lowercase string of up to 12
/// characters.
///
/// # Arguments
///
/// * `value` - The base37 hash to decode. Must be in the range `1..6582952005840035281`
///   and must not be evenly divisible by 37 (such values would represent names
///   with trailing underscores, which are not valid after normalisation).
///
/// # Returns
///
/// The decoded lowercase username string, or `"invalid_name"` if `value` is out of
/// range or divisible by 37.
///
/// # Call Stack
///
/// **Called by:** [`to_safe_name`], player name display paths throughout rs-engine.
/// **Calls:** Nothing (indexes into [`USERHASH_CHAR`]).
pub fn to_raw_username(mut value: u64) -> String {
    if !is_valid_userhash(value) {
        return "invalid_name".to_string();
    }

    let mut chars = ['\0'; 12];
    let mut len = 0usize;

    while value != 0 {
        let l1 = value;
        value /= 37;
        chars[11 - len] = USERHASH_CHAR[(l1 - value * 37) as usize];
        len += 1;
    }

    chars[12 - len..].iter().collect()
}

/// Normalizes a username by encoding it to a base37 hash and decoding it back.
///
/// This round-trip through [`to_userhash`] and [`to_raw_username`] lowercases the
/// name, strips unsupported characters (replacing them with underscores), removes
/// trailing underscores, and truncates to 12 characters. The result is the canonical
/// lowercase form of the username.
///
/// # Arguments
///
/// * `name` - The raw username string to normalize.
///
/// # Returns
///
/// The normalized lowercase username, or `"invalid_name"` if the round-trip produces
/// an invalid hash (e.g. empty string).
///
/// # Call Stack
///
/// **Called by:** [`to_screen_name`], player name storage paths throughout rs-engine.
/// **Calls:** [`to_userhash`], [`to_raw_username`].
pub fn to_safe_name(name: &str) -> String {
    to_raw_username(to_userhash(name))
}

/// Converts a username into its display-ready screen name.
///
/// First normalizes the name via [`to_safe_name`], then replaces underscores with
/// spaces and applies [`to_title_case`] so that each word is capitalized. For
/// example, `"hello_world"` becomes `"Hello World"`.
///
/// # Arguments
///
/// * `name` - The raw username string to convert.
///
/// # Returns
///
/// A title-cased, space-separated display name suitable for rendering on screen.
///
/// # Call Stack
///
/// **Called by:** Chat message formatting, player display throughout rs-engine.
/// **Calls:** [`to_safe_name`], [`to_title_case`].
pub fn to_screen_name(name: &str) -> String {
    to_title_case(&to_safe_name(name).replace('_', " "))
}

/// Converts a string to title case, capitalizing the first letter of each
/// whitespace-delimited word and lowercasing the rest.
///
/// Words are split by `split_whitespace`, so multiple consecutive spaces, tabs,
/// or other Unicode whitespace are collapsed into a single space in the output.
///
/// # Arguments
///
/// * `s` - The input string. May be empty, single-word, or multi-word.
///
/// # Returns
///
/// A new `String` where every word has its first character uppercased and all
/// remaining characters lowercased. Words are joined by a single space. Returns
/// an empty string if the input is empty or contains only whitespace.
///
/// # Call Stack
///
/// **Called by:** [`to_screen_name`].
/// **Calls:** Nothing (uses standard iterator and `char` methods).
pub fn to_title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn userhash_lowercase() {
        let hash = to_userhash("jordan");
        assert!(hash > 0);
    }

    #[test]
    fn userhash_uppercase_same_as_lowercase() {
        assert_eq!(to_userhash("Jordan"), to_userhash("jordan"));
        assert_eq!(to_userhash("HELLO"), to_userhash("hello"));
    }

    #[test]
    fn userhash_with_underscore() {
        let hash = to_userhash("hello_world");
        assert!(hash > 0);
    }

    #[test]
    fn userhash_with_digits() {
        let hash = to_userhash("player123");
        assert!(hash > 0);
    }

    #[test]
    fn userhash_empty_string() {
        assert_eq!(to_userhash(""), 0);
    }

    #[test]
    fn userhash_truncates_at_12_chars() {
        let a = to_userhash("abcdefghijkl");
        let b = to_userhash("abcdefghijklmnop");
        assert_eq!(a, b);
    }

    #[test]
    fn userhash_trims_whitespace() {
        assert_eq!(to_userhash("  hello  "), to_userhash("hello"));
    }

    #[test]
    fn userhash_trailing_underscore_normalized() {
        let h1 = to_userhash("hello_");
        let h2 = to_userhash("hello");
        // trailing underscore = char value 0, so 37*x + 0 gets divided out
        assert_eq!(h1, h2);
    }

    #[test]
    fn raw_username_round_trip() {
        let hash = to_userhash("jordan");
        let name = to_raw_username(hash);
        assert_eq!(name, "jordan");
    }

    #[test]
    fn raw_username_round_trip_digits() {
        let hash = to_userhash("abc123");
        let name = to_raw_username(hash);
        assert_eq!(name, "abc123");
    }

    #[test]
    fn raw_username_round_trip_underscore() {
        let hash = to_userhash("hello_world");
        let name = to_raw_username(hash);
        assert_eq!(name, "hello_world");
    }

    #[test]
    fn raw_username_zero_returns_invalid() {
        assert_eq!(to_raw_username(0), "invalid_name");
    }

    #[test]
    fn raw_username_out_of_range_returns_invalid() {
        assert_eq!(to_raw_username(6582952005840035281), "invalid_name");
        assert_eq!(to_raw_username(u64::MAX), "invalid_name");
    }

    #[test]
    fn raw_username_mod37_zero_returns_invalid() {
        assert_eq!(to_raw_username(37), "invalid_name");
        assert_eq!(to_raw_username(37 * 37), "invalid_name");
    }

    #[test]
    fn safe_name_normalizes() {
        assert_eq!(to_safe_name("JORDAN"), "jordan");
        assert_eq!(to_safe_name("Jordan"), "jordan");
    }

    #[test]
    fn safe_name_round_trip() {
        let name = to_safe_name("player_one");
        assert_eq!(name, "player_one");
    }

    #[test]
    fn screen_name_title_case() {
        assert_eq!(to_screen_name("hello_world"), "Hello World");
    }

    #[test]
    fn screen_name_single_word() {
        assert_eq!(to_screen_name("jordan"), "Jordan");
    }

    #[test]
    fn screen_name_all_caps_normalized() {
        assert_eq!(to_screen_name("HELLO_WORLD"), "Hello World");
    }

    #[test]
    fn title_case_basic() {
        assert_eq!(to_title_case("hello world"), "Hello World");
    }

    #[test]
    fn title_case_already_correct() {
        assert_eq!(to_title_case("Hello World"), "Hello World");
    }

    #[test]
    fn title_case_all_caps() {
        assert_eq!(to_title_case("HELLO WORLD"), "Hello World");
    }

    #[test]
    fn title_case_single_word() {
        assert_eq!(to_title_case("hello"), "Hello");
    }

    #[test]
    fn title_case_empty() {
        assert_eq!(to_title_case(""), "");
    }

    #[test]
    fn title_case_single_char() {
        assert_eq!(to_title_case("a"), "A");
    }

    #[test]
    fn userhash_single_char_letters() {
        for c in 'a'..='z' {
            let hash = to_userhash(&c.to_string());
            let back = to_raw_username(hash);
            assert_eq!(back, c.to_string());
        }
    }

    #[test]
    fn userhash_single_digit() {
        for d in '0'..='9' {
            let hash = to_userhash(&d.to_string());
            let back = to_raw_username(hash);
            assert_eq!(back, d.to_string());
        }
    }

    #[test]
    fn userhash_max_length_name() {
        let name = "abcdefghijkl"; // 12 chars
        let hash = to_userhash(name);
        let back = to_raw_username(hash);
        assert_eq!(back, name);
    }

    #[test]
    fn userhash_special_chars_ignored() {
        // Characters outside a-z, 0-9 are treated as 0 (underscore)
        let h1 = to_userhash("a!b");
        let h2 = to_userhash("a_b");
        assert_eq!(h1, h2);
    }

    #[test]
    fn userhash_mixed_case_digits() {
        assert_eq!(to_userhash("Player1"), to_userhash("player1"));
    }

    #[test]
    fn raw_username_boundary_value() {
        // Just below the upper boundary
        let hash = to_userhash("zzzzzzzzzzzz");
        let back = to_raw_username(hash);
        assert_eq!(back, "zzzzzzzzzzzz");
    }

    #[test]
    fn screen_name_multiple_underscores() {
        assert_eq!(to_screen_name("a_b_c"), "A B C");
    }

    #[test]
    fn screen_name_numeric_name() {
        assert_eq!(to_screen_name("123"), "123");
    }

    #[test]
    fn userhash_consistency_across_calls() {
        let h1 = to_userhash("test_name");
        let h2 = to_userhash("test_name");
        assert_eq!(h1, h2);
    }

    #[test]
    fn safe_name_strips_special_chars() {
        let safe = to_safe_name("Hello!World");
        // ! becomes underscore in encoding, trailing underscore stripped
        assert!(!safe.contains('!'));
    }

    #[test]
    fn title_case_mixed_whitespace() {
        // split_whitespace handles multiple spaces
        assert_eq!(to_title_case("hello   world"), "Hello World");
    }

    #[test]
    fn round_trip_all_underscore_name() {
        // leading underscore = 0 in base37, gets divided out
        let hash = to_userhash("_a");
        let back = to_raw_username(hash);
        // The leading underscore encodes as 0*37 + ...
        assert_eq!(back, "a");
    }
}
