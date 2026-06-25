pub mod log_layer;

use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use log_layer::{LogBuffer, LogLine};
use mpsc::{UnboundedReceiver, unbounded_channel};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Sparkline, Wrap};
use rs_engine::TickStats;
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};
use tokio::sync::{mpsc, watch};
use watch::channel;

const BANNER: &str = r"
██████╗ ██╗   ██╗███████╗████████╗ ██████╗██╗████████╗██╗   ██╗
██╔══██╗██║   ██║██╔════╝╚══██╔══╝██╔════╝██║╚══██╔══╝╚██╗ ██╔╝
██████╔╝██║   ██║███████╗   ██║   ██║     ██║   ██║    ╚████╔╝
██╔══██╗██║   ██║╚════██║   ██║   ██║     ██║   ██║     ╚██╔╝
██║  ██║╚██████╔╝███████║   ██║   ╚██████╗██║   ██║      ██║
╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝    ╚═════╝╚═╝   ╚═╝      ╚═╝
                   rev {REV} · rust edition";

/// How many ticks of history to keep for the sparklines.
const HISTORY: usize = 240; // 240 × 600ms ≈ 2.4 min

const PET_FRAMES: [[&str; 3]; 4] = [
    [" \\[^_^]/ ", "  |==|   ", "  /  \\   "],
    ["  [>.<]~>", "  |==|   ", "  /  \\   "],
    ["  [OwO]  ", " \\|==|/  ", "  /  \\   "],
    ["  [^o^]  ", "  |==|~  ", "  /  \\   "],
];

const ROASTS: &[&str] = &[
    "Lost City is still decoding login packets from last Tuesday",
    "Our tick budget: sub-millisecond. Lost City's: a prayer and a Thread.sleep",
    "Lost City's PlayerInfo makes spaghetti look like clean architecture",
    "Lost City launched in 2024 and STILL doesn't have basic features working",
    "Lost City's ISAAC cipher is just rot13 with extra steps",
    "We have more NPCs loaded than Lost City has lines of working code",
    "Lost City's zones are so broken even the GE clerk filed a bug report",
    "Per-packet structs btw. Lost City still uses a match the size of Varrock",
    "Lost City couldn't find itself. That's why it's lost.",
    "Lost City's tick loop is just while(true) { Thread.sleep(600) }",
    "Our cache builds from source. Lost City's builds from vibes and prayers",
    "Lost City still thinks XTEA keys are a premium tea blend",
    "Our server compiles RuneScript. Lost City compiles excuses.",
    "If Lost City was a quest it'd be One Small Favour. Never ending.",
    "Even the Wise Old Man wouldn't touch Lost City's codebase",
    "Our collision works. Lost City's bridges are still impassable walls.",
    "We load 5,792 NPC spawns. Lost City loads 5,792 panics.",
    "Lost City's login flow has more states than a US road trip",
    "Rust btw. Lost City is mass world 302 at Falador.",
    "Our RuneScript compiler has 100% parity. Lost City has 100% bugs.",
    "Lost City is the Duel Arena of RSPS. Everyone knows it's rigged.",
    "Zero wire-format in rs-engine. Lost City's engine IS the wire format.",
    "Lost City is bronze armor. Nobody wears it past Tutorial Island.",
    "We have typed opcodes. Lost City has typed 'help' into Stack Overflow.",
    "Lost City tried to ::noclip but their server doesn't have a command handler",
    "They called it Lost City because the players are lost too",
    "Lost City? More like Lost Cause.",
    "Our server has more uptime than Lost City has features",
];

/// Channels the TUI hands back to the rest of the app.
#[derive()]
pub struct TuiHandles {
    pub stats_tx: watch::Sender<TickStats>,
    pub trigger_rx: UnboundedReceiver<()>,
}

pub struct TuiSinks {
    pub stats_rx: watch::Receiver<TickStats>,
    pub reload_tx: mpsc::UnboundedSender<()>,
}

pub fn make_channels() -> (TuiHandles, TuiSinks) {
    let (stats_tx, stats_rx) = channel(TickStats::default());
    let (reload_tx, trigger_rx) = unbounded_channel();
    (
        TuiHandles {
            stats_tx,
            trigger_rx,
        },
        TuiSinks {
            stats_rx,
            reload_tx,
        },
    )
}

