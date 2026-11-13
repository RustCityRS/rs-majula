//! `#testscript` commands: assertions, the command call log, and the clock.
//!
//! Ported from rs-native's `ops/test.rs`. The production registry
//! ([`register_ops`] in rs-engine) never includes these; the headless runner
//! (`rs-test`) extends its engine's registry with [`build`] and with the
//! engine-typed commands (`setup_player`, `test_op`) that need a live world.
//! A script that reaches one of these opcodes anywhere else fails as an
//! unknown opcode.
//!
//! Results and the call log live in thread-locals. The runner drives one
//! engine on one thread and clears them between tests with [`clear`].
//!
//! [`register_ops`]: https://docs.rs/rs-engine

use crate::register::OpsRegistry;
use crate::state::ExecutionState;
use crate::{handlers, none};
use runec::opcodes::*;
use std::cell::{Cell, RefCell};
use std::mem;
use tracing::info;

/// One assertion's outcome, in the order the script ran them.
#[derive(Debug, Clone)]
pub struct AssertionResult {
    pub passed: bool,
    pub message: String,
    /// Program counter of the assertion, for locating it in the script.
    pub pc: i32,
}

/// One recorded call of an instrumented command (`mes`, `loc_change`,
/// `damage`).
#[derive(Debug, Clone)]
pub struct CallRecord {
    pub opcode: u16,
    pub name: &'static str,
    pub arg: String,
}

thread_local! {
    static RESULTS: RefCell<Vec<AssertionResult>> = const { RefCell::new(Vec::new()) };
    static CALL_LOG: RefCell<Vec<CallRecord>> = const { RefCell::new(Vec::new()) };
    static RECORDING: Cell<bool> = const { Cell::new(false) };
    static FAIL_EXPECTED: Cell<bool> = const { Cell::new(false) };
    static WAIT_CURSOR: Cell<usize> = const { Cell::new(0) };
}

/// Turns the call log on for this thread. Off by default, so the hooks in
/// `mes` and `loc_change` cost one thread-local read in production.
pub fn set_recording(on: bool) {
    RECORDING.with(|r| r.set(on));
}

/// Whether the call log is on, for a hook whose argument costs a format.
pub fn recording() -> bool {
    RECORDING.with(|r| r.get())
}

/// Records a call of an instrumented command, if recording is on.
pub fn record_call(opcode: u16, name: &'static str, arg: &str) {
    if !RECORDING.with(|r| r.get()) {
        return;
    }
    CALL_LOG.with(|log| {
        log.borrow_mut().push(CallRecord {
            opcode,
            name,
            arg: arg.to_string(),
        });
    });
}

/// Number of calls of the command named `name` recorded since this test began.
pub fn call_count(name: &str) -> usize {
    CALL_LOG.with(|log| log.borrow().iter().filter(|r| r.name == name).count())
}

/// Number of calls of the command named `name` recorded at or after log
/// position `since`.
pub fn call_count_since(name: &str, since: usize) -> usize {
    CALL_LOG.with(|log| {
        log.borrow()
            .iter()
            .skip(since)
            .filter(|r| r.name == name)
            .count()
    })
}

/// The log position `wait_for` has consumed up to.
pub fn cursor() -> usize {
    WAIT_CURSOR.with(|c| c.get())
}

/// Marks every call recorded so far as consumed by `wait_for`.
pub fn advance_cursor() {
    let len = CALL_LOG.with(|log| log.borrow().len());
    WAIT_CURSOR.with(|c| c.set(len));
}

fn call_has_arg(name: &str, expected_arg: &str) -> bool {
    CALL_LOG.with(|log| {
        log.borrow()
            .iter()
            .any(|r| r.name == name && r.arg == expected_arg)
    })
}

/// The argument of every recorded call of the command named `name`, in order.
pub fn calls_for(name: &str) -> Vec<String> {
    CALL_LOG.with(|log| {
        log.borrow()
            .iter()
            .filter(|r| r.name == name)
            .map(|r| r.arg.clone())
            .collect()
    })
}

/// Drops every call recorded so far. `setup_player` calls this once its
/// login has run, so the welcome messages of `[login]` are not mistaken for
/// what the test went on to cause.
pub fn forget_calls() {
    CALL_LOG.with(|log| log.borrow_mut().clear());
    WAIT_CURSOR.with(|c| c.set(0));
}

