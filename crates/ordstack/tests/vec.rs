// Frames have no `Drop`; `drop(f)` / `forget(up)` here end borrows on purpose.
#![allow(clippy::drop_non_drop, clippy::forget_non_drop)]

use ordstack::{AllocErrorKind, Frame, SSlice, SVec, Stack};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

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
fn grows_in_place() {
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    let mut v: SVec<u64> = SVec::new_in(&mut f);
    assert_eq!(v.capacity(), 0);
    v.push(0);
    assert_eq!(v.capacity(), 4);
    let first = v.as_ptr();
    for i in 1..2000 {
        v.push(i);
    }
    assert_eq!(first, v.as_ptr(), "the buffer never moves");
    assert_eq!(v.capacity(), 2048);
    assert!(v.iter().copied().eq(0..2000));
    assert_eq!(v.pop(), Some(1999));
    assert_eq!(v.len(), 1999);
    drop(v);
    drop(f);
    assert_eq!(st.extent(), 2048 * 8);
    assert_eq!(st.high_water(), 2048 * 8);
}

#[test]
fn drops_back_to_front() {
    let l = log();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v = SVec::new_in(&mut f);
    for i in 0..5 {
        v.push(D(i, l.clone()));
    }
    drop(v.pop());
    drop(v);
    assert_eq!(*l.borrow(), [4, 3, 2, 1, 0]);
}

#[test]
fn freeze_and_descend() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut lower: SVec<u32> = SVec::new_in(&mut f);
    lower.extend([1, 2, 3]);
    let p = lower.as_ptr();
    {
        let mut above = lower.above();
        let mut upper: SVec<u32> = SVec::new_in(&mut above);
        upper.extend(0..100);
        assert_eq!(upper.len(), 100);
    }
    lower.extend(4..=100);
    assert_eq!(p, lower.as_ptr());
    assert!(lower.iter().copied().eq(1..=100));
    {
        let (elems, mut above) = lower.split_above();
        let b = above.alloc(elems.len() as u32);
        elems[0] = *b;
    }
    assert_eq!(lower[0], 100);
}

#[test]
fn freeze_gives_back_spare_capacity() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v: SVec<u32> = SVec::new_in(&mut f);
    v.extend(0..5);
    let a: SSlice<u32> = v.freeze();
    assert_eq!(f.end_offset(), 20);
    let mut w: SVec<u32> = SVec::new_in(&mut f);
    w.extend(10..13);
    let b = w.freeze();
    let c = f.alloc(9u32);
    assert_eq!((&*a, &*b, *c), (&[0, 1, 2, 3, 4][..], &[10, 11, 12][..], 9));
    assert_eq!(f.end_offset(), 20 + 12 + 4);
    let empty: SVec<u32> = SVec::with_capacity_in(16, &mut f);
    assert!(empty.freeze().is_empty());
    assert_eq!(f.end_offset(), 36);
}

/// A tree whose parent's child list grows: children are collected on a
/// scratch stack and copied once into the tree's stack.
struct Node<'t> {
    id: u32,
    kids: SSlice<'t, Node<'t>>,
}

fn grow<'t>(tree: &mut Frame<'t>, scratch: &mut Frame<'_>, depth: u32, next: &mut u32) -> Node<'t> {
    let id = *next;
    *next += 1;
    let mut kids: SVec<Node<'t>> = SVec::new_in(scratch);
    for _ in 0..depth {
        let (_, mut above) = kids.split_above();
        let kid = grow(tree, &mut above, depth - 1, next);
        kids.push(kid);
    }
    Node {
        id,
        kids: kids.freeze_into(tree),
    }
}

fn sum(n: &Node<'_>) -> u32 {
    n.id + n.kids.iter().map(sum).sum::<u32>()
}

#[test]
fn freeze_into_another_stack() {
    let mut tree = Stack::with_reserve(1 << 20);
    let mut scratch = Stack::with_reserve(1 << 20);
    let mut next = 0;
    let (mut tf, mut sf) = (tree.frame(), scratch.frame());
    let root = grow(&mut tf, &mut sf, 4, &mut next);
    assert_eq!(next, 65); // 1 + 4 + 12 + 24 + 24
    assert_eq!(sum(&root), (0..65).sum::<u32>());
}

#[test]
fn reserve_capacity_and_shrink() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v: SVec<u16> = SVec::with_capacity_in(10, &mut f);
    assert_eq!(v.capacity(), 10);
    v.extend_from_slice(&[1, 2, 3]);
    v.reserve(7);
    assert_eq!(v.capacity(), 10);
    v.reserve(8);
    assert_eq!(v.capacity(), 20);
    v.reserve(1000);
    assert_eq!(v.capacity(), 1003);
    v.shrink_to_fit();
    assert_eq!(v.capacity(), 3);
    v.pop();
    v.pop();
    v.pop();
    v.shrink_to_fit();
    assert_eq!(v.capacity(), 0);
    v.push(5);
    assert_eq!(&*v, [5]);
    let doubled: Vec<u16> = v.iter().map(|x| x * 2).collect();
    let mut up = v.above();
    let w = SVec::from_iter_in(doubled, &mut up);
    assert_eq!(&*w, [10]);
}

