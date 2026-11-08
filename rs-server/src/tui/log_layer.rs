use std::collections::VecDeque;
use std::fmt::{Debug, Write as _};
use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

const MAX_LOG_LINES: usize = 5000;

#[derive(Debug, Clone)]
pub struct LogLine {
    pub level: tracing::Level,
    pub target: String,
    pub message: String,
}

pub type LogBuffer = Arc<Mutex<VecDeque<LogLine>>>;

pub fn new_buffer() -> LogBuffer {
    Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_LINES)))
}

pub struct TuiLogLayer {
    buf: LogBuffer,
}

impl TuiLogLayer {
    pub fn new(buf: LogBuffer) -> Self {
        Self { buf }
    }
}

/// Targets that should never reach the TUI viewport even when the global
/// filter accepts them - typically because the TUI shows the same data in
/// a structured way (live stats panel) and the raw line would just clutter
/// the log. The events still flow to other layers (e.g. the file log).
const SUPPRESSED_TARGETS: &[&str] = &["tick_stats"];

impl<S: Subscriber> Layer<S> for TuiLogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        if SUPPRESSED_TARGETS.contains(&meta.target()) {
            return;
        }
        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);

        let line = LogLine {
            level: *meta.level(),
            target: meta.target().to_string(),
            message: visitor.0,
        };

        if let Ok(mut q) = self.buf.lock() {
            if q.len() >= MAX_LOG_LINES {
                q.pop_front();
            }
            q.push_back(line);
        }
    }
}

struct MessageVisitor(String);

impl Visit for MessageVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.0.push_str(value);
        } else {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            let _ = write!(self.0, "{}={}", field.name(), value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            let _ = write!(self.0, "{value:?}");
        } else {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            let _ = write!(self.0, "{}={:?}", field.name(), value);
        }
    }
}
