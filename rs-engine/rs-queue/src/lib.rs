use rs_datastruct::LinkList;
use rs_vm::ScriptError;
use rs_vm::state::{QueuePriority, QueuedScript, ScriptArgument};

/// A triple-lane script queue that routes queued scripts by priority into
/// separate [`LinkList`]s. Normal/Strong/Long scripts go into `queue`,
/// Weak scripts go into `weak`, and Engine scripts go into `engine` (with
/// delay forced to 0). Soft priority is not supported and returns an error.
///
/// Each lane is iterated independently during the player/NPC phase tick,
/// with Strong scripts clearing the weak queue.
pub struct ScriptQueue {
    pub queue: LinkList<QueuedScript>,
    pub weak: LinkList<QueuedScript>,
    pub engine: LinkList<QueuedScript>,
}

impl ScriptQueue {
    /// Creates a new [`ScriptQueue`] with all three lanes empty.
    ///
    /// # Returns
    ///
    /// A [`ScriptQueue`] with empty `queue`, `weak`, and `engine` lists.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Player::new` (`rs-entity/src/player.rs`)
    /// **Calls:** [`LinkList::new`]
    pub fn new() -> Self {
        ScriptQueue {
            queue: LinkList::new(),
            weak: LinkList::new(),
            engine: LinkList::new(),
        }
    }

    /// Enqueues a script into the appropriate lane based on its priority.
    ///
    /// * `Normal`, `Strong`, `Long` -> appended to `queue`
    /// * `Engine` -> appended to `engine` with delay forced to 0
    /// * `Weak` -> appended to `weak`
    /// * `Soft` -> returns a [`ScriptError::Runtime`] error
    ///
    /// # Arguments
    ///
    /// * `priority` - Determines which lane the script is routed to.
    /// * `script_id` - The script identifier to execute when the delay expires.
    /// * `delay` - Number of ticks to wait before execution (overridden to 0 for Engine).
    /// * `args` - Optional typed arguments passed to the script on execution.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The script was successfully enqueued.
    /// * `Err(ScriptError::Runtime)` - The priority was `Soft`, which is not supported.
    ///
    /// # Side Effects
    ///
    /// * Appends a [`QueuedScript`] to the tail of the appropriate [`LinkList`] lane.
    /// * For `Engine` priority, the `delay` field is overwritten to 0 regardless of input.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptPlayer::queue` (`rs-engine/src/engine.rs`),
    ///   `ScriptNpc::queue` (`rs-engine/src/engine.rs`)
    /// **Calls:** [`LinkList::add_tail`]
    pub fn add(
        &mut self,
        priority: QueuePriority,
        script_id: i32,
        delay: u16,
        args: Option<Vec<ScriptArgument>>,
    ) -> Result<(), ScriptError> {
        let mut queued = QueuedScript {
            priority,
            script_id,
            delay,
            args,
        };
        match priority {
            QueuePriority::Normal | QueuePriority::Strong | QueuePriority::Long => {
                self.queue.add_tail(queued)
            }
            QueuePriority::Engine => {
                queued.delay = 0;
                self.engine.add_tail(queued);
            }
            QueuePriority::Weak => self.weak.add_tail(queued),
            QueuePriority::Soft => {
                return Err(ScriptError::Runtime(format!(
                    "Cannot queue soft script: {script_id}"
                )));
            }
        }
        Ok(())
    }

    /// Removes every queued script matching `script_id` from the normal and
    /// weak lanes. The engine lane is left untouched.
    ///
    /// # Arguments
    ///
    /// * `script_id` - The script identifier to unlink from the queues.
    pub fn remove_any(&mut self, script_id: i32) {
        Self::unlink_matching(&mut self.queue, script_id);
        Self::unlink_matching(&mut self.weak, script_id);
    }

