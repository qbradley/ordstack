//! Stack-allocated containers whose stack discipline is proved by the borrow
//! checker. (Full crate documentation is written in Task 12.)

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

#[cfg(not(any(unix, miri)))]
compile_error!("ordstack 0.1 supports Unix targets only (Linux and macOS are tested)");

mod boxed;
mod debug;
mod error;
mod fixed;
mod frame;
mod macros;
mod region;
mod slice;
mod stack;

pub use boxed::SBox;
pub use error::{AllocError, AllocErrorKind};
pub use fixed::SFixedVec;
pub use frame::Frame;
pub use slice::{SSlice, SStr};
pub use stack::{DEFAULT_RESERVE, Stack};

/// Iterator types.
pub mod iter {
    pub use crate::slice::IntoIter;
}
