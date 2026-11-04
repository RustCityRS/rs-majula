//! 289's text markup in 2011's.
//!
//! 289's content colours text with the 2004 client's five-character codes
//! (`@dre@Cook@dbl@`), which its `Sp` table maps to RGB, with `@str@` and
//! `@end@` turning a strikethrough on and off; the client swallows any other
//! `@xxx@` unseen. 647's client knows none of them: its renderer
//! (`AClass14`) reads `<col=rrggbb>`, `<str>`, `</str>` and the like, so a
//! 289 quest journal would print its codes. Each text-carrying 647 encoder
//! passes its text through [`markup`] for the client to draw the colours 289
//! drew.

use std::borrow::Cow;

/// The 2004 client's colour codes (289 `client.js`, `Sp`).
const COLOURS: [(&[u8; 3], &str); 17] = [
    (b"red", "ff0000"),
    (b"gre", "00ff00"),
    (b"blu", "0000ff"),
    (b"yel", "ffff00"),
    (b"cya", "00ffff"),
    (b"mag", "ff00ff"),
    (b"whi", "ffffff"),
    (b"bla", "000000"),
    (b"lre", "ff9040"),
    (b"dre", "800000"),
    (b"dbl", "000080"),
    (b"or1", "ffb000"),
    (b"or2", "ff7000"),
    (b"or3", "ff3000"),
    (b"gr1", "c0ff00"),
    (b"gr2", "80ff00"),
    (b"gr3", "40ff00"),
];

/// `text` with its 2004 `@xxx@` codes as 2011 tags; unchanged when it has
/// none.
pub fn markup(text: &str) -> Cow<'_, str> {
    let bytes = text.as_bytes();
    let is_code = |i: usize| bytes[i] == b'@' && i + 4 < bytes.len() && bytes[i + 4] == b'@';
    if !(0..bytes.len()).any(is_code) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 16);
    let mut i = 0;
    let mut from = 0;
    while i < bytes.len() {
        if !is_code(i) {
            i += 1;
            continue;
        }
        out.push_str(&text[from..i]);
        let code = &bytes[i + 1..i + 4];
        if let Some((_, rgb)) = COLOURS.iter().find(|(c, _)| c.as_slice() == code) {
            out.push_str("<col=");
            out.push_str(rgb);
            out.push('>');
        } else if code == b"str" {
            out.push_str("<str>");
        } else if code == b"end" {
            out.push_str("</str>");
        }
        i += 5;
        from = i;
    }
    out.push_str(&text[from..]);
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::markup;

    #[test]
    fn codes_become_2011_tags() {
        assert_eq!(
            markup("@dbl@I can start this quest by speaking to the @dre@Cook@dbl@."),
            "<col=000080>I can start this quest by speaking to the <col=800000>Cook<col=000080>."
        );
        assert_eq!(
            markup("@str@It was the Duke's birthday@end@"),
            "<str>It was the Duke's birthday</str>"
        );
        assert_eq!(
            markup("@lre@x@or2@y@gr3@z"),
            "<col=ff9040>x<col=ff7000>y<col=40ff00>z"
        );
    }

    #[test]
    fn plain_text_and_stray_ats_pass_through() {
        assert!(matches!(
            markup("You steal a cake."),
            std::borrow::Cow::Borrowed(_)
        ));
        assert_eq!(markup("mail@example.com"), "mail@example.com");
        assert_eq!(markup("a @ bb @ c"), "a @ bb @ c");
    }

    /// The 2004 client draws no unknown code; neither does 2011's.
    #[test]
    fn an_unknown_code_is_dropped() {
        assert_eq!(markup("@xyz@hidden"), "hidden");
    }
}
