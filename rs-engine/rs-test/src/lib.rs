//! Headless content tests (tier T2).
//!
//! Runs every `[label,...]` below a `#testscript` line in the revision's
//! content against a real [`Engine`](rs_engine::Engine): the packed configs,
//! the loaded map, the full opcode set and the engine's own tick. Only the
//! socket is missing -- players log in through the harness instead -- so a
//! passing test exercises the same script, VM and engine code a client would.
//!
//! Rev 530 and 647 only: the test commands are declared by the content's
//! `engine.rs2`, and the harness is built on the 530 and 647 login paths.

#[cfg(since_530)]
pub mod runner;
#[cfg(since_647)]
pub mod talk;
