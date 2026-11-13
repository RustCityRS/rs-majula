pub mod archive;
mod revision;

pub use revision::{REVS, compile_target_revision};

#[cfg(before_647)]
pub const STRING_TERMINATOR: u8 = 10;
#[cfg(since_647)]
pub const STRING_TERMINATOR: u8 = 0;