    /// Unlinks every entry in a single queue lane whose script matches
    /// `script_id`, preserving the relative order of the entries left behind.
    ///
    /// The walk relies on the [`LinkList`] traversal cursor: [`LinkList::head`]
    /// and [`LinkList::next`] each return a node's handle *after* advancing the
    /// cursor to that node's successor. So by the time an entry is yielded the
    /// cursor already points past it, and [`LinkList::unlink`] only repatches
    /// the neighboring nodes' links (it never touches the cursor). Removing the
    /// just-yielded entry mid-walk is therefore safe, and every remaining entry
    /// is still visited exactly once.
    ///
    /// # Arguments
    ///
    /// * `list` - The queue lane to scan (the `queue` or `weak` list).
    /// * `script_id` - The script identifier whose entries should be removed.
    ///
    /// # Side Effects
    ///
    /// * Removes (and drops) every matching [`QueuedScript`] from `list`.
    /// * Leaves `list`'s traversal cursor at the sentinel (iteration exhausted).
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`ScriptQueue::remove_any`]
    /// **Calls:** [`LinkList::head`], [`LinkList::next`], [`LinkList::unlink`]
    fn unlink_matching(list: &mut LinkList<QueuedScript>, script_id: i32) {
        let mut handle = list.head();
        while let Some(idx) = handle {
            if list[idx].script_id == script_id {
                list.unlink(idx);
            }
            // The cursor already sits on `idx`'s successor (set when `idx` was
            // yielded), so this returns the next entry even though `idx` may
            // have just been unlinked.
            handle = list.next();
        }
    }

    /// Counts the queued scripts matching `script_id` across the normal and
    /// weak lanes. The engine lane is not counted.
    ///
    /// # Arguments
    ///
    /// * `script_id` - The script identifier to count.
    ///
    /// # Returns
    ///
    /// The number of matching entries in the `queue` and `weak` lanes.
    pub fn count_by_script(&self, script_id: i32) -> i32 {
        self.queue
            .iter()
            .chain(self.weak.iter())
            .filter(|q| q.script_id == script_id)
            .count() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_empty_queues() {
        let q = ScriptQueue::new();
        assert!(q.queue.is_empty());
        assert!(q.weak.is_empty());
        assert!(q.engine.is_empty());
    }

    #[test]
    fn add_normal_goes_to_queue() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Normal, 1, 5, None).unwrap();
        assert!(!q.queue.is_empty());
        assert!(q.weak.is_empty());
        assert!(q.engine.is_empty());
    }

    #[test]
    fn add_strong_goes_to_queue() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Strong, 2, 10, None).unwrap();
        assert!(!q.queue.is_empty());
    }

    #[test]
    fn add_long_goes_to_queue() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Long, 3, 15, None).unwrap();
        assert!(!q.queue.is_empty());
    }

    #[test]
    fn add_weak_goes_to_weak() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Weak, 4, 20, None).unwrap();
        assert!(q.queue.is_empty());
        assert!(!q.weak.is_empty());
        assert!(q.engine.is_empty());
    }

    #[test]
    fn add_engine_goes_to_engine_with_zero_delay() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Engine, 5, 999, None).unwrap();
        assert!(q.queue.is_empty());
        assert!(q.weak.is_empty());
        assert!(!q.engine.is_empty());

        let h = q.engine.head().unwrap();
        let script = q.engine.get(h);
        assert_eq!(script.delay, 0);
        assert_eq!(script.script_id, 5);
    }

    #[test]
    fn add_soft_returns_error() {
        let mut q = ScriptQueue::new();
        let result = q.add(QueuePriority::Soft, 6, 0, None);
        assert!(result.is_err());
        assert!(q.queue.is_empty());
        assert!(q.weak.is_empty());
        assert!(q.engine.is_empty());
    }

    #[test]
    fn add_preserves_script_id_and_delay() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Normal, 42, 100, None).unwrap();
        let h = q.queue.head().unwrap();
        let script = q.queue.get(h);
        assert_eq!(script.script_id, 42);
        assert_eq!(script.delay, 100);
        assert_eq!(script.priority, QueuePriority::Normal);
    }

    #[test]
    fn add_with_args() {
        let mut q = ScriptQueue::new();
        let args = vec![
            ScriptArgument::Int(10),
            ScriptArgument::String("test".into()),
        ];
        q.add(QueuePriority::Normal, 1, 0, Some(args)).unwrap();
        let h = q.queue.head().unwrap();
        let script = q.queue.get(h);
        assert!(script.args.is_some());
        let args = script.args.as_ref().unwrap();
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn add_multiple_normal_preserves_order() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Normal, 1, 0, None).unwrap();
        q.add(QueuePriority::Normal, 2, 0, None).unwrap();
        q.add(QueuePriority::Normal, 3, 0, None).unwrap();

        let h = q.queue.head().unwrap();
        assert_eq!(q.queue.get(h).script_id, 1);
        let h = q.queue.next().unwrap();
        assert_eq!(q.queue.get(h).script_id, 2);
        let h = q.queue.next().unwrap();
        assert_eq!(q.queue.get(h).script_id, 3);
        assert!(q.queue.next().is_none());
    }

    #[test]
    fn add_mixed_priorities_to_queue() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Normal, 1, 0, None).unwrap();
        q.add(QueuePriority::Strong, 2, 0, None).unwrap();
        q.add(QueuePriority::Long, 3, 0, None).unwrap();

        let mut count = 0;
        let mut h = q.queue.head();
        while h.is_some() {
            count += 1;
            h = q.queue.next();
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn add_all_priority_types() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Normal, 1, 0, None).unwrap();
        q.add(QueuePriority::Strong, 2, 0, None).unwrap();
        q.add(QueuePriority::Long, 3, 0, None).unwrap();
        q.add(QueuePriority::Weak, 4, 0, None).unwrap();
        q.add(QueuePriority::Engine, 5, 50, None).unwrap();
        let result = q.add(QueuePriority::Soft, 6, 0, None);
        assert!(result.is_err());

        assert!(!q.queue.is_empty());
        assert!(!q.weak.is_empty());
        assert!(!q.engine.is_empty());
    }

    #[test]
    fn engine_delay_forced_to_zero_regardless_of_input() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Engine, 1, 0, None).unwrap();
        q.add(QueuePriority::Engine, 2, 100, None).unwrap();
        q.add(QueuePriority::Engine, 3, u16::MAX, None).unwrap();

        let mut h = q.engine.head();
        while let Some(idx) = h {
            assert_eq!(q.engine.get(idx).delay, 0);
            h = q.engine.next();
        }
    }

    #[test]
    fn soft_error_message_contains_script_id() {
        let mut q = ScriptQueue::new();
        let err = q.add(QueuePriority::Soft, 12345, 0, None).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("12345"));
    }

    #[test]
    fn add_none_args() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Normal, 1, 0, None).unwrap();
        let h = q.queue.head().unwrap();
        assert!(q.queue.get(h).args.is_none());
    }

    #[test]
    fn queue_iteration_pattern() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Normal, 1, 3, None).unwrap();
        q.add(QueuePriority::Normal, 2, 1, None).unwrap();
        q.add(QueuePriority::Normal, 3, 0, None).unwrap();

        // Simulate engine tick: iterate and collect ready scripts (delay == 0)
        let mut ready = Vec::new();
        let mut h = q.queue.head();
        while let Some(idx) = h {
            let script = q.queue.get_mut(idx);
            if script.delay == 0 {
                ready.push(script.script_id);
            } else {
                script.delay -= 1;
            }
            h = q.queue.next();
        }
        assert_eq!(ready, vec![3]);
    }

    #[test]
    fn queue_unlink_during_iteration() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Normal, 1, 0, None).unwrap();
        q.add(QueuePriority::Normal, 2, 0, None).unwrap();
        q.add(QueuePriority::Normal, 3, 0, None).unwrap();

        // Unlink script 2 during iteration
        let mut executed = Vec::new();
        let mut h = q.queue.head();
        while let Some(idx) = h {
            let script = q.queue.get(idx);
            if script.script_id == 2 {
                q.queue.unlink(idx);
            } else {
                executed.push(script.script_id);
            }
            h = q.queue.next();
        }
        assert_eq!(executed, vec![1, 3]);
    }

    #[test]
    fn strong_clears_weak_pattern() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Weak, 1, 0, None).unwrap();
        q.add(QueuePriority::Weak, 2, 0, None).unwrap();
        q.add(QueuePriority::Strong, 3, 0, None).unwrap();

        // Engine pattern: if strong exists, clear weak
        let has_strong = {
            let mut found = false;
            let mut h = q.queue.head();
            while let Some(idx) = h {
                if q.queue.get(idx).priority == QueuePriority::Strong {
                    found = true;
                }
                h = q.queue.next();
            }
            found
        };

        if has_strong {
            q.weak.clear();
        }
        assert!(q.weak.is_empty());
        assert!(!q.queue.is_empty());
    }

    #[test]
    fn engine_priority_preservation() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Engine, 1, 100, None).unwrap();
        let h = q.engine.head().unwrap();
        assert_eq!(q.engine.get(h).priority, QueuePriority::Engine);
        assert_eq!(q.engine.get(h).delay, 0); // always forced to 0
    }

    #[test]
    fn weak_queue_independent_of_main() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Normal, 1, 0, None).unwrap();
        q.add(QueuePriority::Weak, 2, 0, None).unwrap();

        // Clear main queue shouldn't affect weak
        q.queue.clear();
        assert!(!q.weak.is_empty());
        let h = q.weak.head().unwrap();
        assert_eq!(q.weak.get(h).script_id, 2);
    }

    #[test]
    fn delay_decrement_pattern() {
        let mut q = ScriptQueue::new();
        q.add(QueuePriority::Normal, 1, 3, None).unwrap();

        for tick in 0..4 {
            let h = q.queue.head().unwrap();
            let script = q.queue.get_mut(h);
            if tick < 3 {
                assert!(script.delay > 0);
                script.delay -= 1;
            } else {
                assert_eq!(script.delay, 0);
            }
        }
    }
}
