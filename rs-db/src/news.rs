use crate::{Db, Error};
use serde::Serialize;

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct NewsCategory {
    pub id: i16,
    pub name: String,
    pub style: String,
}

/// A row of a news listing.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct NewsSummary {
    pub id: i32,
    pub title: String,
    pub category_name: String,
    pub category_style: String,
    /// e.g. `18-Oct-2026`
    pub short_date: String,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct NewsPost {
    pub id: i32,
    pub category_id: i16,
    pub category_name: String,
    pub title: String,
    pub content: String,
    /// e.g. `18th October 2026`
    pub long_date: String,
    /// Previous and next post in the same category.
    pub prev_id: Option<i32>,
    pub next_id: Option<i32>,
}

impl Db {
    pub async fn news_categories(&self) -> Result<Vec<NewsCategory>, Error> {
        sqlx::query_as("SELECT id, name, style FROM news_categories ORDER BY id")
            .fetch_all(&self.pool)
            .await
    }

    /// Newest-first posts, optionally in one category.
    pub async fn news_page(
        &self,
        category_id: Option<i16>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<NewsSummary>, Error> {
        sqlx::query_as(
            "SELECT p.id, p.title, c.name AS category_name, c.style AS category_style,
                    to_char(p.created_at, 'FMDD-Mon-YYYY') AS short_date
             FROM news_posts p JOIN news_categories c ON c.id = p.category_id
             WHERE $1::smallint IS NULL OR p.category_id = $1
             ORDER BY p.id DESC
             OFFSET $2 LIMIT $3",
        )
        .bind(category_id)
        .bind(offset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn news_post(&self, id: i32) -> Result<Option<NewsPost>, Error> {
        sqlx::query_as(
            "SELECT p.id, p.category_id, c.name AS category_name, p.title, p.content,
                    to_char(p.created_at, 'FMDDth FMMonth YYYY') AS long_date,
                    (SELECT max(id) FROM news_posts
                     WHERE category_id = p.category_id AND id < p.id) AS prev_id,
                    (SELECT min(id) FROM news_posts
                     WHERE category_id = p.category_id AND id > p.id) AS next_id
             FROM news_posts p JOIN news_categories c ON c.id = p.category_id
             WHERE p.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }
}