// --- Review Focus 1 ---
#[test]
fn forgotten_child_is_reclaimed() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v: SVec<u64> = SVec::new_in(&mut f);
    v.extend(0..4);
    let p = v.as_ptr();
    {
        let mut up = v.above();
        let mut w: SVec<u64> = SVec::new_in(&mut up);
        w.extend(0..100);
        std::mem::forget(w);
        std::mem::forget(up);
    }
    v.extend(4..8);
    assert_eq!(p, v.as_ptr());
    assert!(v.iter().copied().eq(0..8));
    drop(v);
    drop(f);
    assert_eq!(st.extent(), 64, "the forgotten child's space was reclaimed");
}

// --- Review Focus 2 ---
#[test]
fn failed_growth_changes_nothing() {
    let mut st = Stack::with_reserve(4096);
    if st.capacity() != 4096 {
        return; // sizes below assume 4 KiB pages
    }
    let mut f = st.frame();
    let mut v: SVec<u64> = SVec::with_capacity_in(512, &mut f);
    v.extend(0..512);
    let (e_val, e) = v.try_push(77).unwrap_err();
    assert_eq!((e_val, e.kind()), (77, AllocErrorKind::Exhausted));
    assert_eq!(
        v.try_reserve(1).unwrap_err().kind(),
        AllocErrorKind::Exhausted
    );
    assert_eq!(
        v.try_reserve(usize::MAX).unwrap_err().kind(),
        AllocErrorKind::CapacityOverflow
    );
    assert_eq!((v.len(), v.capacity()), (512, 512));
    assert_eq!(v.pop(), Some(511));
    v.push(1);
    assert_eq!(v.len(), 512);
}

#[test]
fn doubling_falls_back_to_the_exact_size() {
    let mut st = Stack::with_reserve(4096);
    if st.capacity() != 4096 {
        return;
    }
    let mut f = st.frame();
    let mut v: SVec<u8> = SVec::with_capacity_in(3000, &mut f);
    v.extend_from_slice(&[0; 3000]);
    v.push(1); // doubling to 6000 does not fit; 3001 does
    assert_eq!(v.capacity(), 3001);
}

#[test]
#[should_panic(expected = "ordstack: stack reservation exhausted")]
fn push_panics_when_exhausted() {
    let mut st = Stack::with_reserve(0);
    let mut f = st.frame();
    let mut v: SVec<u8> = SVec::new_in(&mut f);
    v.push(1);
}

// --- Review Focus 3 ---
#[test]
fn overaligned_after_odd_block() {
    #[repr(align(64))]
    #[derive(Clone, Copy)]
    struct A(u8);
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let _odd = f.alloc(1u8);
    {
        let v: SVec<A> = SVec::with_capacity_in(4, &mut f);
        assert!(v.freeze().is_empty());
    }
    assert_eq!(f.end_offset(), 1, "an empty freeze keeps no padding");
    let mut v: SVec<A> = SVec::new_in(&mut f);
    v.push(A(1));
    v.push(A(2));
    assert_eq!(v.as_ptr() as usize % 64, 0);
    let s = v.freeze();
    assert_eq!(f.end_offset(), 64 + 128);
    let after = f.alloc(9u8);
    assert_eq!((s[0].0, s[1].0, *after), (1, 2, 9));
}

// --- Review Focus 4 ---
static Z_DROPS: AtomicUsize = AtomicUsize::new(0);
struct Z;
impl Drop for Z {
    fn drop(&mut self) {
        Z_DROPS.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn zst_elements() {
    let n = if cfg!(miri) { 1_000 } else { 1_000_000 };
    let mut st = Stack::with_reserve(0);
    let mut f = st.frame();
    let mut v: SVec<Z> = SVec::new_in(&mut f);
    assert_eq!(v.capacity(), usize::MAX);
    for _ in 0..n {
        v.push(Z);
    }
    v.reserve(usize::MAX - n);
    assert_eq!(
        v.try_reserve(usize::MAX).unwrap_err().kind(),
        AllocErrorKind::CapacityOverflow
    );
    assert_eq!(v.len(), n);
    drop(v.pop());
    assert_eq!(Z_DROPS.load(Ordering::Relaxed), 1);
    let _ = v.above().alloc(Z);
    let s = v.freeze();
    assert_eq!(s.len(), n - 1);
    assert_eq!(f.end_offset(), 0);
    drop(s);
    assert_eq!(Z_DROPS.load(Ordering::Relaxed), n + 1);
}
