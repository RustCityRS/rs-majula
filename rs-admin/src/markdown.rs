use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd};
use rs_admin_api::media;

/// Where case uploads are served from.
pub const MEDIA_PATH: &str = "/api/cases/media/";

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

/// The upload `url` names, if it's one.
fn upload(url: &str) -> Option<&str> {
    url.strip_prefix(MEDIA_PATH)
        .filter(|name| media::valid_name(name))
}

/// The HTML a comment written in `markdown` shows as.
pub fn render(markdown: &str) -> String {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let mut events = Parser::new_ext(markdown, options);
    let mut out = Vec::new();
    while let Some(event) = events.next() {
        let (dest_url, title) = match event {
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => (dest_url, title),
            Event::Html(html) | Event::InlineHtml(html) => {
                out.push(Event::Text(html));
                continue;
            }
            Event::SoftBreak => {
                out.push(Event::HardBreak);
                continue;
            }
            event => {
                out.push(event);
                continue;
            }
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
        let Some(name) = upload(&dest_url) else {
            // Nothing from elsewhere loads: it stays a link.
            out.push(Event::InlineHtml(CowStr::from(format!(
                "<a href=\"{}\">{}</a>",
                escape(&dest_url),
                escape(if alt.is_empty() { &dest_url } else { &alt })
            ))));
            continue;
        };
        let (src, alt, title) = (
            escape(&format!("{MEDIA_PATH}{name}")),
            escape(&alt),
            escape(&title),
        );
        let title = if title.is_empty() {
            String::new()
        } else {
            format!(" title=\"{title}\"")
        };
        let html = if name.ends_with(".mp4") || name.ends_with(".webm") {
            format!(
                "<video src=\"{src}\" controls preload=\"metadata\" playsinline{title} aria-label=\"{alt}\"></video>"
            )
        } else {
            format!("<a href=\"{src}\"><img src=\"{src}\" alt=\"{alt}\"{title}></a>")
        };
        out.push(Event::InlineHtml(html.into()));
    }
    let mut html = String::with_capacity((markdown.len() * 3) >> 1);
    pulldown_cmark::html::push_html(&mut html, out.into_iter());
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &str = "/api/cases/media/0123456789abcdef0123456789abcdef.png";
    const MP4: &str = "/api/cases/media/0123456789abcdef0123456789abcdef.mp4";

    #[test]
    fn renders_markdown_with_line_breaks() {
        let html = render("Seen **twice**\nat the GE.\n\n- one");
        assert!(html.contains("<strong>twice</strong><br />"), "{html}");
        assert!(html.contains("<li>one</li>"), "{html}");
    }

    #[test]
    fn uploads_embed_and_videos_play() {
        let html = render(&format!("![The ad]({PNG})"));
        assert!(
            html.contains(&format!(
                "<a href=\"{PNG}\"><img src=\"{PNG}\" alt=\"The ad\"></a>"
            )),
            "{html}"
        );
        let html = render(&format!("![Clip]({MP4})"));
        assert!(
            html.contains(&format!("<video src=\"{MP4}\" controls")),
            "{html}"
        );
    }

    #[test]
    fn nothing_else_embeds() {
        let html = render("![x](https://example.com/a.png) ![y](/api/cases/media/../users.json)");
        assert!(!html.contains("<img"), "{html}");
        assert!(
            html.contains("<a href=\"https://example.com/a.png\">x</a>"),
            "{html}"
        );
    }

    #[test]
    fn html_shows_as_text() {
        let html = render("<script>alert(1)</script> and <b>bold</b>");
        assert!(!html.contains("<script"), "{html}");
        assert!(!html.contains("<b>"), "{html}");
        assert!(html.contains("&lt;b&gt;"), "{html}");
    }
}
