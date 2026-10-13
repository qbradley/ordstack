use ordstack::{Frame, SFixedVec, SSlice, SStr, Stack};
use std::cell::RefCell;
use std::rc::Rc;

struct D(u32, Rc<RefCell<Vec<u32>>>);
impl Drop for D {
    fn drop(&mut self) {
        self.1.borrow_mut().push(self.0);
    }
}

#[test]
fn fills_up_to_capacity() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v = SFixedVec::with_capacity_in(3, &mut f);
    assert_eq!((v.len(), v.capacity(), v.is_full()), (0, 3, false));
    v.push(1u32);
    v.push(2);
    assert_eq!(v.try_push(3), Ok(()));
    assert_eq!(v.try_push(4), Err(4));
    assert!(v.is_full());
    assert_eq!(v.pop(), Some(3));
    assert_eq!(&*v, [1, 2]);
    assert_eq!(f.end_offset(), 12);
}

#[test]
#[should_panic(expected = "ordstack: SFixedVec capacity exceeded")]
fn push_past_capacity_panics() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v = SFixedVec::with_capacity_in(1, &mut f);
    v.push(1u8);
    v.push(2u8);
}

#[test]
fn drops_back_to_front_and_into_slice_keeps_values() {
    let l = Rc::new(RefCell::new(Vec::new()));
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v = SFixedVec::with_capacity_in(4, &mut f);
    for i in 0..3 {
        v.push(D(i, l.clone()));
    }
    let s = v.into_slice();
    assert_eq!(s.len(), 3);
    assert!(l.borrow().is_empty());
    drop(s);
    assert_eq!(*l.borrow(), [2, 1, 0]);

    let mut w = SFixedVec::with_capacity_in(2, &mut f);
    w.push(D(7, l.clone()));
    w.push(D(8, l.clone()));
    drop(w);
    assert_eq!(*l.borrow(), [2, 1, 0, 8, 7]);
}

/// The reason the type exists: a node's children and the children's own
/// blocks live on the same stack.
struct Node<'f> {
    name: SStr<'f>,
    kids: SSlice<'f, Node<'f>>,
}

fn build<'f>(f: &mut Frame<'f>, depth: u32) -> Node<'f> {
    let name = f.alloc_str(&format!("n{depth}"));
    let mut kids = SFixedVec::with_capacity_in(if depth == 0 { 0 } else { 3 }, f);
    for _ in 0..kids.capacity() {
        let kid = build(f, depth - 1);
        kids.push(kid);
    }
    Node {
        name,
        kids: kids.into_slice(),
    }
}

fn count(n: &Node<'_>) -> usize {
    assert!(n.name.starts_with('n'));
    1 + n.kids.iter().map(count).sum::<usize>()
}

#[test]
fn tree_on_one_stack() {
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    let root = build(&mut f, 3);
    assert_eq!(count(&root), 40);
}
