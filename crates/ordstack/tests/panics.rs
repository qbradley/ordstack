//! Review Focus 5: panics in user code called by the library.

use ordstack::{SFixedVec, SVec, Stack};
use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

type Log = Rc<RefCell<Vec<u32>>>;

/// Logs its id when dropped; panics in its destructor if `bomb` is set
/// (but never while already unwinding, which would abort).
struct P {
    id: u32,
    bomb: bool,
    log: Log,
}
impl Drop for P {
    fn drop(&mut self) {
        self.log.borrow_mut().push(self.id);
        if self.bomb && !std::thread::panicking() {
            panic!("bomb {}", self.id);
        }
    }
}
fn fill<'v, 'f>(v: &mut SVec<'v, 'f, P>, n: u32, bomb: u32, log: &Log) {
    for id in 0..n {
        v.push(P {
            id,
            bomb: id == bomb,
            log: log.clone(),
        });
    }
}
fn p(id: u32, log: &Log) -> P {
    P {
        id,
        bomb: false,
        log: log.clone(),
    }
}
fn sorted(log: &Log) -> Vec<u32> {
    let mut v = log.borrow().clone();
    v.sort_unstable();
    v
}

#[test]
fn vec_drop_continues_after_a_panicking_destructor() {
    let log = Log::default();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v = SVec::new_in(&mut f);
    fill(&mut v, 5, 2, &log);
    assert!(catch_unwind(AssertUnwindSafe(move || drop(v))).is_err());
    assert_eq!(*log.borrow(), [4, 3, 2, 1, 0]);
    let mut w: SVec<u32> = SVec::new_in(&mut f);
    w.push(1);
    assert_eq!(&*w, [1]);
}

#[test]
fn slice_drop_continues_after_a_panicking_destructor() {
    let log = Log::default();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v = SVec::new_in(&mut f);
    fill(&mut v, 4, 3, &log);
    let s = v.freeze();
    assert!(catch_unwind(AssertUnwindSafe(move || drop(s))).is_err());
    assert_eq!(*log.borrow(), [3, 2, 1, 0]);
    assert_eq!(*f.alloc(7u8), 7);
}

#[test]
fn slice_drop_panic_in_the_middle_drops_the_rest_once() {
    let log = Log::default();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v = SVec::new_in(&mut f);
    fill(&mut v, 6, 3, &log);
    let s = v.freeze();
    assert!(catch_unwind(AssertUnwindSafe(move || drop(s))).is_err());
    assert_eq!(*log.borrow(), [5, 4, 3, 2, 1, 0]);
}

