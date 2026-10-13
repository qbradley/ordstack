//! [`Frame`], the allocation cursor.

use core::mem::{align_of, size_of};
use core::ptr::NonNull;

use crate::boxed::SBox;
use crate::error::{AllocError, AllocErrorKind, fail};
use crate::stack::Stack;

/// An allocation cursor above some lower object.
///
/// A frame is obtained by mutably borrowing the stack ([`Stack::frame`]),
/// another frame ([`Frame::sub`]) or a growable container (`above`). The
/// lower object stays mutably borrowed, and therefore frozen, for `'f`.
/// Blocks allocated through the frame are branded `'f`.
///
/// A frame has no destructor. Its memory is reclaimed the next time the
/// object beneath it is used.
pub struct Frame<'f> {
    pub(crate) st: &'f Stack,
    /// Everything this frame has allocated lies below `end`.
    pub(crate) end: usize,
}

impl<'f> Frame<'f> {
    /// A frame above this one. This frame is frozen while the sub-frame, or
    /// anything allocated through it, is alive.
    pub fn sub(&mut self) -> Frame<'_> {
        self.st.reset_to(self.end);
        Frame {
            st: self.st,
            end: self.end,
        }
    }

    /// Reclaims everything above this frame now instead of at its next use.
    pub fn reset_above(&mut self) {
        self.st.reset_to(self.end);
    }

    /// Reclaims everything above this frame and returns those pages to the OS.
    pub fn trim_above(&mut self) {
        self.st.reset_to(self.end);
        self.st.release_above(self.end);
    }

    /// Offset of this frame's end from the base of its stack.
    pub fn end_offset(&self) -> usize {
        self.end
    }

    /// An uninitialised block for `n` values of `T`, owned by this frame.
    /// Returns a dangling pointer when the block has zero size.
    pub(crate) fn try_alloc_array<T>(&mut self, n: usize) -> Result<NonNull<T>, AllocError> {
        let bytes = size_of::<T>()
            .checked_mul(n)
            .ok_or(AllocError::new(AllocErrorKind::CapacityOverflow))?;
        if bytes == 0 {
            return Ok(NonNull::dangling());
        }
        // `&mut self` proves nothing above `end` is reachable (invariant I).
        self.st.reset_to(self.end);
        let start = self.st.try_bump(bytes, align_of::<T>())?;
        self.end = start + bytes;
        Ok(self.st.ptr_at(start).cast())
    }

    /// Moves `value` into a frozen block.
    ///
    /// # Panics
    /// If the stack is exhausted or `T`'s alignment is above 4096.
    pub fn alloc<T>(&mut self, value: T) -> SBox<'f, T> {
        self.try_alloc(value).unwrap_or_else(|e| fail(e))
    }

    /// Moves `value` into a frozen block, or drops it and reports why not.
    pub fn try_alloc<T>(&mut self, value: T) -> Result<SBox<'f, T>, AllocError> {
        let p = self.try_alloc_array::<T>(1)?;
        // SAFETY: `p` is a fresh, aligned block for one `T` (or dangling and
        // aligned for a zero-size `T`), below `self.end`, which does not
        // decrease while `'f` is alive.
        unsafe {
            p.as_ptr().write(value);
            Ok(SBox::from_raw(p))
        }
    }

    /// Like [`alloc`](Self::alloc), with the value built by `f`. `f` runs
    /// before any space is reserved.
    pub fn alloc_with<T>(&mut self, f: impl FnOnce() -> T) -> SBox<'f, T> {
        let value = f();
        self.alloc(value)
    }
}