/// Ephemeral status flash (e.g. "starting server…") shown briefly in the
/// hint bar then auto-cleared so it doesn't lie about state.
struct Flash {
    text: String,
    until: Instant,
}

struct App {
    log_buf: LogBuffer,
    sinks: TuiSinks,
    search: String,
    search_focused: bool,
    scroll_back: usize,
    started_at: Instant,
    flash: Option<Flash>,

    // Sparkline history.
    tick_ms_history: VecDeque<u64>,
    last_seen_clock: u32,
    mem_mb_history: VecDeque<u64>,
    last_mem_poll: Instant,
    mem_peak_mb: u64,
    mem_rss_bytes: u64,
    mem_rss_delta: i64,
    sys: System,
    pid: Pid,
    roast_index: usize,
    pet_frame: usize,
    last_roast_change: Instant,
    last_frame_change: Instant,
    roast_cycle: usize,
    commentary: Option<(String, String)>,
    last_seen_log_len: usize,
}

impl App {
    fn new(log_buf: LogBuffer, sinks: TuiSinks) -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().with_memory()),
        );
        let pid = Pid::from_u32(std::process::id());
        Self {
            log_buf,
            sinks,
            search: String::new(),
            search_focused: false,
            scroll_back: 0,
            started_at: Instant::now(),
            flash: None,
            tick_ms_history: VecDeque::with_capacity(HISTORY),
            last_seen_clock: 0,
            mem_mb_history: VecDeque::with_capacity(HISTORY),
            last_mem_poll: Instant::now() - Duration::from_secs(10),
            mem_peak_mb: 0,
            mem_rss_bytes: 0,
            mem_rss_delta: 0,
            sys,
            pid,
            roast_index: 0,
            pet_frame: 0,
            last_roast_change: Instant::now(),
            last_frame_change: Instant::now(),
            roast_cycle: 0,
            commentary: None,
            last_seen_log_len: 0,
        }
    }

    fn flash_for(&mut self, text: impl Into<String>, ms: u64) {
        self.flash = Some(Flash {
            text: text.into(),
            until: Instant::now() + Duration::from_millis(ms),
        });
    }

    fn poll_metrics(&mut self) {
        // Tick history: only push when the tick number advances, to keep the
        // sparkline as a true per-tick series (not per-frame).
        let stats = self.sinks.stats_rx.borrow().clone();
        if stats.clock != self.last_seen_clock && stats.clock > 0 {
            self.last_seen_clock = stats.clock;
            if self.tick_ms_history.len() >= HISTORY {
                self.tick_ms_history.pop_front();
            }
            // Cap at 600ms (the full tick budget) so spikes don't crush the
            // chart's vertical scale.
            self.tick_ms_history
                .push_back(stats.total_ms.min(600.0) as u64);
        }

        // Memory: poll at most every 1s - sysinfo refresh isn't free.
        if self.last_mem_poll.elapsed() >= Duration::from_secs(1) {
            self.last_mem_poll = Instant::now();
            self.sys.refresh_processes_specifics(
                sysinfo::ProcessesToUpdate::Some(&[self.pid]),
                true,
                ProcessRefreshKind::nothing().with_memory(),
            );
            if let Some(p) = self.sys.process(self.pid) {
                let bytes = p.memory();
                let mb = bytes / (1024 * 1024);
                if mb > self.mem_peak_mb {
                    self.mem_peak_mb = mb;
                }
                if self.mem_mb_history.len() >= HISTORY {
                    self.mem_mb_history.pop_front();
                }
                self.mem_mb_history.push_back(mb);
                if self.mem_rss_bytes > 0 {
                    self.mem_rss_delta = bytes as i64 - self.mem_rss_bytes as i64;
                }
                self.mem_rss_bytes = bytes;
            }
        }

        // Auto-clear stale flash messages.
        if let Some(f) = &self.flash {
            if Instant::now() >= f.until {
                self.flash = None;
            }
        }

        if self.last_frame_change.elapsed() >= Duration::from_millis(800) {
            self.last_frame_change = Instant::now();
            self.pet_frame = (self.pet_frame + 1) % PET_FRAMES.len();
        }
        if self.last_roast_change.elapsed() >= Duration::from_secs(6) {
            self.last_roast_change = Instant::now();
            self.roast_cycle += 1;

            if self.roast_cycle % 4 == 3 {
                // Commentary break - react to a recent log line.
                self.commentary = self.pick_log_reaction();
            } else {
                self.commentary = None;
                self.roast_index = (self.roast_index + 1) % ROASTS.len();
            }
        }
    }

    fn pick_log_reaction(&mut self) -> Option<(String, String)> {
        let buf = self.log_buf.lock().ok()?;
        let len = buf.len();
        if len == 0 || len == self.last_seen_log_len {
            return None;
        }

        // Scan the newest logs we haven't reacted to yet, prefer
        // errors > warnings > plain info for maximum entertainment.
        let scan_start = if self.last_seen_log_len < len {
            self.last_seen_log_len
        } else {
            len.saturating_sub(20)
        };
        self.last_seen_log_len = len;

        let mut best: Option<&LogLine> = None;
        let mut best_priority: u8 = 0;
        for line in buf.range(scan_start..) {
            let p = match line.level {
                tracing::Level::ERROR => 4,
                tracing::Level::WARN => 3,
                tracing::Level::INFO => 2,
                _ => 1,
            };
            if p >= best_priority {
                best_priority = p;
                best = Some(line);
            }
        }

        let line = best?;
        let snippet = if line.message.len() > 60 {
            format!("{}...", &line.message[..57])
        } else {
            line.message.clone()
        };
        let comment = react_to_log(line);
        Some((snippet, comment))
    }
}

