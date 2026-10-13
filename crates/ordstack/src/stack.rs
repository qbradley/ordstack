//! [`Stack`] and the primitives that move `top`. This is the only module
//! that writes `top`.

use core::cell::Cell;
use core::fmt;
use core::ptr::NonNull;

use crate::debug;
use crate::error::{AllocError, AllocErrorKind, fail};
use crate::region::{MAX_ALIGN, Region};

/// Address space reserved by [`Stack::new`].
#[cfg(target_pointer_width = "64")]
pub const DEFAULT_RESERVE: usize = 4 << 30;
/// Address space reserved by [`Stack::new`].
#[cfg(not(target_pointer_width = "64"))]
pub const DEFAULT_RESERVE: usize = 64 << 20;

/// One contiguous address range with a single bump offset.
///
/// A stack hands out memory only through a [`Frame`](crate::Frame). It is
/// neither `Send` nor `Sync`.
pub struct Stack {
    region: Region,
    /// Every live block lies in `[0, top)`.
    top: Cell<usize>,
    high: Cell<usize>,
}

impl Stack {
    /// Reserves [`DEFAULT_RESERVE`] bytes of address space. Pages are
    /// committed by the kernel when first touched.
    ///
    /// # Panics
    /// If the mapping fails.
    pub fn new() -> Stack {
        Stack::with_reserve(DEFAULT_RESERVE)
    }

    /// Reserves `bytes`, rounded up to whole pages.
    ///
    /// # Panics
    /// If the mapping fails.
    pub fn with_reserve(bytes: usize) -> Stack {
        Stack::try_with_reserve(bytes).unwrap_or_else(|e| fail(e))
    }

    /// Reserves `bytes`, rounded up to whole pages.
    pub fn try_with_reserve(bytes: usize) -> Result<Stack, AllocError> {
        let region = Region::reserve(bytes)?;
        debug::register(region.base().as_ptr() as usize, region.len());
        Ok(Stack {
            region,
            top: Cell::new(0),
            high: Cell::new(0),
        })
    }

    /// Reclaims everything on the stack now. Taking `&mut self` proves that
    /// no frame, container or block of this stack is still alive.
    pub fn reset(&mut self) {
        self.reset_to(0);
    }

    /// Reclaims everything and returns the stack's pages to the OS. The
    /// reservation itself is kept.
    pub fn trim(&mut self) {
        self.reset_to(0);
        self.release_above(0);
    }

    /// Size of the reservation in bytes.
    pub fn capacity(&self) -> usize {
        self.region.len()
    }

    /// Bytes currently in use, including space that will be reclaimed lazily.
    pub fn extent(&self) -> usize {
        self.top.get()
    }

    /// Largest [`extent`](Self::extent) reached so far.
    pub fn high_water(&self) -> usize {
        self.high.get()
    }

    fn set_top(&self, top: usize) {
        self.top.set(top);
        if top > self.high.get() {
            self.high.set(top);
        }
        debug::set_top(self.region.base().as_ptr() as usize, top);
    }

    /// Reset on reacquire. The caller holds `&mut` to the object whose end
    /// is `off`, so by invariant I nothing above `off` is reachable.
    pub(crate) fn reset_to(&self, off: usize) {
        debug_assert!(
            off <= self.top.get(),
            "ordstack: internal error: reset above top"
        );
        if self.top.get() > off {
            self.set_top(off);
        }
    }

    /// Allocates an aligned block at `top`; returns its start offset.
    pub(crate) fn try_bump(&self, size: usize, align: usize) -> Result<usize, AllocError> {
        debug_assert!(align.is_power_of_two());
        if align > MAX_ALIGN {
            return Err(AllocError::new(AllocErrorKind::AlignTooLarge));
        }
        let overflow = AllocError::new(AllocErrorKind::CapacityOverflow);
        let start = self.top.get().checked_add(align - 1).ok_or(overflow)? & !(align - 1);
        let end = start.checked_add(size).ok_or(overflow)?;
        if end > self.region.len() {
            return Err(AllocError::new(AllocErrorKind::Exhausted));
        }
        self.set_top(end);
        Ok(start)
    }

