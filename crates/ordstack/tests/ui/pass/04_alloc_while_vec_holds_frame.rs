#![allow(unused)]
use ordstack::{SVec, Stack};
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let a: SVec<u32> = SVec::new_in(&mut f);
    a.push(1);
}
