//! [`SVec`], the growable vector.

use core::cell::Cell;
use core::marker::PhantomData;
use core::mem::{ManuallyDrop, align_of, size_of};
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use crate::debug;
use crate::error::{AllocError, AllocErrorKind, fail};
use crate::frame::Frame;
use crate::slice::{IntoIter, SSlice, drop_back_to_front};

const OVERFLOW: AllocError = AllocError::new(AllocErrorKind::CapacityOverflow);

/// A growable vector on a stack.
///
/// The vector holds its frame exclusively, so it grows in place. Append and
/// reservation take `&self`: existing element references remain valid because
/// only the uninitialized tail is written. Operations that move, remove or
/// mutate elements still require `&mut self`. To allocate anything else on
/// the same stack, take a frame above the vector with [`above`](Self::above);
/// the vector is frozen until that frame and everything allocated through it
/// are gone.
///
/// Do not append while this vector is stored as a hash, ordered-set, or
/// ordered-map key: changing a key's contents invalidates that collection's
/// lookup or ordering invariants.
///
/// Elements are dropped back to front.
pub struct SVec<'v, 'f, T> {
    frame: &'v mut Frame<'f>,
    /// Start of the buffer; dangling while nothing is allocated.
    ptr: Cell<NonNull<T>>,
    len: Cell<usize>,
    /// `usize::MAX` for zero-size `T`, which never allocates.
    cap: Cell<usize>,
    _owns: PhantomData<T>,
}

impl<'v, 'f, T> SVec<'v, 'f, T> {
    const ZST: bool = size_of::<T>() == 0;

    /// An empty vector on top of `frame`. Allocates nothing yet.
    pub fn new_in(frame: &'v mut Frame<'f>) -> SVec<'v, 'f, T> {
        let cap = if Self::ZST { usize::MAX } else { 0 };
        SVec {
            frame,
            ptr: Cell::new(NonNull::dangling()),
            len: Cell::new(0),
            cap: Cell::new(cap),
            _owns: PhantomData,
        }
    }

    /// An empty vector with room for exactly `cap` elements. A zero-size `T`
    /// needs no room: its capacity is always `usize::MAX`.
    ///
    /// # Panics
    /// If the stack is exhausted, the capacity overflows, or a non-zero-size
    /// `T` has alignment above 4096.
    pub fn with_capacity_in(cap: usize, frame: &'v mut Frame<'f>) -> SVec<'v, 'f, T> {
        let v = SVec::new_in(frame);
        if cap > v.cap.get() {
            v.try_grow_to(cap).unwrap_or_else(|e| fail(e));
        }
        v
    }

    /// Collects `iter` into a vector on top of `frame`.
    ///
    /// # Panics
    /// If the stack is exhausted, the capacity overflows, or a non-zero-size
    /// `T` has alignment above 4096. The elements collected so far are
    /// dropped.
    pub fn from_iter_in<I: IntoIterator<Item = T>>(
        iter: I,
        frame: &'v mut Frame<'f>,
    ) -> SVec<'v, 'f, T> {
        let mut v = SVec::new_in(frame);
        v.extend(iter);
        v
    }

    fn allocated(&self) -> bool {
        !Self::ZST && self.cap.get() > 0
    }

    /// Offset of the buffer's start. Only meaningful when allocated.
    fn start_off(&self) -> usize {
        self.frame.st.offset_of(self.ptr.get().cast())
    }

    /// Offset of the end of this vector: its capacity end, or the frame's
    /// end while nothing is allocated.
    fn end_off(&self) -> usize {
        if self.allocated() {
            self.start_off() + self.cap.get() * size_of::<T>()
        } else {
            self.frame.end
        }
    }

    fn check(&self) {
        debug::check_live(
            self.ptr.get().as_ptr().cast(),
            if Self::ZST {
                0
            } else {
                self.cap.get() * size_of::<T>()
            },
        );
    }

    /// Sets the capacity to `new_cap > self.cap`. Never moves the buffer.
    /// On failure the vector is unchanged.
    fn try_grow_to(&self, new_cap: usize) -> Result<(), AllocError> {
        debug_assert!(!Self::ZST && new_cap > self.cap.get());
        let st = self.frame.st;
        let bytes = new_cap.checked_mul(size_of::<T>()).ok_or(OVERFLOW)?;
        let old_end = self.end_off();
        // Any allocation above our end must hold an exclusive borrow of the
        // vector. Shared element references lie inside our initialized
        // prefix, so they cannot make space above our capacity reachable.
        st.reset_to(old_end);
        if self.allocated() {
            let new_end = self.start_off().checked_add(bytes).ok_or(OVERFLOW)?;
            st.try_extend(old_end, new_end)?;
        } else {
            let start = st.try_bump(bytes, align_of::<T>())?;
            self.ptr.set(st.ptr_at(start).cast());
        }
        self.cap.set(new_cap);
        Ok(())
    }

