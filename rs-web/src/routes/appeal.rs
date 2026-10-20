use crate::app::{App, AppError};
use axum::Form;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::Html;
use minijinja::context;
use minijinja::value::Serde;
use rs_db::{AppealFiled, AppealLogin, NewAppeal, is_typeable_password, validate_name};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::info;

/// Wrong passwords an IP may give, here or at the game's login (the same
/// count), before it's refused for [`FAILURE_WINDOW_SECS`].
const MAX_FAILURES: i64 = 5;
const FAILURE_WINDOW_SECS: i32 = 60;

const MIN_STATEMENT_CHARS: usize = 20;
const MAX_STATEMENT_CHARS: usize = 2000;

#[derive(Deserialize)]
pub struct SignIn {
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
}

#[derive(Deserialize)]
pub struct AppealForm {
    #[serde(flatten)]
    sign_in: SignIn,
    #[serde(default)]
    sanction: String,
    #[serde(default)]
    statement: String,
}

pub async fn form(State(app): State<Arc<App>>) -> Result<Html<String>, AppError> {
    form_page(&app, None, "", "", "")
}

/// Files an appeal, once the username and password check out.
pub async fn submit(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<AppealForm>,
) -> Result<Html<String>, AppError> {
    let ip = app.client_ip(&headers, peer);
    let statement = clean_statement(&form.statement);
    let refill = |error: &str| {
        form_page(
            &app,
            Some(error),
            &form.sign_in.username,
            &form.sanction,
            &statement,
        )
    };
    let (hash, login) = match sign_in(&app, &form.sign_in, ip).await? {
        Ok(found) => found,
        Err(error) => return refill(error),
    };
    let sanction = form.sanction.as_str();
    let under = match sanction {
        "ban" => login.banned_until_ms.is_some(),
        "mute" => login.muted_until_ms.is_some(),
        _ => return refill("Choose whether you're appealing a ban or a mute."),
    };
    if !under {
        return refill(&format!(
            "Your account isn't {} at the moment, so there's nothing to appeal.",
            if sanction == "ban" { "banned" } else { "muted" }
        ));
    }
    let length = statement.chars().count();
    if !(MIN_STATEMENT_CHARS..=MAX_STATEMENT_CHARS).contains(&length) {
        return refill(&format!(
            "Please explain your appeal in {MIN_STATEMENT_CHARS} to {MAX_STATEMENT_CHARS} characters."
        ));
    }

    let filed = app
        .db
        .file_appeal(&NewAppeal {
            name_hash: hash,
            sanction,
            statement: &statement,
            ip: Some(ip),
        })
        .await?;
    match filed {
        AppealFiled::Filed { id } => {
            info!(
                "{} appealed their {} (appeal {}) from {}",
                login.display_name, sanction, id, ip
            );
            app.templates.render(
                "views/appeal/sent.html",
                context! { id, username => login.display_name },
            )
        }
        AppealFiled::NotSanctioned => refill("There's nothing to appeal on your account."),
        AppealFiled::AlreadyOpen { id } => refill(&format!(
            "You already have an appeal against your {sanction} waiting (reference #{id}). \
             Check its status below."
        )),
        AppealFiled::AlreadyDecided { id } => refill(&format!(
            "Your appeal against this {sanction} has already been decided (reference #{id}). \
             It can't be appealed again."
        )),
    }
}

/// One appeal, as its sender sees it.
#[derive(Serialize)]
struct AppealStatus {
    id: i64,
    sanction: String,
    filed: String,
    /// `received`, `reviewing`, `upheld` or `overturned`.
    status: &'static str,
    note: Option<String>,
    decided: Option<String>,
}

pub async fn status_form(State(app): State<Arc<App>>) -> Result<Html<String>, AppError> {
    app.templates
        .render("views/appeal/status.html", context! {})
}

/// The appeals a player has sent, and how each stands.
pub async fn status(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<SignIn>,
) -> Result<Html<String>, AppError> {
    let ip = app.client_ip(&headers, peer);
    let (hash, login) = match sign_in(&app, &form, ip).await? {
        Ok(found) => found,
        Err(error) => {
            return app.templates.render(
                "views/appeal/status.html",
                context! { error, username => form.username },
            );
        }
    };
    let appeals: Vec<AppealStatus> = app
        .db
        .appeals_of(hash)
        .await?
        .into_iter()
        .map(|a| AppealStatus {
            id: a.id,
            sanction: a.sanction,
            filed: a.filed_date,
            status: match (a.decision.as_deref(), a.case_id) {
                (Some("overturned"), _) => "overturned",
                (Some(_), _) => "upheld",
                (None, Some(_)) => "reviewing",
                (None, None) => "received",
            },
            note: a.decision_note.filter(|n| !n.is_empty()),
            decided: a.decided_date,
        })
        .collect();
    app.templates.render(
        "views/appeal/status.html",
        context! {
            username => login.display_name,
            appeals => Serde(appeals),
            signed_in => true,
        },
    )
}

/// The player whose username and password these are, or what to say.
/// Wrong passwords count against the IP alongside the game's own logins,
/// so this can't be used to guess passwords faster than the game allows.
async fn sign_in(
    app: &App,
    form: &SignIn,
    ip: IpAddr,
) -> Result<Result<(u64, AppealLogin), &'static str>, AppError> {
    const WRONG: &str = "Invalid username or password.";
    if app
        .db
        .recent_login_failures(ip, FAILURE_WINDOW_SECS)
        .await?
        >= MAX_FAILURES
    {
        return Ok(Err(
            "Too many incorrect logins from your address. Please wait a minute and try again.",
        ));
    }
    // Nobody's password is one the game's login screen couldn't type.
    let Ok(name) = validate_name(&form.username) else {
        return Ok(Err(WRONG));
    };
    if !is_typeable_password(&form.password) {
        app.db.record_login_failure(ip).await?;
        return Ok(Err(WRONG));
    }
    let Some(login) = app.db.appeal_login(name.hash).await? else {
        app.db.record_login_failure(ip).await?;
        return Ok(Err(WRONG));
    };
    let passwords = app.passwords.clone();
    let password = form.password.clone();
    let hash = login.password_hash.clone();
    if !spawn_blocking(move || passwords.verify(&password, &hash)).await? {
        app.db.record_login_failure(ip).await?;
        return Ok(Err(WRONG));
    }
    Ok(Ok((name.hash, login)))
}

/// The statement as it's stored: trimmed, line endings made `\n`, and no
/// other control characters.
fn clean_statement(text: &str) -> String {
    text.replace("\r\n", "\n")
        .chars()
        .filter(|&c| c == '\n' || !c.is_control())
        .collect::<String>()
        .trim()
        .to_owned()
}

fn form_page(
    app: &App,
    error: Option<&str>,
    username: &str,
    sanction: &str,
    statement: &str,
) -> Result<Html<String>, AppError> {
    app.templates.render(
        "views/appeal/form.html",
        context! { error, username, sanction, statement, max => MAX_STATEMENT_CHARS },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statements_are_cleaned() {
        assert_eq!(
            clean_statement("  I was\r\nhacked\u{7}!  "),
            "I was\nhacked!"
        );
        assert_eq!(clean_statement("\t\n"), "");
    }
}
