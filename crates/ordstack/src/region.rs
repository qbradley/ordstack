//! The operating-system mapping behind a stack. The only module that calls
//! `libc`. It knows nothing about blocks.

use core::ptr::NonNull;

use crate::error::{AllocError, AllocErrorKind};

/// Largest supported block alignment. The region's base is aligned to it.
pub(crate) const MAX_ALIGN: usize = 4096;

/// A reserved, readable and writable address range.
pub(crate) struct Region {
    base: NonNull<u8>,
    len: usize,
}

impl Region {
    /// Reserves `bytes` rounded up to whole pages. Zero bytes maps nothing.
    pub(crate) fn reserve(bytes: usize) -> Result<Region, AllocError> {
        let page = os::page_size();
        let len = bytes
            .checked_add(page - 1)
            .map(|b| b & !(page - 1))
            .filter(|&b| b <= isize::MAX as usize - page)
            .ok_or(AllocError::new(AllocErrorKind::CapacityOverflow))?;
        if len == 0 {
            let base = NonNull::new(core::ptr::without_provenance_mut(MAX_ALIGN)).unwrap();
            return Ok(Region { base, len: 0 });
        }
        let base = os::map(len).ok_or(AllocError::new(AllocErrorKind::MapFailed))?;
        // Every block alignment relies on this (SOUNDNESS.md, H6); it is
        // checked in every build rather than assumed of the platform.
        assert_eq!(
            base.as_ptr() as usize % MAX_ALIGN,
            0,
            "ordstack: the operating system returned a mapping not aligned to {MAX_ALIGN}"
        );
        Ok(Region { base, len })
    }

    pub(crate) fn base(&self) -> NonNull<u8> {
        self.base
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Returns the whole pages at or above offset `from` to the OS. The range
    /// stays mapped; its contents become unspecified.
    pub(crate) fn release_pages(&self, from: usize) {
        let page = os::page_size();
        let Some(start) = from.checked_add(page - 1).map(|b| b & !(page - 1)) else {
            return;
        };
        if start >= self.len {
            return;
        }
        // SAFETY: `start < len`, so the range lies inside our mapping.
        // `start` is a multiple of the page size and the base is page
        // aligned because `mmap` returned it; were it not, `madvise` would
        // fail and discard nothing.
        unsafe { os::discard(self.base.add(start), self.len - start) };
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        if self.len > 0 {
            // SAFETY: `base`/`len` are exactly what `os::map` returned.
            unsafe { os::unmap(self.base, self.len) };
        }
    }
}

#[cfg(all(unix, not(miri)))]
mod os {
    use core::ptr::NonNull;

    pub(super) fn page_size() -> usize {
        // SAFETY: `sysconf` has no preconditions.
        let p = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        (p.max(1) as usize).max(super::MAX_ALIGN)
    }

    pub(super) fn map(len: usize) -> Option<NonNull<u8>> {
        // SAFETY: an anonymous private mapping at a kernel-chosen address
        // touches no existing memory.
        let p = unsafe {
            libc::mmap(
                core::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                -1,
                0,
            )
        };
        if p == libc::MAP_FAILED {
            None
        } else {
            NonNull::new(p.cast())
        }
    }

    /// # Safety
    /// `base`/`len` must be a live mapping returned by [`map`].
    pub(super) unsafe fn unmap(base: NonNull<u8>, len: usize) {
        // SAFETY: guaranteed by the caller.
        unsafe { libc::munmap(base.as_ptr().cast(), len) };
    }

    /// # Safety
    /// The page-aligned range must lie inside a live mapping from [`map`],
    /// and nothing may rely on its contents.
    pub(super) unsafe fn discard(start: NonNull<u8>, len: usize) {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const ADVICE: libc::c_int = libc::MADV_DONTNEED;
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        const ADVICE: libc::c_int = libc::MADV_FREE;
        // SAFETY: guaranteed by the caller. A failure only means the pages
        // were not returned, which is not an error for us.
        unsafe { libc::madvise(start.as_ptr().cast(), len, ADVICE) };
    }
}

// Miri rejects MAP_NORESERVE, so the region is a heap block there.
#[cfg(miri)]
mod os {
    use core::ptr::NonNull;
    use std::alloc::{Layout, alloc, dealloc};

    pub(super) fn page_size() -> usize {
        super::MAX_ALIGN
    }

    pub(super) fn map(len: usize) -> Option<NonNull<u8>> {
        let layout = Layout::from_size_align(len, super::MAX_ALIGN).ok()?;
        // SAFETY: `len > 0` is checked by the caller.
        NonNull::new(unsafe { alloc(layout) })
    }

    /// # Safety
    /// `base`/`len` must be a live block returned by [`map`].
    pub(super) unsafe fn unmap(base: NonNull<u8>, len: usize) {
        let layout = Layout::from_size_align(len, super::MAX_ALIGN).unwrap();
        // SAFETY: guaranteed by the caller.
        unsafe { dealloc(base.as_ptr(), layout) };
    }

    /// # Safety
    /// None under Miri: this is a no-op.
    pub(super) unsafe fn discard(_start: NonNull<u8>, _len: usize) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AllocErrorKind;

    #[test]
    fn reserve_is_writable_end_to_end() {
        let r = Region::reserve(1 << 16).unwrap();
        assert!(r.len() >= 1 << 16);
        assert_eq!(r.base().as_ptr() as usize % MAX_ALIGN, 0);
        // SAFETY: both offsets are inside the region just reserved.
        unsafe {
            r.base().as_ptr().write(1);
            r.base().as_ptr().add(r.len() - 1).write(2);
            assert_eq!(r.base().as_ptr().read(), 1);
        }
    }

    #[test]
    fn reserve_rounds_up_to_a_page() {
        let r = Region::reserve(1).unwrap();
        assert!(r.len() >= MAX_ALIGN);
        assert_eq!(r.len() % MAX_ALIGN, 0);
    }

    #[test]
    fn zero_reserve_maps_nothing() {
        let r = Region::reserve(0).unwrap();
        assert_eq!(r.len(), 0);
        r.release_pages(0);
    }

    #[test]
    fn absurd_reserve_is_an_error() {
        let e = Region::reserve(usize::MAX).err().unwrap();
        assert_eq!(e.kind(), AllocErrorKind::CapacityOverflow);
        let e = Region::reserve(isize::MAX as usize).err().unwrap();
        assert!(matches!(
            e.kind(),
            AllocErrorKind::MapFailed | AllocErrorKind::CapacityOverflow
        ));
    }

    #[test]
    fn released_pages_stay_usable() {
        let r = Region::reserve(1 << 16).unwrap();
        // SAFETY: offsets are inside the region.
        unsafe { r.base().as_ptr().add(8192).write(7) };
        r.release_pages(1);
        // SAFETY: as above; the page is still mapped after being discarded.
        unsafe { r.base().as_ptr().add(8192).write(9) };
        r.release_pages(r.len() + 5);
    }
}
