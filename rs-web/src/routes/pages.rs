use crate::app::{App, AppError};
use axum::extract::{Query, State};
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use minijinja::context;
use minijinja::value::Serde;
use serde::Deserialize;
use std::sync::Arc;

/// Serves `templates/pages/<path>.html` or `templates/pages/<path>/index.html`.
pub async fn page(State(app): State<Arc<App>>, uri: Uri) -> Result<Response, AppError> {
    let path = uri.path().trim_matches('/');
    if !is_page_path(path) {
        return Ok(not_found());
    }
    let candidates = if path.is_empty() {
        vec!["pages/index.html".to_string()]
    } else {
        vec![
            format!("pages/{path}.html"),
            format!("pages/{path}/index.html"),
        ]
    };
    render_first(&app, &candidates)
}

#[derive(Deserialize)]
pub struct PollQuery {
    pollid: Option<String>,
}

/// `/results?pollid=N`, the archived form of `/polls/N`.
pub async fn poll_results(
    State(app): State<Arc<App>>,
    Query(q): Query<PollQuery>,
) -> Result<Response, AppError> {
    match q.pollid.filter(|id| is_number(id)) {
        Some(id) => render_first(&app, &[format!("pages/polls/{id}.html")]),
        None => Ok(not_found()),
    }
}

/// Frames a world's map viewer, which draws that world's revision. Prefers a
/// world that is online and open, then one that is online.
pub async fn worldmap(State(app): State<Arc<App>>) -> Result<Response, AppError> {
    let worlds = app.worlds_now();
    let world = worlds
        .iter()
        .find(|w| w.online && !w.maintenance)
        .or_else(|| worlds.iter().find(|w| w.online))
        .or(worlds.first());
    let html = app.templates.render(
        "pages/worldmap/applet.html",
        context! { world => world.map(Serde) },
    )?;
    Ok(html.into_response())
}

fn render_first(app: &App, names: &[String]) -> Result<Response, AppError> {
    for name in names {
        if let Some(html) = app.templates.render_if_exists(name)? {
            return Ok(html.into_response());
        }
    }
    Ok(not_found())
}

pub fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "Page not found").into_response()
}

/// Only plain `a-z0-9_-` segments, so a URL can't reach outside `pages/`.
fn is_page_path(path: &str) -> bool {
    path.is_empty()
        || path.split('/').all(|seg| {
            !seg.is_empty()
                && seg
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        })
}

fn is_number(s: &str) -> bool {
    !s.is_empty() && s.len() <= 4 && s.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_paths() {
        assert!(is_page_path(""));
        assert!(is_page_path("faq"));
        assert!(is_page_path("rs2/skills/fighting"));
        assert!(is_page_path("hiscores/index-v2"));
        assert!(!is_page_path(".."));
        assert!(!is_page_path("faq/../layout/header"));
        assert!(!is_page_path("faq//billing"));
        assert!(!is_page_path("faq/billing.html"));
        assert!(!is_page_path("%2e%2e"));
    }
}
