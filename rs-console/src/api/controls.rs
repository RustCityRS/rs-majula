use crate::audit::Event;
use crate::controls::{
    self, ActiveSwitch, Desired, Lockdown, MAX_LOCKDOWN_HOURS, PRESETS, PatternCheck, Preset,
    checked_label, checked_patterns,
};
use crate::http::{ApiError, App, Staff};
use crate::users::now_ms;
use axum::Json;
use axum::extract::State;
use rs_admin_api::WorldControls;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// How one world stands against what's wanted.
#[derive(Serialize)]
pub struct WorldSync {
    node: u8,
    world: u16,
    online: bool,
    supported: bool,
    in_sync: bool,
    applied: Option<WorldControls>,
    matched: Option<Vec<u32>>,
    error: Option<String>,
}

#[derive(Serialize)]
pub struct ControlsPage {
    presets: &'static [Preset],
    desired: Desired,
    wanted: WorldControls,
    worlds: Vec<WorldSync>,
    can_change: bool,
    /// What's off about the switch just turned on.
    warnings: Vec<String>,
}

fn page(app: &App, staff: &Staff) -> ControlsPage {
    let desired = app.controls.desired();
    let wanted = desired.world();
    let worlds = app
        .worlds
        .views()
        .into_iter()
        .map(|v| {
            let applied = v.status.as_ref().and_then(|s| s.controls.clone());
            let push = app
                .controls
                .push(v.config.node)
                .filter(|p| p.sent == wanted);
            let in_sync = applied.as_ref() == Some(&wanted);
            // The world's own counts are current (a script reload redoes
            // them); a world on an older release only has the push's.
            let current = v
                .status
                .as_ref()
                .and_then(|s| s.control_matches.clone())
                .filter(|m| in_sync && m.len() == wanted.switches.len());
            WorldSync {
                node: v.config.node,
                world: v.config.id,
                online: v.online,
                supported: applied.is_some(),
                in_sync,
                applied,
                matched: current.or_else(|| push.as_ref().and_then(|p| p.result.clone().ok())),
                error: push.and_then(|p| p.result.err()),
            }
        })
        .collect();
    ControlsPage {
        presets: PRESETS,
        desired,
        wanted,
        worlds,
        can_change: staff.require_admin().is_ok(),
        warnings: Vec::new(),
    }
}

/// The most patterns checked at once: every preset's, with room to spare.
const MAX_CHECK_PATTERNS: usize = 64;
const MAX_PATTERN_CHARS: usize = 100;

#[derive(Deserialize)]
pub struct CheckRequest {
    patterns: Vec<String>,
}

/// `POST /api/controls/check`: how many scripts each pattern would match on
/// every online world, switching nothing off.
pub async fn check(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Json(req): Json<CheckRequest>,
) -> Result<Json<PatternCheck>, ApiError> {
    let patterns: Vec<String> = req
        .patterns
        .iter()
        .map(|p| p.trim().to_owned())
        .filter(|p| !p.is_empty())
        .collect();
    if patterns.len() > MAX_CHECK_PATTERNS
        || patterns
            .iter()
            .any(|p| p.chars().count() > MAX_PATTERN_CHARS)
    {
        return Err(ApiError::bad_request(format!(
            "Check at most {MAX_CHECK_PATTERNS} patterns of at most {MAX_PATTERN_CHARS} characters."
        )));
    }
    Ok(Json(controls::check(&app, &patterns).await))
}

/// `GET /api/controls`.
pub async fn get(State(app): State<Arc<App>>, staff: Staff) -> Json<ControlsPage> {
    Json(page(&app, &staff))
}

#[derive(Deserialize)]
pub struct SwitchRequest {
    /// A preset's key, or a custom switch's; none for a new custom switch.
    #[serde(default)]
    key: Option<String>,
    on: bool,
    /// For a new custom switch.
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    patterns: Vec<String>,
}

