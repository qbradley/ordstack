//! [`FrameAlloc`]: a frame as an [`allocator_api2`] allocator.

use crate::frame::Frame;
use crate::stack::Stack;
use allocator_api2::alloc::{AllocError, Allocator};
use core::alloc::Layout;
use core::ptr::NonNull;

/// An allocator on the top of the stack above a frame, for collections
/// generic over [`allocator_api2::alloc::Allocator`]: `allocator_api2`'s
/// `Vec`, `hashbrown`'s maps and sets, and others. Requires the
/// `allocator-api2` feature.
///
/// Made by [`Frame::allocator`]. Any number of collections share one
/// through `&FrameAlloc`, or one collection owns it. The block on top of the
/// stack grows, shrinks and is freed in place; a block that is not on top is
/// copied when it grows, and its old space comes back only with the rest of
/// the adapter's memory: when the frame is next used after the adapter and
/// everything allocated through it are gone.
///
/// A failed allocation is reported as [`AllocError`] to the collection.
/// Collections turn that into an abort in their infallible methods
/// (`push`, `insert`, …), as `std`'s do; use their `try_reserve` to handle
/// it.
pub struct FrameAlloc<'v, 'f> {
    frame: &'v mut Frame<'f>,
}

impl<'f> Frame<'f> {
    /// An allocator for collections generic over
    /// [`allocator_api2::alloc::Allocator`]. This frame is frozen while the
    /// allocator, or anything allocated through it, is alive. Requires the
    /// `allocator-api2` feature.
    ///
    /// # Examples
    /// ```
    /// use allocator_api2::vec::Vec;
    /// use ordstack::Stack;
    ///
    /// let mut st = Stack::with_reserve(1 << 16);
    /// let mut f = st.frame();
    /// let a = f.allocator();
    /// let mut names: Vec<&str, _> = Vec::new_in(&a);
    /// let mut lens: Vec<usize, _> = Vec::new_in(&a); // both grow at once
    /// for s in ["fig", "pear"] {
    ///     names.push(s);
    ///     lens.push(s.len());
    /// }
    /// assert_eq!(lens, [3, 4]);
    /// ```
    pub fn allocator(&mut self) -> FrameAlloc<'_, 'f> {
        // `&mut self` proves nothing above `end` is reachable (invariant I).
        self.st.reset_to(self.end);
        FrameAlloc { frame: self }
    }
}

impl FrameAlloc<'_, '_> {
    fn st(&self) -> &Stack {
        self.frame.st
    }

    /// A fresh block at the top.
    fn bump(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let st = self.st();
        let start = st
            .try_bump(layout.size(), layout.align())
            .map_err(|_| AllocError)?;
        Ok(NonNull::slice_from_raw_parts(
            st.ptr_at(start),
            layout.size(),
        ))
    }

    /// The offset of `p` when the block `[p, p + size)` ends at the top.
    /// Never for a zero-size block: it occupies nothing, and its pointer may
    /// not be in the region at all (allocator-api2's `Box` deallocates
    /// `NonNull::dangling()` for zero-size values).
    fn top_block(&self, p: NonNull<u8>, size: usize) -> Option<usize> {
        if size == 0 {
            return None;
        }
        let st = self.st();
        let off = st.offset_of(p);
        (off + size == st.extent()).then_some(off)
    }
}

fn aligned(p: NonNull<u8>, align: usize) -> bool {
    p.as_ptr().addr() & (align - 1) == 0
}

// SAFETY: blocks are disjoint ranges of the stack's region above the
// frame's end, and stay valid until they are deallocated or the adapter and
// every borrow of it are gone; no reset can happen before that, because the
// adapter holds the frame mutably. `FrameAlloc` is not `Clone`, and copies
// of `&FrameAlloc` all name the same adapter. See SOUNDNESS.md, "The
// allocator adapter".
unsafe impl Allocator for FrameAlloc<'_, '_> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.bump(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        if let Some(off) = self.top_block(ptr, layout.size()) {
            // The block is on top: nothing live lies above its start.
            self.st().reset_to(off);
        }
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old: Layout,
        new: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        debug_assert!(new.size() >= old.size());
        if aligned(ptr, new.align()) {
            if let Some(off) = self.top_block(ptr, old.size()) {
                let new_end = off.checked_add(new.size()).ok_or(AllocError)?;
                self.st()
                    .try_extend(off + old.size(), new_end)
                    .map_err(|_| AllocError)?;
                return Ok(NonNull::slice_from_raw_parts(ptr, new.size()));
            }
        }
        let fresh = self.bump(new)?;
        // SAFETY: the caller guarantees `ptr` is a live block of `old.size()`
        // bytes from this adapter; `fresh` was just bumped above every live
        // block, so the two do not overlap. The old block is not on top (or
        // is misaligned for `new`) and keeps its space until reclamation.
        unsafe {
            core::ptr::copy_nonoverlapping(ptr.as_ptr(), fresh.cast::<u8>().as_ptr(), old.size())
        };
        Ok(fresh)
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old: Layout,
        new: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        // SAFETY: the caller's contract for `grow_zeroed` is `grow`'s. The
        // result holds `new.size() >= old.size()` bytes, so the zeroed range
        // `[old.size(), new.size())` lies inside it.
        unsafe {
            let p = self.grow(ptr, old, new)?;
            p.cast::<u8>()
                .as_ptr()
                .add(old.size())
                .write_bytes(0, new.size() - old.size());
            Ok(p)
        }
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old: Layout,
        new: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        debug_assert!(new.size() <= old.size());
        if aligned(ptr, new.align()) {
            if let Some(off) = self.top_block(ptr, old.size()) {
                // On top: give back the tail; nothing live lies above it.
                self.st().reset_to(off + new.size());
            }
            return Ok(NonNull::slice_from_raw_parts(ptr, new.size()));
        }
        let fresh = self.bump(new)?;
        // SAFETY: as in `grow`; `new.size() <= old.size()` bytes are copied.
        unsafe {
            core::ptr::copy_nonoverlapping(ptr.as_ptr(), fresh.cast::<u8>().as_ptr(), new.size())
        };
        Ok(fresh)
    }
}
