use crate::app::{App, AppError};
use crate::routes::pages::not_found;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use minijinja::context;
use minijinja::value::Serde;
use rs_db::{HISCORE_CATEGORIES, validate_name};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Rows per table page; a focused rank sits in the middle one.
const PAGE_SIZE: i64 = 21;

/// Names for [`HISCORE_CATEGORIES`], in the same order.
const CATEGORY_NAMES: [&str; 20] = [
    "Overall",
    "Attack",
    "Defence",
    "Strength",
    "Hitpoints",
    "Ranged",
    "Prayer",
    "Magic",
    "Cooking",
    "Woodcutting",
    "Fletching",
    "Fishing",
    "Firemaking",
    "Crafting",
    "Smithing",
    "Mining",
    "Herblore",
    "Agility",
    "Thieving",
    "Runecrafting",
];

#[derive(Serialize)]
struct Category {
    id: i16,
    name: &'static str,
    icon: String,
}

fn categories() -> impl Iterator<Item = Category> {
    HISCORE_CATEGORIES
        .into_iter()
        .zip(CATEGORY_NAMES)
        .map(|(id, name)| Category {
            id,
            name,
            icon: match id {
                0 => "/img/title/blank.gif".to_string(),
                _ => format!("/img/hiscores/{}.gif", name.to_lowercase()),
            },
        })
}

/// Every hiscores URL the 2004 site used:
///
/// - `/hiscores?category=N`: the top of table N (default Overall)
/// - `/hiscores?user=X`, `/hiscorepersonal?username=X`: X's ranks
/// - `/hiscores?user=X&category=N`: table N around X
/// - `/hiscorerank?rank=R&category=N`: table N around rank R
#[derive(Deserialize)]
pub struct HiscoresQuery {
    #[serde(alias = "username")]
    user: Option<String>,
    category: Option<String>,
    rank: Option<String>,
}

pub async fn index(
    State(app): State<Arc<App>>,
    Query(q): Query<HiscoresQuery>,
) -> Result<Response, AppError> {
    let category = q
        .category
        .as_deref()
        .and_then(|c| c.parse().ok())
        .filter(|c| HISCORE_CATEGORIES.contains(c));
    let user = q.user.as_deref().map(str::trim).filter(|u| !u.is_empty());
    let rank = q.rank.as_deref().and_then(|r| r.trim().parse::<i64>().ok());

    match (user, category) {
        (Some(user), None) => personal(&app, user).await,
        (Some(user), Some(category)) => {
            let player = match validate_name(user) {
                Ok(name) => app.db.player_hiscores(name.hash).await?,
                Err(_) => None,
            };
            let focus = player
                .as_ref()
                .and_then(|p| p.rank(category))
                .map(|r| r.rank);
            let name = player.map(|p| p.name);
            table(&app, category, focus, name).await
        }
        (None, category) => table(&app, category.unwrap_or(0), rank, None).await,
    }
}

/// Table `category`, centred on rank `focus` if given. `user` is the player
/// the page is about, if any: the skill menu keeps them selected.
async fn table(
    app: &App,
    category: i16,
    focus: Option<i64>,
    user: Option<String>,
) -> Result<Response, AppError> {
    let offset = match focus {
        Some(rank) => {
            let count = app.db.hiscores_count(category).await?;
            (rank - 1 - PAGE_SIZE / 2).min(count - PAGE_SIZE).max(0)
        }
        None => 0,
    };
    let entries = app.db.hiscores_page(category, offset, PAGE_SIZE).await?;
    let current = categories().find(|c| c.id == category);

    let html = app.templates.render(
        "views/hiscores/table.html",
        context! {
            categories => Serde(categories().collect::<Vec<_>>()),
            category => Serde(current),
            entries => Serde(entries),
            focus,
            user,
        },
    )?;
    Ok(html.into_response())
}

async fn personal(app: &App, user: &str) -> Result<Response, AppError> {
    let player = match validate_name(user) {
        Ok(name) => app.db.player_hiscores(name.hash).await?,
        Err(_) => None,
    };
    let rows = player.as_ref().map(|p| {
        categories()
            .filter_map(|c| {
                let rank = p.rank(c.id)?;
                Some(context! { category => Serde(c), rank => Serde(rank) })
            })
            .collect::<Vec<_>>()
    });

    let html = app.templates.render(
        "views/hiscores/personal.html",
        context! {
            search => user,
            name => player.map(|p| p.name),
            rows,
        },
    )?;
    Ok(html.into_response())
}

#[derive(Deserialize)]
pub struct LiteQuery {
    player: Option<String>,
}

/// `/index_lite.ws?player=X`: one `rank,level,xp` line per table, in
/// [`HISCORE_CATEGORIES`] order. An unranked skill is `-1,1,0`; a player not
/// on the hiscores at all is a 404.
pub async fn lite(
    State(app): State<Arc<App>>,
    Query(q): Query<LiteQuery>,
) -> Result<Response, AppError> {
    let Some(name) = q.player.as_deref().and_then(|p| validate_name(p).ok()) else {
        return Ok(not_found());
    };
    let Some(player) = app.db.player_hiscores(name.hash).await? else {
        return Ok(not_found());
    };
    let body: String = HISCORE_CATEGORIES
        .iter()
        .map(|&c| match player.rank(c) {
            Some(r) => format!("{},{},{}\n", r.rank, r.level, r.xp),
            None => "-1,1,0\n".to_string(),
        })
        .collect();
    Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body).into_response())
}