fn react_to_log(line: &LogLine) -> String {
    let msg = line.message.to_lowercase();

    let comment = if msg.contains("connection from") {
        "a brave adventurer approaches... or a bot. probably a bot."
    } else if msg.contains("login ok") || msg.contains("login") && msg.contains("ok") {
        "welcome home hero. Lost City could never serve a login this clean"
    } else if msg.contains("js5") {
        "serving cache data at lightspeed. you're welcome, client."
    } else if msg.contains("pack_all") || msg.contains("packing") {
        "packing content... Lost City is still unpacking their excuses"
    } else if msg.contains("script") && (msg.contains("loaded") || msg.contains("compiled")) {
        "RuneScript goes brrr. 284 scripts and counting."
    } else if msg.contains("hot-reload") || msg.contains("reload") {
        "hot-reload? in THIS economy? we're living in the future"
    } else if msg.contains("spawn") || msg.contains("npc") {
        "more NPCs spawned than Lost City has daily active players"
    } else if msg.contains("zone") {
        "zones doing zone things. functioning, unlike some servers..."
    } else if msg.contains("engine") && msg.contains("tick") {
        "engine humming along. you love to see it."
    } else if msg.contains("accepting connections") || msg.contains("ready") {
        "server's OPEN. Lost City's still in closed alpha... mentally"
    } else if msg.contains("rsa") || msg.contains("key") {
        "crypto loaded. military grade. Lost City uses ROT13."
    } else if msg.contains("cache") && msg.contains("build") {
        "cache built from source like civilized developers"
    } else if msg.contains("xtea") {
        "XTEA keys secured. it's not a tea brand, Lost City."
    } else if msg.contains("closed") || msg.contains("dropped") || msg.contains("disconnect") {
        "another one bites the dust. connection go bye bye"
    } else if msg.contains("error") || msg.contains("failed") {
        "yikes... but still better than Lost City on a GOOD day"
    } else if msg.contains("warn") {
        "hmm that's concerning. filing under 'not my problem yet'"
    } else {
        match line.level {
            tracing::Level::ERROR => "oof. we don't talk about this one.",
            tracing::Level::WARN => "noted. adding to the 'investigate later' pile",
            tracing::Level::INFO => "business as usual. flawless. chef's kiss.",
            tracing::Level::DEBUG => "debugging like professionals. println? never heard of her.",
            tracing::Level::TRACE => "we trace everything. Lost City traces nothing.",
        }
    };

    comment.to_string()
}

pub async fn run(log_buf: LogBuffer, sinks: TuiSinks) -> Result<()> {
    let tui_thread_id = std::thread::current().id();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().id() == tui_thread_id {
            let _ = disable_raw_mode();
            let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen);
            eprintln!("{info}");
            std::process::exit(1);
        } else {
            tracing::error!(
                "PANIC on {:?}: {info}",
                std::thread::current().name().unwrap_or("unnamed")
            );
        }
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, App::new(log_buf, sinks)).await;

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> Result<()>
where
    <B as ratatui::backend::Backend>::Error: Send,
    <B as ratatui::backend::Backend>::Error: Sync,
    <B as ratatui::backend::Backend>::Error: 'static,
{
    let frame_rate = Duration::from_millis(50);
    let mut last_tick = Instant::now();

    loop {
        app.poll_metrics();
        terminal.draw(|f| ui(f, &app))?;

        let timeout = frame_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if handle_key(&mut app, key.code, key.modifiers)? {
                    return Ok(());
                }
            }
        }

        if last_tick.elapsed() >= frame_rate {
            last_tick = Instant::now();
        }
    }
}

