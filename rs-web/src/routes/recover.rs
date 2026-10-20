use super::create::check_new_passwords;
use crate::app::{App, AppError};
use axum::Form;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::Html;
use minijinja::context;
use rs_db::validate_name;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::info;

/// Wrong codes an IP may give, here or at the game's login (the same
/// count), before it's refused for [`FAILURE_WINDOW_SECS`].
const MAX_FAILURES: i64 = 5;
const FAILURE_WINDOW_SECS: i32 = 60;

#[derive(Deserialize)]
pub struct RecoverForm {
    #[serde(default)]
    username: String,
    #[serde(default)]
    code: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    password2: String,
}

pub async fn form(State(app): State<Arc<App>>) -> Result<Html<String>, AppError> {
    app.templates.render("views/recover/form.html", context! {})
}

/// The code as staff read it out, however it was typed: `abcd efgh-jklm`
/// is `ABCDEFGHJKLM`.
fn typed_code(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Sets a new password with the code staff gave the player.
pub async fn submit(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<RecoverForm>,
) -> Result<Html<String>, AppError> {
    const WRONG: &str = "That username and code don't match a password reset. \
                         Check the code, or ask staff for a new one if it's more than 3 days old.";
    let ip = app.client_ip(&headers, peer);
    let username = form.username.clone();
    let refill = |error: &str| {
        app.templates
            .render("views/recover/form.html", context! { error, username })
    };
    if app
        .db
        .recent_login_failures(ip, FAILURE_WINDOW_SECS)
        .await?
        >= MAX_FAILURES
    {
        return refill(
            "Too many incorrect attempts from your address. Please wait a minute and try again.",
        );
    }
    if let Err(error) = check_new_passwords(&form.password, &form.password2) {
        return refill(&error);
    }
    let Ok(name) = validate_name(&form.username) else {
        return refill(WRONG);
    };
    let Some(reset) = app.db.password_reset(name.hash).await? else {
        app.db.record_login_failure(ip).await?;
        return refill(WRONG);
    };
    let passwords = app.passwords.clone();
    let code = typed_code(&form.code);
    let code_hash = reset.code_hash.clone();
    let password = form.password.clone();
    let checked = spawn_blocking(move || {
        passwords
            .verify(&code, &code_hash)
            .then(|| passwords.hash(&password))
    })
    .await?;
    let password_hash = match checked {
        None => {
            app.db.record_login_failure(ip).await?;
            return refill(WRONG);
        }
        Some(None) => return Err(anyhow::anyhow!("password hashing failed").into()),
        Some(Some(hash)) => hash,
    };
    if !app
        .db
        .redeem_password_reset(name.hash, &reset.code_hash, &password_hash)
        .await?
    {
        return refill(WRONG);
    }
    info!(
        "{} set a new password with a reset code from {}",
        reset.display_name, ip
    );
    app.templates.render(
        "views/recover/done.html",
        context! { username => reset.display_name },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_typed_loosely() {
        assert_eq!(typed_code(" abcd-EFGH jklm\n"), "ABCDEFGHJKLM");
        assert_eq!(typed_code("ABCDEFGHJKLM"), "ABCDEFGHJKLM");
    }
}
