mod account;
mod appeal;
mod create;
mod hiscores;
pub(crate) mod messages;
mod news;
mod pages;
mod play;
mod recover;
mod recovery;
#[cfg(since_289)]
mod screenshots;
mod sign_in;

use crate::app::App;
use crate::metrics;
use axum::http::{HeaderName, HeaderValue, Response, header};
use axum::response::Redirect;
use axum::routing::{get, post};
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
    // The site's own pictures change only when one is fixed, so a month
    // is long enough to cache them and short enough to see a fix. Only
    // what's found: a missing one shouldn't stay missing once it's added.
    router = router.nest_service(
        "/img",
        SetResponseHeader::overriding(
            ServeDir::new(public.join("img")),
            header::CACHE_CONTROL,
            |res: &Response<_>| {
                res.status()
                    .is_success()
                    .then(|| HeaderValue::from_static("public, max-age=2592000"))
            },
        ),
    );
    // The 2005 title screen's game screenshots; the 2004 one didn't link any.
    #[cfg(since_289)]
    {
        router = router
            .route("/screenshots", get(screenshots::index))
            .route("/screenshots/{name}", get(screenshots::shot));
    }
    router
        .route("/title", get(play::title))
        .route("/play", get(|| async { Redirect::to("/detail") }))
        .route("/serverlist", get(play::serverlist))
        .route("/client", get(play::client))
        .route("/api/v1/worldlist", get(play::worldlist))
        .route("/create", get(create::form).post(create::submit))
        .route(
            "/account",
            get(account::form).post(account::show).layer(no_store()),
        )
        .route(
            "/account/password",
            post(account::password).layer(no_store()),
        )
        .route("/account/data", post(account::data).layer(no_store()))
        .route("/account/delete", post(account::delete).layer(no_store()))
        .route(
            "/messages",
            get(messages::form).post(messages::list).layer(no_store()),
        )
        .route("/messages/view", post(messages::view).layer(no_store()))
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
        .route("/support", get(pages::support))
        .route("/manifest.webmanifest", get(pages::manifest))
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

/// Keeps a page out of the browser's cache and history snapshots: a
/// signed-in page (an account, its messages) shouldn't come back on the
/// back button after it's left.
fn no_store() -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))
}

/// Sets `name` on every response that hasn't set it itself.
fn header(name: HeaderName, value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::if_not_present(name, HeaderValue::from_static(value))
}