/// Returns `Ok(true)` to quit, `Ok(false)` otherwise.
fn handle_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> Result<bool> {
    if app.search_focused {
        match code {
            KeyCode::Esc => {
                app.search.clear();
                app.search_focused = false;
            }
            KeyCode::Enter => app.search_focused = false,
            KeyCode::Backspace => {
                app.search.pop();
            }
            KeyCode::Char(c) => app.search.push(c),
            _ => {}
        }
        return Ok(false);
    }

    if mods.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('c')) {
        return Ok(true);
    }

    match code {
        KeyCode::Char('q') => return Ok(true),
        #[cfg(debug_assertions)]
        KeyCode::Char('c') => {
            let _ = app.sinks.reload_tx.send(());
            app.flash_for("reload signal sent", 3000);
        }
        KeyCode::Char('/') => {
            app.search_focused = true;
        }
        KeyCode::Esc => {
            app.search.clear();
        }
        KeyCode::PageUp => app.scroll_back = app.scroll_back.saturating_add(10),
        KeyCode::PageDown => app.scroll_back = app.scroll_back.saturating_sub(10),
        KeyCode::Up => app.scroll_back = app.scroll_back.saturating_add(1),
        KeyCode::Down => app.scroll_back = app.scroll_back.saturating_sub(1),
        KeyCode::End => app.scroll_back = 0,
        _ => {}
    }
    Ok(false)
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // banner
            Constraint::Length(6), // stats + timings + graphs
            Constraint::Length(5), // Sir Roastalot
            Constraint::Min(5),    // log
            Constraint::Length(3), // search bar
            Constraint::Length(1), // hints
        ])
        .split(f.area());

    draw_banner(f, chunks[0]);
    draw_stats_row(f, chunks[1], app);
    draw_tamagotchi(f, chunks[2], app);
    draw_log(f, chunks[3], app);
    draw_search(f, chunks[4], app);
    draw_hints(f, chunks[5], app);
}

fn draw_banner(f: &mut ratatui::Frame, area: Rect) {
    let banner = BANNER.replace("{REV}", crate::REVISION);
    let para = Paragraph::new(banner).style(Style::default().fg(Color::Magenta));
    f.render_widget(para, area);
}

fn draw_stats_row(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(38),
            Constraint::Length(24),
            Constraint::Length(24),
            Constraint::Min(20),
        ])
        .split(area);

    draw_stats_column(f, cols[0], app);
    draw_timings_left(f, cols[1], app);
    draw_timings_right(f, cols[2], app);
    draw_graphs(f, cols[3], app);
}

fn draw_stats_column(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let stats = app.sinks.stats_rx.borrow().clone();

    let status = if stats.clock == 0 {
        Span::styled("loading", Style::default().fg(Color::Yellow))
    } else {
        Span::styled(
            "RUNNING",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    };

    let pct = (stats.total_ms / 600.0) * 100.0;
    let uptime = if stats.clock > 0 {
        let s = app.started_at.elapsed().as_secs();
        format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        "-".into()
    };

    let label = |s: &str| Span::styled(format!("{s:<9}"), Style::default().fg(Color::DarkGray));

    let mem_cur = app.mem_rss_bytes / (1024 * 1024);
    let mem_delta = format_bytes_delta(app.mem_rss_delta);

    let lines = vec![
        Line::from(vec![label("Status:"), status]),
        Line::from(vec![
            label("Clock:"),
            Span::raw(format!(
                "{}  {:.2}ms/600ms ({:.1}%)",
                stats.clock, stats.total_ms, pct
            )),
        ]),
        Line::from(vec![
            label("Players:"),
            Span::raw(format!("{}", stats.player_count)),
        ]),
        Line::from(vec![
            label("NPCs:"),
            Span::raw(format!("{}", stats.npc_count)),
        ]),
        Line::from(vec![label("Up-time:"), Span::raw(uptime)]),
        Line::from(vec![
            label("Memory:"),
            Span::raw(format!("{}mb ({})", mem_cur, mem_delta)),
        ]),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::RIGHT)
            .style(Style::default().fg(Color::White)),
    );
    f.render_widget(para, area);
}