    /// Grows the top block, which currently ends at `old_end`, to `new_end`.
    ///
    /// Caller contract: the block ending at `old_end` must be the top block.
    pub(crate) fn try_extend(&self, old_end: usize, new_end: usize) -> Result<(), AllocError> {
        assert_eq!(
            old_end,
            self.top.get(),
            "ordstack: internal error: grow of a non-top block"
        );
        assert!(
            new_end >= old_end,
            "ordstack: internal error: extend shrinks the block"
        );
        if new_end > self.region.len() {
            return Err(AllocError::new(AllocErrorKind::Exhausted));
        }
        self.set_top(new_end);
        Ok(())
    }

    pub(crate) fn ptr_at(&self, off: usize) -> NonNull<u8> {
        assert!(
            off <= self.region.len(),
            "ordstack: internal error: offset outside the region"
        );
        // SAFETY: `off <= len`, so the result is inside or one past the region.
        unsafe { self.region.base().add(off) }
    }

    /// `p` must point into this stack's region.
    pub(crate) fn offset_of(&self, p: NonNull<u8>) -> usize {
        let off = (p.as_ptr() as usize).wrapping_sub(self.region.base().as_ptr() as usize);
        debug_assert!(off <= self.region.len());
        off
    }

    /// `off` must be at or above `top`. Pages at or above `off` lose their
    /// contents.
    pub(crate) fn release_above(&self, off: usize) {
        debug_assert!(off >= self.top.get());
        self.region.release_pages(off);
    }
}

impl Default for Stack {
    fn default() -> Stack {
        Stack::new()
    }
}

impl fmt::Debug for Stack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stack")
            .field("capacity", &self.capacity())
            .field("extent", &self.extent())
            .field("high_water", &self.high_water())
            .finish()
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        debug::unregister(self.region.base().as_ptr() as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AllocErrorKind;

    #[test]
    fn bump_aligns_and_advances() {
        let st = Stack::with_reserve(1 << 16);
        assert_eq!(st.try_bump(3, 1).unwrap(), 0);
        assert_eq!(st.try_bump(8, 8).unwrap(), 8);
        assert_eq!(st.extent(), 16);
        assert_eq!(st.try_bump(1, 4096).unwrap(), 4096);
        assert_eq!(st.extent(), 4097);
        assert_eq!(st.high_water(), 4097);
    }

    #[test]
    fn failed_bump_changes_nothing() {
        let st = Stack::with_reserve(4096);
        st.try_bump(10, 1).unwrap();
        let cap = st.capacity();
        assert_eq!(
            st.try_bump(cap, 1).unwrap_err().kind(),
            AllocErrorKind::Exhausted
        );
        assert_eq!(
            st.try_bump(1, 8192).unwrap_err().kind(),
            AllocErrorKind::AlignTooLarge
        );
        assert_eq!(
            st.try_bump(usize::MAX, 1).unwrap_err().kind(),
            AllocErrorKind::CapacityOverflow
        );
        assert_eq!(st.extent(), 10);
    }

    #[test]
    fn extend_grows_the_top_block() {
        let st = Stack::with_reserve(4096);
        let start = st.try_bump(16, 8).unwrap();
        st.try_extend(start + 16, start + 64).unwrap();
        assert_eq!(st.extent(), 64);
        let cap = st.capacity();
        assert_eq!(
            st.try_extend(64, cap + 1).unwrap_err().kind(),
            AllocErrorKind::Exhausted
        );
        assert_eq!(st.extent(), 64);
    }

    #[test]
    fn reset_to_lowers_top_only() {
        let st = Stack::with_reserve(4096);
        st.try_bump(100, 1).unwrap();
        st.reset_to(40);
        assert_eq!(st.extent(), 40);
        assert_eq!(st.high_water(), 100);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn shadow_follows_top() {
        let st = Stack::with_reserve(4096);
        let start = st.try_bump(16, 1).unwrap();
        crate::debug::check_live(st.ptr_at(start).as_ptr(), 16);
        st.reset_to(0);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::debug::check_live(st.ptr_at(start).as_ptr(), 16);
        }));
        assert!(r.is_err());
    }

    #[test]
    fn offsets_round_trip() {
        let st = Stack::with_reserve(4096);
        let p = st.ptr_at(24);
        assert_eq!(st.offset_of(p), 24);
    }
}
