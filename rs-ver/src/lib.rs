pub mod archive;
mod revision;
mod type_id;

pub use revision::{REVS, compile_target_revision};
pub use type_id::TypeId;

#[cfg(before_647)]
pub const STRING_TERMINATOR: u8 = 10;
#[cfg(since_647)]
pub const STRING_TERMINATOR: u8 = 0;
