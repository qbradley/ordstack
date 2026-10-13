//! [`Frame`], the allocation cursor.

use core::mem::{align_of, size_of};
use core::ptr::NonNull;

use crate::boxed::SBox;
use crate::error::{AllocError, AllocErrorKind, fail};
use crate::slice::{SSlice, SStr, drop_back_to_front};
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
    ///
    /// # Examples
    /// ```
    /// use ordstack::Stack;
    ///
    /// let mut st = Stack::with_reserve(1 << 16);
    /// let mut f = st.frame();
    /// let kept = f.alloc(1u32);
    /// {
    ///     let mut sub = f.sub();
    ///     let tmp = sub.alloc_slice_copy(&[1u8, 2, 3]);
    ///     assert_eq!(tmp.len(), 3);
    /// }
    /// let next = f.alloc(2u32); // reuses the space the sub-frame used
    /// assert_eq!(*kept + *next, 3);
    /// ```
    pub fn sub(&mut self) -> Frame<'_> {
        self.st.reset_to(self.end);
        Frame {
            st: self.st,
            end: self.end,
        }
    }

    /// Reclaims everything above this frame now, instead of when the frame
    /// next allocates or hands out a sub-frame.
    ///
    /// # Examples
    /// ```
    /// use ordstack::Stack;
    ///
    /// let mut st = Stack::with_reserve(1 << 16);
    /// {
    ///     let mut f = st.frame();
    ///     let _kept = f.alloc(1u64);
    ///     drop(f.sub().alloc([0u8; 100])); // the 100 bytes stay in use for now
    ///     f.reset_above();
    ///     assert_eq!(f.end_offset(), 8);
    /// }
    /// assert_eq!(st.extent(), 8);
    /// ```
    pub fn reset_above(&mut self) {
        self.st.reset_to(self.end);
    }

    /// Reclaims everything above this frame and returns the whole pages
    /// above it to the OS.
    pub fn trim_above(&mut self) {
        self.st.reset_to(self.end);
        self.st.release_above(self.end);
    }

    /// Offset of this frame's end from the base of its stack. Everything the
    /// frame has allocated lies below it.
    pub fn end_offset(&self) -> usize {
        self.end
    }

    /// An uninitialised block for `n` values of `T`, owned by this frame.
    /// Returns a dangling, aligned pointer when the block has zero size; such
    /// a request never reaches the stack. On failure `end` is unchanged; `top`
    /// may have been lowered to `end` by the reset.
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

    /// Copies `src` into a frozen block.
    ///
    /// # Panics
    /// If the stack is exhausted, the size overflows, or a block of non-zero
    /// size needs alignment above 4096.
    pub fn alloc_slice_copy<T: Copy>(&mut self, src: &[T]) -> SSlice<'f, T> {
        let p = self
            .try_alloc_array::<T>(src.len())
            .unwrap_or_else(|e| fail(e));
        // SAFETY: `p` is a fresh block for `src.len()` values, disjoint from
        // `src`; `T: Copy`, so a bitwise copy initialises it.
        unsafe {
            core::ptr::copy_nonoverlapping(src.as_ptr(), p.as_ptr(), src.len());
            SSlice::from_raw_parts(p, src.len())
        }
    }

    /// Clones `src` into a frozen block. If a `clone` panics, the clones made
    /// so far are dropped; the block's space stays with this frame.
    ///
    /// # Panics
    /// If the stack is exhausted, the size overflows, or a block of non-zero
    /// size needs alignment above 4096.
    pub fn alloc_slice_clone<T: Clone>(&mut self, src: &[T]) -> SSlice<'f, T> {
        struct Done<T> {
            ptr: *mut T,
            n: usize,
        }
        impl<T> Drop for Done<T> {
            fn drop(&mut self) {
                // SAFETY: the first `n` slots were initialised by the loop.
                unsafe { drop_back_to_front(self.ptr, self.n) };
            }
        }
        let p = self
            .try_alloc_array::<T>(src.len())
            .unwrap_or_else(|e| fail(e));
        let mut done = Done {
            ptr: p.as_ptr(),
            n: 0,
        };
        for x in src {
            // SAFETY: slot `n < src.len()` is inside the fresh block and
            // uninitialised.
            unsafe { done.ptr.add(done.n).write(x.clone()) };
            done.n += 1;
        }
        core::mem::forget(done);
        // SAFETY: all `src.len()` slots are initialised.
        unsafe { SSlice::from_raw_parts(p, src.len()) }
    }

    /// Copies `src` into a frozen block.
    ///
    /// # Panics
    /// If the stack is exhausted.
    pub fn alloc_str(&mut self, src: &str) -> SStr<'f> {
        // SAFETY: the bytes come from a `str`.
        unsafe { SStr::from_utf8_unchecked(self.alloc_slice_copy(src.as_bytes())) }
    }

    /// Moves `value` into a frozen block.
    ///
    /// # Panics
    /// If the stack is exhausted, or if `T` is not zero-size and its alignment
    /// is above 4096.
    ///
    /// # Examples
    /// ```
    /// use ordstack::Stack;
    ///
    /// let mut st = Stack::with_reserve(1 << 16);
    /// let mut f = st.frame();
    /// let mut b = f.alloc([1u32, 2]);
    /// b[0] = 5;
    /// assert_eq!(*b, [5, 2]);
    /// ```
    pub fn alloc<T>(&mut self, value: T) -> SBox<'f, T> {
        self.try_alloc(value).unwrap_or_else(|e| fail(e))
    }

    /// Moves `value` into a frozen block, or drops it and reports why not.
    /// On failure the frame is unchanged.
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
    ///
    /// # Panics
    /// As [`alloc`](Self::alloc).
    pub fn alloc_with<T>(&mut self, f: impl FnOnce() -> T) -> SBox<'f, T> {
        let value = f();
        self.alloc(value)
    }
}
