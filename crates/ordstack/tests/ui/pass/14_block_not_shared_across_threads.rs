#![allow(unused)]
use ordstack::Stack;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let b = f.alloc(1u32);
    assert_eq!(*b, 1);
}
