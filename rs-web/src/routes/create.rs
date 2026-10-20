use crate::app::{App, AppError};
use axum::Form;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::Html;
use minijinja::context;
use rs_db::{NewPlayer, PlayerName, validate_name};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::task::spawn_blocking;
use tracing::info;

/// Registrations allowed per IP per [`RATE_LIMIT_HOURS`].
const RATE_LIMIT: i64 = 3;
const RATE_LIMIT_HOURS: i32 = 1;

pub async fn form(State(app): State<Arc<App>>) -> Result<Html<String>, AppError> {
    username_page(&app, None)
}

#[derive(Deserialize)]
pub struct CreateForm {
    #[serde(default)]
    step: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    password2: String,
    #[serde(default)]
    terms: String,
}

pub async fn submit(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<CreateForm>,
) -> Result<Html<String>, AppError> {
    let ip = app.client_ip(&headers, peer);
    if app.db.recent_registrations(ip, RATE_LIMIT_HOURS).await? >= RATE_LIMIT {
        app.metrics
            .registrations_rate_limited
            .fetch_add(1, Ordering::Relaxed);
        return username_page(
            &app,
            Some("You have created too many accounts recently. Please try again later."),
        );
    }

    let name = match check_name(&app, &form.username).await? {
        Ok(name) => name,
        Err(error) => return username_page(&app, Some(&error)),
    };
    let username = name.display.as_str();

    match form.step.as_str() {
        "username" => page(&app, "views/create/terms.html", username, None),
        "terms" => page(&app, "views/create/password.html", username, None),
        "password" => {
            if let Err(error) = check_password(&form) {
                return page(&app, "views/create/password.html", username, Some(&error));
            }

            let passwords = app.passwords.clone();
            let password = form.password;
            let Some(password_hash) = spawn_blocking(move || passwords.hash(&password)).await?
            else {
                return Err(anyhow::anyhow!("password hashing failed").into());
            };
            let created = app
                .db
                .create_player(&NewPlayer {
                    name_hash: name.hash,
                    display_name: username,
                    password_hash: &password_hash,
                    staff_mod_level: 0,
                    registration_ip: Some(ip),
                })
                .await?;
            if !created {
                // Registered by someone else since the last check.
                return username_page(&app, Some(&taken(username)));
            }
            info!("Registered player '{}' from {}", username, ip);
            app.metrics.registrations.fetch_add(1, Ordering::Relaxed);
            Ok(app
                .templates
                .render("views/create/finish.html", context! {})?)
        }
        _ => username_page(&app, None),
    }
}

/// A valid, unreserved and unregistered name, or the error to show.
async fn check_name(app: &App, input: &str) -> Result<Result<PlayerName, String>, AppError> {
    let name = match validate_name(input) {
        Ok(name) => name,
        Err(e) => return Ok(Err(e.to_string())),
    };
    if is_reserved(&name.display) || is_filtered(app, &name.display) {
        return Ok(Err("That username is not available.".to_string()));
    }
    if app.db.name_taken(name.hash).await? {
        return Ok(Err(taken(&name.display)));
    }
    Ok(Ok(name))
}

/// Names that would pass for staff.
fn is_reserved(display: &str) -> bool {
    let lower = display.to_ascii_lowercase();
    lower.starts_with("mod ") || lower.starts_with("m0d ")
}

/// Whether the game's chat filter would censor any part of the name. Names
/// can't contain `*`, so any in the output came from the filter.
fn is_filtered(app: &App, display: &str) -> bool {
    app.wordenc
        .as_ref()
        .is_some_and(|w| w.filter(display).contains('*'))
}

fn taken(display: &str) -> String {
    format!("The username \"{display}\" is already taken.")
}

/// A new password, typed twice, by the game's rule ([`rs_db::check_new_password`]):
/// 5 to 20 characters its login screen can type.
pub(super) fn check_new_passwords(password: &str, again: &str) -> Result<(), String> {
    rs_db::check_new_password(password).map_err(|e| e.to_string())?;
    if password != again {
        return Err("Your passwords do not match.".to_owned());
    }
    Ok(())
}

fn check_password(form: &CreateForm) -> Result<(), String> {
    check_new_passwords(&form.password, &form.password2)?;
    if form.terms != "yes" {
        return Err("You must agree to the terms and conditions to create an account.".to_owned());
    }
    Ok(())
}

fn username_page(app: &App, error: Option<&str>) -> Result<Html<String>, AppError> {
    app.templates
        .render("views/create/username.html", context! { error })
}

fn page(
    app: &App,
    template: &str,
    username: &str,
    error: Option<&str>,
) -> Result<Html<String>, AppError> {
    app.templates.render(template, context! { username, error })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(password: &str, password2: &str, terms: &str) -> CreateForm {
        CreateForm {
            step: "password".into(),
            username: "bob".into(),
            password: password.into(),
            password2: password2.into(),
            terms: terms.into(),
        }
    }

    #[test]
    fn password_rules() {
        assert!(check_password(&form("hunter2", "hunter2", "yes")).is_ok());
        assert!(check_password(&form("a b!\"$%^&*()-_=+[{]};:'@#~,<.>/?\\|", "", "yes")).is_err());
        assert!(check_password(&form("p@ss w0rd!", "p@ss w0rd!", "yes")).is_ok());
        assert!(check_password(&form("four", "four", "yes")).is_err());
        assert!(check_password(&form(&"x".repeat(21), &"x".repeat(21), "yes")).is_err());
        assert!(check_password(&form("back`tick", "back`tick", "yes")).is_err());
        // The login screen types a pound sign, and sends it as the same character.
        assert!(check_password(&form("pound\u{a3}1", "pound\u{a3}1", "yes")).is_ok());
        assert!(check_password(&form("caf\u{e9}12", "caf\u{e9}12", "yes")).is_err());
        assert!(check_password(&form("hunter2", "hunter3", "yes")).is_err());
        assert!(check_password(&form("hunter2", "hunter2", "")).is_err());
    }

    #[test]
    fn game_word_filter_catches_names() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(rs_pack::CONTENT_DIR);
        let jag = rs_pack::pack::wordenc::pack_wordenc(&dir);
        let filter = rs_pack::cache::wordenc::WordEncProvider::from_jag(&jag);
        let blocked = |name: &str| filter.filter(name).contains('*');
        for name in ["Arse", "Big Arse", "F U C K", "Mr Bitch"] {
            assert!(blocked(name), "{name} should be blocked");
        }
        for name in [
            "Zezima",
            "Red Rooster",
            "Classic",
            "Grass",
            "Assassin",
            "Cook",
        ] {
            assert!(!blocked(name), "{name} should be allowed");
        }
    }

    #[test]
    fn staff_lookalikes_are_reserved() {
        assert!(is_reserved("Mod Ash"));
        assert!(is_reserved("M0d Ash"));
        assert!(!is_reserved("Modest"));
    }
}
