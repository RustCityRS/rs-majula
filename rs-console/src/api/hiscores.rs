use super::lookup::unavailable;
use super::players::{check_case, checked_name, note_on_case, reason_detail};
use super::support::reason;
use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use axum::Json;
use axum::extract::{Query, State};
use rs_admin_api::{
    HiscoreStanding, HiscoresChange, HiscoresLookup, KeptOff, KeptOffLookup, Outcome, path,
};
use serde::Deserialize;
use std::sync::Arc;

/// The hiscore tables, by the website's number: 0 is Overall, any other a
/// stat id + 1.
pub const TABLES: [(i16, &str); 20] = [
    (0, "Overall"),
    (1, "Attack"),
    (2, "Defence"),
    (3, "Strength"),
    (4, "Hitpoints"),
    (5, "Ranged"),
    (6, "Prayer"),
    (7, "Magic"),
    (8, "Cooking"),
    (9, "Woodcutting"),
    (10, "Fletching"),
    (11, "Fishing"),
    (12, "Firemaking"),
    (13, "Crafting"),
    (14, "Smithing"),
    (15, "Mining"),
    (16, "Herblore"),
    (17, "Agility"),
    (18, "Thieving"),
    (21, "Runecrafting"),
];

fn table_names(tables: &[i16]) -> String {
    tables
        .iter()
        .map(|t| {
            TABLES
                .iter()
                .find(|(id, _)| id == t)
                .map_or_else(|| format!("table {t}"), |(_, name)| (*name).to_owned())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Deserialize)]
pub struct NameQuery {
    name: String,
}

/// `GET /api/hiscores/player?name=`: a player's hiscore entries and what
/// keeps them off; `null` when no such player is registered.
pub async fn player(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Query(query): Query<NameQuery>,
) -> Result<Json<Option<HiscoreStanding>>, ApiError> {
    let name = checked_name(&query.name)?;
    app.worlds
        .ask(path::HISCORES, &HiscoresLookup { name })
        .await
        .map(Json)
        .map_err(unavailable)
}

#[derive(Deserialize)]
pub struct ChangeRequest {
    name: String,
    #[serde(default)]
    hidden: Option<bool>,
    #[serde(default)]
    remove: Vec<i16>,
    #[serde(default)]
    restore: Vec<i16>,
    reason: String,
    #[serde(default)]
    case: Option<u64>,
}

/// `POST /api/hiscores/change`: keeps a player off every hiscore table, or
/// removes their entries from some (a cheater's), or puts them back.
pub async fn change(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<ChangeRequest>,
) -> Result<Json<Outcome>, ApiError> {
    staff.require_admin()?;
    let name = checked_name(&req.name)?;
    let why = reason(&req.reason, true)?;
    check_case(&app, req.case)?;
    if let Some(t) = req
        .remove
        .iter()
        .chain(&req.restore)
        .find(|t| !TABLES.iter().any(|(id, _)| id == *t))
    {
        return Err(ApiError::bad_request(format!(
            "There's no hiscore table {t}."
        )));
    }
    let outcome: Outcome = app
        .worlds
        .ask(
            path::HISCORES_CHANGE,
            &HiscoresChange {
                actor: staff.actor(),
                name: name.clone(),
                hidden: req.hidden,
                remove: req.remove.clone(),
                restore: req.restore.clone(),
                reason: why.clone(),
            },
        )
        .await
        .map_err(unavailable)?;
    let what = match req.hidden {
        Some(true) => "Kept off every table.".to_owned(),
        Some(false) => "Let back on the tables.".to_owned(),
        None if !req.remove.is_empty() => format!("Removed from {}.", table_names(&req.remove)),
        None => format!("Put back on {}.", table_names(&req.restore)),
    };
    let detail = reason_detail(&why, &format!("{what} {}", outcome.message));
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "hiscores_changed")
            .target(name.clone())
            .detail(match req.case {
                Some(id) => format!("Case #{id}. {detail}"),
                None => detail,
            })
            .ok(outcome.ok),
    );
    note_on_case(
        &app,
        &staff,
        req.case,
        "hiscores",
        &name,
        None,
        outcome.ok,
        &outcome.message,
        &why,
    );
    Ok(Json(outcome))
}

/// `GET /api/hiscores/kept-off`: every account staff keep off the
/// hiscores.
pub async fn kept_off(
    State(app): State<Arc<App>>,
    _staff: Staff,
) -> Result<Json<Vec<KeptOff>>, ApiError> {
    app.worlds
        .ask(path::HISCORES_KEPT_OFF, &KeptOffLookup { limit: 500 })
        .await
        .map(Json)
        .map_err(unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_named() {
        assert_eq!(table_names(&[1, 21]), "Attack, Runecrafting");
        assert_eq!(table_names(&[19]), "table 19");
    }
}
