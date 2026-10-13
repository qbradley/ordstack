//! [`SBox`], an owning pointer to one frozen value.

use core::marker::PhantomData;
use core::mem::{ManuallyDrop, size_of};
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use crate::debug;
use crate::macros::deref_traits;

/// An owning pointer to a value in a frozen block.
///
/// Dropping the box drops the value. The bytes are reclaimed with the frame.
pub struct SBox<'f, T> {
    ptr: NonNull<T>,
    _brand: PhantomData<(&'f (), T)>,
}

impl<'f, T> SBox<'f, T> {
    /// # Safety
    /// `ptr` must point to an initialised `T` in a block that stays allocated
    /// and unaliased for `'f` (or be dangling and aligned if `T` is
    /// zero-size, with one logical `T` owned).
    pub(crate) unsafe fn from_raw(ptr: NonNull<T>) -> SBox<'f, T> {
        SBox {
            ptr,
            _brand: PhantomData,
        }
    }

    fn check(&self) {
        debug::check_live(self.ptr.as_ptr().cast(), size_of::<T>());
    }

    /// Moves the value out. Its bytes stay allocated until the frame is
    /// reclaimed.
    pub fn into_inner(this: SBox<'f, T>) -> T {
        this.check();
        let this = ManuallyDrop::new(this);
        // SAFETY: the box owns an initialised `T` and is never used again.
        unsafe { this.ptr.as_ptr().read() }
    }

    /// Gives up ownership and returns a reference valid for the brand. The
    /// value's destructor will not run.
    pub fn leak(this: SBox<'f, T>) -> &'f mut T
    where
        T: 'f,
    {
        this.check();
        let this = ManuallyDrop::new(this);
        // SAFETY: the block is allocated and unaliased for `'f`, and the
        // forgotten box was its only owner.
        unsafe { &mut *this.ptr.as_ptr() }
    }
}

impl<T> Deref for SBox<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.check();
        // SAFETY: initialised and alive for the brand.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for SBox<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.check();
        // SAFETY: as above, and the box is the unique owner.
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Drop for SBox<'_, T> {
    fn drop(&mut self) {
        self.check();
        // SAFETY: unique owner of an initialised value; never used again.
        unsafe { core::ptr::drop_in_place(self.ptr.as_ptr()) };
    }
}

deref_traits!(['f, T] SBox<'f, T>, T);
