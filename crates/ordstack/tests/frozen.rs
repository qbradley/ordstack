use ordstack::{AllocErrorKind, SBox, SSlice, SStr, Stack};
use std::cell::RefCell;
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
fn boxes_sit_side_by_side() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut a = f.alloc(1u64);
    let b = f.alloc(2u8);
    let c = f.alloc(3u32);
    *a += 10;
    assert_eq!((*a, *b, *c), (11, 2, 3));
    assert_eq!(f.end_offset(), 16); // 8 + 1 + 3 padding + 4
}

#[test]
fn sub_frame_is_reclaimed_when_parent_is_used() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let a = f.alloc(1u64);
    {
        let mut s = f.sub();
        let _x = s.alloc([0u8; 100]);
        let mut t = s.sub();
        let _y = t.alloc([0u8; 100]);
    }
    let b = f.alloc(2u64);
    assert_eq!(f.end_offset(), 16);
    assert_eq!((*a, *b), (1, 2));
    drop((a, b));
    let _ = f;
    assert_eq!(st.extent(), 16); // lazy: nothing reclaims on drop
    st.reset();
    assert_eq!(st.extent(), 0);
}

#[test]
fn reset_above_and_trim_above() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let a = f.alloc(7u32);
    {
        let mut s = f.sub();
        let _big = s.alloc([1u8; 8192]);
    }
    f.reset_above();
    f.trim_above();
    let b = f.alloc(8u32);
    assert_eq!((*a, *b), (7, 8));
}

#[test]
fn box_drops_its_value_once() {
    let l = log();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let a = f.alloc(D(1, l.clone()));
    let b = f.alloc(D(2, l.clone()));
    drop(a);
    let inner = SBox::into_inner(b);
    assert_eq!(*l.borrow(), [1]);
    drop(inner);
    assert_eq!(*l.borrow(), [1, 2]);
}

#[test]
fn leak_outlives_the_handle() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let r: &mut u32 = SBox::leak(f.alloc(5u32));
    let other = f.alloc(6u32);
    *r += 1;
    assert_eq!((*r, *other), (6, 6));
}

#[test]
fn alloc_with_runs_before_reserving() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let b = f.alloc_with(|| [3u16; 4]);
    assert_eq!(*b, [3; 4]);
}

#[test]
fn zero_sized_and_overaligned_values() {
    #[repr(align(64))]
    struct A64(u8);
    #[repr(align(8192))]
    struct Huge(#[allow(dead_code)] u8);
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let _odd = f.alloc(1u8);
    let unit = f.alloc(());
    assert_eq!(f.end_offset(), 1);
    let a = f.alloc(A64(9));
    assert_eq!(&*a as *const A64 as usize % 64, 0);
    assert_eq!(a.0, 9);
    assert_eq!(f.end_offset(), 128);
    let e = f.try_alloc(Huge(0)).err().unwrap();
    assert_eq!(e.kind(), AllocErrorKind::AlignTooLarge);
    assert_eq!(f.end_offset(), 128);
    drop(unit);
}

#[test]
fn exhaustion_is_reported_and_changes_nothing() {
    let mut st = Stack::with_reserve(4096);
    if st.capacity() != 4096 {
        return; // larger pages (e.g. 16 KiB on Apple silicon): sizes below assume 4 KiB
    }
    let mut f = st.frame();
    let _a = f.alloc([0u8; 4000]);
    let e = f.try_alloc([0u8; 200]).err().unwrap();
    assert_eq!(e.kind(), AllocErrorKind::Exhausted);
    assert_eq!(f.end_offset(), 4000);
    let _b = f.alloc([0u8; 96]);
    assert_eq!(f.end_offset(), 4096);
}

#[test]
#[should_panic(expected = "ordstack: stack reservation exhausted")]
fn alloc_panics_when_exhausted() {
    let mut st = Stack::with_reserve(0);
    let mut f = st.frame();
    let _ = f.alloc(1u8);
}

#[test]
fn box_forwards_common_traits() {
    use std::collections::HashSet;
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let a = f.alloc(3u32);
    let b = f.alloc(4u32);
    assert!(a < b && a != b);
    assert_eq!(format!("{a:?}"), "3");
    let mut set = HashSet::new();
    set.insert(a);
    assert!(set.contains(&3u32));
}

#[derive(Clone)]
struct C(u32, Rc<RefCell<Vec<u32>>>);
impl Drop for C {
    fn drop(&mut self) {
        self.1.borrow_mut().push(self.0);
    }
}

#[test]
fn slice_copy_and_str() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut s = f.alloc_slice_copy(&[1u32, 2, 3]);
    let t = f.alloc_str("héllo");
    s[1] = 20;
    assert_eq!(&*s, [1, 20, 3]);
    assert_eq!(t.as_str(), "héllo");
    assert_eq!(format!("{t} {t:?}"), "héllo \"héllo\"");
    assert_eq!(f.end_offset(), 12 + 6);
    let e = f.alloc_slice_copy::<u64>(&[]);
    let es = f.alloc_str("");
    assert!(e.is_empty() && es.is_empty());
    assert_eq!(f.end_offset(), 18);
    assert!(SSlice::<u8>::empty().is_empty());
    assert_eq!(SStr::empty().as_str(), "");
}

