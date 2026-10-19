mod create;
mod hiscores;
mod news;
mod pages;
mod play;

use crate::app::App;
use crate::metrics;
use axum::response::Redirect;
use axum::routing::get;
use axum::{Router, middleware};
use std::path::Path;
use std::sync::Arc;
use tower_http::services::ServeDir;

/// Explicit routes for the dynamic pages; anything else is a file under
/// `public/` or, failing that, a template under `templates/pages/`.
pub fn router(app: Arc<App>, public: &Path) -> Router {
    let pages = get(pages::page).with_state(app.clone());
    Router::new()
        .route("/title", get(play::title))
        .route("/play", get(|| async { Redirect::to("/detail") }))
        .route("/serverlist", get(play::serverlist))
        .route("/client", get(play::client))
        .route("/api/v1/worldlist", get(play::worldlist))
        .route("/create", get(create::form).post(create::submit))
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
        .with_state(app)
}
