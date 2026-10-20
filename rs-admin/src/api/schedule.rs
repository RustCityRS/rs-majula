use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use crate::schedule::{Job, MAX_AHEAD_MS, MAX_EVERY_MINS, MIN_EVERY_MINS, Task, describe};
use crate::users::now_ms;
use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// What a world accepts.
const MAX_REBOOT_SECS: u32 = 36_000;
const MAX_STAGGER_SECS: u32 = 3_600;
const MAX_BROADCAST_CHARS: usize = 400;
const MAX_REASON_CHARS: usize = 80;
/// A job set a moment ago for "now" still counts as on time.
const GRACE_MS: u64 = 60_000;

#[derive(Serialize)]
pub struct SchedulePage {
    jobs: Vec<Job>,
    can_change: bool,
}

fn page(app: &App, staff: &Staff) -> SchedulePage {
    SchedulePage {
        jobs: app.schedule.jobs(),
        can_change: staff.require_admin().is_ok(),
    }
}

/// `GET /api/schedule`.
pub async fn list(State(app): State<Arc<App>>, staff: Staff) -> Json<SchedulePage> {
    Json(page(&app, &staff))
}

#[derive(Deserialize)]
pub struct NewJob {
    task: Task,
    #[serde(default)]
    nodes: Vec<u8>,
    at_ms: u64,
    #[serde(default)]
    every_mins: Option<u32>,
}

fn checked_task(task: Task) -> Result<Task, ApiError> {
    Ok(match task {
        Task::Reboot {
            countdown_secs,
            stagger_secs,
            reason,
        } => {
            if countdown_secs > MAX_REBOOT_SECS {
                return Err(ApiError::bad_request(
                    "A countdown can be at most 10 hours.",
                ));
            }
            if stagger_secs > MAX_STAGGER_SECS {
                return Err(ApiError::bad_request(format!(
                    "The stagger can be at most {MAX_STAGGER_SECS} seconds."
                )));
            }
            let reason = reason
                .map(|r| r.trim().to_owned())
                .filter(|r| !r.is_empty());
            if reason
                .as_ref()
                .is_some_and(|r| r.chars().count() > MAX_REASON_CHARS)
            {
                return Err(ApiError::bad_request(format!(
                    "A reason can be at most {MAX_REASON_CHARS} characters."
                )));
            }
            Task::Reboot {
                countdown_secs,
                stagger_secs,
                reason,
            }
        }
        Task::Broadcast { message } => {
            let message = message.trim().to_owned();
            if message.is_empty() || message.chars().count() > MAX_BROADCAST_CHARS {
                return Err(ApiError::bad_request(format!(
                    "A broadcast must be 1 to {MAX_BROADCAST_CHARS} characters."
                )));
            }
            Task::Broadcast { message }
        }
    })
}

/// `POST /api/schedule`: schedules a reboot or broadcast.
pub async fn add(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<NewJob>,
) -> Result<Json<SchedulePage>, ApiError> {
    staff.require_admin()?;
    let task = checked_task(req.task)?;
    let mut nodes: Vec<u8> = Vec::new();
    for node in req.nodes {
        if app.worlds.get(node).is_none() {
            return Err(ApiError::bad_request(format!(
                "There is no world with node {node}."
            )));
        }
        if !nodes.contains(&node) {
            nodes.push(node);
        }
    }
    if let Task::Reboot {
        countdown_secs,
        stagger_secs,
        ..
    } = &task
    {
        let worlds = if nodes.is_empty() {
            app.worlds.views().len()
        } else {
            nodes.len()
        };
        let last =
            u64::from(*countdown_secs) + u64::from(*stagger_secs) * worlds.saturating_sub(1) as u64;
        if last > u64::from(MAX_REBOOT_SECS) {
            return Err(ApiError::bad_request(
                "The last world would reboot more than 10 hours after the countdown starts.",
            ));
        }
    }
    let now = now_ms();
    if req.at_ms + GRACE_MS < now {
        return Err(ApiError::bad_request("That time has passed."));
    }
    if req.at_ms > now + MAX_AHEAD_MS {
        return Err(ApiError::bad_request(
            "Jobs can be set at most a year ahead.",
        ));
    }
    if let Some(every) = req.every_mins
        && !(MIN_EVERY_MINS..=MAX_EVERY_MINS).contains(&every)
    {
        return Err(ApiError::bad_request(format!(
            "Repeats are {MIN_EVERY_MINS} minutes to 31 days apart."
        )));
    }
    let job = app
        .schedule
        .add(Job {
            id: 0,
            task,
            nodes,
            next_ms: req.at_ms.max(now),
            every_mins: req.every_mins,
            by: staff.name().to_owned(),
            by_level: staff.user.role.staff_level(),
            created_ms: now,
            last: None,
        })
        .map_err(ApiError::bad_request)?;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "schedule_added")
            .target(format!("job #{}", job.id))
            .detail(describe(&job)),
    );
    Ok(Json(page(&app, &staff)))
}

/// `POST /api/schedule/{id}/delete`.
pub async fn remove(
    State(app): State<Arc<App>>,
    staff: Staff,
    Path(id): Path<u64>,
) -> Result<Json<SchedulePage>, ApiError> {
    staff.require_admin()?;
    let job = app.schedule.remove(id).map_err(ApiError::bad_request)?;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "schedule_removed")
            .target(format!("job #{}", job.id))
            .detail(describe(&job)),
    );
    Ok(Json(page(&app, &staff)))
}
