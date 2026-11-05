//! Headless content tests (tier T2).
//!
//! Runs every `[label,...]` below a `#testscript` line in the revision's
//! content against a real [`Engine`](rs_engine::Engine): the packed configs,
//! the loaded map, the full opcode set and the engine's own tick. Only the
//! socket is missing -- players log in through the harness instead -- so a
//! passing test exercises the same script, VM and engine code a client would.
//!
//! Rev 647 only: the test commands are declared by `content/647`'s
//! `engine.rs2`, and the harness is built on the 647 login path.

#[cfg(since_647)]
pub mod runner;
#[cfg(since_647)]
pub mod talk;
