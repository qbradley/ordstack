//! [`SFixedVec`], a vector with a capacity fixed at creation.

use core::marker::PhantomData;
use core::mem::{ManuallyDrop, size_of};
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use crate::debug;
use crate::error::fail;
use crate::frame::Frame;
use crate::macros::deref_traits;
use crate::slice::{SSlice, drop_back_to_front};

/// A vector whose capacity is fixed when it is created.
///
/// It is a frozen block: unlike [`SVec`](crate::SVec) it does not hold its
/// frame, so the frame can allocate above it while it is being filled.
pub struct SFixedVec<'f, T> {
    ptr: NonNull<T>,
    len: usize,
    cap: usize,
    _brand: PhantomData<(&'f (), T)>,
}

impl<'f, T> SFixedVec<'f, T> {
    /// Reserves a block for `cap` elements.
    ///
    /// # Panics
    /// If the stack is exhausted, the size overflows, or a block of non-zero
    /// size needs alignment above 4096.
    ///
    /// # Examples
    /// ```
    /// use ordstack::{SBox, SFixedVec, Stack};
    ///
    /// let mut st = Stack::with_reserve(1 << 16);
    /// let mut f = st.frame();
    /// let mut kids: SFixedVec<SBox<u32>> = SFixedVec::with_capacity_in(2, &mut f);
    /// kids.push(f.alloc(1)); // the frame is still usable while `kids` fills
    /// kids.push(f.alloc(2));
    /// assert_eq!(*kids[0] + *kids[1], 3);
    /// ```
    pub fn with_capacity_in(cap: usize, frame: &mut Frame<'f>) -> SFixedVec<'f, T> {
        let ptr = frame.try_alloc_array::<T>(cap).unwrap_or_else(|e| fail(e));
        SFixedVec {
            ptr,
            len: 0,
            cap,
            _brand: PhantomData,
        }
    }

    fn check(&self) {
        debug::check_live(self.ptr.as_ptr().cast(), self.cap * size_of::<T>());
    }

    /// The number of elements the vector can hold.
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Whether the vector is at capacity.
    pub fn is_full(&self) -> bool {
        self.len == self.cap
    }

    /// Appends `value`.
    ///
    /// # Panics
    /// If the vector is full.
    pub fn push(&mut self, value: T) {
        if self.try_push(value).is_err() {
            panic!("ordstack: SFixedVec capacity exceeded");
        }
    }

    /// Appends `value`, or returns it if the vector is full.
    pub fn try_push(&mut self, value: T) -> Result<(), T> {
        if self.len == self.cap {
            return Err(value);
        }
        self.check();
        // SAFETY: `len < cap`: the slot is inside the block and uninitialised.
        unsafe { self.ptr.as_ptr().add(self.len).write(value) };
        self.len += 1;
        Ok(())
    }

    /// Removes and returns the last element.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.check();
        self.len -= 1;
        // SAFETY: the slot was initialised and is now outside `[0, len)`.
        Some(unsafe { self.ptr.as_ptr().add(self.len).read() })
    }

    /// Converts into a frozen slice of the elements pushed so far. Unused
    /// capacity is not reclaimed.
    pub fn into_slice(self) -> SSlice<'f, T> {
        let this = ManuallyDrop::new(self);
        // SAFETY: `len` initialised values in a block allocated for `'f`.
        unsafe { SSlice::from_raw_parts(this.ptr, this.len) }
    }
}

impl<T> Deref for SFixedVec<'_, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.check();
        // SAFETY: `len` initialised values.
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for SFixedVec<'_, T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.check();
        // SAFETY: `len` initialised values; unique owner.
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> Drop for SFixedVec<'_, T> {
    fn drop(&mut self) {
        self.check();
        let len = core::mem::replace(&mut self.len, 0);
        // SAFETY: we own `len` initialised values and `self.len` is now 0.
        unsafe { drop_back_to_front(self.ptr.as_ptr(), len) };
    }
}

deref_traits!(['f, T] SFixedVec<'f, T>, [T]);

impl<'a, T> IntoIterator for &'a SFixedVec<'_, T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;
    fn into_iter(self) -> core::slice::Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut SFixedVec<'_, T> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;
    fn into_iter(self) -> core::slice::IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<'f, T> IntoIterator for SFixedVec<'f, T> {
    type Item = T;
    type IntoIter = crate::slice::IntoIter<'f, T>;
    /// Iterates the elements pushed so far; spare capacity is not reclaimed
    /// (see [`into_slice`](Self::into_slice)).
    fn into_iter(self) -> crate::slice::IntoIter<'f, T> {
        self.into_slice().into_iter()
    }
}
