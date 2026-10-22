mod appeal;
mod create;
mod hiscores;
mod news;
mod pages;
mod play;
mod recover;
mod recovery;

use crate::app::App;
use crate::metrics;
use axum::http::{HeaderName, HeaderValue, header};
use axum::response::Redirect;
use axum::routing::get;
use axum::{Router, middleware};
use std::path::Path;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::set_header::{SetResponseHeader, SetResponseHeaderLayer};

/// Explicit routes for the dynamic pages; anything else is a file under
/// `public/` or, failing that, a template under `templates/pages/`.
pub fn router(app: Arc<App>, public: &Path) -> Router {
    let pages = get(pages::page).with_state(app.clone());
    let mut router = Router::new();
    if let Some(dir) = &app.media_dir {
        // Named by their content, so never stale. ServeDir answers range
        // requests, which video seeking needs.
        router = router.nest_service(
            "/news/media",
            SetResponseHeader::overriding(
                ServeDir::new(dir),
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            ),
        );
    }
    router
        .route("/title", get(play::title))
        .route("/play", get(|| async { Redirect::to("/detail") }))
        .route("/serverlist", get(play::serverlist))
        .route("/client", get(play::client))
        .route("/api/v1/worldlist", get(play::worldlist))
        .route("/create", get(create::form).post(create::submit))
        .route("/appeal", get(appeal::form).post(appeal::submit))
        .route("/recover", get(recover::form).post(recover::submit))
        .route(
            "/recover/request",
            get(recovery::form).post(recovery::submit),
        )
        .route(
            "/recover/request/status",
            get(recovery::status_form).post(recovery::status),
        )
        .route(
            "/appeal/status",
            get(appeal::status_form).post(appeal::status),
        )
        .route("/news", get(news::index))
        .route("/news/{id}", get(news::post))
        .route("/hiscores", get(hiscores::index))
        .route("/hiscorerank", get(hiscores::index))
        .route("/hiscorepersonal", get(hiscores::index))
        .route("/index_lite.ws", get(hiscores::lite))
        .route("/results", get(pages::poll_results))
        .route("/worldmap", get(pages::worldmap))
        .fallback_service(ServeDir::new(public).fallback(pages))
        .layer(middleware::from_fn_with_state(app.clone(), metrics::track))
        .layer(header(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .layer(header(
            header::REFERRER_POLICY,
            "strict-origin-when-cross-origin",
        ))
        .layer(header(
            header::CONTENT_SECURITY_POLICY,
            "frame-ancestors 'self'",
        ))
        .layer(header(header::X_FRAME_OPTIONS, "SAMEORIGIN"))
        .with_state(app)
}

/// Sets `name` on every response that hasn't set it itself.
fn header(name: HeaderName, value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::if_not_present(name, HeaderValue::from_static(value))
}