    /// Makes room for at least `additional` more elements without invalidating
    /// references into the initialized prefix.
    ///
    /// # Panics
    /// If the stack is exhausted, the capacity overflows, or a non-zero-size
    /// `T` has alignment above 4096.
    pub fn reserve(&self, additional: usize) {
        self.try_reserve(additional).unwrap_or_else(|e| fail(e));
    }

    /// Makes room for at least `additional` more elements without invalidating
    /// references into the initialized prefix. On failure the vector is
    /// unchanged (the stack may still have reclaimed unreachable space above
    /// it).
    pub fn try_reserve(&self, additional: usize) -> Result<(), AllocError> {
        let needed = self.len.get().checked_add(additional).ok_or(OVERFLOW)?;
        let cap = self.cap.get();
        if needed <= cap {
            return Ok(());
        }
        // Amortised doubling, falling back to the exact size when the
        // doubled buffer does not fit.
        let doubled = cap.saturating_mul(2).max(4);
        if doubled > needed && self.try_grow_to(doubled).is_ok() {
            return Ok(());
        }
        self.try_grow_to(needed)
    }

    /// The number of elements the vector can hold without growing.
    pub fn capacity(&self) -> usize {
        self.cap.get()
    }

    /// Appends `value` without invalidating existing element references or
    /// iterators.
    ///
    /// # Panics
    /// If the stack is exhausted, the capacity overflows, or a non-zero-size
    /// `T` has alignment above 4096.
    pub fn push(&self, value: T) {
        if let Err((_, e)) = self.try_push(value) {
            fail(e);
        }
    }

    /// Appends `value` without invalidating existing element references or
    /// iterators, or returns it with the reason it did not fit.
    pub fn try_push(&self, value: T) -> Result<(), (T, AllocError)> {
        let len = self.len.get();
        if len == self.cap.get() {
            if let Err(e) = self.try_reserve(1) {
                return Err((value, e));
            }
        }
        self.check();
        // SAFETY: `len < cap`: the slot is inside our buffer and holds no
        // value. For a zero-size `T` the pointer is dangling and aligned.
        unsafe { self.ptr.get().as_ptr().add(len).write(value) };
        self.len.set(len + 1);
        Ok(())
    }

    /// Removes and returns the last element.
    pub fn pop(&mut self) -> Option<T> {
        let len = self.len.get();
        if len == 0 {
            return None;
        }
        self.check();
        self.len.set(len - 1);
        // SAFETY: the slot was initialised and is now outside `[0, len)`.
        Some(unsafe { self.ptr.get().as_ptr().add(len - 1).read() })
    }

    /// Appends a copy of every element of `src` without invalidating
    /// existing element references or iterators.
    ///
    /// # Panics
    /// If the stack is exhausted, the capacity overflows, or a non-zero-size
    /// `T` has alignment above 4096.
    pub fn extend_from_slice(&self, src: &[T])
    where
        T: Copy,
    {
        self.reserve(src.len());
        self.check();
        let len = self.len.get();
        // SAFETY: `reserve` made room for `src.len()` more values; `src`
        // cannot overlap spare capacity that only `self` can reach.
        unsafe {
            core::ptr::copy_nonoverlapping(
                src.as_ptr(),
                self.ptr.get().as_ptr().add(len),
                src.len(),
            )
        };
        self.len.set(len + src.len());
    }

    /// Inserts `value` at `index`, shifting later elements up.
    ///
    /// # Panics
    /// If `index > len`, if the stack is exhausted, the capacity overflows,
    /// or a non-zero-size `T` has alignment above 4096.
    pub fn insert(&mut self, index: usize, value: T) {
        let len = self.len.get();
        assert!(
            index <= len,
            "ordstack: insertion index (is {index}) should be <= len (is {len})"
        );
        self.reserve(1);
        self.check();
        // SAFETY: capacity is at least `len + 1`. The tail `[index, len)` is
        // moved up by one, then the freed slot is written.
        unsafe {
            let p = self.ptr.get().as_ptr().add(index);
            core::ptr::copy(p, p.add(1), len - index);
            p.write(value);
        }
        self.len.set(len + 1);
    }