/// Whether the running test declared `expect_fail`.
pub fn is_expect_fail() -> bool {
    FAIL_EXPECTED.with(|f| f.get())
}

/// Resets results, the call log and `expect_fail` for the next test.
pub fn clear() {
    RESULTS.with(|r| r.borrow_mut().clear());
    CALL_LOG.with(|log| log.borrow_mut().clear());
    FAIL_EXPECTED.with(|f| f.set(false));
    WAIT_CURSOR.with(|c| c.set(0));
}

/// Takes the assertions recorded since the last [`clear`].
pub fn take_results() -> Vec<AssertionResult> {
    RESULTS.with(|r| mem::take(&mut *r.borrow_mut()))
}

/// Records one assertion outcome.
pub fn record(pc: i32, passed: bool, message: String) {
    let marker = if passed { "PASS" } else { "FAIL" };
    info!(pc, "[assert {marker}] {message}");
    RESULTS.with(|r| {
        r.borrow_mut().push(AssertionResult {
            passed,
            message,
            pc,
        });
    });
}

fn called_listing(name: &str, calls: &[String]) -> String {
    calls
        .iter()
        .map(|a| format!("  - {name}(\"{a}\")"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Registers the engine-independent test commands.
///
/// # Opcodes Registered
///
/// - `ASSERT_*`, `TEST_FAIL`, `EXPECT_FAIL` -- assertions.
/// - `ASSERT_CALLED`, `ASSERT_CALLED_WITH` -- read the call log. They take the
///   command's *name*: in RuneScript a bare `mes` is a call to `mes`, so the
///   command cannot be passed as itself.
/// - `TEST_TICK` -- suspends the world script for exactly N ticks.
/// - `WAIT_FOR` -- suspends until an instrumented command is called, or fails
///   once its timeout runs out.
pub fn build() -> OpsRegistry {
    handlers! { |m|
        // 50000
        none!(m, ASSERT_EQ => |s| {
            let expected = s.pop_int();
            let actual = s.pop_int();
            let pass = actual == expected;
            record(s.pc, pass, if pass {
                format!("assert_eq({actual}, {expected})")
            } else {
                format!("assert_eq failed: expected {expected}, got {actual}")
            });
        });

        // 50001
        none!(m, ASSERT_NOT_EQ => |s| {
            let unexpected = s.pop_int();
            let actual = s.pop_int();
            let pass = actual != unexpected;
            record(s.pc, pass, if pass {
                format!("assert_not_eq({actual}, {unexpected})")
            } else {
                format!("assert_not_eq failed: {actual} should not equal {unexpected}")
            });
        });

        // 50002
        none!(m, ASSERT_STR_EQ => |s| {
            let expected = s.pop_string();
            let actual = s.pop_string();
            let pass = actual == expected;
            record(s.pc, pass, if pass {
                format!("assert_str_eq(\"{actual}\", \"{expected}\")")
            } else {
                format!("assert_str_eq failed: expected \"{expected}\", got \"{actual}\"")
            });
        });

        // 50003
        none!(m, ASSERT_TRUE => |s| {
            let value = s.pop_int();
            let pass = value == 1;
            record(s.pc, pass, if pass {
                "assert_true(true)".to_string()
            } else {
                format!("assert_true failed: got {value}")
            });
        });

        // 50004
        none!(m, ASSERT_FALSE => |s| {
            let value = s.pop_int();
            let pass = value == 0;
            record(s.pc, pass, if pass {
                "assert_false(false)".to_string()
            } else {
                format!("assert_false failed: got {value}")
            });
        });

        // 50005
        none!(m, ASSERT_GT => |s| {
            let expected = s.pop_int();
            let actual = s.pop_int();
            let pass = actual > expected;
            record(s.pc, pass, if pass {
                format!("assert_gt({actual} > {expected})")
            } else {
                format!("assert_gt failed: {actual} is not > {expected}")
            });
        });

        // 50006
        none!(m, ASSERT_LT => |s| {
            let expected = s.pop_int();
            let actual = s.pop_int();
            let pass = actual < expected;
            record(s.pc, pass, if pass {
                format!("assert_lt({actual} < {expected})")
            } else {
                format!("assert_lt failed: {actual} is not < {expected}")
            });
        });

        // 50007
        none!(m, ASSERT_GTE => |s| {
            let expected = s.pop_int();
            let actual = s.pop_int();
            let pass = actual >= expected;
            record(s.pc, pass, if pass {
                format!("assert_gte({actual} >= {expected})")
            } else {
                format!("assert_gte failed: {actual} is not >= {expected}")
            });
        });

        // 50008
        none!(m, ASSERT_LTE => |s| {
            let expected = s.pop_int();
            let actual = s.pop_int();
            let pass = actual <= expected;
            record(s.pc, pass, if pass {
                format!("assert_lte({actual} <= {expected})")
            } else {
                format!("assert_lte failed: {actual} is not <= {expected}")
            });
        });

        // 50009
        none!(m, ASSERT_NULL => |s| {
            let value = s.pop_int();
            let pass = value == -1;
            record(s.pc, pass, if pass {
                "assert_null(-1)".to_string()
            } else {
                format!("assert_null failed: expected -1, got {value}")
            });
        });

        // 50010
        none!(m, ASSERT_NOT_NULL => |s| {
            let value = s.pop_int();
            let pass = value != -1;
            record(s.pc, pass, if pass {
                format!("assert_not_null({value})")
            } else {
                "assert_not_null failed: got -1".to_string()
            });
        });

        // 50011
        none!(m, TEST_FAIL => |s| {
            let msg = s.pop_string();
            record(s.pc, false, format!("test_fail: {msg}"));
            s.execution = ExecutionState::Aborted;
        });

        // 50012
        none!(m, ASSERT_CALLED => |s| {
            let expected_count = s.pop_int();
            let name = s.pop_string();
            let actual = call_count(&name) as i32;
            let pass = actual == expected_count;
            record(s.pc, pass, if pass {
                format!("assert_called({name}, {expected_count})")
            } else {
                let calls = calls_for(&name);
                if calls.is_empty() {
                    format!("assert_called failed: {name} was never called (expected {expected_count})")
                } else {
                    format!(
                        "assert_called failed: {name} called {actual} time(s), expected {expected_count}. Actual calls:\n{}",
                        called_listing(&name, &calls)
                    )
                }
            });
        });

        // 50013
        none!(m, ASSERT_CALLED_WITH => |s| {
            let expected_arg = s.pop_string();
            let name = s.pop_string();
            let pass = call_has_arg(&name, &expected_arg);
            record(s.pc, pass, if pass {
                format!("assert_called_with({name}, \"{expected_arg}\")")
            } else {
                let calls = calls_for(&name);
                if calls.is_empty() {
                    format!("assert_called_with failed: {name} was never called (expected \"{expected_arg}\")")
                } else {
                    format!(
                        "assert_called_with failed: {name} was never called with \"{expected_arg}\". Actual calls:\n{}",
                        called_listing(&name, &calls)
                    )
                }
            });
        });

        // 50014
        none!(m, EXPECT_FAIL => |_s| {
            FAIL_EXPECTED.with(|f| f.set(true));
        });

        // 50050
        none!(m, TEST_TICK => |s| {
            let count = s.pop_int();
            if count > 0 {
                // The world phase re-enqueues a suspended world script with
                // the delay it pops, and the queue runs it `delay + 2` passes
                // later (`enqueue_world_script` adds one; `tick_down` counts
                // down after it checks, as the reference engine does) -- so
                // push `count - 2` for `test_tick(n)` to mean n.
                s.push_int(count - 2);
                s.execution = ExecutionState::WorldSuspended;
            }
        });

        // 50054
        none!(m, WAIT_FOR => |s| {
            let timeout = s.pop_int();
            let name = s.pop_string();
            if call_count_since(&name, cursor()) > 0 {
                advance_cursor();
            } else if timeout <= 0 {
                record(s.pc, false, format!(
                    "wait_for failed: {name} was not called within the timeout"
                ));
            } else {
                // Run this instruction again next tick with one tick less:
                // re-push its operands, then the -1 the world phase pops as
                // the suspension delay (a delay of d runs d + 2 passes on).
                s.push_string(&name);
                s.push_int(timeout - 1);
                s.push_int(-1);
                s.pc -= 1;
                s.execution = ExecutionState::WorldSuspended;
            }
        });
    }
}
