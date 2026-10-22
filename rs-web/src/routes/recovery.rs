use super::appeal::clean_statement;
use super::recover::typed_code;
use crate::app::{App, AppError};
use axum::Form;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::Html;
use minijinja::context;
use minijinja::value::Serde;
use rs_db::{
    NewRecovery, RECOVERIES_PER_ADDRESS, RecoveryAnswers, RecoveryFiled, code_from_bits,
    is_typeable_password, validate_name,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::info;

/// Wrong recovery keys an IP may give, here or at the game's login (the
/// same count), before it's refused for [`FAILURE_WINDOW_SECS`].
const MAX_FAILURES: i64 = 5;
const FAILURE_WINDOW_SECS: i32 = 60;

const MIN_DETAILS_CHARS: usize = 20;
const MAX_DETAILS_CHARS: usize = 2000;
/// The longest answer to any other question.
const MAX_ANSWER_CHARS: usize = 500;
/// How many passwords they may say they used.
const MAX_PASSWORDS: usize = 3;
/// The most browser ids a request carries: one per world.
const MAX_UIDS: usize = 8;
/// Groups of 4 in a recovery key: 80 random bits.
const KEY_GROUPS: usize = 4;

const REASONS: [&str; 3] = ["forgot", "hijacked", "other"];
const DEVICES: [&str; 3] = ["computer", "phone", "client"];

#[derive(Default, Deserialize, Serialize)]
pub struct RequestForm {
    #[serde(default)]
    username: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    details: String,
    #[serde(default, skip_serializing)]
    password1: String,
    #[serde(default, skip_serializing)]
    password2: String,
    #[serde(default, skip_serializing)]
    password3: String,
    #[serde(default)]
    created_month: String,
    #[serde(default)]
    last_played: String,
    #[serde(default)]
    device: String,
    #[serde(default)]
    skills: String,
    #[serde(default)]
    items: String,
    #[serde(default)]
    location: String,
    #[serde(default)]
    friends: String,
    #[serde(default)]
    traded_with: String,
    #[serde(default)]
    other_accounts: String,
    #[serde(default, skip_serializing)]
    uids: String,
}

pub async fn form(State(app): State<Arc<App>>) -> Result<Html<String>, AppError> {
    form_page(&app, None, &RequestForm::default())
}

/// Files a recovery request. Whether a password they gave is the
/// account's is kept for staff, and never shown here.
pub async fn submit(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<RequestForm>,
) -> Result<Html<String>, AppError> {
    let ip = app.client_ip(&headers, peer);
    let refill = |error: &str| form_page(&app, Some(error), &form);
    let Ok(name) = validate_name(&form.username) else {
        return refill("Enter the username of the account you can't get into.");
    };
    if !REASONS.contains(&form.reason.as_str()) {
        return refill("Choose why you can't get into your account.");
    }
    let details = clean_statement(&form.details);
    if !(MIN_DETAILS_CHARS..=MAX_DETAILS_CHARS).contains(&details.chars().count()) {
        return refill(&format!(
            "Please tell us what happened in {MIN_DETAILS_CHARS} to {MAX_DETAILS_CHARS} characters."
        ));
    }
    let Some(answers) = answers(&form) else {
        return refill("Check the dates you gave: they can't be in the future.");
    };
    if app.db.recent_recoveries_from(ip).await? >= RECOVERIES_PER_ADDRESS {
        return refill(TOO_MANY);
    }
    let Some(login) = app.db.appeal_login(name.hash).await? else {
        return refill("There's no account with that username.");
    };

    // Checked against the account's password as it is now; a hijacker who
    // changed it leaves nothing to match, which says something too.
    let guesses: Vec<String> = [&form.password1, &form.password2, &form.password3]
        .into_iter()
        .filter(|p| !p.is_empty() && is_typeable_password(p))
        .take(MAX_PASSWORDS)
        .cloned()
        .collect();
    let passwords_given = guesses.len() as i16;
    let key = new_key()?;
    let passwords = app.passwords.clone();
    let hash = login.password_hash;
    let typed = typed_code(&key);
    let (password_matched, key_hash) = spawn_blocking(move || {
        let matched = guesses.iter().any(|g| passwords.verify(g, &hash));
        (matched, passwords.hash(&typed))
    })
    .await?;
    let key_hash = key_hash.ok_or_else(|| anyhow::anyhow!("hashing a recovery key failed"))?;

    let uids = uids(&form.uids);
    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    let user_agent = header("user-agent").and_then(|ua| clean_header(ua, 300));
    let platform = header("sec-ch-ua-platform").and_then(|p| clean_header(p.trim_matches('"'), 32));
    let language = header("accept-language")
        .and_then(|l| clean_header(l.split([',', ';']).next().unwrap_or(""), 35));
    let filed = app
        .db
        .file_recovery(&NewRecovery {
            name_hash: name.hash,
            key_hash: &key_hash,
            reason: &form.reason,
            details: &details,
            passwords_given,
            password_matched,
            answers: &answers,
            ip,
            uids: &uids,
            user_agent: user_agent.as_deref(),
            platform: platform.as_deref(),
            language: language.as_deref(),
        })
        .await?;
    match filed {
        RecoveryFiled::Filed { id } => {
            info!(
                "Recovery request {} for {} from {}",
                id, login.display_name, ip
            );
            app.templates.render(
                "views/recover/sent.html",
                context! { id, key, username => login.display_name },
            )
        }
        RecoveryFiled::NoSuchPlayer => refill("There's no account with that username."),
        RecoveryFiled::TooManyFromAddress => refill(TOO_MANY),
        RecoveryFiled::TooManyWaiting => refill(
            "There are already several requests waiting for this account. \
             Check on yours on the request status page, with its reference and recovery key.",
        ),
    }
}

const TOO_MANY: &str = "You've sent as many requests as you can today. \
                        Check on the one you sent with its reference and recovery key.";

#[derive(Deserialize)]
pub struct StatusForm {
    #[serde(default)]
    reference: String,
    #[serde(default)]
    key: String,
}

/// One request, as its sender sees it.
#[derive(Serialize)]
struct RequestStatus {
    id: i64,
    username: String,
    filed: String,
    /// `received`, `reviewing`, `approved` or `denied`.
    status: &'static str,
    note: Option<String>,
    decided: Option<String>,
    /// An approval whose new password can still be set.
    can_reset: bool,
}

pub async fn status_form(State(app): State<Arc<App>>) -> Result<Html<String>, AppError> {
    app.templates
        .render("views/recover/status.html", context! {})
}

/// How a request stands, for whoever has its reference and recovery key.
/// Wrong keys count against the IP alongside the game's own logins.
pub async fn status(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<StatusForm>,
) -> Result<Html<String>, AppError> {
    const WRONG: &str = "That reference and recovery key don't match a request.";
    let ip = app.client_ip(&headers, peer);
    let reference = form.reference.trim().trim_start_matches('#').to_owned();
    let refill = |error: &str| {
        app.templates.render(
            "views/recover/status.html",
            context! { error, reference => reference.as_str() },
        )
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
    let key = typed_code(&form.key);
    let found = match reference.parse::<i64>() {
        Ok(id) if !key.is_empty() => app.db.recovery(id).await?,
        _ => None,
    };
    let Some(request) = found else {
        app.db.record_login_failure(ip).await?;
        return refill(WRONG);
    };
    let passwords = app.passwords.clone();
    let hash = request.key_hash.clone();
    if !spawn_blocking(move || passwords.verify(&key, &hash)).await? {
        app.db.record_login_failure(ip).await?;
        return refill(WRONG);
    }
    let status = RequestStatus {
        id: request.id,
        username: request.player,
        filed: request.filed_date,
        status: match (request.decision.as_deref(), request.case_id) {
            (Some("approved"), _) => "approved",
            (Some(_), _) => "denied",
            (None, Some(_)) => "reviewing",
            (None, None) => "received",
        },
        note: request.decision_note.filter(|n| !n.is_empty()),
        decided: request.decided_date,
        can_reset: request.reset_until_ms.is_some(),
    };
    app.templates.render(
        "views/recover/status.html",
        context! { request => Serde(status), key => form.key.trim() },
    )
}

/// The answers as stored: free text cleaned and cut short, dates checked.
/// `None` if a date is in the future or isn't one.
fn answers(form: &RequestForm) -> Option<RecoveryAnswers> {
    let text = |value: &str| {
        let value: String = clean_statement(value)
            .chars()
            .take(MAX_ANSWER_CHARS)
            .collect();
        (!value.is_empty()).then_some(value)
    };
    let today = today();
    let created_month = match form.created_month.trim() {
        "" => None,
        month => Some(month_answer(month, today)?),
    };
    let last_played = match form.last_played.trim() {
        "" => None,
        day => Some(day_answer(day, today)?),
    };
    Some(RecoveryAnswers {
        created_month,
        last_played,
        device: DEVICES
            .contains(&form.device.as_str())
            .then(|| form.device.clone()),
        skills: text(&form.skills),
        items: text(&form.items),
        location: text(&form.location),
        friends: text(&form.friends),
        traded_with: text(&form.traded_with),
        other_accounts: text(&form.other_accounts),
    })
}

/// Today as (year, month, day), in UTC.
fn today() -> (i32, u32, u32) {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400) as i64;
    civil_from_days(days)
}

/// The (year, month, day) `days` after 1970-01-01 (Howard Hinnant's
/// algorithm).
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
    (year, month, day)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `2026-03`, if it's a month no later than `today`'s.
fn month_answer(month: &str, today: (i32, u32, u32)) -> Option<String> {
    let (year, month) = month.split_once('-')?;
    let (year, month): (i32, u32) = (year.parse().ok()?, month.parse().ok()?);
    ((2000..=today.0).contains(&year)
        && (1..=12).contains(&month)
        && (year, month) <= (today.0, today.1))
        .then(|| format!("{year:04}-{month:02}"))
}

/// `2026-10-18`, if it's a day no later than `today`.
fn day_answer(day: &str, today: (i32, u32, u32)) -> Option<String> {
    let mut parts = day.splitn(3, '-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    ((2000..=today.0).contains(&year)
        && (1..=12).contains(&month)
        && (1..=days_in_month(year, month)).contains(&day)
        && (year, month, day) <= today)
        .then(|| format!("{year:04}-{month:02}-{day:02}"))
}

/// The browser ids the form gathered: real ones (not 0, nor the unpatched
/// client's 1337), each once.
fn uids(list: &str) -> Vec<i32> {
    let mut uids: Vec<i32> = Vec::new();
    for uid in list.split(',').filter_map(|v| v.trim().parse::<i32>().ok()) {
        if uid != 0 && uid != 1337 && !uids.contains(&uid) && uids.len() < MAX_UIDS {
            uids.push(uid);
        }
    }
    uids
}

/// A header's value as logins keep it: printable ASCII, at most `max`.
fn clean_header(value: &str, max: usize) -> Option<String> {
    let value: String = value
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(max)
        .collect();
    (!value.is_empty()).then_some(value)
}

/// A new recovery key, as `XXXX-XXXX-XXXX-XXXX`.
fn new_key() -> anyhow::Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| anyhow::anyhow!("no randomness: {e}"))?;
    Ok(code_from_bits(u128::from_le_bytes(bytes), KEY_GROUPS))
}

