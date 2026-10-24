use crate::{ConnectionGuards, REVISION};
use axum::Router;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use rs_engine::TickStats;
use rs_engine::counters::{
    self, AUTO_BAN_TRIGGERS, CHAT_KINDS, COUNTERS, LOG_KINDS, MOD_ACTIONS, REPORT_REASONS,
    WEALTH_TYPES,
};
use std::fmt::Write as _;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing::{info, warn};

/// The game tick, and the length a tick may take before it overruns.
const TICK_BUDGET_MS: f64 = 600.0;

/// Upper bounds (seconds) of the tick duration histogram's buckets.
const TICK_BUCKETS: [f64; 12] = [
    0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 0.6, 1.0, 2.5,
];

/// A phase's time (ms) in a tick.
type PhaseTime = fn(&TickStats) -> f64;

/// Each engine phase's label and its time in a tick.
const PHASES: [(&str, PhaseTime); 14] = [
    ("world", |s| s.world),
    ("input", |s| s.input),
    ("npcs", |s| s.npcs),
    ("players", |s| s.players),
    ("logouts", |s| s.logouts),
    ("autosave", |s| s.autosave),
    ("logins", |s| s.logins),
    ("ether", |s| s.ether),
    ("saves", |s| s.saves),
    ("zones", |s| s.zones),
    ("info", |s| s.info),
    ("out", |s| s.out),
    ("cleanup", |s| s.cleanup),
    ("other", |s| {
        s.total_ms
            - (s.world
                + s.input
                + s.npcs
                + s.players
                + s.logouts
                + s.autosave
                + s.logins
                + s.ether
                + s.saves
                + s.zones
                + s.info
                + s.out
                + s.cleanup)
    }),
];

/// Totals over every tick since startup, plus the latest tick's population.
#[derive(Clone, Copy, Default)]
struct TickTotals {
    ticks: u64,
    overruns: u64,
    buckets: [u64; TICK_BUCKETS.len()],
    seconds: f64,
    phase_seconds: [f64; PHASES.len()],
    players: usize,
    npcs: usize,
}

impl TickTotals {
    fn record(&mut self, s: &TickStats) {
        let secs = s.total_ms / 1000.0;
        self.ticks += 1;
        if s.total_ms > TICK_BUDGET_MS {
            self.overruns += 1;
        }
        for (count, &le) in self.buckets.iter_mut().zip(&TICK_BUCKETS) {
            if secs <= le {
                *count += 1;
            }
        }
        self.seconds += secs;
        for (total, (_, ms)) in self.phase_seconds.iter_mut().zip(PHASES) {
            *total += ms(s).max(0.0) / 1000.0;
        }
        self.players = s.player_count;
        self.npcs = s.npc_count;
    }
}

struct Metrics {
    ticks: Mutex<TickTotals>,
    guards: ConnectionGuards,
    members: bool,
    started_secs: u64,
    sys: Mutex<System>,
    pid: Option<Pid>,
}

pub async fn serve(
    host: String,
    port: u16,
    members: bool,
    stats: watch::Receiver<TickStats>,
    guards: ConnectionGuards,
) {
    let listener = match TcpListener::bind((host.as_str(), port)).await {
        Ok(l) => l,
        Err(e) => {
            warn!("Failed to bind metrics on: {}:{} - {}", host, port, e);
            return;
        }
    };
    info!("Metrics available at: http://{}:{}/metrics", host, port);

    let metrics = Arc::new(Metrics {
        ticks: Mutex::default(),
        guards,
        members,
        started_secs: unix_secs(),
        sys: Mutex::new(System::new()),
        pid: sysinfo::get_current_pid().ok(),
    });
    tokio::spawn(record_ticks(stats, metrics.clone()));

    let app = Router::new()
        .route("/metrics", get(render))
        .with_state(metrics);
    if let Err(e) = axum::serve(listener, app).await {
        warn!("Metrics server stopped: {}", e);
    }
}

/// Folds every published tick into the totals. `stats` must be subscribed
/// before the engine starts. The tick loop skips missed ticks rather than
/// bursting, so no tick is published before the last one was seen.
async fn record_ticks(mut stats: watch::Receiver<TickStats>, metrics: Arc<Metrics>) {
    while stats.changed().await.is_ok() {
        let tick = stats.borrow_and_update().clone();
        if let Ok(mut totals) = metrics.ticks.lock() {
            totals.record(&tick);
        }
    }
}

async fn render(State(m): State<Arc<Metrics>>) -> impl IntoResponse {
    let mut out = String::with_capacity(4096);
    m.write(&mut out);
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        out,
    )
}

