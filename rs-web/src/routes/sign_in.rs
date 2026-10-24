use crate::app::{App, AppError};
use rs_db::{AppealLogin, is_typeable_password, validate_name};
use serde::Deserialize;
use std::net::IpAddr;

/// Wrong passwords an IP may give, on the website or at the game's login
/// (the same count), before it's refused for [`FAILURE_WINDOW_SECS`].
const MAX_FAILURES: i64 = 5;
const FAILURE_WINDOW_SECS: i32 = 60;

#[derive(Deserialize)]
pub struct SignIn {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

/// The player whose username and password these are, or what to say.
/// Wrong passwords count against the IP alongside the game's own logins,
/// so this can't be used to guess passwords faster than the game allows.
pub async fn sign_in(
    app: &App,
    form: &SignIn,
    ip: IpAddr,
) -> Result<Result<(u64, AppealLogin), &'static str>, AppError> {
    const WRONG: &str = "Invalid username or password.";
    let Ok(name) = validate_name(&form.username) else {
        return Ok(Err(WRONG));
    };
    // Counted as wrong until it's found right.
    let Some(attempt) = app
        .db
        .claim_login_attempt(ip, FAILURE_WINDOW_SECS, MAX_FAILURES)
        .await?
    else {
        return Ok(Err(
            "Too many incorrect logins from your address. Please wait a minute and try again.",
        ));
    };
    // Nobody's password is one the game's login screen couldn't type.
    if !is_typeable_password(&form.password) {
        return Ok(Err(WRONG));
    }
    let Some(login) = app.db.appeal_login(name.hash).await? else {
        return Ok(Err(WRONG));
    };
    let password = form.password.clone();
    let hash = login.password_hash.clone();
    if !app
        .with_passwords(move |p| p.verify(&password, &hash))
        .await?
    {
        return Ok(Err(WRONG));
    }
    app.db.forgive_login_attempt(attempt).await?;
    Ok(Ok((name.hash, login)))
}
