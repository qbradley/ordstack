//! Allocation failure.

use core::fmt;

/// Why an allocation failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AllocErrorKind {
    /// The stack's reservation is used up.
    Exhausted,
    /// The requested size overflows `usize` or exceeds `isize::MAX`.
    CapacityOverflow,
    /// The requested alignment is above 4096.
    AlignTooLarge,
    /// The operating system refused the mapping.
    MapFailed,
}

/// An allocation on a [`Stack`](crate::Stack) failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllocError {
    kind: AllocErrorKind,
}

impl AllocError {
    pub(crate) const fn new(kind: AllocErrorKind) -> AllocError {
        AllocError { kind }
    }

    /// The reason for the failure.
    pub fn kind(&self) -> AllocErrorKind {
        self.kind
    }
}

impl fmt::Display for AllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.kind {
            AllocErrorKind::Exhausted => "stack reservation exhausted",
            AllocErrorKind::CapacityOverflow => "capacity overflow",
            AllocErrorKind::AlignTooLarge => "alignment above 4096 is not supported",
            AllocErrorKind::MapFailed => "the operating system refused the mapping",
        })
    }
}

impl std::error::Error for AllocError {}

/// The single panic path of every infallible allocating method.
#[cold]
#[inline(never)]
#[track_caller]
pub(crate) fn fail(e: AllocError) -> ! {
    panic!("ordstack: {e}")
}
