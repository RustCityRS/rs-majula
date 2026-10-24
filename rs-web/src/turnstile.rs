use anyhow::{Result, bail};
use reqwest::Client;
use serde::Deserialize;
use std::net::IpAddr;
use std::time::Duration;

const VERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

/// The longest token Cloudflare issues.
const MAX_TOKEN_LEN: usize = 2048;

pub struct Turnstile {
    site_key: String,
    secret: String,
    http: Client,
}

#[derive(Deserialize)]
struct Verdict {
    success: bool,
    #[serde(default, rename = "error-codes")]
    error_codes: Vec<String>,
}

impl Turnstile {
    pub fn new(site_key: String, secret: String) -> Result<Self> {
        let http = Client::builder().timeout(Duration::from_secs(10)).build()?;
        Ok(Self {
            site_key,
            secret,
            http,
        })
    }

    /// The public key the page's widget is drawn with.
    pub fn site_key(&self) -> &str {
        &self.site_key
    }

    /// Whether `token`, from the form, is a challenge passed from `ip` that
    /// hasn't been used before. `Err` if Cloudflare couldn't be asked.
    pub async fn passed(&self, token: &str, ip: IpAddr) -> Result<bool> {
        if token.is_empty() || token.len() > MAX_TOKEN_LEN {
            return Ok(false);
        }
        let ip = ip.to_string();
        let verdict: Verdict = self
            .http
            .post(VERIFY_URL)
            .form(&[
                ("secret", self.secret.as_str()),
                ("response", token),
                ("remoteip", &ip),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        // A bad secret fails every registration: say so rather than blame
        // the visitor.
        if let Some(code) = verdict
            .error_codes
            .iter()
            .find(|c| matches!(c.as_str(), "missing-input-secret" | "invalid-input-secret"))
        {
            bail!("Turnstile refused our secret ({code})");
        }
        Ok(verdict.success)
    }
}