    /// Removes and returns the element at `index`, shifting later elements
    /// down.
    ///
    /// # Panics
    /// If `index >= len`.
    pub fn remove(&mut self, index: usize) -> T {
        let len = self.len.get();
        assert!(
            index < len,
            "ordstack: removal index (is {index}) should be < len (is {len})"
        );
        self.check();
        // SAFETY: `index < len`. The value is read out, then the tail is
        // moved down over its slot.
        unsafe {
            let p = self.ptr.get().as_ptr().add(index);
            let value = p.read();
            core::ptr::copy(p.add(1), p, len - index - 1);
            self.len.set(len - 1);
            value
        }
    }

    /// Removes and returns the element at `index`, replacing it with the
    /// last element.
    ///
    /// # Panics
    /// If `index >= len`.
    pub fn swap_remove(&mut self, index: usize) -> T {
        let len = self.len.get();
        assert!(
            index < len,
            "ordstack: swap_remove index (is {index}) should be < len (is {len})"
        );
        self.check();
        // SAFETY: `index < len`. The value is read out and the last element
        // is moved into its slot (a no-op move when they are the same slot).
        unsafe {
            let base = self.ptr.get().as_ptr();
            let value = base.add(index).read();
            core::ptr::copy(base.add(len - 1), base.add(index), 1);
            self.len.set(len - 1);
            value
        }
    }

    /// Shortens the vector to `len` elements, dropping the rest back to
    /// front. Does nothing if `len` is not below the current length. If a
    /// destructor panics the remaining elements are still dropped.
    pub fn truncate(&mut self, len: usize) {
        let current_len = self.len.get();
        if len >= current_len {
            return;
        }
        self.check();
        let n = current_len - len;
        self.len.set(len);
        // SAFETY: `[len, len + n)` were initialised and are now outside the
        // vector's length.
        unsafe { drop_back_to_front(self.ptr.get().as_ptr().add(len), n) };
    }

