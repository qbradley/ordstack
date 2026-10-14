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
    assert_ne!(
        addr(lo.as_ptr()),
        plo,
        "the lower vector grew in place over the upper one"
    );
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
    assert_eq!(
        again, first,
        "a second adapter did not reuse the first one's memory"
    );
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
    assert_eq!(
        addr(lower.as_ptr()),
        p,
        "the lower vector did not grow in place"
    );
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
    assert_eq!(
        addr(probe.as_ptr()),
        p + cap,
        "a failed growth moved the top"
    );
    assert!(
        a.allocate(Layout::from_size_align(8, 8192).unwrap())
            .is_err()
    );
    assert!(
        a.allocate(Layout::from_size_align(1 << 30, 1).unwrap())
            .is_err()
    );
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
    // SAFETY: every pointer passed back to `a` was returned by
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
        let z = a
            .grow_zeroed(s, l(8, 128), l(40, 128))
            .unwrap()
            .cast::<u8>();
        assert!((0..8).all(|i| *z.as_ptr().add(i) == i as u8));
        assert!((8..40).all(|i| *z.as_ptr().add(i) == 0));
        a.deallocate(z, l(40, 128));
        a.deallocate(one.cast(), l(1, 1));
    }
}
