#![allow(unused)]
use ordstack::{SVec, Stack};
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut v: SVec<u32> = SVec::new_in(&mut f);
    v.push(1);
    let mut it = v.into_iter();
    it.next();
}
