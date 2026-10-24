use rs_db::ScriptErrorLog;
use rs_vm::engine::ScriptFault;
use rustc_hash::FxHashMap;
use tracing::warn;

/// How often (in ticks) the errors gathered go to the database: about a
/// minute, so one that fires every tick is a row a minute.
pub const FLUSH_TICKS: u32 = 100;
/// The most different errors gathered between flushes. A broken script
/// that errs with a new message each time can't flood the database.
const MAX_PENDING: usize = 200;

/// Whom a script ran on.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Subject {
    Player(u64),
    Npc(u16),
    World,
}

/// Where an error happened, beyond the script.
pub struct Context {
    pub subject: Subject,
    pub npc: Option<String>,
    pub x: i16,
    pub z: i16,
    pub y: i16,
    pub replay_id: Option<i64>,
    pub replay_ms: Option<i32>,
}

#[derive(PartialEq, Eq, Hash)]
struct Key {
    script: String,
    line: i32,
    message: String,
    subject: Subject,
}

/// Script errors gathered since the last flush, one entry for each error
/// and whom it happened to.
#[derive(Default)]
pub struct ScriptErrors {
    pending: FxHashMap<Key, ScriptErrorLog>,
    dropped: u32,
}

impl ScriptErrors {
    /// Gathers `fault`, which happened `now_ms` (Unix ms).
    pub fn add(&mut self, fault: &ScriptFault, context: Context, now_ms: i64) {
        let key = Key {
            script: fault.script.clone(),
            line: fault.line,
            message: fault.message.clone(),
            subject: context.subject,
        };
        if let Some(log) = self.pending.get_mut(&key) {
            log.count = log.count.saturating_add(1);
            log.last_ms = now_ms;
            return;
        }
        if self.pending.len() >= MAX_PENDING {
            self.dropped += 1;
            return;
        }
        let name_hash = match context.subject {
            Subject::Player(user37) => Some(user37),
            _ => None,
        };
        self.pending.insert(
            key,
            ScriptErrorLog {
                first_ms: now_ms,
                last_ms: now_ms,
                count: 1,
                script: fault.script.clone(),
                file: fault.file.clone(),
                line: fault.line,
                message: fault.message.clone(),
                trace: fault.trace.join("\n"),
                name_hash,
                npc: context.npc,
                x: context.x,
                z: context.z,
                y: context.y,
                replay_id: context.replay_id,
                replay_ms: context.replay_ms,
            },
        );
    }

    /// What was gathered, oldest first.
    pub fn take(&mut self) -> Vec<ScriptErrorLog> {
        if self.dropped > 0 {
            warn!(
                "Left out {} script errors: over {MAX_PENDING} different ones this minute",
                self.dropped
            );
            self.dropped = 0;
        }
        let mut logs: Vec<ScriptErrorLog> = self.pending.drain().map(|(_, log)| log).collect();
        logs.sort_by_key(|log| log.first_ms);
        logs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fault(message: &str) -> ScriptFault {
        ScriptFault {
            script: "[opnpc1,man]".into(),
            file: "man.rs2".into(),
            line: 12,
            message: message.into(),
            trace: vec!["[proc,talk] - talk.rs2:4".into()],
            player: None,
            npc: None,
        }
    }

    fn on(subject: Subject) -> Context {
        Context {
            subject,
            npc: None,
            x: 3200,
            z: 3200,
            y: 0,
            replay_id: None,
            replay_ms: None,
        }
    }

    #[test]
    fn repeats_are_counted_per_subject() {
        let mut errors = ScriptErrors::default();
        errors.add(&fault("boom"), on(Subject::Player(1)), 1_000);
        errors.add(&fault("boom"), on(Subject::Player(1)), 2_000);
        errors.add(&fault("boom"), on(Subject::Player(2)), 3_000);
        errors.add(&fault("other"), on(Subject::World), 4_000);
        let logs = errors.take();
        assert_eq!(logs.len(), 3);
        assert_eq!(
            (logs[0].count, logs[0].first_ms, logs[0].last_ms),
            (2, 1_000, 2_000)
        );
        assert_eq!(logs[0].trace, "[proc,talk] - talk.rs2:4");
        assert_eq!(logs[1].name_hash, Some(2));
        assert_eq!(logs[2].name_hash, None);
        assert!(errors.take().is_empty());
    }

    #[test]
    fn a_flood_of_new_errors_is_capped() {
        let mut errors = ScriptErrors::default();
        for i in 0..MAX_PENDING + 50 {
            errors.add(&fault(&format!("boom {i}")), on(Subject::World), i as i64);
        }
        assert_eq!(errors.take().len(), MAX_PENDING);
    }
}
