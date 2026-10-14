#![allow(unused)]
use allocator_api2::vec::Vec;
use ordstack::Stack;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v: Vec<u32, _> = Vec::new_in(f.allocator());
    v.push(1);
    v.push(2);
}
