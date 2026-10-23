use ammonia::Builder;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::collections::HashSet;

/// File types that play as video rather than show as an image.
const VIDEO_EXTENSIONS: [&str; 3] = [".mp4", ".webm", ".m4v"];

fn is_video(url: &str) -> bool {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .to_ascii_lowercase();
    VIDEO_EXTENSIONS.iter().any(|ext| path.ends_with(ext))
}

/// `text` safe inside a double-quoted attribute or element text.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// The HTML a post written in `markdown` shows as.
pub fn render(markdown: &str) -> String {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let mut events = Parser::new_ext(markdown, options);
    let mut out_events = Vec::new();
    while let Some(event) = events.next() {
        let Event::Start(Tag::Image {
            dest_url, title, ..
        }) = event
        else {
            out_events.push(event);
            continue;
        };
        // The caption is the text up to the image's end.
        let mut alt = String::new();
        for inner in events.by_ref() {
            match inner {
                Event::End(TagEnd::Image) => break,
                Event::Text(text) | Event::Code(text) => alt.push_str(&text),
                _ => {}
            }
        }
        let (src, alt, title) = (escape(&dest_url), escape(&alt), escape(&title));
        let title = if title.is_empty() {
            String::new()
        } else {
            format!(" title=\"{title}\"")
        };
        let html = if is_video(&dest_url) {
            format!(
                "<video class=\"news-media\" src=\"{src}\" controls preload=\"metadata\" playsinline{title} aria-label=\"{alt}\"></video>"
            )
        } else {
            format!(
                "<img class=\"news-media\" src=\"{src}\" alt=\"{alt}\"{title} loading=\"lazy\">"
            )
        };
        out_events.push(Event::InlineHtml(html.into()));
    }
    let mut html = String::with_capacity((markdown.len() * 3) >> 1);
    pulldown_cmark::html::push_html(&mut html, out_events.into_iter());
    sanitize(&html)
}

/// `html` with only what a post may show: the formatting old posts were
/// written in, and none of what runs scripts or dresses up the page
/// (event handlers, `javascript:` links, styles beyond alignment).
pub fn sanitize(html: &str) -> String {
    Builder::default()
        .add_tags(["font", "video"])
        .add_generic_attributes(["align", "valign", "width", "height", "bgcolor", "style"])
        .filter_style_properties(HashSet::from(["text-align"]))
        .add_tag_attributes("font", ["color", "size", "face"])
        .add_tag_attributes("img", ["border", "loading"])
        .add_tag_attributes("table", ["border", "cellpadding", "cellspacing"])
        .add_tag_attributes(
            "video",
            ["src", "controls", "preload", "playsinline", "aria-label"],
        )
        .add_allowed_classes("img", ["news-media"])
        .add_allowed_classes("video", ["news-media"])
        .url_schemes(HashSet::from(["http", "https", "mailto"]))
        .clean(html)
        .to_string()
}

/// The text of a post's (sanitized) `html`, for a link preview: tags taken
/// out, whitespace collapsed, and cut at a word to at most `max` characters.
pub fn excerpt(html: &str, max: usize) -> String {
    // Tags that end a line of text, so the words either side stay apart.
    const BREAKS: [&str; 12] = [
        "p", "br", "div", "li", "td", "th", "tr", "h1", "h2", "h3", "h4", "hr",
    ];
    let mut text = String::with_capacity(html.len());
    let mut tag: Option<String> = None;
    for c in html.chars() {
        match (&mut tag, c) {
            (None, '<') => tag = Some(String::new()),
            (Some(name), '>') => {
                let name = name.trim_start_matches('/');
                let name = name.split([' ', '/']).next().unwrap_or("");
                if BREAKS.contains(&name.to_ascii_lowercase().as_str()) {
                    text.push(' ');
                }
                tag = None;
            }
            (Some(name), c) => name.push(c),
            (None, c) => text.push(c),
        }
    }
    let text = text
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&");
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut out = String::new();
    for word in words {
        if out.chars().count() + word.chars().count() + 1 > max {
            out.push_str("...");
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

/// Where a post's (sanitized) `html` shows its first image, if it has one.
pub fn first_image(html: &str) -> Option<String> {
    let tag = &html[html.find("<img ")?..];
    let tag = &tag[..tag.find('>')?];
    let src = &tag[tag.find(" src=\"")? + 6..];
    let src = &src[..src.find('"')?];
    Some(src.replace("&amp;", "&"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_take_the_text_and_first_image() {
        let html = render(
            "Hello **there**, &amp; welcome.\n\n![Map](/news/media/ab.gif)\n\n![x](/news/media/cd.png)",
        );
        assert_eq!(excerpt(&html, 200), "Hello there, & welcome.");
        assert_eq!(
            excerpt("<p>one</p><p>two<br>three</p>", 200),
            "one two three"
        );
        assert_eq!(excerpt(&html, 12), "Hello there,...");
        assert_eq!(first_image(&html).as_deref(), Some("/news/media/ab.gif"));
        assert_eq!(first_image("<p>none</p>"), None);
    }

    #[test]
    fn renders_markdown() {
        let html =
            render("# Update\n\nSome **bold** and a [link](https://example.com).\n\n- one\n- two");
        assert!(html.contains("<h1>Update</h1>"), "{html}");
        assert!(html.contains("<strong>bold</strong>"), "{html}");
        assert!(
            html.contains("<a href=\"https://example.com\" rel=\"noopener noreferrer\">link</a>"),
            "{html}"
        );
        assert!(html.contains("<li>one</li>"), "{html}");
    }

    #[test]
    fn images_and_gifs_are_images() {
        let html = render("![The *new* map](/news/media/ab.gif \"Map\")");
        assert!(
            html.contains(
                "<img class=\"news-media\" src=\"/news/media/ab.gif\" alt=\"The new map\" title=\"Map\" loading=\"lazy\">"
            ),
            "{html}"
        );
    }

    #[test]
    fn videos_play() {
        let html = render("![Trailer](/news/media/cd.MP4)");
        assert!(
            html.contains("<video class=\"news-media\" src=\"/news/media/cd.MP4\" controls"),
            "{html}"
        );
        assert!(!html.contains("<img"), "{html}");
    }

    #[test]
    fn captions_are_escaped() {
        let html = render("![a \"quote\" <b>](/x.png)");
        assert!(html.contains("alt=\"a &quot;quote&quot; \""), "{html}");
    }

    #[test]
    fn old_formatting_passes_through() {
        let html = render("<p align=center><b>Old post</b> <font color=red>here</font></p>");
        assert!(
            html.contains(
                "<p align=\"center\"><b>Old post</b> <font color=\"red\">here</font></p>"
            ),
            "{html}"
        );
        let html = render(
            "| a | b |
|:-:|--|
| 1 | 2 |",
        );
        assert!(
            html.contains("<th style=\"text-align:center\">a</th>"),
            "{html}"
        );
    }

    #[test]
    fn scripts_never_get_through() {
        let html = render(concat!(
            "<script>alert(1)</script>

",
            "<img src=x onerror=alert(2)> <a href=\"javascript:alert(3)\">a</a>

",
            "[b](javascript:alert(4)) <div style=\"position:fixed;top:0\">c</div>

",
            "<iframe src=\"https://example.com\"></iframe>",
        ));
        for bad in ["script", "alert", "onerror", "position", "iframe"] {
            assert!(!html.contains(bad), "{bad}: {html}");
        }
    }
}