#[test]
fn into_iter_drop_panic_in_the_middle_drops_the_rest_once() {
    let log = Log::default();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v = SVec::new_in(&mut f);
    fill(&mut v, 7, 3, &log);
    let mut it = v.freeze().into_iter();
    drop(it.next()); // 0
    drop(it.next_back()); // 6
    assert_eq!(*log.borrow(), [0, 6]);
    assert!(catch_unwind(AssertUnwindSafe(move || drop(it))).is_err());
    assert_eq!(*log.borrow(), [0, 6, 5, 4, 3, 2, 1]);
    assert_eq!(sorted(&log), [0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn truncate_survives_a_panicking_destructor() {
    let log = Log::default();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v = SVec::new_in(&mut f);
    fill(&mut v, 5, 3, &log);
    assert!(catch_unwind(AssertUnwindSafe(|| v.truncate(1))).is_err());
    assert_eq!(*log.borrow(), [4, 3, 2, 1]);
    assert_eq!(v.len(), 1);
    v.push(p(9, &log));
    drop(v);
    assert_eq!(sorted(&log), [0, 1, 2, 3, 4, 9]);
}

#[test]
fn retain_survives_a_panicking_predicate() {
    let log = Log::default();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v = SVec::new_in(&mut f);
    fill(&mut v, 8, u32::MAX, &log);
    let r = catch_unwind(AssertUnwindSafe(|| {
        v.retain(|p| {
            if p.id == 5 {
                panic!("predicate");
            }
            p.id % 2 == 1
        })
    }));
    assert!(r.is_err());
    assert_eq!(*log.borrow(), [0, 2, 4]);
    assert!(v.iter().map(|p| p.id).eq([1, 3, 5, 6, 7]));
    drop(v);
    assert_eq!(sorted(&log), [0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn retain_survives_a_panicking_destructor() {
    let log = Log::default();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v = SVec::new_in(&mut f);
    fill(&mut v, 6, 2, &log);
    assert!(catch_unwind(AssertUnwindSafe(|| v.retain(|p| p.id % 2 == 1))).is_err());
    assert_eq!(*log.borrow(), [0, 2]);
    assert!(v.iter().map(|p| p.id).eq([1, 3, 4, 5]));
    drop(v);
    assert_eq!(sorted(&log), [0, 1, 2, 3, 4, 5]);
}

struct Fragile(u32, Log);
impl Clone for Fragile {
    fn clone(&self) -> Fragile {
        if self.0 == 3 {
            panic!("clone");
        }
        Fragile(self.0 + 100, self.1.clone())
    }
}
impl Drop for Fragile {
    fn drop(&mut self) {
        self.1.borrow_mut().push(self.0);
    }
}

#[test]
fn slice_clone_drops_partial_work_on_panic() {
    let log = Log::default();
    let src: Vec<Fragile> = (0..5).map(|i| Fragile(i, log.clone())).collect();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    assert!(catch_unwind(AssertUnwindSafe(|| drop(f.alloc_slice_clone(&src)))).is_err());
    assert_eq!(*log.borrow(), [102, 101, 100]);
    let again = f.alloc_slice_clone(&src[..3]);
    assert!(again.iter().map(|x| x.0).eq([100, 101, 102]));
    drop(again);
    assert_eq!(*log.borrow(), [102, 101, 100, 102, 101, 100]);
}

#[test]
fn alloc_with_reserves_nothing_if_the_closure_panics() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let _a = f.alloc(1u64);
    let r = catch_unwind(AssertUnwindSafe(|| {
        drop(f.alloc_with(|| -> [u64; 8] { panic!("ctor") }))
    }));
    assert!(r.is_err());
    assert_eq!(f.end_offset(), 8);
}

#[test]
fn extend_keeps_what_was_pushed_before_the_iterator_panicked() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v: SVec<u32> = SVec::new_in(&mut f);
    let it = (0..10).map(|i| if i == 4 { panic!("iter") } else { i });
    assert!(catch_unwind(AssertUnwindSafe(|| v.extend(it))).is_err());
    assert_eq!(&*v, [0, 1, 2, 3]);
    v.push(4);
    assert_eq!(v.len(), 5);
}

#[test]
fn extend_panic_drops_pushed_elements_once_and_frees_the_frame() {
    let log = Log::default();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let end = f.end_offset();
    let lg = log.clone();
    let it = (0..10).map(move |i| {
        if i == 4 {
            panic!("iter");
        }
        p(i, &lg)
    });
    assert!(catch_unwind(AssertUnwindSafe(|| drop(SVec::from_iter_in(it, &mut f)))).is_err());
    // The half-built vector is dropped during unwinding: back to front.
    assert_eq!(*log.borrow(), [3, 2, 1, 0]);
    assert_eq!(f.end_offset(), end);
    let mut v = SVec::new_in(&mut f);
    v.push(p(20, &log));
    drop(v);
    assert_eq!(*log.borrow(), [3, 2, 1, 0, 20]);
}

#[test]
fn zero_size_elements_with_drop_through_svec() {
    // Zero-size, with a destructor that counts (a thread local, as the type has no room).
    thread_local!(static N: Cell<u32> = const { Cell::new(0) });
    struct Zst;
    impl Drop for Zst {
        fn drop(&mut self) {
            N.with(|n| n.set(n.get() + 1));
        }
    }
    assert_eq!(size_of::<Zst>(), 0);
    let made = Cell::new(0u32);
    let mk = || {
        made.set(made.get() + 1);
        Zst
    };
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let _anchor = f.alloc(1u64);
    let end = f.end_offset();
    {
        let mut v: SVec<Zst> = SVec::new_in(&mut f);
        for _ in 0..5 {
            v.push(mk());
        }
        v.insert(2, mk());
        v.insert(0, mk());
        assert_eq!(v.len(), 7);
        drop(v.remove(3)); // 1 dropped
        assert_eq!(N.with(Cell::get), 1);
        v.retain(|_| false); // 6 dropped
        assert_eq!(N.with(Cell::get), 7);
        for _ in 0..4 {
            v.push(mk());
        }
        v.truncate(1); // 3 dropped
        assert_eq!(N.with(Cell::get), 10);
        drop(v.pop()); // 1
    }
    assert_eq!(f.end_offset(), end);
    {
        let mut fx: SFixedVec<Zst> = SFixedVec::with_capacity_in(3, &mut f);
        for _ in 0..3 {
            fx.push(mk());
        }
        assert_eq!(f.end_offset(), end);
    }
    assert_eq!(f.end_offset(), end);
    assert_eq!(N.with(Cell::get), made.get());
    assert_eq!(made.get(), 5 + 2 + 4 + 3);
}

#[test]
fn insert_on_an_exhausted_stack_changes_nothing() {
    let mut st = Stack::with_reserve(4096);
    if st.capacity() != 4096 {
        return; // sizes below assume 4 KiB pages
    }
    let mut f = st.frame();
    let mut v: SVec<u8> = SVec::with_capacity_in(4096, &mut f);
    let cap = v.capacity();
    assert_eq!(cap, 4096);
    v.extend((0..4096).map(|i| i as u8));
    let before: Vec<u8> = v.to_vec();
    let err = catch_unwind(AssertUnwindSafe(|| v.insert(7, 99))).unwrap_err();
    let msg = err
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap();
    assert!(msg.starts_with("ordstack: "), "{msg}");
    assert_eq!((v.len(), v.capacity()), (4096, cap));
    assert_eq!(&*v, &before[..]);
}