/// `POST /api/controls/switch`: turns a kill switch on or off, and sets
/// every world to match at once.
pub async fn switch(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<SwitchRequest>,
) -> Result<Json<ControlsPage>, ApiError> {
    staff.require_admin()?;
    let preset = req
        .key
        .as_deref()
        .and_then(|k| PRESETS.iter().find(|p| p.key == k));
    let new = if !req.on {
        None
    } else if let Some(preset) = preset {
        Some((
            preset.key.to_owned(),
            preset.label.to_owned(),
            preset.patterns.iter().map(|p| p.to_string()).collect(),
        ))
    } else if req.key.is_none() {
        let label = checked_label(req.label.as_deref().unwrap_or_default())
            .map_err(ApiError::bad_request)?;
        let patterns = checked_patterns(&req.patterns).map_err(ApiError::bad_request)?;
        Some((String::new(), label, patterns))
    } else {
        return Err(ApiError::bad_request("There's no such switch to turn on."));
    };
    // Checked against the worlds' scripts first: a pattern that matches
    // nothing switches nothing off, and would look like it worked.
    let warnings = match &new {
        Some((_, _, patterns)) => {
            let check = controls::check(&app, patterns).await;
            if let Some(refusal) = check.refusal() {
                return Err(ApiError::bad_request(refusal));
            }
            check.warnings()
        }
        None => Vec::new(),
    };

    let by = staff.name().to_owned();
    let changed = app
        .controls
        .update(|desired| match new {
            Some((key, label, patterns)) => {
                if !key.is_empty() && desired.switches.iter().any(|s| s.key == key) {
                    return Ok(None);
                }
                let key = if key.is_empty() {
                    (1..)
                        .map(|n| format!("custom-{n}"))
                        .find(|k| !desired.switches.iter().any(|s| &s.key == k))
                        .expect("a free key")
                } else {
                    key
                };
                let switch = ActiveSwitch {
                    key,
                    label,
                    patterns,
                    by,
                    at_ms: now_ms(),
                };
                desired.switches.push(switch.clone());
                Ok(Some(switch))
            }
            None => {
                let key = req.key.as_deref().unwrap_or_default();
                let i = desired.switches.iter().position(|s| s.key == key);
                Ok(i.map(|i| desired.switches.remove(i)))
            }
        })
        .map_err(ApiError::bad_request)?;

    if let Some(switch) = changed {
        app.audit.record(
            Event::new(
                Some(staff.name()),
                staff.client.ip,
                if req.on {
                    "kill_switch_on"
                } else {
                    "kill_switch_off"
                },
            )
            .target(&switch.label)
            .detail(switch.patterns.join(" ")),
        );
        controls::sync(&app, staff.name(), None).await;
    }
    Ok(Json(ControlsPage {
        warnings,
        ..page(&app, &staff)
    }))
}

#[derive(Deserialize)]
pub struct LockdownRequest {
    /// Turn away accounts younger than this; 0 lifts the lockdown.
    hours: u32,
}

/// `POST /api/controls/lockdown`.
pub async fn lockdown(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<LockdownRequest>,
) -> Result<Json<ControlsPage>, ApiError> {
    staff.require_admin()?;
    if req.hours > MAX_LOCKDOWN_HOURS {
        return Err(ApiError::bad_request(format!(
            "A lockdown turns away accounts at most {MAX_LOCKDOWN_HOURS} hours old."
        )));
    }
    let by = staff.name().to_owned();
    let before = app
        .controls
        .update(|desired| {
            let before = desired.lockdown.as_ref().map_or(0, |l| l.hours);
            desired.lockdown = (req.hours > 0).then(|| Lockdown {
                hours: req.hours,
                by,
                at_ms: now_ms(),
            });
            Ok(before)
        })
        .map_err(ApiError::bad_request)?;
    if before != req.hours {
        app.audit.record(
            Event::new(
                Some(staff.name()),
                staff.client.ip,
                if req.hours > 0 {
                    "lockdown_on"
                } else {
                    "lockdown_off"
                },
            )
            .target("every world")
            .detail(if req.hours > 0 {
                format!("accounts younger than {} hours turned away", req.hours)
            } else {
                String::new()
            }),
        );
        controls::sync(&app, staff.name(), None).await;
    }
    Ok(Json(page(&app, &staff)))
}
