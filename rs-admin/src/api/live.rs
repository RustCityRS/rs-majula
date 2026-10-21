use super::access::{ceiling, check_replay, hides_self, may_see};
use super::lookup::{address_label, sees_addresses, unavailable};
use super::replays::checked_id;
use crate::audit::Event;
use crate::farms;
use crate::http::{ApiError, App, Staff};
use crate::users::now_ms;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures_util::future::join_all;
use rs_admin_api::{
    AccountActivity, AccountsLookup, AnticheatLeader, AnticheatLookup, LiveReplay, OnlinePlayer,
    path,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
pub struct Positioned {
    #[serde(flatten)]
    player: OnlinePlayer,
    world: u16,
    node: u8,
}

#[derive(Serialize)]
pub struct Everyone {
    players: Vec<Positioned>,
    unreachable: Vec<u16>,
}

/// `GET /api/live/players`: everyone online on every world, where they are.
pub async fn players(State(app): State<Arc<App>>, staff: Staff) -> Json<Everyone> {
    let worlds = app.worlds.online();
    let lists = join_all(worlds.iter().map(|w| app.worlds.players(w))).await;
    let mut players = Vec::new();
    let mut unreachable = Vec::new();
    for (world, list) in worlds.iter().zip(lists) {
        match list {
            Ok(list) => players.extend(list.into_iter().map(|mut player| {
                // Nobody watches staff above them live.
                if !may_see(&staff, player.staff_level) {
                    player.replay = None;
                }
                Positioned {
                    player,
                    world: world.id,
                    node: world.node,
                }
            })),
            Err(_) => unreachable.push(world.id),
        }
    }
    Json(Everyone {
        players,
        unreachable,
    })
}

/// The windows the leaderboard and farms look back over, and how many
/// stretches a leader's trend splits it into.
fn window(name: Option<&str>) -> Result<(i64, u32), ApiError> {
    const HOUR: i64 = 3_600_000;
    match name.unwrap_or("24h") {
        "24h" => Ok((24 * HOUR, 24)),
        "7d" => Ok((7 * 24 * HOUR, 28)),
        "30d" => Ok((30 * 24 * HOUR, 30)),
        _ => Err(ApiError::bad_request("Pick 24h, 7d or 30d.")),
    }
}

#[derive(Deserialize)]
pub struct WindowQuery {
    window: Option<String>,
}

#[derive(Serialize)]
pub struct LeaderView {
    #[serde(flatten)]
    leader: AnticheatLeader,
    online_world: Option<u16>,
}

#[derive(Serialize)]
pub struct Leaderboard {
    since_ms: i64,
    leaders: Vec<LeaderView>,
}

/// `GET /api/live/anticheat?window=24h|7d|30d`: the players the checks
/// flagged most over the window, with their scores over it.
pub async fn anticheat(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<WindowQuery>,
) -> Result<Json<Leaderboard>, ApiError> {
    let (span, buckets) = window(query.window.as_deref())?;
    let since_ms = now_ms() as i64 - span;
    let leaders: Vec<AnticheatLeader> = app
        .worlds
        .ask(
            path::ANTICHEAT,
            &AnticheatLookup {
                since_ms,
                buckets,
                max_staff_level: ceiling(&staff),
                limit: 100,
            },
        )
        .await
        .map_err(unavailable)?;
    Ok(Json(Leaderboard {
        since_ms,
        leaders: leaders
            .into_iter()
            .filter(|leader| !hides_self(&staff, &leader.name))
            .map(|leader| LeaderView {
                online_world: leader
                    .online_node
                    .and_then(|n| app.worlds.get(n))
                    .map(|w| w.id),
                leader,
            })
            .collect(),
    }))
}

/// The most accounts the farm search weighs.
const FARM_ACCOUNTS: u32 = 3000;

#[derive(Serialize)]
pub struct Member {
    #[serde(flatten)]
    account: AccountActivity,
    online_world: Option<u16>,
}

#[derive(Serialize)]
pub struct FarmView {
    members: Vec<Member>,
    shared: Vec<farms::Shared>,
    score: u32,
}

#[derive(Serialize)]
pub struct Farms {
    since_ms: i64,
    /// How many accounts were weighed, and whether that's all of them.
    accounts: usize,
    capped: bool,
    farms: Vec<FarmView>,
    /// Whether addresses are real, or labels standing in for them.
    addresses_shown: bool,
}

/// `GET /api/live/farms?window=24h|7d|30d`: groups of accounts active over
/// the window that look like one bot farm. Audited when it shows addresses.
pub async fn farms(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<WindowQuery>,
) -> Result<Json<Farms>, ApiError> {
    let (span, _) = window(query.window.as_deref())?;
    let since_ms = now_ms() as i64 - span;
    let accounts: Vec<AccountActivity> = app
        .worlds
        .ask(
            path::ACCOUNTS,
            &AccountsLookup {
                since_ms,
                limit: FARM_ACCOUNTS,
            },
        )
        .await
        .map_err(unavailable)?;
    let weighed = accounts.len();
    let found = tokio::task::spawn_blocking({
        let accounts = accounts.clone();
        move || farms::group(&accounts, 2)
    })
    .await
    .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Grouping failed."))?;

    let sees = sees_addresses(&staff);
    let mask = |ip: String| if sees { ip } else { address_label(&app, &ip) };
    let mut accounts: Vec<Option<AccountActivity>> = accounts.into_iter().map(Some).collect();
    let farms: Vec<FarmView> = found
        .into_iter()
        .map(|farm| FarmView {
            members: farm
                .members
                .iter()
                .filter_map(|&i| accounts[i].take())
                .map(|mut account| {
                    account.registration_ip = account.registration_ip.take().map(mask);
                    account.login_ips = std::mem::take(&mut account.login_ips)
                        .into_iter()
                        .map(mask)
                        .collect();
                    Member {
                        online_world: account
                            .online_node
                            .and_then(|n| app.worlds.get(n))
                            .map(|w| w.id),
                        account,
                    }
                })
                .collect(),
            shared: farm
                .shared
                .into_iter()
                .map(|mut s| {
                    if let farms::Reason::Registered { ip } | farms::Reason::LoggedIn { ip } =
                        &mut s.reason
                    {
                        *ip = mask(std::mem::take(ip));
                    }
                    s
                })
                .collect(),
            score: farm.score,
        })
        .collect();
    if sees && !farms.is_empty() {
        app.audit.record(
            Event::new(Some(staff.name()), staff.client.ip, "addresses_viewed")
                .target("bot farm groups"),
        );
    }
    Ok(Json(Farms {
        since_ms,
        accounts: weighed,
        capped: weighed == FARM_ACCOUNTS as usize,
        farms,
        addresses_shown: sees,
    }))
}

#[derive(Deserialize)]
pub struct LiveQuery {
    /// The world recording it.
    node: u8,
    player: String,
    token: Option<String>,
}

/// `GET /api/replays/{id}/live?node=&player=&token=`: a session going on
/// now, from the world recording it: the recording so far, then with the
/// token from its header what's been recorded since. Starting is audited as
/// watching it.
pub async fn replay(
    State(app): State<Arc<App>>,
    staff: Staff,
    Path(id): Path<String>,
    Query(query): Query<LiveQuery>,
) -> Result<Response, ApiError> {
    let id = checked_id(&id)?;
    let player = super::players::checked_name(&query.player)?;
    let token = query
        .token
        .filter(|t| t.len() <= 64 && t.bytes().all(|b| b.is_ascii_hexdigit()));
    let start = token.is_none();
    if start {
        check_replay(&app, &staff, &id).await?;
    }
    let bytes = app
        .worlds
        .live_replay(
            query.node,
            &LiveReplay {
                id: id.clone(),
                token,
            },
        )
        .await
        .map_err(|e| ApiError::new(StatusCode::BAD_GATEWAY, format!("{e}.")))?;
    if start {
        app.audit.record(
            Event::new(Some(staff.name()), staff.client.ip, "replay_viewed")
                .target(player)
                .detail(format!("session {id}, live")),
        );
    }
    Ok(([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response())
}