impl Metrics {
    fn write(&self, out: &mut String) {
        let t = self.ticks.lock().map(|t| *t).unwrap_or_default();

        family(out, "rs_build_info", "gauge", "The running build.");
        let _ = writeln!(
            out,
            "rs_build_info{{rev=\"{REVISION}\",version=\"{}\",members=\"{}\"}} 1",
            env!("CARGO_PKG_VERSION"),
            self.members
        );

        family(out, "rs_ticks_total", "counter", "Game ticks run.");
        let _ = writeln!(out, "rs_ticks_total {}", t.ticks);
        family(
            out,
            "rs_tick_overruns_total",
            "counter",
            "Ticks that took longer than the 600ms tick.",
        );
        let _ = writeln!(out, "rs_tick_overruns_total {}", t.overruns);

        family(
            out,
            "rs_tick_duration_seconds",
            "histogram",
            "How long each tick's work took.",
        );
        for (le, count) in TICK_BUCKETS.iter().zip(t.buckets) {
            let _ = writeln!(
                out,
                "rs_tick_duration_seconds_bucket{{le=\"{le}\"}} {count}"
            );
        }
        let _ = writeln!(
            out,
            "rs_tick_duration_seconds_bucket{{le=\"+Inf\"}} {}",
            t.ticks
        );
        let _ = writeln!(out, "rs_tick_duration_seconds_sum {}", t.seconds);
        let _ = writeln!(out, "rs_tick_duration_seconds_count {}", t.ticks);

        family(
            out,
            "rs_tick_phase_seconds_total",
            "counter",
            "Time spent in each engine phase, summed over every tick.",
        );
        for ((phase, _), secs) in PHASES.iter().zip(t.phase_seconds) {
            let _ = writeln!(
                out,
                "rs_tick_phase_seconds_total{{phase=\"{phase}\"}} {secs}"
            );
        }

        family(out, "rs_players", "gauge", "Players in the world.");
        let _ = writeln!(out, "rs_players {}", t.players);
        family(out, "rs_npcs", "gauge", "NPCs in the world.");
        let _ = writeln!(out, "rs_npcs {}", t.npcs);

        let guards = [
            ("game", &self.guards.game),
            ("js5", &self.guards.js5),
            ("http", &self.guards.http),
            ("jaggrab", &self.guards.jaggrab),
        ];
        family(
            out,
            "rs_connections",
            "gauge",
            "Open connections by listener.",
        );
        for (kind, guard) in guards {
            let _ = writeln!(out, "rs_connections{{kind=\"{kind}\"}} {}", guard.active());
        }
        family(
            out,
            "rs_connections_accepted_total",
            "counter",
            "Connections accepted by listener.",
        );
        for (kind, guard) in guards {
            let _ = writeln!(
                out,
                "rs_connections_accepted_total{{kind=\"{kind}\"}} {}",
                guard.served()
            );
        }

        write_counters(out);

        family(
            out,
            "process_start_time_seconds",
            "gauge",
            "Start time of the process since the Unix epoch.",
        );
        let _ = writeln!(out, "process_start_time_seconds {}", self.started_secs);
        if let Some(rss) = self.resident_bytes() {
            family(
                out,
                "process_resident_memory_bytes",
                "gauge",
                "Resident memory size.",
            );
            let _ = writeln!(out, "process_resident_memory_bytes {rss}");
        }
    }

    fn resident_bytes(&self) -> Option<u64> {
        let pid = self.pid?;
        let mut sys = self.sys.lock().ok()?;
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );
        sys.process(pid).map(|p| p.memory())
    }
}