#[test]
fn slice_drops_back_to_front() {
    let l = log();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let src: Vec<C> = (0..4).map(|i| C(i, l.clone())).collect();
    let s = f.alloc_slice_clone(&src);
    drop(s);
    assert_eq!(*l.borrow(), [3, 2, 1, 0]);
}

#[test]
fn slice_into_iter_yields_in_order_and_drops_the_rest() {
    let l = log();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let src: Vec<C> = (0..5).map(|i| C(i, l.clone())).collect();
    let s = f.alloc_slice_clone(&src);
    let mut it = s.into_iter();
    assert_eq!(it.len(), 5);
    assert_eq!(it.next().unwrap().0, 0);
    assert_eq!(it.next_back().unwrap().0, 4);
    assert_eq!(*l.borrow(), [0, 4]);
    drop(it);
    assert_eq!(*l.borrow(), [0, 4, 3, 2, 1]);
}

#[test]
fn slice_and_str_leak() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let xs: &mut [u8] = SSlice::leak(f.alloc_slice_copy(b"abc"));
    let s: &mut str = SStr::leak(f.alloc_str("xyz"));
    xs[0] = b'z';
    s.make_ascii_uppercase();
    assert_eq!((&*xs, &*s), (&b"zbc"[..], "XYZ"));
}

#[test]
fn zero_size_value_with_alignment_above_the_limit_succeeds() {
    // Zero-size requests never reach the stack, so the 4096 limit does not
    // apply to them; the handle's pointer is dangling but aligned.
    #[repr(align(8192))]
    struct BigZst;
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let _odd = f.alloc(1u8);
    let z = f.try_alloc(BigZst).unwrap();
    assert_eq!(&*z as *const BigZst as usize % 8192, 0);
    let none = f.alloc_slice_copy::<[u64; 0]>(&[[], []]);
    assert_eq!(none.len(), 2);
    assert_eq!(f.end_offset(), 1);
}

#[test]
fn failed_allocation_changes_no_object_but_may_reclaim_above() {
    // The lazy reset to the frame's end runs before the step that fails, so
    // the failure leaves `end` alone while `top` drops from 108 to 8.
    #[repr(align(8192))]
    struct Huge(#[allow(dead_code)] u8);
    let mut st = Stack::with_reserve(1 << 16);
    {
        let mut f = st.frame();
        let kept = f.alloc(1u64);
        drop(f.sub().alloc([0u8; 100]));
        let e = f.try_alloc(Huge(0)).err().unwrap();
        assert_eq!(e.kind(), AllocErrorKind::AlignTooLarge);
        assert_eq!(f.end_offset(), 8);
        assert_eq!(*kept, 1);
    }
    assert_eq!(st.extent(), 8);
}
