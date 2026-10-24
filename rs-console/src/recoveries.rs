use crate::audit::Event;
use crate::cases::{Change, Priority};
use crate::http::App;
use rs_admin_api::{
    Outcome, RecoveriesWaiting, Recovery, RecoveryCheck, RecoveryEvidence, RecoveryLink, path,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// How often the website is asked for new recovery requests.
const CHECK_INTERVAL: Duration = Duration::from_secs(30);
/// Who opens their cases, as the case log and audit trail show it.
pub const OPENED_BY: &str = "Account recovery";

/// What a world's records say about `request`.
pub async fn check(app: &App, request: &Recovery) -> Result<RecoveryEvidence, String> {
    let found: Option<RecoveryEvidence> = app
        .worlds
        .ask(
            path::RECOVERY_CHECK,
            &RecoveryCheck {
                request: request.clone(),
            },
        )
        .await?;
    found.ok_or_else(|| format!("{} isn't registered any more.", request.player))
}

/// Takes in what's waiting, checking each new request against the
/// account's records. Returns how many cases it opened.
pub async fn take_in(app: &App) -> Result<usize, String> {
    let waiting: Vec<Recovery> = app
        .worlds
        .web(path::RECOVERIES_WAITING, &RecoveriesWaiting { limit: 50 })
        .await?;
    let mut opened = 0;
    for request in waiting {
        // A case opened before the website heard about it isn't opened twice.
        let case = match app.cases.of_recovery(request.id) {
            Some(id) => id,
            None => {
                // Someone else in the account is the urgent kind.
                let priority = if request.reason == "hijacked" {
                    Priority::High
                } else {
                    Priority::Normal
                };
                let case = app.cases.create(
                    OPENED_BY,
                    format!("Account recovery #{}", request.id),
                    request.player.clone(),
                    priority,
                    vec![Change::RecoveryFiled {
                        recovery: request.clone(),
                    }],
                )?;
                opened += 1;
                info!(
                    "Opened case #{} for {}'s recovery request #{}",
                    case.id, request.player, request.id
                );
                app.audit.record(
                    Event::system(None, "recovery_received")
                        .target(&request.player)
                        .detail(format!(
                            "{} recovery request #{}, case #{}",
                            request.reason, request.id, case.id
                        )),
                );
                // Staff can check again from the case if no world could now.
                match check(app, &request).await {
                    Ok(evidence) => {
                        if let Err(e) = app.cases.change(OPENED_BY, case.id, |_| {
                            Ok(vec![Change::RecoveryChecked { evidence }])
                        }) {
                            warn!("Couldn't note case #{}'s evidence: {}", case.id, e);
                        }
                    }
                    Err(e) => warn!(
                        "Couldn't check recovery request #{} against the records: {}",
                        request.id, e
                    ),
                }
                case.id
            }
        };
        let linked: Outcome = app
            .worlds
            .web(
                path::RECOVERY_LINK,
                &RecoveryLink {
                    id: request.id,
                    case_id: case,
                },
            )
            .await?;
        if !linked.ok {
            warn!(
                "The website didn't link recovery request #{}: {}",
                request.id, linked.message
            );
        }
    }
    Ok(opened)
}

/// Takes in new recovery requests every [`CHECK_INTERVAL`], while the
/// console can reach the website.
pub async fn keep_taking_in(app: Arc<App>) {
    let mut interval = tokio::time::interval(CHECK_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut failing = false;
    loop {
        interval.tick().await;
        match take_in(&app).await {
            Ok(_) => failing = false,
            Err(e) if !failing => {
                warn!("Couldn't take in recovery requests from the website: {}", e);
                failing = true;
            }
            Err(_) => {}
        }
    }
}
