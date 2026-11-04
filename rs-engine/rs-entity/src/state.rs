use rs_queue::ScriptQueue;
use rs_timer::ScriptTimer;
use rs_vm::state::ScriptState;

/// Shared state for script execution and delay management on an entity (player or NPC).
///
/// Tracks whether the entity is currently delayed (blocked from acting until a future
/// tick), whether it is protected from new interactions, the currently executing script,
/// and the pending script queues.
pub struct EntityState {
    pub delayed: bool,
    pub delayed_until: u32,
    pub protect: bool,
    pub active_script: Option<Box<ScriptState>>,
    pub queues: ScriptQueue,
    pub timers: Box<ScriptTimer>,
}

impl EntityState {
    /// Creates a new `EntityState` with all fields in their default idle state.
    ///
    /// # Returns
    /// An `EntityState` that is not delayed, not protected, with no active script
    /// and empty script queues.
    pub fn new() -> Self {
        Self {
            delayed: false,
            delayed_until: 0,
            protect: false,
            active_script: None,
            queues: ScriptQueue::new(),
            timers: Box::new(ScriptTimer::new()),
        }
    }

    /// Checks whether a pending delay has expired and clears it if so.
    ///
    /// # Arguments
    /// * `clock` - The current engine tick.
    ///
    /// # Side Effects
    /// * Sets `self.delayed` to `false` when `clock >= self.delayed_until`.
    pub fn check_delay(&mut self, clock: u32) {
        if self.delayed && clock >= self.delayed_until {
            self.delayed = false;
        }
    }
}
