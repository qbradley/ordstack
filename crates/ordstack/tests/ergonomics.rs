//! The 0.2a additions: `Frame::stack`, `Frame::alloc_slice_from_iter`,
//! `sformat!`, and `IntoIterator`.

use ordstack::{SFixedVec, SVec, Stack, sformat};
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

/// Pushes its id to a shared log when dropped.
struct D(u32, Rc<RefCell<Vec<u32>>>);
impl Drop for D {
    fn drop(&mut self) {
        self.1.borrow_mut().push(self.0);
    }
}
fn log() -> Rc<RefCell<Vec<u32>>> {
    Rc::new(RefCell::new(Vec::new()))
}

#[test]
fn stack_statistics_are_readable_through_a_frame() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let _a = f.alloc([0u8; 100]);
    assert_eq!(f.stack().extent(), 100);
    assert_eq!(f.stack().high_water(), 100);
    assert!(f.stack().capacity() >= 1 << 16);
}

#[test]
fn sub_and_above_frames_report_the_whole_stack() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let _a = f.alloc(1u64); // 0..8
    let mut v: SVec<u64> = SVec::with_capacity_in(4, &mut f); // exactly 8..40
    v.extend(0..4);
    {
        let up = v.above();
        assert_eq!(up.stack().extent(), 40);
    }
    drop(v);
    let mut sub = f.sub(); // resets to 8
    let _b = sub.alloc(2u32); // 8..12
    assert_eq!(sub.stack().extent(), 12);
    assert_eq!(sub.stack().high_water(), 40);
}

#[test]
fn alloc_slice_from_iter_collects_in_place() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let s = f.alloc_slice_from_iter((1..=4u32).map(|x| x * 10));
    assert_eq!(&*s, [10, 20, 30, 40]);
    assert_eq!(f.end_offset(), 16);
    let t = f.alloc_slice_from_iter(core::iter::empty::<u64>());
    assert!(t.is_empty());
    assert_eq!(f.end_offset(), 16);
    let u = f.alloc_slice_from_iter([7u8]);
    assert_eq!(&*u, [7]);
    assert_eq!(f.end_offset(), 17);
}

/// Claims at most one item but yields three.
struct Liar(u32);
impl Iterator for Liar {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 3 {
            return None;
        }
        self.0 += 1;
        Some(self.0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(1))
    }
}

#[test]
fn alloc_slice_from_iter_ignores_a_wrong_size_hint() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let s = f.alloc_slice_from_iter(Liar(0));
    assert_eq!(&*s, [1, 2, 3]);
}

#[test]
fn alloc_slice_from_iter_drops_what_it_collected_on_panic() {
    let l = log();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let r = catch_unwind(AssertUnwindSafe(|| {
        f.alloc_slice_from_iter((0..3).map(|i| {
            if i == 2 {
                panic!("boom");
            }
            D(i, l.clone())
        }))
    }));
    assert!(r.is_err());
    assert_eq!(*l.borrow(), [1, 0]);
    let b = f.alloc(5u32); // the frame is usable
    assert_eq!(*b, 5);
}

#[test]
fn sformat_formats_into_a_frozen_string() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let a = sformat!(&mut f, "{}-{}", 1, "x");
    let b = sformat!(&mut f, "{a}!");
    assert_eq!(a.as_str(), "1-x");
    assert_eq!(b.as_str(), "1-x!");
    assert_eq!(f.end_offset(), 7);
}

struct Fails;
impl std::fmt::Display for Fails {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Err(std::fmt::Error)
    }
}

#[test]
#[should_panic(expected = "ordstack: a formatting trait implementation returned an error")]
fn sformat_panics_when_display_fails() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let _ = sformat!(&mut f, "{}", Fails);
}

struct Panics;
impl std::fmt::Display for Panics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("partial")?;
        panic!("display panicked");
    }
}

#[test]
fn sformat_leaves_the_frame_usable_after_a_panicking_display() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let kept = f.alloc(1u64);
    let r = catch_unwind(AssertUnwindSafe(|| {
        let _ = sformat!(&mut f, "{}", Panics);
    }));
    assert!(r.is_err());
    let s = sformat!(&mut f, "ok");
    assert_eq!(s.as_str(), "ok");
    assert_eq!(*kept, 1);
    assert_eq!(f.end_offset(), 10); // the partial string's space was reused
}

#[test]
fn borrowed_iteration_over_every_container() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut s = f.alloc_slice_copy(&[1u32, 2, 3]);
    for x in &mut s {
        *x *= 2;
    }
    let mut sum = 0;
    for x in &s {
        sum += x;
    }
    assert_eq!(sum, 12);

    let mut fx = SFixedVec::with_capacity_in(3, &mut f);
    fx.push(1u32);
    fx.push(2);
    for x in &mut fx {
        *x += 1;
    }
    assert_eq!((&fx).into_iter().copied().collect::<Vec<_>>(), [2, 3]);

    let mut v: SVec<u32> = SVec::new_in(&mut f);
    v.extend([5, 6]);
    for x in &mut v {
        *x *= 10;
    }
    assert_eq!((&v).into_iter().copied().collect::<Vec<_>>(), [50, 60]);
}

#[test]
fn sfixedvec_by_value() {
    let l = log();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut fx = SFixedVec::with_capacity_in(4, &mut f);
    for i in 0..4 {
        fx.push(D(i, l.clone()));
    }
    let mut it = fx.into_iter();
    assert_eq!(it.next().map(|d| d.0), Some(0)); // D(0) dropped here
    drop(it);
    assert_eq!(*l.borrow(), [0, 3, 2, 1]);
}

#[test]
fn svec_by_value_reclaims_the_buffer_lazily() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let _kept = f.alloc(1u64); // 0..8
    let mut v: SVec<u64> = SVec::new_in(&mut f);
    v.extend(0..100);
    let sum: u64 = v.into_iter().sum();
    assert_eq!(sum, 4950);
    assert_eq!(f.end_offset(), 8); // nothing was frozen
    let b = f.alloc(7u64); // reuses the buffer's space
    assert_eq!(f.end_offset(), 16);
    assert_eq!(*b, 7);
}

#[test]
fn svec_by_value_drops_the_rest_back_to_front() {
    let l = log();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let v = SVec::new_in(&mut f);
    for i in 0..5 {
        v.push(D(i, l.clone()));
    }
    let mut it = v.into_iter();
    assert_eq!(it.len(), 5);
    assert_eq!(it.next().map(|d| d.0), Some(0));
    assert_eq!(it.next_back().map(|d| d.0), Some(4));
    drop(it);
    assert_eq!(*l.borrow(), [0, 4, 3, 2, 1]);
}

#[test]
fn svec_by_value_empty_and_zero_size() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let v: SVec<u64> = SVec::new_in(&mut f);
    assert_eq!(v.into_iter().count(), 0);
    let mut z: SVec<()> = SVec::new_in(&mut f);
    z.extend([(), (), ()]);
    assert_eq!(z.into_iter().count(), 3);
    assert_eq!(f.end_offset(), 0);
}

#[test]
fn forgotten_svec_iterator_leaks_and_the_frame_recovers() {
    // Plain `u32` elements: a forgotten iterator of values that own heap
    // memory would be reported by Miri as a leak, which is not the point.
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v: SVec<u32> = SVec::new_in(&mut f);
    v.extend([1, 2, 3]);
    let mut it = v.into_iter();
    assert_eq!(it.next(), Some(1));
    core::mem::forget(it);
    let b = f.alloc(9u32); // reclaims the buffer
    assert_eq!(*b, 9);
    assert_eq!(f.end_offset(), 4);
}
