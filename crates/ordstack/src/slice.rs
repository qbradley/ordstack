//! [`SSlice`] and [`SStr`], owning frozen slices.

use core::fmt;
use core::marker::PhantomData;
use core::mem::{ManuallyDrop, size_of};
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use crate::debug;
use crate::macros::deref_traits;

/// Drops the `len` values starting at `ptr`, last first. If a destructor
/// panics, the remaining values are still dropped during unwinding; a second
/// panic aborts.
///
/// # Safety
/// `ptr` must point to `len` initialised values that the caller owns and
/// will not touch again.
pub(crate) unsafe fn drop_back_to_front<T>(ptr: *mut T, len: usize) {
    struct Rest<T> {
        ptr: *mut T,
        left: usize,
    }
    impl<T> Rest<T> {
        fn run(&mut self) {
            while self.left > 0 {
                self.left -= 1;
                // SAFETY: index `left` is below the original length and is
                // visited once, because `left` is lowered before the drop.
                unsafe { core::ptr::drop_in_place(self.ptr.add(self.left)) };
            }
        }
    }
    impl<T> Drop for Rest<T> {
        fn drop(&mut self) {
            self.run();
        }
    }
    Rest { ptr, left: len }.run();
}

/// An owning slice in a frozen block. Elements are dropped back to front.
pub struct SSlice<'f, T> {
    ptr: NonNull<T>,
    len: usize,
    _brand: PhantomData<(&'f (), T)>,
}

impl<'f, T> SSlice<'f, T> {
    /// An empty slice. Allocates nothing.
    pub const fn empty() -> SSlice<'f, T> {
        SSlice {
            ptr: NonNull::dangling(),
            len: 0,
            _brand: PhantomData,
        }
    }

    /// # Safety
    /// `ptr` must point to `len` initialised values in a block that stays
    /// allocated and unaliased for `'f` (or be dangling and aligned when
    /// `len * size_of::<T>() == 0`).
    pub(crate) unsafe fn from_raw_parts(ptr: NonNull<T>, len: usize) -> SSlice<'f, T> {
        SSlice {
            ptr,
            len,
            _brand: PhantomData,
        }
    }

    fn check(&self) {
        debug::check_live(self.ptr.as_ptr().cast(), self.len * size_of::<T>());
    }

    /// Gives up ownership and returns a reference valid for the brand. The
    /// elements' destructors will not run.
    pub fn leak(this: SSlice<'f, T>) -> &'f mut [T]
    where
        T: 'f,
    {
        this.check();
        let this = ManuallyDrop::new(this);
        // SAFETY: allocated, initialised and unaliased for `'f`.
        unsafe { core::slice::from_raw_parts_mut(this.ptr.as_ptr(), this.len) }
    }
}

impl<T> Deref for SSlice<'_, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.check();
        // SAFETY: `len` initialised values, alive for the brand.
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for SSlice<'_, T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.check();
        // SAFETY: as above, and the slice is the unique owner.
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> Drop for SSlice<'_, T> {
    fn drop(&mut self) {
        self.check();
        let len = core::mem::replace(&mut self.len, 0);
        // SAFETY: we own `len` initialised values and `self.len` is now 0.
        unsafe { drop_back_to_front(self.ptr.as_ptr(), len) };
    }
}

deref_traits!(['f, T] SSlice<'f, T>, [T]);

/// By-value iterator over an [`SSlice`]. Elements not yet yielded are
/// dropped back to front when the iterator is dropped.
pub struct IntoIter<'f, T> {
    ptr: NonNull<T>,
    head: usize,
    tail: usize,
    _brand: PhantomData<(&'f (), T)>,
}

impl<'f, T> IntoIterator for SSlice<'f, T> {
    type Item = T;
    type IntoIter = IntoIter<'f, T>;
    fn into_iter(self) -> IntoIter<'f, T> {
        self.check();
        let this = ManuallyDrop::new(self);
        IntoIter {
            ptr: this.ptr,
            head: 0,
            tail: this.len,
            _brand: PhantomData,
        }
    }
}

impl<T> Iterator for IntoIter<'_, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.head == self.tail {
            return None;
        }
        self.head += 1;
        // SAFETY: index `head - 1` is initialised and is never read again.
        Some(unsafe { self.ptr.as_ptr().add(self.head - 1).read() })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.tail - self.head;
        (n, Some(n))
    }
}

impl<T> DoubleEndedIterator for IntoIter<'_, T> {
    fn next_back(&mut self) -> Option<T> {
        if self.head == self.tail {
            return None;
        }
        self.tail -= 1;
        // SAFETY: index `tail` is initialised and is never read again.
        Some(unsafe { self.ptr.as_ptr().add(self.tail).read() })
    }
}

impl<T> ExactSizeIterator for IntoIter<'_, T> {}

impl<T> Drop for IntoIter<'_, T> {
    fn drop(&mut self) {
        let (head, n) = (self.head, self.tail - self.head);
        self.tail = self.head;
        // SAFETY: `[head, head + n)` are the initialised values not yet
        // yielded, and the bounds were just emptied.
        unsafe { drop_back_to_front(self.ptr.as_ptr().add(head), n) };
    }
}

/// An owning string in a frozen block.
pub struct SStr<'f> {
    bytes: SSlice<'f, u8>,
}

impl<'f> SStr<'f> {
    /// An empty string. Allocates nothing.
    pub const fn empty() -> SStr<'f> {
        SStr {
            bytes: SSlice::empty(),
        }
    }

    /// # Safety
    /// `bytes` must be valid UTF-8.
    pub(crate) unsafe fn from_utf8_unchecked(bytes: SSlice<'f, u8>) -> SStr<'f> {
        SStr { bytes }
    }

    /// The string as a `&str`.
    pub fn as_str(&self) -> &str {
        self
    }

    /// Gives up ownership and returns a reference valid for the brand.
    pub fn leak(this: SStr<'f>) -> &'f mut str {
        // SAFETY: the bytes are valid UTF-8 by construction.
        unsafe { core::str::from_utf8_unchecked_mut(SSlice::leak(this.bytes)) }
    }
}

impl Deref for SStr<'_> {
    type Target = str;
    fn deref(&self) -> &str {
        // SAFETY: only built from a `str` or from a frozen `SString`.
        unsafe { core::str::from_utf8_unchecked(&self.bytes) }
    }
}

impl fmt::Display for SStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

deref_traits!(['f] SStr<'f>, str);
