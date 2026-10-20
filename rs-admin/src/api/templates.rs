use super::players::checked_name;
use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use crate::templates::{Standing, Template};
use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
pub struct TemplatesPage {
    templates: Vec<Template>,
    can_edit: bool,
}

/// `GET /api/templates`.
pub async fn list(State(app): State<Arc<App>>, staff: Staff) -> Json<TemplatesPage> {
    Json(TemplatesPage {
        templates: app.templates.all(),
        can_edit: staff.require_admin().is_ok(),
    })
}

#[derive(Deserialize)]
pub struct SaveRequest {
    templates: Vec<Template>,
}

/// `POST /api/templates`: replaces every template.
pub async fn save(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<SaveRequest>,
) -> Result<Json<TemplatesPage>, ApiError> {
    staff.require_admin()?;
    let templates = app
        .templates
        .save(req.templates)
        .map_err(ApiError::bad_request)?;
    let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "templates_saved").detail(names.join(", ")),
    );
    Ok(Json(TemplatesPage {
        templates,
        can_edit: true,
    }))
}

#[derive(Deserialize)]
pub struct NameQuery {
    name: String,
}

/// `GET /api/templates/standing?name=`: where a player stands on each
/// template's ladder.
pub async fn standing(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Query(query): Query<NameQuery>,
) -> Result<Json<Vec<Standing>>, ApiError> {
    let name = checked_name(&query.name)?;
    Ok(Json(app.templates.standings(&name)))
}