    /// Removes every element, dropping them back to front. Keeps the
    /// capacity.
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    /// Keeps the elements for which `keep` returns `true`, preserving their
    /// order. Removed elements are dropped front to back, as they are
    /// visited. If `keep` or a destructor panics, every element is still
    /// owned by the vector or dropped exactly once.
    pub fn retain(&mut self, mut keep: impl FnMut(&T) -> bool) {
        /// On exit, including by panic, closes the gap and restores `len`.
        struct Gap<'a, 'v, 'f, T> {
            v: &'a mut SVec<'v, 'f, T>,
            /// Elements `[0, write)` are kept; `[read, orig)` are unvisited.
            read: usize,
            write: usize,
            orig: usize,
        }
        impl<T> Drop for Gap<'_, '_, '_, T> {
            fn drop(&mut self) {
                let rest = self.orig - self.read;
                // SAFETY: `[read, orig)` are initialised and `[write, read)`
                // holds no values, so the rest moves down over the gap.
                unsafe {
                    let p = self.v.ptr.get().as_ptr();
                    core::ptr::copy(p.add(self.read), p.add(self.write), rest);
                }
                self.v.len.set(self.write + rest);
            }
        }

        self.check();
        let orig = self.len.replace(0);
        let mut g = Gap {
            v: self,
            read: 0,
            write: 0,
            orig,
        };
        while g.read < g.orig {
            let base = g.v.ptr.get().as_ptr();
            // SAFETY: `read < orig`, so the slot holds an unvisited value.
            let cur = unsafe { base.add(g.read) };
            // If `keep` panics the value is still in the unvisited range.
            // SAFETY: as above; the reference ends before the value moves.
            let kept = keep(unsafe { &*cur });
            g.read += 1;
            if kept {
                // SAFETY: slot `write <= read - 1` holds no value (or is
                // `cur` itself); the value moves there.
                unsafe { core::ptr::copy(cur, base.add(g.write), 1) };
                g.write += 1;
            } else {
                // `read` is already past the value, so a panicking
                // destructor cannot cause a second drop.
                // SAFETY: the value is initialised and never used again.
                unsafe { core::ptr::drop_in_place(cur) };
            }
        }
    }

    /// Gives unused capacity back to the stack. Never moves the buffer.
    pub fn shrink_to_fit(&mut self) {
        if !self.allocated() {
            return;
        }
        let st = self.frame.st;
        if self.len.get() == 0 {
            st.reset_to(self.frame.end);
            self.ptr.set(NonNull::dangling());
            self.cap.set(0);
        } else {
            let len = self.len.get();
            st.reset_to(self.start_off() + len * size_of::<T>());
            self.cap.set(len);
        }
    }

    /// Freeze and descend: a frame above this vector. The vector cannot be
    /// used until the frame and everything allocated through it are gone.
    /// The new frame starts at the end of the vector's capacity, so the
    /// vector keeps its spare room.
    ///
    /// # Examples
    /// ```
    /// use ordstack::{SVec, Stack};
    ///
    /// let mut st = Stack::with_reserve(1 << 16);
    /// let mut f = st.frame();
    /// let mut v: SVec<u32> = SVec::new_in(&mut f);
    /// v.push(1);
    /// {
    ///     let mut up = v.above();
    ///     let b = up.alloc(2u32);
    ///     assert_eq!(*b, 2);
    /// }
    /// v.push(3);
    /// assert_eq!(&*v, [1, 3]);
    /// ```
    pub fn above(&mut self) -> Frame<'_> {
        let end = self.end_off();
        self.frame.st.reset_to(end);
        Frame {
            st: self.frame.st,
            end,
        }
    }

    /// Like [`above`](Self::above), but the elements stay readable and
    /// writable. The vector still cannot grow or shrink.
    ///
    /// # Examples
    /// ```
    /// use ordstack::{SVec, Stack};
    ///
    /// let mut st = Stack::with_reserve(1 << 16);
    /// let mut f = st.frame();
    /// let mut v: SVec<u32> = SVec::from_iter_in([1, 2, 3], &mut f);
    /// let (elems, mut up) = v.split_above();
    /// let copy = up.alloc_slice_copy(elems);
    /// elems[0] = 10;
    /// assert_eq!(&*copy, [1, 2, 3]);
    /// assert_eq!(elems, [10, 2, 3]);
    /// ```
    pub fn split_above(&mut self) -> (&mut [T], Frame<'_>) {
        self.check();
        let end = self.end_off();
        self.frame.st.reset_to(end);
        let frame = Frame {
            st: self.frame.st,
            end,
        };
        // SAFETY: `len` initialised values. The frame only hands out memory
        // at or above `end`, which is disjoint from the elements.
        (
            unsafe { core::slice::from_raw_parts_mut(self.ptr.get().as_ptr(), self.len.get()) },
            frame,
        )
    }

    fn into_parts(self) -> (&'v mut Frame<'f>, NonNull<T>, usize, bool) {
        let allocated = self.allocated();
        let this = ManuallyDrop::new(self);
        // SAFETY: `this` is never used or dropped again, so this moves the
        // unique reference out of it.
        let frame = unsafe { core::ptr::read(&this.frame) };
        (frame, this.ptr.get(), this.len.get(), allocated)
    }

    /// Freezes the vector in place, without copying. Spare capacity goes
    /// back to the stack, and the frame can be used again.
    ///
    /// # Examples
    /// ```
    /// use ordstack::{SVec, Stack};
    ///
    /// let mut st = Stack::with_reserve(1 << 16);
    /// let mut f = st.frame();
    /// let mut v: SVec<u32> = SVec::new_in(&mut f);
    /// v.extend([1, 2, 3]);
    /// let s = v.freeze();
    /// let b = f.alloc(4u32); // sits beside `s`
    /// assert_eq!((&*s, *b), (&[1, 2, 3][..], 4));
    /// ```
    pub fn freeze(self) -> SSlice<'f, T> {
        self.check();
        let (frame, ptr, len, allocated) = self.into_parts();
        if allocated {
            let st = frame.st;
            if len == 0 {
                st.reset_to(frame.end);
                return SSlice::empty();
            }
            let new_end = st.offset_of(ptr.cast()) + len * size_of::<T>();
            st.reset_to(new_end);
            frame.end = new_end;
        }
        // SAFETY: `len` initialised values now lie below `frame.end`, which
        // does not decrease while `'f` is alive. When nothing is allocated,
        // `len * size_of::<T>() == 0` and `ptr` is dangling and aligned.
        unsafe { SSlice::from_raw_parts(ptr, len) }
    }

    /// Moves the elements into a frozen block of `dst` with one bitwise
    /// copy. This vector's buffer is left to be reclaimed.
    ///
    /// `dst` cannot be the frame this vector holds, nor any frame beneath
    /// it: those stay mutably borrowed for as long as the vector exists. In
    /// practice `dst` is a frame of another stack. To freeze onto the
    /// vector's own frame use [`freeze`](Self::freeze).
    ///
    /// # Panics
    /// If `dst`'s stack is exhausted, or if the block has non-zero size and
    /// `T` needs alignment above 4096. The vector is then dropped with its
    /// elements during unwinding.
    ///
    /// # Examples
    /// ```
    /// use ordstack::{SVec, Stack};
    ///
    /// let mut results = Stack::with_reserve(1 << 16);
    /// let mut scratch = Stack::with_reserve(1 << 16);
    /// let (mut out, mut tmp) = (results.frame(), scratch.frame());
    /// let mut v: SVec<u32> = SVec::new_in(&mut tmp);
    /// v.extend([3, 1, 2]);
    /// v.sort();
    /// let s = v.freeze_into(&mut out);
    /// assert_eq!(&*s, [1, 2, 3]);
    /// ```
    ///
    /// The vector's own frame is rejected by the borrow checker:
    ///
    /// ```compile_fail,E0499
    /// use ordstack::{SVec, Stack};
    ///
    /// let mut st = Stack::with_reserve(1 << 16);
    /// let mut f = st.frame();
    /// let v: SVec<u32> = SVec::new_in(&mut f);
    /// let s = v.freeze_into(&mut f); // error: `f` is already mutably borrowed by `v`
    /// ```
    pub fn freeze_into<'g>(self, dst: &mut Frame<'g>) -> SSlice<'g, T> {
        self.check();
        let len = self.len.get();
        let p = dst.try_alloc_array::<T>(len).unwrap_or_else(|e| fail(e));
        // SAFETY: `p` is a fresh block for `len` values and cannot overlap
        // our buffer. Setting `self.len = 0` hands ownership to the copy.
        unsafe {
            core::ptr::copy_nonoverlapping(self.ptr.get().as_ptr(), p.as_ptr(), len);
            self.len.set(0);
            SSlice::from_raw_parts(p, len)
        }
    }
}

