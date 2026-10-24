use super::players::checked_name;
use crate::http::{ApiError, App, Staff};
use crate::prefs::{PrefsOp, Recent, SavedFilter, StaffPrefs};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub struct PrefsView {
    pinned: Vec<String>,
    recent: Vec<Recent>,
    filters: Vec<SavedFilter>,
    utc: bool,
}

impl From<StaffPrefs> for PrefsView {
    fn from(p: StaffPrefs) -> Self {
        Self {
            pinned: p.pinned,
            recent: p.recent,
            filters: p.filters,
            utc: p.utc,
        }
    }
}

/// `GET /api/prefs`: the caller's pinned and recent players, saved filters
/// and time zone.
pub async fn get(State(app): State<Arc<App>>, staff: Staff) -> Json<PrefsView> {
    Json(app.prefs.of(staff.name()).into())
}

/// `POST /api/prefs`: one change to them, answered with them all.
pub async fn change(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(op): Json<PrefsOp>,
) -> Result<Json<PrefsView>, ApiError> {
    let op = match op {
        PrefsOp::Pin { name } => PrefsOp::Pin {
            name: checked_name(&name)?,
        },
        PrefsOp::Unpin { name } => PrefsOp::Unpin {
            name: checked_name(&name)?,
        },
        PrefsOp::Viewed { name } => PrefsOp::Viewed {
            name: checked_name(&name)?,
        },
        op => op,
    };
    app.prefs
        .change(staff.name(), op)
        .map(|p| Json(p.into()))
        .map_err(|e| ApiError::new(StatusCode::CONFLICT, e))
}