/// The engine's game event totals (see `rs_engine::counters`).
fn write_counters(out: &mut String) {
    let c = &COUNTERS;
    labelled(
        out,
        "rs_chat_messages_total",
        "Chat messages sent.",
        "kind",
        &CHAT_KINDS,
        &c.chat,
    );
    labelled(
        out,
        "rs_chat_muted_total",
        "Chat messages muted players tried to send.",
        "kind",
        &CHAT_KINDS,
        &c.chat_muted,
    );
    labelled(
        out,
        "rs_auto_bans_total",
        "Automated bans for what only a modified client sends.",
        "trigger",
        &AUTO_BAN_TRIGGERS,
        &c.auto_bans,
    );
    labelled(
        out,
        "rs_reports_total",
        "Abuse reports, by rule broken.",
        "reason",
        &REPORT_REASONS,
        &c.reports,
    );
    family(
        out,
        "rs_mutes_total",
        "counter",
        "Mutes moderators made from abuse reports.",
    );
    let _ = writeln!(out, "rs_mutes_total {}", counters::get(&c.mutes));
    labelled(
        out,
        "rs_mod_actions_total",
        "Staff moderation commands that took, by action.",
        "action",
        &MOD_ACTIONS,
        &c.mod_actions,
    );
    labelled(
        out,
        "rs_wealth_coins_total",
        "Coin value of wealth events, by type.",
        "type",
        &WEALTH_TYPES,
        &c.wealth_coins,
    );
    labelled(
        out,
        "rs_wealth_items_total",
        "Items moved by wealth events, by type.",
        "type",
        &WEALTH_TYPES,
        &c.wealth_items,
    );
    labelled(
        out,
        "rs_db_log_rows_total",
        "Log rows written to the database, by kind.",
        "kind",
        &LOG_KINDS,
        &c.log_rows,
    );
    family(
        out,
        "rs_db_log_batches_total",
        "counter",
        "Log batches written to the database (one per tick with logs).",
    );
    let _ = writeln!(
        out,
        "rs_db_log_batches_total {}",
        counters::get(&c.log_batches)
    );
    family(
        out,
        "rs_db_log_batches_dropped_total",
        "counter",
        "Log batches lost to a failed write or an offline database.",
    );
    let _ = writeln!(
        out,
        "rs_db_log_batches_dropped_total {}",
        counters::get(&c.log_batches_dropped)
    );
    family(
        out,
        "rs_db_log_rows_dropped_total",
        "counter",
        "Log rows in the dropped batches.",
    );
    let _ = writeln!(
        out,
        "rs_db_log_rows_dropped_total {}",
        counters::get(&c.log_rows_dropped)
    );
    for (name, help, counter) in [
        (
            "rs_db_replay_chunks_total",
            "Session recording chunks stored.",
            &c.replay_chunks,
        ),
        (
            "rs_db_replay_bytes_total",
            "Compressed bytes of the session recording chunks stored.",
            &c.replay_bytes,
        ),
        (
            "rs_db_replay_chunks_dropped_total",
            "Session recording chunks lost to a full queue, a failed write or an offline database.",
            &c.replay_chunks_dropped,
        ),
        (
            "rs_script_errors_total",
            "Scripts aborted by an error.",
            &c.script_errors,
        ),
    ] {
        family(out, name, "counter", help);
        let _ = writeln!(out, "{name} {}", counters::get(counter));
    }
}

/// A counter family with one series per label value.
fn labelled(
    out: &mut String,
    name: &str,
    help: &str,
    label: &str,
    values: &[&str],
    totals: &[AtomicU64],
) {
    family(out, name, "counter", help);
    for (value, total) in values.iter().zip(totals) {
        let _ = writeln!(
            out,
            "{name}{{{label}=\"{value}\"}} {}",
            counters::get(total)
        );
    }
}

fn family(out: &mut String, name: &str, kind: &str, help: &str) {
    let _ = writeln!(out, "# HELP {name} {help}\n# TYPE {name} {kind}");
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_fill_cumulative_buckets_and_phases() {
        let mut t = TickTotals::default();
        t.record(&TickStats {
            total_ms: 4.0,
            npcs: 3.0,
            player_count: 7,
            ..Default::default()
        });
        t.record(&TickStats {
            total_ms: 700.0,
            npcs: 650.0,
            ..Default::default()
        });

        assert_eq!((t.ticks, t.overruns, t.players), (2, 1, 0));
        // 4ms lands in every bucket from 5ms up; 700ms only from 1s up.
        assert_eq!(t.buckets, [0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2]);
        let phase = |name| t.phase_seconds[PHASES.iter().position(|p| p.0 == name).unwrap()];
        assert!((phase("npcs") - 0.653).abs() < 1e-9);
        assert!((phase("other") - 0.051).abs() < 1e-9);
    }

    #[test]
    fn counters_render_a_fixed_set_of_series() {
        COUNTERS.wealth(7, 3, 120);
        // Unknown types and negative amounts stay out.
        COUNTERS.wealth(4, 1, 1);
        COUNTERS.wealth(8, -1, -5);

        let mut out = String::new();
        write_counters(&mut out);
        let series: Vec<&str> = out.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(
            series.len(),
            2 + 2 + 3 + 12 + 1 + 5 + 6 + 6 + 3 + 1 + 1 + 1 + 3 + 1
        );
        assert!(series.contains(&"rs_db_replay_chunks_dropped_total 0"));
        assert!(series.contains(&"rs_wealth_coins_total{type=\"shop_sell\"} 120"));
        assert!(series.contains(&"rs_wealth_items_total{type=\"shop_sell\"} 3"));
        assert!(series.contains(&"rs_wealth_coins_total{type=\"low_alch\"} 0"));
        assert!(series.contains(&"rs_reports_total{reason=\"macroing\"} 0"));
    }
}
