#![allow(unused)]
use ordstack::Stack;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let b = st.frame().alloc(5u32);
    assert_eq!(*b, 5);
}
