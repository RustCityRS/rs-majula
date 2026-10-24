use crate::users::now_ms;
use crate::worlds::{Outcome, Reply};
use reqwest::Client;
use reqwest::header::{CONTENT_TYPE, RETRY_AFTER};
use rs_admin_api::iso_time;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::warn;

/// How long Discord gets to take a message.
const TIMEOUT: Duration = Duration::from_secs(10);
/// Messages waiting to go out; more than this and the newest are dropped.
const QUEUE: usize = 64;
/// The longest a rate limit is waited out before one retry.
const MAX_RETRY_WAIT: Duration = Duration::from_secs(30);

/// The cards' edge colours, GitHub's.
pub const GREEN: u32 = 0x2da44e;
pub const RED: u32 = 0xcf222e;
pub const ORANGE: u32 = 0xbf8700;
pub const BLUE: u32 = 0x0969da;
pub const PURPLE: u32 = 0x8250df;

/// Discord's limits on a card's parts, in characters.
const MAX_TITLE: usize = 256;
const MAX_TEXT: usize = 4096;
const MAX_FIELD: usize = 1024;

/// One message: a card (a Discord embed) with a coloured edge, a title, and
/// fields under it, like GitHub's.
pub struct Card {
    title: String,
    colour: u32,
    text: Option<String>,
    fields: Vec<(String, String, bool)>,
}

impl Card {
    pub fn new(title: impl Into<String>, colour: u32) -> Self {
        Self {
            title: title.into(),
            colour,
            text: None,
            fields: Vec::new(),
        }
    }

    /// The text under the title.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// A short field, side by side with the others.
    pub fn field(mut self, name: &str, value: impl Into<String>) -> Self {
        self.fields.push((name.to_owned(), value.into(), true));
        self
    }

    /// A field across the card.
    pub fn wide(mut self, name: &str, value: impl Into<String>) -> Self {
        self.fields.push((name.to_owned(), value.into(), false));
        self
    }

    /// The embed, with `by` at the top and the time at the bottom.
    fn embed(&self, by: &str) -> Value {
        let fields: Vec<Value> = self
            .fields
            .iter()
            .map(|(name, value, inline)| {
                json!({ "name": name, "value": clip(value, MAX_FIELD), "inline": inline })
            })
            .collect();
        let mut embed = json!({
            "author": { "name": by },
            "title": clip(&self.title, MAX_TITLE),
            "color": self.colour,
            "fields": fields,
            "timestamp": iso_time(now_ms() as i64),
        });
        if let Some(text) = &self.text {
            embed["description"] = Value::from(clip(text, MAX_TEXT));
        }
        embed
    }
}

/// A countdown's card, `title` and when ("Reboot in 5 minutes"), with
/// how far apart the `worlds` go and why.
pub fn countdown(
    title: &str,
    colour: u32,
    seconds: u32,
    stagger_secs: u32,
    worlds: usize,
    reason: Option<&str>,
) -> Card {
    let mut card = Card::new(format!("{title} {}", from_now(seconds)), colour);
    if stagger_secs > 0 && worlds > 1 {
        card = card.field("Stagger", format!("{} apart", span(stagger_secs)));
    }
    if let Some(reason) = reason {
        card = card.wide("Reason", escape(reason));
    }
    card
}

/// Tells a Discord channel, through its webhook, what staff did to the
/// worlds: one card per action, however many worlds it covered.
pub struct Discord {
    queue: Option<Sender<Value>>,
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

    /// Posts `card` for what `by` did, naming the worlds that took it first
    /// and any that didn't at the bottom. Nothing is posted if no world
    /// took it.
    pub fn world_action(&self, by: &str, replies: &[Reply<rs_admin_api::Outcome>], card: Card) {
        if self.queue.is_some()
            && let Some(card) = world_card(replies, card)
        {
            self.post(by, &card);
        }
    }

    /// Posts `card`, from `by`.
    pub fn post(&self, by: &str, card: &Card) {
        let Some(queue) = &self.queue else {
            return;
        };
        if queue.try_send(card.embed(by)).is_err() {
            warn!("Dropped a Discord world notice: too many waiting");
        }
    }
}

/// [`Discord::world_action`]'s card.
fn world_card(replies: &[Reply<rs_admin_api::Outcome>], mut card: Card) -> Option<Card> {
    let mut took = Vec::new();
    let mut refused = Vec::new();
    for reply in replies {
        match &reply.result {
            Outcome::Answered { answer } if answer.ok => took.push(reply.world),
            Outcome::Answered { answer } => refused.push(format!(
                "World {}: {}",
                reply.world,
                escape(&answer.message)
            )),
            Outcome::Failed { error } => {
                refused.push(format!("World {}: {}", reply.world, escape(error)))
            }
        }
    }
    if took.is_empty() {
        return None;
    }
    let name = if took.len() == 1 { "World" } else { "Worlds" };
    card.fields.insert(0, (name.to_owned(), list(&took), true));
    if !refused.is_empty() {
        card = card.wide("Didn't take it", refused.join("\n"));
    }
    Some(card)
}

async fn send_all(client: Client, url: String, mut messages: Receiver<Value>) {
    while let Some(embed) = messages.recv().await {
        // Nobody gets pinged, whatever a reason or broadcast says.
        let body = json!({ "embeds": [embed], "allowed_mentions": { "parse": [] } }).to_string();
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

/// "1", "1 and 2", "1, 2 and 3".
fn list(ids: &[u16]) -> String {
    match ids {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => {
            let rest: Vec<String> = rest.iter().map(u16::to_string).collect();
            format!("{} and {last}", rest.join(", "))
        }
    }
}

/// `text`, cut to `max` characters.
fn clip(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((cut, _)) => text[..cut].to_owned(),
        None => text.to_owned(),
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
        assert_eq!(list(&[1]), "1");
        assert_eq!(list(&[1, 2]), "1 and 2");
        assert_eq!(list(&[1, 2, 3]), "1, 2 and 3");
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
    fn one_card_covers_every_world() {
        let replies = [reply(1, true), reply(2, true), reply(3, false)];
        let card = world_card(&replies, Card::new("Starting up", GREEN)).unwrap();
        let embed = card.embed("Jordan");
        assert_eq!(embed["author"]["name"], "Jordan");
        assert_eq!(embed["title"], "Starting up");
        assert_eq!(embed["color"], GREEN);
        assert_eq!(embed["fields"][0]["name"], "Worlds");
        assert_eq!(embed["fields"][0]["value"], "1 and 2");
        assert_eq!(embed["fields"][1]["name"], "Didn't take it");
        assert_eq!(
            embed["fields"][1]["value"],
            "World 3: It's already running."
        );
        assert_eq!(embed["fields"][1]["inline"], false);

        let one = world_card(&replies[..1], Card::new("Up", GREEN)).unwrap();
        assert_eq!(one.embed("Jordan")["fields"][0]["name"], "World");
        assert!(world_card(&replies[2..], Card::new("Up", GREEN)).is_none());
    }

    #[test]
    fn long_parts_are_cut_to_fit() {
        assert_eq!(clip("abcdef", 3), "abc");
        assert_eq!(clip("\u{e9}\u{20ac}", 1), "\u{e9}");
        assert_eq!(clip("ab", 3), "ab");
    }

    #[test]
    fn staff_text_is_shown_as_typed() {
        assert_eq!(escape("**hi**\n<@1>"), "\\*\\*hi\\*\\* \\<@1\\>");
    }
}
