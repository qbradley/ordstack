//! Stack-allocated containers whose stack discipline is proved by the borrow
//! checker. (Full crate documentation is written in Task 12.)

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

#[cfg(not(any(unix, miri)))]
compile_error!("ordstack 0.1 supports Unix targets only (Linux and macOS are tested)");

mod debug;
mod error;
mod region;
mod stack;

pub use error::{AllocError, AllocErrorKind};
pub use stack::{DEFAULT_RESERVE, Stack};
