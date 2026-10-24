use super::create::check_new_passwords;
use super::sign_in::{SignIn, sign_in};
use crate::app::{App, AppError};
use crate::worlds::world_of_node;
use axum::Form;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{Html, IntoResponse, Response};
use minijinja::context;
use minijinja::value::Serde;
use rs_db::{AccountDeleted, AccountLogin};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

/// How many of their latest logins the account page lists.
const LOGINS_SHOWN: i64 = 50;

#[derive(Deserialize)]
pub struct PasswordForm {
    #[serde(flatten)]
    sign_in: SignIn,
    #[serde(default)]
    new_password: String,
    #[serde(default)]
    new_password2: String,
}

#[derive(Deserialize)]
pub struct DeleteForm {
    #[serde(flatten)]
    sign_in: SignIn,
    /// Their username again, typed out to show they mean it.
    #[serde(default)]
    confirm: String,
}

/// A login, as the account page lists it.
#[derive(Serialize)]
struct LoginRow {
    at: String,
    world: i16,
    ip: String,
    device: String,
}

impl From<AccountLogin> for LoginRow {
    fn from(l: AccountLogin) -> Self {
        let device = match (l.web, l.platform) {
            (Some(true), Some(platform)) => format!("Browser ({platform})"),
            (Some(true), None) => "Browser".to_owned(),
            _ => "Desktop client".to_owned(),
        };
        Self {
            at: l.at,
            world: world_of_node(l.node),
            ip: l.ip,
            device,
        }
    }
}

pub async fn form(State(app): State<Arc<App>>) -> Result<Html<String>, AppError> {
    app.templates.render("views/account/form.html", context! {})
}

/// Signs in and shows the account.
pub async fn show(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<SignIn>,
) -> Result<Html<String>, AppError> {
    let ip = app.client_ip(&headers, peer);
    match sign_in(&app, &form, ip).await? {
        Ok((hash, _)) => home(&app, hash, None, None).await,
        Err(error) => refill(&app, error, &form.username),
    }
}

/// Changes the password, given the current one.
pub async fn password(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<PasswordForm>,
) -> Result<Html<String>, AppError> {
    let ip = app.client_ip(&headers, peer);
    let (hash, login) = match sign_in(&app, &form.sign_in, ip).await? {
        Ok(found) => found,
        Err(error) => return refill(&app, error, &form.sign_in.username),
    };
    if let Err(error) = check_new_passwords(&form.new_password, &form.new_password2) {
        return home(&app, hash, None, Some(&error)).await;
    }
    let new_password = form.new_password.clone();
    let Some(new_hash) = app.with_passwords(move |p| p.hash(&new_password)).await? else {
        return Err(anyhow::anyhow!("password hashing failed").into());
    };
    if !app
        .db
        .change_password(hash, &login.password_hash, &new_hash)
        .await?
    {
        return home(
            &app,
            hash,
            None,
            Some("Your password was changed somewhere else just now. Sign in again."),
        )
        .await;
    }
    info!("{} changed their password from {}", login.display_name, ip);
    home(&app, hash, Some("Your password has been changed."), None).await
}

/// Everything kept about the player, as a JSON file.
pub async fn data(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<SignIn>,
) -> Result<Response, AppError> {
    let ip = app.client_ip(&headers, peer);
    let (hash, login) = match sign_in(&app, &form, ip).await? {
        Ok(found) => found,
        Err(error) => return Ok(refill(&app, error, &form.username)?.into_response()),
    };
    let Some(json) = app.db.export_account(hash).await? else {
        return Ok(refill(&app, "Invalid username or password.", &form.username)?.into_response());
    };
    info!("{} downloaded their data from {}", login.display_name, ip);
    let file = format!(
        "attachment; filename=\"{}-data.json\"",
        login.display_name.replace(' ', "_")
    );
    let mut response = json.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(header::CONTENT_DISPOSITION, HeaderValue::from_str(&file)?);
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

/// Deletes the account, once its username is typed again to confirm.
pub async fn delete(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<DeleteForm>,
) -> Result<Html<String>, AppError> {
    let ip = app.client_ip(&headers, peer);
    let (hash, login) = match sign_in(&app, &form.sign_in, ip).await? {
        Ok(found) => found,
        Err(error) => return refill(&app, error, &form.sign_in.username),
    };
    if !form
        .confirm
        .trim()
        .eq_ignore_ascii_case(login.display_name.trim())
    {
        return home(
            &app,
            hash,
            None,
            Some("To delete your account, type your username in the box exactly as it's shown."),
        )
        .await;
    }
    let error = match app.db.delete_account(hash).await? {
        AccountDeleted::Deleted => {
            info!("{} deleted their account from {}", login.display_name, ip);
            return app.templates.render(
                "views/account/deleted.html",
                context! { username => login.display_name },
            );
        }
        AccountDeleted::Online => {
            "You're logged in to the game. Log out first, then try again in a minute."
        }
        AccountDeleted::Sanctioned => {
            "Your account can't be deleted while it's banned or muted. \
             Email contact@rustcity.org if you still want it deleted."
        }
        AccountDeleted::Staff => {
            "Staff accounts are deleted by another member of staff. Email contact@rustcity.org."
        }
        AccountDeleted::Gone => return refill(&app, "That account no longer exists.", ""),
    };
    home(&app, hash, None, Some(error)).await
}

/// The account page, signed in, with `message` or `error` on it.
async fn home(
    app: &App,
    hash: u64,
    message: Option<&str>,
    error: Option<&str>,
) -> Result<Html<String>, AppError> {
    let Some(account) = app.db.account(hash).await? else {
        return refill(app, "That account no longer exists.", "");
    };
    let logins: Vec<LoginRow> = app
        .db
        .account_logins(hash, LOGINS_SHOWN)
        .await?
        .into_iter()
        .map(LoginRow::from)
        .collect();
    app.templates.render(
        "views/account/home.html",
        context! {
            username => account.display_name,
            created => account.created_date,
            staff => account.staff,
            online => account.online,
            sanctioned => account.banned || account.muted,
            logins => Serde(logins),
            shown => LOGINS_SHOWN,
            message,
            error,
        },
    )
}

/// The sign-in form again, with `error`.
fn refill(app: &App, error: &str, username: &str) -> Result<Html<String>, AppError> {
    app.templates
        .render("views/account/form.html", context! { error, username })
}
