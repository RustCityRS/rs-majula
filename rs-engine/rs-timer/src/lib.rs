use rs_vm::state::{ScriptArgument, TimedScript, TimerPriority};

/// A player's timers, one per script ID, kept in the order they were first set
/// like the reference engine's `Map`. Normal and soft timers share the list;
/// the timer phase walks it once per priority.
pub struct ScriptTimer {
    timers: Vec<Slot>,
    next_seq: u64,
}

struct Slot {
    seq: u64,
    timer: TimedScript,
}

impl ScriptTimer {
    /// Creates an empty [`ScriptTimer`].
    ///
    /// # Call Stack
    ///
    /// **Called by:** `EntityState::new` (`rs-entity/src/state.rs`)
    pub fn new() -> Self {
        Self {
            timers: Vec::new(),
            next_seq: 0,
        }
    }

    /// Sets the timer for `script_id`. An existing timer for that script is
    /// replaced where it stands, as `Map.set` does; otherwise the timer goes
    /// on the end.
    ///
    /// # Arguments
    ///
    /// * `priority` - Whether the timer is normal or soft.
    /// * `script_id` - The script the timer runs.
    /// * `interval` - Number of ticks between firings.
    /// * `clock` - The game tick at which the timer was set (used to compute readiness).
    /// * `args` - Optional typed arguments passed to the script on each firing.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptPlayer::settimer` (`rs-engine/src/engine.rs`)
    pub fn add(
        &mut self,
        priority: TimerPriority,
        script_id: i32,
        interval: u16,
        clock: u32,
        args: Option<Vec<ScriptArgument>>,
    ) {
        let timer = TimedScript {
            clock,
            args,
            script_id,
            interval,
            priority,
        };
        match self
            .timers
            .iter_mut()
            .find(|slot| slot.timer.script_id == script_id)
        {
            Some(slot) => slot.timer = timer,
            None => {
                let seq = self.next_seq;
                self.next_seq += 1;
                self.timers.push(Slot { seq, timer });
            }
        }
    }

    /// Removes the timer for `script_id`, normal or soft, if there is one.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptPlayer::cleartimer` (`rs-engine/src/engine.rs`)
    pub fn remove(&mut self, script_id: i32) {
        self.timers.retain(|slot| slot.timer.script_id != script_id);
    }

    /// Returns the timer for `script_id`, if there is one.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptPlayer::gettimer` (`rs-engine/src/engine.rs`)
    pub fn get(&self, script_id: i32) -> Option<&TimedScript> {
        self.timers
            .iter()
            .find(|slot| slot.timer.script_id == script_id)
            .map(|slot| &slot.timer)
    }

    /// The number of timers set.
    pub fn len(&self) -> usize {
        self.timers.len()
    }

    /// Whether no timers are set.
    pub fn is_empty(&self) -> bool {
        self.timers.is_empty()
    }

    /// Steps a walk over the timers that tolerates scripts setting and
    /// clearing timers between steps, like iterating a JS `Map`. Start with
    /// `cursor` at 0; each call returns the next timer in order and moves the
    /// cursor past it. Timers set during the walk are reached, cleared ones
    /// are skipped, and a timer set again keeps its place, so it is not
    /// visited twice.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::process_timers` (`rs-engine/src/phases/player.rs`)
    pub fn next_from(&mut self, cursor: &mut u64) -> Option<&mut TimedScript> {
        let at = self.timers.partition_point(|slot| slot.seq < *cursor);
        let slot = self.timers.get_mut(at)?;
        *cursor = slot.seq + 1;
        Some(&mut slot.timer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(t: &mut ScriptTimer) -> Vec<i32> {
        let mut cursor = 0;
        let mut ids = Vec::new();
        while let Some(timer) = t.next_from(&mut cursor) {
            ids.push(timer.script_id);
        }
        ids
    }

    #[test]
    fn add_keeps_insertion_order() {
        let mut t = ScriptTimer::new();
        for id in [30, 10, 20] {
            t.add(TimerPriority::Normal, id, 5, 0, None);
        }
        assert_eq!(ids(&mut t), [30, 10, 20]);
    }

    #[test]
    fn add_again_replaces_in_place() {
        let mut t = ScriptTimer::new();
        t.add(TimerPriority::Normal, 1, 10, 100, None);
        t.add(TimerPriority::Normal, 2, 10, 100, None);
        t.add(TimerPriority::Soft, 1, 20, 200, None);
        assert_eq!(ids(&mut t), [1, 2]);
        let timer = t.get(1).unwrap();
        assert_eq!(
            (timer.priority, timer.interval, timer.clock),
            (TimerPriority::Soft, 20, 200)
        );
    }

    #[test]
    fn remove_clears_either_priority() {
        let mut t = ScriptTimer::new();
        t.add(TimerPriority::Normal, 1, 10, 100, None);
        t.add(TimerPriority::Soft, 2, 10, 100, None);
        t.remove(2);
        t.remove(999);
        assert_eq!(ids(&mut t), [1]);
        assert!(t.get(2).is_none());
    }

    /// What timer scripts do mid-walk: the walk reaches timers set after the
    /// current one, skips cleared ones, and does not revisit one set again.
    #[test]
    fn walk_follows_changes_made_between_steps() {
        let mut t = ScriptTimer::new();
        for id in [1, 2, 3, 4] {
            t.add(TimerPriority::Normal, id, 5, 0, None);
        }
        let mut cursor = 0;
        let mut seen = Vec::new();
        while let Some(timer) = t.next_from(&mut cursor) {
            let id = timer.script_id;
            seen.push(id);
            if id == 2 {
                t.remove(1); // already visited
                t.remove(3); // not yet visited
                t.add(TimerPriority::Normal, 2, 9, 0, None); // set again
                t.add(TimerPriority::Normal, 5, 5, 0, None); // new
            }
        }
        assert_eq!(seen, [1, 2, 4, 5]);
        assert_eq!(ids(&mut t), [2, 4, 5]);
    }

    /// Clearing a timer and setting it again moves it to the end, as deleting
    /// and re-setting a `Map` key does, so the walk reaches it again.
    #[test]
    fn walk_reaches_a_timer_cleared_and_set_again() {
        let mut t = ScriptTimer::new();
        t.add(TimerPriority::Normal, 1, 5, 0, None);
        t.add(TimerPriority::Normal, 2, 5, 0, None);
        let mut cursor = 0;
        let mut seen = Vec::new();
        while let Some(timer) = t.next_from(&mut cursor) {
            let id = timer.script_id;
            seen.push(id);
            if seen.len() == 1 {
                t.remove(1);
                t.add(TimerPriority::Normal, 1, 5, 0, None);
            }
        }
        assert_eq!(seen, [1, 2, 1]);
    }
}
