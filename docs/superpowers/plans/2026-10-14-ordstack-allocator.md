# ordstack allocator adapter (issue #4) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Behind a cargo feature `allocator-api2`, `Frame::allocator()` returns a `FrameAlloc` that implements `allocator_api2::alloc::Allocator`, so `allocator_api2::vec::Vec` and `hashbrown` maps run on a frame.

**Architecture:** One new module, `crates/ordstack/src/alloc.rs`, compiled only with the feature. `FrameAlloc<'v, 'f>` holds `&'v mut Frame<'f>` (as `SVec` does) and allocates through the stack's existing `&self` methods (`try_bump`, `try_extend`, `reset_to`, `ptr_at`, `offset_of`, `extent`). The block on top grows, shrinks and is freed in place; any other block is copied when it grows. Three new `unsafe` blocks (two copies and one zeroing) and one `unsafe impl`.

**Tech Stack:** Rust 2024, MSRV 1.85, `allocator-api2 0.2` (optional), `hashbrown 0.17` (dev only), `trybuild`, Miri (Stacked and Tree Borrows), `cargo fuzz`, `scripts/check.sh`.

**Spec:** `docs/superpowers/specs/2026-10-14-ordstack-allocator-design.md`.

## Global Constraints

- Branch `allocator-api2` in `/home/ubuntu/ordstack` (exists; holds the spec and this plan, and has `origin/main` merged in).
- Feature `allocator-api2`, off by default, = `["dep:allocator-api2"]`; dependency `allocator-api2 = { version = "0.2", optional = true }` (default features). Dev-dependency `hashbrown = { version = "0.17", default-features = false, features = ["allocator-api2", "default-hasher"] }`.
- No existing public signature or behaviour changes; `Frame`'s fields and methods are untouched (spec §2).
- Errors are `allocator_api2::alloc::AllocError`; the adapter never panics on user input (spec §4).
- **Machine rule (hard):** the dev box runs only git, edits, `cargo fmt`, `cargo check`, `cargo metadata`. Every test, clippy, Miri, fuzz and `check.sh` run goes through `.superpowers/trainbox.sh '<cmd>'` (it force-pushes the committed HEAD to the train box and runs there), so commit before each run. Wrap runs in `timeout`.
- Commits end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **A failed allocation inside `push`/`insert`** (not `try_reserve`): allocator-api2's `Vec` calls `handle_alloc_error`, which aborts, as `std::Vec` does. The docs must say so and point at `try_reserve` (Task 5 doc text).
2. **An adapter on a frame from `SVec::above`/`split_above` or `sub()`**: the lower data must be untouched and the lower vector must grow in place again afterwards (Task 1 test `adapter_above_an_svec`, and Task 3's model scope).
3. **Zero-size blocks next to non-zero ones at the top**: freeing or growing one must not release a live neighbour (Task 1 tests `zero_size` and `zero_size_beside_the_top_block`).
4. **Growth that needs a larger alignment while on top** must copy rather than extend misaligned (Task 1 test `grow_and_shrink_change_alignment`).
5. **Two adapters in sequence on one frame** reuse the same memory (Task 1 test `memory_is_reclaimed_when_the_adapter_ends`).

---

### Task 1: The adapter and its runtime tests

**Files:**
- Create: `crates/ordstack/src/alloc.rs`, `crates/ordstack/tests/allocator.rs`
- Modify: `crates/ordstack/Cargo.toml`, `crates/ordstack/src/lib.rs` (module list and re-exports), `Cargo.lock`

**Interfaces:**
- Consumes (existing, `pub(crate)` on `Stack`): `try_bump(&self, size, align) -> Result<usize, ordstack::AllocError>`, `try_extend(&self, old_end, new_end) -> Result<(), _>` (asserts `old_end == top`), `reset_to(&self, off)`, `ptr_at(&self, off) -> NonNull<u8>`, `offset_of(&self, NonNull<u8>) -> usize`; public `Stack::extent(&self) -> usize` (= top). `Frame { pub(crate) st: &'f Stack, pub(crate) end: usize }`.
- Produces: `pub fn Frame::allocator(&mut self) -> FrameAlloc<'_, 'f>`; `pub struct FrameAlloc<'v, 'f>`; `unsafe impl allocator_api2::alloc::Allocator for FrameAlloc<'_, '_>`; re-export `ordstack::FrameAlloc` (feature-gated).

- [ ] **Step 1: Add the feature and dependencies**

In `crates/ordstack/Cargo.toml`, after the `[target.'cfg(unix)'.dependencies]` table:

```toml
[dependencies]
allocator-api2 = { version = "0.2", optional = true }

[features]
## `FrameAlloc`: a frame as an `allocator_api2` allocator.
allocator-api2 = ["dep:allocator-api2"]
```

and in `[dev-dependencies]`:

```toml
hashbrown = { version = "0.17", default-features = false, features = ["allocator-api2", "default-hasher"] }
```

and at the end:

```toml
[package.metadata.docs.rs]
all-features = true
```

Run: `cd /home/ubuntu/ordstack && cargo metadata --format-version 1 >/dev/null && git diff --stat Cargo.lock`
Expected: `Cargo.lock` gains `allocator-api2`, `hashbrown`, `foldhash` (and `equivalent` if pulled).

- [ ] **Step 2: Write the failing tests**

Create `crates/ordstack/tests/allocator.rs`:

```rust
//! `Frame::allocator` and `FrameAlloc`: collections generic over
//! `allocator_api2::alloc::Allocator` on a frame.
#![cfg(feature = "allocator-api2")]

use allocator_api2::alloc::Allocator;
use allocator_api2::vec::Vec as AVec;
use core::alloc::Layout;
use ordstack::{SVec, Stack};
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Counted<'c>(u32, &'c Cell<usize>);

impl Drop for Counted<'_> {
    fn drop(&mut self) {
        self.1.set(self.1.get() + 1);
    }
}

fn addr<T: ?Sized>(p: *const T) -> usize {
    p.cast::<u8>() as usize
}

#[test]
fn vec_push_grow_drop() {
    let drops = Cell::new(0);
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    {
        let a = f.allocator();
        let mut v = AVec::new_in(&a);
        for i in 0..1000 {
            v.push(Counted(i, &drops));
        }
        assert!(v.iter().map(|c| c.0).eq(0..1000));
        v.truncate(10);
        assert_eq!(drops.get(), 990);
    }
    assert_eq!(drops.get(), 1000);
}

#[test]
fn growth_at_the_top_is_in_place() {
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    let a = f.allocator();
    let mut v: AVec<u64, _> = AVec::with_capacity_in(4, &a);
    v.push(1);
    let p = addr(v.as_ptr());
    v.reserve(100);
    assert_eq!(addr(v.as_ptr()), p, "growth on top moved the buffer");
    // The next block starts right after the grown buffer.
    let probe: AVec<u8, _> = AVec::with_capacity_in(1, &a);
    assert_eq!(addr(probe.as_ptr()), p + v.capacity() * 8);
    assert_eq!(v[0], 1);
}

#[test]
fn interleaved_vectors_copy() {
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    let a = f.allocator();
    let mut lo: AVec<u32, _> = AVec::with_capacity_in(1, &a);
    let mut hi: AVec<u32, _> = AVec::with_capacity_in(1, &a);
    lo.push(0);
    hi.push(0);
    let (plo, phi) = (addr(lo.as_ptr()), addr(hi.as_ptr()));
    hi.reserve(10);
    assert_eq!(addr(hi.as_ptr()), phi, "the upper vector moved");
    lo.reserve(10);
    assert_ne!(addr(lo.as_ptr()), plo, "the lower vector grew in place over the upper one");
    for i in 1..100 {
        lo.push(i);
        hi.push(1000 + i);
    }
    assert!(lo.iter().copied().eq(0..100));
    assert!(hi.iter().copied().eq(std::iter::once(0).chain(1001..1100)));
}

#[test]
fn deallocating_the_top_block_reclaims_it() {
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    let a = f.allocator();
    let keep: AVec<u32, _> = AVec::with_capacity_in(3, &a);
    let v: AVec<u32, _> = AVec::with_capacity_in(8, &a);
    let p = addr(v.as_ptr());
    drop(v);
    let w: AVec<u32, _> = AVec::with_capacity_in(8, &a);
    assert_eq!(addr(w.as_ptr()), p);
    // Freeing a block that is not on top gives nothing back.
    drop(keep);
    let x: AVec<u32, _> = AVec::with_capacity_in(1, &a);
    assert_eq!(addr(x.as_ptr()), p + 32);
}

#[test]
fn shrink_at_the_top_releases_the_tail() {
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    let a = f.allocator();
    let mut v: AVec<u32, _> = AVec::with_capacity_in(64, &a);
    v.extend([1, 2, 3, 4]);
    let p = addr(v.as_ptr());
    v.shrink_to_fit();
    assert_eq!(addr(v.as_ptr()), p);
    assert_eq!(v.capacity(), 4);
    let probe: AVec<u8, _> = AVec::with_capacity_in(1, &a);
    assert_eq!(addr(probe.as_ptr()), p + 16);
    assert_eq!(v, [1, 2, 3, 4]);
}

#[test]
fn memory_is_reclaimed_when_the_adapter_ends() {
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    let first = {
        let a = f.allocator();
        let mut v: AVec<u32, _> = AVec::with_capacity_in(16, &a);
        v.push(9);
        let w: AVec<u32, _> = AVec::with_capacity_in(16, &a);
        drop(w);
        addr(v.as_ptr())
    };
    let again = {
        let a = f.allocator();
        let v: AVec<u32, _> = AVec::with_capacity_in(16, &a);
        addr(v.as_ptr())
    };
    assert_eq!(again, first, "a second adapter did not reuse the first one's memory");
    let b = f.alloc(7u32);
    assert_eq!(addr(&*b), first);
}

#[test]
fn adapter_above_an_svec() {
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    let mut lower: SVec<u32> = SVec::new_in(&mut f);
    lower.extend(0..10);
    let p = addr(lower.as_ptr());
    {
        let (elems, mut up) = lower.split_above();
        let a = up.allocator();
        let mut v: AVec<u32, _> = AVec::new_in(&a);
        v.extend(100..200);
        assert!(elems.iter().copied().eq(0..10));
    }
    lower.reserve(1000);
    assert_eq!(addr(lower.as_ptr()), p, "the lower vector did not grow in place");
    assert!(lower.iter().copied().eq(0..10));
}

#[test]
fn hashbrown_map_on_a_frame() {
    let n: u32 = if cfg!(miri) { 100 } else { 10_000 };
    let mut st = Stack::with_reserve(1 << 24);
    let mut f = st.frame();
    let a = f.allocator();
    let mut m = hashbrown::HashMap::new_in(&a);
    let mut log: AVec<u32, _> = AVec::new_in(&a);
    for i in 0..n {
        m.insert(i, i.to_string());
        log.push(i);
    }
    for i in 0..n {
        assert_eq!(m.get(&i).map(String::as_str), Some(&*i.to_string()));
    }
    assert_eq!(m.len(), n as usize);
    assert!(log.iter().copied().eq(0..n));
}

#[test]
fn a_panic_during_extend_leaves_everything_usable() {
    let drops = Cell::new(0);
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    {
        let a = f.allocator();
        let mut v = AVec::new_in(&a);
        let r = catch_unwind(AssertUnwindSafe(|| {
            v.extend((0..100).map(|i| {
                assert!(i != 50, "boom");
                Counted(i, &drops)
            }));
        }));
        assert!(r.is_err());
        assert_eq!(v.len(), 50);
        assert_eq!(drops.get(), 0);
        v.push(Counted(100, &drops));
        drop(v);
        assert_eq!(drops.get(), 51);
    }
    let b = f.alloc(1u8);
    assert_eq!(*b, 1);
}

#[test]
fn errors_change_nothing() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let a = f.allocator();
    let mut v: AVec<u8, _> = AVec::new_in(&a);
    v.extend_from_slice(b"abc");
    let (p, cap) = (addr(v.as_ptr()), v.capacity());
    assert!(v.try_reserve(1 << 30).is_err());
    assert_eq!(&v[..], b"abc");
    assert_eq!((addr(v.as_ptr()), v.capacity()), (p, cap));
    let probe: AVec<u8, _> = AVec::with_capacity_in(1, &a);
    assert_eq!(addr(probe.as_ptr()), p + cap, "a failed growth moved the top");
    assert!(a.allocate(Layout::from_size_align(8, 8192).unwrap()).is_err());
    assert!(a.allocate(Layout::from_size_align(1 << 30, 1).unwrap()).is_err());
}

#[test]
fn zero_size() {
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    let a = f.allocator();
    for align in [1, 8, 4096] {
        let l = Layout::from_size_align(0, align).unwrap();
        let p = a.allocate(l).unwrap();
        assert_eq!(p.len(), 0);
        assert_eq!(addr(p.as_ptr()) % align, 0);
        // SAFETY: `p` was allocated by `a` with layout `l`.
        unsafe { a.deallocate(p.cast(), l) };
    }
    let mut v: AVec<(), _> = AVec::new_in(&a);
    for _ in 0..1000 {
        v.push(());
    }
    assert_eq!(v.len(), 1000);
}

#[test]
fn zero_size_beside_the_top_block() {
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    let a = f.allocator();
    let mut v: AVec<u8, _> = AVec::with_capacity_in(8, &a);
    v.extend_from_slice(b"12345678");
    let z = Layout::from_size_align(0, 1).unwrap();
    let e = a.allocate(z).unwrap();
    assert_eq!(addr(e.as_ptr()), addr(v.as_ptr()) + 8);
    // Freeing the empty block on top must not give back `v`'s bytes.
    // SAFETY: `e` was allocated by `a` with layout `z`.
    unsafe { a.deallocate(e.cast(), z) };
    let probe: AVec<u8, _> = AVec::with_capacity_in(1, &a);
    assert_eq!(addr(probe.as_ptr()), addr(v.as_ptr()) + 8);
    assert_eq!(&v[..], b"12345678");
}

#[test]
fn grow_and_shrink_change_alignment() {
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    let a = f.allocator();
    let l = |size, align| Layout::from_size_align(size, align).unwrap();
    // SAFETY (whole block): every pointer passed back to `a` was returned by
    // it with the layout given, and every access stays inside a live block.
    unsafe {
        // At offset 0 (4096-aligned): growing to a larger alignment stays in place.
        let p = a.allocate(l(16, 1)).unwrap().cast::<u8>();
        let q = a.grow(p, l(16, 1), l(32, 64)).unwrap();
        assert_eq!(addr(q.as_ptr()), addr(p.as_ptr()));
        a.deallocate(q.cast(), l(32, 64));
        // Misaligned on top: one byte first, so the block starts at offset 1.
        let one = a.allocate(l(1, 1)).unwrap();
        let p = a.allocate(l(16, 1)).unwrap().cast::<u8>();
        for i in 0..16 {
            p.as_ptr().add(i).write(i as u8);
        }
        let g = a.grow(p, l(16, 1), l(64, 64)).unwrap().cast::<u8>();
        assert_eq!(addr(g.as_ptr()) % 64, 0);
        assert!((0..16).all(|i| *g.as_ptr().add(i) == i as u8));
        let s = a.shrink(g, l(64, 64), l(8, 128)).unwrap().cast::<u8>();
        assert_eq!(addr(s.as_ptr()) % 128, 0);
        assert!((0..8).all(|i| *s.as_ptr().add(i) == i as u8));
        let z = a.grow_zeroed(s, l(8, 128), l(40, 128)).unwrap().cast::<u8>();
        assert!((0..8).all(|i| *z.as_ptr().add(i) == i as u8));
        assert!((8..40).all(|i| *z.as_ptr().add(i) == 0));
        a.deallocate(z, l(40, 128));
        a.deallocate(one.cast(), l(1, 1));
    }
}
```

- [ ] **Step 3: Run them to see them fail**

Commit (`git add -A crates/ordstack Cargo.lock && git commit -qm "wip: allocator tests"`), then:
Run: `.superpowers/trainbox.sh 'timeout 900 cargo test -p ordstack --features allocator-api2 --test allocator 2>&1 | grep -E "^error|no method" | head -5'`
Expected: `error[E0599]: no method named `allocator` found for struct `Frame``.

- [ ] **Step 4: Write the adapter**

Create `crates/ordstack/src/alloc.rs`:

```rust
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
        Ok(NonNull::slice_from_raw_parts(st.ptr_at(start), layout.size()))
    }

    /// The offset of `p` when the block `[p, p + size)` ends at the top.
    fn top_block(&self, p: NonNull<u8>, size: usize) -> Option<usize> {
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
```

In `crates/ordstack/src/lib.rs`, add `#[cfg(feature = "allocator-api2")] mod alloc;` at the top of the module list (before `mod boxed;`) and `#[cfg(feature = "allocator-api2")] pub use alloc::FrameAlloc;` before `pub use boxed::SBox;`.

- [ ] **Step 5: Run them to see them pass, natively and under Miri**

Commit, then:
Run: `.superpowers/trainbox.sh 'timeout 900 cargo test -p ordstack --all-features --test allocator 2>&1 | grep -E "test result|FAILED|panicked" ; timeout 1500 cargo +nightly miri test -p ordstack --all-features --test allocator 2>&1 | grep -E "test result|FAILED|Undefined|error:" ; MIRIFLAGS=-Zmiri-tree-borrows timeout 1500 cargo +nightly miri test -p ordstack --all-features --test allocator 2>&1 | grep -E "test result|FAILED|Undefined|error:"'`
Expected: three `test result: ok. 14 passed` lines.

- [ ] **Step 6: Lint, both feature sets, and MSRV**

Run: `.superpowers/trainbox.sh 'cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -3 && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -1 && timeout 900 cargo +1.85 test -p ordstack --all-features --test allocator 2>&1 | grep "test result"'`
Expected: no clippy errors, `test result: ok. 14 passed` on 1.85.

- [ ] **Step 7: Commit**

Squash the WIP commits into one: `git reset --soft <BASE> && git commit -m "FrameAlloc: a frame as an allocator-api2 Allocator (issue #4)"` plus the trailer.

---

### Task 2: Compile-fail cases

**Files:**
- Create: `crates/ordstack/tests/ui/fail/20_alloc_while_frame_allocator_lives.rs`, `21_vec_outlives_frame_allocator.rs`, `22_stack_dropped_under_owned_allocator.rs`, `23_frame_allocator_not_send.rs` (each with a `.stderr`), and their twins in `tests/ui/pass/` (generated)
- Modify: `crates/ordstack/tests/ui.rs` (doc comment: the command gains `--all-features`)

**Interfaces:**
- Consumes: `Frame::allocator`, `FrameAlloc` (Task 1).

- [ ] **Step 1: Write the four cases** (exactly one `// ERROR` line each, so the twin compiles)

`20_alloc_while_frame_allocator_lives.rs`:
```rust
use allocator_api2::vec::Vec;
use ordstack::Stack;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let a = f.allocator();
    let mut v: Vec<u32, _> = Vec::new_in(&a);
    let _b = f.alloc(1u32); // ERROR
    v.push(1);
}
```

`21_vec_outlives_frame_allocator.rs`:
```rust
use allocator_api2::vec::Vec;
use ordstack::Stack;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let a = f.allocator();
    let mut v: Vec<u32, _> = Vec::new_in(&a);
    v.push(1);
    drop(a); // ERROR
    v.push(2);
}
```

`22_stack_dropped_under_owned_allocator.rs`:
```rust
use allocator_api2::vec::Vec;
use ordstack::Stack;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v: Vec<u32, _> = Vec::new_in(f.allocator());
    v.push(1);
    drop(st); // ERROR
    v.push(2);
}
```

`23_frame_allocator_not_send.rs`:
```rust
use ordstack::Stack;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let a = f.allocator();
    std::thread::scope(|s| drop(s.spawn(move || drop(a)))); // ERROR
}
```

Run `scripts/gen-ui-pass.sh` to make the twins.

- [ ] **Step 2: Run the suite to see the four fail for lack of `.stderr`**

Commit, then: `.superpowers/trainbox.sh 'ORDSTACK_UI=1 timeout 900 cargo +1.85 test -p ordstack --all-features --test ui 2>&1 | grep -E "^test .*(ok|error|mismatch)|test result" | tail -50'`
Expected: the 23 `pass` twins and cases 01–19 ok; 20–23 report a missing `.stderr` (they do fail to compile).

- [ ] **Step 3: Record the messages and read them**

Run: `.superpowers/trainbox.sh 'ORDSTACK_UI=1 TRYBUILD=overwrite timeout 900 cargo +1.85 test -p ordstack --all-features --test ui >/dev/null 2>&1; echo done'`, then copy each file back: `for n in 20_alloc_while_frame_allocator_lives 21_vec_outlives_frame_allocator 22_stack_dropped_under_owned_allocator 23_frame_allocator_not_send; do ssh -p 2222 xinyan@localhost "cat ~/ordstack/crates/ordstack/tests/ui/fail/$n.stderr" > crates/ordstack/tests/ui/fail/$n.stderr; done`, and read the first lines of each.
Expected: 20 `E0499` (`f` borrowed twice), 21 `E0505` (move out of `a` while borrowed), 22 `E0505` (move out of `st`), 23 `E0277` (`Stack` cannot be shared between threads safely, or `Cell` not `Sync`). Any other code is a finding: stop and fix the case.

- [ ] **Step 4: Run the suite again, and update `ui.rs`'s doc line**

Change the doc comment's command in `tests/ui.rs` to `ORDSTACK_UI=1 cargo +1.85 test -p ordstack --all-features --test ui`. Commit, then run step 2's command again.
Expected: `test result: ok`.

- [ ] **Step 5: Commit** — squash to `ui: compile-fail cases for FrameAlloc`.

---

### Task 3: Model scope and fuzzing

**Files:**
- Modify: `crates/ordstack/tests/common/model.rs`, `crates/ordstack/tests/model.rs`, `crates/ordstack/fuzz/Cargo.toml`

**Interfaces:**
- Consumes: `Frame::allocator`, `FrameAlloc` (Task 1); the model's `Cx`, `Tracked`, `arg`, `ids`, `SVec::split_above`.
- Produces: op 23, when the feature is on: `split_above`, then `alloc_scope`.

- [ ] **Step 1: Write the failing fixed script**

Append to `crates/ordstack/tests/model.rs`:

```rust
#[test]
#[cfg(feature = "allocator-api2")]
fn fixed_script_allocator() {
    // Each scope starts with op 23 and ends with op 7 (`7 % 8`) and which
    // vector to drop first. Op bytes 0..8 act on vector 0, 8..16 on vector 1.
    // 1: push to 0 x2, push to 1 x2, pop 0, reserve 1 by 40, push to 0,
    //    shrink 1, clear 0, push 0; drop vector 1 first.
    // 2: push to 1 x3, pop 1 then 0 (empty); drop vector 0 first.
    // 3: at depth 1, push to 0; the script ends inside the scope.
    model::run(&[
        0, 23, 0, 0, 8, 8, 3, 12, 40, 0, 13, 6, 0, 7, 1, //
        23, 8, 8, 8, 11, 3, 7, 0, //
        0, 11, 0, 23, 0,
    ]);
}
```

Commit and run: `.superpowers/trainbox.sh 'timeout 900 cargo test -p ordstack --all-features --test model fixed_script_allocator 2>&1 | grep -E "test result|panicked" | head'`
Expected: it passes, wrongly, because op 23 is still a push. Note this; the assertion that makes it meaningful is step 2's model, so the RED here is the next step's first run.

- [ ] **Step 2: Add the scope**

In `crates/ordstack/tests/common/model.rs`:

1. Header table: change the row `| 0..=5, 21..=23 | push a fresh value |` to `| 0..=5, 21..=22 | push a fresh value (and 23 without the `allocator-api2` feature) |`, and add the row `| 23        | with the `allocator-api2` feature: `split_above`, then two allocator-api2 vectors on one `FrameAlloc` in the new frame (see `alloc_scope`) |`.
2. In `level`'s `match op % 25`, before `0..=5 | 21..=23 => {`, insert:

```rust
                #[cfg(feature = "allocator-api2")]
                23 => {
                    let (elems, mut up) = v.split_above();
                    alloc_scope(&mut up, input, cx);
                    assert_eq!(ids(elems), m, "a lower vector changed under an allocator");
                }
```

3. After `map_scope`, add:

```rust
/// Two `allocator_api2` vectors on one adapter, driven until the script
/// ends the scope. Op `b % 8` acts on vector `(b / 8) % 2`: 0..=2 push,
/// 3 pop, 4 reserve `arg % 64`, 5 shrink_to_fit, 6 clear, 7 end (then
/// `arg % 2` names the vector dropped first). Vectors drop front to back.
#[cfg(feature = "allocator-api2")]
fn alloc_scope<'c>(up: &mut Frame<'_>, input: &mut dyn Iterator<Item = u8>, cx: &'c Cx) {
    use allocator_api2::vec::Vec as AVec;
    let a = up.allocator();
    let mut vs: [AVec<Tracked<'c>, _>; 2] = [AVec::new_in(&a), AVec::new_in(&a)];
    let mut ms: [Vec<u32>; 2] = [Vec::new(), Vec::new()];
    let mut first = 0;
    while let Some(op) = input.next() {
        let i = (op as usize / 8) % 2;
        match op % 8 {
            0..=2 => {
                let t = cx.make();
                ms[i].push(t.id);
                vs[i].push(t);
            }
            3 => {
                if let Some(t) = vs[i].pop() {
                    let id = ms[i].pop().unwrap();
                    assert_eq!(t.id, id);
                    cx.expect([id]);
                    drop(t);
                }
            }
            4 => vs[i].reserve(arg(input) % 64),
            5 => vs[i].shrink_to_fit(),
            6 => {
                cx.expect(ms[i].drain(..));
                vs[i].clear();
            }
            _ => {
                first = arg(input) % 2;
                break;
            }
        }
        for j in 0..2 {
            assert_eq!(ids(&vs[j]), ms[j], "an allocator-api2 vector differs from the model");
        }
    }
    let [v0, v1] = vs;
    let (a_first, a_second) = if first == 0 { (v0, v1) } else { (v1, v0) };
    let (m_first, m_second) = (ms[first].clone(), ms[1 - first].clone());
    cx.expect(m_first);
    drop(a_first);
    cx.expect(m_second);
    drop(a_second);
}
```

4. Under "Verification" later (Task 4) the doc text is updated; nothing else here.

In `crates/ordstack/fuzz/Cargo.toml`:

```toml
[features]
default = ["allocator-api2"]
allocator-api2 = ["ordstack/allocator-api2", "dep:allocator-api2"]

[dependencies]
libfuzzer-sys = "0.4"
ordstack = { path = ".." }
allocator-api2 = { version = "0.2", optional = true }
```

- [ ] **Step 3: Run the model tests**

Commit and run: `.superpowers/trainbox.sh 'timeout 1200 cargo test -p ordstack --all-features --test model 2>&1 | grep -E "test result|panicked|FAILED"; timeout 1200 cargo test -p ordstack --test model 2>&1 | grep -E "test result|FAILED"'`
Expected: both `test result: ok` (8 tests with the feature, 7 without). To prove the scope checks something, temporarily change `cx.expect(m_first)` to `cx.expect(m_second.clone())`, run `--test model fixed_script_allocator` and see `drop order differs from the model`, then revert.

- [ ] **Step 4: Miri and fuzz**

Run: `.superpowers/trainbox.sh 'timeout 2400 cargo +nightly miri test -p ordstack --all-features --test model 2>&1 | grep -E "test result|Undefined|error:"; cd crates/ordstack && timeout 600 cargo +nightly fuzz run model -- -max_total_time=120 -max_len=4096 2>&1 | grep -E "Done|ERROR|panicked" | tail -3'`
Expected: `test result: ok` under Miri, and the fuzzer's `Done` line with no crash.

- [ ] **Step 5: Commit** — squash to `model: an allocator-api2 scope (op 23 with the feature); fuzz with the feature`.

---

### Task 4: SOUNDNESS, the table check, `check.sh` and CI

**Files:**
- Modify: `SOUNDNESS.md`, `scripts/check-unsafe-table.sh`, `scripts/check.sh`, `.github/workflows/ci.yml`

- [ ] **Step 1: Teach the check about the new module, and watch it fail**

In `scripts/check-unsafe-table.sh`, in the first `case`, add `alloc) ty=FrameAlloc ;;` before `*)`.
Commit and run: `.superpowers/trainbox.sh 'scripts/check-unsafe-table.sh'`
Expected: FAIL — `unsafe blocks in crates/ordstack/src: 81; rows in SOUNDNESS.md: 78`.

- [ ] **Step 2: Update SOUNDNESS.md**

1. "Why T holds", first paragraph: after "…whose end is the new `top`." add: "`FrameAlloc` (feature `allocator-api2`) raises `top` the same way when it allocates or grows its top block in place."
2. The call-site table: add rows

| `Frame::allocator` | `self.end` | `self.end`; the adapter's blocks lie above it and borrow the adapter |
| `FrameAlloc::deallocate`, block on top | the block's start | not an object end: see "The allocator adapter" |
| `FrameAlloc::shrink`, block on top and aligned | block start + new size | not an object end: see "The allocator adapter" |

3. New section after "Why growth is in place":

```markdown
## The allocator adapter

With the `allocator-api2` feature, `Frame::allocator(&mut self)` resets to
the frame's end and returns `FrameAlloc`, which holds the `&mut Frame`. Its
blocks are allocated by `try_bump` above `top` and lie above the frame's
end; collections reach them only through `&FrameAlloc` or by owning the
`FrameAlloc`, both of which keep the frame mutably borrowed, so I holds.
While the adapter lives nothing else allocates on the stack: the frame is
borrowed by the adapter and every frame below is frozen by the borrows that
made this one. So the adapter's blocks are the only objects above the
frame's end, and `top` is the end of the highest of them (or the frame's end).

- **Disjoint.** A fresh block starts at or above `top`. A block grows in
  place only when it ends at `top`, so it grows into space no live block
  uses. A zero-size block occupies nothing.
- **T.** `deallocate` and `shrink` lower `top` only for a block that ends
  at `top`, to that block's start or new end; every other live block ends
  at or below the block's start, so `top` stays `>=` every live end. (Two
  blocks can end at `top` only if one of them has size zero.)
- **E.** The frame's `end` is not written. The adapter's blocks are not
  frozen blocks and carry no `'f` brand.
- **The trait's contract.** A block stays valid until it is deallocated,
  shrunk or grown (each returns the block that replaces it), or until a
  reset; no reset can happen while the adapter is alive or borrowed.
  `allocate`, `grow` and `shrink` return blocks of the requested size and
  alignment: `try_bump` aligns the start, and the in-place paths are taken
  only when the existing start is aligned for the new layout. Failures
  return `AllocError` and leave `top` unchanged (`try_bump`, `try_extend`).
- **H3.** No user code runs inside the adapter, so nothing re-enters it
  mid-operation.
```

4. H8: add "`FrameAlloc` holds `&mut Frame`, which holds `&Stack`, so it is neither `Send` nor `Sync` (`tests/ui/fail/23_*`). Its `unsafe impl` is of `allocator_api2::alloc::Allocator`, not of `Send` or `Sync`."
5. Per-function table: the count line "128 public functions" becomes 129. Add to `### Frame`: `| Frame::allocator | &mut Frame<'f> | resets; the adapter holds the frame (see "The allocator adapter") |`. Add after the `SFrozenMap` section:

```markdown
### `FrameAlloc` (feature `allocator-api2`)

| Function | Takes | Preserves I, T, E, B because |
|---|---|---|
| `allocate` | `&FrameAlloc` | `try_bump`; see "The allocator adapter" |
| `deallocate` | `&FrameAlloc` | lowers `top` to the block's start only when the block ends at `top` |
| `grow` | `&FrameAlloc` | in place by `try_extend` when the block ends at `top` and is aligned for the new layout; otherwise a fresh block and a copy (row 79) |
| `grow_zeroed` | `&FrameAlloc` | `grow`, then zeroes the new tail (row 80) |
| `shrink` | `&FrameAlloc` | in place when aligned, lowering `top` to the new end if on top; otherwise a fresh block and a copy (row 81) |
```

6. "Every `unsafe` block": "finds 78 blocks: 75 in library code" → "finds 81 blocks: 78 in library code"; "There is one numbered row for each, in file order" → "There is one numbered row for each, in file order, except that `alloc.rs` (feature `allocator-api2`) comes last as rows 79–81"; "There is no `unsafe impl` and no `unsafe trait`." → "There is one `unsafe impl`, of `allocator_api2::alloc::Allocator` for `FrameAlloc` (feature `allocator-api2`; justified in "The allocator adapter"), and no `unsafe trait`." Rows appended after row 78:

```markdown
| 79 | `alloc.rs` | `FrameAlloc::grow` | `copy_nonoverlapping(ptr, fresh, old.size())` | the caller's contract: `ptr` is a live block of `old.size()` bytes from this adapter; `fresh` was just bumped above every live block, so they are disjoint |
| 80 | `alloc.rs` | `FrameAlloc::grow_zeroed` | `grow`, then `write_bytes` on `[old.size(), new.size())` | the caller's contract is `grow`'s; the result holds `new.size()` bytes |
| 81 | `alloc.rs` | `FrameAlloc::shrink` | `copy_nonoverlapping(ptr, fresh, new.size())` | as row 79, copying `new.size() <= old.size()` bytes |
```

7. Verification: compile-fail "19 cases" → "23 cases"; "Compiler messages are pinned … `ORDSTACK_UI=1`" sentence gains "with `--all-features`"; the test-suite bullet gains "with the default features and with `--all-features`"; the model bullet gains, after the map-scope sentence: "With the `allocator-api2` feature, op 23 opens an allocator scope instead of pushing: two `allocator_api2` vectors on one `FrameAlloc` above the vector, driven by push, pop, reserve, shrink_to_fit and clear on either vector, compared with `Vec` models after every step and dropped in either order." Miri, ASan and the fuzz target run with the feature.

- [ ] **Step 3: `check.sh` and CI run with the feature**

`scripts/check.sh`: clippy line gets `--all-features`; replace the two stable test steps with `cargo test --workspace` (step name "tests (stable, debug, default features)"), `cargo test --workspace --all-features` ("tests (stable, debug, all features)") and `cargo test --workspace --all-features --release`; MSRV step: `ORDSTACK_UI=1 cargo +1.85 test --workspace --all-features`; docs: `cargo doc --workspace --no-deps --all-features`; both Miri steps and the ASan step gain `--all-features` after `-p ordstack`. The fuzz step is unchanged (the fuzz crate enables the feature by default).

`.github/workflows/ci.yml`: in `test`, clippy gets `--all-features`; after `cargo test --workspace` add `- run: cargo test --workspace --all-features`; the release and doc runs get `--all-features`. `ui`: `cargo test -p ordstack --all-features --test ui`. `miri`: `cargo miri test -p ordstack --all-features`. `sanitizer-and-fuzz`: the test run gets `--all-features`.

- [ ] **Step 4: Run the check**

Commit and run: `.superpowers/trainbox.sh 'scripts/check-unsafe-table.sh'`
Expected: `unsafe blocks …: 81; rows …: 81`, `compile-fail cases: 23; stated …: 23`, `reset_to call sites and F.end writers: listed`.

- [ ] **Step 5: Commit** — squash to `SOUNDNESS: the allocator adapter; check.sh and CI run with all features`.

---

### Task 5: Docs and the full gate

**Files:**
- Modify: `crates/ordstack/src/lib.rs` (crate docs), `README.md`, `CHANGELOG.md`

- [ ] **Step 1: Crate docs**

In `crates/ordstack/src/lib.rs`, after the `//! # Maps` section (before `//! # Reclamation is lazy`), add (prose only, no links, so the docs build with the feature off too):

```rust
//! # Collections generic over an allocator
//!
//! With the `allocator-api2` feature, `Frame::allocator` returns a
//! `FrameAlloc`, an allocator for collections generic over
//! `allocator_api2::alloc::Allocator`: `allocator_api2::vec::Vec`,
//! `hashbrown`'s maps and sets, and others. Any number of them can grow on
//! one frame at once through `&FrameAlloc`; the one on top grows in place,
//! the others copy when they grow. Everything allocated through the adapter
//! is reclaimed when the frame is next used after it.
//!
//! Choose `SVec`, `SString` and `SMap` when the result should outlive the
//! scope that builds it (`freeze` gives frame-lifetime handles in place,
//! `freeze_into` copies to another stack), when insertion order matters
//! (`SMap`), or when no space may be wasted. Choose the adapter to reuse
//! code written for `Vec<T, A>` or `HashMap<K, V, S, A>`, or to grow
//! several collections on one frame at once; growth that is not at the top
//! copies and leaves dead space until the adapter's memory is reclaimed,
//! and nothing allocated through it outlives the adapter. As with `std`, a
//! failed allocation in an infallible method such as `push` aborts; use the
//! collection's `try_reserve` to handle it.
```

- [ ] **Step 2: README and CHANGELOG**

README: after the paragraph ending "…phases, and how to choose a reservation." add:

```markdown
With the `allocator-api2` feature, `Frame::allocator` also lets code written
for `Vec<T, A>` or `hashbrown`'s `HashMap` run on a frame, several
collections at once; the crate documentation's *Collections generic over an
allocator* section says when to use it and when to use `SVec`/`SMap`.
```

CHANGELOG, under `## 0.2.0 (unreleased)`, last line:
`- Feature \`allocator-api2\`: \`Frame::allocator\` returns \`FrameAlloc\`, an \`allocator_api2\` allocator, so \`allocator_api2::vec::Vec\`, \`hashbrown\` and other allocator-generic collections run on a frame (issue #4)`

- [ ] **Step 3: The full gate**

Commit and run: `.superpowers/trainbox.sh 'FUZZ_SECONDS=120 timeout 5400 scripts/check.sh > /tmp/ordstack-check.log 2>&1; echo exit=$?; grep -E "^== |All checks passed|error|FAILED" /tmp/ordstack-check.log | tail -30'`
Expected: `exit=0` and `All checks passed.`

- [ ] **Step 4: Commit** — squash to `docs: collections generic over an allocator`.