/// The worlds whose `/uid` pages the form asks for the browser's ids.
fn uid_pages(app: &App) -> Vec<String> {
    app.worlds
        .iter()
        .map(|w| format!("{}/uid", w.address.trim_end_matches('/')))
        .collect()
}

fn form_page(app: &App, error: Option<&str>, form: &RequestForm) -> Result<Html<String>, AppError> {
    app.templates.render(
        "views/recover/request.html",
        context! {
            error,
            form => Serde(form),
            max => MAX_DETAILS_CHARS,
            max_answer => MAX_ANSWER_CHARS,
            uid_pages => uid_pages(app),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_checked() {
        let today = (2026, 10, 22);
        assert_eq!(month_answer("2026-3", today).as_deref(), Some("2026-03"));
        assert_eq!(month_answer("2026-11", today), None);
        assert_eq!(month_answer("1999-01", today), None);
        assert_eq!(
            day_answer("2026-10-22", today).as_deref(),
            Some("2026-10-22")
        );
        assert_eq!(day_answer("2026-10-23", today), None);
        assert_eq!(day_answer("2025-02-29", today), None);
        assert_eq!(
            day_answer("2024-02-29", today).as_deref(),
            Some("2024-02-29")
        );
        assert_eq!(day_answer("soon", today), None);
    }

    #[test]
    fn days_count_from_1970() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_748), (2026, 10, 22));
    }

    #[test]
    fn only_real_browser_ids_are_kept() {
        assert_eq!(uids("5, 0, 1337, 5,-9,x"), [5, -9]);
    }

    #[test]
    fn keys_read_out_like_reset_codes() {
        let key = new_key().unwrap();
        assert_eq!(key.len(), 19);
        assert_eq!(typed_code(&key).len(), 16);
        assert_ne!(new_key().unwrap(), key);
    }
}