fn draw_tamagotchi(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let stats = app.sinks.stats_rx.borrow().clone();
    let title = if stats.clock > 0 {
        format!(
            " Sir Roastalot (lvl 126) -- roasting Lost City since tick {} ",
            stats.clock
        )
    } else {
        " Sir Roastalot (lvl 126) -- warming up... ".to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 20 || inner.height < 2 {
        return;
    }

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(11), Constraint::Min(20)])
        .split(inner);

    let frame = &PET_FRAMES[app.pet_frame];
    let pet_lines: Vec<Line> = frame
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(Color::Yellow))))
        .collect();
    f.render_widget(Paragraph::new(pet_lines), cols[0]);

    if stats.clock > 0 {
        let lines = if let Some((snippet, comment)) = &app.commentary {
            vec![
                Line::from(vec![
                    Span::styled("log: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("\"{}\"", snippet), Style::default().fg(Color::Cyan)),
                ]),
                Line::from(Span::styled(
                    format!("  ^ {}", comment),
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::ITALIC),
                )),
            ]
        } else {
            vec![
                Line::from(Span::styled(
                    format!("\"{}\"", ROASTS[app.roast_index]),
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::ITALIC),
                )),
                Line::from(Span::styled(
                    format!(
                        "  tick {} | {:.2}ms | {} npcs loaded -- stay lost, Lost City",
                        stats.clock, stats.total_ms, stats.npc_count
                    ),
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        };
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), cols[1]);
    } else {
        f.render_widget(
            Paragraph::new("warming up the roast chamber...")
                .style(Style::default().fg(Color::DarkGray)),
            cols[1],
        );
    }
}

fn phase_line(name: &str, ms: f64) -> Line<'static> {
    let color = if ms > 100.0 {
        Color::Red
    } else if ms > 10.0 {
        Color::Yellow
    } else {
        Color::White
    };
    Line::from(vec![
        Span::styled(format!(" {name:<9}"), Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{ms:.2}ms"), Style::default().fg(color)),
    ])
}

fn draw_timings_left(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let stats = app.sinks.stats_rx.borrow().clone();

    let lines = vec![
        phase_line("input:", stats.input),
        phase_line("npcs:", stats.npcs),
        phase_line("players:", stats.players),
        phase_line("logouts:", stats.logouts),
        phase_line("logins:", stats.logins),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::LEFT | Borders::RIGHT)
            .title(" Tick phases ")
            .style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(para, area);
}

fn draw_timings_right(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let stats = app.sinks.stats_rx.borrow().clone();

    let lines = vec![
        Line::from(""),
        phase_line("zones:", stats.zones),
        phase_line("info:", stats.info),
        phase_line("out:", stats.out),
        phase_line("cleanup:", stats.cleanup),
        phase_line("world:", stats.world),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::RIGHT)
            .style(Style::default().fg(Color::White)),
    );
    f.render_widget(para, area);
}

fn draw_graphs(f: &mut ratatui::Frame, area: Rect, app: &App) {
    // Memory (RSS) graph.
    let data: Vec<u64> = app.mem_mb_history.iter().copied().collect();
    let cur = data.last().copied().unwrap_or(0);
    let delta = format_bytes_delta(app.mem_rss_delta);
    let title = Line::from(vec![
        Span::styled(" Memory · ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}MB", cur), Style::default().fg(Color::White)),
        Span::styled(" · Peak ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}MB", app.mem_peak_mb),
            Style::default().fg(Color::White),
        ),
        Span::styled(" · ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{} ", delta), Style::default().fg(Color::White)),
    ]);
    let sparkline = Sparkline::default()
        .block(Block::default().borders(Borders::TOP).title(title))
        .data(&data)
        .max(app.mem_peak_mb.max(1))
        .bar_set(symbols::bar::NINE_LEVELS)
        .style(Style::default().fg(Color::LightGreen));
    f.render_widget(sparkline, area);
}

