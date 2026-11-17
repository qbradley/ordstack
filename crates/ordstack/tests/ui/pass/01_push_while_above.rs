#![allow(unused)]
use ordstack::{SVec, Stack};
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut lower: SVec<u32> = SVec::new_in(&mut f);
    let mut above = lower.above();
    let upper: SVec<u32> = SVec::new_in(&mut above);
    upper.push(2);
}
