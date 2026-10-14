#![allow(unused)]
use ordstack::Stack;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let b = f.sub().alloc(5u32);
    assert_eq!(*b, 5);
}
