use crate::worlds::{Outcome, Reply};
use reqwest::Client;
use reqwest::header::{CONTENT_TYPE, RETRY_AFTER};
use serde_json::json;
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::warn;

/// How long Discord gets to take a message.
const TIMEOUT: Duration = Duration::from_secs(10);
/// Messages waiting to go out; more than this and the newest are dropped.
const QUEUE: usize = 64;
/// The longest a rate limit is waited out before one retry.
const MAX_RETRY_WAIT: Duration = Duration::from_secs(30);

/// Tells a Discord channel, through its webhook, what staff did to the
/// worlds: one message per action, however many worlds it covered.
pub struct Discord {
    queue: Option<Sender<String>>,
}

impl Discord {
    /// Posts to `webhook`, or nowhere if `None`. Must be made inside the
    /// runtime: it starts the task that sends the messages, in order.
    pub fn new(webhook: Option<String>) -> Self {
        let queue = webhook.map(|url| {
            let client = Client::builder()
                .timeout(TIMEOUT)
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("building the HTTP client");
            let (queue, messages) = mpsc::channel(QUEUE);
            tokio::spawn(send_all(client, url, messages));
            queue
        });
        Self { queue }
    }

    /// Posts what `by` did to the worlds that took it, in one message, with
    /// any that didn't under it. `say` gets those worlds ("Worlds 1 and 2")
    /// and the verb to go with them ("is" or "are"). Nothing is posted if no
    /// world took it.
    pub fn world_action(
        &self,
        by: &str,
        replies: &[Reply<rs_admin_api::Outcome>],
        say: impl FnOnce(&str, &str) -> String,
    ) {
        if self.queue.is_some()
            && let Some(message) = world_notice(replies, say)
        {
            self.post_by(by, message);
        }
    }

    /// Posts `message`, signed by `by`.
    pub fn post_by(&self, by: &str, message: String) {
        let Some(queue) = &self.queue else {
            return;
        };
        let content = format!("{message}\n-# by {}", escape(by));
        if queue.try_send(content).is_err() {
            warn!("Dropped a Discord world notice: too many waiting");
        }
    }
}

/// [`Discord::world_action`]'s message, unsigned.
fn world_notice(
    replies: &[Reply<rs_admin_api::Outcome>],
    say: impl FnOnce(&str, &str) -> String,
) -> Option<String> {
    let mut took = Vec::new();
    let mut refused = Vec::new();
    for reply in replies {
        match &reply.result {
            Outcome::Answered { answer } if answer.ok => took.push(reply.world),
            Outcome::Answered { answer } => refused.push((reply.world, answer.message.as_str())),
            Outcome::Failed { error } => refused.push((reply.world, error.as_str())),
        }
    }
    if took.is_empty() {
        return None;
    }
    let verb = if took.len() == 1 { "is" } else { "are" };
    let mut message = say(&format!("**{}**", worlds(&took)), verb);
    for (world, why) in refused {
        message.push_str(&format!("\n-# World {world} didn't: {}", escape(why)));
    }
    Some(message)
}

async fn send_all(client: Client, url: String, mut messages: Receiver<String>) {
    while let Some(content) = messages.recv().await {
        // Nobody gets pinged, whatever a reason or broadcast says.
        let body = json!({ "content": content, "allowed_mentions": { "parse": [] } }).to_string();
        for attempt in 0..2 {
            let sent = client
                .post(&url)
                .header(CONTENT_TYPE, "application/json")
                .body(body.clone())
                .send()
                .await;
            match sent {
                Ok(r) if r.status().is_success() => break,
                Ok(r) if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS && attempt == 0 => {
                    let wait = r
                        .headers()
                        .get(RETRY_AFTER)
                        .and_then(|v| v.to_str().ok()?.parse::<f64>().ok())
                        .map_or(Duration::from_secs(2), Duration::from_secs_f64);
                    tokio::time::sleep(wait.min(MAX_RETRY_WAIT)).await;
                }
                Ok(r) => {
                    warn!("Discord refused a world notice: {}", r.status());
                    break;
                }
                // The URL holds the webhook's token, so it stays out of the log.
                Err(e) => {
                    warn!(
                        "Couldn't post a world notice to Discord: {}",
                        e.without_url()
                    );
                    break;
                }
            }
        }
    }
}

/// "World 1", "Worlds 1 and 2", "Worlds 1, 2 and 3".
pub fn worlds(ids: &[u16]) -> String {
    match ids {
        [] => "No worlds".to_owned(),
        [one] => format!("World {one}"),
        [rest @ .., last] => {
            let rest: Vec<String> = rest.iter().map(u16::to_string).collect();
            format!("Worlds {} and {last}", rest.join(", "))
        }
    }
}

/// When something `secs` away happens: "now", "in 5 minutes", "in 1 hour
/// 30 minutes".
pub fn from_now(secs: u32) -> String {
    if secs == 0 {
        return "now".to_owned();
    }
    format!("in {}", span(secs))
}

/// `secs` in words: "45 seconds", "5 minutes", "1 hour 30 minutes".
pub fn span(secs: u32) -> String {
    let parts = [
        (secs / 3600, "hour"),
        (secs / 60 % 60, "minute"),
        (secs % 60, "second"),
    ];
    let words: Vec<String> = parts
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|&(n, unit)| format!("{n} {unit}{}", if n == 1 { "" } else { "s" }))
        .collect();
    words.join(" ")
}

/// Staff-typed `text` as Discord will show it: on one line, with nothing
/// read as formatting.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_control() {
            out.push(' ');
            continue;
        }
        if "\\*_~`|>#[]()<".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worlds_read_as_a_list() {
        assert_eq!(worlds(&[1]), "World 1");
        assert_eq!(worlds(&[1, 2]), "Worlds 1 and 2");
        assert_eq!(worlds(&[1, 2, 3]), "Worlds 1, 2 and 3");
    }

    #[test]
    fn spans_read_as_words() {
        assert_eq!(from_now(0), "now");
        assert_eq!(from_now(300), "in 5 minutes");
        assert_eq!(span(1), "1 second");
        assert_eq!(span(5400), "1 hour 30 minutes");
        assert_eq!(span(3661), "1 hour 1 minute 1 second");
    }

    fn reply(world: u16, ok: bool) -> Reply<rs_admin_api::Outcome> {
        let answer = if ok {
            rs_admin_api::Outcome::ok("Done.")
        } else {
            rs_admin_api::Outcome::refused("It's already running.")
        };
        Reply {
            node: world as u8,
            world,
            result: Outcome::Answered { answer },
        }
    }

    #[test]
    fn one_message_covers_every_world() {
        let replies = [reply(1, true), reply(2, true), reply(3, false)];
        let notice = world_notice(&replies, |worlds, are| {
            format!("{worlds} {are} starting up")
        });
        assert_eq!(
            notice.as_deref(),
            Some("**Worlds 1 and 2** are starting up\n-# World 3 didn't: It's already running.")
        );
        let one = world_notice(&replies[..1], |worlds, are| format!("{worlds} {are} up"));
        assert_eq!(one.as_deref(), Some("**World 1** is up"));
        assert!(world_notice(&replies[2..], |_, _| String::new()).is_none());
    }

    #[test]
    fn staff_text_is_shown_as_typed() {
        assert_eq!(escape("**hi**\n<@1>"), "\\*\\*hi\\*\\* \\<@1\\>");
    }
}
