use crate::audit::Event;
use crate::cases::{Appeal, Change, Priority};
use crate::http::App;
use rs_admin_api::{AppealLink, AppealsWaiting, Outcome, path};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// How often the website is asked for new appeals.
const CHECK_INTERVAL: Duration = Duration::from_secs(30);
/// Who opens their cases, as the case log and audit trail show it.
pub const OPENED_BY: &str = "Appeals";

/// Takes in what's waiting. Returns how many cases it opened.
pub async fn take_in(app: &App) -> Result<usize, String> {
    let waiting: Vec<rs_admin_api::Appeal> = app
        .worlds
        .web(path::APPEALS_WAITING, &AppealsWaiting { limit: 50 })
        .await?;
    let mut opened = 0;
    for appeal in waiting {
        // A case opened before the website heard about it isn't opened twice.
        let case = match app.cases.of_appeal(appeal.id) {
            Some(id) => id,
            None => {
                let what = if appeal.sanction == "mute" {
                    "Mute"
                } else {
                    "Ban"
                };
                let case = app.cases.create(
                    OPENED_BY,
                    format!("{what} appeal #{}", appeal.id),
                    appeal.player.clone(),
                    Priority::Normal,
                    vec![Change::AppealFiled {
                        appeal: Appeal {
                            id: appeal.id,
                            sanction: appeal.sanction.clone(),
                            until_ms: appeal.sanction_until_ms,
                            issued_by: appeal.issued_by.clone(),
                            reason: appeal.reason.clone(),
                            statement: appeal.statement.clone(),
                            filed_ms: appeal.filed_ms,
                        },
                    }],
                )?;
                opened += 1;
                info!(
                    "Opened case #{} for {}'s appeal #{}",
                    case.id, appeal.player, appeal.id
                );
                app.audit.record(
                    Event::system(None, "appeal_received")
                        .target(&appeal.player)
                        .detail(format!(
                            "{} appeal #{}, case #{}",
                            appeal.sanction, appeal.id, case.id
                        )),
                );
                case.id
            }
        };
        let linked: Outcome = app
            .worlds
            .web(
                path::APPEAL_LINK,
                &AppealLink {
                    id: appeal.id,
                    case_id: case,
                },
            )
            .await?;
        if !linked.ok {
            warn!(
                "The website didn't link appeal #{}: {}",
                appeal.id, linked.message
            );
        }
    }
    Ok(opened)
}

/// Takes in new appeals every [`CHECK_INTERVAL`], while the console can
/// reach the website.
pub async fn keep_taking_in(app: Arc<App>) {
    let mut interval = tokio::time::interval(CHECK_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut failing = false;
    loop {
        interval.tick().await;
        match take_in(&app).await {
            Ok(_) => failing = false,
            Err(e) if !failing => {
                warn!("Couldn't take in appeals from the website: {}", e);
                failing = true;
            }
            Err(_) => {}
        }
    }
}