impl<T> Deref for SVec<'_, '_, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.check();
        // SAFETY: `len` initialised values.
        unsafe { core::slice::from_raw_parts(self.ptr.get().as_ptr(), self.len.get()) }
    }
}

impl<T> DerefMut for SVec<'_, '_, T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.check();
        // SAFETY: `len` initialised values; unique owner.
        unsafe { core::slice::from_raw_parts_mut(self.ptr.get().as_ptr(), self.len.get()) }
    }
}

impl<T> Extend<T> for SVec<'_, '_, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        // A hint that does not fit is not an error; `push` decides.
        let _ = self.try_reserve(iter.size_hint().0);
        for x in iter {
            self.push(x);
        }
    }
}

impl<T> Drop for SVec<'_, '_, T> {
    fn drop(&mut self) {
        self.check();
        let len = self.len.replace(0);
        // SAFETY: we own `len` initialised values and `self.len` is now 0.
        // The buffer itself is reclaimed when the frame is next used.
        unsafe { drop_back_to_front(self.ptr.get().as_ptr(), len) };
    }
}

crate::macros::deref_traits!(['v, 'f, T] SVec<'v, 'f, T>, [T]);

impl<'a, T> IntoIterator for &'a SVec<'_, '_, T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;
    fn into_iter(self) -> core::slice::Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut SVec<'_, '_, T> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;
    fn into_iter(self) -> core::slice::IterMut<'a, T> {
        self.iter_mut()
    }
}

/// By-value iterator over an [`SVec`]. It keeps the vector's frame
/// borrowed, so the buffer is not frozen: once the iterator is gone, its
/// space is reclaimed lazily, as for a dropped vector. Elements not yet
/// yielded are dropped back to front.
pub struct VecIntoIter<'v, 'f, T> {
    // Declared first, so it is dropped before the frame borrow ends.
    inner: IntoIter<'v, T>,
    _frame: &'v mut Frame<'f>,
}

impl<'v, 'f, T> IntoIterator for SVec<'v, 'f, T> {
    type Item = T;
    type IntoIter = VecIntoIter<'v, 'f, T>;
    fn into_iter(self) -> VecIntoIter<'v, 'f, T> {
        self.check();
        let (frame, ptr, len, _) = self.into_parts();
        // SAFETY: `ptr` holds `len` initialised values in the vector's buffer,
        // which lies at or above `frame.end`. Only a reset by this frame or by
        // an object beneath it could reclaim the buffer, and the iterator holds
        // `frame` exclusively for `'v`, so neither can run while `inner`
        // exists. With nothing allocated, or for a zero-size `T`, `ptr` is
        // dangling and aligned and the block has zero size.
        let slice = unsafe { SSlice::<'v, T>::from_raw_parts(ptr, len) };
        VecIntoIter {
            inner: slice.into_iter(),
            _frame: frame,
        }
    }
}

impl<T> Iterator for VecIntoIter<'_, '_, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> DoubleEndedIterator for VecIntoIter<'_, '_, T> {
    fn next_back(&mut self) -> Option<T> {
        self.inner.next_back()
    }
}

impl<T> ExactSizeIterator for VecIntoIter<'_, '_, T> {}
