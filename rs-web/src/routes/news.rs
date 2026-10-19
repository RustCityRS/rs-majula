use crate::app::{App, AppError};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use minijinja::context;
use minijinja::value::Serde;
use serde::Deserialize;
use std::sync::Arc;

const PAGE_SIZE: i64 = 17;

#[derive(Deserialize)]
pub struct NewsQuery {
    cat: Option<i16>,
    page: Option<i64>,
}

/// `/news?cat=N&page=N`: newest first, optionally one category (`cat=0` is
/// all of them).
pub async fn index(
    State(app): State<Arc<App>>,
    Query(q): Query<NewsQuery>,
) -> Result<Response, AppError> {
    let page = q.page.unwrap_or(1).max(1);
    let categories = app.db.news_categories().await?;
    let category = q.cat.and_then(|id| categories.iter().find(|c| c.id == id));
    let category_id = category.map(|c| c.id);

    // One extra row tells us whether there is a next page.
    let mut news = app
        .db
        .news_page(category_id, (page - 1) * PAGE_SIZE, PAGE_SIZE + 1)
        .await?;
    let more = news.len() as i64 > PAGE_SIZE;
    news.truncate(PAGE_SIZE as usize);

    let href = |page: i64| {
        let mut params = Vec::new();
        if let Some(id) = category_id {
            params.push(format!("cat={id}"));
        }
        if page > 1 {
            params.push(format!("page={page}"));
        }
        if params.is_empty() {
            "/news".to_string()
        } else {
            format!("/news?{}", params.join("&"))
        }
    };
    let heading = match category {
        Some(c) => format!("Latest {} News", c.name),
        None => "Latest News".to_string(),
    };

    let html = app.templates.render(
        "views/news/index.html",
        context! {
            heading,
            categories => Serde(categories),
            news => Serde(news),
            prev_href => (page > 1).then(|| href(page - 1)),
            next_href => more.then(|| href(page + 1)),
        },
    )?;
    Ok(html.into_response())
}

pub async fn post(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let Ok(id) = id.parse::<i32>() else {
        return Ok(Redirect::to("/news").into_response());
    };
    let Some(post) = app.db.news_post(id).await? else {
        return Ok(Redirect::to("/news").into_response());
    };
    let categories = app.db.news_categories().await?;

    let html = app.templates.render(
        "views/news/post.html",
        context! {
            heading => format!("Latest {} News", post.category_name),
            prev_href => post.prev_id.map(|id| format!("/news/{id}")),
            next_href => post.next_id.map(|id| format!("/news/{id}")),
            categories => Serde(categories),
            post => Serde(post),
        },
    )?;
    Ok(html.into_response())
}
