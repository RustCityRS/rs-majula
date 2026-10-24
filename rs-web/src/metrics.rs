use crate::app::App;
use axum::Router;
use axum::extract::{MatchedPath, Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tracing::{info, warn};

/// Upper bounds (seconds) of the request duration histogram's buckets.
const DURATION_BUCKETS: [f64; 10] = [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0];

/// The route label of requests no explicit route matched: static files,
/// template pages and 404s.
const FALLBACK_ROUTE: &str = "fallback";

#[derive(Clone, Copy, Default)]
struct Durations {
    count: u64,
    buckets: [u64; DURATION_BUCKETS.len()],
    seconds: f64,
}

impl Durations {
    fn record(&mut self, secs: f64) {
        self.count += 1;
        for (count, &le) in self.buckets.iter_mut().zip(&DURATION_BUCKETS) {
            if secs <= le {
                *count += 1;
            }
        }
        self.seconds += secs;
    }
}

#[derive(Default)]
struct Requests {
    /// By (route, status code).
    counts: BTreeMap<(String, u16), u64>,
    /// By route.
    durations: BTreeMap<String, Durations>,
}

pub struct Metrics {
    started_secs: u64,
    requests: Mutex<Requests>,
    pub registrations: AtomicU64,
    pub registrations_rate_limited: AtomicU64,
    pub registrations_challenge_failed: AtomicU64,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            started_secs: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            requests: Mutex::default(),
            registrations: AtomicU64::new(0),
            registrations_rate_limited: AtomicU64::new(0),
            registrations_challenge_failed: AtomicU64::new(0),
        }
    }
}

impl Metrics {
    fn record(&self, route: &str, status: u16, secs: f64) {
        let Ok(mut requests) = self.requests.lock() else {
            return;
        };
        *requests
            .counts
            .entry((route.to_string(), status))
            .or_default() += 1;
        requests
            .durations
            .entry(route.to_string())
            .or_default()
            .record(secs);
    }

    fn write(&self, out: &mut String) {
        family(out, "rs_web_build_info", "gauge", "The running build.");
        let _ = writeln!(
            out,
            "rs_web_build_info{{version=\"{}\"}} 1",
            env!("CARGO_PKG_VERSION")
        );

        if let Ok(requests) = self.requests.lock() {
            family(
                out,
                "rs_web_requests_total",
                "counter",
                "HTTP requests answered, by route and status.",
            );
            for ((route, status), count) in &requests.counts {
                let _ = writeln!(
                    out,
                    "rs_web_requests_total{{route=\"{route}\",status=\"{status}\"}} {count}"
                );
            }

            family(
                out,
                "rs_web_request_duration_seconds",
                "histogram",
                "How long each request took to answer, by route.",
            );
            for (route, d) in &requests.durations {
                for (le, count) in DURATION_BUCKETS.iter().zip(d.buckets) {
                    let _ = writeln!(
                        out,
                        "rs_web_request_duration_seconds_bucket{{route=\"{route}\",le=\"{le}\"}} {count}"
                    );
                }
                let _ = writeln!(
                    out,
                    "rs_web_request_duration_seconds_bucket{{route=\"{route}\",le=\"+Inf\"}} {}",
                    d.count
                );
                let _ = writeln!(
                    out,
                    "rs_web_request_duration_seconds_sum{{route=\"{route}\"}} {}",
                    d.seconds
                );
                let _ = writeln!(
                    out,
                    "rs_web_request_duration_seconds_count{{route=\"{route}\"}} {}",
                    d.count
                );
            }
        }

        family(
            out,
            "rs_web_registrations_total",
            "counter",
            "Players registered through /create.",
        );
        let _ = writeln!(
            out,
            "rs_web_registrations_total {}",
            self.registrations.load(Ordering::Relaxed)
        );
        family(
            out,
            "rs_web_registrations_rate_limited_total",
            "counter",
            "/create submissions refused by the per-IP registration limit.",
        );
        let _ = writeln!(
            out,
            "rs_web_registrations_rate_limited_total {}",
            self.registrations_rate_limited.load(Ordering::Relaxed)
        );
        family(
            out,
            "rs_web_registrations_challenge_failed_total",
            "counter",
            "/create submissions refused by the Turnstile check.",
        );
        let _ = writeln!(
            out,
            "rs_web_registrations_challenge_failed_total {}",
            self.registrations_challenge_failed.load(Ordering::Relaxed)
        );

        family(
            out,
            "process_start_time_seconds",
            "gauge",
            "Start time of the process since the Unix epoch.",
        );
        let _ = writeln!(out, "process_start_time_seconds {}", self.started_secs);
    }
}

/// Middleware counting and timing every request. Added with `Router::layer`,
/// so explicit routes see their [`MatchedPath`]; everything else is
/// [`FALLBACK_ROUTE`], which keeps the label set small whatever URLs are hit.
pub async fn track(State(app): State<Arc<App>>, req: Request, next: Next) -> Response {
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map_or(FALLBACK_ROUTE.to_string(), |p| p.as_str().to_string());
    let start = Instant::now();
    let response = next.run(req).await;
    app.metrics.record(
        &route,
        response.status().as_u16(),
        start.elapsed().as_secs_f64(),
    );
    response
}

pub async fn serve(host: String, port: u16, app: Arc<App>) {
    let listener = match TcpListener::bind((host.as_str(), port)).await {
        Ok(l) => l,
        Err(e) => {
            warn!("Failed to bind metrics on: {}:{} - {}", host, port, e);
            return;
        }
    };
    info!("Metrics available at: http://{}:{}/metrics", host, port);
    let router = Router::new().route("/metrics", get(render)).with_state(app);
    if let Err(e) = axum::serve(listener, router).await {
        warn!("Metrics server stopped: {}", e);
    }
}

async fn render(State(app): State<Arc<App>>) -> impl IntoResponse {
    let mut out = String::with_capacity(4096);
    app.metrics.write(&mut out);
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        out,
    )
}

fn family(out: &mut String, name: &str, kind: &str, help: &str) {
    let _ = writeln!(out, "# HELP {name} {help}\n# TYPE {name} {kind}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_fill_cumulative_buckets_per_route() {
        let m = Metrics::default();
        m.record("/hiscores", 200, 0.004);
        m.record("/hiscores", 200, 0.3);
        m.record(FALLBACK_ROUTE, 404, 0.001);

        let requests = m.requests.lock().unwrap();
        assert_eq!(requests.counts[&("/hiscores".to_string(), 200)], 2);
        assert_eq!(requests.counts[&(FALLBACK_ROUTE.to_string(), 404)], 1);
        let d = requests.durations["/hiscores"];
        // 4ms lands in every bucket; 300ms only from 500ms up.
        assert_eq!(d.buckets, [1, 1, 1, 1, 1, 1, 2, 2, 2, 2]);
        assert_eq!(d.count, 2);
    }

    #[test]
    fn exposition_has_every_family() {
        let m = Metrics::default();
        m.record("/create", 200, 0.01);
        m.registrations.fetch_add(1, Ordering::Relaxed);
        let mut out = String::new();
        m.write(&mut out);
        for line in [
            "rs_web_requests_total{route=\"/create\",status=\"200\"} 1",
            "rs_web_request_duration_seconds_count{route=\"/create\"} 1",
            "rs_web_registrations_total 1",
            "rs_web_registrations_rate_limited_total 0",
            "rs_web_registrations_challenge_failed_total 0",
        ] {
            assert!(out.contains(line), "missing {line}");
        }
    }
}
