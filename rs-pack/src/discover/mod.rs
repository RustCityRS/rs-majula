//! Discover — cache → `.pack` registry files in `content/pack/`.
//!
//! Bootstrap path for a fresh cache revision: enumerate JS5 archive contents
//! and emit baseline `id=name` registries. Idempotent by default —
//! hand-maintained names in existing pack files are preserved; only auto-named
//! / missing entries are (re)written.

pub mod dbrow;
pub mod npc;
pub mod symbols;

pub use dbrow::{DbDiscoveryReport, discover_dbrows};
pub use npc::{NpcDiscoveryReport, discover_npcs};
pub use symbols::{DiscoveryReport, TypeReport, discover_symbols};
