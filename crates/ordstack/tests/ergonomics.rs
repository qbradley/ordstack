//! The 0.2a additions: `Frame::stack`, `Frame::alloc_slice_from_iter`,
//! `sformat!`, and `IntoIterator`.

use ordstack::{SVec, Stack};
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
