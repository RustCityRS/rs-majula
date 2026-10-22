use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

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
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_markdown() {
        let html =
            render("# Update\n\nSome **bold** and a [link](https://example.com).\n\n- one\n- two");
        assert!(html.contains("<h1>Update</h1>"), "{html}");
        assert!(html.contains("<strong>bold</strong>"), "{html}");
        assert!(
            html.contains("<a href=\"https://example.com\">link</a>"),
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
    fn html_passes_through() {
        let html = render("<p align=center><b>Old post</b></p>");
        assert!(
            html.contains("<p align=center><b>Old post</b></p>"),
            "{html}"
        );
    }
}
