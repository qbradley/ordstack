//! Allocation failure.

use core::fmt;

/// Why an allocation failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AllocErrorKind {
    /// The stack's reservation is used up.
    Exhausted,
    /// A size computation overflowed `usize` (element count times element
    /// size, an offset plus a size, or a length plus an additional count), or
    /// a reservation requested of
    /// [`Stack::with_reserve`](crate::Stack::with_reserve) cannot be
    /// represented (more than `isize::MAX` minus a page, after rounding up
    /// to whole pages). A size that fits in `usize` but is larger than the
    /// stack's reservation, including one above `isize::MAX`, reports
    /// [`Exhausted`](Self::Exhausted) instead.
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
