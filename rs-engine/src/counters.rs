use rs_db::ModAction;
use std::sync::atomic::{AtomicU64, Ordering};

/// Chat kinds, indexing [`Counters::chat`] and [`Counters::chat_muted`].
pub const CHAT_KINDS: [&str; 2] = ["public", "private"];
pub const CHAT_PUBLIC: usize = 0;
pub const CHAT_PRIVATE: usize = 1;

/// What only a modified client sends, earning an automated ban. Indexes
/// [`Counters::auto_bans`].
#[derive(Clone, Copy)]
pub enum AutoBan {
    ReportReason,
    ReportOffender,
    PrivateName,
}

pub const AUTO_BAN_TRIGGERS: [&str; 3] = ["report_reason", "report_offender", "private_name"];

/// The report screen's 12 rules, by the reason index the client sends.
pub const REPORT_REASONS: [&str; 12] = [
    "offensive_language",
    "item_scamming",
    "password_scamming",
    "bug_abuse",
    "staff_impersonation",
    "account_sharing",
    "macroing",
    "multiple_logging",
    "encouraging_rule_breaking",
    "customer_support_misuse",
    "advertising",
    "real_world_trading",
];

/// The scripts' `^wealth_*` types, from `^wealth_pickup` (5) on.
pub const WEALTH_TYPES: [&str; 6] = [
    "pickup",
    "shop_buy",
    "shop_sell",
    "low_alch",
    "high_alch",
    "partyroom",
];
const FIRST_WEALTH_TYPE: i32 = 5;

/// Staff moderation commands that took, indexing [`Counters::mod_actions`].
pub const MOD_ACTIONS: [&str; 5] = ["mute", "unmute", "ban", "unban", "kick"];

/// Where `action` counts in [`Counters::mod_actions`].
pub const fn mod_action_index(action: ModAction) -> usize {
    match action {
        ModAction::Mute { .. } => 0,
        ModAction::Unmute => 1,
        ModAction::Ban { .. } => 2,
        ModAction::Unban => 3,
        ModAction::Kick => 4,
    }
}

/// The kinds of rows in a log batch, indexing [`Counters::log_rows`].
pub const LOG_KINDS: [&str; 3] = ["chat", "session", "wealth"];

pub struct Counters {
    pub chat: [AtomicU64; 2],
    pub chat_muted: [AtomicU64; 2],
    pub auto_bans: [AtomicU64; 3],
    pub reports: [AtomicU64; 12],
    pub mutes: AtomicU64,
    pub mod_actions: [AtomicU64; 5],
    pub wealth_coins: [AtomicU64; 6],
    pub wealth_items: [AtomicU64; 6],
    pub log_rows: [AtomicU64; 3],
    pub log_batches: AtomicU64,
    pub log_batches_dropped: AtomicU64,
    pub log_rows_dropped: AtomicU64,
    pub replay_chunks: AtomicU64,
    pub replay_bytes: AtomicU64,
    pub replay_chunks_dropped: AtomicU64,
    pub script_errors: AtomicU64,
}

pub static COUNTERS: Counters = Counters {
    chat: [const { AtomicU64::new(0) }; 2],
    chat_muted: [const { AtomicU64::new(0) }; 2],
    auto_bans: [const { AtomicU64::new(0) }; 3],
    reports: [const { AtomicU64::new(0) }; 12],
    mutes: AtomicU64::new(0),
    mod_actions: [const { AtomicU64::new(0) }; 5],
    wealth_coins: [const { AtomicU64::new(0) }; 6],
    wealth_items: [const { AtomicU64::new(0) }; 6],
    log_rows: [const { AtomicU64::new(0) }; 3],
    log_batches: AtomicU64::new(0),
    log_batches_dropped: AtomicU64::new(0),
    log_rows_dropped: AtomicU64::new(0),
    replay_chunks: AtomicU64::new(0),
    replay_bytes: AtomicU64::new(0),
    replay_chunks_dropped: AtomicU64::new(0),
    script_errors: AtomicU64::new(0),
};

/// Adds `n` to a total.
pub fn add(counter: &AtomicU64, n: u64) {
    counter.fetch_add(n, Ordering::Relaxed);
}

/// Reads a total.
pub fn get(counter: &AtomicU64) -> u64 {
    counter.load(Ordering::Relaxed)
}

impl Counters {
    /// Counts a script's `wealth_event`. Types outside [`WEALTH_TYPES`] and
    /// negative amounts are left out.
    pub fn wealth(&self, event_type: i32, count: i32, value: i32) {
        let Some(i) = usize::try_from(event_type - FIRST_WEALTH_TYPE)
            .ok()
            .filter(|&i| i < WEALTH_TYPES.len())
        else {
            return;
        };
        add(&self.wealth_items[i], count.max(0) as u64);
        add(&self.wealth_coins[i], value.max(0) as u64);
    }
}
