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
    pub short_date: String,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct NewsPost {
    pub id: i32,
    pub category_id: i16,
    pub category_name: String,
    pub title: String,
    pub content: String,
    pub long_date: String,
    pub prev_id: Option<i32>,
    pub next_id: Option<i32>,
}

/// A post as the staff console edits it.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct NewsAdminPost {
    pub id: i32,
    pub category_id: i16,
    pub title: String,
    pub content: String,
    pub markdown: Option<String>,
    pub created_ms: i64,
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

    /// Posts with their HTML, newest first, older than post `before` when
    /// paging: for the staff console.
    pub async fn news_admin_page(
        &self,
        before: Option<i32>,
        limit: i64,
    ) -> Result<Vec<NewsAdminPost>, Error> {
        sqlx::query_as(
            "SELECT id, category_id, title, content, markdown,
                    (extract(epoch FROM created_at) * 1000)::bigint AS created_ms
             FROM news_posts
             WHERE $1::int IS NULL OR id < $1
             ORDER BY id DESC
             LIMIT $2",
        )
        .bind(before)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Adds a post written in `markdown`, shown as `content`. `None` if the
    /// category doesn't exist.
    pub async fn create_news_post(
        &self,
        category_id: i16,
        title: &str,
        content: &str,
        markdown: &str,
    ) -> Result<Option<NewsAdminPost>, Error> {
        sqlx::query_as(
            "INSERT INTO news_posts (category_id, title, content, markdown)
             SELECT $1, $2, $3, $4 WHERE EXISTS (SELECT 1 FROM news_categories WHERE id = $1)
             RETURNING id, category_id, title, content, markdown,
                       (extract(epoch FROM created_at) * 1000)::bigint AS created_ms",
        )
        .bind(category_id)
        .bind(title)
        .bind(content)
        .bind(markdown)
        .fetch_optional(&self.pool)
        .await
    }

    /// Edits a post, keeping its date. `None` if there's no such post or
    /// category.
    pub async fn update_news_post(
        &self,
        id: i32,
        category_id: i16,
        title: &str,
        content: &str,
        markdown: &str,
    ) -> Result<Option<NewsAdminPost>, Error> {
        sqlx::query_as(
            "UPDATE news_posts SET category_id = $2, title = $3, content = $4, markdown = $5
             WHERE id = $1 AND EXISTS (SELECT 1 FROM news_categories WHERE id = $2)
             RETURNING id, category_id, title, content, markdown,
                       (extract(epoch FROM created_at) * 1000)::bigint AS created_ms",
        )
        .bind(id)
        .bind(category_id)
        .bind(title)
        .bind(content)
        .bind(markdown)
        .fetch_optional(&self.pool)
        .await
    }

    /// Deletes a post. Returns whether there was one.
    pub async fn delete_news_post(&self, id: i32) -> Result<bool, Error> {
        let result = sqlx::query("DELETE FROM news_posts WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }
}
