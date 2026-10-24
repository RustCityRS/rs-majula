//! The Message Centre: messages staff send a player, which they read here.
//! One way only: players can't reply. The site keeps no session, so after
//! one sign-in each page carries a short-lived token (see [`SignInTokens`])
//! in a hidden field of its forms, to move between the list and a message.
//!
//! [`SignInTokens`]: crate::token::SignInTokens

use super::appeal::clean_statement;
use super::sign_in::{SignIn, sign_in};
use crate::app::{App, AppError};
use axum::Form;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::Html;
use minijinja::context;
use minijinja::value::Serde;
use rs_admin_api::{MAX_MESSAGE_CHARS, MAX_MESSAGE_SUBJECT_CHARS};
use rs_db::{AppealLogin, PlayerMessage};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

/// How many messages the list shows, newest first.
const SHOWN: i64 = 100;

#[derive(Deserialize)]
pub struct ListForm {
    #[serde(flatten)]
    sign_in: SignIn,
    #[serde(default)]
    token: String,
}

#[derive(Deserialize)]
pub struct ViewForm {
    #[serde(default)]
    token: String,
    #[serde(default)]
    id: i64,
}

/// A message, as the list shows it.
#[derive(Serialize)]
struct MessageRow {
    id: i64,
    subject: String,
    sent: String,
    unread: bool,
}

impl From<PlayerMessage> for MessageRow {
    fn from(m: PlayerMessage) -> Self {
        Self {
            id: m.id,
            subject: m.subject,
            sent: m.sent_date,
            unread: !m.read,
        }
    }
}

/// A subject as it's stored (trimmed, with no control characters or line
/// breaks), or what's wrong with it.
pub(crate) fn subject_text(raw: &str) -> Result<String, String> {
    let subject = raw
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .to_owned();
    if subject.is_empty() || subject.chars().count() > MAX_MESSAGE_SUBJECT_CHARS {
        return Err(format!(
            "Subjects are 1 to {MAX_MESSAGE_SUBJECT_CHARS} characters."
        ));
    }
    Ok(subject)
}

/// A message as it's stored (trimmed, line endings made `\n`, no other
/// control characters), or what's wrong with it.
pub(crate) fn message_text(raw: &str) -> Result<String, String> {
    let body = clean_statement(raw);
    if body.is_empty() || body.chars().count() > MAX_MESSAGE_CHARS {
        return Err(format!("Messages are 1 to {MAX_MESSAGE_CHARS} characters."));
    }
    Ok(body)
}

pub async fn form(State(app): State<Arc<App>>) -> Result<Html<String>, AppError> {
    app.templates
        .render("views/messages/form.html", context! {})
}

/// Signs in, or carries on signed in, and lists their messages.
pub async fn list(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<ListForm>,
) -> Result<Html<String>, AppError> {
    if !form.token.is_empty() {
        return match signed_in(&app, &form.token).await? {
            Some(login) => home(&app, &login, None).await,
            None => expired(&app),
        };
    }
    let ip = app.client_ip(&headers, peer);
    match sign_in(&app, &form.sign_in, ip).await? {
        Ok((_, login)) => home(&app, &login, None).await,
        Err(error) => refill(&app, error, &form.sign_in.username),
    }
}

/// One message, which marks it read.
pub async fn view(
    State(app): State<Arc<App>>,
    Form(form): Form<ViewForm>,
) -> Result<Html<String>, AppError> {
    let Some(login) = signed_in(&app, &form.token).await? else {
        return expired(&app);
    };
    let Some(message) = app.db.open_message(login.id, form.id).await? else {
        return home(&app, &login, Some("That message couldn't be found.")).await;
    };
    app.templates.render(
        "views/messages/message.html",
        context! {
            token => app.tokens.issue(login.id, &login.password_hash),
            subject => message.subject,
            sent => message.sent_date,
            body => message.body,
        },
    )
}

/// The player a token signs in, if it's valid for them as they are now: it
/// names their row, and was issued under the password they still have.
async fn signed_in(app: &App, token: &str) -> Result<Option<AppealLogin>, AppError> {
    let Some(id) = app.tokens.claimed(token) else {
        return Ok(None);
    };
    let login = app.db.login_by_id(id).await?;
    Ok(login.filter(|l| app.tokens.check(token, l.id, &l.password_hash)))
}

/// The list, signed in as `login`, with `error` on it.
async fn home(
    app: &App,
    login: &AppealLogin,
    error: Option<&str>,
) -> Result<Html<String>, AppError> {
    let messages: Vec<MessageRow> = app
        .db
        .messages_of(login.id, SHOWN)
        .await?
        .into_iter()
        .map(MessageRow::from)
        .collect();
    app.templates.render(
        "views/messages/list.html",
        context! {
            token => app.tokens.issue(login.id, &login.password_hash),
            username => &login.display_name,
            messages => Serde(messages),
            shown => SHOWN,
            error,
        },
    )
}

/// The sign-in form again, after a token ran out or was refused.
fn expired(app: &App) -> Result<Html<String>, AppError> {
    refill(
        app,
        "Your login has run out: it lasts 30 minutes from your last click, and ends when your \
         password changes or the site restarts. Please log in again.",
        "",
    )
}

/// The sign-in form again, with `error`.
fn refill(app: &App, error: &str, username: &str) -> Result<Html<String>, AppError> {
    app.templates
        .render("views/messages/form.html", context! { error, username })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subjects_are_one_line_and_limited() {
        assert_eq!(
            subject_text("  Lost\r\nmy\titems\u{7} ").unwrap(),
            "Lostmyitems"
        );
        assert!(subject_text("").is_err());
        assert!(subject_text(" \n\t ").is_err());
        let longest = "é".repeat(MAX_MESSAGE_SUBJECT_CHARS);
        assert_eq!(subject_text(&longest).unwrap(), longest);
        assert!(subject_text(&format!("{longest}a")).is_err());
    }

    #[test]
    fn messages_keep_line_breaks_and_are_limited() {
        assert_eq!(
            message_text(" Hello\r\n\r\nWe found\u{0}\u{1b} your\u{7f} items. ").unwrap(),
            "Hello\n\nWe found your items."
        );
        assert!(message_text("").is_err());
        assert!(message_text("\r\n \u{7}").is_err());
        // Counted in characters, a line break as one.
        let longest = format!("{}\r\n{}", "é".repeat(1999), "é".repeat(2000));
        assert_eq!(
            message_text(&longest).unwrap().chars().count(),
            MAX_MESSAGE_CHARS
        );
        assert!(message_text(&format!("{longest}a")).is_err());
    }
}