fn draw_log(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let q = app.search.to_lowercase();
    let lines: Vec<LogLine> = if let Ok(buf) = app.log_buf.lock() {
        if q.is_empty() {
            buf.iter().cloned().collect()
        } else {
            buf.iter()
                .filter(|l| {
                    l.message.to_lowercase().contains(&q) || l.target.to_lowercase().contains(&q)
                })
                .cloned()
                .collect()
        }
    } else {
        Vec::new()
    };

    let height = area.height.saturating_sub(2) as usize;
    let max_back = lines.len().saturating_sub(height);
    let scroll = app.scroll_back.min(max_back);
    let end = lines.len().saturating_sub(scroll);
    let start = end.saturating_sub(height);

    let rendered: Vec<Line> = lines[start..end]
        .iter()
        .map(|l| format_line(l, &q))
        .collect();

    let title = if app.search.is_empty() {
        format!(" log  ({} lines) ", lines.len())
    } else {
        format!(" log  filter='{}' ({} match) ", app.search, lines.len())
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let para = Paragraph::new(rendered).block(block);
    f.render_widget(para, area);
}

fn format_line(line: &LogLine, query: &str) -> Line<'static> {
    let level_style = match line.level {
        tracing::Level::ERROR => Style::default().fg(Color::Red),
        tracing::Level::WARN => Style::default().fg(Color::Yellow),
        tracing::Level::INFO => Style::default().fg(Color::Green),
        tracing::Level::DEBUG => Style::default().fg(Color::Blue),
        tracing::Level::TRACE => Style::default().fg(Color::DarkGray),
    };
    let mut spans = vec![
        Span::styled(format!("{:<5} ", line.level), level_style),
        Span::styled(
            format!("{:<28} ", trim_target(&line.target)),
            Style::default().fg(Color::DarkGray),
        ),
    ];
    if query.is_empty() {
        spans.push(Span::raw(line.message.clone()));
    } else {
        spans.extend(highlight(&line.message, query));
    }
    Line::from(spans)
}

fn trim_target(t: &str) -> String {
    if t.len() <= 28 {
        t.to_string()
    } else {
        format!("…{}", &t[t.len() - 27..])
    }
}

fn highlight(text: &str, query: &str) -> Vec<Span<'static>> {
    let lowered = text.to_lowercase();
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if let Some(pos) = lowered[i..].find(query) {
            let abs = i + pos;
            if abs > i {
                out.push(Span::raw(text[i..abs].to_string()));
            }
            let end = abs + query.len();
            out.push(Span::styled(
                text[abs..end].to_string(),
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ));
            i = end;
        } else {
            out.push(Span::raw(text[i..].to_string()));
            break;
        }
    }
    out
}

fn draw_search(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let style = if app.search_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let cursor = if app.search_focused { "_" } else { "" };
    let body = format!("/{}{}", app.search, cursor);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" search (press / to focus, Esc to clear) ")
        .style(style);
    let para = Paragraph::new(body).block(block);
    f.render_widget(para, area);
}

fn draw_hints(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let mut hints: Vec<Span<'static>> = Vec::new();
    if cfg!(debug_assertions) {
        hints.push(key_hint("c", "compile+reload assets & scripts"));
        hints.push(Span::raw("  "));
    }
    hints.push(key_hint("/", "search"));
    hints.push(Span::raw("  "));
    hints.push(key_hint("PgUp/PgDn", "scroll"));
    hints.push(Span::raw("  "));
    hints.push(key_hint("End", "tail"));
    hints.push(Span::raw("  "));
    hints.push(key_hint("q", "quit"));
    if let Some(flash) = &app.flash {
        hints.push(Span::raw("    "));
        hints.push(Span::styled(
            flash.text.clone(),
            Style::default().fg(Color::Cyan),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(hints)), area);
}

fn format_bytes_delta(delta: i64) -> String {
    let sign = if delta >= 0 { "+" } else { "-" };
    let abs = delta.unsigned_abs();
    if abs < 1024 {
        format!("{sign}{}b", abs)
    } else if abs < 1024 * 1024 {
        format!("{sign}{:.1}kb", abs as f64 / 1024.0)
    } else {
        format!("{sign}{:.1}mb", abs as f64 / (1024.0 * 1024.0))
    }
}

fn key_hint(key: &str, label: &str) -> Span<'static> {
    Span::styled(format!("[{key}] {label}"), Style::default().fg(Color::Gray))
}

#[allow(dead_code)]
fn _phantom(_: Arc<()>) {}
